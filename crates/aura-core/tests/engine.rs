//! M1 集成测试（§40）：synthetic pipeline、play/pause/stop、seek/generation、
//! underrun、音量平滑、命令队列、重采样管线、PCM 安全。

mod common;

use aura_core::types::AudioCommand;
use aura_core::{Channels, Engine, PlaybackState, SampleRate, Volume};
use common::{SyntheticDecoder, Waveform};

const RATE: u32 = 48_000;
const CHUNK: usize = 128; // 渲染块（帧）——刻意小于 fill 块以覆盖缓冲边界

fn engine_with(waveform: Waveform, total_frames: u64) -> (Engine, aura_core::AudioCommandQueue) {
    let (mut engine, poster) = Engine::new(
        SampleRate::new(RATE).unwrap(),
        Channels::new(2).unwrap(),
        8192,
    )
    .unwrap();
    engine
        .load_track(Box::new(SyntheticDecoder::new(
            RATE,
            2,
            total_frames,
            512,
            waveform,
        )))
        .unwrap();
    engine.reserve_capture(48_000 * 2);
    (engine, poster)
}

#[test]
fn deterministic_pipeline_identical_output() {
    let run = || {
        let (mut engine, mut poster) = engine_with(
            Waveform::Sine {
                freq_hz: 440.0,
                amplitude: 0.8,
            },
            u64::from(RATE) * 2,
        );
        poster.post(AudioCommand::Play).unwrap();
        poster
            .post(AudioCommand::SetVolume { volume: 0.8 })
            .unwrap();
        let _ = engine.render_for_frames(CHUNK as u64 * 40);
        engine.capture().to_vec()
    };
    let a = run();
    let b = run();
    assert_eq!(a.len(), CHUNK * 40 * 2);
    assert_eq!(a, b, "相同输入+命令序列必须产出相同 PCM（§26）");
    // 播放确实有非静音内容
    assert!(a.iter().any(|s| s.abs() > 0.1));
}

#[test]
fn stopped_renders_silence_and_position_stays() {
    let (mut engine, _poster) = engine_with(Waveform::Ramp, u64::from(RATE) * 2);
    let _ = engine.render_for_frames(CHUNK as u64 * 4);
    assert!(
        engine.capture().iter().all(|s| *s == 0.0),
        "Stopped 必须输出静音"
    );
    assert_eq!(engine.position_frames(), 0);
    assert_eq!(engine.state(), PlaybackState::Stopped);
}

#[test]
fn play_pause_stop_state_machine() {
    let (mut engine, mut poster) = engine_with(
        Waveform::Sine {
            freq_hz: 440.0,
            amplitude: 0.8,
        },
        u64::from(RATE) * 4,
    );

    // Play：位置前进、内容非静音
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(CHUNK as u64 * 4);
    assert_eq!(engine.state(), PlaybackState::Playing);
    assert_eq!(engine.position_frames(), (CHUNK * 4) as u64);
    assert!(engine.capture().iter().any(|s| s.abs() > 0.1));

    // Pause：位置冻结、静音（先清掉 Play 阶段的捕获）
    poster.post(AudioCommand::Pause).unwrap();
    engine.clear_capture();
    let _ = engine.render_for_frames(CHUNK as u64 * 4);
    assert_eq!(engine.state(), PlaybackState::Paused);
    assert_eq!(
        engine.position_frames(),
        (CHUNK * 4) as u64,
        "暂停期间位置不增长"
    );
    assert!(engine.capture().iter().all(|s| *s == 0.0), "暂停必须静音");

    // Resume：从暂停处继续
    poster.post(AudioCommand::Play).unwrap();
    engine.clear_capture();
    let _ = engine.render_for_frames(CHUNK as u64 * 4);
    assert_eq!(engine.position_frames(), (CHUNK * 8) as u64);

    // Stop：位置归零（先清掉 Resume 阶段的捕获）
    poster.post(AudioCommand::Stop).unwrap();
    engine.clear_capture();
    let _ = engine.render_for_frames(CHUNK as u64 * 2);
    assert_eq!(engine.state(), PlaybackState::Stopped);
    assert_eq!(engine.position_frames(), 0, "Stop 后位置归零（D-023）");
    assert!(engine.capture().iter().all(|s| *s == 0.0));
}

