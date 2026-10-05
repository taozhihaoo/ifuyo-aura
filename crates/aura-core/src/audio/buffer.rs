//! lock-free SPSC 音频环形缓冲（M1 §11–13）。
//!
//! - 底层 `rtrb`（M0 已锁定），SPSC 由类型系统强制（句柄不可 Clone，各只有一份）；
//! - 包装为**帧**语义（交错 f32，容量按帧对齐）；
//! - generation-aware discard：producer bump 时记录缓冲中残留的旧代帧数，
//!   consumer 检测到代变化后**只精确丢弃这些旧代帧**（bump 之后写入的新代数据
//!   不受影响）；并发残余竞态见 DECISIONS D-019，确定性单线程路径下零泄漏；
//! - overrun 策略（D-022）：producer 拒绝写入（返回实际写入帧数），由 engine 侧
//!   背压，不覆盖、不丢数据；underrun 策略：consumer 返回不足帧数，由输出侧补静音。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rtrb::RingBuffer;

use crate::error::AudioError;
use crate::types::{Channels, Generation};

/// 创建一对帧语义的 SPSC 句柄。`capacity_samples` 必须是声道数的整数倍。
pub fn ring_buffer(
    channels: Channels,
    capacity_samples: usize,
) -> Result<(ProducerHandle, ConsumerHandle), AudioError> {
    if capacity_samples == 0 || !capacity_samples.is_multiple_of(usize::from(channels.get())) {
        return Err(AudioError::InvalidParameter(
            "capacity must be a non-zero multiple of channels",
        ));
    }
    let generation = Arc::new(AtomicU64::new(Generation::FIRST.get()));
    let stale_frames = Arc::new(AtomicU64::new(0));
    let (producer, consumer) = RingBuffer::new(capacity_samples);
    let capacity_frames = capacity_samples / usize::from(channels.get());
    Ok((
        ProducerHandle {
            inner: producer,
            channels,
            capacity_frames,
            generation: Arc::clone(&generation),
            stale_frames: Arc::clone(&stale_frames),
        },
        ConsumerHandle {
            inner: consumer,
            channels,
            capacity_frames,
            generation,
            stale_frames,
        },
    ))
}

/// 生产端句柄（唯一）。属于解码/填充线程。
pub struct ProducerHandle {
    inner: rtrb::Producer<f32>,
    channels: Channels,
    capacity_frames: usize,
    generation: Arc<AtomicU64>,
    stale_frames: Arc<AtomicU64>,
}

impl ProducerHandle {
    #[must_use]
    pub const fn channels(&self) -> Channels {
        self.channels
    }

    /// 缓冲总容量（帧）。
    #[must_use]
    pub const fn capacity_frames(&self) -> usize {
        self.capacity_frames
    }

    /// 当前剩余可写空间（帧）。
    #[must_use]
    pub fn free_frames(&self) -> usize {
        self.inner.slots() / usize::from(self.channels.get())
    }

    /// 写入交错 PCM；缓冲不足时**部分写入**（不覆盖、不丢弃已消费数据）。
    /// 返回实际写入的帧数；调用方据此背压。
    pub fn push_frames(&mut self, interleaved: &[f32]) -> u64 {
        let (pushed, _) = self.inner.push_partial_slice(interleaved);
        (pushed.len() / usize::from(self.channels.get())) as u64
    }

    /// 进入新数据代：load / seek 时调用。
    ///
    /// 记录当前缓冲中残留的旧代帧数，供 consumer 精确丢弃；
    /// bump 之后 producer 写入的数据自动属于新代。
    pub fn bump_generation(&mut self) -> Generation {
        let buffered = self.capacity_frames - self.free_frames();
        eprintln!("[gen-probe] bump: buffered={buffered} frames");
        self.stale_frames.store(buffered as u64, Ordering::Release);
        let g = self
            .generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        Generation::from_raw(g)
    }

