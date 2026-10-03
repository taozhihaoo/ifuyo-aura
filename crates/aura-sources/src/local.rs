//! 本地文件 Source（§23/§24）：metadata 复用 aura-library，不重写 tag 解析。

use std::path::Path;

use aura_library::metadata as lib_metadata;
use aura_library::scanner;

use crate::error::{Result, SourceError};
use crate::model::{
    Attribution, CachePolicy, Resolved, ResolvedLicense, ResolvedStream, SeekCapability,
    SourceKind, SourceMetadata, StreamType, TrackMeta,
};

/// 解析本地音频文件（用户主动选择的路径；§62 不做主动扫描）。
///
/// # Errors
/// 文件不存在 / 扩展名不支持 / 标签解析失败 → [`SourceError`]。
pub fn resolve(path: &Path) -> Result<Resolved> {
    if !path.is_file() {
        return Err(SourceError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file does not exist"),
        });
    }
    if !scanner::is_supported(path) {
        return Err(SourceError::UnsupportedInput {
            reason: format!(
                "extension {:?} is not a supported audio format",
                path.extension().and_then(|e| e.to_str()).unwrap_or("")
            ),
        });
    }
    let parsed = lib_metadata::parse_file(path).map_err(|e| match e {
        aura_library::error::LibraryError::Metadata { reason, .. } => {
            SourceError::ParseError { reason }
        }
        aura_library::error::LibraryError::Io { source, .. } => SourceError::Io {
            path: path.to_path_buf(),
            source,
        },
        other => SourceError::ParseError {
            reason: crate::error::short(&other.to_string()),
        },
    })?;
    let m = parsed.metadata;

    Ok(Resolved {
        kind: SourceKind::Local,
        streams: vec![ResolvedStream {
            url: path.to_string_lossy().into_owned(),
            mime: None,
            codec: m.codec.clone(),
            bitrate_bps: None,
            sample_rate: m.sample_rate,
            channels: m.channels,
            seekable: SeekCapability::Seekable,
            stream_type: StreamType::Finite,
        }],
        meta: SourceMetadata {
            track: TrackMeta {
                title: m.title.clone(),
                artist: m.artist.clone(),
                album: m.album.clone(),
                duration_seconds: m
                    .duration_frames
                    .map(|f| f.div_ceil(u64::from(m.sample_rate.unwrap_or(1))).max(1)),
            },
            ..SourceMetadata::default()
        },
        // §23/§94：本地 = Allowed（M4 不建 M5 cache，只声明政策）。
        license: ResolvedLicense::Unknown,
        cache_policy: CachePolicy::Allowed,
        attribution: Attribution::default(),
    })
}
