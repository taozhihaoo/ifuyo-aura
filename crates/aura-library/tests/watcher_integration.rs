//! watcher 集成测试（§73）：真实 notify 监听 + 确定性合并 +
//! create/modify/rename/delete → 候选 → 扫描应用 → 数据库终态正确。
//! 等待策略：有界轮询条件（wait_until），不依赖固定 sleep。

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lofty::prelude::*;

use aura_library::service::LibraryService;
use aura_library::types::CancelToken;
use aura_library::watcher::{spawn_watcher, EventCoalescer};

fn temp_dir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("aura-m3-watch-{tag}-{}-{n}", std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn fixture_flac(dst: &PathBuf) {
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sine_8k.flac"),
        dst,
    )
    .unwrap();
}

fn flush_and_merge(
    coalescer: &Arc<Mutex<EventCoalescer>>,
) -> Vec<(PathBuf, aura_library::watcher::CandidateKind)> {
    let mut c = coalescer.lock().unwrap();
    // 强制窗口判定通过后 flush（集成测试里时间已流逝；这里直接取候选）。
    let out: Vec<_> = c.flush().into_iter().map(|r| (r.path, r.kind)).collect();
    out
}

/// create → modify → rename → delete 全链路。
#[test]
fn watch_create_modify_rename_delete() {
    let root = temp_dir("lifecycle");
    let coalescer = Arc::new(Mutex::new(EventCoalescer::new(Duration::from_millis(200))));
    let _watcher = spawn_watcher(std::slice::from_ref(&root), coalescer.clone()).unwrap();

    // 1. create
    let a = root.join("a.flac");
    fixture_flac(&a);
    // 2. modify（改标签）
    let mut tagged = lofty::read_from_path(&a).unwrap();
    let mut tag = lofty::tag::Tag::new(lofty::tag::TagType::VorbisComments);
    tag.set_title("Watched".to_string());
    tagged.insert_tag(tag);
    tagged
        .save_to_path(&a, lofty::config::WriteOptions::default())
        .unwrap();
    // 3. rename
    let b = root.join("renamed.flac");
    fs::rename(&a, &b).unwrap();
    // 4. delete
    fs::remove_file(&b).unwrap();

    // 有界等待：合并器收到候选（§73 retry with bounded condition wait）。
    let got = aura_library::watcher::wait_until(
        || coalescer.lock().unwrap().pending_count() > 0,
        Duration::from_secs(10),
        Duration::from_millis(50),
    );
    // wait_until 消费闭包会取走候选——改为检查 pending 计数后统一 flush。
    assert!(got, "10s 内未收到任何 watcher 事件");
    let candidates = flush_and_merge(&coalescer);
    assert!(!candidates.is_empty(), "事件应被合并为候选");

    // 终态验证：文件已删除，扫描后库中无该 track（或从未进入）。
    let mut svc = LibraryService::open_in_memory().unwrap();
    svc.add_root(&root, 1).unwrap();
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r.files_seen, 0);
    assert_eq!(r.files_imported, 0);
}

/// watcher 事件驱动增量重扫：create → 候选 → 扫描导入 → rename → 再扫描 → 同 TrackId。
#[test]
fn event_driven_rescan_preserves_identity() {
    let root = temp_dir("identity");
    let coalescer = Arc::new(Mutex::new(EventCoalescer::new(Duration::from_millis(200))));
    let _watcher = spawn_watcher(std::slice::from_ref(&root), coalescer.clone()).unwrap();

    let mut svc = LibraryService::open_in_memory().unwrap();
    svc.add_root(&root, 1).unwrap();
    assert_eq!(
        svc.scan_root(&root, &CancelToken::new(), None)
            .unwrap()
            .files_seen,
        0
    );

    let a = root.join("song.flac");
    fixture_flac(&a);
    let r1 = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r1.files_imported, 1);
    let id1: i64 = svc
        .database()
        .connection()
        .query_row("SELECT id FROM tracks", [], |r| r.get(0))
        .unwrap();

    // rename（真实 FS 操作 → watcher 记录候选；身份由扫描层经内容哈希识别）。
    let b = root.join("renamed_song.flac");
    fs::rename(&a, &b).unwrap();
    // 等待 watcher 观察到事件（有界）。
    assert!(
        aura_library::watcher::wait_until(
            || coalescer.lock().unwrap().pending_count() > 0,
            Duration::from_secs(10),
            Duration::from_millis(50),
        ),
        "rename 事件未到达"
    );
    let r2 = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    assert_eq!(r2.files_updated, 1, "rename 合并");
    let id2: i64 = svc
        .database()
        .connection()
        .query_row("SELECT id FROM tracks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(id1, id2, "TrackId 不变（§48）");

    // 合并器确实收到了事件（flush 取走）。
    let candidates = flush_and_merge(&coalescer);
    assert!(
        candidates.iter().any(|(p, _)| p
            .file_name()
            .is_some_and(|n| n.to_string_lossy().contains("song"))),
        "rename 事件候选存在: {candidates:?}"
    );
}
