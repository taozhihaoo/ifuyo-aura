//! 基础缓存测试（§97）：create/reopen/chunk write/read/hash/missing/
//! corrupt/orphan + WAL + 原子写顺序 + §98 边界辅助（chunk_bounds）。

use std::sync::Arc;

use aura_cache::db::{CacheChunkRow, CacheDb, CacheEntryRow};
use aura_cache::keys::content_key;
use aura_cache::store::{chunk_bounds, sha256_hex, ChunkStore, CHUNK_SIZE};
use aura_testkit::faultfs::{ChunkFs, FaultFS, RealFs};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("aura-m5-store-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

fn make_entry(db: &CacheDb, ck: &str, now: i64) {
    db.upsert_entry(&CacheEntryRow {
        content_key: ck.to_string(),
        source_kind: "DirectHttp".into(),
        url_hash: ck.to_string(),
        redacted_url: "https://example.com/a.mp3".into(),
        content_length: Some(CHUNK_SIZE as u64 * 3),
        mime: Some("audio/mpeg".into()),
        codec: Some("mp3".into()),
        seekable: true,
        complete: false,
        etag: Some("\"v1\"".into()),
        last_modified: None,
        policy: "Allowed".into(),
        created_at_ms: now,
        last_accessed_ms: now,
        total_bytes: 0,
    })
    .unwrap();
}

/// §97：create + reopen + WAL。
#[test]
fn cache_create_reopen_wal() {
    let root = temp_root("reopen");
    let db_path = root.join("cache.db");
    {
        let db = CacheDb::open(&db_path).unwrap();
        assert_eq!(db.entry_count().unwrap(), 0);
    }
    {
        let db = CacheDb::open(&db_path).unwrap();
        assert_eq!(db.entry_count().unwrap(), 0);
        let mode: String = db.quick_check().unwrap();
        assert_eq!(mode, "ok");
    }
    // WAL 生效。
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode.to_lowercase(), "wal");
    let _ = std::fs::remove_dir_all(&root);
}

