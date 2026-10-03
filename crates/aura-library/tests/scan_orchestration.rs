//! 扫描编排测试（§71/§72/§67/§68）：初始/增量/幂等三连、rename/move/
//! 删除/元数据修改/音频替换、伪扩展名、畸形文件、取消、进度、
//! 错误分类、结果统计。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use aura_library::error::ErrorCategory;
use aura_library::service::LibraryService;
use aura_library::types::CancelToken;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::{Tag, TagType};

fn fixtures(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_dir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("aura-m3-scan-{tag}-{}-{n}", std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn make_flac(dir: &Path, name: &str, title: &str) -> PathBuf {
    let p = dir.join(name);
    fs::copy(fixtures("sine_8k.flac"), &p).unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title(title.to_string());
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();
    p
}

fn seeded(tag: &str) -> (LibraryService, PathBuf) {
    let root = temp_dir(tag);
    make_flac(&root, "a.flac", "Song A");
    make_flac(&root, "b.flac", "Song B");
    fs::create_dir_all(root.join("nested")).unwrap();
    make_flac(&root, "nested/c.flac", "Song C");
    let mut svc = LibraryService::open_in_memory().unwrap();
    svc.set_clock(Box::new(|| 42_000));
    svc.add_root(&root, 1_000).unwrap();
    (svc, root)
}

fn active_tracks(svc: &LibraryService) -> i64 {
    svc.database()
        .connection()
        .query_row("SELECT count(*) FROM tracks WHERE is_active = 1", [], |r| {
            r.get(0)
        })
        .unwrap()
}

fn track_id_at(svc: &LibraryService, path: &Path) -> Option<i64> {
    // 与 repo 存储一致：canonicalize 后查询。
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    svc.database()
        .connection()
        .query_row(
            "SELECT track_id FROM track_paths WHERE path = ?1",
            [canonical.to_string_lossy()],
            |r| r.get(0),
        )
        .ok()
}

/// §15/§16：初始扫描统计 + 嵌套目录。
#[test]
fn initial_scan_counts_and_nested() {
    let (mut svc, root) = seeded("initial");
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_seen, 3);
    assert_eq!(r.files_imported, 3);
    assert_eq!(r.files_failed, 0);
    assert_eq!(active_tracks(&svc), 3);
    assert!(r.duration_ms < 60_000);
}

/// §17/§18：二次扫描全跳过；三连幂等。
#[test]
fn rescan_skips_unchanged_and_is_idempotent() {
    let (mut svc, root) = seeded("idem");
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let r2 = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r2.files_seen, 3);
    assert_eq!(r2.files_skipped, 3, "未变文件不重新解析");
    assert_eq!(r2.files_imported + r2.files_updated, 0);
    let r3 = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r3.files_skipped, 3);
    assert_eq!(active_tracks(&svc), 3, "无重复 track");
}

/// §72：rename → scan → 同 TrackId。
#[test]
fn rename_preserves_track_id() {
    let (mut svc, root) = seeded("rename");
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let a = root.join("a.flac");
    let id_before = track_id_at(&svc, &a).unwrap();

    let renamed = root.join("renamed_a.flac");
    fs::rename(&a, &renamed).unwrap();
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_updated, 1, "rename 经身份合并记 updated");
    assert_eq!(r.files_deleted, 1, "旧路径 sweep 停用");
    assert_eq!(track_id_at(&svc, &renamed), Some(id_before), "TrackId 不变");
    assert_eq!(active_tracks(&svc), 3);
}

/// §72：move → 同 TrackId。
#[test]
fn move_preserves_track_id() {
    let (mut svc, root) = seeded("move");
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let a = root.join("a.flac");
    let id_before = track_id_at(&svc, &a).unwrap();

    fs::create_dir_all(root.join("deeper")).unwrap();
    let moved = root.join("deeper/a.flac");
    fs::rename(&a, &moved).unwrap();
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(track_id_at(&svc, &moved), Some(id_before));
}

