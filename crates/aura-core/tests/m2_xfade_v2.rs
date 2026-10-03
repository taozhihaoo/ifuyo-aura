//! M2 Crossfade 测试（§54）。

mod common;

use aura_core::{AudioCommand, Channels, Engine, SampleRate};
use common::{SyntheticDecoder, Waveform};

const RATE: u32 = 48_000;

fn crossfade_engine(seconds: f64) -> (Engine, aura_core::AudioCommandQueue) {
    let (mut engine, poster) = Engine::new(
        SampleRate::new(RATE).unwrap(),
        Channels::new(2).unwrap(),
        16_384,
    )
    .unwrap();
    engine.set_crossfade_seconds(seconds);
    let rate = u64::from(RATE);
    engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        rate,
        512,
        Waveform::Sine {
            freq_hz: 440.0,
            amplitude: 0.8,
        },
    )));
    engine.preload_next(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        rate,
        512,
        Waveform::Sine {
            freq_hz: 880.0,
            amplitude: 0.8,
        },
    )));
    engine.reserve_capture((2 * rate * 2) as usize);
    (engine, poster)
}

fn render_two_tracks(engine: &mut Engine, poster: &mut aura_core::AudioCommandQueue) {
    poster.post(AudioCommand::Play).unwrap();
    engine.render_for_frames(2 * u64::from(RATE));
}

#[test]
fn crossfade_zero_equals_plain_transition() {
    let (mut engine, mut poster) = crossfade_engine(0.0);
    assert_eq!(engine.crossfade_frames(), 0, "0s = 关闭");
    render_two_tracks(&mut engine, &mut poster);
    assert_eq!(engine.capture().len(), 2 * 48_000 * 2);
}

#[test]
fn crossfade_one_and_six_and_twelve_seconds_overlap() {
    for seconds in [1.0f64, 6.0, 12.0] {
        let (mut engine, mut poster) = crossfade_engine(seconds);
        render_two_tracks(&mut engine, &mut poster);
        let cap = engine.capture();
        assert_eq!(cap.len(), 2 * 48_000 * 2);
        assert!(cap.iter().all(|s| s.is_finite()), "{seconds}s: 有限");
        assert!(cap.iter().any(|s| s.abs() > 0.05), "{seconds}s: 有信号");
    }
}

#[test]
fn crossfade_longer_than_track_downgrades_without_panic() {
    let (mut engine, mut poster) = crossfade_engine(6.0);
    render_two_tracks(&mut engine, &mut poster);
    let cap = engine.capture();
    assert_eq!(cap.len(), 2 * 48_000 * 2);
    assert!(cap.iter().all(|s| s.is_finite()));
}

#[test]
fn crossfade_gain_monotonic_no_nan() {
    let rate = 48_000u64;
    let (mut engine, mut poster) = Engine::new(
        SampleRate::new(RATE).unwrap(),
        Channels::new(2).unwrap(),
        16_384,
    )
    .unwrap();
    engine.set_crossfade_seconds(6.0);
    engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        rate,
        512,
        Waveform::Dc(0.5),
    )));
    engine.preload_next(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        rate,
        512,
        Waveform::Dc(-0.5),
    )));
    poster.post(AudioCommand::Play).unwrap();
    engine.render_for_frames(rate / 2);
    engine.render_for_frames(rate);
    let cap = engine.capture();
    assert!(cap.iter().all(|s| s.is_finite()), "无 NaN/Inf");

    // A=+0.5 DC：采样 = 0.5·ga − 0.5·gb；ga 单调降 → 值单调从 +0.5 向 −0.5 过渡
    let start = (rate / 2) as usize;
    let mid: Vec<f32> = cap[start..start + rate as usize / 2]
        .chunks_exact(2)
        .map(|c| c[0])
        .collect();
    let mut prev = f32::MAX;
    for (i, &v) in mid.iter().enumerate() {
        assert!(v <= prev + 1e-4, "A 增益非单调 @{}: {v} > {prev}", i);
        prev = v;
    }
}

#[test]
fn crossfade_deterministic() {
    let run = || {
        let (mut engine, mut poster) = crossfade_engine(6.0);
        poster.post(AudioCommand::Play).unwrap();
        engine.render_for_frames(2 * 48_000);
        engine.capture().to_vec()
    };
    assert_eq!(run(), run(), "同输入必须产出相同 PCM（§17）");
}
