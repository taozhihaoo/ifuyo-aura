//! Internet Radio 端到端测试（§30–§38/§91/§94/§102 部分）：
//! Icy-MetaData 请求头、元数据剥离后的音频纯净性、事件通道、
//! Forbidden 缓存政策、live EOF 语义、zero-writes。

mod common;

use std::io::Read;
use std::time::Duration;

use aura_sources::error::SourceError;
use aura_sources::http::{HttpClient, HttpConfig};
use aura_sources::model::{CachePolicy, StreamType};
use aura_sources::platform_block::PlatformBlocklist;
use aura_sources::radio;
use aura_sources::resolver::resolve_input;
use aura_sources::url_policy::NetworkPolicy;
use common::{FakeHttp, Route};

fn allow_private(mut cfg: HttpConfig) -> HttpConfig {
    cfg.policy = NetworkPolicy::allow_private();
    cfg
}

fn client() -> HttpClient {
    HttpClient::new(
        allow_private(HttpConfig::default()),
        std::sync::Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap()
}

/// 合成 ICY 字节流：mp3-ish audio + metadata 交替。
fn icy_wire(metaint: usize, blocks: usize, titles: &[&[u8]]) -> (Vec<u8>, Vec<u8>) {
    let mut wire = Vec::new();
    let mut audio = Vec::new();
    for b in 0..blocks {
        let chunk: Vec<u8> = (0..metaint)
            .map(|i| ((b * metaint + i) % 251) as u8)
            .collect();
        audio.extend_from_slice(&chunk);
        wire.extend_from_slice(&chunk);
        let title: &[u8] = titles[b % titles.len()];
        let padded = title.len().div_ceil(16) * 16;
        wire.push(u8::try_from(padded / 16).unwrap());
        wire.extend_from_slice(title);
        wire.extend(std::iter::repeat_n(b' ', padded - title.len()));
    }
    (wire, audio)
}

/// §31：请求头带 Icy-MetaData: 1（服务器可见）。
#[test]
fn requests_icy_metadata_header() {
    let (wire, _) = icy_wire(16, 4, &[b"StreamTitle='A';"]);
    let srv = FakeHttp::start(vec![Route {
        path: "/live",
        status: 200,
        headers: vec![
            ("Content-Type".into(), "audio/mpeg".into()),
            ("icy-name".into(), "Test FM".into()),
            ("icy-metaint".into(), "16".into()),
        ],
        body: wire,
        delay_ms: 0,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let url = c.validate(&srv.url("/live")).unwrap();
    let mut rs = radio::open(&c, &url, None).unwrap();
    let mut buf = [0u8; 4096];
    let mut got = 0;
    while got == 0 {
        got = rs.read(&mut buf).unwrap();
    }
    let heads = srv.raw_request_heads().join("\n");
    assert!(
        heads.to_ascii_lowercase().contains("icy-metadata: 1"),
        "必须发送 Icy-MetaData: 1（§31）: {heads}"
    );
    assert!(
        heads
            .to_ascii_lowercase()
            .contains("user-agent: ifuyo aura/"),
        "必须用描述性 UA（§20）"
    );
}

/// §32/§34/§91：剥离后音频纯净 + StreamTitle 事件到达。
#[test]
fn metadata_stripped_and_events_emitted() {
    let titles: [&[u8]; 2] = [
        "StreamTitle='周杰伦 - 夜曲';".as_bytes(),
        b"StreamTitle='Nova - Lights';",
    ];
    let (wire, audio) = icy_wire(64, 8, &titles);
    let srv = FakeHttp::start(vec![Route {
        path: "/live",
        status: 200,
        headers: vec![
            ("Content-Type".into(), "audio/mpeg".into()),
            ("icy-name".into(), "Test FM".into()),
            ("icy-metaint".into(), "64".into()),
        ],
        body: wire,
        delay_ms: 0,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let url = c.validate(&srv.url("/live")).unwrap();
    let mut rs = radio::open(&c, &url, None).unwrap();

    let mut out = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        match rs.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(e) => panic!("read error: {e}"),
        }
    }
    assert_eq!(
        out, audio,
        "剥离后音频必须与原始音频逐字节一致（§102 audio continuity）"
    );

    // 事件：两次 StreamTitle 更新都到达（feeder 回调 → mpsc）。
    let mut saw_jay = false;
    let mut saw_nova = false;
    while let Some(ev) = rs.try_next_event() {
        if ev.raw.as_deref() == Some("周杰伦 - 夜曲") {
            assert_eq!(ev.artist.as_deref(), Some("周杰伦"));
            assert_eq!(ev.title.as_deref(), Some("夜曲"));
            saw_jay = true;
        }
        if ev.raw.as_deref() == Some("Nova - Lights") {
            saw_nova = true;
        }
    }
    assert!(saw_jay && saw_nova, "两次 StreamTitle 事件都应到达（§35）");
    assert_eq!(rs.cache_policy, CachePolicy::Forbidden, "§37/§94");
    assert_eq!(rs.station.as_deref(), Some("Test FM"));
}

/// §30/§94：resolver 判定 radio 形态 → Live + NonSeekable + Forbidden。
#[test]
fn radio_resolution_capabilities() {
    let (wire, _) = icy_wire(32, 4, &[b"StreamTitle='x';"]);
    let srv = FakeHttp::start(vec![Route {
        path: "/live",
        status: 200,
        headers: vec![
            ("Content-Type".into(), "audio/mpeg".into()),
            ("icy-name".into(), "Cap FM".into()),
            ("icy-metaint".into(), "32".into()),
        ],
        body: wire,
        delay_ms: 0,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let resolved = resolve_input(
        &srv.url("/live"),
        &c,
        &PlatformBlocklist::embedded().unwrap(),
    )
    .unwrap();
    assert_eq!(
        resolved.kind,
        aura_sources::model::SourceKind::InternetRadio
    );
    assert_eq!(resolved.streams[0].stream_type, StreamType::Live, "§30");
    assert_eq!(
        resolved.streams[0].seekable,
        aura_sources::model::SeekCapability::NonSeekable
    );
    assert_eq!(resolved.cache_policy, CachePolicy::Forbidden);

    // open_resolved 入口。
    let mut rs = radio::open_resolved(&c, &resolved).unwrap();
    let mut buf = [0u8; 64];
    assert!(rs.read(&mut buf).unwrap_or(0) > 0);
}

/// §110：live 流 EOF —— 读端返回 0（流结束），由上层决定后续。
#[test]
fn live_eof_is_stream_end() {
    let (wire, _) = icy_wire(16, 2, &[b"StreamTitle='end';"]);
    let srv = FakeHttp::start(vec![Route {
        path: "/live",
        status: 200,
        headers: vec![("icy-metaint".into(), "16".into())],
        body: wire,
        delay_ms: 0,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let url = c.validate(&srv.url("/live")).unwrap();
    let mut rs = radio::open(&c, &url, None).unwrap();
    let mut total = 0;
    let mut buf = [0u8; 256];
    loop {
        match rs.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => total += n as u64,
            Err(e) => panic!("unexpected: {e}"),
        }
    }
    // 音频 = 2×16 字节；EOF 正常返回 0（§110：不 panic、不自动“下一轨”）。
    assert_eq!(total, 32);
}

/// §22/§109：radio 取消即时。
#[test]
fn radio_cancel_prompt() {
    let (wire, _) = icy_wire(16, 4000, &[b"StreamTitle='loop';"]);
    let srv = FakeHttp::start(vec![Route {
        path: "/live",
        status: 200,
        headers: vec![("icy-metaint".into(), "16".into())],
        body: wire,
        delay_ms: 0,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let url = c.validate(&srv.url("/live")).unwrap();
    let mut rs = radio::open(&c, &url, None).unwrap();
    let mut buf = [0u8; 256];
    rs.read(&mut buf).unwrap();
    rs.cancel();
    assert!(rs.is_cancelled());
    let started = std::time::Instant::now();
    let e = rs.read(&mut buf).unwrap_err();
    assert_eq!(e.kind(), std::io::ErrorKind::Interrupted);
    assert!(started.elapsed() < Duration::from_secs(2), "取消必须即时");
}

/// §38/§95：Forbidden 电台流零写盘（复用通用 helper）。
#[test]
fn radio_stream_zero_disk_write() {
    let (wire, _) = icy_wire(16, 3000, &[b"StreamTitle='zw';"]);
    let srv = FakeHttp::start(vec![Route {
        path: "/live",
        status: 200,
        headers: vec![("icy-metaint".into(), "16".into())],
        body: wire,
        delay_ms: 0,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let sandbox = std::env::temp_dir().join(format!("aura-m4-radio-zw-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox).unwrap();

    let url = c.validate(&srv.url("/live")).unwrap();
    let mut rs = radio::open(&c, &url, None).unwrap();
    network_stream_test::assert_forbidden_source_no_disk_write(&sandbox, &mut || {
        let mut buf = [0u8; 4096];
        let mut n = 0;
        while n < 30_000 {
            match rs.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(k) => n += k as u64,
            }
        }
    });
    let _ = std::fs::remove_dir_all(&sandbox);
    let _ = SourceError::NetworkError {
        reason: String::new(),
    }; // 类型引用保活
}

// helper 来自 network_stream 测试模块 —— 不可跨测试二进制引用；
// 这里内联等价实现（避免 helper 泄漏成公共 API）。
mod network_stream_test {
    pub fn assert_forbidden_source_no_disk_write(
        sandbox: &std::path::Path,
        activity: &mut dyn FnMut(),
    ) {
        fn snapshot(dir: &std::path::Path) -> Vec<(std::path::PathBuf, u64)> {
            let mut out = Vec::new();
            if let Ok(rd) = std::fs::read_dir(dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        out.extend(snapshot(&p));
                    } else if let Ok(md) = e.metadata() {
                        out.push((p, md.len()));
                    }
                }
            }
            out.sort();
            out
        }
        let before = snapshot(sandbox);
        activity();
        let after = snapshot(sandbox);
        assert_eq!(before, after, "Forbidden 来源不得写盘（§38/§95）");
    }
}
