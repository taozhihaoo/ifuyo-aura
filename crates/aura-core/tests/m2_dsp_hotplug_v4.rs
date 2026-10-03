//! M2 Loudness + EQ + Hotplug 测试（§55–57）。

mod common;

use aura_core::{AudioCommand, Channels, Engine, SampleRate, TrackLoudness};
use common::{SyntheticDecoder, Waveform};

const RATE: u32 = 48_000;
const RATE64: u64 = 48_000;

fn engine_48k() -> (Engine, aura_core::AudioCommandQueue) {
    Engine::new(
        SampleRate::new(RATE).unwrap(),
        Channels::new(2).unwrap(),
        8192,
    )
    .unwrap()
}

fn load_dc_track(engine: &mut Engine, level: f32) {
    let _ = engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        RATE64,
        512,
        Waveform::Dc(level),
    )));
}

// ===== Loudness（§55）=====

#[test]
fn loudness_disabled_is_unity() {
    let (mut engine, mut poster) = engine_48k();
    engine.set_limiter_enabled(false);
    engine.set_track_loudness(
        TrackLoudness {
            integrated_lufs: -30.0,
            true_peak_db: -10.0,
            loudness_available: true,
        },
        false, // enabled = false（§23）
    );
    load_dc_track(&mut engine, 0.25);
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(4_800);
    let cap = engine.capture();
    assert!(
        (cap[0] - 0.25).abs() < 1e-6,
        "loudness off 保持原始增益: {}",
        cap[0]
    );
}

#[test]
fn loudness_minus16_unity_quieter_boosts_louder_attenuates() {
    // -16 LUFS → 0dB；-20 → +4dB；-12 → -4dB（fixture 1–3，无峰值约束）
    for (lufs, expect_db) in [(-16.0, 0.0), (-20.0, 4.0), (-12.0, -4.0)] {
        let (mut engine, mut poster) = engine_48k();
        engine.set_limiter_enabled(false);
        engine.set_track_loudness(
            TrackLoudness {
                integrated_lufs: lufs,
                true_peak_db: -60.0,
                loudness_available: true,
            },
            true,
        );
        load_dc_track(&mut engine, 0.25);
        poster.post(AudioCommand::Play).unwrap();
        let _ = engine.render_for_frames(4_800);
        let got = engine.capture()[0];
        let expect = 0.25 * 10.0f32.powf(expect_db as f32 / 20.0);
        assert!(
            (got - expect).abs() < 1e-4,
            "{lufs} LUFS: got {got} expect {expect}"
        );
    }
}

#[test]
fn loudness_peak_safety_caps_gain() {
    // -24 LUFS 但 true peak -0.1dBTP：增益被钳制
    let (mut engine, mut poster) = engine_48k();
    engine.set_track_loudness(
        TrackLoudness {
            integrated_lufs: -24.0,
            true_peak_db: -0.1,
            loudness_available: true,
        },
        true,
    );
    load_dc_track(&mut engine, 0.25);
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(4_800);
    let got = engine.capture()[0];
    assert!(got.is_finite() && got.abs() <= 0.9, "峰值安全: got {got}");
}

#[test]
fn loudness_missing_metadata_is_unity() {
    let (mut engine, mut poster) = engine_48k();
    engine.set_limiter_enabled(false);
    // 默认 missing（§25：不猜测、gain=1.0）
    load_dc_track(&mut engine, 0.25);
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(4_800);
    assert!((engine.capture()[0] - 0.25).abs() < 1e-6);
}

// ===== EQ（§57）=====

#[test]
fn eq_flat_passthrough_and_preset_effect() {
    let (mut engine, mut poster) = engine_48k();
    engine.set_limiter_enabled(false);
    load_dc_track(&mut engine, 0.25);
    engine.set_eq_preset(&aura_core::dsp::EQ_PRESETS[0]);
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(9_600);
    let flat_out = engine.capture()[9_000];
    assert!((flat_out - 0.25).abs() < 1e-3, "Flat DC 直通: {flat_out}");

    // Bass Boost（31Hz +6dB）：以 31Hz 正弦验证低频增益生效
    //（DC 频谱在 0Hz，peak band 对其增益为 0——物理正确，不能用 DC 验证）
    engine.clear_capture();
    let _ = engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        RATE64,
        512,
        Waveform::Sine {
            freq_hz: 31.0,
            amplitude: 0.25,
        },
    )));
    // load_track 后状态回到 Stopped：显式恢复播放（否则输出静音）
    poster.post(AudioCommand::Play).unwrap();
    engine.set_eq_preset(&aura_core::dsp::EQ_PRESETS[1]);
    let _ = engine.render_for_frames(28_800);
    let boosted_max = engine
        .capture()
        .chunks_exact(2)
        .map(|c| c[0])
        .fold(0.0f32, |a, b| a.max(b.abs()));
    assert!(boosted_max.is_finite());
    assert!(boosted_max > 0.25, "31Hz 应被 +6dB 提升: {boosted_max}");
}

