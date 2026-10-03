//! ifuyo Aura 音频内核：decode → resample → lock-free SPSC ring buffer → output。
//!
//! 边界约束（ARCHITECTURE.md / M1 §4）：
//! - 不得依赖 Tauri、UI、filesystem、network；
//! - 播放位置以 `consumed_frames / sample_rate` 为准，不用 wall clock；
//! - audio callback 内禁止 allocation / lock / logging / syscall / blocking / IO / panic。
//!
//! M1 实装范围：解码（FLAC 真实路径 + synthetic source）、重采样、SPSC ring buffer、
//! 基础播放时钟/位置/状态机、seek、generation 陈旧样本拒绝、音量与 10ms 平滑、
//! underrun 处理、错误模型。M2+（gapless/crossfade/loudness/EQ）一律未实现。

pub mod audio;
pub mod decoder;
pub mod engine;
pub mod error;
pub mod output;
pub mod resampler;
pub mod types;

pub use audio::buffer::{ring_buffer, ConsumerHandle, ProducerHandle};
pub use audio::gain::{GainSmoother, SMOOTHING_WINDOW_MS};
pub use audio::position::PositionTracker;
pub use decoder::flac::FlacDecoder;
pub use decoder::pcm::{map_channels, map_channels_into, PcmBuffer};
pub use decoder::Decoder;
pub use engine::command::{AudioCommandQueue, CommandQueueError};
pub use engine::player::Engine;
pub use error::AudioError;
pub use output::null::NullOutput;
pub use output::{AudioOutput, OutputStage, SharedOutputState};
pub use resampler::Resampler;
pub use types::{
    AudioCommand, AudioFormat, Channels, FrameCount, Generation, PlaybackState, SampleRate,
    TrackPosition, Volume,
};
