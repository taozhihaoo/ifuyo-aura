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
}

impl SharedOutputState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: Arc::new(AtomicU64::new(0)),
            underruns: Arc::new(AtomicU64::new(0)),
            target_gain_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
        }
    }

    /// 控制面：设置目标音量（值域已由 `Volume` 校验）。
    pub fn set_target_gain(&self, v: f32) {
        self.target_gain_bits.store(v.to_bits(), Ordering::Release);
    }

    fn target_gain(&self) -> f32 {
        f32::from_bits(self.target_gain_bits.load(Ordering::Acquire))
    }

    pub fn add_position(&self, frames: u64) {
        self.position.fetch_add(frames, Ordering::Relaxed);
    }

    pub fn add_underrun(&self) {
        self.underruns.fetch_add(1, Ordering::Relaxed);
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
}

impl OutputStage {
    #[must_use]
    pub fn new(sample_rate: SampleRate, channels: Channels) -> Self {
        Self {
            smoother: GainSmoother::new(sample_rate),
            seen_generation: Generation::FIRST,
            channels,
        }
    }

    #[must_use]
    pub const fn channels(&self) -> Channels {
        self.channels
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
    ) -> u64 {
        // Stopped/Paused：输出静音，位置不动
        if state != PlaybackState::Playing {
            out.fill(0.0);
            return 0;
        }

        let frames = out.len() / usize::from(self.channels.get());
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
        // 位置按输出实际消费的帧数推进（underrun 静音帧也是被消费的时间）
        shared.add_position(frames as u64);
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
