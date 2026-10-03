//! Internet Radio Source（§30–§37/§91/§94/§110）：live、non-seekable、
//! `CachePolicy::Forbidden`、ICY 元数据剥离 + StreamMetadataEvent 通道、
//! ≤30s 滚动内存缓冲（D-051）。EOF 语义（§110）：live 流的 EOF 是
//! 「流结束/来源失败」，由上层决定后续，不自动切轨。

use std::io::Read;
use std::sync::mpsc;

use crate::error::{Result, SourceError};
use crate::http::HttpClient;
use crate::icy::{meta_to_event, IcyReader};
use crate::model::{
    CachePolicy, Resolved, ResolvedStream, SourceKind, StreamMetadataEvent, StreamType,
};
use crate::network_stream::{buffer_capacity, ByteSource, NetworkAudioStream};

/// 打开一个网络电台流。
///
/// - 请求头携带 `Icy-MetaData: 1` + `Icy-Aware`（§31）；
/// - `icy-metaint` 存在时注入 [`IcyReader`] 剥离元数据；
/// - 元数据事件经 mpsc 通道送出（§91）；
/// - 缓冲 = min(码率×30s, 2 MiB)（§37/D-051）。
///
/// # Errors
/// 网络/状态/重定向策略拒绝。
pub fn open(http: &HttpClient, url: &url::Url, station: Option<String>) -> Result<RadioStream> {
    let resp = http.get_stream_with_headers(
        url,
        &[
            ("Icy-MetaData", "1".to_string()),
            ("Icy-Aware", "true".to_string()),
        ],
    )?;

    let headers = resp.headers();
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    let metaint: Option<usize> = header("icy-metaint").and_then(|v| v.trim().parse().ok());
    let bitrate_bps: Option<u64> = header("icy-br")
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(|k| k * 1000);
    let icy_name = header("icy-name");
    let content_length = resp.content_length();

    let (tx, rx) = mpsc::channel::<StreamMetadataEvent>();
    let station_cb = station.clone();
    let reader: ByteSource = match metaint {
        Some(interval) => {
            let mut icy = IcyReader::new(resp, interval);
            icy.on_meta = Some(Box::new(move |meta| {
                if let Some(ev) = meta_to_event(meta, station_cb.as_deref()) {
                    // 通道满（消费慢）时丢弃旧事件语义：try_send 即可。
                    let _ = tx.send(ev); // 无界通道：try 不存在，send 恒成功（feeder 独占发送端）
                }
            }));
            Box::new(icy)
        }
        None => Box::new(resp),
    };

    let capacity = buffer_capacity(StreamType::Live, bitrate_bps);
    let idle = http.config().stream_idle_timeout;
    let stream = NetworkAudioStream::start_reader(
        http,
        url.clone(),
        reader,
        capacity,
        false,
        content_length,
        idle,
    );

    Ok(RadioStream {
        stream,
        events: rx,
        cache_policy: CachePolicy::Forbidden,
        station: icy_name.or(station),
    })
}

/// 电台流句柄：音频读取 + 元数据事件 + 生命周期。
pub struct RadioStream {
    stream: NetworkAudioStream,
    events: mpsc::Receiver<StreamMetadataEvent>,
    /// 恒为 [`CachePolicy::Forbidden`]（§37/§94）。
    pub cache_policy: CachePolicy,
    /// 电台名（icy-name 优先）。
    pub station: Option<String>,
}

impl RadioStream {
    /// 取当前积累的元数据事件（非阻塞；§35/§91 backend 事件）。
    #[must_use]
    pub fn try_next_event(&self) -> Option<StreamMetadataEvent> {
        self.events.try_recv().ok()
    }

    /// 用户取消（§22）。
    pub fn cancel(&self) {
        self.stream.cancel();
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.stream.is_cancelled()
    }

    /// 拆出音频字节流与元数据事件接收端（M6 shell 装载用：NetworkAudioStream
    /// 自身即 [`aura_core::decoder::AudioByteStream`]；事件端当前由调用方
    /// 决定消费策略，drop 即放弃 ICY StreamTitle → UI 接线，属已知限制）。
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        NetworkAudioStream,
        std::sync::mpsc::Receiver<StreamMetadataEvent>,
    ) {
        (self.stream, self.events)
    }
}

impl Read for RadioStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

/// 从 resolver 的 [`Resolved`] 打开电台流（便捷入口）。
///
/// # Errors
/// kind 不是 [`SourceKind::InternetRadio`] 时返回
/// [`SourceError::UnsupportedInput`]；网络错误原样。
pub fn open_resolved(http: &HttpClient, resolved: &Resolved) -> Result<RadioStream> {
    if resolved.kind != SourceKind::InternetRadio {
        return Err(SourceError::UnsupportedInput {
            reason: format!("resolved kind {:?} is not an internet radio", resolved.kind),
        });
    }
    let s: &ResolvedStream =
        resolved
            .streams
            .first()
            .ok_or_else(|| SourceError::UnsupportedInput {
                reason: "no stream".into(),
            })?;
    let url = url::Url::parse(&s.url).map_err(|_| SourceError::InvalidUrl {
        reason: "resolved stream url invalid".into(),
    })?;
    open(http, &url, resolved.meta.stream.icy_name.clone())
}
