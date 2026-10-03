//! 解析顺序测试（§12/§13/§23/§101 部分）：本地文件 / scheme 白名单 /
//! 封锁平台先于网络（zero-request）/ 未知输入。网络探测类用例在
//! http_matrix / radio 测试中覆盖。

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use aura_sources::error::SourceError;
use aura_sources::http::{HttpClient, HttpConfig};
use aura_sources::model::{CachePolicy, SeekCapability, SourceKind, StreamType};
use aura_sources::platform_block::PlatformBlocklist;
use aura_sources::resolver::resolve_input;
use aura_sources::url_policy::NetworkPolicy;
use common::FakeHttp;

fn fixtures(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn allow_private(mut cfg: HttpConfig) -> HttpConfig {
    cfg.policy = NetworkPolicy::allow_private();
    cfg
}

/// §23/§24：本地 resolve —— metadata 来自 aura-library（不重复实现），
/// Seekable + Finite + CachePolicy::Allowed。
#[test]
fn local_file_resolution() {
    let mut cfg = HttpConfig::default();
    cfg.dns_override = Some(Arc::new(|_h| {
        Err(std::io::Error::other("no network for local resolution"))
    }));
    let http = HttpClient::new(cfg, Arc::new(PlatformBlocklist::embedded().unwrap())).unwrap();
    let path = fixtures("sine_8k.flac");
    let r = resolve_input(
        &path.to_string_lossy(),
        &http,
        &PlatformBlocklist::embedded().unwrap(),
    )
    .unwrap();

    assert_eq!(r.kind, SourceKind::Local);
    assert_eq!(r.streams.len(), 1);
    let s = &r.streams[0];
    assert_eq!(s.seekable, SeekCapability::Seekable);
    assert_eq!(s.stream_type, StreamType::Finite);
    assert_eq!(s.codec.as_deref(), Some("flac"));
    assert_eq!(r.cache_policy, CachePolicy::Allowed);
    assert_eq!(r.meta.track.title, None, "无标签 fixture：不猜 title");
    assert_eq!(
        r.license,
        aura_sources::model::ResolvedLicense::Unknown,
        "不猜 license"
    );
    assert_eq!(
        r.attribution,
        aura_sources::model::Attribution::default(),
        "不编造署名"
    );
}

#[test]
fn local_missing_file_is_io_error() {
    let http = HttpClient::new(
        allow_private(HttpConfig::default()),
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap();
    let e = resolve_input(
        "Z:/definitely/missing/song.flac",
        &http,
        &PlatformBlocklist::embedded().unwrap(),
    )
    .unwrap_err();
    assert!(matches!(e, SourceError::Io { .. }), "{e:?}");
}

/// §13：URL scheme 白名单在 resolver 层同样生效。
#[test]
fn resolver_rejects_disallowed_schemes() {
    let http = HttpClient::new(
        allow_private(HttpConfig::default()),
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap();
    for bad in [
        "ftp://example.com/a.mp3",
        "file:///C:/music/a.flac",
        "data:audio/mpeg;base64,AA",
        "javascript:alert(1)",
    ] {
        let e = resolve_input(bad, &http, &PlatformBlocklist::embedded().unwrap()).unwrap_err();
        assert!(matches!(e, SourceError::InvalidUrl { .. }), "{bad}: {e:?}");
    }
}

/// §3/§87/§125：封锁平台在 resolver 层拒绝且零请求（DNS spy 不被触发）。
#[test]
fn resolver_blocks_platform_zero_request() {
    let spy = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let spy2 = Arc::clone(&spy);
    let mut cfg = allow_private(HttpConfig::default());
    cfg.dns_override = Some(Arc::new(move |_h| {
        spy2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(vec!["127.0.0.1".parse().unwrap()])
    }));
    let http = HttpClient::new(cfg, Arc::new(PlatformBlocklist::embedded().unwrap())).unwrap();
    let e = resolve_input(
        "https://music.163.com/song?id=1",
        &http,
        &PlatformBlocklist::embedded().unwrap(),
    )
    .unwrap_err();
    let SourceError::UnsupportedPlatform { platform, .. } = e else {
        panic!("expected UnsupportedPlatform, got {e:?}")
    };
    assert_eq!(platform, "netease-music");
    assert_eq!(
        spy.load(std::sync::atomic::Ordering::Relaxed),
        0,
        "零 DNS/网络动作"
    );
}

/// §12：未知输入明确报错。
#[test]
fn unknown_input_rejected() {
    let http = HttpClient::new(
        allow_private(HttpConfig::default()),
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap();
    for input in ["", "   ", "just some words", "rb:unknown-station"] {
        let e = resolve_input(input, &http, &PlatformBlocklist::embedded().unwrap()).unwrap_err();
        // 未知输入一律明确报错：纯文本/空 → UnsupportedInput；
        // 未注册 scheme（rb:）→ InvalidUrl（scheme 不允许）。两者都是确定性拒绝。
        assert!(
            matches!(
                e,
                SourceError::UnsupportedInput { .. } | SourceError::InvalidUrl { .. }
            ),
            "{input:?}: {e:?}"
        );
    }
}

/// m3u8 在 M4-A 可识别但不实现（明确错误信息）。
#[test]
fn hls_input_reports_tier_b_status() {
    let srv = FakeHttp::start(vec![]);
    let http = HttpClient::new(
        allow_private(HttpConfig::default()),
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap();
    let e = resolve_input(
        &srv.url("/live.m3u8"),
        &http,
        &PlatformBlocklist::embedded().unwrap(),
    )
    .unwrap_err();
    assert!(matches!(e, SourceError::UnsupportedInput { .. }), "{e:?}");
    assert_eq!(srv.request_count(), 0, "未实现来源不产生请求");
}

/// direct vs radio 形态判定：icy 头 → Live/Forbidden；range+len → Seekable/Finite。
#[test]
fn http_probe_distinguishes_radio_and_direct() {
    let radio_body = vec![0x55u8; 64];
    let srv = FakeHttp::start(vec![
        common::Route {
            path: "/live",
            status: 200,
            headers: vec![
                ("Content-Type".into(), "audio/mpeg".into()),
                ("icy-name".into(), "Test Radio".into()),
                ("icy-br".into(), "128".into()),
                ("icy-metaint".into(), "8192".into()),
            ],
            body: radio_body,
            delay_ms: 0,
            malformed: false,
        },
        common::Route {
            path: "/file.mp3",
            status: 200,
            headers: vec![
                ("Content-Type".into(), "audio/mpeg".into()),
                ("Accept-Ranges".into(), "bytes".into()),
                ("Content-Length".into(), "64".into()),
            ],
            body: vec![0x55u8; 64],
            delay_ms: 0,
            malformed: false,
        },
    ]);
    let http = HttpClient::new(
        allow_private(HttpConfig::default()),
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap();

    let radio = resolve_input(
        &srv.url("/live"),
        &http,
        &PlatformBlocklist::embedded().unwrap(),
    )
    .unwrap();
    assert_eq!(radio.kind, SourceKind::InternetRadio);
    assert_eq!(radio.streams[0].stream_type, StreamType::Live);
    assert_eq!(radio.streams[0].seekable, SeekCapability::NonSeekable);
    assert_eq!(radio.cache_policy, CachePolicy::Forbidden, "§94");
    assert_eq!(radio.streams[0].bitrate_bps, Some(128_000));
    assert_eq!(radio.meta.stream.icy_name.as_deref(), Some("Test Radio"));

    let direct = resolve_input(
        &srv.url("/file.mp3"),
        &http,
        &PlatformBlocklist::embedded().unwrap(),
    )
    .unwrap();
    assert_eq!(direct.kind, SourceKind::DirectHttp);
    assert_eq!(direct.streams[0].stream_type, StreamType::Finite);
    assert_eq!(direct.streams[0].seekable, SeekCapability::Seekable);
    assert_eq!(direct.cache_policy, CachePolicy::SessionOnly, "§94");
}
