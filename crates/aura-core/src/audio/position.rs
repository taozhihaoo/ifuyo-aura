//! 帧权威播放位置状态（M1 §14–15）。
//!
//! - 权威：`consumed_frames`，由输出侧消费驱动；
//! - 暂停期间 position 不增长；
//! - Stop 语义：position 归零（M1 决策，见 DECISIONS.md D-023）；
//! - seek：直接设置目标 frame（并触发新 generation，见 generation 处理）。

use crate::error::AudioError;
use crate::types::{SampleRate, TrackPosition};

/// 帧权威播放位置跟踪器。
#[derive(Debug, Clone)]
pub struct PositionTracker {
    consumed_frames: u64,
    sample_rate: SampleRate,
}

impl PositionTracker {
    #[must_use]
    pub const fn new(sample_rate: SampleRate) -> Self {
        Self {
            consumed_frames: 0,
            sample_rate,
        }
    }

    #[must_use]
    pub const fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// 输出侧每消费 n 帧调用一次。
    pub fn advance(&mut self, frames: u64) {
        self.consumed_frames = self.consumed_frames.saturating_add(frames);
    }

    /// seek：直接设置目标 frame（新 generation 由调用方处理）。
    pub fn set_frame(&mut self, frame: u64) {
        self.consumed_frames = frame;
    }

    /// stop：归零（D-023）。
    pub fn reset(&mut self) {
        self.consumed_frames = 0;
    }

    #[must_use]
    pub const fn consumed_frames(&self) -> u64 {
        self.consumed_frames
    }

    /// 派生表示：当前播放位置。
    #[must_use]
    pub const fn track_position(&self) -> TrackPosition {
        TrackPosition::new(self.consumed_frames, self.sample_rate)
    }

    /// 变更输出采样率（设备切换等场景）；保持 frame 不变（帧数语义随新采样率解释）。
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) -> Result<(), AudioError> {
        if sample_rate.get() == 0 {
            return Err(AudioError::InvalidParameter("sample rate is zero"));
        }
        self.sample_rate = sample_rate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_and_pause_semantics() {
        let mut pos = PositionTracker::new(SampleRate::new(48_000).unwrap());
        pos.advance(480);
        pos.advance(480);
        assert_eq!(pos.consumed_frames(), 960);
        // 暂停 = 不再调用 advance；position 保持
        assert_eq!(pos.track_position().frame(), 960);
    }

    #[test]
    fn seek_sets_frame_and_stop_resets() {
        let mut pos = PositionTracker::new(SampleRate::new(48_000).unwrap());
        pos.advance(5_000);
        pos.set_frame(100_000);
        assert_eq!(pos.consumed_frames(), 100_000);
        pos.reset();
        assert_eq!(pos.consumed_frames(), 0);
    }

    #[test]
    fn advance_saturates_never_overflows() {
        let mut pos = PositionTracker::new(SampleRate::new(48_000).unwrap());
        pos.advance(u64::MAX);
        pos.advance(100);
        assert_eq!(pos.consumed_frames(), u64::MAX);
    }
}
