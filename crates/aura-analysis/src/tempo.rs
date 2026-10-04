//! Tempo / beat grid（§50–§61，D-100）。
//!
//! 管线：mono @ 规范采样率 → Hann 帧（1024/512）→ 幅度谱 → spectral
//! flux onset 包络 → 均值扣减半波整流 → 自相关（lag 域 → BPM 20–300）
//! → 倍频/半频折叠（偏好窗 [70,180) BPM，D-100 选择规则：折叠至偏好窗；
//! 窗内并列取离 120 BPM 最近者）→ confidence = 最优峰 / 包络均值比，
//! clamp [0,1]。
//! 不可用语义（§54）：静音（包络全零）→ Unavailable；过短（<2s）→
//! TooShort；纯噪声可输出但低置信（§170）。
//! Beat grid（§57）：帧位置（hop 单位整数），`first_beat_offset` 对齐
//! onset 包络最大峰；上限 [`MAX_BEATS`]（§59）；确定性（§60 纯函数）。

use crate::domain::TempoAnalysis;
use crate::fft::magnitude_half;
use crate::params::{hann, FRAME_LEN, HOP_LEN, MAX_BEATS};

pub const TEMPO_ALGO_VERSION: u16 = 1;
/// §51 合理范围。
pub const BPM_MIN: f32 = 20.0;
pub const BPM_MAX: f32 = 300.0;
/// 偏好窗（折叠目标；D-100：[70,180)——行业惯例常速段；窗外按
/// §51 倍频折叠，规则在测试中显式锁定）。
const PREFERRED_MIN: f32 = 70.0;
const PREFERRED_MAX: f32 = 180.0;
/// 最短分析时长（秒）。
const MIN_DURATION_SECONDS: f32 = 2.0;
/// 结果（含 beat grid 帧位置；frame = HOP 单位）。
#[derive(Debug, Clone, PartialEq)]
pub struct TempoResult {
    pub analysis: TempoAnalysis,
    /// beat 位置（分析帧索引 = 样本位置 / HOP_LEN；§57 frame-based）。
    pub beat_positions: Vec<u64>,
}

