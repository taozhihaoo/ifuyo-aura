//! 直链 HTTP/HTTPS 音频 Source（§25–§29）与解析顺序（§12/§13）。
//!
//! 直链解析探测响应头：
//! - `icy-*` 头（icy-name/icy-metaint/icy-br）→ 电台形态（Live/NonSeekable/Forbidden）；
//! - 否则 direct：`Accept-Ranges: bytes` + `Content-Length` → Seekable+Finite，
//!   否则 NonSeekable+UnknownDuration（§27：不伪造 seek）；
//! - Content-Type 只作提示，最终 codec 由 decoder probing 判定（§26/§77）。

use std::path::Path;

use self::direct_http::ResolvedHeaders;
use crate::error::{Result, SourceError};
use crate::http::HttpClient;
use crate::local;
use crate::model::{
    Attribution, CachePolicy, Resolved, ResolvedLicense, ResolvedStream, SeekCapability,
    SourceKind, SourceMetadata, StreamType,
};
use crate::platform_block::PlatformBlocklist;
use crate::url_policy::redact_url;

/// 解析顺序（§13）：
/// 1. 本地存在的文件路径；
/// 2. URL（scheme 白名单 → 凭据拒绝 → 封锁表 → 私网策略，全部先于网络）；
/// 3. 网络探测（direct vs radio，由响应头判定）；
/// 4. 其余输入 → [`SourceError::UnsupportedInput`]。
///
/// # Errors
/// 见各步骤。
pub fn resolve_input(
    input: &str,
    http: &HttpClient,
    blocklist: &PlatformBlocklist,
) -> Result<Resolved> {
    let trimmed = input.trim();

    // 1. 本地路径（无 URL scheme 且文件真实存在）。
    //    scheme 探测按 WHATWG：字母开头的 [A-Za-z0-9+.-]+ 且长度 >= 2
    //    （单字母 = Windows 盘符，仍按路径处理）。
    if !url_like(trimmed) {
        let path = Path::new(trimmed);
        if path.exists() {
            return local::resolve(path);
        }
        // 带盘符/分隔符但不存在 → 明确的本地路径错误（不是 URL）。
        if is_probably_path(trimmed) {
            return Err(SourceError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            });
        }
        return Err(SourceError::UnsupportedInput {
            reason: "input is neither an existing file nor a URL".into(),
        });
    }

    // 2. URL 校验（scheme/凭据/封锁表/私网 —— 全部先于任何请求，§13 关键）。
    let url = crate::url_policy::validate_url(trimmed, &http.config().policy, blocklist)?;

    // B 级格式在 M4-A 明确不支持（可识别、不实现）。
    let lower = url.path().to_ascii_lowercase();
    if lower.ends_with(".m3u8") || lower.contains(".m3u8?") {
        return Err(SourceError::UnsupportedInput {
            reason: "HLS (m3u8) is a tier-B source, not enabled in this build".into(),
        });
    }

    // 3. 网络探测。
    let probe = direct_http::probe(http, &url)?;
    let streams = vec![stream_from_headers(&url, &probe)];
    let (kind, cache, stream_type) = if probe.is_icy {
        (
            SourceKind::InternetRadio,
            CachePolicy::Forbidden,
            StreamType::Live,
        )
    } else {
        (
            SourceKind::DirectHttp,
            CachePolicy::SessionOnly,
            probe.stream_type,
        )
    };
    let _ = stream_type;

    Ok(Resolved {
        kind,
        streams,
        meta: SourceMetadata {
            stream: crate::model::StreamMeta {
                icy_name: probe.icy_name.clone(),
                ..crate::model::StreamMeta::default()
            },
            ..SourceMetadata::default()
        },
        // §52：无明确证据 → Unknown（不猜 license）。
        license: ResolvedLicense::Unknown,
        cache_policy: cache,
        attribution: Attribution::default(),
    })
}

