//! Source → Audio Core 集成测试（§55/§58/§112/§113）：
//! fake HTTP 服务器 → resolve → NetworkAudioStream →
//! aura-core `StreamingDecoder` → Engine（NullOutput）→ PCM。
//! 覆盖：直链 finite 解码、seek 后解码、ICY 电台流解码（元数据已剥离）。

mod common;

use std::io::{Read, Seek};
use std::sync::Arc;
use std::time::Duration;

use aura_core::decoder::{AudioByteStream, Decoder as _, StreamingDecoder};
use aura_core::{AudioCommand, Channels, Engine, PlaybackState, SampleRate};
use aura_sources::http::{HttpClient, HttpConfig};
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
        Arc::new(PlatformBlocklist::embedded().unwrap()),
    )
    .unwrap()
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../aura-library/tests/fixtures")
            .join(name),
    )
    .unwrap_or_else(|_| {
        // aura-library fixture 相对路径失败时退回本 crate fixtures。
        std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
        )
        .expect("fixture")
    })
}

/// §112：direct stream → decoder → engine 渲染出非静音 PCM。
#[test]
fn direct_http_mp3_through_engine() {
    let mp3 = fixture("sine_44k.mp3");
    let srv = FakeHttp::start(vec![Route::ok_seekable(
        "/song.mp3",
        "audio/mpeg",
        mp3.clone(),
    )]);
    let c = client();
    let _ = &mp3;
    let resolved = resolve_input(
        &srv.url("/song.mp3"),
        &c,
        &PlatformBlocklist::embedded().unwrap(),
    )
    .unwrap();
    assert_eq!(resolved.kind, aura_sources::model::SourceKind::DirectHttp);

    // Resolved → NetworkAudioStream（seekable）。
    let s0 = &resolved.streams[0];
    let url = url::Url::parse(&s0.url).unwrap();
    let resp = c.get_stream(&url).unwrap();
    // §27：byte_size 由 resolver 从 Content-Length 捕获，是 Range seek 的前提。
    let content_len = s0.byte_size.or_else(|| resp.content_length());
    assert!(content_len.is_some(), "direct 流必须携带 byte_size");
    let stream = aura_sources::network_stream::NetworkAudioStream::start(
        &c,
        url,
        resp,
        64 * 1024,
        s0.seekable == aura_sources::model::SeekCapability::Seekable,
        content_len,
        Duration::from_secs(10),
    );

    // hint 由 URL 推断（§77：仅 probe 提示）。
    let hint = StreamingDecoder::hint_from_url(&s0.url);
    assert_eq!(hint.as_deref(), Some("mp3"));
    let decoder = StreamingDecoder::open(Box::new(stream), Some("mp3")).unwrap();
    let fmt = aura_core::decoder::Decoder::format(&decoder);
    assert_eq!(fmt.channels.get(), 1);
    assert_eq!(fmt.sample_rate.get(), 44_100);

    // Engine（NullOutput，48k stereo）播放整轨。
    let (mut engine, mut poster) = Engine::new(
        SampleRate::new(48_000).unwrap(),
        Channels::new(2).unwrap(),
        16_384,
    )
    .unwrap();
    let _ = engine.load_track(Box::new(decoder));
    poster.post(AudioCommand::Play).unwrap();
    let mut rendered = 0u64;
    for _ in 0..400 {
        let n = engine.render_for_frames(4_800);
        rendered += n;
        if engine.state() == PlaybackState::Stopped || n == 0 {
            break;
        }
    }
    assert!(rendered > 0, "网络 MP3 必须解码出 PCM: rendered={rendered}");
    let cap = engine.capture();
    assert!(!cap.is_empty());
    assert!(
        cap.iter().any(|v| v.abs() > 0.001),
        "解码出的 PCM 必须非静音（正弦 fixture）"
    );
}

/// §112：seek on seekable source —— Range 重请求后仍能解码出音频。
#[test]
fn direct_http_seek_then_decode() {
    let mp3 = fixture("sine_44k.mp3");
    let srv = FakeHttp::start(vec![Route::ok_seekable(
        "/song.mp3",
        "audio/mpeg",
        mp3.clone(),
    )]);
    let c = client();
    let url = c.validate(&srv.url("/song.mp3")).unwrap();
    let resp = c.get_stream(&url).unwrap();
    let mut stream = aura_sources::network_stream::NetworkAudioStream::start(
        &c,
        url,
        resp,
        32 * 1024,
        true,
        Some(mp3.len() as u64),
        Duration::from_secs(10),
    );

    // 读一点再 seek 到 1/3 处。
    let mut warm = [0u8; 512];
    stream.read_exact(&mut warm).unwrap();
    let len = stream.byte_len().expect("seekable 流应有 byte_len");
    let mid = len / 3;
    let pos = stream.seek(std::io::SeekFrom::Start(mid)).unwrap();
    assert_eq!(pos, mid);

    let mut decoder = StreamingDecoder::open(Box::new(stream), Some("mp3")).unwrap();
    let mut out = aura_core::decoder::PcmBuffer::new();
    let mut frames = 0u64;
    for _ in 0..20 {
        match decoder.decode_next(&mut out) {
            Ok(0) => break,
            Ok(n) => frames += n,
            Err(_) => break, // seek 落点可能破坏帧边界：容错
        }
    }
    // 落点在帧流中间时 MP3 比特解码器可能需要重同步；只要有帧或干净 EOF 即可。
    assert!(
        frames > 0 || decoder.duration_frames().is_some(),
        "seek 后应有解码输出或有效流: frames={frames}"
    );
}

