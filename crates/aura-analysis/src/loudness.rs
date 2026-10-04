//! Integrated LUFS / LRA / True Peak（§67–§74，D-099）。
//!
//! 标准（§68）：ITU-R BS.1770-4 链路——
//! - K-weighting：两节 biquad（high-shelf +4dB；high-pass ≈38Hz），
//!   双线性变换系数随采样率计算；
//! - 门控块：400ms / hop 100ms；绝对门 -70 LUFS + 相对门 -10 LU；
//! - channel weight：mono fold-down 单通道 = 1.0（§88）；
//! - LRA：3s short-term（hop 1s），绝对门 -70、相对门 -20 LU，P10–P95。
//! - True peak（§70）：4× 过采样（zero-stuff + 33-tap sinc×Hann 低通），
//!   取过采样峰值 → dBTP；正弦测试锁定其与 sample peak 的区分。
//! - 内存（§91）：K-weight 滤波 + 100ms hop 平方和累加（O(时长/10)
//!   f64，2 小时 ≈ 3.4MB，有界）；静音（§72）→ `None`（below
//!   measurable）。

use crate::domain::LoudnessAnalysis;

const LOUDNESS_ALGO_VERSION: u16 = 1;
const ABSOLUTE_GATE_LUFS: f32 = -70.0;
const RELATIVE_GATE_LU: f64 = 10.0;
const LRA_RELATIVE_GATE_LU: f64 = 20.0;
const TRUE_PEAK_OVERSAMPLE: usize = 4;
const TRUE_PEAK_TAPS: usize = 33;
const CHANNEL_WEIGHT: f64 = 1.0;

/// 全结果（含不可测语义）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessResult {
    pub analysis: LoudnessAnalysis,
}

/// K-weighting 两节 biquad 系数 `(b0,b1,b2,a1,a2)`（a0 归一）。
fn k_weight_coefficients(fs: f32) -> [(f32, f32, f32, f32, f32); 2] {
    // Stage 1：high-shelf（BS.1770 参考点）。
    let f0 = 1_681.975_f32;
    let gain_db = 3.999_844_f32;
    let q = 0.707_175_f32;
    let s1 = biquad(f0, fs, q, ShelfOrHighPass::Shelf, gain_db);
    // Stage 2：high-pass。
    let f0 = 38.135_47_f32;
    let q = 0.500_327_f32;
    let s2 = biquad(f0, fs, q, ShelfOrHighPass::HighPass, 0.0);
    [s1, s2]
}

enum ShelfOrHighPass {
    Shelf,
    HighPass,
}

