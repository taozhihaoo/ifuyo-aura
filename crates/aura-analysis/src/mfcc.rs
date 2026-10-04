//! MFCC（§81–§85，D-103）：pre-emphasis 0.97 → Hann 1024/512 →
//! FFT 1024 → 功率谱 → 26 mel 滤波器（20–8kHz，HTK 式）→ ln（floor
//! 1e-10）→ DCT-II → 13 系数 → 帧均值/方差摘要（§83）。
//!
//! 数值安全（§85）：log 输入 floor 1e-10；输出逐项 is_finite 检查。
//! 确定性（§84）：纯定序 f32/f64 运算。

use crate::domain::MfccFeatures;
use crate::fft::fft_forward;
use crate::params::{hann, FRAME_LEN, HOP_LEN};

pub const MFCC_ALGO_VERSION: u16 = 1;
pub const MEL_FILTERS: usize = 26;
pub const MFCC_COUNT: usize = 13;
const PRE_EMPHASIS: f32 = 0.97;
const LOG_FLOOR: f64 = 1e-10;

/// HTK 式 Hz→mel / mel→Hz。
fn hz_to_mel(hz: f32) -> f32 {
    1127.0 * (1.0 + hz / 700.0).ln()
}

fn mel_to_hz(m: f32) -> f32 {
    700.0 * (m / 1127.0).exp() - 700.0
}

/// mel 滤波器组（三角，MEL_FILTERS 个，20Hz–(sr/2)）。
fn mel_filterbank(sample_rate: u32, fft_len: usize) -> Vec<Vec<f32>> {
    let nyquist = sample_rate as f32 / 2.0;
    let bins = fft_len / 2 + 1;
    let m_lo = hz_to_mel(20.0);
    let m_hi = hz_to_mel(nyquist);
    let points: Vec<f32> = (0..MEL_FILTERS + 2)
        .map(|i| mel_to_hz(m_lo + (m_hi - m_lo) * i as f32 / (MEL_FILTERS + 1) as f32))
        .collect();
    let mut bank = Vec::with_capacity(MEL_FILTERS);
    for f in 0..MEL_FILTERS {
        let (left, center, right) = (points[f], points[f + 1], points[f + 2]);
        let mut weights = vec![0.0f32; bins];
        for (k, w) in weights.iter_mut().enumerate() {
            let freq = k as f32 * nyquist / (bins - 1) as f32;
            if freq > left && freq < right {
                *w = if freq <= center {
                    (freq - left) / (center - left)
                } else {
                    (right - freq) / (right - center)
                };
            }
        }
        bank.push(weights);
    }
    bank
}

/// DCT-II（MEL_FILTERS 输入 → MFCC_COUNT 系数，正交归一）。
fn dct_ii(input: &[f32]) -> Vec<f32> {
    let n = input.len();
    let k_count = MFCC_COUNT.min(n);
    let mut out = vec![0.0f32; k_count];
    for (k, o) in out.iter_mut().enumerate() {
        let mut sum = 0.0f64;
        for (i, &v) in input.iter().enumerate() {
            sum += f64::from(v)
                * (std::f64::consts::PI * k as f64 * (2 * i + 1) as f64 / (2 * n) as f64).cos();
        }
        let scale = if k == 0 {
            (1.0 / n as f64).sqrt()
        } else {
            (2.0 / n as f64).sqrt()
        };
        *o = (sum * scale) as f32;
    }
    out
}

