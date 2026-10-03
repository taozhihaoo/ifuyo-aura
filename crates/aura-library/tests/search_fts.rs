//! FTS5 搜索端到端测试（§32/§35/§75）：导入 → 索引 → 检索。
//! 覆盖：title/artist/album/path、中文二元组、单字前缀、英文前缀、
//! 大小写不敏感、特殊字符、空查询、无结果、稳定排序。

use std::fs;
use std::path::PathBuf;

use aura_library::db::{repo, Database};
use aura_library::search;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::{Tag, TagType};

fn temp_dir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("aura-m3-fts-{tag}-{}-{n}", std::process::id()));
    fs::create_dir_all(&p).unwrap();
    p
}

struct Env {
    db: Database,
    root: PathBuf,
    root_id: aura_library::types::RootId,
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

    fn import(&mut self, name: &str, title: &str, artist: &str, album: &str) {
        let p = self.root.join(name);
        fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sine_8k.flac"),
            &p,
        )
        .unwrap();
        let mut tagged = lofty::read_from_path(&p).unwrap();
        let mut tag = Tag::new(TagType::VorbisComments);
        tag.set_title(title.to_string());
        tag.set_artist(artist.to_string());
        tag.set_album(album.to_string());
        tagged.insert_tag(tag);
        tagged.save_to_path(&p, WriteOptions::default()).unwrap();
        let tx = self.db.transaction().unwrap();
        repo::import_file(&tx, self.root_id, &p, 2000).unwrap();
        tx.commit().unwrap();
    }

    fn search(&self, q: &str) -> Vec<i64> {
        search::search(self.db.connection(), q, 50, 0)
            .unwrap()
            .into_iter()
            .map(|h| h.track_id.0)
            .collect()
    }
}
impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn seeded() -> Env {
    let mut env = Env::new("seed");
    env.import(
        "a.flac",
        "Wish You Were Here",
        "Pink Floyd",
        "The Dark Side of the Moon",
    );
    env.import("b.flac", "夜曲", "周杰伦", "十一月的萧邦");
    env.import("c.flac", "アイドル", "YOASOBI", "THE BOOK");
    env.import("d.flac", "Stream of Lights", "流光乐队", "霓虹夜");
    env
}

#[test]
fn search_title_exact_and_prefix() {
    let env = seeded();
    assert_eq!(env.search("Wish You Were Here").len(), 1);
    assert_eq!(env.search("wish").len(), 1, "大小写不敏感 + 前缀");
}

/// §35：pink → Pink Floyd。
#[test]
fn search_artist_case_insensitive_prefix() {
    let env = seeded();
    let hits = env.search("pink");
    assert_eq!(hits.len(), 1);
}

#[test]
fn search_album() {
    let env = seeded();
    assert_eq!(env.search("dark side").len(), 1);
}

/// §32：path 可搜索（用独特文件名 token 验证）。
#[test]
fn search_path() {
    let mut env = Env::new("path");
    env.import("darksidefile.flac", "T1", "A1", "AL1");
    let hits = env.search("darksidefile");
    assert_eq!(hits.len(), 1, "路径 token 可检索: {hits:?}");
}

/// §33：中文二元组（音乐 → 夜曲? no；流光乐队 → 流光/光光…）。
#[test]
fn search_chinese_bigram() {
    let env = seeded();
    assert_eq!(env.search("夜曲").len(), 1, "二元组匹配曲目");
    assert_eq!(env.search("流光").len(), 1, "流光乐队命中");
    assert!(
        !env.search("夜曲").contains(&(env.search("流光")[0])),
        "不同曲目不串"
    );
}

/// 跨字段：artist 中的中文也能命中。
#[test]
fn search_chinese_artist() {
    let env = seeded();
    assert_eq!(env.search("周杰伦").len(), 1);
}

/// 多字滑窗查询（跨二元组边界）。
#[test]
fn search_chinese_substring() {
    let env = seeded();
    // 「光乐」在索引里是 sliding bigram 之一（流光乐队）。
    assert_eq!(env.search("光乐").len(), 1);
}

