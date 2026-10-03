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
fn probe11() {
    let (mut engine, mut poster) = Engine::new(
        SampleRate::new(48_000).unwrap(),
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
    engine.reserve_capture(4096);
    poster.post(AudioCommand::Play).unwrap();
    poster
        .post(AudioCommand::SetVolume { volume: 0.8 })
        .unwrap();
    engine.render_for_frames(4096);
    engine.clear_capture();
    engine.fill();
    poster
        .post(AudioCommand::SetVolume { volume: 0.6 })
        .unwrap();
    engine.set_eq_preset(&aura_core::dsp::EQ_PRESETS[1]);
    let mut block = vec![0.0f32; 1024 * 2];
    engine.render_block_only(&mut block);
    let count = || ALLOC_COUNT.load(Ordering::Relaxed);
    for k in 1..=3 {
        let w = count();
        engine.render_block_only(&mut block);
        println!("PK11 b{k}={}", count() - w);
    }
}