/// WHATWG 式 scheme 探测。
fn url_like(s: &str) -> bool {
    match s.find(':') {
        Some(colon) if colon >= 2 => {
            let scheme = &s[..colon];
            scheme.starts_with(|c: char| c.is_ascii_alphabetic())
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        }
        _ => false,
    }
}

fn is_probably_path(s: &str) -> bool {
    // Windows 盘符 / 绝对路径分隔符 / 相对路径。
    s.starts_with(['/', '\\'])
        || s.as_bytes().get(1) == Some(&b':')
        || s.starts_with("./")
        || s.starts_with("../")
}

fn stream_from_headers(url: &url::Url, p: &ResolvedHeaders) -> ResolvedStream {
    ResolvedStream {
        url: url.to_string(),
        mime: p.content_type.clone(),
        codec: p.codec_hint.clone(),
        bitrate_bps: p.bitrate_bps,
        sample_rate: None,
        channels: None,
        seekable: p.seekable,
        stream_type: p.stream_type,
    }
}

/// 直链探测实现（direct.rs 的薄封装，保持模块内聚）。
mod direct_http {
    use super::*;

    /// 探测到的响应头摘要。
    #[derive(Debug, Clone)]
    pub struct ResolvedHeaders {
        pub content_type: Option<String>,
        pub codec_hint: Option<String>,
        pub bitrate_bps: Option<u64>,
        pub icy_name: Option<String>,
        pub is_icy: bool,
        pub seekable: SeekCapability,
        pub stream_type: StreamType,
    }

    /// 发起探测请求（消耗一次 GET；流由调用方继续消费或丢弃）。
    ///
    /// # Errors
    /// 网络/状态/重定向策略。
    pub fn probe(http: &HttpClient, url: &url::Url) -> Result<ResolvedHeaders> {
        let resp = http.get_stream(url).map_err(|e| match e {
            SourceError::NetworkError { .. } | SourceError::Timeout => {
                SourceError::SourceUnavailable {
                    reason: crate::error::short(&format!(
                        "connect failed for {}",
                        redact_url(url.as_str())
                    )),
                }
            }
            other => other,
        })?;
        let headers = resp.headers();
        let get = |name: &str| {
            headers
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let content_type = get("content-type");
        let icy_name = get("icy-name");
        let is_icy =
            icy_name.is_some() || get("icy-metaint").is_some() || get("ice-audio-info").is_some();
        let bitrate_bps: Option<u64> = get("icy-br")
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(|kbps| kbps * 1000);
        let accept_ranges =
            get("accept-ranges").is_some_and(|v| v.to_ascii_lowercase().contains("bytes"));
        let content_length = resp.content_length();

        let (seekable, stream_type) = if is_icy {
            (SeekCapability::NonSeekable, StreamType::Live)
        } else if accept_ranges && content_length.is_some() {
            (SeekCapability::Seekable, StreamType::Finite)
        } else {
            (SeekCapability::NonSeekable, StreamType::UnknownDuration)
        };
        let codec_hint = content_type.as_deref().and_then(codec_from_mime);

        Ok(ResolvedHeaders {
            content_type,
            codec_hint,
            bitrate_bps,
            icy_name,
            is_icy,
            seekable,
            stream_type,
        })
    }

    /// Content-Type → codec 提示（仅提示；§77 最终由 decoder probing 决定）。
    #[must_use]
    pub fn codec_from_mime(mime: &str) -> Option<String> {
        let base = mime
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        match base.as_str() {
            "audio/mpeg" => Some("mp3".into()),
            "audio/ogg" | "application/ogg" | "audio/vorbis" => Some("ogg".into()),
            "audio/flac" | "audio/x-flac" => Some("flac".into()),
            "audio/aac" | "audio/aacp" | "audio/mp4" => Some("aac".into()),
            "audio/wav" | "audio/x-wav" | "audio/wave" => Some("wav".into()),
            _ => None,
        }
    }
}
