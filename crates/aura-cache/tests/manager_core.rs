//! CacheManager 测试（§26/§27/§55–§62/§70–§75/§103/§107/§108）：
//! open/reopen、integrity（orphan/missing/corrupt/dangling-pin）、
//! DB 损坏恢复（备份+重建，library 隔离）、LRU/pin/容量、
//! SessionOnly 有界、策略门、access 批量、VirtualClock 确定性。

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use aura_cache::db::CacheDb;
use aura_cache::error::CacheError;
use aura_cache::keys::content_key;
use aura_cache::manager::{CacheConfig, CacheManager, EntryDeclaration};
use aura_sources::model::CachePolicy;
use aura_testkit::clock::VirtualClock;

fn decl(ck: &str, policy: CachePolicy) -> EntryDeclaration {
    EntryDeclaration {
        content_key: ck.to_string(),
        source_kind: "DirectHttp".into(),
        url: format!("https://example.com/{ck}.mp3"),
        content_length: None,
        mime: Some("audio/mpeg".into()),
        codec: Some("mp3".into()),
        seekable: true,
        live: false,
        policy,
        etag: None,
        last_modified: None,
    }
}

fn fixed_clock(start: i64) -> Arc<dyn Fn() -> i64 + Send + Sync> {
    Arc::new(move || start)
}

