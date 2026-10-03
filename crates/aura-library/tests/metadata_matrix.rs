//! metadata 读取测试矩阵（§69/§70）：完整/缺失/Unicode/空/畸形标签、
//! 多封面选择策略、invalid file、artwork 不可用不影响 track 解析。

use std::fs;
use std::path::{Path, PathBuf};

use aura_library::error::ErrorCategory;
use aura_library::metadata::{self, sniff_mime};
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::*;
use lofty::tag::{Tag, TagType};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

/// 每测试独立临时目录（原子计数保证唯一）。
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("aura-m3-meta-{tag}-{}-{n}", std::process::id()));
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tag_type_for(ext: &str) -> TagType {
    match ext {
        "mp3" => TagType::Id3v2,
        "wav" => TagType::RiffInfo,
        _ => TagType::VorbisComments,
    }
}

fn copy_with_tags(src_name: &str, dst: &Path, title: Option<&str>, artist: Option<&str>) {
    fs::copy(fixture(src_name), dst).unwrap();
    let ext = src_name.rsplit('.').next().unwrap_or("");
    let mut tagged = lofty::read_from_path(dst).unwrap();
    let mut tag = Tag::new(tag_type_for(ext));
    if let Some(t) = title {
        tag.set_title(t.to_string());
    }
    if let Some(a) = artist {
        tag.set_artist(a.to_string());
    }
    tagged.insert_tag(tag);
    tagged.save_to_path(dst, WriteOptions::default()).unwrap();
}

/// §70 完整标签。
#[test]
fn full_tags_flac() {
    let tmp = TempDir::new("full");
    let p = tmp.path("full.flac");
    fs::copy(fixture("sine_8k.flac"), &p).unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title("Aurora Theme".to_string());
    tag.set_artist("Nova".to_string());
    tag.set_album("Northern Lights".to_string());
    tag.insert_text(ItemKey::AlbumArtist, "Nova Collective".to_string());
    tag.set_genre("Ambient".to_string());
    tag.set_year(2024);
    tag.set_track(3);
    tag.set_track_total(12);
    tag.set_disk(1);
    tag.set_disk_total(2);
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();

    let parsed = metadata::parse_file(&p).unwrap();
    let m = &parsed.metadata;
    assert_eq!(m.title.as_deref(), Some("Aurora Theme"));
    assert_eq!(m.artist.as_deref(), Some("Nova"));
    assert_eq!(m.album.as_deref(), Some("Northern Lights"));
    assert_eq!(m.album_artist.as_deref(), Some("Nova Collective"));
    assert_eq!(m.genre.as_deref(), Some("Ambient"));
    assert_eq!(m.year, Some(2024));
    assert_eq!(m.track_number, Some(3));
    assert_eq!(m.track_total, Some(12));
    assert_eq!(m.disc_number, Some(1));
    assert_eq!(m.disc_total, Some(2));
    assert_eq!(m.codec.as_deref(), Some("flac"));
    assert_eq!(m.container.as_deref(), Some("flac"));
    assert_eq!(m.sample_rate, Some(8000));
    assert_eq!(m.channels, Some(1));
    // 0.25s @ 8kHz = 2000 帧（FLAC 精确）。
    assert_eq!(m.duration_frames, Some(2000));
    assert_eq!(m.audio_sig(), "flac/8000/1/2000");
    assert!(parsed.artwork.is_none());
}

/// §70 缺失标签（只有 title）。
#[test]
fn title_only_tags() {
    let tmp = TempDir::new("only");
    let p = tmp.path("only.mp3");
    copy_with_tags("sine_8k.mp3", &p, Some("Only Title"), None);
    let parsed = metadata::parse_file(&p).unwrap();
    assert_eq!(parsed.metadata.title.as_deref(), Some("Only Title"));
    assert!(parsed.metadata.artist.is_none());
    assert!(parsed.metadata.album.is_none());
    assert!(parsed.metadata.year.is_none());
    assert_eq!(parsed.metadata.codec.as_deref(), Some("mp3"));
    // MP3 时长为比特率估计，只验证量级。
    let d = parsed.metadata.duration_frames.unwrap_or(0);
    assert!((1000..=8000).contains(&d), "mp3 时长应接近 2000 帧: {d}");
}