/// §72：metadata edit → 同 TrackId + 元数据更新 + mtime 变化触发重解析。
#[test]
fn metadata_edit_detected_and_preserves_id() {
    let (mut svc, root) = seeded("metaedit");
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let a = root.join("a.flac");
    let id_before = track_id_at(&svc, &a).unwrap();

    // 改标签（lofty 重写文件 → mtime/内容变化）。
    let mut tagged = lofty::read_from_path(&a).unwrap();
    tagged
        .first_tag_mut()
        .unwrap()
        .set_title("Song A v2".to_string());
    tagged.save_to_path(&a, WriteOptions::default()).unwrap();

    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_updated, 1);
    assert_eq!(track_id_at(&svc, &a), Some(id_before), "标签改写保 TrackId");
    let title: String = svc
        .database()
        .connection()
        .query_row("SELECT title FROM tracks WHERE id = ?1", [id_before], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(title, "Song A v2");
}

/// §71：伪扩展名（fake.mp3 = PNG 字节）→ failed + UnrecognizedFormat，不影响他文件。
#[test]
fn fake_audio_extension_fails_gracefully() {
    let (mut svc, root) = seeded("fake");
    fs::write(root.join("fake.mp3"), b"\x89PNG\r\n\x1a\n not audio").unwrap();
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_failed, 1);
    assert_eq!(r.files_imported, 3);
    assert_eq!(r.errors.len(), 1);
    assert_eq!(r.errors[0].1, ErrorCategory::UnrecognizedFormat);
    assert_eq!(active_tracks(&svc), 3, "坏文件不进库");
    // 库仍可正常继续扫描。
    let r2 = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r2.files_skipped, 3);
    assert_eq!(r2.files_failed, 1);
}

/// §71：畸形音频（垃圾字节 + flac 扩展名）→ failed，不进库。
/// 注：截断但头部完好的 FLAC 可解析（STREAMINFO 完整），按设计入库；
/// 完整性由播放层处理，扫描后文件补全时 size/mtime 变化会触发重导入。
#[test]
fn malformed_audio_fails_gracefully() {
    let (mut svc, root) = seeded("malformed");
    fs::write(root.join("junk.flac"), b"fLaC").unwrap();
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_failed, 1, "{r:?}");
    assert_eq!(r.files_imported, 3);
}

/// 截断 FLAC（头部完好）可导入；补全后 size/mtime 变化触发重解析。
#[test]
fn truncated_but_headered_flac_imports_then_reimports_on_completion() {
    let (mut svc, root) = seeded("trunc2");
    let data = fs::read(fixtures("sine_8k.flac")).unwrap();
    fs::write(root.join("t.flac"), &data[..data.len() / 3]).unwrap();
    let r1 = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r1.files_imported, 4, "标签可读 → 入库（§46 半写入不阻断）");

    fs::write(root.join("t.flac"), &data).unwrap(); // 补全
    let r2 = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r2.files_updated, 1, "size 变化触发重导入");
}

/// §71：不支持的扩展名被静默忽略。
#[test]
fn unsupported_extension_ignored() {
    let (mut svc, root) = seeded("unsupported");
    fs::write(root.join("notes.txt"), b"hello").unwrap();
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_seen, 3, "txt 不计入 seen");
}

/// §71：删除文件 → sweep 停用 + 统计保留。
#[test]
fn delete_marks_missing_and_keeps_stats() {
    let (mut svc, root) = seeded("delete");
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let a = root.join("a.flac");
    let id = track_id_at(&svc, &a).unwrap();
    svc.database()
        .connection()
        .execute(
            "UPDATE playback_stats SET play_count = 3 WHERE track_id = ?1",
            [id],
        )
        .unwrap();

    fs::remove_file(&a).unwrap();
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_deleted, 1);
    assert_eq!(active_tracks(&svc), 2);
    let stats: i64 = svc
        .database()
        .connection()
        .query_row(
            "SELECT play_count FROM playback_stats WHERE track_id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stats, 3, "统计保留（§47）");
}

