//! metadata 读取（lofty）+ 规范化（§11/§22/§23/§27/§31/§66）。
//!
//! 失败语义：文件级 metadata 失败返回 `Err`（扫描记 `failed`，不中断）；
//! 封面失败只记 `ArtworkUnavailable`，track 照常入库（§31）。

use std::borrow::Cow;
use std::path::Path;

use lofty::file::{FileType, TaggedFile};
use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::properties::FileProperties;
use lofty::tag::ItemKey;

use crate::error::{ErrorCategory, LibraryError, Result};
use crate::normalize;
use crate::types::{Artwork, TrackMetadata};

/// 单张封面字节上限（§66 恶意图防护）。
pub const MAX_ARTWORK_BYTES: usize = 8 << 20;

/// 解析产物：规范化 metadata + 首选封面。
#[derive(Debug, Clone)]
pub struct ParsedFile {
    pub metadata: TrackMetadata,
    /// 按 §27 选择策略取出的封面（front cover 优先）。
    pub artwork: Option<Artwork>,
    /// 封面不可用的原因（`artwork` 为 `None` 且此为 `Some` 时）。
    pub artwork_unavailable: Option<ErrorCategory>,
}

/// 读取并解析一个音频文件。
///
/// # Errors
/// 打不开/不可识别为音频/属性读取失败 → [`LibraryError::Metadata`]
/// （调用方按单文件失败处理，扫描继续）。
pub fn parse_file(path: &Path) -> Result<ParsedFile> {
    let tagged = lofty::read_from_path(path).map_err(|e| LibraryError::Metadata {
        path: path.to_path_buf(),
        reason: short_reason(&e.to_string()),
    })?;
    Ok(from_tagged(&tagged))
}

/// 从已读取的 `TaggedFile` 提取（供测试复用）。
#[must_use]
pub fn from_tagged(tagged: &TaggedFile) -> ParsedFile {
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let mut m = TrackMetadata::default();
    if let Some(tag) = tag {
        m.title = opt_display(tag.title());
        m.artist = opt_display(tag.artist());
        m.album = opt_display(tag.album());
        m.genre = opt_display(tag.genre());
        m.album_artist = opt_display(
            tag.get_string(&ItemKey::AlbumArtist)
                .map(std::borrow::Cow::Borrowed),
        );
        m.year = tag
            .year()
            .and_then(|y| normalize::parse_year(&y.to_string()));
        m.track_number = non_zero(tag.track());
        m.track_total = non_zero(tag.track_total());
        m.disc_number = non_zero(tag.disk());
        m.disc_total = non_zero(tag.disk_total());
    }

    let props: &FileProperties = tagged.properties();
    m.duration_frames = Some(
        u64::try_from(props.duration().as_millis())
            .unwrap_or(0)
            .saturating_mul(u64::from(props.sample_rate().unwrap_or(0)))
            / 1000,
    );
    m.sample_rate = props.sample_rate();
    m.channels = props.channels().map(u16::from);
    let (codec, container) = codec_container(tagged.file_type());
    m.codec = Some(codec);
    m.container = Some(container);

    let artwork = tag.and_then(select_picture).and_then(|p| {
        let bytes = p.data().to_vec();
        match sniff_mime(&bytes) {
            Some(mime) if bytes.len() <= MAX_ARTWORK_BYTES => Some(Artwork { bytes, mime }),
            Some(_) => None, // 超限：跳过（§66）
            None => None,    // 非 JPEG/PNG：跳过
        }
    });
    let artwork_unavailable = if artwork.is_none() && tag.is_some_and(|t| !t.pictures().is_empty())
    {
        Some(ErrorCategory::ArtworkUnavailable)
    } else {
        None
    };

    ParsedFile {
        metadata: m,
        artwork,
        artwork_unavailable,
    }
}

/// 封面选择策略（§27，D-040）：`CoverFront` > 其余按声明顺序第一个。
fn select_picture(tag: &lofty::tag::Tag) -> Option<&lofty::picture::Picture> {
    let pics = tag.pictures();
    pics.iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pics.first())
}

/// 字节签名判 MIME（不信任标签声明）。
#[must_use]
pub fn sniff_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 3 && bytes[..3] == [0xFF, 0xD8, 0xFF] {
        Some("image/jpeg")
    } else if bytes.len() >= 8 && bytes[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        Some("image/png")
    } else {
        None
    }
}

/// codec/container 短名（§11：m4a 不假定 AAC；MP4 族记 `mp4_audio`，
/// 精确 codec 由播放层按 M2 policy 判定）。
#[must_use]
pub fn codec_container(ft: FileType) -> (String, String) {
    match ft {
        FileType::Flac => ("flac".into(), "flac".into()),
        FileType::Mpeg => ("mp3".into(), "mpeg".into()),
        FileType::Vorbis => ("vorbis".into(), "ogg".into()),
        FileType::Opus => ("opus".into(), "ogg".into()),
        FileType::Speex => ("speex".into(), "ogg".into()),
        FileType::Wav => ("pcm".into(), "riff".into()),
        FileType::Aiff => ("pcm".into(), "aiff".into()),
        FileType::Mp4 => ("mp4_audio".into(), "mp4".into()),
        _ => ("unknown".into(), "unknown".into()),
    }
}

fn opt_display(raw: Option<Cow<'_, str>>) -> Option<String> {
    raw.as_ref().and_then(|s| normalize::display_text(s))
}

fn non_zero(v: Option<u32>) -> Option<u32> {
    v.filter(|&n| n > 0)
}

fn short_reason(msg: &str) -> String {
    // 只保留首行 + 截断，避免整个错误 blob 进库/日志（§80）。
    let first = msg.lines().next().unwrap_or("unknown");
    let mut s: String = first.chars().take(120).collect();
    if first.chars().count() > 120 {
        s.push('…');
    }
    s
}
