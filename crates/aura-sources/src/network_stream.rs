//! 网络音频流（§28/§29/§62）：有界内存缓冲 + feeder 线程 + 背压 +
//! 消费侧空闲检测 + 取消 + （仅 finite+Accept-Ranges 时）Range 重请求 seek。
//!
//! 内存边界（§29/§100）：管道容量构造时固定（finite 1 MiB；live 按
//! `min(bitrate×30s, 2 MiB)` 收缩，未知码率 256 KiB，§37/D-051），无限累积
//! 被结构排除。
//!
//! 取消协议（D-052，reqwest blocking 无 read_timeout 的工程解）：
//! `Response` 唯一宿主是共享 slot；feeder **分段借用**（take → read 一块 →
//! 放回）。取消方持锁取走并 drop response 即关闭 socket，随后 feeder 的
//! 下一次 read 以错误退出并看到 cancel 标志。stalled-read 期间 take 会等到
//! 该块读完（消费侧立即返回 Timeout，不等待；socket 由 tcp_keepalive 兜底
//! 关闭，feeder 线程随之退出 —— 不产生永久孤儿线程）。

use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::error::{Result, SourceError};
use crate::http::HttpClient;

/// finite 流默认管道容量：1 MiB。
pub const FINITE_BUFFER_BYTES: usize = 1 << 20;
/// live 流容量上限。
pub const LIVE_BUFFER_MAX_BYTES: usize = 2 << 20;
/// live 流未知码率时的保守容量（≈ 64kbps × 30s）。
pub const LIVE_BUFFER_DEFAULT_BYTES: usize = 256 * 1024;

/// 按流形态与码率计算管道容量（D-051）。
#[must_use]
pub fn buffer_capacity(stream_type: crate::model::StreamType, bitrate_bps: Option<u64>) -> usize {
    match stream_type {
        crate::model::StreamType::Live => {
            let for_30s = bitrate_bps.map(|b| (b / 8).saturating_mul(30) as usize);
            match for_30s {
                Some(n) => n.clamp(LIVE_BUFFER_DEFAULT_BYTES, LIVE_BUFFER_MAX_BYTES),
                None => LIVE_BUFFER_DEFAULT_BYTES,
            }
        }
        _ => FINITE_BUFFER_BYTES,
    }
}

/// feeder 借用的字节源（Response 或其上的 IcyReader 等组合层）。
pub type ByteSource = Box<dyn Read + Send>;
type ResponseSlot = Arc<Mutex<Option<ByteSource>>>;

struct StreamCore {
    state: Mutex<StreamState>,
    cv: Condvar,
    slot: ResponseSlot,
    cancel: AtomicBool,
    /// feeder 代数：seek 重启时 +1；旧代 feeder 在下一次边界检查退出。
    generation: AtomicU64,
    idle_timeout: Duration,
    capacity: usize,
    seekable: bool,
    content_length: Option<u64>,
    url: url::Url,
    http: HttpClient,
}

struct StreamState {
    buffer: VecDeque<u8>,
    feeder_done: bool,
    last_progress: Instant,
    error: Option<SourceError>,
    consumed: u64,
}

/// 网络音频流：`Read`（阻塞、有界、可取消）+ 条件 `Seek`。
pub struct NetworkAudioStream {
    core: Arc<StreamCore>,
}

impl NetworkAudioStream {
    /// 从 HTTP 响应构建（feeder 线程立即开始预填）。
    #[must_use]
    pub fn start(
        http: &HttpClient,
        url: url::Url,
        response: reqwest::blocking::Response,
        capacity: usize,
        seekable: bool,
        content_length: Option<u64>,
        idle_timeout: Duration,
    ) -> Self {
        Self::start_reader(
            http,
            url,
            Box::new(response),
            capacity,
            seekable,
            content_length,
            idle_timeout,
        )
    }

    /// 从任意字节源构建（如 ICY 剥离层包裹的响应）。
    #[must_use]
    pub fn start_reader(
        http: &HttpClient,
        url: url::Url,
        reader: ByteSource,
        capacity: usize,
        seekable: bool,
        content_length: Option<u64>,
        idle_timeout: Duration,
    ) -> Self {
        let core = Arc::new(StreamCore {
            state: Mutex::new(StreamState {
                buffer: VecDeque::new(),
                feeder_done: false,
                last_progress: Instant::now(),
                error: None,
                consumed: 0,
            }),
            cv: Condvar::new(),
            slot: Arc::new(Mutex::new(Some(reader))),
            cancel: AtomicBool::new(false),
            generation: AtomicU64::new(1),
            idle_timeout,
            capacity,
            seekable,
            content_length,
            url,
            http: http.clone(),
        });
        spawn_feeder(Arc::clone(&core), 1);
        Self { core }
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.core.cancel.load(Ordering::Relaxed)
    }

