//! 引擎（M1）：控制面 + 渲染面的单线程确定性核心。
//!
//! ## 线程模型（M1 §38，所有者记录）
//!
//! ```text
//! Control Thread   ──post──►  Command Queue (rtrb SPSC)
//! Engine Thread    ──owns──►  Engine（decoder / resampler / producer / state）
//! Output Callback  ──owns──►  ConsumerHandle + OutputStage（或 NullOutput 确定性渲染）
//! ```
//!
//! M1 中 Engine 以**同步确定性**方式驱动（`render_for_frames` 显式推进），
//! 上述三个角色在测试中由同一线程依次扮演；真实线程化（解码线程 + CPAL 回调）
//! 在 output/device 接线时引入，共享面仅原子量（SharedOutputState）。
//!
//! ## Overrun 策略（D-022）
//!
//! producer 写满即拒绝（部分写入），fill 循环背压暂停——不覆盖、不丢数据。

use std::sync::atomic::Ordering;

use crate::audio::buffer::{ring_buffer, ConsumerHandle, ProducerHandle};
use crate::decoder::pcm::map_channels_into;
use crate::decoder::Decoder;
use crate::engine::command::AudioCommandQueue;
use crate::error::AudioError;
use crate::output::null::NullOutput;
use crate::output::SharedOutputState;
use crate::resampler::Resampler;
use crate::types::{AudioCommand, Channels, Generation, PlaybackState, SampleRate, Volume};

/// 解码填充的目标块（帧）。
const FILL_CHUNK_FRAMES: usize = 1024;

/// 音频播放引擎（单线程确定性核心）。
pub struct Engine {
    state: PlaybackState,
    volume: Volume,
    channels: Channels,
    sample_rate: SampleRate,

    producer: ProducerHandle,
    consumer: ConsumerHandle,

    commands: rtrb::Consumer<AudioCommand>,
    invalid_commands: u64,

    decoder: Option<Box<dyn Decoder>>,
    resampler: Option<Resampler>,
    decode_scratch: crate::decoder::pcm::PcmBuffer,
    mapped_scratch: Vec<f32>,
    resample_in: Vec<f32>,
    eof: bool,

    output: NullOutput,
    /// 渲染工作区（预分配一次；字段级不相交借用，见 render_for_frames）。
    chunk_buf: Vec<f32>,
    /// 与 `output.shared()` 同一 Arc 的克隆（避免跨字段借用）。
    shared: SharedOutputState,
}

impl Engine {
    /// 创建引擎与配套命令投递端。
    ///
    /// `buffer_frames` 为 ring buffer 容量（帧）；渲染 chunk 为其 1/8（至少 1）。
    pub fn new(
        sample_rate: SampleRate,
        channels: Channels,
        buffer_frames: usize,
    ) -> Result<(Self, AudioCommandQueue), AudioError> {
        let (producer, consumer) =
            ring_buffer(channels, buffer_frames * usize::from(channels.get()))?;
        let (cmd_producer, cmd_consumer) = rtrb::RingBuffer::new(64);
        let chunk_frames = (buffer_frames / 8).max(1);

        let output = NullOutput::new(sample_rate, channels, chunk_frames);
        let shared = output.shared().clone();
        let engine = Self {
            state: PlaybackState::Stopped,
            volume: Volume::MAX,
            channels,
            sample_rate,
            producer,
            consumer,
            commands: cmd_consumer,
            invalid_commands: 0,
            decoder: None,
            resampler: None,
            decode_scratch: crate::decoder::pcm::PcmBuffer::new(),
            mapped_scratch: Vec::new(),
            resample_in: Vec::new(),
            eof: false,
            output,
            chunk_buf: vec![0.0; chunk_frames * usize::from(channels.get())],
            shared,
        };
        let poster = AudioCommandQueue::new(cmd_producer, 64);
        Ok((engine, poster))
    }

    #[must_use]
    pub const fn state(&self) -> PlaybackState {
        self.state
    }

    #[must_use]
    pub const fn volume(&self) -> Volume {
        self.volume
    }

    /// 权威播放位置（帧，输出侧消费）。
    #[must_use]
    pub fn position_frames(&self) -> u64 {
        self.output.shared().position.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn underruns(&self) -> u64 {
        self.output.shared().underruns.load(Ordering::Relaxed)
    }

    #[must_use]
    pub const fn invalid_commands(&self) -> u64 {
        self.invalid_commands
    }

    /// 装载音轨（新 generation）。状态回到 Stopped，位置归零。
    pub fn load_track(&mut self, mut decoder: Box<dyn Decoder>) -> Result<(), AudioError> {
        let src_format = decoder.format();
        let src_rate = SampleRate::new(src_format.sample_rate.get())?;
        self.resampler = Resampler::new(src_rate, self.sample_rate, self.channels)?;
        // 解码缓冲按源格式预分配一次
        self.decode_scratch
            .reserve_frames(src_format, FILL_CHUNK_FRAMES * 4);
        decoder.seek_to_frame(0)?;
        self.decoder = Some(decoder);
        self.eof = false;
        self.producer.bump_generation();
        self.apply_stop();
        Ok(())
    }

    /// 控制面投递命令后，由渲染循环调用：应用全部待处理命令。
    pub fn process_commands(&mut self) -> u64 {
        let mut applied = 0u64;
        while let Ok(cmd) = self.commands.pop() {
            self.apply_command(cmd);
            applied += 1;
        }
        applied
    }

    fn apply_command(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::Play => match self.state {
                PlaybackState::Stopped | PlaybackState::Paused => {
                    self.state = PlaybackState::Playing;
                }
                PlaybackState::Playing => {}
            },
            AudioCommand::Pause => {
                if self.state == PlaybackState::Playing {
                    self.state = PlaybackState::Paused;
                }
            }
            AudioCommand::Stop => {
                if self.state != PlaybackState::Stopped {
                    self.apply_stop();
                }
            }
            AudioCommand::Seek { frame } => {
                let Some(decoder) = self.decoder.as_mut() else {
                    self.invalid_commands += 1;
                    return;
                };
                if let Err(_e) = decoder.seek_to_frame(frame) {
                    self.invalid_commands += 1;
                    return;
                }
                self.eof = false;
                self.producer.bump_generation();
                self.output
                    .shared()
                    .position
                    .store(frame, Ordering::Release);
            }
            AudioCommand::SetVolume { volume } => match Volume::new(volume) {
                Ok(v) => {
                    self.volume = v;
                    self.output.shared().set_target_gain(v.get());
                }
                Err(_) => self.invalid_commands += 1,
            },
        }
    }

