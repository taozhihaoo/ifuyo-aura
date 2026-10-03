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
fn probe7() {
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
    // 完全相同的渲染调用两次
    let w1 = ALLOC_COUNT.load(Ordering::Relaxed);
    engine.render_for_frames(1024);
    println!("PK7 r1={}", ALLOC_COUNT.load(Ordering::Relaxed) - w1);
    let w2 = ALLOC_COUNT.load(Ordering::Relaxed);
    engine.render_for_frames(1024);
    println!("PK7 r2={}", ALLOC_COUNT.load(Ordering::Relaxed) - w2);
    // fill 在缓冲满态时是否分配？
    let w3 = ALLOC_COUNT.load(Ordering::Relaxed);
    engine.fill();
    engine.render_for_frames(1024);
    println!(
        "PK7 render_after_fill={}",
        ALLOC_COUNT.load(Ordering::Relaxed) - w3
    );
    let w4 = ALLOC_COUNT.load(Ordering::Relaxed);
    engine.fill();
    engine.render_for_frames(1024);
    println!(
        "PK7 render_after_fill2={}",
        ALLOC_COUNT.load(Ordering::Relaxed) - w4
    );
}
