//! Radio Browser 测试（§103/§85/§89/§124）：discovery 解析、镜像固定注入、
//! fallback（失败镜像→下一个，上限可配）、搜索过滤、url_resolved 优先并
//! 记录、click 默认关（zero-request）、malformed/empty/large、解析容错。
//! 全部本地 fake server（§82/§85），生产路径保持 https。

mod common;

use std::sync::Arc;

use aura_sources::error::SourceError;
use aura_sources::http::{HttpClient, HttpConfig};
use aura_sources::platform_block::PlatformBlocklist;
use aura_sources::radio_browser::{
    codec_supported, fixed_selector, parse_servers, parse_stations, random_selector,
    RadioBrowserSource, StationQuery,
};
use aura_sources::url_policy::NetworkPolicy;
use common::{FakeHttp, Route};

const SERVERS_JSON: &str = r#"[
  {"name":"Mirror A","hostname":"mirror-a.test"},
  {"name":"Mirror B","hostname":"mirror-b.test"},
  {"name":"Mirror C","hostname":"mirror-c.test"}
]"#;

const STATIONS_JSON: &str = r#"[
  {
    "stationuuid":"uuid-1","name":"Jazz FM","url":"http://old.example/jazz",
    "url_resolved":"http://real.example/jazz.mp3","codec":"MP3","bitrate":"128",
    "country":"France","language":"french","homepage":"https://jazz.example","tags":"jazz,smooth",
    "unknown_future_field":123
  },
  {
    "stationuuid":"uuid-2","name":"Rock One","url":"http://old.example/rock",
    "url_resolved":"http://real.example/rock.ogg","codec":"OGG","bitrate":96,
    "country":"Germany","language":"german","homepage":"","tags":"rock"
  },
  {"stationuuid":"uuid-3","name":"Broken","bitrate":"garbage"}
]"#;

fn allow_private(mut cfg: HttpConfig) -> HttpConfig {
    cfg.policy = NetworkPolicy::allow_private();
    cfg
}