#[allow(clippy::many_single_char_names)]
fn biquad(
    f0: f32,
    fs: f32,
    q: f32,
    kind: ShelfOrHighPass,
    gain_db: f32,
) -> (f32, f32, f32, f32, f32) {
    let w0 = std::f32::consts::TAU * f0 / fs;
    let cos_w0 = w0.cos();
    let alpha = w0.sin() / (2.0 * q);
    let (b0, b1, b2, a0, a1, a2);
    match kind {
        ShelfOrHighPass::Shelf => {
            let a = 10f32.powf(gain_db / 40.0);
            let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
            b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
            b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
            b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
            a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
            a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
            a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
        }
        ShelfOrHighPass::HighPass => {
            b0 = (1.0 + cos_w0) / 2.0;
            b1 = -(1.0 + cos_w0);
            b2 = (1.0 + cos_w0) / 2.0;
            a0 = 1.0 + alpha;
            a1 = -2.0 * cos_w0;
            a2 = 1.0 - alpha;
        }
    }
    (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
}

fn hop_lufs(hop_mean_square: f64) -> f32 {
    (-0.691 + 10.0 * (CHANNEL_WEIGHT * hop_mean_square).log10()) as f32
}

/// True peak 滤波器（sinc×Hann 低通，归一直流增益）。
fn true_peak_filter() -> [f32; TRUE_PEAK_TAPS] {
    let mut h = [0.0f32; TRUE_PEAK_TAPS];
    let mid = (TRUE_PEAK_TAPS - 1) / 2;
    for (i, h_i) in h.iter_mut().enumerate() {
        let t = i as f32 - mid as f32;
        let sinc = if t.abs() < 1e-6 {
            1.0
        } else {
            let x = std::f32::consts::PI * t / TRUE_PEAK_OVERSAMPLE as f32;
            x.sin() / x
        };
        *h_i = sinc * hann_win(i, TRUE_PEAK_TAPS);
    }
    // 归一化：每个相位子滤波器（t ≡ p mod OSR）和为 1 → 总和 = OSR。
    // （否则插值输出整体衰减 OSR 倍——首版实测 0.78 峰值测得 0.23。）
    let sum: f32 = h.iter().sum::<f32>() / TRUE_PEAK_OVERSAMPLE as f32;
    for h_i in &mut h {
        *h_i /= sum;
    }
    h
}

fn hann_win(i: usize, n: usize) -> f32 {
    0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos()
}

/// True peak（dBTP）；空输入 → 负无穷（调用方转 None 语义）。
#[must_use]
pub fn true_peak_dbtp(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return f32::NEG_INFINITY;
    }
    let h = true_peak_filter();
    // 扩展信号（零插入相位记录为峰值计算点）。
    let ext_len = samples.len() * TRUE_PEAK_OVERSAMPLE + h.len();
    let sample_at = |ext_idx: i64| -> f32 {
        if ext_idx < 0 {
            return 0.0;
        }
        let base = ext_idx.div_euclid(TRUE_PEAK_OVERSAMPLE as i64);
        let phase = ext_idx.rem_euclid(TRUE_PEAK_OVERSAMPLE as i64);
        if phase != 0 {
            0.0 // 零插入相位
        } else {
            samples.get(base as usize).copied().unwrap_or(0.0)
        }
    };
    let mut peak = 0.0f32;
    for i in 0..ext_len {
        let mut acc = 0.0f32;
        for (t, &c) in h.iter().enumerate() {
            acc += sample_at(i as i64 - t as i64) * c;
        }
        peak = peak.max(acc.abs());
    }
    20.0 * peak.max(1e-9).log10()
}

