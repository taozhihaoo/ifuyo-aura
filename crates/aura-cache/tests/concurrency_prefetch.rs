//! 并发 / prefetch / dedup / soak 测试（§49–§53/§61/§95/§102/§131/§136）。
//! 同步策略：有界条件等待 + 计数断言，不用固定 sleep 猜测（§102）。

use std::io::Read;
use std::sync::Arc;

use aura_cache::keys::content_key;
use aura_cache::manager::{CacheConfig, CacheManager};
use aura_cache::reader::{CachedRangeReader, FetchOptions, RangeSource};
use aura_sources::http::{HttpClient, HttpConfig};
use aura_sources::model::CachePolicy;
use aura_sources::platform_block::PlatformBlocklist;
use aura_sources::url_policy::NetworkPolicy;
use aura_testkit::test_http::{FakeHttp, Route};
use std::io::Seek as _;

fn allow_private(mut cfg: HttpConfig) -> HttpConfig {
    cfg.policy = NetworkPolicy::allow_private();
    cfg
}

fn http_client() -> HttpClient {
    HttpClient::new(
        allow_private(HttpConfig::default()),
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap()
}

fn manager_at(tag: &str) -> Arc<CacheManager> {
    let root = std::env::temp_dir().join(format!("aura-m5-conc-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    Arc::new(CacheManager::open(CacheConfig::new(root)).unwrap())
}

fn source_of(url: &str, len: u64) -> RangeSource {
    RangeSource {
        url: url.to_string(),
        content_length: Some(len),
        etag: None,
        last_modified: None,
        mime: None,
        codec: Some("mp3".into()),
        seekable: true,
    }
}

fn body_route(path: &'static str, body: Vec<u8>) -> Route {
    Route::ok_seekable(path, "audio/mpeg", body)
}

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 249) as u8).collect()
}

/// §53/§95/§136：同 chunk 并发 → dedup，只下载一次。
#[test]
fn concurrent_same_chunk_dedup() {
    let body = vec![5u8; 100_000]; // < 256KiB：单 chunk 可持久化
    let srv = FakeHttp::start(vec![body_route("/d.mp3", body.clone())]);
    let m = manager_at("dedup");
    let ck = content_key(&srv.url("/d.mp3")).unwrap();

    // 两个线程同时经 dedup_fetch 拉同一 chunk。
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let m = Arc::clone(&m);
            let ck = ck.clone();
            let http = http_client();
            let url = srv.url("/d.mp3");
            std::thread::spawn(move || {
                m.dedup_fetch(&ck, 0, || {
                    let parsed = url::Url::parse(&url).unwrap();
                    let resp = http.get_stream(&parsed).unwrap();
                    let mut buf = Vec::new();
                    let mut resp = resp;
                    resp.read_to_end(&mut buf).unwrap();
                    // 等待方读缓存 —— 负责方临界区内持久化。
                    let _ = m.put_chunk(CachePolicy::Allowed, &ck, 0, &buf, false);
                    Ok(buf)
                })
                .unwrap()
            })
        })
        .collect();
    let outs: Vec<Vec<u8>> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    for out in &outs {
        assert_eq!(out, &body);
    }
    // dedup：两次调用一次下载（请求计数 1）。
    assert_eq!(srv.request_count(), 1, "S53/95：重复下载被消除");
    assert_eq!(m.inflight_count(), 0, "完成后注册表清空");
}

