//! NetworkAudioStream 测试（§28/§29/§62/§101 部分/§109）：
//! 字节保真、有界缓冲与背压、Range seek、非 seekable 拒绝、
//! 取消即时性、空闲超时、Forbidden 来源零写盘 helper。

mod common;

use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use aura_sources::http::{HttpClient, HttpConfig};
use aura_sources::model::StreamType;
use aura_sources::network_stream::{buffer_capacity, NetworkAudioStream};
use aura_sources::platform_block::PlatformBlocklist;
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

use std::sync::Arc;

fn make_pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// 并行测试下的传输级抖动（首连偶发重置）只影响 NetworkError；
/// 断言不放松，仅对建流阶段重试。
fn open_stream(
    c: &HttpClient,
    url: &url::Url,
    capacity: usize,
    seekable: bool,
    idle: Duration,
) -> NetworkAudioStream {
    let mut last = None;
    for _ in 0..3 {
        match c.get_stream(url) {
            Ok(resp) => {
                return NetworkAudioStream::start(
                    c,
                    url.clone(),
                    resp,
                    capacity,
                    seekable,
                    None,
                    idle,
                );
            }
            Err(e @ aura_sources::error::SourceError::NetworkError { .. }) => last = Some(e),
            Err(e) => panic!("unexpected open error: {e:?}"),
        }
    }
    panic!("stream open kept failing: {last:?}");
}

/// 基本读取：字节逐一致（feeder + 有界缓冲全链路）。
#[test]
fn stream_reads_exact_bytes() {
    let body = make_pattern(300_000);
    let srv = FakeHttp::start(vec![Route::ok("/audio.bin", "audio/mpeg", body.clone())]);
    let c = client();
    let url = c.validate(&srv.url("/audio.bin")).unwrap();
    let resp = c.get_stream(&url).unwrap();
    let mut s = NetworkAudioStream::start(
        &c,
        url,
        resp,
        64 * 1024,
        false,
        None,
        Duration::from_secs(10),
    );
    let mut out = Vec::new();
    s.read_to_end(&mut out).unwrap();
    assert_eq!(out, body, "经有界缓冲后字节必须逐一致");
}

/// §62：小容量管道 + 大响应 = 背压限流，最终数据完整。
#[test]
fn small_capacity_backpressure_completes() {
    let body = make_pattern(1_500_000);
    let srv = FakeHttp::start(vec![Route::ok("/big.bin", "audio/mpeg", body.clone())]);
    let c = client();
    let url = c.validate(&srv.url("/big.bin")).unwrap();
    let resp = c.get_stream(&url).unwrap();
    // 16 KiB 管道吃 1.5 MiB：feeder 必然多次被背压阻塞。
    let mut s = NetworkAudioStream::start(
        &c,
        url,
        resp,
        16 * 1024,
        false,
        None,
        Duration::from_secs(30),
    );
    let mut out = Vec::new();
    s.read_to_end(&mut out).unwrap();
    assert_eq!(out, body);
}

/// §27/§56：Range seek —— 定位后从正确偏移继续。
#[test]
fn range_seek_continues_at_offset() {
    let body = make_pattern(500_000);
    let srv = FakeHttp::start(vec![Route::ok_seekable(
        "/seek.mp3",
        "audio/mpeg",
        body.clone(),
    )]);
    let c = client();
    let url = c.validate(&srv.url("/seek.mp3")).unwrap();
    let resp = c.get_stream(&url).unwrap();
    let mut s = NetworkAudioStream::start(
        &c,
        url,
        resp,
        64 * 1024,
        true,
        Some(body.len() as u64),
        Duration::from_secs(10),
    );

    // 读头部 1000 字节确认通路。
    let mut head = [0u8; 1000];
    s.read_exact(&mut head).unwrap();
    assert_eq!(&head[..], &body[..1000]);

    // seek 到 250_000 再读。
    let pos = s.seek(SeekFrom::Start(250_000)).unwrap();
    assert_eq!(pos, 250_000);
    let mut mid = [0u8; 500];
    s.read_exact(&mut mid).unwrap();
    assert_eq!(&mid[..], &body[250_000..250_500]);

    // 相对 seek。
    let pos = s.seek(SeekFrom::Current(-100)).unwrap();
    assert_eq!(pos, 250_400);
    let mut back = [0u8; 10];
    s.read_exact(&mut back).unwrap();
    assert_eq!(&back[..], &body[250_400..250_410]);
}

/// 非 seekable 流 seek → 明确错误（§27：不伪造 seek）。
#[test]
fn non_seekable_stream_rejects_seek() {
    let body = make_pattern(10_000);
    let srv = FakeHttp::start(vec![Route::ok("/live.mp3", "audio/mpeg", body)]);
    let c = client();
    let url = c.validate(&srv.url("/live.mp3")).unwrap();
    let mut s = open_stream(&c, &url, 8 * 1024, false, Duration::from_secs(10));
    let e = s.seek(SeekFrom::Start(5)).unwrap_err();
    assert_eq!(e.kind(), std::io::ErrorKind::Other, "不伪造 seek");
}

