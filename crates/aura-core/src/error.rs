//! 音频内核错误模型（M1 §33）。
//!
//! 约束：可诊断、不允许 panic 路径依赖、不携带敏感数据（不含路径密钥等）。

use std::fmt;

/// 音频内核统一错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioError {
    /// 输入音频格式无效或无法识别。
    InvalidFormat(&'static str),
    /// 解码过程失败。
    DecodeError(&'static str),
    /// 格式不支持（M1 仅 FLAC 走真实解码，其余格式返回此错误）。
    UnsupportedFormat(&'static str),
    /// 重采样失败或参数无效。
    ResampleError(&'static str),
    /// 输出设备/输出流错误。
    OutputError(&'static str),
    /// seek 失败（目标越界、解码器拒绝等）。
    SeekError(&'static str),
    /// 命令在当前状态下非法（已按安全策略处理，此处用于显式上报）。
    InvalidCommand(&'static str),
    /// 参数非法（NaN/Inf/越界等）。
    InvalidParameter(&'static str),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioError::InvalidFormat(m) => write!(f, "invalid format: {m}"),
            AudioError::DecodeError(m) => write!(f, "decode error: {m}"),
            AudioError::UnsupportedFormat(m) => write!(f, "unsupported format: {m}"),
            AudioError::ResampleError(m) => write!(f, "resample error: {m}"),
            AudioError::OutputError(m) => write!(f, "output error: {m}"),
            AudioError::SeekError(m) => write!(f, "seek error: {m}"),
            AudioError::InvalidCommand(m) => write!(f, "invalid command: {m}"),
            AudioError::InvalidParameter(m) => write!(f, "invalid parameter: {m}"),
        }
    }
}

impl std::error::Error for AudioError {}
