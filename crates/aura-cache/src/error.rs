//! Cache 错误模型（§123）。

use std::fmt;

/// 缓存层统一错误。可诊断、可显示、不 panic、不含 secret。
#[derive(Debug)]
pub enum CacheError {
    /// 目录/文件权限不足（§125：graceful degrade，不无限重试）。
    PermissionDenied,
    /// 磁盘满（§124：明确报错，不损坏已有 chunks，不动 pinned）。
    DiskFull,
    /// 数据损坏（hash/size 不一致，§24：丢弃重取）。
    Corrupt { what: String },
    /// SQLite 失败（§127：bounded busy timeout 内仍失败）。
    Database(rusqlite::Error),
    /// 网络失败（含离线；§80：不得伪装 EOS）。
    Network { reason: String },
    /// 服务器不支持 Range（§35：转 sequential fill）。
    RangeUnsupported,
    /// 源内容已变化（ETag/Last-Modified/长度不一致，§43）。
    SourceChanged { reason: String },
    /// 全部 pinned 无法腾出空间（§59：不删 pinned）。
    CacheFullPinned,
    /// 用户取消（§94）。
    Cancelled,
    /// 非法 content key / 路径注入企图（§116）。
    InvalidKey { reason: String },
    /// 策略禁止该操作（Forbidden 写持久层等，§3/§6）。
    PolicyViolation { reason: String },
    /// 超时。
    Timeout,
    /// IO 其他失败。
    Io(std::io::Error),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "cache permission denied"),
            Self::DiskFull => write!(f, "disk full"),
            Self::Corrupt { what } => write!(f, "cache corrupt: {what}"),
            Self::Database(e) => write!(f, "cache database error: {e}"),
            Self::Network { reason } => write!(f, "network error: {reason}"),
            Self::RangeUnsupported => write!(f, "server does not support range requests"),
            Self::SourceChanged { reason } => write!(f, "source content changed: {reason}"),
            Self::CacheFullPinned => write!(f, "cache full and all entries pinned"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::InvalidKey { reason } => write!(f, "invalid cache key: {reason}"),
            Self::PolicyViolation { reason } => write!(f, "cache policy violation: {reason}"),
            Self::Timeout => write!(f, "timeout"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for CacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for CacheError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e)
    }
}

impl From<std::io::Error> for CacheError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::StorageFull => Self::DiskFull,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Self::Timeout,
            std::io::ErrorKind::Interrupted => Self::Cancelled,
            _ => Self::Io(e),
        }
    }
}

/// io::Error → CacheError 的语义映射（存储层写失败分类，§124）。
#[must_use]
pub fn classify_io(e: &std::io::Error) -> CacheError {
    CacheError::from(std::io::Error::new(e.kind(), e.to_string()))
}

pub type Result<T> = std::result::Result<T, CacheError>;
