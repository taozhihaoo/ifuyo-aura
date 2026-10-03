//! 仓库层身份解析测试（§10/§18/§47/§72）：
//! rename/move 保 TrackId、标签改写保 TrackId、音频替换出新身份、
//! 幂等三连、实体/封面去重、删除保统计。

use std::fs;
use std::path::{Path, PathBuf};

use aura_library::db::{repo, Database};
use aura_library::types::RootId;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::{Tag, TagType};

fn temp_dir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("aura-m3-repo-{tag}-{}-{n}", std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

struct Env {
    db: Database,
    root: PathBuf,
    root_id: RootId,
}
impl Env {
    fn new(tag: &str) -> Self {
        let mut db = Database::open_in_memory().unwrap();
        let root = temp_dir(tag);
        let tx = db.transaction().unwrap();
        let root_id = repo::insert_root(&tx, &root.to_string_lossy(), 1000).unwrap();
        tx.commit().unwrap();
        Self { db, root, root_id }
    }

    /// 事务内导入单文件。
    fn import(&mut self, path: &Path, now_ms: i64) -> repo::ImportOutcome {
        let tx = self.db.transaction().unwrap();
        let out = repo::import_file(&tx, self.root_id, path, now_ms).unwrap();
        tx.commit().unwrap();
        out
    }

    /// 扫描收尾（模拟整轮扫描后未见路径停用）。
    fn sweep(&mut self, seen: &[PathBuf], now_ms: i64) -> u64 {
        let seen: Vec<String> = seen
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        let tx = self.db.transaction().unwrap();
        let n = repo::mark_missing(&tx, self.root_id, &seen, now_ms).unwrap();
        tx.commit().unwrap();
        n
    }

    fn count(&self, sql: &str) -> i64 {
        self.db
            .connection()
            .query_row(sql, [], |r| r.get(0))
            .unwrap()
    }

    fn track_ids(&self) -> Vec<i64> {
        let mut stmt = self
            .db
            .connection()
            .prepare("SELECT id FROM tracks ORDER BY id")
            .unwrap();
        let rows = stmt.query_map([], |r| r.get(0)).unwrap();
        rows.map(|r| r.unwrap()).collect()
    }
}
impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn setup_track(dir: &Path, name: &str, title: &str, artist: &str) -> PathBuf {
    let p = dir.join(name);
    fs::copy(fixture("sine_8k.flac"), &p).unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title(title.to_string());
    tag.set_artist(artist.to_string());
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();
    p
}

/// §18：scan×3 幂等——无重复 track/path/artist/album/artwork/stats。
#[test]
fn triple_scan_idempotent() {
    let mut env = Env::new("idem");
    let a = setup_track(&env.root, "a.flac", "Song A", "Artist");
    let b = setup_track(&env.root, "b.flac", "Song B", "Artist");
    env.import(&a, 2000);
    env.import(&b, 2000);
    let (t1, p1, art1, stats1) = (
        env.count("SELECT count(*) FROM tracks"),
        env.count("SELECT count(*) FROM track_paths"),
        env.count("SELECT count(*) FROM artworks"),
        env.count("SELECT count(*) FROM playback_stats"),
    );
    for now in [3000, 4000, 5000] {
        env.import(&a, now);
        env.import(&b, now);
    }
    assert_eq!(env.count("SELECT count(*) FROM tracks"), t1, "track 不重复");
    assert_eq!(
        env.count("SELECT count(*) FROM track_paths"),
        p1,
        "path 不重复"
    );
    assert_eq!(env.count("SELECT count(*) FROM artworks"), art1);
    assert_eq!(env.count("SELECT count(*) FROM playback_stats"), stats1);
    assert_eq!(
        env.count("SELECT count(*) FROM artists"),
        1,
        "同 artist 共享实体"
    );
    assert_eq!(
        env.count("SELECT count(*) FROM albums"),
        0,
        "未设专辑名 → 无 album 行"
    );
}

/// §72：rename → 同 TrackId；旧路径停用。
#[test]
fn rename_keeps_track_id() {
    let mut env = Env::new("rename");
    let a = setup_track(&env.root, "old_name.flac", "Song", "Artist");
    let first = env.import(&a, 2000);

    let b = env.root.join("new_name.flac");
    fs::rename(&a, &b).unwrap();
    let second = env.import(&b, 3000);
    env.sweep(&[b.clone()], 3000);

    assert_eq!(first.track_id, second.track_id, "改名保身份");
    assert!(!second.created);
    assert_eq!(
        env.count("SELECT count(*) FROM tracks WHERE is_active = 1"),
        1
    );
    let paths: i64 = env.count("SELECT count(*) FROM track_paths WHERE is_active = 1");
    assert_eq!(paths, 1, "旧路径停用");
}

/// §72：move → 同 TrackId。
#[test]
fn move_keeps_track_id() {
    let mut env = Env::new("move");
    fs::create_dir_all(env.root.join("sub")).unwrap();
    let a = setup_track(&env.root, "x.flac", "Song", "Artist");
    let first = env.import(&a, 2000);

    let b = env.root.join("sub/x.flac");
    fs::rename(&a, &b).unwrap();
    let second = env.import(&b, 3000);
    env.sweep(&[b], 3000);
    assert_eq!(first.track_id, second.track_id, "移动保身份");
}

/// §71 duplicate content：同内容两份 → 一个 track、两条活跃路径。
#[test]
fn duplicate_content_merges() {
    let mut env = Env::new("dup");
    let a = setup_track(&env.root, "one.flac", "Song", "Artist");
    let b = env.root.join("two.flac");
    fs::copy(&a, &b).unwrap();
    let o1 = env.import(&a, 2000);
    let o2 = env.import(&b, 2001);
    assert_eq!(o1.track_id, o2.track_id, "同内容合并");
    assert!(!o2.created);
    assert_eq!(env.count("SELECT count(*) FROM tracks"), 1);
    assert_eq!(
        env.count("SELECT count(*) FROM track_paths WHERE is_active = 1"),
        2
    );
}

/// §72：metadata edit → 同 TrackId + metadata 更新。
#[test]
fn metadata_edit_keeps_track_id() {
    let mut env = Env::new("metaedit");
    let a = setup_track(&env.root, "m.flac", "Old Title", "Artist");
    let first = env.import(&a, 2000);

    // 只改标签（重写文件头 → 内容变化，但 audio_sig 不变）。
    let mut tagged = lofty::read_from_path(&a).unwrap();
    let tag = tagged.first_tag_mut().unwrap();
    tag.set_title("New Title".to_string());
    tagged.save_to_path(&a, WriteOptions::default()).unwrap();

    let second = env.import(&a, 3000);
    assert_eq!(
        first.track_id, second.track_id,
        "标签改写保 TrackId（D-038）"
    );
    assert!(second.updated);
    let title: String = env
        .db
        .connection()
        .query_row(
            "SELECT title FROM tracks WHERE id = ?1",
            [first.track_id.0],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(title, "New Title");
}

/// §72：audio replacement → 新身份；旧 track 停用但统计保留。
/// 替换物：不同规格的真实 FLAC（48kHz/1s，audio_sig 与 8kHz/0.25s 不同）。
#[test]
fn audio_replacement_creates_new_identity_and_keeps_stats() {
    let mut env = Env::new("replace");
    let a = setup_track(&env.root, "r.flac", "Song", "Artist");
    let first = env.import(&a, 2000);

    // 记一次播放。
    env.db
        .connection()
        .execute(
            "UPDATE playback_stats SET play_count = 7, last_played_at_ms = 2500 WHERE track_id = ?1",
            [first.track_id.0],
        )
        .unwrap();

    // 音频内容替换：48kHz 1s FLAC 覆盖 8kHz 0.25s（codec 相同、时长/采样率变）。
    let other = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../aura-core/tests/fixtures/flac_mono_48k_440hz_1s.flac");
    fs::copy(&other, &a).unwrap();
    let second = env.import(&a, 3000);

    assert_ne!(first.track_id, second.track_id, "音频替换出新身份");
    assert_eq!(
        env.count("SELECT count(*) FROM tracks WHERE is_active = 1"),
        1,
        "旧 track 停用"
    );
    // 统计保留在旧 track 上（§47）。
    let old_stats: i64 = env
        .db
        .connection()
        .query_row(
            "SELECT play_count FROM playback_stats WHERE track_id = ?1",
            [first.track_id.0],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(old_stats, 7, "删除/替换文件不销毁历史统计");
}

/// §47：删除文件 + sweep → track 停用、统计保留、复活（恢复文件）不丢统计。
#[test]
fn delete_then_restore_keeps_stats() {
    let mut env = Env::new("delrestore");
    let a = setup_track(&env.root, "d.flac", "Song", "Artist");
    let out = env.import(&a, 2000);
    env.db
        .connection()
        .execute(
            "UPDATE playback_stats SET play_count = 5 WHERE track_id = ?1",
            [out.track_id.0],
        )
        .unwrap();

    fs::remove_file(&a).unwrap();
    env.sweep(&[], 3000);
    assert_eq!(
        env.count("SELECT count(*) FROM tracks WHERE is_active = 1"),
        0,
        "删除后 track 停用"
    );
    let stats: i64 = env
        .db
        .connection()
        .query_row(
            "SELECT play_count FROM playback_stats WHERE track_id = ?1",
            [out.track_id.0],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stats, 5, "统计保留（§47）");

    // 恢复同名文件（同内容）→ 同 track 复活，统计还在。
    setup_track(&env.root, "d.flac", "Song", "Artist");
    let again = env.import(&a, 4000);
    assert_eq!(again.track_id, out.track_id, "恢复文件重关联原 track");
    let stats2: i64 = env
        .db
        .connection()
        .query_row(
            "SELECT play_count FROM playback_stats WHERE track_id = ?1",
            [out.track_id.0],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stats2, 5);
}

/// §26：100 首同 artist 的歌 → 1 个 artist 实体。
#[test]
fn artist_entity_shared() {
    let mut env = Env::new("entities");
    for i in 0..20 {
        let p = setup_track(
            &env.root,
            &format!("s{i}.flac"),
            &format!("Song {i}"),
            "Same Artist",
        );
        env.import(&p, 2000 + i64::from(i));
    }
    assert_eq!(env.count("SELECT count(*) FROM artists"), 1);
    assert_eq!(env.count("SELECT count(*) FROM tracks"), 20);
}

/// §28：同一封面两文件 → 1 行 artwork。
#[test]
fn artwork_dedup_by_content_hash() {
    let tmp = temp_dir("artdedup");
    let png: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];

    let make = |name: &str| {
        let p = tmp.join(name);
        fs::copy(fixture("sine_8k.flac"), &p).unwrap();
        let mut tagged = lofty::read_from_path(&p).unwrap();
        let mut tag = Tag::new(TagType::VorbisComments);
        tag.set_title(name.to_string());
        tag.push_picture(lofty::picture::Picture::new_unchecked(
            lofty::picture::PictureType::CoverFront,
            Some(lofty::picture::MimeType::Png),
            None,
            png.clone(),
        ));
        tagged.insert_tag(tag);
        tagged.save_to_path(&p, WriteOptions::default()).unwrap();
        p
    };
    let a = make("a.flac");
    let b = make("b.flac");

    let mut env = Env::new("artdedup2");
    let root_id = env.root_id;
    let tx = env.db.transaction().unwrap();
    repo::import_file(&tx, root_id, &a, 2000).unwrap();
    repo::import_file(&tx, root_id, &b, 2001).unwrap();
    tx.commit().unwrap();

    assert_eq!(
        env.count("SELECT count(*) FROM artworks"),
        1,
        "内容哈希去重"
    );
    let refs: i64 = env
        .db
        .connection()
        .query_row(
            "SELECT count(*) FROM tracks WHERE artwork_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(refs, 2, "两个 track 引用同一 artwork");
    let _ = fs::remove_dir_all(&tmp);
}