/// MFCC 分析（全长输入；§91：内部流式帧化，帧级系数不保留——只留摘要）。
///
/// # Errors
/// [`AnalysisError`]：过短 / 数值安全失败。
pub fn analyze_mfcc(
    samples: &[f32],
    sample_rate: u32,
) -> Result<MfccFeatures, crate::domain::AnalysisError> {
    use crate::domain::AnalysisError;
    if samples.len() < FRAME_LEN {
        return Err(AnalysisError::TooShort {
            samples: samples.len(),
        });
    }
    if samples.iter().any(|v| !v.is_finite()) {
        return Err(AnalysisError::NumericalSafety {
            feature: "mfcc-input".to_string(),
        });
    }
    // Pre-emphasis（一阶高通）。
    let mut emphasized = Vec::with_capacity(samples.len());
    emphasized.push(samples[0]);
    for w in samples.windows(2) {
        emphasized.push(w[1] - PRE_EMPHASIS * w[0]);
    }

    let bank = mel_filterbank(sample_rate.max(1), FRAME_LEN);
    let mut windowed = vec![0.0f32; FRAME_LEN];
    let mut mean_acc = [0.0f64; MFCC_COUNT];
    let mut sq_acc = [0.0f64; MFCC_COUNT];
    let mut min_acc = [f32::MAX; MFCC_COUNT];
    let mut max_acc = [f32::MIN; MFCC_COUNT];
    let mut frames = 0u64;
    let mut pos = 0usize;
    while pos + FRAME_LEN <= emphasized.len() {
        for i in 0..FRAME_LEN {
            windowed[i] = emphasized[pos + i] * hann(i, FRAME_LEN);
        }
        let (re, im) = fft_forward(&windowed).map_err(|_| AnalysisError::NumericalSafety {
            feature: "mfcc-fft".to_string(),
        })?;
        let half = FRAME_LEN / 2 + 1;
        let mut mel = vec![0.0f32; MEL_FILTERS];
        for (f, weights) in bank.iter().enumerate() {
            let mut acc = 0.0f64;
            for (k, w) in weights.iter().enumerate().take(half) {
                if *w > 0.0 {
                    let p =
                        f64::from(re[k]) * f64::from(re[k]) + f64::from(im[k]) * f64::from(im[k]);
                    acc += f64::from(*w) * p;
                }
            }
            // §85：log floor。
            mel[f] = (acc.max(LOG_FLOOR)).ln() as f32;
        }
        let coefs = dct_ii(&mel);
        frames += 1;
        for (i, &c) in coefs.iter().enumerate() {
            if !c.is_finite() {
                return Err(AnalysisError::NumericalSafety {
                    feature: "mfcc-coef".to_string(),
                });
            }
            let fi = f64::from(c);
            mean_acc[i] += fi;
            sq_acc[i] += fi * fi;
            min_acc[i] = min_acc[i].min(c);
            max_acc[i] = max_acc[i].max(c);
        }
        pos += HOP_LEN;
    }
    if frames == 0 {
        return Err(AnalysisError::TooShort {
            samples: samples.len(),
        });
    }
    let n = frames as f64;
    let mean: Vec<f32> = mean_acc.iter().map(|v| (v / n) as f32).collect();
    let variance: Vec<f32> = mean_acc
        .iter()
        .zip(&sq_acc)
        .map(|(m, s)| ((s / n - m * m).max(0.0)) as f32)
        .collect();
    Ok(MfccFeatures {
        mean,
        variance,
        algorithm_version: MFCC_ALGO_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::ANALYSIS_SAMPLE_RATE;

    fn sine(freq: f32, seconds: f32, sr: u32, amp: f32) -> Vec<f32> {
        let n = (sr as f32 * seconds) as usize;
        (0..n)
            .map(|i| (std::f32::consts::TAU * freq * i as f32 / sr as f32).sin() * amp)
            .collect()
    }

    #[test]
    fn basic_shape_and_finiteness() {
        // §195：finite + deterministic + distinguishable。
        let m = analyze_mfcc(
            &sine(440.0, 2.0, ANALYSIS_SAMPLE_RATE, 0.5),
            ANALYSIS_SAMPLE_RATE,
        )
        .unwrap();
        assert_eq!(m.mean.len(), MFCC_COUNT);
        assert_eq!(m.variance.len(), MFCC_COUNT);
        for v in m.mean.iter().chain(&m.variance) {
            assert!(v.is_finite());
        }
        assert!(m.algorithm_version == MFCC_ALGO_VERSION);
    }

    #[test]
    fn deterministic_across_runs() {
        // §187：系数数/均值/方差一致。
        let samples = sine(660.0, 1.5, ANALYSIS_SAMPLE_RATE, 0.4);
        let a = analyze_mfcc(&samples, ANALYSIS_SAMPLE_RATE).unwrap();
        for _ in 0..9 {
            assert_eq!(analyze_mfcc(&samples, ANALYSIS_SAMPLE_RATE).unwrap(), a);
        }
    }

    #[test]
    fn different_spectra_distinguishable() {
        let low = analyze_mfcc(
            &sine(220.0, 1.0, ANALYSIS_SAMPLE_RATE, 0.5),
            ANALYSIS_SAMPLE_RATE,
        )
        .unwrap();
        let high = analyze_mfcc(
            &sine(3_520.0, 1.0, ANALYSIS_SAMPLE_RATE, 0.5),
            ANALYSIS_SAMPLE_RATE,
        )
        .unwrap();
        let dist: f32 = low
            .mean
            .iter()
            .zip(&high.mean)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            .sqrt();
        assert!(dist > 0.5, "不同频谱 MFCC 可区分：dist={dist}");
    }

    #[test]
    fn log_floor_silence_safe() {
        // §85：静音（全零）→ log(0) 被 floor → 输出有限。
        let m = analyze_mfcc(
            &vec![0.0; ANALYSIS_SAMPLE_RATE as usize * 2],
            ANALYSIS_SAMPLE_RATE,
        )
        .unwrap();
        assert!(m.mean.iter().all(|v| v.is_finite()));
        assert!(m.variance.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn too_short_rejected() {
        let err = analyze_mfcc(&vec![0.1; 100], ANALYSIS_SAMPLE_RATE).unwrap_err();
        assert!(matches!(err, crate::domain::AnalysisError::TooShort { .. }));
    }

    #[test]
    fn nan_input_rejected() {
        let err = analyze_mfcc(
            &vec![f32::NAN; ANALYSIS_SAMPLE_RATE as usize * 2],
            ANALYSIS_SAMPLE_RATE,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            crate::domain::AnalysisError::NumericalSafety { .. }
        ));
    }

    #[test]
    fn mel_filterbank_sane() {
        // 每个 filter 权重和 > 0 且中心最大。
        let bank = mel_filterbank(ANALYSIS_SAMPLE_RATE, FRAME_LEN);
        assert_eq!(bank.len(), MEL_FILTERS);
        for w in &bank {
            let sum: f32 = w.iter().sum();
            assert!(sum > 0.0);
            let max = w.iter().cloned().fold(0.0f32, f32::max);
            assert!(max > 0.0);
        }
    }
}