/// 分析结果：`None` = below measurable（§72）。
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn analyze_loudness(samples: &[f32], sample_rate: u32) -> Option<LoudnessResult> {
    if samples.is_empty() || sample_rate == 0 {
        return None;
    }
    let fs = sample_rate as f32;
    let [s1, s2] = k_weight_coefficients(fs);

    // 400ms 块 / 100ms hop / 3s short-term（1s hop）。
    let hop_len = (0.1 * fs) as usize;
    let hops_per_block = 4; // 400ms / 100ms
    let hops_per_sec = 10; // 1s / 100ms
    let hops_per_st = 3; // 3s / 1s
    if hop_len == 0 {
        return None;
    }

    // 流式 K-weight + hop 平方和（内存 O(时长×10) f64）。
    let (mut x1a, mut x2a, mut y1a, mut y2a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut x1b, mut x2b, mut y1b, mut y2b) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (b0a, b1a, b2a, a1a, a2a) = s1;
    let (b0b, b1b, b2b, a1b, a2b) = s2;
    let mut hop_sums: Vec<f64> = Vec::new();
    let mut hop_acc = 0.0f64;
    let mut hop_fill = 0usize;

    for &x in samples {
        let ya = b0a * x + b1a * x1a + b2a * x2a - a1a * y1a - a2a * y2a;
        x2a = x1a;
        x1a = x;
        y2a = y1a;
        y1a = ya;
        let yb = b0b * ya + b1b * x1b + b2b * x2b - a1b * y1b - a2b * y2b;
        x2b = x1b;
        x1b = ya;
        y2b = y1b;
        y1b = yb;

        hop_acc += f64::from(yb) * f64::from(yb);
        hop_fill += 1;
        if hop_fill == hop_len {
            hop_sums.push(hop_acc / hop_len as f64);
            hop_acc = 0.0;
            hop_fill = 0;
        }
    }
    if hop_fill > 0 {
        hop_sums.push(hop_acc / f64::from(hop_fill as u32));
    }

    // 门控块（400ms = 连续 hops_per_block 个 hop 的均值均方）。
    let block_ms: Vec<f64> = hop_sums
        .windows(hops_per_block)
        .map(|w| w.iter().sum::<f64>() / hops_per_block as f64)
        .collect();
    let block_lufs: Vec<f32> = block_ms.iter().map(|ms| hop_lufs(*ms)).collect();

    // 绝对门 → 相对门。
    let gated: Vec<f64> = block_ms
        .iter()
        .zip(&block_lufs)
        .filter(|(_, l)| **l > ABSOLUTE_GATE_LUFS)
        .map(|(ms, _)| *ms)
        .collect();
    if gated.is_empty() {
        return None; // §72 below measurable
    }
    let gated_mean_sq = gated.iter().sum::<f64>() / gated.len() as f64;
    let relative_gate_lufs =
        (-0.691 + 10.0 * (CHANNEL_WEIGHT * gated_mean_sq).log10()) - RELATIVE_GATE_LU;
    let relative_gate_lufs = relative_gate_lufs as f32;
    let final_ms: Vec<f64> = block_ms
        .iter()
        .zip(&block_lufs)
        .filter(|(_, l)| **l > relative_gate_lufs)
        .map(|(ms, _)| *ms)
        .collect();
    if final_ms.is_empty() {
        return None;
    }
    let final_mean_sq = final_ms.iter().sum::<f64>() / final_ms.len() as f64;
    let integrated = hop_lufs(final_mean_sq);

    // LRA：1s 块（10 hop 合成）→ 3s 窗（3 个 1s 块）。
    let sec_ms: Vec<f64> = hop_sums
        .chunks(hops_per_sec)
        .filter(|c| c.len() == hops_per_sec)
        .map(|c| c.iter().sum::<f64>() / hops_per_sec as f64)
        .collect();
    let mut st_loudness: Vec<f32> = sec_ms
        .windows(hops_per_st)
        .map(|w| {
            let ms = w.iter().sum::<f64>() / hops_per_st as f64;
            hop_lufs(ms)
        })
        .filter(|l| *l > ABSOLUTE_GATE_LUFS)
        .collect();
    let mut lra = 0.0f32;
    if st_loudness.len() > 2 {
        let mean =
            st_loudness.iter().map(|l| f64::from(*l)).sum::<f64>() / st_loudness.len() as f64;
        st_loudness.retain(|l| f64::from(*l) > mean - LRA_RELATIVE_GATE_LU);
        if st_loudness.len() > 2 {
            st_loudness.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let p10 = st_loudness[(st_loudness.len() as f32 * 0.10) as usize];
            let p95 = st_loudness
                [((st_loudness.len() as f32 * 0.95) as usize).min(st_loudness.len() - 1)];
            lra = p95 - p10;
        }
    }

    let analysis = LoudnessAnalysis {
        integrated_lufs: integrated,
        lra,
        true_peak_dbtp: true_peak_dbtp(samples),
        algorithm_version: LOUDNESS_ALGO_VERSION,
    };
    Some(LoudnessResult { analysis })
}

