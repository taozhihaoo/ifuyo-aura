//! 全 DSP 回调零分配证明（§61–62）——独立测试二进制避免并行计数干扰。

mod common;

const RATE: u32 = 48_000;
const RATE64: u64 = 48_000;

fn full_dsp_engine() -> (Engine, aura_core::AudioCommandQueue) {
    let (mut engine, poster) = Engine::new(
        SampleRate::new(RATE).unwrap(),
        Channels::new(2).unwrap(),
        16_384,
    )
    .unwrap();
    engine.set_crossfade_seconds(1.0);
    engine.set_limiter_enabled(true);
    engine.set_track_loudness(
        aura_core::TrackLoudness {
            integrated_lufs: -20.0,
            true_peak_db: -60.0,
            loudness_available: true,
        },
        true,
    );
    engine.set_preamp_db(0.0);
    engine.set_eq_preset(&aura_core::dsp::EQ_PRESETS[1]);
    (engine, poster)
}

fn sine_track(freq: f32, frames: u64) -> Box<SyntheticDecoder> {
    Box::new(SyntheticDecoder::new(
        RATE,
        2,
        frames,
        512,
        Waveform::Sine {
            freq_hz: freq,
            amplitude: 0.6,
        },
    ))
}

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use aura_core::{AudioCommand, Channels, Engine, SampleRate};
use common::{SyntheticDecoder, Waveform};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

struct CountingAlloc;
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        System.realloc(p, l, n)
    }
}

#[global_allocator]
static A: CountingAlloc = CountingAlloc;

#[test]
fn full_dsp_callback_zero_allocation() {
    let (mut engine, mut poster) = full_dsp_engine();
    let _ = engine.load_track(sine_track(440.0, RATE64 * 10));
    engine.reserve_capture(4096);
    poster.post(AudioCommand::Play).unwrap();
    poster
        .post(AudioCommand::SetVolume { volume: 0.8 })
        .unwrap();
    // 预热（一次性分配：捕获/解码/映射/pending 等）
    engine.render_for_frames(4096);
    engine.clear_capture();
    // 灌满解码缓冲（fill 属解码面，允许分配，D-026；度量窗口外）
    engine.fill();
    poster
        .post(AudioCommand::SetVolume { volume: 0.6 })
        .unwrap();
    engine.set_eq_preset(&aura_core::dsp::EQ_PRESETS[1]);
    // 同形状预热块（触发 stage 内一次性分配，如 EQ 系数首次加载）
    // block 缓冲在 warm 计数**之前**分配（度量窗口内不允许任何分配）
    let mut block = vec![0.0f32; 1024 * 2];
    let _ = engine.render_block_only(&mut block);
    let warm = ALLOC_COUNT.load(Ordering::Relaxed);

    // 度量窗口：**纯回调路径**（stage.render：pop → 净化 → volume 平滑 →
    // loudness×preamp → EQ → limiter），全 DSP 开启（§61）
    let rendered = engine.render_block_only(&mut block);

    let after = ALLOC_COUNT.load(Ordering::Relaxed);
    assert_eq!(rendered, 1024);
    assert_eq!(
        after.abs_diff(warm),
        0,
        "全 DSP 开启时回调渲染路径出现额外堆分配（必须为 0，§61）"
    );
}

// ===== 长 Soak（§64–65；CI 版为 60 秒等效，1h 等效版 #[ignore]）=====
