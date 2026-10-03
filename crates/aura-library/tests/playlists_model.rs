//! [B] 歌单/智能歌单/队列模型测试（§58/§59/§60）。

use std::fs;
use std::path::PathBuf;

use lofty::prelude::*;

use aura_library::playlists::QueueCommand;
use aura_library::query::Page;
use aura_library::service::LibraryService;
use aura_library::types::{CancelToken, PlaylistId};

fn temp_dir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("aura-m3-pl-{tag}-{}-{n}", std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn seeded(tag: &str) -> LibraryService {
    let root = temp_dir(tag);
    for (name, title) in [("a.flac", "Alpha"), ("b.flac", "Beta"), ("c.flac", "Gamma")] {
        let p = root.join(name);
        fs::copy(fixture("sine_8k.flac"), &p).unwrap();
        let mut tagged = lofty::read_from_path(&p).unwrap();
        let mut tag = lofty::tag::Tag::new(lofty::tag::TagType::VorbisComments);
        tag.set_title(title.to_string());
        tagged.insert_tag(tag);
        tagged
            .save_to_path(&p, lofty::config::WriteOptions::default())
            .unwrap();
    }
    let mut svc = LibraryService::open_in_memory().unwrap();
    svc.set_clock(Box::new(|| 5_000));
    svc.add_root(&root, 1).unwrap();
    svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    svc
}

fn track_ids(svc: &LibraryService) -> Vec<aura_library::types::TrackId> {
    svc.list_tracks(aura_library::query::SortKey::Title, Page::new(10, 0))
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect()
}

/// §58：stable ordering + duplicate policy + delete semantics + transaction。
#[test]
fn manual_playlist_lifecycle() {
    let mut svc = seeded("manual");
    let ids = track_ids(&svc);
    let pl = svc.create_playlist("My Mix", 1).unwrap();

    svc.playlist_add_track(pl, ids[0], 2).unwrap();
    svc.playlist_add_track(pl, ids[1], 3).unwrap();
    svc.playlist_add_track(pl, ids[0], 4).unwrap(); // 重复允许（不同位置）

    let content = svc.playlist_tracks(pl).unwrap();
    assert_eq!(content.len(), 3);
    assert_eq!(content[0].title.as_deref(), Some("Alpha"));
    assert_eq!(content[1].title.as_deref(), Some("Beta"));
    assert_eq!(content[2].title.as_deref(), Some("Alpha"), "稳定顺序");

    // 删除中间 → 位置压实。
    assert!(svc.playlist_remove_at(pl, 1, 5).unwrap());
    let content = svc.playlist_tracks(pl).unwrap();
    assert_eq!(content.len(), 2);
    assert_eq!(content[1].title.as_deref(), Some("Alpha"));

    // 删除不存在位置 → false。
    assert!(!svc.playlist_remove_at(pl, 99, 6).unwrap());

    // 幂等重复添加不产生重复行（同一位置不会冲突：position 递增）。
    let lists = svc.playlists().unwrap();
    assert_eq!(lists.len(), 1);
    assert_eq!(lists[0].track_count, 2);

    // 删除歌单：级联成员，不影响 track。
    assert!(svc.delete_playlist(pl).unwrap());
    assert!(svc.playlists().unwrap().is_empty());
    assert_eq!(
        svc.list_tracks(aura_library::query::SortKey::Title, Page::new(10, 0))
            .unwrap()
            .len(),
        3,
        "tracks 不受影响"
    );
    // 成员行已级联删除。
    let n: i64 = svc
        .database()
        .connection()
        .query_row("SELECT count(*) FROM playlist_tracks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0, "无 dangling playlist 成员");
}

/// §59：smart playlist 存 executable query representation，不伪造分析。
#[test]
fn smart_playlist_stores_query_and_evaluates() {
    let mut svc = seeded("smart");
    let sp = svc
        .create_smart_playlist("90s+", "year:1990-2000", Some("title"), Some(10), 1)
        .unwrap();

    let def = svc.smart_playlist(sp).unwrap().unwrap();
    assert_eq!(def.name, "90s+");
    assert_eq!(def.limit, Some(10));

    // 当前库无 year 标签 → 不命中（不猜值）。
    let hits = svc.run_smart_playlist(sp, Page::new(10, 0)).unwrap();
    assert!(hits.is_empty());

    // 已有字段过滤可用。
    let sp2 = svc
        .create_smart_playlist("Alpha songs", "title:alpha", None, None, 2)
        .unwrap();
    let hits2 = svc.run_smart_playlist(sp2, Page::new(10, 0)).unwrap();
    assert_eq!(hits2.len(), 1);
    assert_eq!(hits2[0].title.as_deref(), Some("Alpha"));

    // 非法 query 创建被拒。
    assert!(svc
        .create_smart_playlist("Bad", "unknownfield:1", None, None, 3)
        .is_err());
    let _ = PlaylistId(0);
}

/// §60：队列 drag reorder / remove / undo。
#[test]
fn queue_apply_move_remove_undo() {
    let mut svc = seeded("queue");
    let ids = track_ids(&svc);

    for id in &ids {
        svc.queue_apply(QueueCommand::Append { track_id: *id }, 1)
            .unwrap();
    }
    let q = svc.queue_items().unwrap();
    assert_eq!(q.len(), 3);
    assert_eq!(q[0].track_id, ids[0]);

    // move 尾 → 首。
    svc.queue_apply(QueueCommand::Move { from: 2, to: 0 }, 2)
        .unwrap();
    let q = svc.queue_items().unwrap();
    assert_eq!(q[0].track_id, ids[2], "move 生效");
    assert_eq!(q[1].track_id, ids[0]);

    // undo move。
    let undone = svc.queue_undo(3).unwrap();
    assert_eq!(
        undone,
        Some(QueueCommand::Move { from: 2, to: 0 }),
        "undo 返回被撤销命令"
    );
    let q = svc.queue_items().unwrap();
    assert_eq!(q[0].track_id, ids[0], "恢复原序");
    assert_eq!(q[2].track_id, ids[2]);

    // remove 中间 + undo（track_id 从扩展 payload 恢复）。
    svc.queue_apply(QueueCommand::Remove { position: 1 }, 4)
        .unwrap();
    assert_eq!(svc.queue_items().unwrap().len(), 2);
    let undone = svc.queue_undo(5).unwrap();
    assert!(matches!(undone, Some(QueueCommand::Remove { position: 1 })));
    let q = svc.queue_items().unwrap();
    assert_eq!(q.len(), 3);
    assert_eq!(q[1].track_id, ids[1], "remove 被 undo");

    // 空日志 undo → None。
    let _ = svc.queue_undo(6).unwrap();
    let _ = svc.queue_undo(7).unwrap();
    let _ = svc.queue_undo(8).unwrap();
    assert!(svc.queue_undo(9).unwrap().is_none(), "日志空");
}

/// §60：insert 与 undo。
#[test]
fn queue_insert_undo() {
    let mut svc = seeded("qinsert");
    let ids = track_ids(&svc);
    svc.queue_apply(QueueCommand::Append { track_id: ids[0] }, 1)
        .unwrap();
    svc.queue_apply(QueueCommand::Append { track_id: ids[1] }, 2)
        .unwrap();
    svc.queue_apply(
        QueueCommand::Insert {
            track_id: ids[2],
            position: 1,
        },
        3,
    )
    .unwrap();
    let q = svc.queue_items().unwrap();
    assert_eq!(q[1].track_id, ids[2]);

    svc.queue_undo(4).unwrap();
    let q = svc.queue_items().unwrap();
    assert_eq!(q.len(), 2);
    assert_eq!(q[1].track_id, ids[1], "insert 被 undo");
}

/// §58：Clear 的 undo 语义（D-047：M3 不做快照恢复，日志弹出并跳过）。
#[test]
fn queue_clear_undo_is_explicit_noop() {
    let mut svc = seeded("qclear");
    let ids = track_ids(&svc);
    svc.queue_apply(QueueCommand::Append { track_id: ids[0] }, 1)
        .unwrap();
    svc.queue_apply(QueueCommand::Clear, 2).unwrap();
    assert!(svc.queue_items().unwrap().is_empty());
    let undone = svc.queue_undo(3).unwrap();
    assert_eq!(undone, Some(QueueCommand::Clear));
    // 明确的 no-op（无快照恢复）：记录为已知限制。
    assert!(svc.queue_items().unwrap().is_empty());
}
