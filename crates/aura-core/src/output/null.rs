//! NullOutput（M1 §24–26）：无声卡确定性输出。
//!
//! 它是 Engine 内部的输出端：渲染阶段（OutputStage）+ 共享原子状态 + PCM 捕获。
//! "callback invocation" 即 `OutputStage::render`；整条链路
//! `synthetic audio → decode → resample → buffer → output → capture`
//! 全程在 CI 无声卡环境运行。

use crate::error::AudioError;
use crate::output::{AudioOutput, OutputStage, SharedOutputState};
use crate::types::{AudioFormat, Channels, SampleRate};

/// 无声卡输出：捕获 PCM、计数位置与 underrun。
pub struct NullOutput {
    format: AudioFormat,
    chunk_frames: usize,
    stage: OutputStage,
    shared: SharedOutputState,
    capture: Vec<f32>,
    running: bool,
}

impl NullOutput {
    #[must_use]
    pub fn new(sample_rate: SampleRate, channels: Channels, chunk_frames: usize) -> Self {
        Self {
            format: AudioFormat::new(sample_rate, channels),
            chunk_frames,
            stage: OutputStage::new(sample_rate, channels),
            shared: SharedOutputState::new(),
            capture: Vec::new(),
            running: false,
        }
    }

    #[must_use]
    pub const fn chunk_frames(&self) -> usize {
        self.chunk_frames
    }

    #[must_use]
    pub const fn stage(&self) -> &OutputStage {
        &self.stage
    }

    pub const fn stage_mut(&mut self) -> &mut OutputStage {
        &mut self.stage
    }

    #[must_use]
    pub const fn shared(&self) -> &SharedOutputState {
        &self.shared
    }

    /// 捕获到的 PCM（交错 f32）。
    #[must_use]
    pub fn capture(&self) -> &[f32] {
        &self.capture
    }

    /// 捕获缓冲（调用方控制容量：零分配测量前先 reserve）。
    pub fn capture_mut(&mut self) -> &mut Vec<f32> {
        &mut self.capture
    }

    pub fn clear_capture(&mut self) {
        self.capture.clear(); // 保留容量：reserve 后 extend 不再分配
    }

    /// 预分配捕获容量（帧）。
    pub fn reserve_capture(&mut self, frames: usize) {
        self.capture
            .reserve(frames * usize::from(self.format.channels.get()));
    }

    /// 仅在容量不足时追加预留（避免每次调用都触发 realloc）。
    pub fn ensure_capture_capacity(&mut self, frames: usize) {
        let needed = frames * usize::from(self.format.channels.get());
        if self.capture.len() + needed > self.capture.capacity() {
            self.capture.reserve(needed);
        }
    }
}

impl AudioOutput for NullOutput {
    fn format(&self) -> AudioFormat {
        self.format
    }

    fn start(&mut self) -> Result<(), AudioError> {
        self.running = true;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), AudioError> {
        self.running = false;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        self.running = false;
        self.clear_capture();
        Ok(())
    }

    fn consumed_frames(&self) -> u64 {
        self.shared
            .position
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    fn underruns(&self) -> u64 {
        self.shared
            .underruns
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}
