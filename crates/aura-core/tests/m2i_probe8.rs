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
fn probe9() {
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
    let count = || ALLOC_COUNT.load(Ordering::Relaxed);
    for k in 1..=3 {
        engine.debug_render_stepwise(1024, &count, &format!("r{k}"));
    }
}
