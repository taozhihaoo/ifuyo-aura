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
use crate::audio::crossfade::crossfade_frames;
use crate::decoder::pcm::map_channels_into;
use crate::decoder::Decoder;
use crate::dsp::TrackLoudness;
use crate::engine::command::AudioCommandQueue;
use crate::error::AudioError;
use crate::output::device_state::OutputDeviceState;
use crate::output::null::NullOutput;
use crate::output::SharedOutputState;
use crate::resampler::Resampler;
use crate::track::TrackFrameRange;
use std::sync::Arc;

use crate::types::{
    AudioCommand, AudioFormat, Channels, Generation, PlaybackState, SampleRate, Volume,
};

/// 解码填充的目标块（帧）。
/// gapless 预解码窗口默认值：3 秒 × 48kHz（M2 §6）。
pub const DEFAULT_PRELOAD_WINDOW_FRAMES: u64 = 3 * 48_000;

/// 曲目过渡状态（M2 §47）：Normal = gapless 单管线；Crossfading = 双管线混叠。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionState {
    Normal,
    Crossfading { total_frames: u64, done_frames: u64 },
}

/// crossfade 默认时长（秒）：0 = gapless 优先（D-028）。
pub const CROSSFADE_ENGINE_DEFAULT_SECONDS: f64 = 0.0;
const FILL_CHUNK_FRAMES: usize = 1024;

/// 音频播放引擎（单线程确定性核心）。
pub struct Engine {
    state: PlaybackState,
    volume: Volume,
    channels: Channels,
    sample_rate: SampleRate,

    producer: ProducerHandle,
    consumer: ConsumerHandle,
    /// M6（D-064）：消费端已移交给外部真实输出。
    consumer_detached: bool,

    commands: rtrb::Consumer<AudioCommand>,
    invalid_commands: u64,

    decoder: Option<Box<dyn Decoder>>,
    next_decoder: Option<Box<dyn Decoder>>,
    resampler: Option<Resampler>,
    decode_scratch: crate::decoder::pcm::PcmBuffer,
    decode_scratch_b: crate::decoder::pcm::PcmBuffer,
    mapped_scratch: Vec<f32>,
    resample_in: Vec<f32>,
    /// 已解码待推送块（部分推送时保留余量，D-014 修复）。
    pending_block: Vec<f32>,
    /// pending_block 中已推送的采样数。
    pending_pos: usize,
    /// 重采样输入块工作区（预分配复用，D-035）。
    resample_chunk: Vec<f32>,
    /// 重采样输出工作区（预分配复用，D-035）。
    resample_out: Vec<f32>,
    eof: bool,
    /// 曲目自然播完（EOF 且 ring 已被回调耗尽且 Playing；无 next 可接力）。
    /// driver 轮询 take_track_finished 取走后投递 Stop 并通知前端队列。
    track_finished: bool,
    /// 当前曲目已解码帧数（记账用）。
    current_track_decoded: u64,
    /// 当前曲目在全局渲染帧空间中的起点。
    track_start_frame: u64,
    /// 当前曲目的源帧数（activate 时记录）。
    current_source_frames: u64,

