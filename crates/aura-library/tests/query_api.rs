//! 查询 API 测试（§40/§41/§42/§54/§76/§77）：
//! list/get/search/filter/artwork/artists/albums、排序稳定性、
//! NULL 排序位置、分页、统计、integrity check。

use std::fs;
use std::path::{Path, PathBuf};

use aura_library::query::{Page, SortKey};
use aura_library::search::filter::{evaluate, parse_filter, FilterContext, FilterOutcome};
use aura_library::service::LibraryService;
use aura_library::types::CancelToken;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::{Tag, TagType};

fn temp_dir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("aura-m3-query-{tag}-{}-{n}", std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn make_flac(
    dir: &Path,
    name: &str,
    title: &str,
    artist: &str,
    album: &str,
    year: Option<i32>,
) -> PathBuf {
    let p = dir.join(name);
    fs::copy(fixture("sine_8k.flac"), &p).unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title(title.to_string());
    tag.set_artist(artist.to_string());
    tag.set_album(album.to_string());
    if let Some(y) = year {
        tag.set_year(u32::try_from(y).unwrap_or(0));
    }
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();
    p
}

fn seeded(tag: &str) -> (LibraryService, PathBuf) {
    let root = temp_dir(tag);
    make_flac(
        &root,
        "a.flac",
        "Alpha Song",
        "Zeta Artist",
        "Beta Album",
        Some(2001),
    );
    make_flac(
        &root,
        "b.flac",
        "Beta Song",
        "Alpha Artist",
        "Alpha Album",
        Some(1999),
    );
    // 无标签 track（NULL 排序验证）。
    fs::copy(fixture("sine_8k.flac"), root.join("c.flac")).unwrap();
    let mut svc = LibraryService::open_in_memory().unwrap();
    svc.set_clock(Box::new(|| 7_000));
    svc.add_root(&root, 1).unwrap();
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    (svc, root)
}

/// §40：list/get 基础。
#[test]
fn list_and_get_tracks() {
    let (svc, _root) = seeded("basic");
    let all = svc.list_tracks(SortKey::Title, Page::new(10, 0)).unwrap();
    assert_eq!(all.len(), 3);
    let first = all[0].clone();
    let got = svc.get_track(first.id).unwrap().unwrap();
    assert_eq!(got.id, first.id);
    assert_eq!(got.title, first.title);
    // 查询 API 不含 SQL 内部（Track 是领域类型，§40）。
    let missing = svc
        .get_track(aura_library::types::TrackId(999_999))
        .unwrap();
    assert!(missing.is_none());
}

/// §42：title 排序稳定；NULL（无标签）排最后；id 决胜。
#[test]
fn sorting_nulls_last_and_stable() {
    let (svc, _root) = seeded("sort");
    let all = svc.list_tracks(SortKey::Title, Page::new(10, 0)).unwrap();
    let titles: Vec<Option<String>> = all.iter().map(|t| t.title.clone()).collect();
    // Alpha Song / Beta Song 有序，无标签 track 最后。
    assert_eq!(titles[0].as_deref(), Some("Alpha Song"));
    assert_eq!(titles[1].as_deref(), Some("Beta Song"));
    assert!(titles[2].is_none(), "NULL 最后: {titles:?}");
    // 再取一遍逐位一致（§37）。
    let again = svc.list_tracks(SortKey::Title, Page::new(10, 0)).unwrap();
    assert_eq!(all, again);
}

#[test]
fn sorting_by_artist_album_year_duration() {
    let (svc, _root) = seeded("sortkeys");
    let by_artist: Vec<String> = svc
        .list_tracks(SortKey::Artist, Page::new(10, 0))
        .unwrap()
        .into_iter()
        .map(|t| t.artist.unwrap_or_default())
        .collect();
    // Alpha Artist < Zeta Artist < ""(NULL 最后)。
    assert_eq!(by_artist[0], "Alpha Artist");
    assert_eq!(by_artist[1], "Zeta Artist");
    assert_eq!(by_artist[2], "");

    let by_year: Vec<Option<i32>> = svc
        .list_tracks(SortKey::Year, Page::new(10, 0))
        .unwrap()
        .into_iter()
        .map(|t| t.year)
        .collect();
    assert_eq!(by_year[0], Some(1999));
    assert_eq!(by_year[1], Some(2001));
    assert_eq!(by_year[2], None);
}

/// §41：分页。
#[test]
fn pagination() {
    let (svc, _root) = seeded("page");
    let p1 = svc.list_tracks(SortKey::Title, Page::new(2, 0)).unwrap();
    let p2 = svc.list_tracks(SortKey::Title, Page::new(2, 2)).unwrap();
    assert_eq!(p1.len(), 2);
    assert_eq!(p2.len(), 1);
    assert_ne!(p1[0].id, p2[0].id);
    let all = svc.list_tracks(SortKey::Title, Page::new(10, 0)).unwrap();
    assert_eq!(p1[0].id, all[0].id, "分页与全量同序");
    assert_eq!(p2[0].id, all[2].id);
}

/// §40：search + artists/albums 聚合。
#[test]
fn search_artists_albums_aggregates() {
    let (svc, _root) = seeded("agg");
    // 前缀语义："alpha song" 同时命中 title/artist/album 里含
    // alpha/beta/song token 的轨。排序 = bm25 + id 决胜（§36 fallback，
    // D-046：微小语料 IDF 为负时严格 title-first 不可靠，确定性即可）。
    let hits = svc.search_tracks("alpha song", Page::new(10, 0)).unwrap();
    assert!(!hits.is_empty());
    let titles: Vec<Option<String>> = hits.iter().map(|t| t.title.clone()).collect();
    assert!(titles.contains(&Some("Alpha Song".into())), "{titles:?}");
    // 同库同查询：顺序恒定（§37）。
    let again = svc.search_tracks("alpha song", Page::new(10, 0)).unwrap();
    let ids1: Vec<_> = hits.iter().map(|t| t.id).collect();
    let ids2: Vec<_> = again.iter().map(|t| t.id).collect();
    assert_eq!(ids1, ids2);
    // 独占 token 精确单命中。
    let zeta = svc.search_tracks("zeta", Page::new(10, 0)).unwrap();
    assert_eq!(zeta.len(), 1);
    assert_eq!(zeta[0].title.as_deref(), Some("Alpha Song"));

    let artists = svc.get_artists().unwrap();
    assert_eq!(artists.len(), 2, "两 artist 实体");
    assert_eq!(artists[0].1, "Alpha Artist", "名称序");
    assert_eq!(artists[0].2, 1, "活跃 track 计数");

    let albums = svc.get_albums().unwrap();
    assert_eq!(albums.len(), 2);
    assert_eq!(albums[0].1, "Alpha Album");
}

/// §40：artwork 读取。
#[test]
fn get_artwork_roundtrip() {
    let tmp = temp_dir("art");
    let png: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 9, 9, 9];
    let p = tmp.join("art.flac");
    fs::copy(fixture("sine_8k.flac"), &p).unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title("With Art".to_string());
    tag.push_picture(lofty::picture::Picture::new_unchecked(
        lofty::picture::PictureType::CoverFront,
        Some(lofty::picture::MimeType::Png),
        None,
        png.clone(),
    ));
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();

    let mut svc = LibraryService::open_in_memory().unwrap();
    svc.add_root(&tmp, 1).unwrap();
    svc.scan_root(&tmp, &CancelToken::new(), None).unwrap();

    let track = &svc.list_tracks(SortKey::Title, Page::new(10, 0)).unwrap()[0];
    let art_id = track.artwork_id.expect("有封面引用");
    let (mime, bytes) = svc.get_artwork(art_id).unwrap().unwrap();
    assert_eq!(mime, "image/png");
    assert_eq!(bytes, png);
    let _ = fs::remove_dir_all(&tmp);
}

