//! M4 性能基线（§99）：local resolve / direct HTTP resolve / ICY 剥离
//! 吞吐 / Radio Browser 解析 / live 缓冲内存。`#[ignore]`，显式运行：
//! `cargo test -p aura-sources --release --test perf_baseline -- --ignored --nocapture`

mod common;

use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

use aura_sources::http::{HttpClient, HttpConfig};
use aura_sources::icy::IcyReader;
use aura_sources::platform_block::PlatformBlocklist;
use aura_sources::radio_browser::{parse_servers, parse_stations};
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
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap()
}

/// §99：local resolve + URL validation 吞吐。
#[test]
#[ignore = "perf baseline；显式运行"]
fn local_and_url_resolve() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aura-library/tests/fixtures/sine_8k.flac");
    let c = client();
    let bl = PlatformBlocklist::embedded().unwrap();

    let t = Instant::now();
    for _ in 0..1000 {
        let _ = resolve_input(&path.to_string_lossy(), &c, &bl).unwrap();
    }
    let local = t.elapsed();

    let t = Instant::now();
    for _ in 0..10_000 {
        let _ = aura_sources::url_policy::validate_url(
            "https://stream.example.com:8000/live.mp3",
            &NetworkPolicy::deny_private(),
            &bl,
        );
    }
    let urlv = t.elapsed();

    println!("== M4 resolve ==");
    println!(
        "local resolve 1000x   : {local:?} ({:.1} us/次)",
        local.as_secs_f64() * 1e6 / 1000.0
    );
    println!(
        "url validate 10000x   : {urlv:?} ({:.2} us/次)",
        urlv.as_secs_f64() * 1e6 / 10_000.0
    );
}

/// §99：direct HTTP resolve（头探测）延迟（fake server，本机回环）。
#[test]
#[ignore = "perf baseline；显式运行"]
fn direct_http_resolve_latency() {
    let body = vec![0x55u8; 64 * 1024];
    let srv = FakeHttp::start(vec![Route::ok_seekable("/p.mp3", "audio/mpeg", body)]);
    let c = client();
    let bl = PlatformBlocklist::embedded().unwrap();
    let t = Instant::now();
    for _ in 0..100 {
        let _ = resolve_input(&srv.url("/p.mp3"), &c, &bl).unwrap();
    }
    let d = t.elapsed();
    println!("== M4 direct HTTP resolve ==");
    println!(
        "resolve 100x (loopback): {d:?} ({:.2} ms/次)",
        d.as_secs_f64() * 1e3 / 100.0
    );
}

/// §99：ICY 剥离吞吐（纯 parser + 流式 reader）。
#[test]
#[ignore = "perf baseline；显式运行"]
fn icy_strip_throughput() {
    let metaint = 8192;
    let blocks = 128;
    let title = b"StreamTitle='Perf - Test';";
    let padded = title.len().div_ceil(16) * 16;
    let mut wire = Vec::new();
    for _ in 0..blocks {
        wire.extend(std::iter::repeat_n(0x33u8, metaint));
        wire.push(u8::try_from(padded / 16).unwrap());
        wire.extend_from_slice(title);
        wire.extend(std::iter::repeat_n(b' ', padded - title.len()));
    }
    let total = wire.len();
    let t = Instant::now();
    let mut r = IcyReader::new(wire.as_slice(), metaint);
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = r.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        out.extend_from_slice(&buf[..n]);
    }
    let d = t.elapsed();
    println!("== M4 ICY strip ==");
    println!(
        "strip {total} bytes   : {d:?} ({:.0} MB/s, audio {} bytes)",
        total as f64 / d.as_secs_f64() / 1e6,
        out.len()
    );
}

/// §99：Radio Browser JSON 解析吞吐。
#[test]
#[ignore = "perf baseline；显式运行"]
fn radio_browser_parse_throughput() {
    let one = r#"{"stationuuid":"u1","name":"Station","url":"http://a/x","url_resolved":"http://b/x","codec":"MP3","bitrate":128,"country":"X","language":"y","homepage":"http://h","tags":"a,b"}"#;
    let json = format!("[{}]", vec![one; 2000].join(","));
    let t = Instant::now();
    let (stations, _) = parse_stations(&json).unwrap();
    let stations_t = t.elapsed();
    let servers_json = format!(
        "[{}]",
        vec![r#"{"name":"m","hostname":"h.test"}"#; 500].join(",")
    );
    let t = Instant::now();
    let servers = parse_servers(&servers_json).unwrap();
    let servers_t = t.elapsed();
    println!("== M4 Radio Browser parse ==");
    println!(
        "stations 2000 条      : {stations_t:?} ({:.1} us/条, {} 条)",
        stations_t.as_secs_f64() * 1e6 / 2000.0,
        stations.len()
    );
    println!(
        "servers 500 条        : {servers_t:?} ({:.1} us/条, {} 条)",
        servers_t.as_secs_f64() * 1e6 / 500.0,
        servers.len()
    );
}

/// §99/§100：live 缓冲内存可预测性（容量 = 构造参数，运行期不增长）。
#[test]
#[ignore = "perf baseline；显式运行"]
fn live_buffer_memory_bound() {
    use aura_sources::model::StreamType;
    use aura_sources::network_stream::buffer_capacity;
    let cases = [
        (StreamType::Live, Some(128_000u64)),
        (StreamType::Live, Some(320_000)),
        (StreamType::Live, None),
        (StreamType::Finite, None),
    ];
    println!("== M4 live buffer memory ==");
    for (t, br) in cases {
        println!(
            "  {t:?} bitrate={br:?} -> capacity {} bytes",
            buffer_capacity(t, br)
        );
    }
}
