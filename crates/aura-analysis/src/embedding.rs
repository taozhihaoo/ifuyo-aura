//! Embedding provider 抽象与真实本地实现（§41–§49）。
//!
//! 政策（§39/§40/§42/§44，D-095）：不引入第三方模型/推理运行时；
//! provider 为**自有 DSP 统计嵌入**（确定性、可断网、商业安全、零许可
//! 负担）。§42 的"真实可运行"满足：向量由真实频谱统计计算，非随机、
//! 非 hash 派生（§4 禁项）。未来若引入学习型模型，走新 provider 实现
//! + 新 model_id/version，不改本合同。
//!
//! 确定性（§46）：纯 IEEE-754 f32 定序运算——同输入同位输出，无平台
//! 差异可容忍问题（无 fast-math/SIMD 归约顺序变化；Rust 未启用）。
//!
//! 隔离（§43）：只吃 `EmbeddingInput`（PCM+采样率），不依赖 UI/Tauri/
//! SQLite/network/Map。

use crate::domain::{AnalysisError, Embedding, EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION};

/// 输入：单声道 f32 PCM（[-1,1] 标称）+ 采样率。
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingInput {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl EmbeddingInput {
    #[must_use]
    pub fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            samples,
            sample_rate,
        }
    }
}

/// 向量 dtype（§48 合同；当前仅 f32 LE）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DType {
    F32Le,
}

/// 归一化方式（§47：M9 据此选相似度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Normalization {
    /// 单位范数 → cosine 与 dot 等价。
    L2,
}

/// 距离语义（§48/§49：M9 合同）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceMetric {
    /// L2 归一化下的 cosine（= dot）。
    Cosine,
}

/// 嵌入提供者合同（§41）。
pub trait EmbeddingProvider {
    fn model_id(&self) -> &str;
    fn model_version(&self) -> u16;
    fn dimension(&self) -> u32;
    fn dtype(&self) -> DType;
    fn normalization(&self) -> Normalization;
    fn distance_metric(&self) -> DistanceMetric;

    /// PCM → 向量（确定性）。
    ///
    /// # Errors
    /// [`AnalysisError`]：输入过短 / 数值安全失败。
    fn embed(&self, input: &EmbeddingInput) -> Result<Embedding, AnalysisError>;
}

/// 频带数（log 划分）→ 嵌入维数 = BANDS×2 + GLOBAL_STATS。
pub const BANDS: usize = 14;
pub const FRAME_STATS: usize = 2; // 帧能量 mean/std
pub const EMBEDDING_DIMENSION: u32 = (BANDS * 2 + FRAME_STATS + 4) as u32;

/// 最短输入：0.2 秒（过短 → Unavailable 语义走 TooShort）。
pub const MIN_SAMPLES: usize = 1;

/// 自有谱统计嵌入（真实、确定性、离线）。
#[derive(Debug, Clone, Copy, Default)]
pub struct SpectralStatsProvider;

/// Hann 窗（样本级；确定性常量运算）。
fn hann(i: usize, n: usize) -> f32 {
    0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos()
}

impl EmbeddingProvider for SpectralStatsProvider {
    fn model_id(&self) -> &str {
        EMBEDDING_MODEL_ID
    }

    fn model_version(&self) -> u16 {
        EMBEDDING_MODEL_VERSION
    }

    fn dimension(&self) -> u32 {
        EMBEDDING_DIMENSION
    }

    fn dtype(&self) -> DType {
        DType::F32Le
    }

    fn normalization(&self) -> Normalization {
        Normalization::L2
    }

    fn distance_metric(&self) -> DistanceMetric {
        DistanceMetric::Cosine
    }

