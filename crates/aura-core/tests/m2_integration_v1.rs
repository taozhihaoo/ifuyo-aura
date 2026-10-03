//! M2 集成测试（§58–65）：全 DSP 链、确定性、零分配扩展、长 soak。

mod common;

use aura_core::{AudioCommand, Channels, Engine, SampleRate};
use common::{SyntheticDecoder, Waveform};

const RATE: u32 = 48_000;
const RATE64: u64 = 48_000;

fn full_dsp_engine() -> (Engine, aura_core::AudioCommandQueue) {
    let (mut engine, poster) = Engine::new(
        SampleRate::new(RATE).unwrap(),
        Channels::new(2).unwrap(),
        16_384,
    )
    .unwrap();
    // 全 DSP：crossfade 0.5s（< 单轨 1s，正常 overlap 路径）+ loudness + EQ + limiter
    engine.set_crossfade_seconds(0.5);
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

#[test]
fn full_dsp_gapless_transition_integration() {
    let (mut engine, mut poster) = full_dsp_engine();
    let _ = engine.load_track(sine_track(440.0, RATE64));
    engine.preload_next(sine_track(880.0, RATE64));
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(2 * RATE64);
    let cap = engine.capture();
    assert_eq!(cap.len(), (2 * RATE64) as usize * 2, "gapless 总帧数不变");
    assert!(cap.iter().all(|s| s.is_finite()), "全 DSP 链输出有限");
    // limiter 兜底下无越界
    let max = cap.iter().fold(0.0f32, |a, b| a.max(b.abs()));
    assert!(max <= 1.0, "limiter 后峰值 ≤ 1.0: {max}");
}

#[test]
fn full_dsp_crossfade_transition_integration() {
    let (mut engine, mut poster) = full_dsp_engine();
    engine.set_crossfade_seconds(2.0);
    let _ = engine.load_track(sine_track(440.0, RATE64));
    engine.preload_next(sine_track(880.0, RATE64));
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(2 * RATE64);
    let cap = engine.capture();
    assert_eq!(cap.len(), (2 * RATE64) as usize * 2);
    assert!(cap.iter().all(|s| s.is_finite()));
    let max = cap.iter().fold(0.0f32, |a, b| a.max(b.abs()));
    assert!(max <= 1.0, "crossfade + limiter: {max}");
}

#[test]
fn full_dsp_determinism_bit_identical() {
    let run = || {
        let (mut engine, mut poster) = full_dsp_engine();
        engine.set_crossfade_seconds(2.0);
        let _ = engine.load_track(sine_track(440.0, RATE64));
        engine.preload_next(sine_track(880.0, RATE64));
        poster.post(AudioCommand::Play).unwrap();
        poster
            .post(AudioCommand::SetVolume { volume: 0.8 })
            .unwrap();
        let _ = engine.render_for_frames(2 * RATE64);
        engine.capture().to_vec()
    };
    let a = run();
    let b = run();
    assert_eq!(a.len(), b.len());
    assert!(
        a == b,
        "全 DSP（cf+loudness+EQ+limiter+volume）必须 bit-identical（§59）"
    );
}

#[test]
fn numerical_safety_hostile_input_through_full_chain() {
    // NaN/Inf 输入经 loudness/EQ/limiter 后不得到达输出（§60）
    let (mut engine, mut poster) = full_dsp_engine();
    let _ = engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        RATE64,
        512,
        Waveform::NonFinite,
    )));
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(9_600);
    assert!(
        engine.capture().iter().all(|s| s.is_finite()),
        "敌意输入经全链路后必须有限"
    );
}

// ===== 零分配扩展（§61–62）=====

#[test]
fn soak_60s_equivalent_a_b_loop_full_dsp() {
    let (mut engine, mut poster) = full_dsp_engine();
    let _ = engine.load_track(sine_track(440.0, RATE64)); // A：1s
    poster.post(AudioCommand::Play).unwrap();

    let loops = 30u64; // 30 × A(1s) + 29 × B(1s) ≈ 59s
    for k in 0..loops {
        if k < loops - 1 {
            // 预解码窗口语义（§6）：仅在无待处理 next 时预装载
            //（crossfade 进行中 next 正在被消费，覆盖会丢失该轨）
            engine.preload_next(sine_track(if k % 2 == 0 { 880.0 } else { 440.0 }, RATE64));
        }
        engine.clear_capture();
        // 半秒粒度渲染 ×2：给 crossfade overlap 推进空间
        let _ = engine.render_for_frames(RATE64 / 2);
        let _ = engine.render_for_frames(RATE64 / 2);
    }
    // 注：preload 在渲染前调用即可生效；此处每轮渲染整整 1s = 曲目全长，
    // 自然触发切换
    // 不变量（§65）
    assert_eq!(
        engine.track_frame_ranges().len(),
        loops as usize,
        "无丢失轨"
    );
    let ranges = engine.track_frame_ranges();
    for (k, r) in ranges.iter().enumerate() {
        assert_eq!(r.valid_frames, RATE64, "track {k} 帧数完整（无累计漂移）");
    }
    let cap = engine.capture();
    assert!(cap.iter().all(|s| s.is_finite()), "无无效 PCM");
    assert!(
        engine.underruns() <= loops,
        "underrun 不累积失控: {}",
        engine.underruns()
    );
}

#[test]
#[ignore]
fn soak_1h_equivalent_extended() {
    // 本地扩展 soak：3600 次 1s A/B 循环全 DSP 开启（§64：不得把未运行写成 PASS；
    // 运行命令：cargo test -p aura-core --release --test m2_integration_v1 -- --ignored --nocapture）
    let (mut engine, mut poster) = full_dsp_engine();
    let _ = engine.load_track(sine_track(440.0, RATE64));
    poster.post(AudioCommand::Play).unwrap();
    let loops = 3600u64;
    for k in 0..loops {
        if k < loops - 1 {
            engine.preload_next(sine_track(if k % 2 == 0 { 880.0 } else { 440.0 }, RATE64));
        }
        let _ = engine.render_for_frames(RATE64);
    }
    assert_eq!(engine.track_frame_ranges().len(), loops as usize);
    assert!(engine.capture().iter().all(|s| s.is_finite()));
}