/// §34：单字前缀（标题「夜曲」与专辑「霓虹夜」均含「夜」起始 token）。
#[test]
fn search_single_character() {
    let env = seeded();
    let hits = env.search("夜");
    assert_eq!(hits.len(), 2, "夜曲 + 霓虹夜(专辑): {hits:?}");
    // 夜曲 track 在结果中。
    let titles: Vec<String> = hits
        .iter()
        .map(|id| {
            env.db
                .connection()
                .query_row("SELECT title FROM tracks WHERE id = ?1", [id], |r| r.get(0))
                .unwrap()
        })
        .collect();
    assert!(titles.contains(&"夜曲".to_string()), "{titles:?}");
}

#[test]
fn search_japanese() {
    let env = seeded();
    assert_eq!(env.search("アイドル").len(), 1);
    // 浊音假名（ド）必须原样保留（NFKD 陷阱）。
    assert_eq!(env.search("ドル").len(), 1);
}

/// §65：特殊字符不 panic、不注入。
#[test]
fn search_special_characters_no_panic() {
    let env = seeded();
    for evil in [
        "\"; DROP TABLE tracks; --",
        "a:b (c) -d *",
        "NEAR(a b)",
        "' OR 1=1 --",
        "🎵🎶",
        "!!!",
        "",
        "   ",
    ] {
        let _ = env.search(evil); // 不得 panic
    }
    assert_eq!(env.count_tracks(), 4, "注入后表仍在");
}

/// §75：空查询 → 无结果（不是全表）。
#[test]
fn search_empty_query_no_results() {
    let env = seeded();
    assert!(env.search("").is_empty());
    assert!(env.search("   ").is_empty());
}

#[test]
fn search_no_result_for_unknown() {
    let env = seeded();
    assert!(env.search("zzzqqqxxx").is_empty());
}

/// §37：相同数据库 + 相同查询 → 相同顺序。
#[test]
fn search_stable_ordering() {
    let env = seeded();
    // 四轨都含 "o" 类词？造一个多命中查询：全部标题/艺术家共享 token 难，
    // 直接验证两次调用逐位一致即可（含并列时的 track_id 决胜）。
    let a = env.search("o");
    let b = env.search("o");
    assert_eq!(a, b, "重复调用顺序稳定");
}

/// 分页：limit/offset 与排序稳定（§41）。
#[test]
fn search_pagination() {
    let env = seeded();
    let page1 = search::search(env.db.connection(), "o", 2, 0).unwrap();
    let page2 = search::search(env.db.connection(), "o", 2, 2).unwrap();
    let all = search::search(env.db.connection(), "o", 50, 0).unwrap();
    let mut concatenated = page1;
    concatenated.extend(page2);
    assert_eq!(
        concatenated.len().min(all.len()),
        concatenated.len(),
        "分页结果不重复"
    );
    for (i, hit) in concatenated.iter().enumerate() {
        if i < all.len() {
            assert_eq!(hit.track_id, all[i].track_id, "分页与全量顺序一致");
        }
    }
}

impl Env {
    fn count_tracks(&self) -> i64 {
        self.db
            .connection()
            .query_row("SELECT count(*) FROM tracks", [], |r| r.get(0))
            .unwrap()
    }
}

/// FTS 行与 track 生命周期同步：标签改写后索引反映新值。
#[test]
fn fts_index_follows_metadata_update() {
    let mut env = Env::new("upd");
    let p = env.root.join("u.flac");
    fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sine_8k.flac"),
        &p,
    )
    .unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title("Old Title".to_string());
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();
    let tx = env.db.transaction().unwrap();
    repo::import_file(&tx, env.root_id, &p, 2000).unwrap();
    tx.commit().unwrap();
    assert_eq!(env.search("Old Title").len(), 1);

    // 改标签 → 再导入 → 新标题可检索，旧标题不再命中。
    let mut tagged = lofty::read_from_path(&p).unwrap();
    tagged
        .first_tag_mut()
        .unwrap()
        .set_title("Fresh Title".to_string());
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();
    let tx = env.db.transaction().unwrap();
    repo::import_file(&tx, env.root_id, &p, 3000).unwrap();
    tx.commit().unwrap();
    assert_eq!(env.search("Fresh Title").len(), 1);
    assert_eq!(env.search("Old Title").len(), 0, "旧行不残留");
}