#[test]
fn eq_extreme_gains_stable_no_explosion() {
    let (mut engine, mut poster) = engine_48k();
    // 极端 +12dB 全频段（§40 stability）；limiter 兜底保持开启
    let mut bands = aura_core::dsp::bands_from_preset(&aura_core::dsp::EQ_PRESETS[0]);
    for b in &mut bands {
        b.gain_db = 12.0;
    }
    engine.set_eq_bands(&bands);
    let _ = engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        RATE64,
        512,
        Waveform::Sine {
            freq_hz: 440.0,
            amplitude: 0.9,
        },
    )));
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(RATE64);
    let cap = engine.capture();
    assert!(cap.iter().all(|s| s.is_finite()), "极端增益下不得 NaN/Inf");
    let max = cap.iter().fold(0.0f32, |a, b| a.max(b.abs()));
    assert!(max <= 1.001, "limiter 兜底: max {max}");
}

#[test]
fn eq_silence_and_impulse() {
    let (mut engine, mut poster) = engine_48k();
    engine.set_limiter_enabled(false);
    engine.set_eq_preset(&aura_core::dsp::EQ_PRESETS[2]); // Treble Boost
                                                          // 静音输入：输出静音
    load_dc_track(&mut engine, 0.0);
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(9_600);
    assert!(
        engine.capture().iter().all(|s| s.abs() < 1e-9),
        "静音进静音出"
    );

    // impulse：有限输出
    engine.clear_capture();
    let _ = engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        RATE64,
        512,
        Waveform::Impulse,
    )));
    let _ = engine.render_for_frames(2_400);
    assert!(engine.capture().iter().all(|s| s.is_finite()));
}

// ===== Hotplug（§56，NullOutput 模拟）=====

#[test]
fn hotplug_remove_restores_position_no_autoplay() {
    let (mut engine, mut poster) = engine_48k();
    let _ = engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        RATE64 * 4,
        512,
        Waveform::RampFrom(0),
    )));
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(9_600);
    let pos_before = engine.position_frames();
    assert_eq!(pos_before, 9_600);

    // 设备断开：安全暂停、位置保留、不 panic（§27）
    engine.device_removed();
    assert_eq!(
        engine.device_state(),
        aura_core::output::device_state::OutputDeviceState::Removed
    );
    assert_eq!(engine.state(), aura_core::PlaybackState::Paused);
    let _ = engine.render_for_frames(9_600);
    assert_eq!(engine.position_frames(), pos_before, "暂停期间位置不增长");

    // 设备恢复（不同采样率）：可恢复状态、不自动播放（§28/§31）
    engine.device_restored(SampleRate::new(44_100).unwrap(), Channels::new(2).unwrap());
    assert_eq!(
        engine.device_state(),
        aura_core::output::device_state::OutputDeviceState::Connected
    );
    assert_eq!(
        engine.state(),
        aura_core::PlaybackState::Paused,
        "恢复后不得自动播放"
    );
    assert_eq!(engine.position_frames(), pos_before, "位置跨重配置保留");

    // 用户显式恢复播放
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(2_400);
    assert_eq!(engine.position_frames(), pos_before + 2_400);
}

#[test]
fn hotplug_default_device_change_and_error_no_panic() {
    let (mut engine, mut poster) = engine_48k();
    let _ = engine.load_track(Box::new(SyntheticDecoder::new(
        RATE,
        2,
        RATE64 * 4,
        512,
        Waveform::RampFrom(0),
    )));
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(4_800);

    // 默认设备变化 → 同恢复路径（§29）
    engine.default_device_changed(SampleRate::new(96_000).unwrap(), Channels::new(2).unwrap());
    assert_eq!(
        engine.device_state(),
        aura_core::output::device_state::OutputDeviceState::Connected
    );

    // 流错误上报：仅计数，不 panic、不误触发 next track（§48）
    for _ in 0..5 {
        engine.report_device_error();
    }
    assert_eq!(engine.device_errors(), 5);
    let _ = engine.render_for_frames(2_400);
    assert!(
        engine.track_frame_ranges().len() <= 1,
        "设备错误不得触发曲目切换"
    );
}