/// 合成目标 LUFS 的 1kHz 正弦振幅（fixture 用；mono 公式反解）。
/// L = -0.691 + 10·log10(A²/2) → A = sqrt(2·10^((L+0.691)/10))。
#[must_use]
pub fn amplitude_for_target_lufs(target_lufs: f32) -> f32 {
    ((2.0 * 10f64.powf(f64::from(target_lufs + 0.691) / 10.0)).sqrt()) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_1k(seconds: f32, amp: f32, sr: u32) -> Vec<f32> {
        let n = (sr as f32 * seconds) as usize;
        (0..n)
            .map(|i| (std::f32::consts::TAU * 1000.0 * i as f32 / sr as f32).sin() * amp)
            .collect()
    }

    #[test]
    fn full_scale_stereo_equiv_near_reference() {
        // mono 1kHz 0dBFS → L = -0.691 + 10log10(0.5) ≈ -3.70 LUFS。
        // 理论 -3.70（无 K-weight）；K-weight 在 1kHz 处 shelf 响应
        // 约 +0.4dB → 容差 ±0.6（§73 容差哲学，非 bit-exact §180）。
        let r = analyze_loudness(&sine_1k(6.0, 1.0, 48_000), 48_000).unwrap();
        assert!(
            (r.analysis.integrated_lufs - (-3.70)).abs() < 0.6,
            "integrated={}",
            r.analysis.integrated_lufs
        );
    }

    #[test]
    fn fixtures_near_target_lufs_with_tolerance() {
        // §73：-12/-16/-20/-23 LUFS 附近 fixture，显式容差 ±0.5 LU。
        for target in [-12.0f32, -16.0, -20.0, -23.0] {
            let amp = amplitude_for_target_lufs(target);
            let r = analyze_loudness(&sine_1k(6.0, amp, 48_000), 48_000).unwrap();
            assert!(
                (r.analysis.integrated_lufs - target).abs() < 0.5,
                "target {target}: got {}",
                r.analysis.integrated_lufs
            );
        }
    }

    #[test]
    fn scale_relation_monotonic() {
        // §193：+6 dB 振幅 → 集成响度 +≈6 LU。
        let quiet = analyze_loudness(&sine_1k(5.0, 0.05, 48_000), 48_000).unwrap();
        let loud = analyze_loudness(&sine_1k(5.0, 0.10, 48_000), 48_000).unwrap();
        let delta = loud.analysis.integrated_lufs - quiet.analysis.integrated_lufs;
        assert!((delta - 6.02).abs() < 0.2, "Δ={delta}");
    }

    #[test]
    fn silence_below_measurable() {
        // §72：静音 → None（不产生假 -0 LUFS）。
        assert!(analyze_loudness(&vec![0.0; 48_000 * 5], 48_000).is_none());
        assert!(analyze_loudness(&[], 48_000).is_none());
    }

    #[test]
    fn true_peak_of_sine_exceeds_sample_peak() {
        // §70：8k/48k 频率正弦（相位使 sample peak 低于连续峰值）→
        // true peak ≥ sample peak，且接近理论 dBFS。
        let sr = 48_000;
        let f = 8_000.0; // 每 6 样本一周期 → 样本峰值 < 1
        let samples: Vec<f32> = (0..sr)
            .map(|i| (std::f32::consts::TAU * f * i as f32 / sr as f32).sin() * 0.9)
            .collect();
        let sample_peak = samples.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        let tp = true_peak_dbtp(&samples);
        let tp_lin = 10f32.powf(tp / 20.0);
        assert!(tp_lin >= sample_peak * 0.98, "TP ≥ sample peak");
        assert!((tp_lin - 0.9).abs() < 0.05, "TP≈连续峰值 0.9：{tp_lin}");
    }

    #[test]
    fn true_peak_empty_is_neg_infinity() {
        assert!(true_peak_dbtp(&[]) == f32::NEG_INFINITY);
    }

    #[test]
    fn lra_and_true_peak_finite_for_tone() {
        // 20s 音（有 short-term 块）。
        let r = analyze_loudness(&sine_1k(20.0, 0.2, 22_050), 22_050).unwrap();
        assert!(r.analysis.lra.is_finite());
        assert!(r.analysis.true_peak_dbtp.is_finite());
        assert!(r.analysis.integrated_lufs.is_finite());
    }

    #[test]
    fn amplitude_for_target_matches_loudness_call() {
        // fixture 公式自洽：amplitude_for_target(-16) → 分析得 -16±0.5。
        let amp = amplitude_for_target_lufs(-16.0);
        let r = analyze_loudness(&sine_1k(6.0, amp, 48_000), 48_000).unwrap();
        assert!((r.analysis.integrated_lufs + 16.0).abs() < 0.5);
    }
}
