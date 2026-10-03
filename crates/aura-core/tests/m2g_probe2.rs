mod common;
use aura_core::{AudioCommand, Channels, Engine, SampleRate};
use common::{SyntheticDecoder, Waveform};

#[test]
fn probe_gapless2() {
    let rate = 48_000u32;
    let n = 24_000u64;
    let (mut engine, mut poster) = Engine::new(
        SampleRate::new(rate).unwrap(),
        Channels::new(2).unwrap(),
        8192,
    )
    .unwrap();
    engine
        .load_track(Box::new(SyntheticDecoder::new(
            rate,
            2,
            n,
            512,
            Waveform::RampFrom(0),
        )))
        .unwrap();
    engine.preload_next(Box::new(SyntheticDecoder::new(
        rate,
        2,
        n,
        512,
        Waveform::RampFrom(n),
    )));
    poster.post(AudioCommand::Play).unwrap();
    engine.render_for_frames(2 * n);
    let cap = engine.capture();
    let frames: Vec<f32> = cap.chunks_exact(2).map(|c| c[0]).collect();
    println!(
        "P2 underruns={} boundaries={:?}",
        engine.underruns(),
        engine.track_boundaries()
    );
    // 找第一个 B 值（≥ 24000）
    let first_b = frames.iter().position(|v| *v >= 24000.0);
    println!("P2 first_b_frame={first_b:?}");
    // 打印过渡区 23990..24110
    println!("P2 around: {:?}", &frames[23990..24010]);
    // 全部不连续点
    let mut br = Vec::new();
    for (i, w) in frames.windows(2).enumerate() {
        if (w[1] - w[0] - 1.0).abs() > 1e-3 {
            br.push((i, w[0], w[1]));
        }
    }
    println!("P2 breaks: {:?}", &br[..br.len().min(8)]);
    println!("P2 ranges: {:?}", engine.track_frame_ranges());
}