    fn apply_stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.output.shared().position.store(0, Ordering::Release);
        self.producer.bump_generation();
        self.eof = false;
    }

    /// 解码填充：直到缓冲满、EOF 或无音轨。返回本次推入的帧数。
    pub fn fill(&mut self) -> u64 {
        let mut pushed_total = 0u64;
        loop {
            if self.eof {
                break;
            }
            let free = self.producer.free_frames();
            if free == 0 {
                break; // 背压：缓冲满（D-022）
            }
            let Some(decoder) = self.decoder.as_mut() else {
                break;
            };

            // 1) 解码一批（源格式）
            self.decode_scratch.clear();
            let decoded_frames = match decoder.decode_next(&mut self.decode_scratch) {
                Ok(n) => n,
                Err(_) => {
                    self.eof = true;
                    break;
                }
            };
            if decoded_frames == 0 {
                self.eof = true;
                // flush resampler 残留输入（EOF 尾块）：先取出 resampler 避免借用纠缠
                if let Some(mut r) = self.resampler.take() {
                    if !self.resample_in.is_empty() {
                        let out = r
                            .process_chunk_partial(&self.resample_in)
                            .unwrap_or_default();
                        self.resample_in.clear();
                        let playing = self.state == PlaybackState::Playing
                            || self.state == PlaybackState::Paused;
                        if playing {
                            pushed_total += self.producer.push_frames(&out);
                        }
                    }
                    self.resampler = Some(r);
                }
                break;
            }

            // 2) 声道映射到输出声道（零分配：复用 mapped_scratch 容量）
            let src_channels = decoder.format().channels;
            let mapped_ok = map_channels_into(
                self.decode_scratch.data(),
                src_channels,
                self.channels,
                &mut self.mapped_scratch,
            )
            .is_ok();

            // 3) 重采样或直通（先取出 resampler 解借用）
            if let Some(mut r) = self.resampler.take() {
                if mapped_ok {
                    self.resample_in.extend_from_slice(&self.mapped_scratch);
                }
                while self.resample_in.len() / usize::from(self.channels.get())
                    >= r.input_frames_next()
                {
                    let need = r.input_frames_next() * usize::from(self.channels.get());
                    let chunk: Vec<f32> = self.resample_in[..need].to_vec();
                    self.resample_in.drain(..need);
                    let out = r.process_chunk(&chunk).unwrap_or_default();
                    let playing =
                        self.state == PlaybackState::Playing || self.state == PlaybackState::Paused;
                    if playing {
                        pushed_total += self.producer.push_frames(&out);
                    }
                    if self.producer.free_frames() == 0 {
                        self.resampler = Some(r);
                        return pushed_total;
                    }
                }
                self.resampler = Some(r);
            } else {
                let playing =
                    self.state == PlaybackState::Playing || self.state == PlaybackState::Paused;
                if playing && mapped_ok {
                    pushed_total += self.producer.push_frames(&self.mapped_scratch);
                }
            }

            if self.producer.free_frames() == 0 {
                break;
            }
        }
        pushed_total
    }

    /// 确定性渲染：推进 n 帧（命令 → 填充 → 渲染 → 捕获）。
    ///
    /// 同一输入 + 同一命令序列 + 同一虚拟时钟 ⇒ 完全相同的 PCM（M1 §26）。
    /// 返回实际渲染的帧数。
    pub fn render_for_frames(&mut self, n: u64) -> u64 {
        let ch = usize::from(self.channels.get());
        let chunk_frames = self.output.chunk_frames();
        let shared = self.shared.clone();
        let mut rendered: u64 = 0;
        while rendered < n {
            self.process_commands();
            if self.state == PlaybackState::Playing {
                self.fill();
            }
            let take = usize::try_from(n - rendered)
                .unwrap_or(usize::MAX)
                .min(chunk_frames);
            let want = take * ch;
            let got = {
                let stage = self.output.stage_mut();
                stage.render(
                    &mut self.chunk_buf[..want],
                    &mut self.consumer,
                    &shared,
                    self.state,
                )
            };
            let _ = got;
            let chunk = &self.chunk_buf[..want];
            self.output.capture_mut().extend_from_slice(chunk);
            rendered += take as u64;
        }
        rendered
    }

    /// 捕获访问。
    #[must_use]
    pub fn capture(&self) -> &[f32] {
        self.output.capture()
    }

    pub fn reserve_capture(&mut self, frames: usize) {
        self.output.reserve_capture(frames);
    }

    pub fn clear_capture(&mut self) {
        self.output.clear_capture();
    }

    /// 当前 generation（producer 视角）。
    #[must_use]
    pub fn generation(&self) -> Generation {
        self.producer.generation()
    }
}
