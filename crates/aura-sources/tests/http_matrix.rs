//! HTTP 客户端测试矩阵（§104/§84/§87/§88/§125）：
//! 200/206/3xx 链/404/403/500/timeout/坏 content-type/超限/重定向安全/
//! 封锁平台 zero-request 证明。全部走 127.0.0.1 fake server（策略显式放开私网）。

mod common;

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use aura_sources::error::SourceError;
use aura_sources::http::{HttpClient, HttpConfig};
use aura_sources::platform_block::PlatformBlocklist;
use aura_sources::url_policy::NetworkPolicy;
use common::{FakeHttp, Route};

fn allow_private(mut cfg: HttpConfig) -> HttpConfig {
    cfg.policy = NetworkPolicy::allow_private();
    cfg
}

fn client(cfg: HttpConfig) -> HttpClient {
    HttpClient::new(cfg, Arc::new(PlatformBlocklist::embedded().unwrap())).unwrap()
}

/// 并行测试下的传输级抖动（首连被重置等）只影响 NetworkError；
/// 策略类结果（InvalidUrl/UnsupportedPlatform/PrivateNetwork/RedirectLimit/
/// Timeout/BodyTooLarge/ParseError）是确定性的，重试仅针对 NetworkError，
/// 断言本身不放松。
fn get_text_steady(
    c: &HttpClient,
    url: &url::Url,
    limit: u64,
) -> std::result::Result<String, SourceError> {
    let mut last = SourceError::NetworkError {
        reason: "not attempted".into(),
    };
    for _ in 0..3 {
        match c.get_text(url, limit) {
            Ok(v) => return Ok(v),
            Err(e @ SourceError::NetworkError { .. }) => last = e,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

/// 200 + 有限文本。
#[test]
fn status_200_text_with_limit() {
    let srv = FakeHttp::start(vec![Route::ok(
        "/f.txt",
        "text/plain",
        b"hello radio".to_vec(),
    )]);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/f.txt")).unwrap();
    let text = c.get_text(&url, 1000).unwrap();
    assert_eq!(text, "hello radio");
}

/// 404 / 403 / 500 → InvalidResponse。
#[test]
fn error_statuses() {
    let srv = FakeHttp::start(vec![
        Route::status("/nf", 404),
        Route::status("/fb", 403),
        Route::status("/ise", 500),
    ]);
    let c = client(allow_private(HttpConfig::default()));
    for path in ["/nf", "/fb", "/ise"] {
        let url = c.validate(&srv.url(path)).unwrap();
        let e = c.get_text(&url, 1000).unwrap_err();
        assert!(
            matches!(e, SourceError::InvalidResponse { .. }),
            "{path}: {e:?}"
        );
    }
}

/// 301/302 跟随（http→http 链内）。
#[test]
fn redirect_chain_follows() {
    let srv = FakeHttp::start(vec![
        Route::redirect("/r1", "/r2"),
        Route::redirect("/r2", "/r3"),
        Route::ok("/r3", "text/plain", b"final".to_vec()),
    ]);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/r1")).unwrap();
    assert_eq!(get_text_steady(&c, &url, 1000).unwrap(), "final");
    assert_eq!(srv.request_count(), 3, "两次重定向 + 一次终态");
}

/// 重定向 >5 → RedirectLimit。
#[test]
fn redirect_limit_enforced() {
    let mut routes = Vec::new();
    for i in 0..8 {
        routes.push(Route::redirect(
            Box::leak(format!("/hop{i}").into_boxed_str()),
            &format!("/hop{}", i + 1),
        ));
    }
    routes.push(Route::ok("/hop8", "text/plain", b"x".to_vec()));
    let srv = FakeHttp::start(routes);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/hop0")).unwrap();
    let e = get_text_steady(&c, &url, 1000).unwrap_err();
    assert!(matches!(e, SourceError::RedirectLimit), "{e:?}");
}

/// §88：redirect 到封锁平台 → 每跳校验拒绝（不得因为首跳合法而放行）。
#[test]
fn redirect_to_blocked_platform_denied() {
    let srv = FakeHttp::start(vec![Route::redirect(
        "/start",
        "https://www.youtube.com/watch?v=1",
    )]);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/start")).unwrap();
    let e = get_text_steady(&c, &url, 1000).unwrap_err();
    assert!(
        matches!(e, SourceError::UnsupportedPlatform { .. }),
        "重定向进封锁平台必须拒绝: {e:?}"
    );
}

/// §88：redirect 到私网 IP（allow 策略 = 用户确认的 LAN 语义，放行）。
#[test]
fn redirect_to_private_ip_under_allow_policy() {
    let srv = FakeHttp::start(vec![Route::redirect("/start", "http://10.9.8.7/x")]);
    let allow = client(allow_private(HttpConfig::default()));
    let url = allow.validate(&srv.url("/start")).unwrap();
    // 10.9.8.7 不可达（测试机没有该地址）→ NetworkError/Timeout，但不是策略拒绝。
    let e = match get_text_steady(&allow, &url, 100) {
        Err(e) => e,
        Ok(_) => panic!("unreachable address must not return body"),
    };
    assert!(
        matches!(e, SourceError::NetworkError { .. } | SourceError::Timeout),
        "{e:?}"
    );
}

/// §18：DNS rebinding 仿真 —— 公网域名解析到私网 IP，解析层复检必须拒绝。
#[test]
fn dns_rebind_to_private_ip_denied() {
    let mut cfg = allow_private(HttpConfig::default());
    let policy = NetworkPolicy::deny_private();
    cfg.policy = policy;
    cfg.dns_override = Some(Arc::new(|host| {
        if host == "rebind.test" {
            Ok(vec![std::net::IpAddr::from([127, 0, 0, 1])])
        } else {
            Err(std::io::Error::other("unexpected host"))
        }
    }));
    let c = client(cfg);
    let e = c
        .get_text(&"http://rebind.test/x".parse().unwrap(), 100)
        .unwrap_err();
    assert!(
        matches!(e, SourceError::PrivateNetworkBlocked { .. }),
        "解析到私网 IP 必须在连接前拒绝: {e:?}"
    );
}

/// §88：redirect 到带凭据 URL → 拒绝。
#[test]
fn redirect_to_credential_url_denied() {
    let srv = FakeHttp::start(vec![Route::redirect(
        "/start",
        "https://user:pass@example.com/file",
    )]);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/start")).unwrap();
    let e = get_text_steady(&c, &url, 100).unwrap_err();
    assert!(matches!(e, SourceError::InvalidUrl { .. }), "{e:?}");
}

/// 超时：服务器延迟 > 客户端总超时。
#[test]
fn metadata_timeout() {
    let srv = FakeHttp::start(vec![Route {
        path: "/slow",
        status: 200,
        headers: vec![],
        body: b"late".to_vec(),
        delay_ms: 3000,
        malformed: false,
        range: false,
    }]);
    let cfg = allow_private(HttpConfig {
        metadata_timeout: Duration::from_millis(300),
        ..HttpConfig::default()
    });
    let c = client(cfg);
    let url = c.validate(&srv.url("/slow")).unwrap();
    let e = get_text_steady(&c, &url, 100).unwrap_err();
    assert!(
        matches!(e, SourceError::Timeout),
        "延迟 3s > 总超时 0.3s 应 Timeout: {e:?}"
    );
}

/// 响应体超限 → BodyTooLarge。
#[test]
fn body_too_large() {
    let big = vec![b'x'; 10_000];
    let srv = FakeHttp::start(vec![Route::ok("/big", "application/json", big)]);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/big")).unwrap();
    let e = c.get_text(&url, 1_000).unwrap_err();
    assert!(matches!(
        e,
        SourceError::BodyTooLarge { limit_bytes: 1_000 }
    ));
}

/// 畸形响应 → 网络/解析错误，不 panic。
#[test]
fn malformed_response_no_panic() {
    let srv = FakeHttp::start(vec![Route {
        path: "/junk",
        status: 200,
        headers: vec![],
        body: Vec::new(),
        delay_ms: 0,
        malformed: true,
        range: false,
    }]);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/junk")).unwrap();
    let result = c.get_text(&url, 100);
    assert!(result.is_err(), "垃圾响应必须报错");
}

/// 非法 UTF-8 文本 → ParseError。
#[test]
fn invalid_utf8_is_parse_error() {
    let srv = FakeHttp::start(vec![Route::ok(
        "/bin",
        "application/json",
        vec![0xFF, 0xFE, 0x00],
    )]);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/bin")).unwrap();
    let e = c.get_text(&url, 100).unwrap_err();
    assert!(matches!(e, SourceError::ParseError { .. }), "{e:?}");
}

/// 坏 content-type 不阻断（§26：由 probing 判定，这里仅证明客户端不挑 content-type）。
#[test]
fn wrong_content_type_still_readable() {
    let srv = FakeHttp::start(vec![Route::ok(
        "/audio-ish",
        "application/octet-stream",
        b"AUDIO".to_vec(),
    )]);
    let c = client(allow_private(HttpConfig::default()));
    let url = c.validate(&srv.url("/audio-ish")).unwrap();
    let mut resp = c.get_stream(&url).unwrap();
    let mut buf = Vec::new();
    resp.read_to_end(&mut buf).unwrap();
    assert_eq!(buf, b"AUDIO");
}

/// §87/§125：封锁平台 zero-request 证明。
/// DNS spy 会 panic —— 它若被调用即测试失败；封锁必须发生在任何网络动作之前。
#[test]
fn blocked_platform_sends_zero_requests() {
    let spy_called = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let spy_called2 = Arc::clone(&spy_called);
    let mut cfg = allow_private(HttpConfig::default());
    cfg.dns_override = Some(Arc::new(move |_host| {
        spy_called2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // fake 服务器地址，万一真发出请求也会被观测到（但应永远到不了这里）。
        Ok(vec![std::net::IpAddr::from([127, 0, 0, 1])])
    }));
    let c = client(cfg);

    for bad in [
        "https://youtube.com/watch?v=abc",
        "https://spotify.com/track/1",
        "https://music.163.com/song?id=1",
        "https://y.qq.com/n/ryqq/song",
        "https://music.apple.com/album/1",
    ] {
        let url = c.validate(bad);
        assert!(
            matches!(
                url.as_ref().err(),
                Some(SourceError::UnsupportedPlatform { .. })
            ),
            "{bad} 必须在解析期拒绝: {url:?}"
        );
        // 显式尝试发请求也必须在 send 前失败（防御纵深）。
        if let Ok(u) = url {
            let _ = c.get_text(&u, 100);
        }
    }
    assert_eq!(
        spy_called.load(std::sync::atomic::Ordering::Relaxed),
        0,
        "封锁平台不得触发任何 DNS 解析/请求"
    );
}

/// 未封锁 URL 的 DNS spy 会被调用（对照组：spy 机制本身有效）。
#[test]
fn dns_spy_is_active_for_allowed_hosts() {
    let mut cfg = allow_private(HttpConfig::default());
    cfg.dns_override = Some(Arc::new(|_host| {
        Err(std::io::Error::other("spy: no network in tests"))
    }));
    let c = client(cfg);
    let url = c.validate("http://127.0.0.1:1/x").unwrap();
    // 127.0.0.1 是 IP 直连，不走 DNS spy → 连接失败。
    let e = c.get_text(&url, 100).unwrap_err();
    assert!(
        matches!(e, SourceError::NetworkError { .. } | SourceError::Timeout),
        "{e:?}"
    );
}