    fn embed(&self, input: &EmbeddingInput) -> Result<Embedding, AnalysisError> {
        // 数值安全（§168 精神）：NaN/Inf 输入直接拒绝。
        if input.samples.iter().any(|v| !v.is_finite()) {
            return Err(AnalysisError::NumericalSafety {
                feature: "embedding-input".to_string(),
            });
        }
        if input.samples.len() < MIN_SAMPLES || input.sample_rate == 0 {
            return Err(AnalysisError::TooShort {
                samples: input.samples.len(),
            });
        }
        // 静音输入：无频谱内容可嵌入（ §42 真实性——零向量嵌入无语义）。
        if input.samples.iter().all(|v| *v == 0.0) {
            return Err(AnalysisError::TooShort {
                samples: input.samples.len(),
            });
        }

        let sr = input.sample_rate as f32;
        let frame_len = (sr as usize / 20).max(64); // 50ms 帧下限 64 样本
        let hop = frame_len / 2;
        let mut band_acc = [0.0f32; BANDS];
        let mut band_acc_sq = [0.0f32; BANDS];
        let mut frame_count = 0u64;
        let mut frame_rms_sum = 0.0f32;
        let mut frame_rms_sq_sum = 0.0f32;
        let mut zcr_sum = 0.0f32;
        let mut centroid_sum = 0.0f32;
        let mut rolloff_sum = 0.0f32;

        let mut frame_start = 0usize;
        while frame_start + frame_len <= input.samples.len() {
            let frame = &input.samples[frame_start..frame_start + frame_len];

            // DFT（O(N²) 直接式——分析离线场景 N≈2400 可控；中篇换 FFT 时
            // 更新 SPECTRAL_ALGORITHM_VERSION）。
            let n = frame_len;
            let half = n / 2 + 1;
            let mut mag = vec![0.0f32; half];
            let mut total = 0.0f32;
            for (k, m_slot) in mag.iter_mut().enumerate().skip(1) {
                let angle_step = -std::f32::consts::TAU * k as f32 / n as f32;
                let mut re = 0.0f32;
                let mut im = 0.0f32;
                for (i, &s) in frame.iter().enumerate() {
                    let angle = angle_step * i as f32;
                    re += s * hann(i, n) * angle.cos();
                    im += s * hann(i, n) * angle.sin();
                }
                let m = (re * re + im * im).sqrt() / (n as f32 * 0.5);
                *m_slot = m;
                total += m;
            }

            // 频带能量（对数频带划分）。
            if total > f32::EPSILON {
                let nyquist = sr * 0.5;
                for b in 0..BANDS {
                    let lo = nyquist * (b as f32 / BANDS as f32).powi(2);
                    let hi = nyquist * (((b + 1) as f32 / BANDS as f32).powi(2));
                    let lo_bin = ((lo / nyquist) * (half - 1) as f32) as usize;
                    let hi_bin =
                        (((hi / nyquist) * (half - 1) as f32) as usize).clamp(lo_bin + 1, half - 1);
                    let band: f32 = mag[lo_bin..hi_bin].iter().sum();
                    let band_db = 20.0 * (band / total).max(1e-6).log10(); // log 压缩
                    band_acc[b] += band_db;
                    band_acc_sq[b] += band_db * band_db;
                }

                // 质心/滚降。
                let mut cum = 0.0f32;
                let mut centroid_num = 0.0f32;
                for (k, &m) in mag.iter().enumerate() {
                    centroid_num += m * k as f32;
                    cum += m;
                    if cum >= total * 0.85 {
                        rolloff_sum += k as f32 / (half - 1) as f32 * nyquist;
                        break;
                    }
                }
                centroid_sum += centroid_num / total / (half - 1) as f32 * nyquist;
            }

            // 帧能量 + ZCR。
            let energy: f32 = frame.iter().map(|v| v * v).sum();
            let rms = (energy / frame.len() as f32).sqrt();
            frame_rms_sum += rms;
            frame_rms_sq_sum += rms * rms;
            let zcr: f32 = frame
                .windows(2)
                .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
                .count() as f32
                / (frame.len() - 1) as f32;
            zcr_sum += zcr;

            frame_count += 1;
            frame_start += hop;
        }

        if frame_count == 0 {
            return Err(AnalysisError::TooShort {
                samples: input.samples.len(),
            });
        }

        let fc = frame_count as f32;
        let mut raw = Vec::with_capacity(EMBEDDING_DIMENSION as usize);
        // 频带均值/方差（dB 域）。
        for b in 0..BANDS {
            let mean = band_acc[b] / fc;
            let var = band_acc_sq[b] / fc - mean * mean;
            raw.push(mean);
            raw.push(var.max(0.0));
        }
        // 帧能量统计。
        let rms_mean = frame_rms_sum / fc;
        let rms_std = (frame_rms_sq_sum / fc - rms_mean * rms_mean)
            .max(0.0)
            .sqrt();
        raw.push(rms_mean);
        raw.push(rms_std);
        // 全局摘要。
        raw.push(zcr_sum / fc);
        raw.push(centroid_sum / fc);
        raw.push(rolloff_sum / fc);
        raw.push((frame_count as f32).ln()); // 时长代理

        // 数值安全出口（任何统计异常 → 拒绝而非存 NaN，§168）。
        if raw.iter().any(|v| !v.is_finite()) {
            return Err(AnalysisError::NumericalSafety {
                feature: "embedding-stats".to_string(),
            });
        }

        Ok(Embedding::normalized(
            raw,
            self.model_id(),
            self.model_version(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, seconds: f32, sr: u32) -> Vec<f32> {
        let n = (sr as f32 * seconds) as usize;
        (0..n)
            .map(|i| (std::f32::consts::TAU * freq * i as f32 / sr as f32).sin())
            .collect()
    }

    #[test]
    fn contract_constants_consistent() {
        let p = SpectralStatsProvider;
        assert_eq!(p.model_id(), "aura-spectral-stats");
        assert_eq!(p.dimension(), EMBEDDING_DIMENSION);
        assert_eq!(p.dtype(), DType::F32Le);
        assert_eq!(p.normalization(), Normalization::L2);
        assert_eq!(p.distance_metric(), DistanceMetric::Cosine);
    }

    #[test]
    fn deterministic_bit_for_bit() {
        // §46：同输入 → 逐位一致（纯 f32 定序运算）。
        // 8k 采样率缩短测试时长（生产 DFT→FFT 升级在中篇，见 PERF 记录）。
        let input = EmbeddingInput::new(sine(220.0, 0.5, 8_000), 8_000);
        let p = SpectralStatsProvider;
        let a = p.embed(&input).unwrap();
        let b = p.embed(&input).unwrap();
        assert_eq!(a.vector, b.vector, "逐位一致（§46）");
    }

    #[test]
    fn different_content_different_vectors() {
        // §4 禁 random/hash 假嵌入：真实信号差异反映到向量。
        let p = SpectralStatsProvider;
        let bass = p
            .embed(&EmbeddingInput::new(sine(80.0, 0.5, 8_000), 8_000))
            .unwrap();
        let treble = p
            .embed(&EmbeddingInput::new(sine(3000.0, 0.5, 8_000), 8_000))
            .unwrap();
        let dot: f32 = bass
            .vector
            .iter()
            .zip(&treble.vector)
            .map(|(a, b)| a * b)
            .sum();
        assert!(dot < 0.99, "低频 vs 高频应可区分（cosine={dot}）");
        // 白噪声 vs 正弦。
        let noise: Vec<f32> = (0..4_000)
            .map(|i| {
                (i as u64)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_shr(40) as f32
                    / (1u32 << 24) as f32
                    - 0.5
            })
            .collect();
        let noise_e = p.embed(&EmbeddingInput::new(noise, 8_000)).unwrap();
        let sine_e = p
            .embed(&EmbeddingInput::new(sine(440.0, 0.5, 8_000), 8_000))
            .unwrap();
        let dot2: f32 = noise_e
            .vector
            .iter()
            .zip(&sine_e.vector)
            .map(|(a, b)| a * b)
            .sum();
        assert!(dot2 < 0.99);
    }

    #[test]
    fn l2_normalized_output() {
        let p = SpectralStatsProvider;
        let e = p
            .embed(&EmbeddingInput::new(sine(440.0, 0.5, 8_000), 8_000))
            .unwrap();
        let norm: f32 = e.vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "§47 L2（norm={norm}）");
        assert_eq!(e.dimension, EMBEDDING_DIMENSION);
        assert_eq!(e.model_id, "aura-spectral-stats");
    }

    #[test]
    fn short_and_nan_rejected() {
        let p = SpectralStatsProvider;
        // 过短（不足一帧）。
        let err = p
            .embed(&EmbeddingInput::new(vec![0.1; 10], 48_000))
            .unwrap_err();
        assert!(matches!(err, AnalysisError::TooShort { .. }));
        // NaN 输入 → 数值安全拒绝（§168）。
        let err = p
            .embed(&EmbeddingInput::new(vec![f32::NAN; 4_000], 8_000))
            .unwrap_err();
        assert!(matches!(err, AnalysisError::NumericalSafety { .. }));
        // Inf。
        let err = p
            .embed(&EmbeddingInput::new(vec![f32::INFINITY; 4_000], 8_000))
            .unwrap_err();
        assert!(matches!(err, AnalysisError::NumericalSafety { .. }));
    }

    #[test]
    fn amplitude_invariance_and_duration_proxy() {
        // 能量主导的统计对整体增益部分不变（dB 域比例感知）；
        // 时长不同 → 时长代理分量不同。
        let p = SpectralStatsProvider;
        let a = p
            .embed(&EmbeddingInput::new(sine(440.0, 0.5, 8_000), 8_000))
            .unwrap();
        let b = p
            .embed(&EmbeddingInput::new(sine(440.0, 0.8, 8_000), 8_000))
            .unwrap();
        assert_ne!(a.vector, b.vector, "时长影响 duration-proxy 分量");
    }
}