/// §97：chunk write/read/hash。
#[test]
fn chunk_write_read_hash() {
    let root = temp_root("chunkrw");
    let store = ChunkStore::new(&root, Arc::new(RealFs)).unwrap();
    let ck = content_key("https://example.com/a.mp3").unwrap();
    let data: Vec<u8> = (0..CHUNK_SIZE).map(|i| (i % 251) as u8).collect();

    let rec = store.write_chunk(&ck, 0, &data).unwrap();
    assert_eq!(rec.size, CHUNK_SIZE as u64);
    assert_eq!(rec.sha256, sha256_hex(&data));
    assert_eq!(store.read_chunk(&ck, 0).unwrap(), data);

    // 路径形态：chunks/<2c>/<ck>/00000000.bin（§14/§116）。
    let p = root
        .join("chunks")
        .join(&ck[..2])
        .join(&ck)
        .join("00000000.bin");
    assert!(p.is_file(), "{p:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// §97：chunk missing → Corrupt 语义（不 panic、不返回脏数据）。
#[test]
fn chunk_missing_detected() {
    let root = temp_root("missing");
    let store = ChunkStore::new(&root, Arc::new(RealFs)).unwrap();
    let ck = content_key("https://example.com/a.mp3").unwrap();
    let e = store.read_chunk(&ck, 7).unwrap_err();
    assert!(matches!(e, aura_cache::error::CacheError::Corrupt { .. }));
    let _ = std::fs::remove_dir_all(&root);
}

/// §24/§97：corrupt（文件被改）→ hash 不一致可检出。
#[test]
fn chunk_corruption_detectable() {
    let root = temp_root("corrupt");
    let store = ChunkStore::new(&root, Arc::new(RealFs)).unwrap();
    let ck = content_key("https://example.com/a.mp3").unwrap();
    let data = vec![7u8; 1024];
    let rec = store.write_chunk(&ck, 0, &data).unwrap();

    // 模拟位翻转（磁盘损坏）。
    let p = root
        .join("chunks")
        .join(&ck[..2])
        .join(&ck)
        .join("00000000.bin");
    let mut bytes = std::fs::read(&p).unwrap();
    bytes[100] ^= 0xFF;
    std::fs::write(&p, &bytes).unwrap();

    let on_disk = store.read_chunk(&ck, 0).unwrap();
    assert_ne!(sha256_hex(&on_disk), rec.sha256, "损坏必须被 hash 检出");
    let _ = std::fs::remove_dir_all(&root);
}

/// §97/§22：orphan chunk（文件在、DB 无记录）→ 可列出并可回收。
#[test]
fn orphan_chunk_listed_and_cleanable() {
    let root = temp_root("orphan");
    let store = ChunkStore::new(&root, Arc::new(RealFs)).unwrap();
    let ck = content_key("https://example.com/orphan.mp3").unwrap();
    store.write_chunk(&ck, 3, &[1, 2, 3]).unwrap();

    // 无 DB 记录：integrity 视角 = orphan；清理 = remove_entry_files。
    let files = store.list_entry_files(&ck).unwrap();
    assert_eq!(files.len(), 1);
    store.remove_entry_files(&ck).unwrap();
    assert!(store.list_entry_files(&ck).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

/// §65/§101：write failure（FaultFS）→ temp 不残留、错误上抛。
#[test]
fn faultfs_write_failure_leaves_no_temp() {
    let root = temp_root("fwfail");
    let fault = Arc::new(FaultFS::new());
    let store = ChunkStore::new(&root, Arc::clone(&fault) as Arc<dyn ChunkFs>).unwrap();
    let ck = content_key("https://example.com/f.mp3").unwrap();

    fault.arm(aura_testkit::faultfs::Fault::WriteFailure);
    let e = store.write_chunk(&ck, 0, &[1, 2, 3]).unwrap_err();
    assert!(matches!(
        e,
        aura_cache::error::CacheError::Io(_)
            | aura_cache::error::CacheError::DiskFull
            | aura_cache::error::CacheError::PermissionDenied
    ));
    assert!(fault.fired().len() == 1, "故障确实被消费");
    // tmp 目录无残留。
    let tmp: Vec<_> = std::fs::read_dir(root.join("tmp")).unwrap().collect();
    assert!(tmp.is_empty(), "写失败不得残留 temp: {tmp:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// §65/§101：truncation → 写后大小校验捕获。
#[test]
fn faultfs_truncation_detected() {
    let root = temp_root("ftrunc");
    let fault = Arc::new(FaultFS::new());
    let store = ChunkStore::new(&root, Arc::clone(&fault) as Arc<dyn ChunkFs>).unwrap();
    let ck = content_key("https://example.com/t.mp3").unwrap();

    fault.arm(aura_testkit::faultfs::Fault::Truncate { keep_bytes: 10 });
    let e = store.write_chunk(&ck, 0, &[9u8; 100]).unwrap_err();
    assert!(matches!(e, aura_cache::error::CacheError::Corrupt { .. }));
    let _ = std::fs::remove_dir_all(&root);
}

/// §65/§101：rename failure → temp 清理、目标不出现。
#[test]
fn faultfs_rename_failure_no_temp_left() {
    let root = temp_root("frename");
    let fault = Arc::new(FaultFS::new());
    let store = ChunkStore::new(&root, Arc::clone(&fault) as Arc<dyn ChunkFs>).unwrap();
    let ck = content_key("https://example.com/r.mp3").unwrap();

    fault.arm(aura_testkit::faultfs::Fault::RenameFailure);
    assert!(store.write_chunk(&ck, 0, &[1]).is_err());
    assert!(!root
        .join("chunks")
        .join(&ck[..2])
        .join(&ck)
        .join("00000000.bin")
        .exists());
    let tmp: Vec<_> = std::fs::read_dir(root.join("tmp")).unwrap().collect();
    assert!(tmp.is_empty(), "rename 失败不得残留 temp");
    let _ = std::fs::remove_dir_all(&root);
}

/// §64：chunk 记录 + entry 聚合在 DB 层一致。
#[test]
fn db_chunk_bookkeeping() {
    let db = CacheDb::open_in_memory().unwrap();
    let ck = content_key("https://example.com/db.mp3").unwrap();
    make_entry(&db, &ck, 100);

    for idx in 0..3u64 {
        db.insert_chunk(&CacheChunkRow {
            content_key: ck.clone(),
            chunk_index: idx,
            size: 100,
            sha256: sha256_hex(&[idx as u8; 16]),
            created_at_ms: 100,
            last_accessed_ms: 100,
        })
        .unwrap();
    }
    assert_eq!(db.chunk_count().unwrap(), 3);
    let entry = db.get_entry(&ck).unwrap().unwrap();
    assert_eq!(entry.total_bytes, 300, "聚合字节数随 chunk 更新");

    db.delete_chunk(&ck, 1).unwrap();
    let entry = db.get_entry(&ck).unwrap().unwrap();
    assert_eq!(entry.total_bytes, 200);
    assert_eq!(db.list_chunks(&ck).unwrap().len(), 2);

    // CASCADE：删 entry 连带 chunks。
    db.delete_entry(&ck).unwrap();
    assert_eq!(db.chunk_count().unwrap(), 0);
}

/// §32：partial last chunk 合法。
#[test]
fn partial_last_chunk_legal() {
    let bounds = chunk_bounds((CHUNK_SIZE as u64) * 2 + 1000);
    assert_eq!(bounds.len(), 3);
    assert_eq!(bounds[0], (0, CHUNK_SIZE));
    assert_eq!(bounds[1], (CHUNK_SIZE as u64, CHUNK_SIZE));
    assert_eq!(
        bounds[2],
        ((CHUNK_SIZE as u64) * 2, 1000),
        "末块 < 256KiB 合法"
    );
    assert!(chunk_bounds(0).is_empty());
}