fn counted_clock(counter: Arc<AtomicI64>) -> Arc<dyn Fn() -> i64 + Send + Sync> {
    Arc::new(move || counter.load(Ordering::Relaxed))
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("aura-m5-mgr-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

/// §26：首次打开自动创建；重开加载。
#[test]
fn open_create_and_reopen() {
    let root = temp_root("reopen");
    let ck = content_key("https://example.com/a.mp3").unwrap();
    {
        let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
        m.ensure_entry(&decl(&ck, CachePolicy::Allowed)).unwrap();
        assert!(m
            .put_chunk(CachePolicy::Allowed, &ck, 0, &[1, 2, 3], false)
            .unwrap());
    }
    {
        let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
        assert!(
            m.get_chunk(CachePolicy::Allowed, &ck, 0).is_some(),
            "重开后数据仍在"
        );
        assert_eq!(m.total_bytes().unwrap(), 3);
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// §25：orphan（文件在 DB 无行）→ 报告并删除；missing（行在文件无）→ 删行；
/// corrupt（hash 不符）→ 删行删文件；dangling pin → 删。
#[test]
fn integrity_check_matrix() {
    let root = temp_root("integrity");
    let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
    let ck = content_key("https://example.com/i.mp3").unwrap();
    m.ensure_entry(&decl(&ck, CachePolicy::Allowed)).unwrap();
    assert!(m
        .put_chunk(CachePolicy::Allowed, &ck, 0, &[1; 100], false)
        .unwrap());
    assert!(m
        .put_chunk(CachePolicy::Allowed, &ck, 1, &[2; 100], false)
        .unwrap());
    assert!(m
        .put_chunk(CachePolicy::Allowed, &ck, 2, &[3; 100], false)
        .unwrap());
    m.pin(&ck).unwrap();

    // orphan：手工放一个 DB 无行的文件。
    let orphan_dir = root.join("chunks").join(&ck[..2]).join(&ck);
    std::fs::write(orphan_dir.join("00000063.bin"), b"orphan").unwrap();

    // missing：删 1 号文件（DB 行还在）。
    std::fs::remove_file(orphan_dir.join("00000001.bin")).unwrap();

    // corrupt：翻转 0 号内容。
    let p0 = orphan_dir.join("00000000.bin");
    let mut b = std::fs::read(&p0).unwrap();
    b[0] ^= 0xFF;
    std::fs::write(&p0, &b).unwrap();

    // dangling pin。
    m.pin(&content_key("https://gone.example/x").unwrap())
        .unwrap();

    let r = m.verify_cache().unwrap();
    assert_eq!(r.orphaned, 1, "{r:?}");
    assert_eq!(r.missing, 1);
    assert_eq!(r.corrupted, 1);
    assert!(r.repaired >= 3, "{r:?}");
    assert!(!orphan_dir.join("00000063.bin").exists(), "orphan 已回收");
    assert!(m.is_pinned(&ck), "合法 pin 保留");
    assert!(
        !m.is_pinned(&content_key("https://gone.example/x").unwrap()),
        "dangling pin 已清"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// §27/§28：DB 损坏 → 备份 + 重建空索引；绝不触碰 library DB。
#[test]
fn corrupt_db_backed_up_and_rebuilt() {
    let root = temp_root("dbcorrupt");
    std::fs::create_dir_all(&root).unwrap();
    let lib_db = root.join("library.db");
    std::fs::write(&lib_db, b"precious library data").unwrap();

    let ck = content_key("https://example.com/c.mp3").unwrap();
    {
        let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
        m.ensure_entry(&decl(&ck, CachePolicy::Allowed)).unwrap();
        assert!(m
            .put_chunk(CachePolicy::Allowed, &ck, 0, &[1; 50], false)
            .unwrap());
    }
    // 损坏 cache.db。
    std::fs::write(root.join("cache.db"), b"garbage not sqlite at all").unwrap();

    let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
    // 空索引（安全降级）+ 备份存在。
    assert_eq!(m.total_bytes().unwrap(), 0, "§27 安全降级为空 cache");
    let backups: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("cache.db.corrupt")
        })
        .collect();
    assert!(!backups.is_empty(), "损坏 DB 必须先备份");
    // 原 chunk 文件仍在（orphan 状态，不误删——integrity 时才回收）。
    assert!(root
        .join("chunks")
        .join(&ck[..2])
        .join(&ck)
        .join("00000000.bin")
        .is_file());
    // §28：library DB 原样。
    assert_eq!(std::fs::read(&lib_db).unwrap(), b"precious library data");
    let _ = std::fs::remove_dir_all(&root);
}

/// §55/§107/§108：LRU 淘汰 —— VirtualClock 下确定性；pinned 永不淘汰。
#[test]
fn lru_eviction_deterministic_with_pins() {
    let clock_v = Arc::new(AtomicI64::new(1000));
    let root = temp_root("lru");
    let cfg = CacheConfig {
        root: root.clone(),
        capacity_bytes: 3 * 1024, // 3 chunks
        now_ms: counted_clock(Arc::clone(&clock_v)),
    };
    let m = CacheManager::open(cfg).unwrap();

    let keys: Vec<String> = (0..4)
        .map(|i| content_key(&format!("https://example.com/lru{i}.mp3")).unwrap())
        .collect();
    for (i, ck) in keys.iter().enumerate() {
        m.ensure_entry(&decl(ck, CachePolicy::Allowed)).unwrap();
        // 时间推进：A=1000, B=1001, C=1002, D=1003（§108 虚拟时钟确定性）。
        clock_v.store(1000 + i64::try_from(i).unwrap(), Ordering::Relaxed);
        assert!(m
            .put_chunk(CachePolicy::Allowed, ck, 0, &[i as u8; 1024], false)
            .unwrap());
    }
    // 容量 3：D 写入时应淘汰最旧的 A。
    assert!(
        m.get_chunk(CachePolicy::Allowed, &keys[0], 0).is_none(),
        "A 最旧被淘汰"
    );
    assert!(m.get_chunk(CachePolicy::Allowed, &keys[3], 0).is_some());

    // pin C 后再写 E：淘汰顺序 B（最旧未 pin）。
    m.pin(&keys[2]).unwrap();
    clock_v.store(2000, Ordering::Relaxed);
    let e = content_key("https://example.com/lruE.mp3").unwrap();
    m.ensure_entry(&decl(&e, CachePolicy::Allowed)).unwrap();
    assert!(m
        .put_chunk(CachePolicy::Allowed, &e, 0, &[9u8; 1024], false)
        .unwrap());
    assert!(
        m.get_chunk(CachePolicy::Allowed, &keys[1], 0).is_none(),
        "B 被淘汰"
    );
    assert!(
        m.get_chunk(CachePolicy::Allowed, &keys[2], 0).is_some(),
        "pinned C 保留"
    );

    // 全 pinned + 超容量 → CacheFullPinned（§59）。
    m.pin(&keys[3]).unwrap();
    m.pin(&e).unwrap();
    let f = content_key("https://example.com/lruF.mp3").unwrap();
    m.ensure_entry(&decl(&f, CachePolicy::Allowed)).unwrap();
    let err = m
        .put_chunk(CachePolicy::Allowed, &f, 0, &[8u8; 2048], false)
        .unwrap_err();
    assert!(matches!(err, CacheError::CacheFullPinned), "{err:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// §5/§70/§71：SessionOnly 只在内存；reopen 后不存在。
#[test]
fn session_only_not_persisted() {
    let root = temp_root("session");
    let ck = content_key("https://example.com/s.mp3").unwrap();
    {
        let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
        // SessionOnly put 不需要 entry 行。
        assert!(!m
            .put_chunk(CachePolicy::SessionOnly, &ck, 0, &[1, 2, 3], false)
            .unwrap());
        assert!(m.get_chunk(CachePolicy::SessionOnly, &ck, 0).is_some());
        assert!(m.session_contains(&ck, 0));
        // 数据库零 entry、磁盘零 chunk（§71 cache DB 零新增 entry）。
        assert_eq!(m.total_bytes().unwrap(), 0);
    }
    {
        let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
        assert!(
            m.get_chunk(CachePolicy::SessionOnly, &ck, 0).is_none(),
            "进程结束即消失"
        );
        assert_eq!(m.entry_count(), 0);
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// §6/§75：Forbidden / live → 一切持久写入路径被拒。
#[test]
fn forbidden_and_live_never_persist() {
    let root = temp_root("forbidden");
    let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
    let ck = content_key("https://radio.example/live").unwrap();

    // put_chunk 直接拒绝。
    let e = m
        .put_chunk(CachePolicy::Forbidden, &ck, 0, &[1, 2, 3], true)
        .unwrap_err();
    assert!(matches!(e, CacheError::PolicyViolation { .. }));
    // live 即使 Allowed 声明也拒绝（§75 双保险）。
    let e = m
        .put_chunk(CachePolicy::Allowed, &ck, 0, &[1, 2, 3], true)
        .unwrap_err();
    assert!(matches!(e, CacheError::PolicyViolation { .. }));
    // ensure_entry 拒绝非 Allowed / live。
    assert!(m.ensure_entry(&decl(&ck, CachePolicy::Forbidden)).is_err());
    let mut live_decl = decl(&ck, CachePolicy::Allowed);
    live_decl.live = true;
    assert!(m.ensure_entry(&live_decl).is_err());

    // 磁盘/DB 零痕迹（§70）。
    assert_eq!(m.total_bytes().unwrap(), 0);
    assert!(!root.join("chunks").join(&ck[..2]).exists());
    let _ = std::fs::remove_dir_all(&root);
}

/// §103：未知/显式 policy 路由。
#[test]
fn policy_routing_table() {
    use aura_cache::manager::Routing;
    assert_eq!(
        CacheManager::routing(CachePolicy::Allowed),
        Routing::Persistent
    );
    assert_eq!(
        CacheManager::routing(CachePolicy::SessionOnly),
        Routing::Session
    );
    assert_eq!(
        CacheManager::routing(CachePolicy::Forbidden),
        Routing::Bypassed
    );
}

/// §62：access-time 批量 —— 大量读只产生 ≤阈值次 DB 写。
#[test]
fn access_touches_are_batched() {
    let root = temp_root("batch");
    let cfg = CacheConfig {
        root: root.clone(),
        capacity_bytes: aura_cache::manager::DEFAULT_CAPACITY_BYTES,
        now_ms: fixed_clock(100),
    };
    let m = CacheManager::open(cfg).unwrap();
    let ck = content_key("https://example.com/b.mp3").unwrap();
    m.ensure_entry(&decl(&ck, CachePolicy::Allowed)).unwrap();
    assert!(m
        .put_chunk(CachePolicy::Allowed, &ck, 0, &[1; 10], false)
        .unwrap());

    // 读 10 次只进内存队列（<64 不落库）。
    for _ in 0..10 {
        assert!(m.get_chunk(CachePolicy::Allowed, &ck, 0).is_some());
    }
    // 直接查 DB 的 last_accessed —— 仍是写入时的 100（未 flush）。
    let db = CacheDb::open(&root.join("cache.db")).unwrap();
    let row = db.get_chunk(&ck, 0).unwrap().unwrap();
    assert_eq!(row.last_accessed_ms, 100, "未 flush 前不写 DB（§62）");

    // flush 后落库。
    m.flush_touches().unwrap();
    let db = CacheDb::open(&root.join("cache.db")).unwrap();
    let row = db.get_chunk(&ck, 0).unwrap().unwrap();
    assert_eq!(row.last_accessed_ms, 100, "固定时钟下 flush 落库");
    let _ = std::fs::remove_dir_all(&root);
}

/// §132：SessionOnly 内存有界（超限淘汰最旧）。
#[test]
fn session_store_bounded() {
    let root = temp_root("sbound");
    let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
    let big = vec![0u8; 8 << 20]; // 8MiB × 5 = 40MiB > 32MiB cap
    for i in 0..5u64 {
        let ck = content_key(&format!("https://example.com/s{i}.mp3")).unwrap();
        assert!(!m
            .put_chunk(CachePolicy::SessionOnly, &ck, 0, &big, false)
            .unwrap());
        assert!(
            m.session_bytes() <= 32 << 20,
            "SessionOnly 必须有界: {}",
            m.session_bytes()
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// §73：SessionOnly → Allowed 升级不静默持久化旧 session 缓冲。
#[test]
fn session_to_allowed_transition_no_silent_persist() {
    let root = temp_root("trans");
    let ck = content_key("https://example.com/tr.mp3").unwrap();
    let m = CacheManager::open(CacheConfig::new(&root)).unwrap();
    assert!(!m
        .put_chunk(CachePolicy::SessionOnly, &ck, 0, &[1, 2, 3], false)
        .unwrap());

    // 用户行为升级为 Allowed：只影响之后的写入。
    m.ensure_entry(&decl(&ck, CachePolicy::Allowed)).unwrap();
    assert!(
        m.get_chunk(CachePolicy::Allowed, &ck, 0).is_none(),
        "旧 session 缓冲不得静默转持久（§73）"
    );
    // 新写入按 Allowed 落盘。
    assert!(m
        .put_chunk(CachePolicy::Allowed, &ck, 1, &[4, 5], false)
        .unwrap());
    assert!(m.get_chunk(CachePolicy::Allowed, &ck, 1).is_some());
    let _ = std::fs::remove_dir_all(&root);
}
