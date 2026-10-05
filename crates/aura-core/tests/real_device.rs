//! M12.5 L1 真实设备集成测试（`#[ignore]`，本机手动跑）。
//!
//! 三个已修复 P0（流被 drop / 回调硬编码 Playing / stop 不归零）的共性：
//! NullOutput 路径全绿但真实 CPAL 回调路径从未被执行。本套件在**真实
//! 输出设备**上断言流生命周期、位置推进、状态传播、seek、曲终检测——
//! 这些正是 NullOutput 无法抓住的接缝。
//!
//! PCM 内容正确性仍由 NullOutput 确定性测试承担（计划 §4：不拆除）；
//! 本套件断言的是"真实回调在跑"的可观测代理：位置前进、underrun 计数、
//! 状态冻结。运行：`cargo test -p aura-core --test real_device -- --ignored --nocapture`
//!
//! 漂移说明：计划原文写"engine.capture() 对 PCM 断言"——外部 CPAL 输出
//! 接管消费端后 capture 不可见（render_for_frames 禁用），按架构以位置/
//! underrun/状态断言替代（等价于"回调真的在消费"）。

use std::time::{Duration, Instant};

use aura_core::types::AudioCommand;
use aura_core::{output::device::CpalOutput, Channels, Engine, PlaybackState, SampleRate};

mod common;
use common::{SyntheticDecoder, Waveform};

const RATE: u32 = 48_000;

struct RealOutput {
    engine: Engine,
    poster: aura_core::AudioCommandQueue,
    // 流所有权必须随测试存活（P0-1 教训：绑定局部变量返回即 drop）。
    _output: CpalOutput,
    shared: aura_core::SharedOutputState,
}

/// 真实设备 + 2s 合成曲目（低音量 0.05，避免打扰）。
fn setup(total_seconds: u64) -> RealOutput {
    let (mut engine, poster) = Engine::new(
        SampleRate::new(RATE).unwrap(),
        Channels::new(2).unwrap(),
        16_384,
    )
    .unwrap();
    engine
        .load_track(Box::new(SyntheticDecoder::new(
            RATE,
            2,
            u64::from(RATE) * total_seconds,
            512,
            Waveform::Sine {
                freq_hz: 440.0,
                amplitude: 0.05,
            },
        )))
        .unwrap();
    let (consumer, shared, eq) = engine.detach_consumer_for_external_output().unwrap();
    let output = CpalOutput::open_default(consumer, shared.clone(), eq)
        .expect("需要真实音频输出设备（本套件为 #[ignore] 手动套件）");
    RealOutput {
        engine,
        poster,
        _output: output,
        shared,
    }
}

fn pos_seconds(ro: &RealOutput) -> f64 {
    ro.shared
        .position
        .load(std::sync::atomic::Ordering::Relaxed) as f64
        / f64::from(RATE)
}

/// driver 职责的一部分：把引擎状态发布给回调（缺失=回调按 Stopped 静音，
/// 位置永不推进——本套件第一次实跑就抓住了 harness 漏掉这一步）。
fn pump(ro: &mut RealOutput) {
    ro.engine.fill_and_process();
    ro.shared.set_playback_state(ro.engine.state());
}

fn underruns(ro: &RealOutput) -> u64 {
    ro.shared
        .underruns
        .load(std::sync::atomic::Ordering::Relaxed)
}

#[test]
#[ignore = "需要真实音频输出设备；本机手动跑 --ignored"]
fn real_stream_position_advances_and_states_propagate() {
    let mut ro = setup(3);
    ro.poster.post(AudioCommand::Play).unwrap();

    // 真实回调消费：1s 内位置必须前进 ≥0.5s（回调不跑=位置恒 0，P0-1 症状）。
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(1) {
        pump(&mut ro);
        std::thread::sleep(Duration::from_millis(10));
    }
    let p1 = pos_seconds(&ro);
    assert!(p1 >= 0.5, "真实设备上位置必须推进，实际 {p1:.2}s");

    // Pause：位置冻结（P0-2 症状=暂停后仍以实时速率假推进）。
    ro.poster.post(AudioCommand::Pause).unwrap();
    pump(&mut ro);
    std::thread::sleep(Duration::from_millis(400));
    let p2 = pos_seconds(&ro);
    assert!(
        (p2 - p1).abs() < 0.1,
        "暂停后位置必须冻结：{p1:.2} → {p2:.2}"
    );

    // Resume：从暂停处继续。
    ro.poster.post(AudioCommand::Play).unwrap();
    pump(&mut ro);
    std::thread::sleep(Duration::from_millis(400));
    let p3 = pos_seconds(&ro);
    assert!(p3 > p2 + 0.1, "恢复后位置必须继续前进：{p2:.2} → {p3:.2}");
}

#[test]
#[ignore = "需要真实音频输出设备；本机手动跑 --ignored"]
fn real_stream_seek_jumps_position() {
    let mut ro = setup(4);
    ro.poster.post(AudioCommand::Play).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    pump(&mut ro);

    ro.poster
        .post(AudioCommand::Seek {
            frame: u64::from(RATE) * 3, // 3s
        })
        .unwrap();
    pump(&mut ro);
    std::thread::sleep(Duration::from_millis(300));
    pump(&mut ro);
    let p = pos_seconds(&ro);
    assert!(
        (p - 3.0).abs() < 0.4,
        "seek 后位置应 ≈3.0s（真实解码器+回调联动），实际 {p:.2}s"
    );
}

#[test]
#[ignore = "需要真实音频输出设备；本机手动跑 --ignored"]
fn real_stream_eof_finishes_and_stops() {
    // 1s 曲目 seek 到 ~0.83s：真实输出上放完 → track_finished（M12.6）→ Stop 语义。
    let mut ro = setup(1);
    ro.poster
        .post(AudioCommand::Seek {
            frame: 40_000, // ~0.83s，余 ~170ms
        })
        .unwrap();
    ro.poster.post(AudioCommand::Play).unwrap();
    pump(&mut ro);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut finished = false;
    while Instant::now() < deadline {
        pump(&mut ro);
        if ro.engine.take_track_finished() {
            finished = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(finished, "真实设备上曲目自然播完必须被检测（5s 内）");

    ro.poster.post(AudioCommand::Stop).unwrap();
    pump(&mut ro);
    assert_eq!(ro.engine.state(), PlaybackState::Stopped);
    assert_eq!(ro.engine.position_frames(), 0, "曲终 Stop 归零（可再播）");
}

#[test]
#[ignore = "需要真实音频输出设备；本机手动跑 --ignored"]
fn real_stream_no_underrun_when_fed_at_driver_cadence() {
    // 以应用 driver 的 10Hz 节拍供数：缓冲 16384 帧应吸收抖动，underrun=0。
    let mut ro = setup(3);
    ro.poster.post(AudioCommand::Play).unwrap();
    for _ in 0..150 {
        pump(&mut ro);
        std::thread::sleep(Duration::from_millis(10));
    }
    let u = underruns(&ro);
    assert_eq!(u, 0, "10Hz 供数节奏下不应 underrun（实际 {u}）");
}