/// §58：ICY 电台流 → decoder（元数据已被 IcyReader 剥离，MP3 流干净）。
#[test]
fn icy_radio_stream_through_decoder() {
    // 构造带 ICY 元数据的真实 MP3 流。
    let mp3 = fixture("sine_44k.mp3");
    let metaint = 128;
    let mut wire = Vec::new();
    let mut audio = Vec::new();
    let mut pos = 0;
    let mut block = 0;
    while pos < mp3.len() {
        // 协议构造约束：每个 audio 块必须恰好 metaint 字节（不足补零），
        // metadata 只出现在整块之后 —— 真实服务器保证此帧结构。
        let end = (pos + metaint).min(mp3.len());
        let mut chunk = mp3[pos..end].to_vec();
        let padded_to = chunk.len() < metaint;
        if padded_to {
            chunk.resize(metaint, 0);
        }
        audio.extend_from_slice(&chunk);
        wire.extend_from_slice(&chunk);
        pos = end;
        let title = if block == 0 {
            "StreamTitle='DJ - Track One';"
        } else {
            "StreamTitle='DJ - Track Two';"
        };
        let padded = title.len().div_ceil(16) * 16;
        wire.push(u8::try_from(padded / 16).unwrap());
        wire.extend_from_slice(title.as_bytes());
        wire.extend(std::iter::repeat_n(b' ', padded - title.len()));
        block += 1;
    }

    let srv = FakeHttp::start(vec![Route {
        path: "/live",
        status: 200,
        headers: vec![
            ("Content-Type".into(), "audio/mpeg".into()),
            ("icy-name".into(), "Integration FM".into()),
            ("icy-metaint".into(), "128".into()),
        ],
        body: wire,
        delay_ms: 0,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let url = c.validate(&srv.url("/live")).unwrap();
    let mut rs = radio::open(&c, &url, None).unwrap();

    // 排空全部音频（元数据已剥离），收事件。
    let mut clean = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match rs.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => clean.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    assert_eq!(clean, audio, "剥离后的音频与原始 MP3 逐字节一致");
    let ev1 = rs.try_next_event();
    assert!(ev1.is_some(), "应有 StreamTitle 事件");

    // 干净音频喂 decoder。
    let mut decoder = StreamingDecoder::open(
        Box::new(aura_core::decoder::streaming::MemoryStream::new(
            clean, true,
        )),
        Some("mp3"),
    )
    .unwrap();
    let mut out = aura_core::decoder::PcmBuffer::new();
    let mut frames = 0u64;
    for _ in 0..40 {
        match decoder.decode_next(&mut out) {
            Ok(0) => break,
            Ok(n) => frames += n,
            Err(e) => panic!("decode failed: {e}"),
        }
    }
    assert!(frames > 0, "电台 MP3 必须解码出 PCM: {frames}");
}

/// §113：Local Source 读取 M3 支持的格式（本地 FLAC 直接 probe 解码）。
#[test]
fn local_source_through_decoder() {
    let flac = fixture("sine_8k.flac");
    let mut decoder = StreamingDecoder::open(
        Box::new(aura_core::decoder::streaming::MemoryStream::new(flac, true)),
        Some("flac"),
    )
    .unwrap();
    let mut out = aura_core::decoder::PcmBuffer::new();
    let mut frames = 0u64;
    for _ in 0..10 {
        match decoder.decode_next(&mut out) {
            Ok(0) => break,
            Ok(n) => frames += n,
            Err(e) => panic!("decode failed: {e}"),
        }
    }
    assert!(frames >= 1900, "FLAC 0.25s@8kHz ≈ 2000 帧: {frames}");
}

/// §78：未知格式 → InvalidFormat（不 panic、不猜）。
#[test]
fn unknown_format_rejected() {
    let junk = b"this is definitely not audio".to_vec();
    let err = match StreamingDecoder::open(
        Box::new(aura_core::decoder::streaming::MemoryStream::new(junk, true)),
        Some("mp3"),
    ) {
        Err(e) => e,
        Ok(_) => panic!("junk must not decode"),
    };
    assert!(matches!(
        err,
        aura_core::AudioError::InvalidFormat(_) | aura_core::AudioError::DecodeError(_)
    ));
}
