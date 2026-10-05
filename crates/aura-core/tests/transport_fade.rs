//! M12.5 传输淡变（1s play/pause/stop 淡入淡出）单元测试。
//!
//! 直连 OutputStage + ring pair + SharedOutputState（与 CPAL 回调同路径）：
//! 恒定 0.5 输入下，输出值直接暴露淡变增益曲线（增益形状自证）。

use aura_core::output::{
    OutputStage, SharedOutputState, FADE_CMD_CANCEL, FADE_CMD_IN, FADE_CMD_NONE, FADE_CMD_OUT,
};
use aura_core::{ring_buffer, Channels, ProducerHandle, SampleRate};

const AMP: f32 = 0.5; // 输入恒定幅度（避开 limiter 阈值）
const TOTAL: u64 = 1_000; // 淡变总帧数（测试用 1s/48）

fn setup() -> (
    ProducerHandle,
    aura_core::ConsumerHandle,
    OutputStage,
    SharedOutputState,
) {
    let (p, c) = ring_buffer(Channels::new(2).unwrap(), 8_192).unwrap();
    let stage = OutputStage::new(SampleRate::new(48_000).unwrap(), Channels::new(2).unwrap());
    (p, c, stage, SharedOutputState::new())
}

/// 推入 n 帧恒定幅度立体声 PCM。
fn push(p: &mut ProducerHandle, frames: usize) {
    let buf = vec![AMP; frames * 2];
    let mut pushed = 0;
    while pushed < buf.len() {
        pushed += p.push_frames(&buf[pushed..]) as usize * 2;
    }
}

fn render(
    stage: &mut OutputStage,
    c: &mut aura_core::ConsumerHandle,
    shared: &SharedOutputState,
    frames: usize,
) -> Vec<f32> {
    let mut out = vec![0.0f32; frames * 2];
    stage.render(&mut out, c, shared, aura_core::PlaybackState::Playing, None);
    out
}

#[test]
fn fade_out_shapes_linear_holds_silence_after_completion() {
    let (mut p, mut c, mut stage, shared) = setup();
    push(&mut p, 3_000);
    shared.set_playback_state(aura_core::PlaybackState::Playing);
    shared.request_fade(FADE_CMD_OUT, TOTAL);

    // 块 1（480 帧）：首样本增益 1.0，末样本 ≈ 1 - 479/1000（帧域线性）。
    let a = render(&mut stage, &mut c, &shared, 480);
    assert!((a[0] - AMP).abs() < 1e-4, "首样本全增益，得 {}", a[0]);
    assert!(
        (a[958] - AMP * (1.0 - 479.0 / TOTAL as f32)).abs() < 1e-3,
        "末样本按帧进度衰减，得 {}",
        a[958]
    );
    assert_eq!(shared.fade_progress(), 480, "进度按帧计（非采样）");

    // 块 2 与块 1 增益衔接（无跳变）：块 2 首增益 = 块 1 末增益。
    let b = render(&mut stage, &mut c, &shared, 480);
    assert!(
        (b[0] - a[958]).abs() < 1e-3,
        "块间增益连续，{} vs {}",
        b[0],
        a[958]
    );

    // 跨过 total：进度钳位，增益钳位 0（相位不复位——driver 应用暂停前
    // 的间隙保持静音，防回跳爆音）。
    let _ = render(&mut stage, &mut c, &shared, 480);
    assert_eq!(shared.fade_progress(), TOTAL, "进度钳位在 total");
    let d = render(&mut stage, &mut c, &shared, 480);
    assert!(d.iter().all(|s| s.abs() < 1e-5), "完成后钳位静音");

    // driver 序列收尾：clear_fade → 仍静音；随后显式 0 起点淡入恢复。
    shared.clear_fade();
    let e = render(&mut stage, &mut c, &shared, 480);
    assert!(e.iter().all(|s| s.abs() < 1e-5), "clear 后钳位静音保持");
    shared.request_fade_gain(FADE_CMD_IN, TOTAL, 0.0);
    let f = render(&mut stage, &mut c, &shared, 480);
    assert!(f[0].abs() < 1e-5, "淡入从 0 出发");
    assert!(f[958] > 0.1, "淡入上行，得 {}", f[958]);
}

#[test]
fn fade_in_ramps_to_full_and_latches_at_unity() {
    let (mut p, mut c, mut stage, shared) = setup();
    push(&mut p, 3_000);
    shared.set_playback_state(aura_core::PlaybackState::Playing);
    shared.request_fade_gain(FADE_CMD_IN, TOTAL, 0.0);

    let a = render(&mut stage, &mut c, &shared, 480);
    assert!(a[0].abs() < 1e-5, "首样本静音起步，得 {}", a[0]);
    assert!(
        (a[400] - AMP * (400.0 / 2.0 / TOTAL as f32)).abs() < 1e-3,
        "帧域线性上行，得 {}",
        a[400]
    );

    // 跨过 total 后增益钳位 1.0：后续输出等于输入（稳态无操作）。
    let _ = render(&mut stage, &mut c, &shared, 480);
    let _ = render(&mut stage, &mut c, &shared, 480);
    assert_eq!(shared.fade_progress(), TOTAL);
    let d = render(&mut stage, &mut c, &shared, 480);
    assert!(d.iter().all(|s| (s - AMP).abs() < 1e-4), "钳位后全增益直通");
}

#[test]
fn mid_fade_reversal_continues_from_published_gain() {
    let (mut p, mut c, mut stage, shared) = setup();
    push(&mut p, 3_000);
    shared.set_playback_state(aura_core::PlaybackState::Playing);

    // 淡出 400 帧后中途反转（用户淡出中按播放）：driver 场景即
    // request_fade(IN)——起始增益取最近发布的块末值。
    shared.request_fade(FADE_CMD_OUT, TOTAL);
    let a = render(&mut stage, &mut c, &shared, 400);
    let last_gain = a[a.len() - 1] / AMP;
    assert!(
        (last_gain - 0.6).abs() < 0.01,
        "400 帧处增益 ≈0.6，得 {last_gain}"
    );

    shared.request_fade(FADE_CMD_IN, TOTAL);
    let b = render(&mut stage, &mut c, &shared, 480);
    let first_gain = b[0] / AMP;
    assert!(
        (first_gain - last_gain).abs() < 0.01,
        "反转首样本衔接当前增益（不跳变），{} vs {last_gain}",
        first_gain
    );
    assert!(b[b.len() - 1] / AMP > first_gain, "反转后上行");
}

#[test]
fn fade_cancel_restores_full_gain_immediately() {
    let (mut p, mut c, mut stage, shared) = setup();
    push(&mut p, 2_000);
    shared.set_playback_state(aura_core::PlaybackState::Playing);
    shared.request_fade(FADE_CMD_OUT, TOTAL);
    let _ = render(&mut stage, &mut c, &shared, 200);

    shared
        .fade_cmd
        .store(FADE_CMD_CANCEL, std::sync::atomic::Ordering::Release);
    let b = render(&mut stage, &mut c, &shared, 480);
    assert!(b.iter().all(|s| (s - AMP).abs() < 1e-4), "取消后立即全增益");
    assert_eq!(
        shared.fade_cmd.load(std::sync::atomic::Ordering::Acquire),
        FADE_CMD_NONE,
        "命令单次消费"
    );
}