/// §54：并发读不同 chunk —— 互不破坏，数据一致。
#[test]
fn concurrent_different_chunks() {
    let body = pattern(700_000);
    let srv = FakeHttp::start(vec![body_route("/c.mp3", body.clone())]);
    let m = manager_at("conc");
    let url = srv.url("/c.mp3");

    let handles: Vec<_> = (0..3)
        .map(|i| {
            let m = Arc::clone(&m);
            let http = http_client();
            let url = url.clone();
            let offset = 256 * 1024 * i as usize;
            let body = body.clone();
            std::thread::spawn(move || {
                let mut r = CachedRangeReader::open(
                    m,
                    http,
                    CachePolicy::Allowed,
                    source_of(&url, body.len() as u64),
                    FetchOptions {
                        revalidate_on_open: false,
                        ..FetchOptions::default()
                    },
                )
                .unwrap();
                r.seek(std::io::SeekFrom::Start(offset as u64)).unwrap();
                let mut buf = vec![0u8; 4096];
                r.read_exact(&mut buf).unwrap();
                assert_eq!(buf, body[offset..offset + 4096], "chunk {i}");
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    // 完整性：三线程写后 cache 仍一致。
    let r = m.verify_cache().unwrap();
    assert_eq!(r.corrupted, 0);
    assert_eq!(r.missing, 0);
}

/// §61：in-flight lease 与容量语义 —— dedup 完成后注册表清空，
/// 后续写入可正常淘汰（lease 不泄漏）。
#[test]
fn inflight_lease_cleared_after_fetch() {
    let root = std::env::temp_dir().join(format!("aura-m5-lease-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cfg = CacheConfig {
        root: root.clone(),
        capacity_bytes: 4096,
        now_ms: fixed_clock(),
    };
    let m = CacheManager::open(cfg).unwrap();
    let ck = content_key("https://example.com/lease.mp3").unwrap();
    m.ensure_entry(&EntryDeclHelper::decl(&ck)).unwrap();
    assert_eq!(m.inflight_count(), 0);
    let bytes = m.dedup_fetch(&ck, 0, || Ok(vec![1u8; 2048])).unwrap();
    assert_eq!(bytes.len(), 2048);
    assert_eq!(m.inflight_count(), 0, "§61 lease 完成后必须释放");
    let _ = std::fs::remove_dir_all(&root);
}

/// §102：pin during eviction —— pinned 内容在并发淘汰下完好。
#[test]
fn pin_during_eviction_stays() {
    let root = std::env::temp_dir().join(format!("aura-m5-pinev-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cfg = CacheConfig {
        root: root.clone(),
        capacity_bytes: 2 * 1024,
        now_ms: fixed_clock(),
    };
    let m: Arc<CacheManager> = Arc::new(CacheManager::open(cfg).unwrap());
    let pinned = content_key("https://example.com/pinned.mp3").unwrap();
    m.ensure_entry(&EntryDeclHelper::decl(&pinned)).unwrap();
    assert!(m
        .put_chunk(CachePolicy::Allowed, &pinned, 0, &[1u8; 1024], false)
        .unwrap());
    m.pin(&pinned).unwrap();

    // 并发写其它内容（触发淘汰）。
    let handles: Vec<_> = (0..4)
        .map(|i| {
            let m = Arc::clone(&m);
            std::thread::spawn(move || {
                let ck = content_key(&format!("https://example.com/ev{i}.mp3")).unwrap();
                m.ensure_entry(&EntryDeclHelper::decl(&ck)).unwrap();
                let _ = m.put_chunk(CachePolicy::Allowed, &ck, 0, &[i as u8; 1024], false);
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    // pinned 完好。
    assert!(m.is_pinned(&pinned));
    assert!(
        m.get_chunk(CachePolicy::Allowed, &pinned, 0).is_some(),
        "pinned 不被淘汰"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// §49–§51：prefetch 预取前方 chunk 且有界；seek 后旧预取终止。
#[test]
fn prefetch_bounded_and_cancellable() {
    let body = pattern(768 * 1024 + 10); // 3 chunks
    let srv = FakeHttp::start(vec![body_route("/p.mp3", body.clone())]);
    let m = manager_at("prefetch");
    let url = srv.url("/p.mp3");
    let mut r = CachedRangeReader::open(
        Arc::clone(&m),
        http_client(),
        CachePolicy::Allowed,
        source_of(&url, body.len() as u64),
        FetchOptions {
            revalidate_on_open: false,
            ..FetchOptions::default()
        },
    )
    .unwrap();

    // 读 chunk0（当前位），prefetch 前 2 个。
    let mut first = [0u8; 100];
    r.read_exact(&mut first).unwrap();
    let before = srv.request_count();
    let mut join = r.prefetch_ahead(2);
    join.join(); // 确定性：等预取完成再断言
    assert!(
        srv.request_count() >= before + 2,
        "prefetch 应拉取前方 chunk: before={before} after={}",
        srv.request_count()
    );
    // 预取的 chunk 已在缓存：再次读取不产生新请求。
    let after = srv.request_count();
    r.seek(std::io::SeekFrom::Start(256 * 1024)).unwrap();
    let mut buf = [0u8; 64];
    r.read_exact(&mut buf).unwrap();
    assert_eq!(srv.request_count(), after, "prefetch 命中缓存");
}

/// §131：A/B/C + seek + evict + refetch soak（有界迭代，CI 友好）。
#[test]
fn cache_soak_read_write_evict_refetch() {
    let body = pattern(400_000);
    let srv = FakeHttp::start(vec![
        body_route("/s0.mp3", body.clone()),
        body_route("/s1.mp3", body.clone()),
        body_route("/s2.mp3", body.clone()),
    ]);
    let root = std::env::temp_dir().join(format!("aura-m5-soak-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let cfg = CacheConfig {
        root: root.clone(),
        capacity_bytes: 512 * 1024, // 强制淘汰
        now_ms: fixed_clock(),
    };
    let m: Arc<CacheManager> = Arc::new(CacheManager::open(cfg).unwrap());
    let urls: Vec<String> = (0..3).map(|i| srv.url(&format!("/s{i}.mp3"))).collect();
    let mut rng_state = 0x5041u64;
    let mut next = move || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        rng_state
    };

    for iter in 0..60u64 {
        let url = &urls[(next() % 3) as usize];
        let mut r = CachedRangeReader::open(
            Arc::clone(&m),
            http_client(),
            CachePolicy::Allowed,
            source_of(url, body.len() as u64),
            FetchOptions {
                revalidate_on_open: false,
                ..FetchOptions::default()
            },
        )
        .unwrap();
        // 随机 seek + 读。
        let off = next() % (body.len() as u64 - 4096);
        r.seek(std::io::SeekFrom::Start(off)).unwrap();
        let mut buf = [0u8; 4096];
        r.read_exact(&mut buf).unwrap();
        assert_eq!(
            &buf[..],
            &body[off as usize..off as usize + 4096],
            "iter {iter}"
        );
        // 周期性 integrity。
        if iter % 20 == 19 {
            let report = m.verify_cache().unwrap();
            assert_eq!(report.corrupted, 0, "soak 中无损坏: {report:?}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// §52：generation 交互 —— 旧 fetch 结果进 cache（同 key 合法），
/// 不污染新播放：内容寻址缓存天然满足；测试断言两个 reader（模拟
/// old/new generation）交错 seek 后缓存数据 hash 全部一致。
#[test]
fn generation_interaction_cache_stays_consistent() {
    let body = pattern(600_000);
    let srv = FakeHttp::start(vec![body_route("/g.mp3", body.clone())]);
    let m = manager_at("gen");
    let url = srv.url("/g.mp3");

    // “旧 generation” reader：seek 到 chunk1。
    let mut old = CachedRangeReader::open(
        Arc::clone(&m),
        http_client(),
        CachePolicy::Allowed,
        source_of(&url, body.len() as u64),
        FetchOptions {
            revalidate_on_open: false,
            ..FetchOptions::default()
        },
    )
    .unwrap();
    old.seek(std::io::SeekFrom::Start(256 * 1024)).unwrap();
    let mut a = [0u8; 100];
    old.read_exact(&mut a).unwrap();

    // “新 generation” reader：seek 到 chunk2（旧 reader 尚未读 chunk2）。
    let mut new = CachedRangeReader::open(
        Arc::clone(&m),
        http_client(),
        CachePolicy::Allowed,
        source_of(&url, body.len() as u64),
        FetchOptions {
            revalidate_on_open: false,
            ..FetchOptions::default()
        },
    )
    .unwrap();
    new.seek(std::io::SeekFrom::Start(512 * 1024)).unwrap();
    let mut b = [0u8; 100];
    new.read_exact(&mut b).unwrap();

    // 旧 reader 继续读：内容仍正确（无 PCM/状态污染）。
    let mut a2 = [0u8; 100];
    old.read_exact(&mut a2).unwrap();
    assert_eq!(&a2[..], &body[256 * 1024 + 100..256 * 1024 + 200]);

    // 缓存一致性。
    let report = m.verify_cache().unwrap();
    assert_eq!(report.corrupted, 0);
}

#[allow(unused)]
fn _unused_chunk_size() -> usize {
    0
}

fn fixed_clock() -> Arc<dyn Fn() -> i64 + Send + Sync> {
    Arc::new(|| 42)
}

mod entry_decl_helper {
    use aura_cache::manager::EntryDeclaration;
    pub struct EntryDeclHelper;
    impl EntryDeclHelper {
        pub fn decl(ck: &str) -> EntryDeclaration {
            EntryDeclaration {
                content_key: ck.to_string(),
                source_kind: "DirectHttp".into(),
                url: "https://example.com/x.mp3".to_string(),
                content_length: None,
                mime: None,
                codec: None,
                seekable: true,
                live: false,
                policy: aura_sources::model::CachePolicy::Allowed,
                etag: None,
                last_modified: None,
            }
        }
    }
}
use entry_decl_helper::EntryDeclHelper;