#[test]
fn seek_provenance_no_stale_generation() {
    // Fixture D ramp：sample[n] = n → 输出值直接暴露其来源帧（provenance 自证）
    let total = RATE as u64 * 3;
    let (mut engine, mut poster) = engine_with(Waveform::Ramp, total);
    // provenance 用大值 ramp 精确溯源；limiter 会压缩 >-1dBFS 的值 → 关闭（D-034）
    engine.set_limiter_enabled(false);

    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(1_000);
    // gen1：ramp 0..999
    assert_eq!(
        &engine.capture()[..CHUNK],
        &{
            let mut expect = Vec::new();
            for n in 0..CHUNK {
                expect.push(n as f32);
                expect.push(n as f32);
            }
            expect
        }[..CHUNK]
    );

    // seek 到 20000：新 generation；旧数据永不输出
    poster.post(AudioCommand::Seek { frame: 20_000 }).unwrap();
    engine.clear_capture();
    let _ = engine.render_for_frames(3_000);
    let cap = engine.capture();
    assert_eq!(cap.len(), 3_000 * 2);
    // 不变量（§21–22 核心要求）：
    //  1) 旧 generation 数据（值域 (0, 20000) 的 ramp 样本）永远不得出现；
    //  2) 非零样本全部来自新 generation 的 [20000, 23000)；
    //  3) 0 值只允许来自 seek 接缝处的 underrun 静音（缓冲中被丢弃的旧代数据
    //     形成短暂空洞，是 D-019 记录的预期行为）。
    let mut first_nonzero: Option<f32> = None;
    let mut new_gen_samples = 0usize;
    for pair in cap.chunks_exact(2) {
        assert_eq!(pair[0], pair[1], "stereo 通道一致");
        let v = pair[0];
        if v == 0.0 {
            continue; // underrun 静音
        }
        assert!(
            (20_000.0..23_000.0).contains(&v),
            "stale generation 泄漏：样本 {v} 不在新代值域"
        );
        if first_nonzero.is_none() {
            first_nonzero = Some(v);
        }
        new_gen_samples += 1;
    }
    assert!(new_gen_samples > 0, "seek 后必须有新代数据输出");
    assert!(
        first_nonzero.unwrap() >= 20_000.0,
        "第一个非零样本必须来自新代起点"
    );
    // 权威位置 = seek 目标 + 渲染帧数
    assert_eq!(engine.position_frames(), 23_000);
}

#[test]
fn underrun_outputs_silence_and_counts() {
    // 480 帧的小音轨：耗尽后必然 underrun
    let (mut engine, mut poster) = engine_with(Waveform::Ramp, 480);
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(4_800);
    assert!(engine.underruns() >= 1, "耗尽后必须计 underrun");
    // ramp 只覆盖前 480 帧（数据耗尽处起为静音）
    let cap = engine.capture();
    assert!(cap[..480 * 2].iter().all(|s| s.is_finite()));
    assert!(
        cap[480 * 2..].iter().all(|s| *s == 0.0),
        "underrun 必须输出静音"
    );
    // 管线可恢复：位置照常推进，无 panic
    assert_eq!(engine.position_frames(), 4_800);
}

#[test]
fn volume_smoothing_no_jump_and_repeatable() {
    let run = |poster_fn: &dyn Fn(&mut aura_core::AudioCommandQueue)| {
        let (mut engine, mut poster) = engine_with(Waveform::Dc(1.0), u64::from(RATE) * 4);
        poster_fn(&mut poster);
        poster.post(AudioCommand::Play).unwrap();
        let _ = engine.render_for_frames(CHUNK as u64 * 12);
        engine.capture().to_vec()
    };

    // SetVolume 0：DC 输入下相邻采样跳变受 10ms 窗口约束
    let cap = run(&|p| {
        p.post(AudioCommand::SetVolume { volume: 1.0 }).unwrap();
        p.post(AudioCommand::SetVolume { volume: 0.0 }).unwrap();
    });
    // 前 4 chunk 全量（目标 1.0），随后平滑下落
    for w in cap.windows(2) {
        let d = (w[0] - w[1]).abs();
        assert!(d <= 1.0 / 480.0 + 1e-4, "爆音：相邻采样跳变 {d}");
    }
    assert_eq!(*cap.last().unwrap(), 0.0, "平滑终点精确静音");

    // 可重复
    let cap2 = run(&|p| {
        p.post(AudioCommand::SetVolume { volume: 1.0 }).unwrap();
        p.post(AudioCommand::SetVolume { volume: 0.0 }).unwrap();
    });
    assert_eq!(cap, cap2, "音量过渡必须可重复");
}

