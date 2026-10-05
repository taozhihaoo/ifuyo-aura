//! 输出抽象（M1 §24/§29–31）。
//!
//! - [`AudioOutput`] trait：最小输出设备接口；
//! - [`OutputStage`]：**回调路径的唯一逻辑**——generation-aware 读取、underrun
//!   补静音、10ms 增益平滑、原子计数。真实设备回调与 NullOutput 确定性渲染
//!   共用这一份代码。
//!
//! Callback 铁律（M1 §31）：`OutputStage::render` 路径零 allocation、零锁、
//! 零 logging、零 syscall、零 panic；由 `tests/zero_alloc.rs` 用计数分配器证明。

pub mod device;
pub mod device_state;
pub mod null;

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use crate::audio::buffer::ConsumerHandle;
use crate::audio::gain::GainSmoother;
use crate::types::{AudioFormat, Channels, Generation, PlaybackState, SampleRate};

/// 输出共享状态（callback 与控制面之间的全部通道：仅原子量，§39）。
#[derive(Clone)]
pub struct SharedOutputState {
    /// 权威播放位置（输出侧已消费帧数）。
    pub position: Arc<AtomicU64>,
    /// underrun 计数。
    pub underruns: Arc<AtomicU64>,
    /// 目标增益（f32 bits；由控制面写入）。
    pub target_gain_bits: Arc<AtomicU32>,
    /// 音轨静态增益（loudness × preamp；f32 bits）。
    pub track_gain_bits: Arc<AtomicU32>,
    /// 播放状态（控制面发布；设备回调据此决定静音/推进位置——暂停时
    /// 位置必须停住 §183）。
    playback_state_bits: Arc<AtomicU32>,
}

impl SharedOutputState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: Arc::new(AtomicU64::new(0)),
            underruns: Arc::new(AtomicU64::new(0)),
            target_gain_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            track_gain_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            playback_state_bits: Arc::new(AtomicU32::new(state_bits(PlaybackState::Stopped))),
        }
    }

    /// 控制面：发布当前播放状态（driver 线程低频写入）。
    pub fn set_playback_state(&self, state: PlaybackState) {
        self.playback_state_bits
            .store(state_bits(state), Ordering::Release);
    }

    /// 回调侧：读取最近发布的播放状态。
    #[must_use]
    pub fn playback_state(&self) -> PlaybackState {
        match self.playback_state_bits.load(Ordering::Acquire) {
            b if b == state_bits(PlaybackState::Playing) => PlaybackState::Playing,
            b if b == state_bits(PlaybackState::Paused) => PlaybackState::Paused,
            _ => PlaybackState::Stopped,
        }
    }

    /// 控制面：设置目标音量（值域已由 `Volume` 校验）。
    pub fn set_target_gain(&self, v: f32) {
        self.target_gain_bits.store(v.to_bits(), Ordering::Release);
    }

    fn target_gain(&self) -> f32 {
        f32::from_bits(self.target_gain_bits.load(Ordering::Acquire))
    }

    /// 控制面：设置音轨静态增益（loudness × preamp 线性积）。
    pub fn set_track_gain(&self, v: f32) {
        self.track_gain_bits.store(v.to_bits(), Ordering::Release);
    }

    fn track_gain(&self) -> f32 {
        f32::from_bits(self.track_gain_bits.load(Ordering::Acquire))
    }

    pub fn add_position(&self, frames: u64) {
        self.position.fetch_add(frames, Ordering::Relaxed);
    }

    /// 仅在确认 Playing 状态下由回调调用（暂停时位置必须停住 §183）。
    pub fn add_position_playing(&self, frames: u64) {
        self.position.fetch_add(frames, Ordering::Relaxed);
    }

    pub fn add_underrun(&self) {
        self.underruns.fetch_add(1, Ordering::Relaxed);
    }
}

fn state_bits(state: PlaybackState) -> u32 {
    match state {
        PlaybackState::Stopped => 0,
        PlaybackState::Playing => 1,
        PlaybackState::Paused => 2,
    }
}

impl Default for SharedOutputState {
    fn default() -> Self {
        Self::new()
    }
}

/// 输出渲染阶段：回调路径唯一逻辑（见模块文档）。
pub struct OutputStage {
    smoother: GainSmoother,
    seen_generation: Generation,
    channels: Channels,
    eq_cascade: crate::dsp::EqCascade,
    local_eq: crate::dsp::EqCoeffs,
    last_eq_version: u32,
    /// limiter 开关（生产默认开；测试 provenance 可关，D-034）。
    limiter_enabled: bool,
    /// 上一块渲染状态（检测 Playing↔非 Playing 切换，做短淡变消爆音 §突刺）。
    last_state: PlaybackState,
    /// 淡变长度（帧，10ms @当前采样率；§突刺消爆音）。
    fade_frames: usize,
}

impl OutputStage {
    #[must_use]
    pub fn new(sample_rate: SampleRate, channels: Channels) -> Self {
        Self {
            smoother: GainSmoother::new(sample_rate),
            seen_generation: Generation::FIRST,
            channels,
            eq_cascade: crate::dsp::EqCascade::new(usize::from(channels.get())),
            local_eq: crate::dsp::EqCoeffs::passthrough(),
            last_eq_version: 0,
            limiter_enabled: true,
            last_state: PlaybackState::Stopped,
            fade_frames: sample_rate.get() as usize / 100, // 10ms
        }
    }