    /// gapless 预解码窗口（帧）。
    preload_window_frames: u64,
    /// crossfade 长度（输出帧）；0 = 关闭（gapless 优先）。
    crossfade_frames: u64,
    /// 输出设备状态（M2 §26–28）。
    device_state: OutputDeviceState,
    /// 设备流错误计数（stream error 上报）。
    device_errors: u64,
    /// 音轨响度元数据（M2 消费端）。
    track_loudness: TrackLoudness,
    /// loudness 归一开关。
    loudness_enabled: bool,
    /// preamp 线性增益。
    preamp_gain: f32,
    /// EQ 参数总线。
    eq_bus: Arc<crate::dsp::EqBus>,
    /// 曲目过渡状态。
    transition: TransitionState,
    /// crossfade 中 B 管线的映射输出。
    mapped_b: Vec<f32>,
    /// B 管线已解码帧数（crossfade 记账）。
    next_track_decoded: u64,
    /// 曲目边界（全局渲染帧号；起始虚拟 0 不含）。
    track_boundaries: Vec<u64>,
    /// 每条已播放音轨的帧记账。
    track_frame_ranges: Vec<TrackFrameRange>,

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
            consumer_detached: false,
            invalid_commands: 0,
            decoder: None,
            next_decoder: None,
            resampler: None,
            decode_scratch: crate::decoder::pcm::PcmBuffer::new(),
            decode_scratch_b: crate::decoder::pcm::PcmBuffer::new(),
            mapped_scratch: Vec::new(),
            resample_in: Vec::new(),
            resample_chunk: Vec::new(),
            resample_out: Vec::new(),
            pending_block: Vec::new(),
            pending_pos: 0,
            eof: false,
            track_finished: false,
            current_track_decoded: 0,
            track_start_frame: 0,
            current_source_frames: 0,
            crossfade_frames: 0,
            transition: TransitionState::Normal,
            mapped_b: Vec::new(),
            next_track_decoded: 0,
            preload_window_frames: DEFAULT_PRELOAD_WINDOW_FRAMES,
            track_boundaries: Vec::new(),
            track_frame_ranges: Vec::new(),
            device_state: OutputDeviceState::Connected,
            device_errors: 0,
            track_loudness: TrackLoudness::missing(),
            loudness_enabled: false,
            preamp_gain: 1.0,
            eq_bus: Arc::new(crate::dsp::EqBus::new(crate::dsp::EqCoeffs::passthrough())),
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
    pub fn load_track(&mut self, decoder: Box<dyn Decoder>) -> Result<(), AudioError> {
        self.pending_block.clear();
        self.pending_pos = 0;
        self.transition = TransitionState::Normal;
        self.next_decoder = None;
        self.mapped_b.clear();
        self.next_track_decoded = 0;
        self.activate_decoder(decoder)?;
        self.producer.bump_generation();
        self.apply_stop();
        Ok(())
    }

    /// 激活解码器为当前管线（重采样/缓冲按源格式重建），不动 generation/位置/状态。
    fn activate_decoder(&mut self, decoder: Box<dyn Decoder>) -> Result<(), AudioError> {
        let src_format = decoder.format();
        let src_rate = SampleRate::new(src_format.sample_rate.get())?;
        self.resampler = Resampler::new(src_rate, self.sample_rate, self.channels)?;
        // 解码缓冲按源格式预分配一次
        self.decode_scratch
            .reserve_frames(src_format, FILL_CHUNK_FRAMES * 4);
        let mut decoder = decoder;
        decoder.seek_to_frame(0)?;
        self.current_source_frames = decoder.duration_frames().unwrap_or(0);
        self.current_track_decoded = 0;
        self.decoder = Some(decoder);
        self.eof = false;
        Ok(())
    }

    /// 预装载下一音轨（M2 §6 gapless）：不打断当前播放。
    ///
    /// 当前曲目进入预解码窗口时由控制面调用；曲目自然结束时无缝切换。
    pub fn preload_next(&mut self, decoder: Box<dyn Decoder>) {
        self.next_decoder = Some(decoder);
    }

    /// 当前曲目剩余源帧数（已知 duration 时），用于预解码窗口判断（M2 §6）。
    #[must_use]
    pub fn source_frames_remaining(&self) -> Option<u64> {
        let d = self.decoder.as_ref()?;
        let total = d.duration_frames()?;
        let consumed = self.position_frames();
        Some(total.saturating_sub(consumed))
    }

    /// 预解码窗口（帧）。
    #[must_use]
    pub const fn preload_window_frames(&self) -> u64 {
        self.preload_window_frames
    }

    pub fn set_preload_window_frames(&mut self, frames: u64) {
        self.preload_window_frames = frames;
    }

    /// 已发生的曲目边界（全局渲染帧号，含虚拟 0 起点）。
    #[must_use]
    pub fn track_boundaries(&self) -> &[u64] {
        &self.track_boundaries
    }

