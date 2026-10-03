//! CachedRangeReader（§29–§43/§79–§84/§95/§136）：cache 优先的字节流。
//!
//! 读取模型（§30）：offset → chunk span → 命中即读 → miss 则
//! Range 按块拉取（§33，206 + Content-Range 逐项校验 §34）→
//! 200（服务器忽略 Range）→ 标记 non-range + sequential fill（§35/§36）。
//!
//! - 离线（§38/§80/§83）：缓存命中照读；miss 网络失败 → `CacheError::Network`
//!   **不伪装 EOS**；
//! - revalidation（§40–§43/§84）：open 时 If-None-Match 一次；304=Validated、
//!   200=内容变更（作废旧 chunks 全量重填，§43 绝无半新半旧）、网络失败=
//!   Stale（离线可用）；
//! - retry（§45–§48）：仅 timeout/连接类/5xx，3 次，指数退避 base 200ms
//!   cap 1.6s，jitter 注入（测试 = 恒 0，确定性 §48/§106）；
//! - FetchKey 去重（§53/§95/§136）：`(content_key, chunk_index)` 共享
//!   in-flight 任务（同线程等待者复用结果；跨线程注册表 M5-g）。

use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::Duration;

use aura_sources::http::HttpClient;
use aura_sources::model::CachePolicy;

use crate::error::{CacheError, Result};
use crate::keys;
use crate::manager::CacheManager;
use crate::store::CHUNK_SIZE;

/// 读取器的源声明（来自 M4 `ResolvedStream`）。
#[derive(Debug, Clone)]
pub struct RangeSource {
    pub url: String,
    pub content_length: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub mime: Option<String>,
    pub codec: Option<String>,
    /// M4 探测结果；服务器实际行为可在首块后降级（§35）。
    pub seekable: bool,
}

/// 新鲜度状态（§84）：显式枚举，不用一堆 bool。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// 新声明的 entry（未验证）。
    Fresh,
    /// 304 确认后。
    Validated,
    /// revalidation 网络失败：离线可用（§83）。
    Stale,
    /// 无 validator（服务器未提供 ETag/LM）。
    Unknown,
}

/// 拉取失败（内部）。
enum FetchFail {
    /// 服务器不支持 Range（200 on Range）→ 转 sequential（§35）。
    RangeUnsupported,
    Source(aura_sources::error::SourceError),
}

impl From<aura_sources::error::SourceError> for FetchFail {
    fn from(e: aura_sources::error::SourceError) -> Self {
        Self::Source(e)
    }
}

/// 重试/退避配置（§45–§48）。测试注入确定性 sleeper。
#[derive(Clone)]
pub struct FetchOptions {
    pub max_attempts: u32,
    pub backoff_base: Duration,
    pub backoff_cap: Duration,
    /// 恒 0 = 测试确定性；生产可加有界 jitter（§48）。
    pub jitter: Arc<dyn Fn(u64) -> u64 + Send + Sync>,
    /// open 时是否做条件 revalidation（§42；默认 true）。
    pub revalidate_on_open: bool,
    /// 测试用：退避睡眠替换（默认真睡；测试注入 no-op 保证快速确定性）。
    pub sleeper: Arc<dyn Fn(Duration) + Send + Sync>,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_base: Duration::from_millis(200),
            backoff_cap: Duration::from_millis(1600),
            jitter: Arc::new(|_| 0),
            revalidate_on_open: true,
            sleeper: Arc::new(std::thread::sleep),
        }
    }
}

/// 可缓存范围读取器：`Read + Seek`（并实现 core 的 `AudioByteStream`）。
pub struct CachedRangeReader {
    manager: Arc<CacheManager>,
    http: HttpClient,
    opts: FetchOptions,
    policy: CachePolicy,
    ck: String,
    info: RangeSource,
    pos: u64,
    /// 服务器 Range 能力：None=未知 / Some(true)=206 / Some(false)=200。
    range_capable: Option<bool>,
    /// non-range 模式的顺序填充器（§36/§37）。
    seq: Option<SequentialFill>,
    /// 已填充高水位（sequential 模式的 seek 边界，§37）。
    filled_high_water: u64,
    freshness: Freshness,
}

/// sequential fill 状态：一个全量 GET 响应按 chunk 组装。
struct SequentialFill {
    response: Box<dyn aura_sources::http::byte_stream::ByteStream>,
    /// 当前响应内绝对 offset（响应从 0 开始）。
    stream_pos: u64,
}