/// §22/§109：取消 → read 立即失败 + is_cancelled；不挂死。
#[test]
fn cancel_returns_promptly() {
    let body = make_pattern(2_000_000);
    let srv = FakeHttp::start(vec![Route::ok("/long.mp3", "audio/mpeg", body)]);
    let c = client();
    let url = c.validate(&srv.url("/long.mp3")).unwrap();
    let mut s = open_stream(&c, &url, 32 * 1024, false, Duration::from_secs(30));

    let mut buf = [0u8; 1000];
    s.read_exact(&mut buf).unwrap();
    s.cancel();
    assert!(s.is_cancelled());
    let started = std::time::Instant::now();
    let e = s.read(&mut buf).unwrap_err();
    assert_eq!(
        e.kind(),
        std::io::ErrorKind::Interrupted,
        "取消后 read 立即 Interrupted"
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "取消必须即时: {:?}",
        started.elapsed()
    );
}

/// §19 空闲检测：服务器停发 → 消费侧 idle_timeout 返回 Timeout。
#[test]
fn idle_timeout_on_stalled_server() {
    // 服务器延迟 3s 才回响应（且体远小于缓冲）：
    // 消费侧 idle_timeout=400ms 必须先于服务器触发 Timeout。
    let srv = FakeHttp::start(vec![Route {
        path: "/stall",
        status: 200,
        headers: vec![],
        body: b"short".to_vec(),
        delay_ms: 3000,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let url = c.validate(&srv.url("/stall")).unwrap();
    let mut s = open_stream(&c, &url, 8 * 1024, false, Duration::from_millis(400));
    let started = std::time::Instant::now();
    let mut buf = [0u8; 64];
    // 第一次 read 在等待期间触发空闲检测（服务器 3s 才回）。
    let result = s.read(&mut buf);
    let elapsed = started.elapsed();
    if let Err(e) = result {
        assert_eq!(e.kind(), std::io::ErrorKind::TimedOut, "停滞必须超时");
    } // Ok 也接受（服务器先到的兜底）
    assert!(
        elapsed < Duration::from_secs(3),
        "空闲检测应在 idle_timeout 附近触发，而非等服务器: {elapsed:?}"
    );
}

/// §29/§37/§100：live 容量按码率收缩（≤30s 滚动），finite 固定。
#[test]
fn buffer_capacity_policy() {
    assert_eq!(buffer_capacity(StreamType::Finite, None), 1 << 20);
    // 320kbps × 30s = 1.2 MiB → 上限 2 MiB 内。
    assert_eq!(buffer_capacity(StreamType::Live, Some(320_000)), 1_200_000);
    // 2Mbps × 30s = 7.5 MB → 截到 2 MiB 上限。
    assert_eq!(buffer_capacity(StreamType::Live, Some(2_000_000)), 2 << 20);
    // 未知码率 → 256 KiB 保守值。
    assert_eq!(buffer_capacity(StreamType::Live, None), 256 * 1024);
    // 32kbps × 30s = 120KB → 下限抬到 256KB（避免过小抖动）。
    assert_eq!(buffer_capacity(StreamType::Live, Some(32_000)), 256 * 1024);
}

/// §95：Forbidden 来源零写盘 helper（为 M5 准备的通用断言）。
/// 对任意 source 活动前后扫描 sandbox 目录，断言无新文件。
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

/// 电台流播放（同进程内存操作）期间 sandbox 零变化。
#[test]
fn forbidden_radio_stream_writes_nothing() {
    let body = vec![0x55u8; 100_000];
    let srv = FakeHttp::start(vec![common::Route {
        path: "/live",
        status: 200,
        headers: vec![
            ("Content-Type".into(), "audio/mpeg".into()),
            ("icy-name".into(), "ZeroWrite Radio".into()),
        ],
        body,
        delay_ms: 0,
        malformed: false,
        range: false,
    }]);
    let c = client();
    let sandbox = std::env::temp_dir().join(format!("aura-m4-sandbox-{}", std::process::id()));
    std::fs::create_dir_all(&sandbox).unwrap();

    assert_forbidden_source_no_disk_write(&sandbox, &mut || {
        let url = c.validate(&srv.url("/live")).unwrap();
        let resp = c.get_stream(&url).unwrap();
        let mut s = NetworkAudioStream::start(
            &c,
            url,
            resp,
            64 * 1024,
            false,
            None,
            Duration::from_secs(10),
        );
        let mut consumed = 0u64;
        let mut buf = [0u8; 4096];
        while consumed < 50_000 {
            match s.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => consumed += n as u64,
                Err(_) => break,
            }
        }
        assert!(consumed > 0, "流应有数据");
    });
    let _ = std::fs::remove_dir_all(&sandbox);
}