    /// 用户取消（§22/§109）：立即关闭响应、终结流；read 返回 `Cancelled`。
    /// feeder 线程在当前块读完后的下一次边界检查退出（正常服务器 = 毫秒级）。
    pub fn cancel(&self) {
        self.shutdown_core(SourceError::Cancelled);
    }

    /// 共享关闭路径：置取消 + 取走并丢弃 response + 唤醒所有等待者。
    fn shutdown_core(&self, error: SourceError) {
        self.core.cancel.store(true, Ordering::Relaxed);
        // 块间（feeder 未持有 response 时）可立即拿到；mid-read 时等当前块。
        if let Ok(mut slot) = self.core.slot.lock() {
            *slot = None;
        }
        if let Ok(mut st) = self.core.state.lock() {
            st.feeder_done = true;
            st.error = Some(error);
        }
        self.core.cv.notify_all();
    }

    /// seek：终止当前 feeder，Range 重请求，换代重启。
    fn range_seek(&self, pos: u64) -> Result<()> {
        if !self.core.seekable {
            return Err(SourceError::UnsupportedInput {
                reason: "stream is not seekable".into(),
            });
        }
        // 1) 关闭旧响应 + 置换代。
        self.core.cancel.store(true, Ordering::Relaxed);
        if let Ok(mut slot) = self.core.slot.lock() {
            *slot = None;
        }
        {
            let mut st = self.core.state.lock().expect("state lock");
            st.buffer.clear();
            st.feeder_done = false;
            st.error = None;
            st.last_progress = Instant::now();
            st.consumed = pos;
        }
        // 2) Range 请求（逐跳校验由 client 的 redirect policy 继续 负责）。
        let url = self.core.url.clone();
        let resp = self
            .core
            .http
            .get_stream_with_headers(&url, &[("Range", format!("bytes={pos}-"))])?;
        let status = resp.status();
        if pos > 0 && status == reqwest::StatusCode::OK {
            // 服务器忽略 Range：无法定位，宁可明确失败也不伪造（§27）。
            return Err(SourceError::InvalidResponse {
                reason: "server ignored Range request".into(),
            });
        }
        if !status.is_success() {
            return Err(SourceError::InvalidResponse {
                reason: format!("HTTP {status} after Range request"),
            });
        }
        // 3) 换代 + 注入新响应 + 新 feeder。
        self.core.cancel.store(false, Ordering::Relaxed);
        let new_gen = self.core.generation.load(Ordering::Relaxed) + 1;
        self.core.generation.store(new_gen, Ordering::Relaxed);
        *self.core.slot.lock().expect("slot lock") = Some(Box::new(resp));
        spawn_feeder(Arc::clone(&self.core), new_gen);
        Ok(())
    }

    /// 消费一次等待循环（单个条件检查周期）。
    fn wait_once(core: &StreamCore, mut st: MutexGuard<'_, StreamState>) -> WaitOnce {
        if core.cancel.load(Ordering::Relaxed) {
            return WaitOnce::Fail(st.error.take().unwrap_or(SourceError::Cancelled));
        }
        let has_error = st.error.is_some();
        if let Some(e) = st.error.take() {
            if st.buffer.is_empty() {
                return WaitOnce::Fail(e); // 错误但还有缓冲数据：先吐数据，下次再报
            }
            // 保留错误供下一轮。
            st.error = Some(e);
        }
        if !st.buffer.is_empty() {
            return WaitOnce::Data;
        }
        if has_error {
            let e = st.error.take().unwrap_or(SourceError::Cancelled);
            return WaitOnce::Fail(e);
        }
        if st.feeder_done {
            return WaitOnce::Eof; // 干净 EOF（§110：finite 正常；live 语义上层定义）
        }
        if Instant::now().duration_since(st.last_progress) > core.idle_timeout {
            return WaitOnce::Stall;
        }
        let _guard = core
            .cv
            .wait_timeout(st, Duration::from_millis(100))
            .expect("condvar");
        WaitOnce::Retry
    }
}

/// [`NetworkAudioStream::wait_once`] 的结果。
enum WaitOnce {
    /// 缓冲有数据。
    Data,
    /// 干净 EOF。
    Eof,
    /// 空闲停滞：调用方 shutdown 后重试。
    Stall,
    /// 最终失败。
    Fail(SourceError),
    /// 条件未就绪，重锁重查。
    Retry,
}

impl Read for NetworkAudioStream {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        let n = loop {
            let st = self.core.state.lock().expect("state lock");
            match Self::wait_once(&self.core, st) {
                WaitOnce::Data => {
                    let mut st = self.core.state.lock().expect("state lock");
                    let n = out.len().min(st.buffer.len());
                    for b in out.iter_mut().take(n) {
                        *b = st.buffer.pop_front().unwrap_or(0);
                    }
                    st.consumed += n as u64;
                    break n;
                }
                WaitOnce::Eof => break 0,
                WaitOnce::Stall => {
                    // 必须先释放锁（wait_once 已释放）再 shutdown。
                    self.shutdown_core(SourceError::Timeout);
                    // shutdown 后 cancel/error 置位 → 下轮 Fail 返回。
                }
                WaitOnce::Fail(e) => return Err(map_to_io(e)),
                WaitOnce::Retry => {}
            }
        };
        Ok(n)
    }
}

