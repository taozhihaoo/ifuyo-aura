mod common;
use aura_core::{AudioCommand, Channels, Engine, SampleRate};
use common::{SyntheticDecoder, Waveform};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

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
fn probe8() {
    let (mut engine, mut poster) = Engine::new(
        SampleRate::new(48_000).unwrap(),
        Channels::new(2).unwrap(),
        16_384,
    )
    .unwrap();
    engine.load_track(Box::new(SyntheticDecoder::new(
        48_000,
        2,
        48_000 * 10,
        512,
        Waveform::Sine {
            freq_hz: 440.0,
            amplitude: 0.6,
        },
    )));
    poster.post(AudioCommand::Play).unwrap();
    engine.render_for_frames(4096);
    engine.fill();
    // r1 之后 r2 的 2 次分配：跟踪 track_boundaries push？
    // RampFrom 改 sine 后 fill 每次都推满 → free=0 → break；没有 Vec grow。
    // 二分：多次连续 render，看分配是否线性增长
    for k in 1..=6 {
        let w = ALLOC_COUNT.load(Ordering::Relaxed);
        engine.render_for_frames(1024);
        println!("PK8 r{k}={}", ALLOC_COUNT.load(Ordering::Relaxed) - w);
    }
}