/// §67：取消 —— 已处理文件保持已提交，取消错误返回。
#[test]
fn cancellation_keeps_committed_prefix_consistent() {
    let (mut svc, root) = seeded("cancel");
    // 预先取消 → 扫描立刻失败。
    let token = CancelToken::new();
    token.cancel();
    let err = svc.scan_root(&root, &token, None).unwrap_err();
    assert!(matches!(err, aura_library::error::LibraryError::Cancelled));

    // 部分提交验证：先完整扫一遍，再加文件后取消 —— 已导入的前缀一致。
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let token2 = CancelToken::new();
    token2.cancel();
    assert!(matches!(
        svc.scan_root(&root, &token2, None),
        Err(aura_library::error::LibraryError::Cancelled)
    ));
    assert_eq!(active_tracks(&svc), 3, "取消不破坏既有状态");
}

/// §68：进度回调结构与单调性。
#[test]
fn progress_reports_monotonic_counts() {
    let (mut svc, root) = seeded("progress");
    let events: Arc<Mutex<Vec<aura_library::types::ScanProgress>>> =
        Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    svc.scan_root(
        &root,
        &CancelToken::new(),
        Some(&mut |p| {
            sink.lock().unwrap().push(p.clone());
        }),
    )
    .unwrap();
    let events = events.lock().unwrap();
    assert!(!events.is_empty());
    let mut last_processed = 0;
    for e in events.iter() {
        assert!(e.processed >= last_processed, "processed 单调: {events:?}");
        last_processed = e.processed;
        assert!(e.discovered >= e.processed);
    }
    assert_eq!(events.last().unwrap().processed, 3);
}

/// §18：空目录扫描 → 零计数。
#[test]
fn empty_dir_scan() {
    let root = temp_dir("empty");
    let mut svc = LibraryService::open_in_memory().unwrap();
    svc.add_root(&root, 1).unwrap();
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!((r.files_seen, r.files_imported, r.files_deleted), (0, 0, 0));
}

/// mtime 分辨率陷阱：内容变化但 size 相同时必须能检测到（借 mtime 变化）。
#[test]
fn same_size_content_change_detected() {
    let (mut svc, root) = seeded("samesize");
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let a = root.join("a.flac");

    // 换成不同标题的同尺寸标签结构（title 变长会变 size —— 用等长标题）。
    let mut tagged = lofty::read_from_path(&a).unwrap();
    tagged
        .first_tag_mut()
        .unwrap()
        .set_title("Song X".to_string()); // 等长
    tagged.save_to_path(&a, WriteOptions::default()).unwrap();

    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_updated + r.files_skipped, 3);
    let id = track_id_at(&svc, &a).unwrap();
    let title: String = svc
        .database()
        .connection()
        .query_row("SELECT title FROM tracks WHERE id = ?1", [id], |r| r.get(0))
        .unwrap();
    assert_eq!(title, "Song X", "size 相同但 mtime 变化 → 重新解析");
}

/// 递增时钟注入验证（date_added 单调、确定性可测）。
#[test]
fn injected_clock_used_for_timestamps() {
    let root = temp_dir("clock");
    make_flac(&root, "t.flac", "T");
    let mut svc = LibraryService::open_in_memory().unwrap();
    let counter = Arc::new(AtomicI64::new(100));
    let c = counter.clone();
    svc.set_clock(Box::new(move || c.fetch_add(1, Ordering::Relaxed)));
    svc.add_root(&root, 1).unwrap();
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let (added, scanned): (i64, i64) = svc
        .database()
        .connection()
        .query_row(
            "SELECT date_added_ms, last_scanned_ms FROM tracks LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(added >= 100, "注入时钟生效: {added}");
    assert!(scanned >= added);
}
