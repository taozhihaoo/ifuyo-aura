//! 零分配证明（M1 §31–32）：回调执行路径（OutputStage::render + ring pop +
//! capture 追加）不得产生任何堆分配。
//!
//! 机制：计数全局分配器（真实机制，不是"看起来没有 Vec"）。
//! 本文件是独立测试二进制，全局分配器替换不影响其他测试。

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use aura_core::types::AudioCommand;
use aura_core::{Channels, Engine, SampleRate};
use common::{SyntheticDecoder, Waveform};

mod common;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

#[test]
fn callback_render_path_is_allocation_free() {
    let (mut engine, mut poster) = Engine::new(
        SampleRate::new(48_000).unwrap(),
        Channels::new(2).unwrap(),
        8192,
    )
    .unwrap();
    // 10 秒 ramp，一次性灌满 8192 帧缓冲
    engine
        .load_track(Box::new(SyntheticDecoder::new(
            48_000,
            2,
            48_000 * 10,
            1024,
            Waveform::Ramp,
        )))
        .unwrap();
    engine.reserve_capture(16_384);
    poster.post(AudioCommand::Play).unwrap();
    poster
        .post(AudioCommand::SetVolume { volume: 0.7 })
        .unwrap();

    // 预热：触发所有一次性分配（捕获缓冲、解码缓冲、映射缓冲等）
    engine.render_for_frames(4096);
    engine.clear_capture();

    // 度量边界（D-026）：M1 单线程引擎中 decode/fill 属解码面（分配允许），
    // 零分配承诺覆盖 **OutputStage::render 回调路径**（真实线程模型中即
    // audio callback）。先显式 fill 灌满缓冲（fill 本身在度量之外），
    // 度量窗口内的 fill 因背压变为 no-op，窗口内即纯回调路径。
    engine.fill();
    poster
        .post(AudioCommand::SetVolume { volume: 0.6 })
        .unwrap();
    let warm = ALLOC_COUNT.load(Ordering::Relaxed);

    // 恰好一个渲染 chunk：缓冲满态背压，fill 为 no-op
    let rendered = engine.render_for_frames(1024);

    let after = ALLOC_COUNT.load(Ordering::Relaxed);
    assert_eq!(rendered, 1024);
    let leaked = after.abs_diff(warm);
    assert_eq!(leaked, 0, "回调渲染路径出现额外堆分配（必须为 0）");
}
