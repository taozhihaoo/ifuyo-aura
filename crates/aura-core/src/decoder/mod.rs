//! Decoder abstraction（M1 §8–9）。
//!
//! 职责边界：**compressed audio → decoded PCM frames**。
//! 不负责 output device / UI / playback clock / seek policy / volume / caching。
//!
//! - [`Decoder`] trait：统一解码接口；
//! - [`FlacDecoder`]：symphonia FLAC 真实解码路径（M1 第一格式）；
//! - 内部 PCM 为交错 f32（源格式），声道映射与重采样由后续阶段处理。

pub mod flac;
pub mod pcm;
pub mod streaming;
pub mod sub_range;
pub use sub_range::SubRangeDecoder;

pub use streaming::{AudioByteStream, StreamingDecoder};

use crate::error::AudioError;
use crate::types::AudioFormat;

pub use pcm::PcmBuffer;

/// 解码器抽象。数据来源由构造方决定（内存字节 / 上游流），本层不做 IO。
pub trait Decoder: Send {
    /// 源格式（解码输出的采样率/声道）。
    fn format(&self) -> AudioFormat;

    /// 总帧数（若容器/编码可给出）。
    fn duration_frames(&self) -> Option<u64>;

    /// 解码下一批帧，追加到 `out`（源格式交错 f32），返回追加帧数。
    /// 返回 0 表示流结束（EOF）。`out` 由调用方复用，本实现只追加。
    fn decode_next(&mut self, out: &mut PcmBuffer) -> Result<u64, AudioError>;

    /// seek 到目标帧（源时间基）。实现必须保证后续 `decode_next`
    /// 返回的第一帧就是目标帧或之后的第一帧。
    fn seek_to_frame(&mut self, frame: u64) -> Result<(), AudioError>;
}