    #[must_use]
    pub const fn channels(&self) -> Channels {
        self.channels
    }

    /// 重建 EQ 级联状态（输出声道数变化时，M2 §31）。
    pub fn reset_eq(&mut self, channels: usize) {
        self.eq_cascade = crate::dsp::EqCascade::new(channels);
        self.local_eq = crate::dsp::EqCoeffs::passthrough();
        self.last_eq_version = 0;
    }

    /// limiter 开关（仅测试 provenance 场景关闭；生产恒开，D-034）。
    pub fn set_limiter_enabled(&mut self, enabled: bool) {
        self.limiter_enabled = enabled;
    }

    /// 10ms 淡变帧数（§突刺消爆音；按采样率换算）。
    fn fade_frames(&self) -> usize {
        self.fade_frames
    }

    /// 渲染至多 `out.len()` 个采样：从 ring buffer 弹出 → underrun 补静音 →
    /// 平滑增益。返回实际渲染的帧数（含 underrun 静音帧）。
    ///
    /// 约束：`out` 由调用方预分配；本函数不分配、不加锁、不 panic。
    pub fn render(
        &mut self,
        out: &mut [f32],
        consumer: &mut ConsumerHandle,
        shared: &SharedOutputState,
        state: PlaybackState,
        eq_bus: Option<&crate::dsp::EqBus>,
    ) -> u64 {
        // 非 Playing：切换块（last_state==Playing）消费缓冲残余并做 10ms
        // 淡出（消除波形硬切爆音"突刺"），位置按实际消费推进——此后纯静音。
        // §183：淡出后位置冻结。
        if state != PlaybackState::Playing {
            let ch = usize::from(self.channels.get());
            let was_playing = self.last_state == PlaybackState::Playing;
            self.last_state = state;
            if !was_playing {
                out.fill(0.0);
                return 0;
            }
            let got = consumer.pop_frames_until_generation(out, &mut self.seen_generation) as usize;
            let rendered = got * ch;
            out[rendered..].fill(0.0);
            // 10ms 线性淡出（覆盖 got 的前段；其余静音）。
            let fade = self.fade_frames().min(got.max(1));
            for i in 0..got {
                let g = if i < fade {
                    1.0 - (i as f32 / fade as f32)
                } else {
                    0.0
                };
                for c in 0..ch {
                    out[i * ch + c] *= g;
                }
            }
            shared.add_position(got as u64);
            return got as u64;
        }
        self.last_state = state;

        let frames = out.len() / usize::from(self.channels.get());
        // 恢复淡入（暂停/停止 → 播放的切换块）：消除恢复瞬间的爆音。
        let fade_in = self.last_state != PlaybackState::Playing;
        self.last_state = state;
        let got = consumer.pop_frames_until_generation(out, &mut self.seen_generation);
        if (got as usize) < frames {
            // underrun：尾部保持/置为静音，计数一次
            let silence_from = got as usize * usize::from(self.channels.get());
            out[silence_from..].fill(0.0);
            shared.add_underrun();
        }

        // PCM 安全（§37）：非有限值确定性替换为 0（D-025），随后平滑增益
        for s in out.iter_mut() {
            if !s.is_finite() {
                *s = 0.0;
            }
        }

        let target = shared.target_gain();
        self.smoother.set_target(target);
        self.smoother.process(out);
        // 音轨静态增益（loudness × preamp，§44 第一级）
        let track_gain = shared.track_gain();
        if track_gain != 1.0 {
            for s in out.iter_mut() {
                *s *= track_gain;
            }
        }
        // 恢复淡入（10ms 线性）：非 Playing → Playing 的切换块起始。
        if fade_in {
            let ch = usize::from(self.channels.get());
            let fade = self.fade_frames().min(frames);
            for f in 0..fade {
                let g = f as f32 / fade as f32;
                for c in 0..ch {
                    out[f * ch + c] *= g;
                }
            }
        }
        // EQ（§44 第三级；快照经 EqBus 原子发布，回调 load 无锁）
        if let Some(bus) = eq_bus {
            let v = bus.version();
            if v != self.last_eq_version && bus.load_into(&mut self.local_eq) {
                self.last_eq_version = v;
            }
            self.eq_cascade.process(&self.local_eq, out);
        }
        // Limiter（§44 末级，§21；D-034：provenance 测试可关）
        if self.limiter_enabled {
            crate::dsp::limit_block(out);
        }
        // 位置按输出实际消费的帧数推进（underrun 静音帧也是被消费的时间；
        // 本分支仅在 Playing 进入——Paused/Stopped 已在函数头静音返回）。
        shared.add_position_playing(frames as u64);
        frames as u64
    }
}

/// 最小输出设备抽象（M1 §29）。
///
/// 真实 CPAL 适配器（output/device）与 NullOutput 都实现此 trait；
/// M1 的重点与全部测试基于 NullOutput。
pub trait AudioOutput: Send {
    fn format(&self) -> AudioFormat;

    /// 开始/恢复消费。
    fn start(&mut self) -> Result<(), crate::error::AudioError>;

    /// 暂停消费（设备保持开启）。
    fn pause(&mut self) -> Result<(), crate::error::AudioError>;

    /// 停止并关闭。
    fn stop(&mut self) -> Result<(), crate::error::AudioError>;

    /// 输出侧已消费帧数（权威位置的来源）。
    fn consumed_frames(&self) -> u64;

    /// underrun 计数。
    fn underruns(&self) -> u64;
}