    /// 当前代（producer 视角）。
    #[must_use]
    pub fn generation(&self) -> Generation {
        Generation::from_raw(self.generation.load(Ordering::Acquire))
    }
}

/// 消费端句柄（唯一）。属于音频输出回调侧。
pub struct ConsumerHandle {
    inner: rtrb::Consumer<f32>,
    channels: Channels,
    capacity_frames: usize,
    generation: Arc<AtomicU64>,
    stale_frames: Arc<AtomicU64>,
}

impl ConsumerHandle {
    #[must_use]
    pub const fn channels(&self) -> Channels {
        self.channels
    }

    /// 缓冲总容量（帧）。
    #[must_use]
    pub const fn capacity_frames(&self) -> usize {
        self.capacity_frames
    }

    /// 当前可读（帧）。
    #[must_use]
    pub fn available_frames(&self) -> usize {
        self.inner.slots() / usize::from(self.channels.get())
    }

    /// 当前数据代。
    #[must_use]
    pub fn read_generation(&self) -> Generation {
        Generation::from_raw(self.generation.load(Ordering::Acquire))
    }

    /// 原始 FIFO 读取；返回实际弹出的帧数（underrun 时少于请求）。
    pub fn pop_frames(&mut self, out: &mut [f32]) -> u64 {
        let (popped, _) = self.inner.pop_partial_slice(out);
        (popped.len() / usize::from(self.channels.get())) as u64
    }

    /// generation-aware 读取（M1 §12/§21–22）。
    ///
    /// 若检测到数据代变化：精确丢弃 bump 时记录的旧代帧数（不含新代数据），
    /// `seen` 更新为新代；随后同一调用内正常读取（bump 时缓冲为空的场景不
    /// 牺牲任何块）。弹出过程中若发生代切换，本块整体丢弃并清掉残余后返回 0。
    pub fn pop_frames_until_generation(&mut self, out: &mut [f32], seen: &mut Generation) -> u64 {
        let current = self.read_generation();
        if current != *seen {
            *seen = current;
            self.discard_stale();
        }
        let before = self.read_generation();
        let frames = self.pop_frames(out);
        let after = self.read_generation();
        if before != after {
            // 弹出过程中发生代切换：本块可能混有旧代样本，整块丢弃并清掉残余。
            *seen = after;
            self.discard_stale();
            return 0;
        }
        frames
    }

    /// 丢弃残留旧代帧（精确数量，不波及 bump 后写入的新代数据）。
    fn discard_stale(&mut self) {
        let mut to_skip = self.stale_frames.swap(0, Ordering::AcqRel);
        if to_skip > 0 {
            let mut skipped = 0u64;
            let ch = usize::from(self.channels.get());
            let mut sink = [0.0f32; 4096];
            let sink_frames = sink.len() / ch;
            let mut probe_rest = to_skip;
            while probe_rest > 0 {
                let want = probe_rest.min(sink_frames as u64) as usize;
                let (popped, _) = self.inner.pop_partial_slice(&mut sink[..want * ch]);
                let n = (popped.len() / ch) as u64;
                if n == 0 {
                    break;
                }
                skipped += n;
                probe_rest -= n;
            }
            eprintln!(
                "[gen-probe] discard_stale: requested={to_skip} skipped={skipped} (首样本值≈{:.1})",
                sink[0]
            );
            to_skip -= skipped;
            if to_skip > 0 {
                eprintln!("[gen-probe] discard_stale UNDER-DISCARD by {to_skip}");
            }
            return;
        }
        if to_skip == 0 {
            return;
        }
        let ch = usize::from(self.channels.get());
        let mut sink = [0.0f32; 4096];
        let sink_frames = sink.len() / ch;
        while to_skip > 0 {
            let want_frames = to_skip.min(sink_frames as u64) as usize;
            let (popped, _) = self.inner.pop_partial_slice(&mut sink[..want_frames * ch]);
            let frames = (popped.len() / ch) as u64;
            if frames == 0 {
                break;
            }
            to_skip = to_skip.saturating_sub(frames);
        }
    }
}
