//! Source 层统一错误（§11）：可诊断、可显示、不 panic、不泄漏凭据。

use std::fmt;

/// 来源系统错误。`Display` 输出面向用户可读，不含 URL query/凭据。
#[derive(Debug)]
pub enum SourceError {
    /// 输入无法识别为任何已知来源形式（§12：未知输入明确报错）。
    UnsupportedInput { reason: String },
    /// 命中 `unsupported_platforms.json`：在发出任何请求之前拒绝（§3）。
    UnsupportedPlatform { platform: String, message: String },
    /// URL 解析/校验失败（scheme/authority/credentials，§14/§15）。
    InvalidUrl { reason: String },
    /// 响应不符合预期（状态码/头/内容类型，§26）。
    InvalidResponse { reason: String },
    /// 网络层失败（连接/中断/DNS，§49：不静默换服务）。
    NetworkError { reason: String },
    /// 连接或读取超时（§19）。
    Timeout,
    /// 超过重定向上限或重定向策略拒绝（§16）。
    RedirectLimit,
    /// 响应体超过上限（§21）。
    BodyTooLarge { limit_bytes: u64 },
    /// 私网/本地地址被策略拒绝（§17）。
    PrivateNetworkBlocked { host: String },
    /// codec 不支持（§46/§78：不 panic、不自动换源）。
    UnsupportedCodec { codec: String },
    /// 受保护流（DRM/加密 HLS/SAMPLE-AES 等，§68：立即拒绝）。
    ProtectedStream,
    /// 许可未知（§52：不猜测具体 license）。
    LicenseUnknown,
    /// 来源不可用（DNS/镜像全失败，§49）。
    SourceUnavailable { reason: String },
    /// 协议解析失败（确定性 parser 返回，§80）。
    ParseError { reason: String },
    /// 用户取消（§22）。
    Cancelled,
    /// 本地文件 IO（Local Source）。
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedInput { reason } => write!(f, "unsupported input: {reason}"),
            Self::UnsupportedPlatform { platform, message } => {
                write!(f, "platform not supported ({platform}): {message}")
            }
            Self::InvalidUrl { reason } => write!(f, "invalid URL: {reason}"),
            Self::InvalidResponse { reason } => write!(f, "invalid response: {reason}"),
            Self::NetworkError { reason } => write!(f, "network error: {reason}"),
            Self::Timeout => write!(f, "network timeout"),
            Self::RedirectLimit => write!(f, "redirect limit exceeded or redirect denied"),
            Self::BodyTooLarge { limit_bytes } => {
                write!(f, "response body exceeds limit ({limit_bytes} bytes)")
            }
            Self::PrivateNetworkBlocked { host } => {
                write!(f, "private/local network address blocked by policy: {host}")
            }
            Self::UnsupportedCodec { codec } => {
                write!(f, "audio codec not supported: {codec}")
            }
            Self::ProtectedStream => write!(f, "protected/encrypted streams are not supported"),
            Self::LicenseUnknown => write!(f, "source license unknown"),
            Self::SourceUnavailable { reason } => write!(f, "source unavailable: {reason}"),
            Self::ParseError { reason } => write!(f, "parse error: {reason}"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Io { path, source } => write!(f, "io error at {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SourceError {
    fn from(source: std::io::Error) -> Self {
        if matches!(
            source.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ) {
            return Self::Timeout;
        }
        Self::NetworkError {
            reason: source.to_string(),
        }
    }
}

impl From<reqwest::Error> for SourceError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            return Self::Timeout;
        }
        if e.is_redirect() {
            return Self::RedirectLimit;
        }
        if e.is_decode() {
            return Self::ParseError {
                reason: short(&e.to_string()),
            };
        }
        // 重定向策略拒绝时，reqwest 把我们的 SourceError 挂在 source 链上 —— 还原类别。
        let mut src = std::error::Error::source(&e);
        while let Some(s) = src {
            if let Some(se) = s.downcast_ref::<SourceError>() {
                return clone_kind(se);
            }
            src = s.source();
        }
        Self::NetworkError {
            reason: short(&e.to_string()),
        }
    }
}

fn clone_kind(e: &SourceError) -> SourceError {
    match e {
        SourceError::RedirectLimit => SourceError::RedirectLimit,
        SourceError::PrivateNetworkBlocked { host } => {
            SourceError::PrivateNetworkBlocked { host: host.clone() }
        }
        SourceError::InvalidUrl { reason } => SourceError::InvalidUrl {
            reason: reason.clone(),
        },
        SourceError::UnsupportedPlatform { platform, message } => {
            SourceError::UnsupportedPlatform {
                platform: platform.clone(),
                message: message.clone(),
            }
        }
        other => SourceError::NetworkError {
            reason: short(&other.to_string()),
        },
    }
}

/// 错误消息只保留首行 + 截断（§96：不泄漏完整响应/URL）。
pub(crate) fn short(msg: &str) -> String {
    let first = msg.lines().next().unwrap_or("unknown");
    let mut s: String = first.chars().take(120).collect();
    if first.chars().count() > 120 {
        s.push('…');
    }
    s
}

pub type Result<T> = std::result::Result<T, SourceError>;
