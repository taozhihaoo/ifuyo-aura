//! 频谱派生特征（§75–§79，D-102）：RMS / Peak / Centroid / Rolloff /
//! Brightness / Energy。
//!
//! 参数（§79，集中 params）：规范采样率 22050、Hann 1024/512。
//! Energy（§76/§77 D-102）：感知代理 = RMS dBFS 线性映射 [-60, 0] →
//! [0,1]（−60 dBFS 以下为 0，0 dBFS 为 1）——不是谱能量也不是振幅本身。
//! Brightness（§78）：质心 / (采样率/2)，[0,1]，有限确定。

use crate::fft::magnitude_half;
use crate::params::{hann, FRAME_LEN, HOP_LEN};

pub const SPECTRAL_ALGO_VERSION: u16 = 1;

/// 结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpectralResult {
    pub rms: f32,
    pub peak: f32,
    pub centroid_hz: f32,
    pub rolloff_hz: f32,
    pub brightness: f32,
    /// §76/§77：RMS dBFS → [0,1]（[-60,0] 线性映射）。
    pub energy: f32,
}

/// 全轨聚合（流式累加，O(1) 内存）。
#[derive(Debug, Default)]
pub struct SpectralAccumulator {
    frames: u64,
    peak: f32,
    centroid_num: f64,
    rolloff_sum: f64,
    brightness_sum: f64,
    total_samples: u64,
    sum_squares: f64,
}

impl SpectralAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// 喂入一段 mono PCM（任意长度；§90 流式）。
    pub fn push(&mut self, chunk: &[f32], sample_rate: u32) {
        if chunk.is_empty() || sample_rate == 0 {
            return;
        }
        self.total_samples += chunk.len() as u64;
        for &v in chunk {
            self.sum_squares += f64::from(v) * f64::from(v);
            self.peak = self.peak.max(v.abs());
        }
        let nyquist = sample_rate as f32 / 2.0;
        let mut windowed = vec![0.0f32; FRAME_LEN];
        let mut pos = 0usize;
        while pos + FRAME_LEN <= chunk.len() {
            for i in 0..FRAME_LEN {
                windowed[i] = chunk[pos + i] * hann(i, FRAME_LEN);
            }
            if let Ok(mag) = magnitude_half(&windowed) {
                let total: f32 = mag.iter().sum();
                if total > f32::EPSILON {
                    let mut num = 0.0f32;
                    for (k, &m) in mag.iter().enumerate() {
                        num += m * k as f32;
                    }
                    let centroid = num / total / (mag.len() - 1) as f32 * nyquist;
                    let mut cum = 0.0f32;
                    let mut rolloff = 0.0f32;
                    for (k, &m) in mag.iter().enumerate() {
                        cum += m;
                        if cum >= total * 0.85 {
                            rolloff = k as f32 / (mag.len() - 1) as f32 * nyquist;
                            break;
                        }
                    }
                    self.centroid_num += f64::from(centroid);
                    self.rolloff_sum += f64::from(rolloff);
                    self.brightness_sum += f64::from((centroid / nyquist).clamp(0.0, 1.0));
                    self.frames += 1;
                }
            }
            pos += HOP_LEN;
        }
    }

    /// 汇总（无频谱帧——纯静音——centroid/rolloff/brightness 为 0）。
    #[must_use]
    pub fn finish(&self) -> SpectralResult {
        let rms = (self.sum_squares / self.total_samples.max(1) as f64).sqrt() as f32;
        let energy = {
            let dbfs = 20.0 * rms.max(1e-9).log10();
            ((dbfs + 60.0) / 60.0).clamp(0.0, 1.0)
        };
        let frames = self.frames.max(1) as f64;
        SpectralResult {
            rms,
            peak: self.peak,
            centroid_hz: (self.centroid_num / frames) as f32,
            rolloff_hz: (self.rolloff_sum / frames) as f32,
            brightness: (self.brightness_sum / frames) as f32,
            energy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f32, seconds: f32, sr: u32, amp: f32) -> Vec<f32> {
        let n = (sr as f32 * seconds) as usize;
        (0..n)
            .map(|i| (std::f32::consts::TAU * freq * i as f32 / sr as f32).sin() * amp)
            .collect()
    }

    #[test]
    fn louder_sine_higher_rms_and_energy() {
        // §190：louder → RMS higher / energy higher。
        let quiet = {
            let mut a = SpectralAccumulator::new();
            a.push(&tone(440.0, 1.0, 22_050, 0.05), 22_050);
            a.finish()
        };
        let loud = {
            let mut a = SpectralAccumulator::new();
            a.push(&tone(440.0, 1.0, 22_050, 0.4), 22_050);
            a.finish()
        };
        assert!(loud.rms > quiet.rms * 5.0, "RMS 随振幅增长");
        assert!(loud.energy > quiet.energy);
        assert!((loud.peak - 0.4).abs() < 1e-3);
    }

    #[test]
    fn centroid_ordering_100hz_1khz_8khz() {
        // §194：100Hz < 1kHz < 8kHz。
        let mut vals = Vec::new();
        for f in [100.0f32, 1_000.0, 8_000.0] {
            let mut a = SpectralAccumulator::new();
            a.push(&tone(f, 1.0, 22_050, 0.5), 22_050);
            vals.push(a.finish().centroid_hz);
        }
        assert!(vals[0] < vals[1], "{vals:?}");
        assert!(vals[1] < vals[2], "{vals:?}");
        // 绝对近似（Hann 泄漏使实测质心略高于纯频率）。
        assert!(
            (vals[1] - 1_000.0).abs() < 400.0,
            "1kHz 质心 ≈ 1k：{}",
            vals[1]
        );
    }

    #[test]
    fn silent_input_energy_near_zero() {
        // §190：silent → energy ≈ 0（-60 dBFS 以下 clamp）。
        let mut a = SpectralAccumulator::new();
        a.push(&vec![0.0f32; 22_050 * 2], 22_050);
        let r = a.finish();
        assert_eq!(r.energy, 0.0);
        assert_eq!(r.centroid_hz, 0.0); // 无频谱帧
    }

    #[test]
    fn brightness_bounded_and_higher_for_treble() {
        let low = {
            let mut a = SpectralAccumulator::new();
            a.push(&tone(200.0, 1.0, 22_050, 0.5), 22_050);
            a.finish()
        };
        let high = {
            let mut a = SpectralAccumulator::new();
            a.push(&tone(6_000.0, 1.0, 22_050, 0.5), 22_050);
            a.finish()
        };
        assert!((0.0..=1.0).contains(&low.brightness));
        assert!((0.0..=1.0).contains(&high.brightness));
        assert!(high.brightness > low.brightness, "§78 亮度随频率");
    }

    #[test]
    fn rolloff_captures_most_energy() {
        let mut a = SpectralAccumulator::new();
        a.push(&tone(1_000.0, 1.0, 22_050, 0.5), 22_050);
        let r = a.finish();
        // 1kHz 单音：85% 能量点在主瓣附近（<< Nyquist 但 ≥ 信号频率附近）。
        assert!(r.rolloff_hz >= 900.0, "rolloff={}", r.rolloff_hz);
        assert!(r.rolloff_hz <= 11_025.0);
    }

    #[test]
    fn finite_everything() {
        let mut a = SpectralAccumulator::new();
        a.push(&tone(997.0, 0.5, 22_050, 0.3), 22_050);
        let r = a.finish();
        for v in [
            r.rms,
            r.peak,
            r.centroid_hz,
            r.rolloff_hz,
            r.brightness,
            r.energy,
        ] {
            assert!(v.is_finite(), "§178 numerical safety");
        }
    }
}