impl CachedRangeReader {
    /// 打开（§78：上层不知道字节来自 cache 还是 HTTP）。
    ///
    /// # Errors
    /// Forbidden（§79 Bypassed 语义）；键非法。
    pub fn open(
        manager: Arc<CacheManager>,
        http: HttpClient,
        policy: CachePolicy,
        info: RangeSource,
        opts: FetchOptions,
    ) -> Result<Self> {
        if policy == CachePolicy::Forbidden {
            return Err(CacheError::PolicyViolation {
                reason: "Forbidden source bypasses cache entirely".into(),
            });
        }
        let ck = keys::content_key(&info.url)?;
        let mut reader = Self {
            manager,
            http,
            opts,
            policy,
            ck,
            info,
            pos: 0,
            range_capable: None,
            seq: None,
            filled_high_water: 0,
            freshness: Freshness::Fresh,
        };
        if reader.policy == CachePolicy::Allowed {
            if reader.opts.revalidate_on_open {
                reader.revalidate();
            } else {
                reader.ensure_entry();
            }
        }
        Ok(reader)
    }

    #[must_use]
    pub fn freshness(&self) -> Freshness {
        self.freshness
    }

    #[must_use]
    pub fn content_key(&self) -> &str {
        &self.ck
    }

    /// §42/§83/§84：open 时一次条件验证。
    /// 304 → Validated；200 → §43 内容变更（作废旧 chunks）；
    /// 网络失败 → Stale（离线可用）；无 validator → Unknown。
    fn revalidate(&mut self) {
        let Some(existing) = self.manager.entry_snapshot(&self.ck) else {
            self.ensure_entry();
            // 新 entry：用声明中的 validator（如服务器首响应携带）。
            if self.info.etag.is_some() || self.info.last_modified.is_some() {
                self.manager.update_validators(
                    &self.ck,
                    self.info.etag.clone(),
                    self.info.last_modified.clone(),
                );
                self.freshness = Freshness::Validated;
            }
            return;
        };
        if existing.etag.is_none() && existing.last_modified.is_none() {
            self.freshness = Freshness::Unknown;
            return;
        }
        let Ok(url) = url::Url::parse(&self.info.url) else {
            self.freshness = Freshness::Unknown;
            return;
        };
        let req = self.http.metadata_get_with_headers(
            &url,
            &[
                ("If-None-Match", existing.etag.clone().unwrap_or_default()),
                (
                    "If-Modified-Since",
                    existing.last_modified.clone().unwrap_or_default(),
                ),
            ],
        );
        let send = match req {
            Ok(rb) => rb.send(),
            Err(_) => {
                self.freshness = Freshness::Stale;
                return;
            }
        };
        match send {
            Ok(resp) => match resp.status().as_u16() {
                304 => self.freshness = Freshness::Validated,
                200 => {
                    // §43：内容变更 → 作废旧 chunks（新内容全量重填）。
                    self.manager.invalidate_content(&self.ck);
                    self.manager.update_validators(
                        &self.ck,
                        resp.headers()
                            .get("etag")
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_string)
                            .or_else(|| existing.etag.clone()),
                        resp.headers()
                            .get("last-modified")
                            .and_then(|v| v.to_str().ok())
                            .map(str::to_string)
                            .or_else(|| existing.last_modified.clone()),
                    );
                    self.freshness = Freshness::Validated;
                }
                _ => self.freshness = Freshness::Stale,
            },
            Err(_) => self.freshness = Freshness::Stale,
        }
    }

    fn ensure_entry(&mut self) {
        let _ = self
            .manager
            .ensure_entry(&crate::manager::EntryDeclaration {
                content_key: self.ck.clone(),
                source_kind: "DirectHttp".into(),
                url: self.info.url.clone(),
                content_length: self.info.content_length,
                mime: self.info.mime.clone(),
                codec: self.info.codec.clone(),
                seekable: self.info.seekable,
                live: false,
                policy: CachePolicy::Allowed,
                etag: self.info.etag.clone(),
                last_modified: self.info.last_modified.clone(),
            });
    }

    fn chunk_span(&self, idx: u64) -> (u64, usize) {
        let start = idx * CHUNK_SIZE as u64;
        let end = self
            .info
            .content_length
            .map(|l| (start + CHUNK_SIZE as u64).min(l))
            .unwrap_or(start + CHUNK_SIZE as u64);
        (start, usize::try_from(end - start).unwrap_or(CHUNK_SIZE))
    }

    /// §45–§48：有限重试 + 指数退避（仅 timeout/网络类/5xx）。
    fn with_retry<T>(&self, mut f: impl FnMut() -> std::result::Result<T, FetchFail>) -> Result<T> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match f() {
                Ok(v) => return Ok(v),
                Err(FetchFail::RangeUnsupported) => return Err(CacheError::RangeUnsupported),
                Err(FetchFail::Source(e)) => {
                    let retryable = matches!(
                        &e,
                        aura_sources::error::SourceError::Timeout
                            | aura_sources::error::SourceError::NetworkError { .. }
                    ) || matches!(
                        &e,
                        aura_sources::error::SourceError::InvalidResponse { reason }
                            if reason.contains("HTTP 5")
                    );
                    if !retryable || attempt >= self.opts.max_attempts {
                        return Err(map_source_err(e));
                    }
                    let exp = 2u32.saturating_pow(attempt.saturating_sub(1));
                    let mut ms = self
                        .opts
                        .backoff_base
                        .as_millis()
                        .saturating_mul(u128::from(exp));
                    ms = ms.min(self.opts.backoff_cap.as_millis());
                    let jitter = (self.opts.jitter)(u64::try_from(ms).unwrap_or(0)).min(500);
                    let sleep = Duration::from_millis(
                        u64::try_from(ms).unwrap_or(0).saturating_add(jitter),
                    );
                    (self.opts.sleeper)(sleep);
                }
            }
        }
    }

    /// §33/§34：Range 拉一个 chunk（206 + Content-Range 校验）。
    fn fetch_chunk_by_range(&self, idx: u64) -> Result<Vec<u8>> {
        let (start, len) = self.chunk_span(idx);
        let end = start + len as u64 - 1; // 含端
        let url = url::Url::parse(&self.info.url).map_err(|_| CacheError::InvalidKey {
            reason: "source url invalid".into(),
        })?;
        let bytes: std::result::Result<Vec<u8>, crate::error::CacheError> = self.with_retry(|| {
            let resp = match self
                .http
                .get_stream_with_headers(&url, &[("Range", format!("bytes={start}-{end}"))])
            {
                Ok(r) => r,
                Err(e) => return Err(FetchFail::Source(e)),
            };
            let status = resp.status().as_u16();
            match status {
                206 => {
                    // §34：Content-Range 逐项校验。
                    let cr = resp
                        .headers()
                        .get("content-range")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    let ok =
                        parse_content_range(cr).is_some_and(|(s, e, _)| s == start && e == end);
                    if !ok {
                        return Err(FetchFail::Source(
                            aura_sources::error::SourceError::InvalidResponse {
                                reason: format!("bad Content-Range: {cr}"),
                            },
                        ));
                    }
                    let boxed: Box<dyn aura_sources::http::byte_stream::ByteStream> =
                        Box::new(resp);
                    read_bounded(boxed, len).map_err(FetchFail::Source)
                }
                200 => Err(FetchFail::RangeUnsupported), // §35：不假定支持
                416 => Err(FetchFail::Source(
                    aura_sources::error::SourceError::InvalidResponse {
                        reason: "HTTP 416 range not satisfiable".into(),
                    },
                )),
                s => Err(FetchFail::Source(
                    aura_sources::error::SourceError::InvalidResponse {
                        reason: format!("HTTP {s}"),
                    },
                )),
            }
        });
        let bytes = bytes?;
        if bytes.len() != len {
            return Err(CacheError::Corrupt {
                what: format!("range size mismatch {} != {len}", bytes.len()),
            });
        }
        Ok(bytes)
    }

    /// 从 cache 或网络取 chunk（Allowed）。
    fn obtain_chunk(&mut self, idx: u64) -> Result<Vec<u8>> {
        if let Some(bytes) = self.manager.get_chunk(CachePolicy::Allowed, &self.ck, idx) {
            return Ok(bytes);
        }
        if self.range_capable == Some(false) {
            return self.seq_fill_chunk(idx);
        }
        let fetched: Result<Vec<u8>> = self.fetch_chunk_by_range(idx);
        match fetched {
            Ok(bytes) => {
                self.range_capable = Some(true);
                self.persist(idx, &bytes)?;
                Ok(bytes)
            }
            Err(CacheError::RangeUnsupported) => {
                // §35：确认 non-range → sequential fill。
                self.range_capable = Some(false);
                self.info.seekable = false;
                self.start_sequential()?;
                self.seq_fill_chunk(idx)
            }
            Err(e) => Err(e),
        }
    }

    fn persist(&self, idx: u64, bytes: &[u8]) -> Result<()> {
        self.manager
            .put_chunk(CachePolicy::Allowed, &self.ck, idx, bytes, false)
            .map(|_| ())
    }

    /// §36：全量 GET（无 Range）。
    fn start_sequential(&mut self) -> Result<()> {
        let url = url::Url::parse(&self.info.url).map_err(|_| CacheError::InvalidKey {
            reason: "source url invalid".into(),
        })?;
        let resp = self.with_retry(|| {
            self.http
                .get_stream_boxed(&url, &[])
                .map_err(FetchFail::Source)
        })?;
        self.seq = Some(SequentialFill {
            response: resp,
            stream_pos: 0,
        });
        Ok(())
    }

    /// §36/§37：推进顺序填充直到产出目标 chunk。
    fn seq_fill_chunk(&mut self, target_idx: u64) -> Result<Vec<u8>> {
        if self.seq.is_none() {
            self.start_sequential()?;
        }
        loop {
            let cur = self.seq.as_ref().map_or(0, |s| s.stream_pos);
            let idx = cur / CHUNK_SIZE as u64;
            let (start, len) = self.chunk_span(idx);
            let mut chunk = vec![0u8; len];
            let mut filled = usize::try_from(cur - start).unwrap_or(0);
            {
                let s = self.seq.as_mut().expect("seq");
                let resp = &mut s.response;
                while filled < len {
                    match resp.read(&mut chunk[filled..]) {
                        Ok(0) => {
                            // 真实 EOF。
                            chunk.truncate(filled);
                            self.seq = None;
                            self.persist(idx, &chunk)?;
                            return if idx == target_idx {
                                Ok(chunk)
                            } else {
                                Err(CacheError::Network {
                                    reason: "stream ended before requested offset".into(),
                                })
                            };
                        }
                        Ok(n) => {
                            filled += n;
                            s.stream_pos += n as u64;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
            }
            self.persist(idx, &chunk)?;
            self.filled_high_water = (idx + 1) * CHUNK_SIZE as u64;
            if idx == target_idx {
                return Ok(chunk);
            }
        }
    }

    fn read_at(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if let Some(len) = self.info.content_length {
            if self.pos >= len {
                return Ok(0);
            }
        }
        let idx = self.pos / CHUNK_SIZE as u64;
        let chunk = match self.obtain_chunk(idx) {
            Ok(c) => c,
            // §80：网络失败不得伪装 EOS。
            Err(e) => return Err(std::io::Error::other(e.to_string())),
        };
        let off_in = usize::try_from(self.pos - idx * CHUNK_SIZE as u64).unwrap_or(0);
        let available = chunk.len().saturating_sub(off_in);
        if available == 0 {
            return Ok(0); // 真实 EOF（无 content_length 的流尾）
        }
        let n = out.len().min(available);
        out[..n].copy_from_slice(&chunk[off_in..off_in + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Read for CachedRangeReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.read_at(buf)
    }
}

impl Seek for CachedRangeReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::Current(d) => {
                if d >= 0 {
                    self.pos.saturating_add(d as u64)
                } else {
                    self.pos.saturating_sub(d.unsigned_abs())
                }
            }
            SeekFrom::End(d) => {
                let Some(len) = self.info.content_length else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "SeekFrom::End without content length",
                    ));
                };
                if d >= 0 {
                    len.saturating_add(d as u64)
                } else {
                    len.saturating_sub(d.unsigned_abs())
                }
            }
        };
        // §37：non-range 只允许已填充区间内 seek（不伪造远程 seek）。
        if self.range_capable == Some(false) && target > self.filled_high_water {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "seek beyond buffered data on non-range source",
            ));
        }
        self.pos = target;
        Ok(self.pos)
    }
}