    /// 设置 crossfade 时长（秒；0 = 关闭 → gapless 优先）。
    pub fn set_crossfade_seconds(&mut self, seconds: f64) {
        self.crossfade_frames = crossfade_frames(seconds, self.sample_rate);
    }

    #[must_use]
    pub const fn crossfade_frames(&self) -> u64 {
        self.crossfade_frames
    }

    #[must_use]
    pub const fn transition(&self) -> TransitionState {
        self.transition
    }

    // ===== M2：响度 / preamp / EQ（控制面）=====

    /// 设置音轨响度元数据（控制面提供；M2 只消费，§2/§19）。
    pub fn set_track_loudness(&mut self, loudness: TrackLoudness, enabled: bool) {
        self.track_loudness = loudness;
        self.loudness_enabled = enabled;
        self.refresh_track_gain();
    }

    /// 设置 preamp（dB，−12 ~ +12，§39）。
    pub fn set_preamp_db(&mut self, db: f32) {
        let db = db.clamp(-12.0, 12.0);
        self.preamp_gain = 10.0f32.powf(db / 20.0);
        self.refresh_track_gain();
    }

    #[must_use]
    pub const fn preamp_gain(&self) -> f32 {
        self.preamp_gain
    }

    /// limiter 开关（生产恒开；provenance 测试关闭，D-034）。
    pub fn set_limiter_enabled(&mut self, enabled: bool) {
        self.output.stage_mut().set_limiter_enabled(enabled);
    }

    /// 发布 EQ 频段（系数在控制面计算，快照原子发布，§41–42）。
    pub fn set_eq_bands(&mut self, bands: &[crate::dsp::EqBand]) {
        let coeffs = crate::dsp::EqCoeffs::from_bands(bands, self.sample_rate.get() as f32);
        self.eq_bus.store(&coeffs);
    }

    /// 按预设发布 EQ。
    pub fn set_eq_preset(&mut self, preset: &crate::dsp::EqPreset) {
        self.set_eq_bands(&crate::dsp::bands_from_preset(preset));
    }

    #[must_use]
    pub fn eq_bus(&self) -> &crate::dsp::EqBus {
        &self.eq_bus
    }

    /// 重算音轨静态增益 = loudness gain × preamp（§44 前两级合并进平滑目标）。
    fn refresh_track_gain(&mut self) {
        let lg = crate::dsp::loudness_gain(&self.track_loudness, self.loudness_enabled);
        let v = lg * self.preamp_gain;
        self.output.shared().set_track_gain(v);
    }

    // ===== M2：输出设备热插拔（§26–33）=====

