//! 核心领域类型（M1 §6）。
//!
//! 纪律：
//! - 时间权威是 frame（`u64`），秒只是 UI/API 表示（`position_seconds`）；
//! - 禁止以浮点秒作为权威状态；
//! - 参数类型在构造时校验（NaN/Inf/负值拒绝），非法输入返回 `AudioError`，
//!   任何路径不得 panic。

use crate::error::AudioError;

/// 采样率（Hz）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SampleRate(u32);

impl SampleRate {
    /// 校验并构造。M1 接受 8 kHz ~ 384 kHz。
    pub fn new(hz: u32) -> Result<Self, AudioError> {
        if (8_000..=384_000).contains(&hz) {
            Ok(Self(hz))
        } else {
            Err(AudioError::InvalidParameter("sample rate out of range"))
        }
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// 声道数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Channels(u16);

impl Channels {
    /// 校验并构造。M1 接受 1（mono）与 2（stereo）。
    pub fn new(n: u16) -> Result<Self, AudioError> {
        if (1..=2).contains(&n) {
            Ok(Self(n))
        } else {
            Err(AudioError::InvalidParameter("channels out of range"))
        }
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// 音频格式：采样率 + 声道数（内部 PCM 一律 f32 交错，见 decoder/pcm）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: SampleRate,
    pub channels: Channels,
}

impl AudioFormat {
    #[must_use]
    pub const fn new(sample_rate: SampleRate, channels: Channels) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }
}

/// 帧数（一帧 = 一个采样点在全部声道上的集合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct FrameCount(pub u64);

/// 数据代（generation）：load / seek 都会递增（M1 §21）。
///
/// 旧 generation 的音频永远不允许到达 output；`u64` 起点为 1。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Generation(u64);

impl Generation {
    /// 首个 generation。
    pub const FIRST: Generation = Generation(1);

    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl From<Generation> for u64 {
    fn from(g: Generation) -> u64 {
        g.0
    }
}

/// 播放位置：权威表示为 frame，秒为派生表示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackPosition {
    frame: u64,
    sample_rate: SampleRate,
}

impl TrackPosition {
    #[must_use]
    pub const fn new(frame: u64, sample_rate: SampleRate) -> Self {
        Self { frame, sample_rate }
    }

    #[must_use]
    pub const fn frame(&self) -> u64 {
        self.frame
    }

    #[must_use]
    pub const fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// 派生表示：秒（仅供 UI/API 展示）。
    #[must_use]
    pub fn position_seconds(&self) -> f64 {
        self.frame as f64 / f64::from(self.sample_rate.get())
    }
}

/// 音量：0.0 ~ 1.0。构造时 clamp、拒绝 NaN/Infinity/负值。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume(f32);

impl Volume {
    /// 全量。
    pub const MAX: Volume = Volume(1.0);
    /// 静音。
    pub const MUTED: Volume = Volume(0.0);

    /// 校验并构造：NaN/Infinity 拒绝；负值拒绝；>1.0 clamp 到 1.0。
    pub fn new(v: f32) -> Result<Self, AudioError> {
        if v.is_nan() {
            return Err(AudioError::InvalidParameter("volume is NaN"));
        }
        if v.is_infinite() {
            return Err(AudioError::InvalidParameter("volume is infinite"));
        }
        if v < 0.0 {
            return Err(AudioError::InvalidParameter("volume is negative"));
        }
        Ok(Self(v.min(1.0)))
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// 播放状态（M1 §16）：仅 Stopped / Paused / Playing。
///
/// 合法转换：Stopped→Playing、Playing→Paused、Paused→Playing、
/// Playing→Stopped、Paused→Stopped。非法操作安全处理、不得 panic。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlaybackState {
    /// 判定从 self 应用 `to` 是否为合法转换；非法转换返回 Err，由调用方安全忽略。
    #[must_use]
    pub const fn can_transition(self, to: PlaybackState) -> bool {
        use PlaybackState::{Paused, Playing, Stopped};
        matches!(
            (self, to),
            (Stopped, Playing)
                | (Playing, Paused)
                | (Paused, Playing)
                | (Playing, Stopped)
                | (Paused, Stopped)
        )
    }
}

/// 播放控制命令（M1 §17）。M1 不实现 queue / playlist。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand {
    Play,
    Pause,
    Stop,
    Seek { frame: u64 },
    SetVolume { volume: f32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_rate_bounds() {
        assert!(SampleRate::new(48_000).is_ok());
        assert!(SampleRate::new(44_100).is_ok());
        assert!(SampleRate::new(0).is_err());
        assert!(SampleRate::new(7_999).is_err());
        assert!(SampleRate::new(384_001).is_err());
    }

    #[test]
    fn channels_bounds() {
        assert!(Channels::new(1).is_ok());
        assert!(Channels::new(2).is_ok());
        assert!(Channels::new(0).is_err());
        assert!(Channels::new(3).is_err());
    }

    #[test]
    fn volume_rejects_non_finite_and_negative_clamps_high() {
        assert!(Volume::new(f32::NAN).is_err());
        assert!(Volume::new(f32::INFINITY).is_err());
        assert!(Volume::new(f32::NEG_INFINITY).is_err());
        assert!(Volume::new(-0.1).is_err());
        assert_eq!(Volume::new(1.5).unwrap().get(), 1.0);
        assert_eq!(Volume::new(0.5).unwrap().get(), 0.5);
        assert_eq!(Volume::MAX.get(), 1.0);
        assert_eq!(Volume::MUTED.get(), 0.0);
    }

    #[test]
    fn generation_increments() {
        let g = Generation::FIRST;
        assert_eq!(g.get(), 1);
        assert_eq!(g.next().get(), 2);
    }

    #[test]
    fn position_seconds_is_derived_from_frames() {
        let sr = SampleRate::new(48_000).unwrap();
        let pos = TrackPosition::new(48_000, sr);
        assert_eq!(pos.frame(), 48_000);
        assert!((pos.position_seconds() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn playback_state_legal_transitions() {
        use PlaybackState::*;
        assert!(Stopped.can_transition(Playing));
        assert!(Playing.can_transition(Paused));
        assert!(Paused.can_transition(Playing));
        assert!(Playing.can_transition(Stopped));
        assert!(Paused.can_transition(Stopped));
        // 非法
        assert!(!Stopped.can_transition(Paused));
        assert!(!Playing.can_transition(Playing));
        assert!(!Stopped.can_transition(Stopped));
    }
}