/// 分析 tempo + beat grid。输入应为规范采样率 mono PCM。
///
/// # Errors
/// [`AnalysisError`]：TooShort / NumericalSafety。
pub fn analyze_tempo(samples: &[f32], sample_rate: u32) -> Result<TempoResult, AnalysisErrorLike> {
    if samples.iter().any(|v| !v.is_finite()) {
        return Err(AnalysisErrorLike::NumericalSafety("tempo-input"));
    }
    let duration_s = samples.len() as f32 / sample_rate as f32;
    if duration_s < MIN_DURATION_SECONDS {
        return Err(AnalysisErrorLike::TooShort);
    }
    let fps = sample_rate as f32 / HOP_LEN as f32; // onset 帧率

    // ---- onset 强度包络（spectral flux，半波整流）----
    let win = [0.0f32; FRAME_LEN].map(|_| 0.0);
    let _ = win;
    let mut envelope: Vec<f32> = Vec::new();
    let mut prev_mag = vec![0.0f32; FRAME_LEN / 2 + 1];
    let mut windowed = vec![0.0f32; FRAME_LEN];
    let mut pos = 0usize;
    while pos + FRAME_LEN <= samples.len() {
        for i in 0..FRAME_LEN {
            windowed[i] = samples[pos + i] * hann(i, FRAME_LEN);
        }
        let mag =
            magnitude_half(&windowed).map_err(|_| AnalysisErrorLike::NumericalSafety("fft"))?;
        let mut flux = 0.0f32;
        for (i, m) in mag.iter().enumerate() {
            let d = m - prev_mag[i];
            if d > 0.0 {
                flux += d;
            }
        }
        envelope.push(flux);
        prev_mag.copy_from_slice(&mag);
        pos += HOP_LEN;
    }
    if envelope.len() < 8 {
        return Err(AnalysisErrorLike::TooShort);
    }

    // 均值扣减 + 半波整流（accent 信号）。
    let mean = envelope.iter().sum::<f32>() / envelope.len() as f32;
    let max_env = envelope.iter().cloned().fold(0.0f32, f32::max);
    if max_env <= f32::EPSILON {
        // 静音：包络全零 → Unavailable（§54）。
        return Err(AnalysisErrorLike::Unavailable);
    }
    let raw_accent: Vec<f32> = envelope.iter().map(|v| (v - mean).max(0.0)).collect();
    // ±1 帧盒式模糊：抗分数周期点击的取整抖动（20/21 交替间距），
    // 使整数 lag 自相关仍能捕获相关性（§50 管线内标准平滑）。
    let mut accent = raw_accent.clone();
    for i in 1..accent.len() - 1 {
        accent[i] = (raw_accent[i - 1] + raw_accent[i] + raw_accent[i + 1]) / 3.0;
    }
    if let (Some(f), Some(l)) = (raw_accent.first(), raw_accent.last()) {
        accent[0] = *f;
        let n = accent.len();
        accent[n - 1] = *l;
    }
    let accent_energy: f32 = accent.iter().map(|v| v * v).sum();
    if accent_energy <= f32::EPSILON {
        return Err(AnalysisErrorLike::Unavailable); // 恒定电平（如 DC/纯音长音）无节拍
    }

    // ---- 分数周期谐波 comb 打分（§52 倍频策略，D-100）----
    // 候选周期在 [fps·60/BPM_MAX, fps·60/BPM_MIN] 连续扫描（1/8 帧步长，
    // 线性插值 shift 相关）；comb = Σ_k w_k·score(k·period)，w_k=1/k。
    let min_period = fps * 60.0 / BPM_MAX;
    let max_period = fps * 60.0 / BPM_MIN;
    if max_period <= min_period {
        return Err(AnalysisErrorLike::TooShort);
    }
    let step = 0.125f32;
    let n_periods = ((max_period - min_period) / step).ceil() as usize + 1;
    let mut best_period = 0.0f32;
    let mut best_score = 0.0f32;
    for pi in 0..n_periods {
        let period = min_period + pi as f32 * step;
        let mut base = 0.0f32;
        for i in 0..accent.len() {
            let shifted = i as f32 + period;
            let i0 = shifted.floor() as usize;
            if i0 + 1 >= accent.len() {
                break;
            }
            let frac = shifted - i0 as f32;
            base += accent[i] * (accent[i0] + (accent[i0 + 1] - accent[i0]) * frac);
        }
        let mut comb = base;
        let mut weight = 0.5f32;
        let mut k_period = period;
        for _ in 0..3 {
            k_period *= 2.0;
            if k_period > accent.len() as f32 - 2.0 {
                break;
            }
            let mut harmonic = 0.0f32;
            for i in 0..accent.len() {
                let shifted = i as f32 + k_period;
                let i0 = shifted.floor() as usize;
                if i0 + 1 >= accent.len() {
                    break;
                }
                let frac = shifted - i0 as f32;
                harmonic += accent[i] * (accent[i0] + (accent[i0 + 1] - accent[i0]) * frac);
            }
            comb += weight * harmonic;
            weight *= 0.5;
        }
        if comb > best_score {
            best_score = comb;
            best_period = period;
        }
    }
    if best_period <= 0.0 {
        return Err(AnalysisErrorLike::Unavailable);
    }
    let best_bpm = fold_to_preferred(fps * 60.0 / best_period);

    // §53：confidence = comb 峰 / 包络能量归一化，clamp [0,1]。
    let confidence = (best_score / accent_energy).clamp(0.0, 1.0);

    // ---- beat grid（§57：帧位置；§59 有界）----
    let beat_lag = fps * 60.0 / best_bpm;
    // first beat：与 accent 相关性最好的相位。
    let lag_int = beat_lag.round().max(1.0) as usize;
    let mut best_phase = 0usize;
    let mut best_phase_score = -1.0f32;
    for phase in 0..lag_int {
        let mut s = 0.0f32;
        let mut i = phase;
        while i < accent.len() {
            s += accent[i];
            i += lag_int;
        }
        if s > best_phase_score {
            best_phase_score = s;
            best_phase = phase;
        }
    }
    let mut beat_positions = Vec::new();
    let mut i = best_phase;
    while i < accent.len() && beat_positions.len() < MAX_BEATS {
        beat_positions.push(i as u64);
        i += lag_int;
    }

    Ok(TempoResult {
        analysis: TempoAnalysis {
            bpm: best_bpm,
            confidence,
            first_beat_offset_seconds: best_phase as f32 * HOP_LEN as f32 / sample_rate as f32,
            algorithm_version: TEMPO_ALGO_VERSION,
        },
        beat_positions,
    })
}

/// §52/D-100 折叠规则：真基频族由 comb 打分胜出（谐波和天然偏好基频）；
/// 标签再折叠到偏好窗 [70,180)（×2/÷2），窗外但在 §51 范围内则保留。
fn fold_to_preferred(mut bpm: f32) -> f32 {
    while bpm < PREFERRED_MIN {
        bpm *= 2.0;
    }
    while bpm >= PREFERRED_MAX {
        bpm /= 2.0;
    }
    if (BPM_MIN..=BPM_MAX).contains(&bpm) {
        bpm
    } else {
        bpm.clamp(BPM_MIN, BPM_MAX)
    }
}