fn http() -> HttpClient {
    HttpClient::new(
        allow_private(HttpConfig::default()),
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap()
}

/// 组装：discovery server + 两个 API server（A=坏，B/C=好）。
/// 镜像 host 列表运行时改写为 [api_a, api_b, api_b]；scheme=http 注入。
struct Rig {
    _discovery: FakeHttp,
    api_a: FakeHttp,
    api_b: FakeHttp,
    src: RadioBrowserSource,
}

fn rig(api_a_routes: Vec<Route>, api_b_routes: Vec<Route>, max_mirrors: usize) -> Rig {
    let api_a = FakeHttp::start(api_a_routes);
    let api_b = FakeHttp::start(api_b_routes);
    let a = format!("{}:{}", api_a.addr.ip(), api_a.addr.port());
    let b = format!("{}:{}", api_b.addr.ip(), api_b.addr.port());
    let servers_json = format!(
        r#"[{{"name":"A","hostname":"{a}"}},{{"name":"B","hostname":"{b}"}},{{"name":"C","hostname":"{b}"}}]"#
    );
    let discovery = FakeHttp::start(vec![Route::ok(
        "/json/servers",
        "application/json",
        servers_json.into_bytes(),
    )]);
    let src = RadioBrowserSource::with_parts(
        http(),
        discovery.url(""),
        "http".to_string(),
        max_mirrors,
        fixed_selector(0),
    );
    Rig {
        _discovery: discovery,
        api_a,
        api_b,
        src,
    }
}

fn search_route(body: &'static [u8]) -> Vec<Route> {
    vec![Route::ok(
        "/json/stations/search",
        "application/json",
        body.to_vec(),
    )]
}

/// 纯解析：servers + stations 容错（§80）。
#[test]
fn parse_fixtures() {
    let servers = parse_servers(SERVERS_JSON).unwrap();
    assert_eq!(servers.len(), 3);
    assert_eq!(servers[1].hostname, "mirror-b.test");

    let (stations, skipped) = parse_stations(STATIONS_JSON).unwrap();
    assert_eq!(stations.len(), 3);
    assert_eq!(skipped, 0);
    assert_eq!(stations[0].bitrate, 128, "字符串 bitrate 容错");
    assert_eq!(stations[1].bitrate, 96, "数字 bitrate");
    assert_eq!(stations[2].bitrate, 0, "垃圾 bitrate → 0（不猜）");
    assert_eq!(stations[0].codec, "MP3");
}

/// §80：malformed / 非数组 / 空。
#[test]
fn parse_malformed_and_empty() {
    assert!(parse_servers("{ not json").is_err());
    assert!(parse_servers("[]").unwrap().is_empty());
    assert!(parse_stations("not json").is_err());
    assert!(parse_stations(r#"{"a":1}"#).is_err());
    assert!(parse_stations("[]").unwrap().0.is_empty());
}

/// §81：属性 —— 任意字节不 panic。
#[test]
fn property_never_panics() {
    let mut rng = aura_testkit::rng::DeterministicRng::new(0xB105);
    for _ in 0..128 {
        let len = usize::try_from(rng.next_u64() % 256).unwrap_or(0);
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next_u64() & 0xFF) as u8).collect();
        let text = String::from_utf8_lossy(&bytes);
        let _ = parse_servers(&text);
        let _ = parse_stations(&text);
    }
}

/// §46：codec 兼容。
#[test]
fn codec_filtering() {
    assert!(codec_supported("MP3"));
    assert!(codec_supported("OGG"));
    assert!(codec_supported("FLAC"));
    assert!(!codec_supported("AAC"));
    assert!(!codec_supported("AAC+,H.264"));
}

/// §40/§42：discovery → 镜像 A 失败 → B 成功（fallback，有限次）。
#[test]
fn fallback_to_next_mirror() {
    let rig = rig(
        vec![Route::status("/json/stations/search", 500)],
        search_route(STATIONS_JSON.as_bytes()),
        3,
    );
    let q = StationQuery {
        name: Some("jazz".into()),
        limit: 10,
        ..Default::default()
    };
    let stations = rig.src.search(&q).unwrap();
    assert_eq!(stations.len(), 3);
    assert!(rig.api_a.request_count() >= 1, "先试 A");
    assert!(rig.api_b.request_count() >= 1, "fallback 到 B");
}

/// §42：max_mirrors=1 → 首镜像失败后不再尝试 → SourceUnavailable。
#[test]
fn fallback_respects_mirror_limit() {
    let rig = rig(
        vec![Route::status("/json/stations/search", 500)],
        search_route(STATIONS_JSON.as_bytes()),
        1,
    );
    let q = StationQuery {
        limit: 10,
        ..Default::default()
    };
    let err = rig.src.search(&q).unwrap_err();
    assert!(
        matches!(err, SourceError::SourceUnavailable { .. }),
        "{err:?}"
    );
    assert_eq!(rig.api_b.request_count(), 0, "上限 1 → 不试 B");
}

/// §45：url_resolved 优先，缺失回退 url（记录实际使用字段）。
#[test]
fn url_resolved_preferred() {
    let (stations, _) = parse_stations(STATIONS_JSON).unwrap();
    let (u, field) = stations[0].stream_url();
    assert_eq!(u, "http://real.example/jazz.mp3");
    assert_eq!(field, "url_resolved", "§45 实际使用字段可记录");
    let missing =
        r#"[{"stationuuid":"u","name":"x","url":"http://fallback/url","url_resolved":""}]"#;
    let (stations, _) = parse_stations(missing).unwrap();
    let (u, field) = stations[0].stream_url();
    assert_eq!(u, "http://fallback/url");
    assert_eq!(field, "url", "回退记录为 url");
}

/// §52/§53/§94：resolve_station 语义。
#[test]
fn resolve_station_semantics() {
    let rig = rig(vec![], search_route(STATIONS_JSON.as_bytes()), 1);
    let (stations, _) = parse_stations(STATIONS_JSON).unwrap();
    let r = rig.src.resolve_station(&stations[0]).unwrap();
    assert_eq!(r.kind, aura_sources::model::SourceKind::RadioBrowser);
    assert_eq!(
        r.streams[0].stream_type,
        aura_sources::model::StreamType::Live
    );
    assert_eq!(
        r.streams[0].seekable,
        aura_sources::model::SeekCapability::NonSeekable
    );
    assert_eq!(
        r.cache_policy,
        aura_sources::model::CachePolicy::Forbidden,
        "§94"
    );
    assert_eq!(
        r.license,
        aura_sources::model::ResolvedLicense::Unknown,
        "§51/§52"
    );
    assert!(r.attribution.required_text.is_none(), "§53 不编造署名");
    assert_eq!(r.attribution.provider.as_deref(), Some("Radio Browser"));
    assert_eq!(r.meta.provider.name.as_deref(), Some("Radio Browser"));
    assert_eq!(r.streams[0].bitrate_bps, Some(128_000));
}

/// §47：click 默认 OFF —— 零请求；显式开启后才发送。
#[test]
fn click_reporting_default_off() {
    let mut rig = rig(
        vec![],
        vec![Route::ok(
            "/json/url/uuid-1",
            "application/json",
            br#"{"ok":true}"#.to_vec(),
        )],
        2, // 首镜像 A 无路由 404 → fallback 到 B（上限内）
    );
    assert!(!rig.src.click_reporting_enabled, "§47 默认 OFF");
    rig.src.report_click("uuid-1").unwrap();
    assert_eq!(rig.api_b.request_count(), 0, "关闭 = 零请求");
    assert_eq!(
        rig._discovery.request_count(),
        0,
        "关闭 = 连 discovery 都不发"
    );

    // 开启后发送。
    rig.src.click_reporting_enabled = true;
    rig.src.report_click("uuid-1").unwrap();
    assert!(rig.api_b.request_count() >= 1, "开启后才发送");
}

/// §49/§50：discovery 不可达 → SourceUnavailable；大响应受限。
#[test]
fn discovery_failure_and_limits() {
    // 不可达的 discovery（不存在的本地端口）。
    let http_client = http();
    let src = RadioBrowserSource::with_parts(
        http_client,
        "http://127.0.0.1:9".to_string(), // discard 端口，必失败
        "http".to_string(),
        1,
        fixed_selector(0),
    );
    let err = src.discover_mirrors().unwrap_err();
    assert!(
        matches!(
            err,
            SourceError::SourceUnavailable { .. } | SourceError::Timeout
        ),
        "§49：离线 → SourceUnavailable（不得换服务）: {err:?}"
    );

    // 大响应体 → 受 64KiB 限制。
    let big = format!("[{}]", r#"{"name":"x","hostname":"h.test"},"#.repeat(4000));
    let servers = FakeHttp::start(vec![Route::ok(
        "/json/servers",
        "application/json",
        big.into_bytes(),
    )]);
    let src = RadioBrowserSource::with_parts(
        http(),
        servers.url(""),
        "http".to_string(),
        1,
        fixed_selector(0),
    );
    let err = src.discover_mirrors().unwrap_err();
    assert!(
        matches!(
            err,
            SourceError::BodyTooLarge { .. } | SourceError::SourceUnavailable { .. }
        ),
        "大响应必须被限制: {err:?}"
    );
}

/// §41/§124：选择器语义（固定注入 vs 生产随机）。
#[test]
fn selector_semantics() {
    let f = fixed_selector(1);
    assert_eq!(f(3), 1);
    assert_eq!(f(0), 0, "空列表不越界");
    let r = random_selector();
    for n in [1usize, 3, 7, 50] {
        assert!(r(n) < n, "随机选择必须落在 [0,n)");
    }
}

/// §43：UA 透出。
#[test]
fn user_agent_is_descriptive() {
    assert!(RadioBrowserSource::user_agent().starts_with("ifuyo Aura/"));
}
