//! 性能基线（M1 §44）：手动运行
//! `cargo test -p aura-core --release --test perf_baseline -- --ignored --nocapture`
//! 结果记录于 docs/PERF.md。常规门禁跳过本文件（--ignored）。

use aura_core::{Channels, Decoder, Engine, Resampler, SampleRate};
use std::time::Instant;

mod common;
use common::{SyntheticDecoder, Waveform};

#[test]
#[ignore]
fn perf_baselines() {
    println!("=== M1 performance baseline ===");

    // 1) ring buffer throughput
    {
        let (mut p, mut c) = aura_core::ring_buffer(Channels::new(2).unwrap(), 8192 * 2).unwrap();
        let block = vec![0.25f32; 1024 * 2];
        let mut pushed = 0u64;
        let t = Instant::now();
        let iters = 200_000usize; // 200k × 1024 帧
        for _ in 0..iters {
            // 先灌后取，保持满循环
            pushed += p.push_frames(&block);
            let mut out = vec![0.0f32; 1024 * 2];
            let _ = c.pop_frames(&mut out);
        }
        let el = t.elapsed();
        println!(
            "ring_buffer: pushed {pushed} frames in {el:?} => {:.0} frames/s",
            pushed as f64 / el.as_secs_f64()
        );
    }

    // 2) FLAC decode throughput（真实解码路径）
    {
        let pcm: Vec<i16> = vec![0; 48_000]; // 夹具为 440Hz 正弦 1 秒（内容不影响解码吞吐）
        let flac = include_bytes!("fixtures/flac_mono_48k_440hz_1s.flac").to_vec();
        let _ = &pcm;
        let runs = 20usize;
        let t = Instant::now();
        let mut total = 0u64;
        for _ in 0..runs {
            let mut dec = aura_core::FlacDecoder::from_bytes(flac.clone()).unwrap();
            let mut out = aura_core::PcmBuffer::new();
            loop {
                let n = dec.decode_next(&mut out).unwrap();
                if n == 0 {
                    break;
                }
                total += n;
            }
        }
        let el = t.elapsed();
        let seconds_decoded = total as f64 / 48_000.0;
        println!(
            "flac_decode: {runs}×1s audio in {el:?} => {:.1}× real-time",
            seconds_decoded / el.as_secs_f64()
        );
    }

    // 3) resample throughput (44100 → 48000, stereo)
    {
        let mut r = Resampler::new(
            SampleRate::new(44_100).unwrap(),
            SampleRate::new(48_000).unwrap(),
            Channels::new(2).unwrap(),
        )
        .unwrap()
        .unwrap();
        let input = vec![0.1f32; r.input_frames_next() * 2];
        let seconds_of_input = 60.0; // 等效 60 秒输入音频
        let in_frames_total = (44_100.0 * seconds_of_input) as usize;
        let t = Instant::now();
        let mut done = 0usize;
        while done < in_frames_total {
            let out = r.process_chunk(&input).unwrap();
            let _ = out;
            done += r.input_frames_next();
        }
        let el = t.elapsed();
        println!(
            "resample 44.1k→48k: {done} input frames in {el:?} => {:.1}× real-time",
            done as f64 / 44_100.0 / el.as_secs_f64()
        );
    }

    // 4) NullOutput render throughput + 5) per-chunk callback time
    {
        let (mut engine, mut poster) = Engine::new(
            SampleRate::new(48_000).unwrap(),
            Channels::new(2).unwrap(),
            8192,
        )
        .unwrap();
        engine
            .load_track(Box::new(SyntheticDecoder::new(
                48_000,
                2,
                48_000 * 600,
                1024,
                Waveform::Ramp,
            )))
            .unwrap();
        poster.post(aura_core::AudioCommand::Play).unwrap();
        // 预热
        engine.render_for_frames(8192);
        let frames_total = 48_000u64 * 60; // 等效 60 秒
        let t = Instant::now();
        engine.render_for_frames(frames_total);
        let el = t.elapsed();
        println!(
            "render: {frames_total} frames in {el:?} => {:.1}× real-time",
            frames_total as f64 / el.as_secs_f64()
        );
        println!(
            "callback avg per 1024-frame chunk: {:?}",
            el / (frames_total / 1024) as u32
        );
    }
}