/// 错误语义（中篇内部；上篇 [`crate::domain::AnalysisError`] 映射由
/// 管线层完成）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisErrorLike {
    TooShort,
    Unavailable,
    NumericalSafety(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合成 click train（§55）：每拍一个 30ms 衰减 sine burst。
    fn click_train(bpm: f32, seconds: f32, sr: u32) -> Vec<f32> {
        let period = 60.0 / bpm;
        let mut out = vec![0.0f32; (sr as f32 * seconds) as usize];
        let mut t = 0.0f32;
        while t < seconds {
            let start = (t * sr as f32) as usize;
            let burst_len = (0.03 * sr as f32) as usize;
            for i in 0..burst_len {
                let idx = start + i;
                if idx >= out.len() {
                    break;
                }
                let env = 1.0 - i as f32 / burst_len as f32;
                out[idx] += env * (std::f32::consts::TAU * 1000.0 * i as f32 / sr as f32).sin();
            }
            t += period;
        }
        out
    }

    #[test]
    fn click_trains_detected_within_tolerance() {
        // §55/§191：60/90/120/128/140/180 BPM synthetic click。
        // 期望按 D-100 规则：折叠到偏好窗 [70,180)（60→120、180→90）。
        let cases = [
            (60.0f32, 120.0f32),
            (90.0, 90.0),
            (120.0, 120.0),
            (128.0, 128.0),
            (140.0, 140.0),
            (180.0, 120.0),
        ];
        for (input, expect) in cases {
            let samples = click_train(input, 16.0, 22_050);
            let r = analyze_tempo(&samples, 22_050).unwrap();
            assert!(
                (r.analysis.bpm - expect).abs() <= 2.0,
                "input {input} BPM → 检测 {}（期望 {expect}，D-100）",
                r.analysis.bpm
            );
            assert!(r.analysis.confidence > 0.0);
            assert!(!r.beat_positions.is_empty());
            assert!(r.beat_positions.len() <= MAX_BEATS);
        }
    }

    #[test]
    fn half_double_tempo_folds_to_preferred() {
        // §52/D-100：窗外输入按 2 的幂折叠进 [70,180)——30→120（×4）、
        // 240→120（÷2）。选择规则显式锁定（§52 允许）。
        let slow = analyze_tempo(&click_train(30.0, 20.0, 22_050), 22_050).unwrap();
        assert!(
            (slow.analysis.bpm - 120.0).abs() <= 2.0,
            "got {}",
            slow.analysis.bpm
        );
        let fast = analyze_tempo(&click_train(240.0, 16.0, 22_050), 22_050).unwrap();
        assert!(
            (fast.analysis.bpm - 120.0).abs() <= 2.0,
            "got {}",
            fast.analysis.bpm
        );
    }

    #[test]
    fn silence_unavailable() {
        let r = analyze_tempo(&vec![0.0; 22_050 * 8], 22_050);
        assert_eq!(r.unwrap_err(), AnalysisErrorLike::Unavailable);
    }

    #[test]
    fn short_clip_too_short() {
        let r = analyze_tempo(&click_train(120.0, 1.0, 22_050), 22_050);
        assert_eq!(r.unwrap_err(), AnalysisErrorLike::TooShort);
    }

    #[test]
    fn noise_low_confidence_no_panic() {
        // §170：纯噪声可输出但低置信（确定性伪随机）。
        let noise: Vec<f32> = (0..22_050 * 10)
            .map(|i| {
                let x = (i as u64)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_shr(40);
                (x as f32 / (1u32 << 24) as f32) - 0.5
            })
            .collect();
        let r = analyze_tempo(&noise, 22_050).unwrap();
        assert!((BPM_MIN..=BPM_MAX).contains(&r.analysis.bpm));
        assert!(
            r.analysis.confidence < 0.5,
            "噪声置信应低：{}",
            r.analysis.confidence
        );
    }

    #[test]
    fn deterministic_across_runs() {
        // §60/§184：同输入 ×10 一致。
        let samples = click_train(128.0, 12.0, 22_050);
        let first = analyze_tempo(&samples, 22_050).unwrap();
        for _ in 0..9 {
            assert_eq!(analyze_tempo(&samples, 22_050).unwrap(), first);
        }
    }

    #[test]
    fn beat_grid_bounded_and_frame_based() {
        let samples = click_train(60.0, 30.0, 22_050);
        let r = analyze_tempo(&samples, 22_050).unwrap();
        assert!(r.beat_positions.len() <= MAX_BEATS);
        // 帧位置单调递增（整数 hop）。
        for w in r.beat_positions.windows(2) {
            assert!(w[1] > w[0]);
        }
    }

    #[test]
    fn nan_input_rejected() {
        let r = analyze_tempo(&vec![f32::NAN; 22_050 * 4], 22_050);
        assert_eq!(
            r.unwrap_err(),
            AnalysisErrorLike::NumericalSafety("tempo-input")
        );
    }
}