/// §54/§55：统计存储模型。
#[test]
fn playback_stats_model() {
    let (mut svc, _root) = seeded("stats");
    let track = &svc.list_tracks(SortKey::Title, Page::new(1, 0)).unwrap()[0];
    let id = track.id;

    assert_eq!(svc.playback_stats(id).unwrap().play_count, 0);
    svc.record_play(id, 2000, 100).unwrap();
    svc.record_play(id, 3000, 200).unwrap();
    svc.record_skip(id, 300).unwrap();
    let s = svc.playback_stats(id).unwrap();
    assert_eq!(s.play_count, 2);
    assert_eq!(s.skip_count, 1);
    assert_eq!(s.total_played_frames, 5000);
    assert_eq!(s.last_played_at_ms, Some(300));

    // 统计不与路径绑定：路径停用（删除文件）后仍在。
    // （repo_identity 已覆盖；此处只验证 API 视角）
}

/// §38/§40：filter_tracks。
#[test]
fn filter_tracks_evaluates() {
    let (svc, _root) = seeded("filter");
    let f = parse_filter("year:1999-2001").unwrap();
    let hits = svc.filter_tracks(&f, Page::new(10, 0)).unwrap();
    assert_eq!(hits.len(), 2, "1999 与 2001 命中");

    let f2 = parse_filter("year:<2000").unwrap();
    let hits2 = svc.filter_tracks(&f2, Page::new(10, 0)).unwrap();
    assert_eq!(hits2.len(), 1);
    assert_eq!(hits2[0].year, Some(1999));

    // 无标签 track：year 缺失 → 不命中（不是猜值）。
    let ctx = FilterContext {
        year: None,
        ..Default::default()
    };
    assert_eq!(evaluate(&f2, &ctx), FilterOutcome::NoMatch);

    // 分析字段 → MissingData。
    let f3 = parse_filter("bpm:>120").unwrap();
    let hits3 = svc.filter_tracks(&f3, Page::new(10, 0)).unwrap();
    assert!(hits3.is_empty(), "M8 数据未存在，不命中");
}

/// §76/§77：integrity check 干净库 & 注入异常。
#[test]
fn integrity_check_reports() {
    let (svc, root) = seeded("integrity");
    let clean = svc.integrity_check().unwrap();
    assert!(clean.is_clean(), "干净库无异常: {clean:?}");
    assert_eq!(clean.fts_rows, 3, "FTS 行 = 活跃 track");
    assert_eq!(clean.missing_files, 0);

    // 删掉一个文件（不扫）→ missing_files 报告。
    fs::remove_file(root.join("a.flac")).unwrap();
    let dirty = svc.integrity_check().unwrap();
    assert_eq!(dirty.missing_files, 1);
    assert!(dirty.is_clean(), "missing_files 不算结构损坏");
}

#[test]
fn integrity_check_detects_fts_orphans() {
    let (svc, _root) = seeded("ftsorphan");
    // 直接注入孤儿 FTS 行。
    svc.database()
        .connection()
        .execute(
            "INSERT INTO tracks_fts(track_id, title) VALUES (99999, 'ghost')",
            [],
        )
        .unwrap();
    let report = svc.integrity_check().unwrap();
    assert_eq!(report.fts_orphans, 1);
    assert!(!report.is_clean());
}