#[test]
fn invalid_commands_are_counted_not_panicking() {
    let (mut engine, mut poster) = engine_with(Waveform::Ramp, u64::from(RATE));
    poster
        .post(AudioCommand::SetVolume { volume: f32::NAN })
        .unwrap();
    poster
        .post(AudioCommand::SetVolume { volume: -1.0 })
        .unwrap();
    poster
        .post(AudioCommand::SetVolume {
            volume: f32::INFINITY,
        })
        .unwrap(); // 非有限 → 非法
    poster.post(AudioCommand::Play).unwrap();
    poster.post(AudioCommand::Play).unwrap(); // 重复 Play → 安全 no-op
    let _ = engine.render_for_frames(CHUNK as u64);
    assert!(engine.invalid_commands() >= 3, "非法命令必须被计数"); /* NaN + -1.0 + Inf */
    assert_eq!(engine.invalid_commands(), 3);
    assert_eq!(engine.state(), PlaybackState::Playing);
    assert_eq!(engine.volume(), Volume::MAX, "非法音量不得生效");
}

#[test]
fn resampled_pipeline_mono_44k_to_stereo_48k() {
    let (mut engine, mut poster) = Engine::new(
        SampleRate::new(RATE).unwrap(),
        Channels::new(2).unwrap(),
        8192,
    )
    .unwrap();
    engine
        .load_track(Box::new(SyntheticDecoder::new(
            44_100,
            1,
            44_100 * 2,
            512,
            Waveform::Sine {
                freq_hz: 1_000.0,
                amplitude: 0.8,
            },
        )))
        .unwrap();
    engine.reserve_capture(48_000 * 2);
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(24_000);
    let cap = engine.capture();
    assert_eq!(cap.len(), 24_000 * 2);
    assert!(
        cap.iter().all(|s| s.is_finite()),
        "重采样输出不得出现 NaN/Inf"
    );
    // 频率保持（带迟滞的过零估频，取左声道中段）
    let start = cap.len() / 10;
    let end = cap.len() - cap.len() / 10;
    let left: Vec<f32> = cap[start..end].chunks_exact(2).map(|c| c[0]).collect();
    const HYSTERESIS: f32 = 0.1;
    let mut crossings = 0usize;
    let mut armed = false;
    for &s in &left {
        if s > HYSTERESIS {
            if !armed {
                crossings += 1;
            }
            armed = true;
        } else if s < -HYSTERESIS {
            armed = false;
        }
    }
    let est = crossings as f64 * f64::from(RATE) / left.len() as f64;
    assert!((est - 1_000.0).abs() < 50.0, "重采样后频率保持：est {est}");
}

#[test]
fn non_finite_pcm_is_sanitized() {
    let (mut engine, mut poster) = engine_with(Waveform::NonFinite, u64::from(RATE));
    poster.post(AudioCommand::Play).unwrap();
    let _ = engine.render_for_frames(CHUNK as u64 * 2);
    assert!(
        engine.capture().iter().all(|s| s.is_finite()),
        "输出不得包含 NaN/Inf（§37：确定性替换为 0）"
    );
    // NaN/Inf 区被替换为 0；其余为 0.5 电平
    assert!(engine.capture()[..16 * 2].iter().all(|s| *s == 0.0));
    assert!(engine.capture()[32 * 2..]
        .iter()
        .all(|s| (*s - 0.5).abs() < 1e-6));
}