fn read_bounded(
    mut resp: Box<dyn aura_sources::http::byte_stream::ByteStream>,
    max: usize,
) -> std::result::Result<Vec<u8>, aura_sources::error::SourceError> {
    use std::io::Read;
    let mut buf = Vec::with_capacity(max);
    let mut chunk = [0u8; 16 * 1024];
    loop {
        let n = resp.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        if buf.len() + n > max {
            return Err(aura_sources::error::SourceError::InvalidResponse {
                reason: "range body oversized".into(),
            });
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(buf)
}

/// `Content-Range: bytes start-end/total` 解析（§34）。
#[must_use]
pub fn parse_content_range(v: &str) -> Option<(u64, u64, Option<u64>)> {
    let rest = v.trim().strip_prefix("bytes ")?;
    let (range, total) = rest.split_once('/')?;
    let (s, e) = range.split_once('-')?;
    let s = s.trim().parse().ok()?;
    let e = e.trim().parse().ok()?;
    let total = if total.trim() == "*" {
        None
    } else {
        Some(total.trim().parse().ok()?)
    };
    Some((s, e, total))
}

fn map_source_err(e: aura_sources::error::SourceError) -> CacheError {
    use aura_sources::error::SourceError as S;
    match e {
        S::Timeout => CacheError::Timeout,
        S::NetworkError { reason } | S::SourceUnavailable { reason } => {
            CacheError::Network { reason }
        }
        S::InvalidResponse { reason } => CacheError::Network { reason },
        S::InvalidUrl { reason } => CacheError::InvalidKey { reason },
        S::Cancelled => CacheError::Cancelled,
        other => CacheError::Network {
            reason: other.to_string(),
        },
    }
}
