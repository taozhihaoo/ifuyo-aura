//! M5 性能基线（§89/§130）：chunk 读写、SHA-256、SQLite lookup、
//! cache hit/miss、Range fetch、LRU 淘汰、reopen、integrity 扫描。
//! `#[ignore]`，显式运行：
//! `cargo test -p aura-cache --release --test perf_baseline -- --ignored --nocapture`

use std::sync::Arc;

use aura_cache::db::{CacheChunkRow, CacheDb, CacheEntryRow};
use aura_cache::keys::content_key;

use aura_cache::store::{sha256_hex, ChunkStore, CHUNK_SIZE};
use aura_testkit::faultfs::RealFs;

fn temp_root(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("aura-m5-perf-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

fn make_entry(db: &CacheDb, ck: &str) {
    db.upsert_entry(&CacheEntryRow {
        content_key: ck.to_string(),
        source_kind: "DirectHttp".into(),
        url_hash: ck.to_string(),
        redacted_url: "https://example.com/x".into(),
        content_length: None,
        mime: None,
        codec: None,
        seekable: true,
        complete: false,
        etag: None,
        last_modified: None,
        policy: "Allowed".into(),
        created_at_ms: 1,
        last_accessed_ms: 1,
        total_bytes: 0,
    })
    .unwrap();
}

/// §89/§130：chunk 写 + 读 + SHA-256 吞吐 + SQLite lookup。
#[test]
#[ignore = "perf baseline；显式运行"]
fn chunk_read_write_hash_sqlite() {
    let root = temp_root("rw");
    let store = ChunkStore::new(&root, Arc::new(RealFs)).unwrap();
    let db = CacheDb::open(&root.join("cache.db")).unwrap();
    let ck = content_key("https://perf.example/x.mp3").unwrap();
    make_entry(&db, &ck);
    let data: Vec<u8> = (0..CHUNK_SIZE).map(|i| (i % 251) as u8).collect();

    let t = std::time::Instant::now();
    for i in 0..100u64 {
        let rec = store.write_chunk(&ck, i, &data).unwrap();
        db.insert_chunk(&CacheChunkRow {
            content_key: ck.clone(),
            chunk_index: i,
            size: rec.size,
            sha256: rec.sha256,
            created_at_ms: 1,
            last_accessed_ms: 1,
        })
        .unwrap();
    }
    let write = t.elapsed();

    let t = std::time::Instant::now();
    for i in 0..100u64 {
        let _ = store.read_chunk(&ck, i).unwrap();
    }
    let read = t.elapsed();

    let t = std::time::Instant::now();
    for _ in 0..100 {
        let _ = sha256_hex(&data);
    }
    let hash = t.elapsed();

    let t = std::time::Instant::now();
    for i in 0..10_000u64 {
        let _ = db.get_chunk(&ck, i % 100);
    }
    let lookup = t.elapsed();

    println!("== M5 cache perf ==");
    println!(
        "256KiB write+db 100x : {write:?} ({:.0} MB/s)",
        (100 * CHUNK_SIZE) as f64 / write.as_secs_f64() / 1e6
    );
    println!(
        "256KiB read 100x     : {read:?} ({:.0} MB/s)",
        (100 * CHUNK_SIZE) as f64 / read.as_secs_f64() / 1e6
    );
    println!(
        "SHA-256 100x 256KiB  : {hash:?} ({:.0} MB/s)",
        (100 * CHUNK_SIZE) as f64 / hash.as_secs_f64() / 1e6
    );
    println!(
        "sqlite lookup 10000x : {lookup:?} ({:.2} us/次)",
        lookup.as_secs_f64() * 1e6 / 10_000.0
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// §130：LRU 淘汰耗时 + reopen + integrity 扫描。
#[test]
#[ignore = "perf baseline；显式运行"]
fn eviction_reopen_integrity() {
    let root = temp_root("lru");
    let db = CacheDb::open(&root.join("cache.db")).unwrap();
    let keys: Vec<String> = (0..200)
        .map(|i| content_key(&format!("https://perf.example/k{i}.mp3")).unwrap())
        .collect();
    for ck in &keys {
        make_entry(&db, ck);
        for idx in 0..3u64 {
            db.insert_chunk(&CacheChunkRow {
                content_key: ck.clone(),
                chunk_index: idx,
                size: CHUNK_SIZE as u64,
                sha256: sha256_hex(&[idx as u8; 32]),
                created_at_ms: 1,
                last_accessed_ms: 1,
            })
            .unwrap();
        }
    }
    let t = std::time::Instant::now();
    for i in 0..100 {
        let _ = db.lru_chunks(16.max(i as i64));
    }
    let lru_scan = t.elapsed();

    let t = std::time::Instant::now();
    let _ = CacheDb::open(&root.join("cache.db")).unwrap();
    let reopen = t.elapsed();

    // integrity：600 chunk 行 + 文件（写真实文件）。
    let store = ChunkStore::new(&root, Arc::new(RealFs)).unwrap();
    let data = vec![0u8; CHUNK_SIZE];
    for ck in keys.iter().take(50) {
        for idx in 0..3u64 {
            store.write_chunk(ck, idx, &data).unwrap();
        }
    }
    let m = aura_cache::manager::CacheManager::open(aura_cache::manager::CacheConfig::new(&root))
        .unwrap();
    let t = std::time::Instant::now();
    let report = m.verify_cache().unwrap();
    let integrity = t.elapsed();
    println!("== M5 integrity/lru/reopen ==");
    println!("lru scan 100x (600行) : {lru_scan:?}");
    println!("cache reopen          : {reopen:?}");
    println!(
        "integrity (50×3 chunk): {integrity:?} (missing={} repaired={})",
        report.missing, report.repaired
    );
    let _ = std::fs::remove_dir_all(&root);
}