    /// 设备断开：安全暂停（保留曲目/位置/状态），可恢复。不 panic。
    pub fn device_removed(&mut self) {
        self.device_state = OutputDeviceState::Removed;
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused; // 安全暂停（§27-4）
        }
        // 位置保留（§32）；曲目/解码器保留（§27-2,3）
    }

    /// 设备恢复：进入可恢复暂停状态，**不自动播放**（§28）。
    pub fn device_restored(&mut self, sample_rate: SampleRate, channels: Channels) {
        self.device_state = OutputDeviceState::Connected;
        let _ = self.set_output_format(sample_rate, channels);
    }

    /// 默认设备变化：与设备断开/恢复同一处理路径（§29）。
    pub fn default_device_changed(&mut self, sample_rate: SampleRate, channels: Channels) {
        self.device_restored(sample_rate, channels);
    }

    /// 设备流错误上报（由适配器错误回调经控制面转入；不 panic）。
    pub fn report_device_error(&mut self) {
        self.device_errors += 1;
    }

    #[must_use]
    pub const fn device_state(&self) -> OutputDeviceState {
        self.device_state
    }

    #[must_use]
    pub const fn device_errors(&self) -> u64 {
        self.device_errors
    }

    /// 新设备格式重配置（§31）：重建重采样器与渲染缓冲，位置保留（§32）。
    pub fn set_output_format(
        &mut self,
        sample_rate: SampleRate,
        channels: Channels,
    ) -> Result<(), AudioError> {
        if self.sample_rate.get() != sample_rate.get() || self.channels.get() != channels.get() {
            self.sample_rate = sample_rate;
            self.channels = channels;
            // 按当前解码器源格式重建重采样器（解码器格式未变）
            let src = self
                .decoder
                .as_ref()
                .map(|d| d.format())
                .map(|f| f.sample_rate.get())
                .unwrap_or(sample_rate.get());
            let src_rate = SampleRate::new(src)?;
            self.resampler = Resampler::new(src_rate, sample_rate, channels)?;
            self.resample_in.clear();
            let chunk_frames = (self.output.chunk_frames()).max(1);
            self.chunk_buf = vec![0.0; chunk_frames * usize::from(channels.get())];
            self.output
                .stage_mut()
                .reset_eq(usize::from(channels.get()));
        }
        Ok(())
    }

    /// TrackFrameRange 记账（M2 §8）：每条已播放音轨的有效帧范围。
    #[must_use]
    pub fn track_frame_ranges(&self) -> &[TrackFrameRange] {
        &self.track_frame_ranges
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
                // 无曲目时拒绝 Play：否则 Playing + 空 ring 持续 underrun
                // 静音而位置照常推进（UI 计时器走字、显示可暂停的假播放态；
                // 用户报告"没有歌播放时计时器在走"）。Stop 后 decoder 保留
                // （归零重播语义），此处不受影响。
                PlaybackState::Stopped | PlaybackState::Paused if self.decoder.is_some() => {
                    self.state = PlaybackState::Playing;
                }
                PlaybackState::Stopped | PlaybackState::Paused => {
                    self.invalid_commands += 1;
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
                // seek 目标是输出帧空间；解码器按源采样率寻址——必须换算
                // （44.1k 源收 48k 帧号=时间×48/44.1：近端 seek 落到超界
                // 位置而失败、远端内容与进度错位）。
                let src_rate = u64::from(decoder.format().sample_rate.get());
                let out_rate = u64::from(self.sample_rate.get());
                let src_frame = if src_rate == out_rate {
                    frame
                } else {
                    u64::try_from(u128::from(frame) * u128::from(src_rate) / u128::from(out_rate))
                        .unwrap_or(u64::MAX)
                };
                if let Err(_e) = decoder.seek_to_frame(src_frame) {
                    self.invalid_commands += 1;
                    return;
                }
                // seek 使 pending 旧代样本失效；transition 取消、next 失效（§49）
                self.pending_block.clear();
                self.pending_pos = 0;
                self.resample_in.clear(); // 源侧未重采样残片同样失效
                self.transition = TransitionState::Normal;
                self.next_decoder = None;
                self.mapped_b.clear();
                self.next_track_decoded = 0;
                self.eof = false;
                self.track_finished = false;
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
        self.pending_block.clear();
        self.pending_pos = 0;
        self.transition = TransitionState::Normal;
        self.next_decoder = None;
        self.mapped_b.clear();
        self.next_track_decoded = 0;
        self.state = PlaybackState::Stopped;
        // Stop 语义 = 回到曲目开头：解码位置必须归零，否则再 Play 会从
        // 中断处继续（进度显示归零但实际音频失同步——M12 用户报告）。
        if let Some(decoder) = self.decoder.as_mut() {
            let _ = decoder.seek_to_frame(0);
        }
        self.output.shared().position.store(0, Ordering::Release);
        self.producer.bump_generation();
        self.eof = false;
        self.track_finished = false;
    }

    /// 将 pending_block 中未推送的部分尽量推入 ring buffer。
    /// 返回推送的帧数。缓冲满时保留余量（pending_pos 记录进度）。
    fn flush_pending(&mut self) -> u64 {
        let ch = usize::from(self.channels.get());
        let mut pushed_total = 0u64;
        while self.pending_pos < self.pending_block.len() {
            let free_samples = self.producer.free_frames() * ch;
            if free_samples == 0 {
                break; // 背压（D-022）：保留余量，不丢弃
            }
            let remaining = self.pending_block.len() - self.pending_pos;
            let n = remaining.min(free_samples);
            let pushed_frames = self
                .producer
                .push_frames(&self.pending_block[self.pending_pos..self.pending_pos + n]);
            let pushed_samples = pushed_frames as usize * ch;
            self.pending_pos += pushed_samples;
            pushed_total += pushed_frames;
            if pushed_samples < n {
                break;
            }
        }
        if self.pending_pos >= self.pending_block.len() {
            self.pending_pos = 0;
            self.pending_block.clear();
        }
        pushed_total
    }

    /// 解码填充：直到缓冲满、EOF 或无音轨。返回本次推入的帧数。
    ///
    /// M2 gapless：当前曲目解码结束时，若已预装载下一曲目则**无缝切换**
    /// （不 bump generation、不清缓冲、位置连续推进，§5–§8）。
    /// 部分推送的解码块通过 pending_block/pending_pos 持久保留（修复 M1
    /// 在非对齐 free 时的帧丢失缺陷，见 DECISIONS D-014）。
    pub fn fill(&mut self) -> u64 {
        let mut pushed_total = 0u64;
        loop {
            // 0) 先推送上次未推送完的余量
            pushed_total += self.flush_pending();
            if self.producer.free_frames() == 0 {
                break; // 背压：缓冲满（D-022）
            }
            if self.eof {
                break;
            }
            if self.decoder.is_none() {
                break;
            }

            // 0.5) crossfade 进入检查（M2 §15 情况 B：cf > 0 且有 next）
            if self.crossfade_frames > 0
                && self.transition == TransitionState::Normal
                && self.next_decoder.is_some()
            {
                let remaining = self
                    .current_source_frames
                    .saturating_sub(self.current_track_decoded);
                // 格式一致性（bypass 双管线前提）：不一致则 gapless 回退（D-033）
                let same_format = self.next_decoder.as_ref().is_some_and(|d| {
                    d.format() == AudioFormat::new(self.sample_rate, self.channels)
                }) && self.resampler.is_none();
                if remaining <= self.crossfade_frames && same_format {
                    self.transition = TransitionState::Crossfading {
                        total_frames: remaining.min(self.crossfade_frames),
                        done_frames: 0,
                    };
                }
            }

            // 1) 解码一批（源格式）
            self.decode_scratch.clear();
            let decoded_frames = {
                let Some(decoder) = self.decoder.as_mut() else {
                    break;
                };
                let n = match decoder.decode_next(&mut self.decode_scratch) {
                    Ok(n) => n,
                    Err(_) => {
                        self.eof = true;
                        break;
                    }
                };
                self.current_track_decoded += n;
                n
            };
            // 1b) Crossfading：B 管线同步解码 + 等功率混入 mapped_scratch
            if let TransitionState::Crossfading {
                total_frames: cf_total,
                done_frames: cf_done,
            } = self.transition
            {
                self.decode_scratch_b.clear();
                let mut b_dec = 0u64;
                while b_dec < decoded_frames {
                    let Some(b_decoder) = self.next_decoder.as_mut() else {
                        break; // B 耗尽：A 侧继续（B 视为静音）
                    };
                    match b_decoder.decode_next(&mut self.decode_scratch_b) {
                        Ok(n) if n > 0 => {
                            b_dec += n;
                            self.next_track_decoded += n;
                        }
                        _ => break,
                    }
                }
                let src_channels = self
                    .next_decoder
                    .as_ref()
                    .expect("crossfade requires next")
                    .format()
                    .channels;
                let b_ok = map_channels_into(
                    self.decode_scratch_b.data(),
                    src_channels,
                    self.channels,
                    &mut self.mapped_b,
                )
                .is_ok();
                let ch = usize::from(self.channels.get());
                let mixable = (decoded_frames as usize * ch).min(self.mapped_scratch.len());
                for idx in 0..mixable {
                    let i = idx / ch;
                    let t = ((cf_done + i as u64) as f32 / cf_total as f32).clamp(0.0, 1.0);
                    let (ga, gb) = crate::audio::crossfade::equal_power_gains(t);
                    let b = if b_ok && idx < self.mapped_b.len() {
                        self.mapped_b[idx]
                    } else {
                        0.0
                    };
                    self.mapped_scratch[idx] = self.mapped_scratch[idx] * ga + b * gb;
                }
                let new_done = (cf_done + decoded_frames).min(cf_total);
                self.transition = TransitionState::Crossfading {
                    total_frames: cf_total,
                    done_frames: new_done,
                };
            }

            if decoded_frames == 0 {
                // 当前曲目完成（自然结束或 crossfade 尾部）。记账（渲染帧空间）。
                let (overlap, b_seed) = match self.transition {
                    TransitionState::Crossfading { done_frames, .. } => {
                        (done_frames, self.next_track_decoded)
                    }
                    _ => (0, 0),
                };
                self.track_frame_ranges.push(TrackFrameRange {
                    source_frames: self.current_source_frames,
                    valid_frames: self.current_track_decoded,
                    rendered_frames: self.current_track_decoded,
                });

                // 下一曲目接管管线：gapless（无缝）或 crossfade（B 已提前 overlap）
                if let Some(next) = self.next_decoder.take() {
                    let a_end = self.track_start_frame + self.current_track_decoded;
                    let b_start = a_end.saturating_sub(overlap);
                    self.track_boundaries.push(b_start);
                    self.track_start_frame = b_start;
                    self.current_track_decoded = b_seed;
                    self.current_source_frames = next.duration_frames().unwrap_or(0);
                    let src_format = next.format();
                    self.decode_scratch
                        .reserve_frames(src_format, FILL_CHUNK_FRAMES * 4);
                    // 不 bump generation、不清缓冲、位置连续
                    self.decoder = Some(next);
                    self.transition = TransitionState::Normal;
                    self.mapped_b.clear();
                    self.next_track_decoded = 0;
                    self.eof = false;
                    continue;
                }

                self.eof = true;
                // flush resampler 残留输入（EOF 尾块）到 pending，由下次推送
                if let Some(mut r) = self.resampler.take() {
                    if !self.resample_in.is_empty() {
                        r.process_chunk_partial_into(&self.resample_in, &mut self.resample_out)
                            .unwrap_or_default();
                        self.resample_in.clear();
                        self.pending_block.extend_from_slice(&self.resample_out);
                    }
                    self.resampler = Some(r);
                }
                continue;
            }

            // 2) 声道映射到输出声道（零分配：复用 mapped_scratch 容量）
            let src_channels = {
                let decoder = self.decoder.as_ref().expect("decoder presence checked");
                decoder.format().channels
            };
            let mapped_ok = map_channels_into(
                self.decode_scratch.data(),
                src_channels,
                self.channels,
                &mut self.mapped_scratch,
            )
            .is_ok();

            // 3) 重采样或直通 → 写入 pending_block（经 flush_pending 推送）
            self.pending_block.clear();
            self.pending_pos = 0;
            if let Some(mut r) = self.resampler.take() {
                if mapped_ok {
                    self.resample_in.extend_from_slice(&self.mapped_scratch);
                }
                while self.resample_in.len() / usize::from(self.channels.get())
                    >= r.input_frames_next()
                {
                    let need = r.input_frames_next() * usize::from(self.channels.get());
                    // 复用预分配 resample_chunk（避免每块 to_vec 分配，D-035）
                    self.resample_chunk.clear();
                    self.resample_chunk
                        .extend_from_slice(&self.resample_in[..need]);
                    self.resample_in.drain(..need);
                    r.process_chunk_into(&self.resample_chunk, &mut self.resample_out)
                        .unwrap_or_default();
                    self.pending_block.extend_from_slice(&self.resample_out);
                }
                self.resampler = Some(r);
            } else if mapped_ok {
                self.pending_block.extend_from_slice(&self.mapped_scratch);
            }

            pushed_total += self.flush_pending();
            if self.producer.free_frames() == 0 {
                break;
            }
            if self.pending_pos < self.pending_block.len() {
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
        // 捕获容量仅在实际不足时追加预留（reserve 本身也是分配；
        // 诊断捕获属捕获面而非回调路径，见 D-035）
        self.output.ensure_capture_capacity(n as usize);
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
                    Some(&self.eq_bus),
                )
            };
            let _ = got;
            let chunk = &self.chunk_buf[..want];
            self.output.capture_mut().extend_from_slice(chunk);
            rendered += take as u64;
        }
        rendered
    }

    /// M6（D-064）：把消费端移交给外部真实输出（CPAL 回调内的 OutputStage）。
    /// 此后 [`Engine::render_for_frames`] 不再可用（ring 的排空由外部回调承担）；
    /// 驱动线程仅调用 [`Engine::fill_and_process`]（ring 生产侧 + 命令处理）。
    /// 返回外部输出所需的 (consumer, shared, eq_bus) 克隆。
    ///
    /// # Errors
    /// 重复调用（消费者已被移交）时返回 [`AudioError::OutputError`]。
    pub fn detach_consumer_for_external_output(
        &mut self,
    ) -> Result<
        (
            crate::audio::buffer::ConsumerHandle,
            crate::output::SharedOutputState,
            Arc<crate::dsp::EqBus>,
        ),
        AudioError,
    > {
        if self.consumer_detached {
            return Err(AudioError::OutputError("consumer already detached"));
        }
        self.consumer_detached = true;
        let consumer = std::mem::replace(
            &mut self.consumer,
            // 占位 consumer：永不可读（detach 后 render_for_frames 禁用）。
            Self::detached_placeholder_consumer(),
        );
        Ok((consumer, self.shared.clone(), Arc::clone(&self.eq_bus)))
    }

    fn detached_placeholder_consumer() -> crate::audio::buffer::ConsumerHandle {
        // 一个空 ring 的 consumer：读永远得 0，任何误用只会静默无输出。
        let (_p, c) = crate::audio::buffer::ring_buffer(
            crate::types::Channels::new(2).expect("2 channels"),
            8,
        )
        .expect("placeholder ring");
        c
    }

    /// M6：驱动线程调用 —— 处理命令 + 播放态下填充 ring（生产侧）。
    /// 消费排空由外部真实输出承担（detach 后）。
    pub fn fill_and_process(&mut self) {
        self.process_commands();
        if self.state == crate::types::PlaybackState::Playing {
            self.fill();
            // 曲目自然播完：EOF 且 ring 已被回调耗尽（有 next 时 fill 内部
            // gapless 接管并复位 eof，不会走到这里）。driver 取走标记后
            // 投递 Stop 并通知前端队列自动前进——否则引擎停留 Playing，
            // 回调持续 underrun 静音而位置照常推进（假播放）。
            if self.eof && self.producer.free_frames() == self.producer.capacity_frames() {
                self.track_finished = true;
            }
        }
    }

    /// 曲目自然播完标记（驱动线程轮询；取走即复位）。
    pub fn take_track_finished(&mut self) -> bool {
        std::mem::take(&mut self.track_finished)
    }

    /// 捕获访问。
    #[must_use]
    /// 渲染单块（不带 fill/命令处理）——零分配测试直接度量回调路径。
    /// 返回渲染帧数。仅测试/诊断用途。
    pub fn render_block_only(&mut self, out: &mut [f32]) -> u64 {
        let shared = self.shared.clone();
        let stage = self.output.stage_mut();
        stage.render(
            out,
            &mut self.consumer,
            &shared,
            self.state,
            Some(&self.eq_bus),
        )
    }

    /// 灌满解码缓冲（解码面操作，允许分配；零分配度量前调用）。
    pub fn fill_buffer_only(&mut self) -> u64 {
        self.fill()
    }

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