/// §70 Unicode（中文/变音/emoji）+ §70 空字符串折叠为 None。
#[test]
fn unicode_and_empty_tags() {
    let tmp = TempDir::new("uni");
    let p = tmp.path("uni.flac");
    fs::copy(fixture("sine_8k.flac"), &p).unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title("夜の曲 — Étude 🎵".to_string());
    tag.set_artist("歌手 SINGER".to_string());
    tag.set_genre("   ".to_string()); // 空白 → None
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();

    let parsed = metadata::parse_file(&p).unwrap();
    assert_eq!(parsed.metadata.title.as_deref(), Some("夜の曲 — Étude 🎵"));
    assert_eq!(parsed.metadata.artist.as_deref(), Some("歌手 SINGER"));
    assert_eq!(parsed.metadata.genre, None, "空白折叠为 None");
}

/// §70 多封面：CoverFront 优先（D-040）。
#[test]
fn multiple_artwork_prefers_front_cover() {
    let tmp = TempDir::new("art");
    let p = tmp.path("art.flac");
    fs::copy(fixture("sine_8k.flac"), &p).unwrap();

    // 最小合法 PNG（1×1）与 JPEG 头样本。
    let png = minimal_png();
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00,
    ];

    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title("With Art".to_string());
    // 先放 back（非首选），再放 front → 必须选中 front。
    tag.push_picture(Picture::new_unchecked(
        PictureType::CoverBack,
        Some(MimeType::Jpeg),
        None,
        jpeg.to_vec(),
    ));
    tag.push_picture(Picture::new_unchecked(
        PictureType::CoverFront,
        Some(MimeType::Png),
        None,
        png.clone(),
    ));
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();

    let parsed = metadata::parse_file(&p).unwrap();
    let art = parsed.artwork.expect("封面应存在");
    assert_eq!(art.mime, "image/png", "应选中 front cover（PNG）");
    assert_eq!(art.bytes, png);
    assert_eq!(parsed.artwork_unavailable, None);
}

/// §31/§66：声称是封面但字节非图片 → track 仍解析成功，artwork 不可用。
#[test]
fn corrupt_artwork_does_not_break_track() {
    let tmp = TempDir::new("badart");
    let p = tmp.path("badart.flac");
    fs::copy(fixture("sine_8k.flac"), &p).unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title("Bad Art".to_string());
    tag.push_picture(Picture::new_unchecked(
        PictureType::CoverFront,
        Some(MimeType::Png),
        None,
        b"not an image at all".to_vec(),
    ));
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();

    let parsed = metadata::parse_file(&p).unwrap();
    assert_eq!(parsed.metadata.title.as_deref(), Some("Bad Art"));
    assert!(parsed.artwork.is_none());
    assert_eq!(
        parsed.artwork_unavailable,
        Some(ErrorCategory::ArtworkUnavailable)
    );
}

/// §70 invalid file：文本伪装的 .mp3 → Err，不 panic。
#[test]
fn fake_mp3_rejected() {
    let tmp = TempDir::new("fake");
    let p = tmp.path("fake.mp3");
    fs::write(&p, b"PK\x03\x04 this is a zip, not audio").unwrap();
    let err = metadata::parse_file(&p);
    assert!(err.is_err(), "伪 MP3 必须被拒");
}

/// §70 malformed：截断的 FLAC → 不得 panic（Ok/Err 皆可，取决于截断点）。
#[test]
fn truncated_flac_does_not_panic() {
    let tmp = TempDir::new("trunc");
    let p = tmp.path("trunc.flac");
    let data = fs::read(fixture("sine_8k.flac")).unwrap();
    fs::write(&p, &data[..data.len() / 2]).unwrap();
    let _ = metadata::parse_file(&p);
}

/// MIME 嗅探：字节签名优先。
#[test]
fn mime_sniffing_by_signature() {
    assert_eq!(sniff_mime(&[0xFF, 0xD8, 0xFF, 0x00]), Some("image/jpeg"));
    assert_eq!(
        sniff_mime(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00]),
        Some("image/png")
    );
    assert_eq!(sniff_mime(b"GIF89a...."), None);
    assert_eq!(sniff_mime(&[]), None);
}

/// 最小合法 1×1 PNG（确定性字节）。
fn minimal_png() -> Vec<u8> {
    const PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // 签名
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR 长度+类型
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1×1
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, // 8bit RGBA
        0x89, // IHDR CRC 尾字节
    ];
    // 只需要签名可嗅探；lofty 存取不校验像素。补齐可识别长度。
    PNG.to_vec()
}