/// 背压：feeder 每次写入后 notify 消费侧；消费侧 pop 后 notify feeder。
impl Drop for NetworkAudioStream {
    fn drop(&mut self) {
        self.shutdown_core(SourceError::Cancelled);
    }
}

impl Seek for NetworkAudioStream {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(p) => p,
            SeekFrom::Current(d) => {
                let cur = self.core.state.lock().expect("state lock").consumed;
                if d >= 0 {
                    cur + d as u64
                } else {
                    cur.saturating_sub(d.unsigned_abs())
                }
            }
            SeekFrom::End(d) => {
                let Some(len) = self.core.content_length else {
                    return Err(map_to_io(SourceError::UnsupportedInput {
                        reason: "stream length unknown; SeekFrom::End unavailable".into(),
                    }));
                };
                if d >= 0 {
                    len + d as u64
                } else {
                    len.saturating_sub(d.unsigned_abs())
                }
            }
        };
        self.range_seek(target).map_err(map_to_io)?;
        Ok(target)
    }
}

fn map_to_io(e: SourceError) -> std::io::Error {
    let kind = match &e {
        SourceError::Timeout => std::io::ErrorKind::TimedOut,
        SourceError::Cancelled => std::io::ErrorKind::Interrupted,
        _ => std::io::ErrorKind::Other,
    };
    std::io::Error::new(kind, e.to_string())
}

fn spawn_feeder(core: Arc<StreamCore>, generation: u64) {
    let _ = std::thread::Builder::new()
        .name(format!("aura-net-feeder-{generation}"))
        .spawn(move || feeder_loop(&core, generation));
}

fn feeder_loop(core: &Arc<StreamCore>, generation: u64) {
    let mut scratch = vec![0u8; 64 * 1024];
    loop {
        // 边界检查：取消 / 换代 → 退出（response 已被关闭方取走）。
        if core.cancel.load(Ordering::Relaxed)
            || core.generation.load(Ordering::Relaxed) != generation
        {
            return;
        }
        // 分段借用：take → read 一块 → 放回（D-052 协议）。
        let mut source = loop {
            if core.cancel.load(Ordering::Relaxed)
                || core.generation.load(Ordering::Relaxed) != generation
            {
                return;
            }
            if let Ok(mut slot) = core.slot.try_lock() {
                if let Some(resp) = slot.take() {
                    break resp;
                }
            }
            // 无响应可借（取消方短暂持有过）：短暂等待后重查边界。
            std::thread::sleep(Duration::from_millis(5));
        };

        // 背压（§62）：缓冲满时不读，放回所有权并让出。
        let space = {
            let st = core.state.lock().expect("state lock");
            core.capacity.saturating_sub(st.buffer.len())
        };
        if space == 0 {
            put_back(core, source);
            core.cv.notify_all();
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }

        let want = scratch.len().min(space);
        let read = source.read(&mut scratch[..want]);
        // 无论结果如何，先把所有权放回（取消方此后总能取到）。
        put_back(core, source);
        // 换代/取消检查必须在写入缓冲之前：旧代 feeder 的数据不得污染
        // 新代已清空的缓冲（range seek 正确性）。
        if core.cancel.load(Ordering::Relaxed)
            || core.generation.load(Ordering::Relaxed) != generation
        {
            return;
        }
        let mut st = core.state.lock().expect("state lock");
        match read {
            Ok(0) => {
                st.feeder_done = true;
                core.cv.notify_all();
                return;
            }
            Ok(n) => {
                st.last_progress = Instant::now();
                st.buffer.extend(&scratch[..n]);
            }
            Err(e) => {
                st.feeder_done = true;
                if st.error.is_none() {
                    st.error = Some(SourceError::from(e));
                }
                core.cv.notify_all();
                return;
            }
        }
        drop(st);
        core.cv.notify_all();
    }
}

/// 所有权放回。若取消方已把 slot 清空并放弃，这里的 drop 即关闭 socket
/// （符合取消语义）；否则恢复可借状态。
fn put_back(core: &StreamCore, source: ByteSource) {
    if let Ok(mut slot) = core.slot.lock() {
        if slot.is_none() && !core.cancel.load(Ordering::Relaxed) {
            *slot = Some(source);
        }
    }
    // else: source 在此 drop → 连接关闭。
}

impl aura_core::decoder::AudioByteStream for NetworkAudioStream {
    fn is_seekable(&self) -> bool {
        self.core.seekable
    }

    fn byte_len(&self) -> Option<u64> {
        self.core.content_length
    }
}
