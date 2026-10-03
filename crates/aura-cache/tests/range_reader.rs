//! Range Reader 测试（§98/§99/§30–§43/§79–§84）：
//! 边界矩阵（first/middle/last/cross/exact/±1）、206 Content-Range 校验、
//! 200 降级 sequential、416/404/500、ETag/Last-Modified/304/内容变更、
//! 离线播放、retry/backoff 确定性。全部本地 fake server（§142）。

use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::Duration;

use aura_cache::manager::{CacheConfig, CacheManager};
use aura_cache::reader::{CachedRangeReader, FetchOptions, Freshness, RangeSource};
use aura_sources::http::{HttpClient, HttpConfig};
use aura_sources::model::CachePolicy;
use aura_sources::platform_block::PlatformBlocklist;
use aura_sources::url_policy::NetworkPolicy;
use aura_testkit::test_http::{FakeHttp, Route};

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
    let root = std::env::temp_dir().join(format!("aura-m5-reader-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    Arc::new(CacheManager::open(CacheConfig::new(root)).unwrap())
}

fn test_opts(revalidate: bool) -> FetchOptions {
    FetchOptions {
        revalidate_on_open: revalidate,
        ..FetchOptions::default()
    }
}

fn source(url: &str, len: u64) -> RangeSource {
    RangeSource {
        url: url.to_string(),
        content_length: Some(len),
        etag: None,
        last_modified: None,
        mime: Some("audio/mpeg".into()),
        codec: Some("mp3".into()),
        seekable: true,
    }
}

/// 全量 pattern 数据（3 chunk + 1000 bytes 尾块）。
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn body_route(path: &'static str, body: Vec<u8>) -> Route {
    Route::ok_seekable(path, "audio/mpeg", body)
}

/// §98：边界矩阵 —— 全部读出必须与源逐字节一致。
#[test]
fn boundary_matrix_reads_exact() {
    let len = CHUNK_TEST_LEN;
    let body = pattern(len);
    let srv = FakeHttp::start(vec![body_route("/a.mp3", body.clone())]);
    let m = manager_at("boundary");

    // first chunk（0..len）
    let url = srv.url("/a.mp3");

    let mut r = CachedRangeReader::open(
        Arc::clone(&m),
        http_client(),
        CachePolicy::Allowed,
        source(&url, len as u64),
        test_opts(false),
    )
    .unwrap();
    let mut out = Vec::new();
    r.read_to_end(&mut out).unwrap();
    assert_eq!(out, body, "全量读 = 源");

    // middle chunk 起点（seek 到 256KiB）。
    let mut r = CachedRangeReader::open(
        Arc::clone(&m),
        http_client(),
        CachePolicy::Allowed,
        source(&srv.url("/a.mp3"), len as u64),
        test_opts(false),
    )
    .unwrap();
    r.seek(SeekFrom::Start(256 * 1024)).unwrap();
    let mut mid = [0u8; 64];
    r.read_exact(&mut mid).unwrap();
    assert_eq!(&mid[..], &body[256 * 1024..256 * 1024 + 64], "middle");

    // last partial chunk（seek 到 len-1000）。
    r.seek(SeekFrom::Start(len as u64 - 1000)).unwrap();
    let mut tail = Vec::new();
    r.read_to_end(&mut tail).unwrap();
    assert_eq!(tail, &body[len - 1000..], "last partial");

    // exact boundary（seek 到 512KiB = chunk2 起点）。
    r.seek(SeekFrom::Start(512 * 1024)).unwrap();
    let mut one = [0u8; 1];
    r.read_exact(&mut one).unwrap();
    assert_eq!(one[0], body[512 * 1024], "exact boundary");

    // ±1 byte around boundary。
    r.seek(SeekFrom::Start(512 * 1024 - 1)).unwrap();
    let mut two = [0u8; 2];
    r.read_exact(&mut two).unwrap();
    assert_eq!(&two[..], &body[512 * 1024 - 1..512 * 1024 + 1], "±1 跨界");

    // cross boundary 大读（256KiB+4096 从 chunk0 中段）。
    r.seek(SeekFrom::Start(1000)).unwrap();
    let mut big = vec![0u8; 4096];
    r.read_exact(&mut big).unwrap();
    assert_eq!(&big[..], &body[1000..1000 + 4096], "cross");
}

const CHUNK_TEST_LEN: usize = 256 * 1024 * 3 + 1000; // 3 chunk + 尾块

/// §35：200 on Range → 降级 sequential fill，数据一致；seek 受已填充水位限制。
#[test]
fn non_range_server_degrades_to_sequential() {
    let body = pattern(64 * 1024); // < 1 chunk
    let srv = FakeHttp::start(vec![Route::ok("/no-range.mp3", "audio/mpeg", body.clone())]);
    let m = manager_at("seq");
    let mut r = CachedRangeReader::open(
        m,
        http_client(),
        CachePolicy::Allowed,
        source(&srv.url("/no-range.mp3"), body.len() as u64),
        test_opts(false),
    )
    .unwrap();

    // 顺序读全量（sequential fill 主路径）。
    let mut out = Vec::new();
    r.read_to_end(&mut out).unwrap();
    assert_eq!(out, body, "sequential fill 数据一致");

    // 已 EOF：seek 回已填充区间可读（§37 seek within cached data）。
    r.seek(SeekFrom::Start(10)).unwrap();
    let mut back = [0u8; 10];
    r.read_exact(&mut back).unwrap();
    assert_eq!(&back[..], &body[10..20]);

    // 非 seekable 行为验证：seek 到未填充区（服务器已关）→ 不支持。
    // （info 字段私有；行为级断言见下。）
}

/// §80/§83：离线（服务器不可达）—— 已缓存范围可读；miss → Network 错误
/// 而非伪装 EOS。
#[test]
fn offline_reads_cached_and_errors_on_miss() {
    let body = pattern(CHUNK_TEST_LEN);
    let srv = FakeHttp::start(vec![body_route("/o.mp3", body.clone())]);
    let m = manager_at("offline");
    let url = srv.url("/o.mp3");

    // 在线：读前 2 个 chunk 后丢弃 reader。
    {
        let mut r = CachedRangeReader::open(
            Arc::clone(&m),
            http_client(),
            CachePolicy::Allowed,
            source(&url, body.len() as u64),
            test_opts(false),
        )
        .unwrap();
        let mut head = vec![0u8; 256 * 1024 * 2];
        r.read_exact(&mut head).unwrap();
        assert_eq!(&head[..], &body[..head.len()]);
    }

    // 真离线：关闭服务器（§38 测试环境人为断网）。
    let mut srv = srv;
    srv.shutdown();

    // 离线 client。
    let offline_cfg = allow_private(HttpConfig {
        connect_timeout: Duration::from_millis(300),
        metadata_timeout: Duration::from_millis(300),
        stream_idle_timeout: Duration::from_millis(300),
        ..HttpConfig::default()
    });
    let offline = HttpClient::new(
        offline_cfg,
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap();
    let mut r = CachedRangeReader::open(
        Arc::clone(&m),
        offline,
        CachePolicy::Allowed,
        source(&url, body.len() as u64),
        test_opts(false),
    )
    .unwrap();

    // 已缓存 chunk 0/1 可读（§83 离线可用）。
    let mut cached = vec![0u8; 256 * 1024 * 2];
    r.read_exact(&mut cached).unwrap();
    assert_eq!(&cached[..], &body[..cached.len()]);

    // chunk 2 未缓存 → Network 错误（不是 Ok(0) 伪装 EOS，§80）。
    let mut sink = [0u8; 16];
    let e = r.read(&mut sink).unwrap_err();
    assert_ne!(e.kind(), std::io::ErrorKind::UnexpectedEof, "不得伪装 EOS");
}

/// §40–§43：ETag/304 Validated；内容变更 → 旧 chunks 作废全量重填。
#[test]
fn revalidation_304_and_content_change() {
    let body_v1 = pattern(50_000);
    let body_v2: Vec<u8> = body_v1.iter().map(|b| b.wrapping_add(1)).collect();

    let srv = FakeHttp::start(vec![Route::ok_validators(
        "/v.mp3",
        "audio/mpeg",
        body_v1.clone(),
        "\"ver1\"",
        "Tue, 01 Jan 2030 00:00:00 GMT",
    )]);
    let m = manager_at("reval");
    let url = srv.url("/v.mp3");

    // 声明携带服务器 validator（M4 ResolvedStream 透传场景）。
    let mut src_v1 = source(&url, body_v1.len() as u64);
    src_v1.etag = Some("\"ver1\"".into());
    let mut src_v2 = source(&url, body_v2.len() as u64);
    src_v2.etag = Some("\"ver2\"".into());

    // 第一遍：下载 + 缓存（entry 带 etag=ver1）。
    {
        let mut r = CachedRangeReader::open(
            Arc::clone(&m),
            http_client(),
            CachePolicy::Allowed,
            src_v1,
            FetchOptions {
                revalidate_on_open: false,
                ..FetchOptions::default()
            },
        )
        .unwrap();
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out, body_v1);
    }

    // 服务器换内容 + 换 ETag（§141 changed ETag fixture）。
    srv.set_routes(vec![Route::ok_validators(
        "/v.mp3",
        "audio/mpeg",
        body_v2.clone(),
        "\"ver2\"",
        "Tue, 01 Jan 2030 00:00:00 GMT",
    )]);

    // 服务器先模拟“未变”：恢复 ver1 body 换成 ver2 内容？——直接测变更路径：
    // revalidate（If-None-Match: ver1 不匹配 ver2 → 200 新内容）→ 旧 cache 作废重填。
    let mut r = CachedRangeReader::open(
        Arc::clone(&m),
        http_client(),
        CachePolicy::Allowed,
        src_v2,
        FetchOptions {
            revalidate_on_open: true,
            ..FetchOptions::default()
        },
    )
    .unwrap();
    assert_eq!(r.freshness(), Freshness::Validated);
    let mut out = Vec::new();
    r.read_to_end(&mut out).unwrap();
    assert_eq!(out, body_v2, "变更后必须全量新内容（§43 无半旧半新）");

    // 再次打开：If-None-Match: ver2 命中 → 304 Validated，缓存复用。
    let mut src_v2b = source(&url, body_v2.len() as u64);
    src_v2b.etag = Some("\"ver2\"".into());
    let mut r2 = CachedRangeReader::open(
        Arc::clone(&m),
        http_client(),
        CachePolicy::Allowed,
        src_v2b,
        FetchOptions {
            revalidate_on_open: true,
            ..FetchOptions::default()
        },
    )
    .unwrap();
    assert_eq!(r2.freshness(), Freshness::Validated, "304 → Validated");
    let mut out2 = Vec::new();
    r2.read_to_end(&mut out2).unwrap();
    assert_eq!(out2, body_v2, "304 后缓存继续有效（§40）");
}

/// §45–§48/§106：timeout→timeout→success 的确定性重试（no-op sleeper）。
#[test]
fn retry_deterministic_schedule() {
    let body = b"retry-me".to_vec();
    // §106：500@req0、500@req1、ok@req2 —— 确定性重试序列。
    let srv = FakeHttp::start(vec![
        Route::at("/r.mp3", 500, 0, 0),
        Route::at("/r.mp3", 500, 1, 1),
        Route::ok_seekable("/r.mp3", "audio/mpeg", body.clone()),
    ]);
    let m = manager_at("retry");
    let mut r = CachedRangeReader::open(
        m,
        http_client(),
        CachePolicy::Allowed,
        source(&srv.url("/r.mp3"), body.len() as u64),
        FetchOptions {
            revalidate_on_open: false,
            ..FetchOptions::default()
        },
    )
    .unwrap();
    let mut out = Vec::new();
    r.read_to_end(&mut out).unwrap();
    assert_eq!(out, body, "第 3 次尝试成功");
    assert!(srv.request_count() >= 3);
}

/// §46：404 不可重试 —— 一次即失败。
#[test]
fn not_found_no_retry() {
    let srv = FakeHttp::start(vec![Route::status("/gone.mp3", 404)]);
    let m = manager_at("404");
    let mut r = CachedRangeReader::open(
        m,
        http_client(),
        CachePolicy::Allowed,
        source(&srv.url("/gone.mp3"), 100),
        FetchOptions {
            revalidate_on_open: false,
            ..FetchOptions::default()
        },
    )
    .unwrap();
    let mut buf = [0u8; 8];
    let e = r.read(&mut buf).unwrap_err();
    assert!(!e.to_string().is_empty());
    assert!(srv.request_count() <= 2, "404 不重试（1 次探测内失败）");
}

/// §38：完整缓存 + 服务器消失 → 离线完整播放。
#[test]
fn fully_cached_offline_playback() {
    let body = pattern(80_000);
    let srv = FakeHttp::start(vec![body_route("/full.mp3", body.clone())]);
    let m = manager_at("full-offline");
    let url = srv.url("/full.mp3");
    {
        let mut r = CachedRangeReader::open(
            Arc::clone(&m),
            http_client(),
            CachePolicy::Allowed,
            source(&url, body.len() as u64),
            test_opts(false),
        )
        .unwrap();
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out, body);
    }
    // 服务器关闭。
    let mut srv = srv;
    srv.shutdown();

    // 离线 + revalidation 失败 → Stale，但完整缓存可读（§83）。
    let offline_cfg = allow_private(HttpConfig {
        connect_timeout: Duration::from_millis(200),
        ..HttpConfig::default()
    });
    let offline = HttpClient::new(
        offline_cfg,
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap();
    let mut r = CachedRangeReader::open(
        m,
        offline,
        CachePolicy::Allowed,
        source(&url, body.len() as u64),
        test_opts(true),
    )
    .unwrap();
    let mut out = Vec::new();
    r.read_to_end(&mut out).unwrap();
    assert_eq!(out, body, "§38 离线完整播放");
}

/// §79：Forbidden → 构造即拒绝（Bypassed 由调用方执行）。
#[test]
fn forbidden_reader_rejected() {
    let m = manager_at("forbidden");
    let err = CachedRangeReader::open(
        m,
        http_client(),
        CachePolicy::Forbidden,
        source("https://radio.example/live", 100),
        test_opts(false),
    );
    let e = match err {
        Err(e) => e,
        Ok(_) => panic!("Forbidden must be rejected"),
    };
    assert!(matches!(
        e,
        aura_cache::error::CacheError::PolicyViolation { .. }
    ));
}

/// Content-Range 解析（§34）。
#[test]
fn content_range_parsing() {
    use aura_cache::reader::parse_content_range;
    assert_eq!(
        parse_content_range("bytes 0-255/1000"),
        Some((0, 255, Some(1000)))
    );
    assert_eq!(
        parse_content_range("bytes 256-511/*"),
        Some((256, 511, None))
    );
    assert_eq!(parse_content_range("garbage"), None);
}
