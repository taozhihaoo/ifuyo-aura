//! 库操作的统一错误出口（§53）：错误路径无 `unwrap()` panic，
//! 应用层可安全地把任何失败转成用户可见提示。

use std::fmt;
use std::path::PathBuf;

/// 单文件扫描失败的稳定分类（§16：path + category，不携带敏感内容）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    PermissionDenied,
    NotFound,
    /// 打开/读取失败（非权限）。
    Unreadable,
    /// 扩展名可识别但内容不是有效音频（§12 fake.mp3）。
    UnrecognizedFormat,
    MalformedMetadata,
    /// 仅封面不可用；track 仍入库（§31）。
    ArtworkUnavailable,
    PathTooLong,
    /// 文件被占用 / 半写入状态（§46）。
    Busy,
    /// 路径越界（root boundary，§14）。
    OutsideRoot,
    /// reparse point / symlink，按策略跳过（§13）。
    ReparseSkipped,
    Unknown,
}

/// 库层统一错误。
#[derive(Debug)]
pub enum LibraryError {
    /// 文件系统 IO（扫描读取、缩略图缓存写盘等）。
    Io {
        path: Option<PathBuf>,
        source: std::io::Error,
    },
    /// SQLite 打开/查询/事务失败。
    Database(rusqlite::Error),
    /// 数据库文件损坏或 schema 不兼容：应用层应显示异常而非 panic（§53）。
    CorruptDatabase { path: PathBuf, reason: String },
    /// 标签读取失败（单文件级，扫描继续）。
    Metadata { path: PathBuf, reason: String },
    /// 路径在允许 root 之外。
    PathOutsideRoot { path: PathBuf },
    /// 用户取消扫描（§67）。
    Cancelled,
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => match path {
                Some(p) => write!(f, "io error at {}: {source}", p.display()),
                None => write!(f, "io error: {source}"),
            },
            Self::Database(e) => write!(f, "database error: {e}"),
            Self::CorruptDatabase { path, reason } => {
                write!(f, "corrupt database {}: {reason}", path.display())
            }
            Self::Metadata { path, reason } => {
                write!(f, "metadata error at {}: {reason}", path.display())
            }
            Self::PathOutsideRoot { path } => {
                write!(f, "path outside library root: {}", path.display())
            }
            Self::Cancelled => write!(f, "scan cancelled"),
        }
    }
}

impl std::error::Error for LibraryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Database(e) => Some(e),
            _ => None,
        }
    }
}

impl From<rusqlite::Error> for LibraryError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e)
    }
}

impl From<std::io::Error> for LibraryError {
    fn from(source: std::io::Error) -> Self {
        Self::Io { path: None, source }
    }
}

pub type Result<T> = std::result::Result<T, LibraryError>;
