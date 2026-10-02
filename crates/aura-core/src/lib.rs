//! ifuyo Aura 音频内核：decode → resample → lock-free SPSC ring buffer → output。
//!
//! 边界约束（ARCHITECTURE.md / M1 §4）：
//! - 不得依赖 Tauri、UI、filesystem、network；
//! - 播放位置以 `consumed_frames / sample_rate` 为准，不用 wall clock；
//! - audio callback 内禁止 allocation / lock / logging / syscall / blocking / IO / panic。
//!
//! M1 实装中（子任务推进）。

pub mod audio;
pub mod error;
pub mod types;

pub use audio::position::PositionTracker;
pub use error::AudioError;
pub use types::{
    AudioCommand, AudioFormat, Channels, FrameCount, Generation, PlaybackState, SampleRate,
    TrackPosition, Volume,
};
