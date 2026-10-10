//! 电平探针（驱动线程侧；可视化/光场消费）。
//!
//! 三条一极 IIR 分频 + 块内 RMS，**无 FFT、无分配、无锁**：
//! `low = LP150`、`mid = LP2500 − LP150`、`high = x − LP2500`。
//! 目的不是测量精度，而是给出"当前有多响 / 偏低频还是偏高能"的可用趋势，
//! 因此按单调、可预期、可确定性测试的顺序设计。

/// 一次块的归一能量读数（全部 ≥ 0；静音 = 0）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LevelProbe {
    /// 全带 RMS。
    pub rms: f32,
    /// 低频带（≈ <150 Hz）。
    pub low: f32,
    /// 中频带（≈ 150 Hz–2.5 kHz）。
    pub mid: f32,
    /// 高频带（≈ >2.5 kHz）。
    pub high: f32,
}

/// 分频状态 + 块累加器（引擎持有；每驱动轮结算一次）。
#[derive(Debug, Clone)]
pub struct LevelTap {
    channels: usize,
    a_low: f32,
    a_mid: f32,
    lp_low: f32,
    lp_mid: f32,
    acc: [f64; 4],
    frames: u64,
}

/// 低频分频点（Hz）。
pub const LOW_HZ: f32 = 150.0;
/// 高频分频点（Hz）。
pub const HIGH_HZ: f32 = 2500.0;

impl LevelTap {
    #[must_use]
    pub fn new(sample_rate: u32, channels: usize) -> Self {
        let mut this = Self {
            channels: channels.max(1),
            a_low: 0.0,
            a_mid: 0.0,
            lp_low: 0.0,
            lp_mid: 0.0,
            acc: [0.0; 4],
            frames: 0,
        };
        this.configure(sample_rate);
        this
    }

    /// 采样率变化时重算系数（一极系数 `a = 1 − e^(−2πf/sr)`）。
    pub fn configure(&mut self, sample_rate: u32) {
        let sr = f64::from(sample_rate.max(1)) as f32;
        self.a_low = 1.0 - (-2.0 * std::f32::consts::PI * LOW_HZ / sr).exp();
        self.a_mid = 1.0 - (-2.0 * std::f32::consts::PI * HIGH_HZ / sr).exp();
    }

    /// 推入一段交错样本（多声道取均值后分频）。
    pub fn push(&mut self, interleaved: &[f32]) {
        let ch = self.channels;
        if ch == 0 {
            return;
        }
        for frame in interleaved.chunks(ch) {
            let mut sum = 0.0f32;
            let mut n = 0usize;
            for &s in frame {
                if s.is_finite() {
                    sum += s;
                    n += 1;
                }
            }
            let x = if n == 0 { 0.0 } else { sum / n as f32 };
            self.lp_low += self.a_low * (x - self.lp_low);
            self.lp_mid += self.a_mid * (x - self.lp_mid);
            let low = self.lp_low;
            let mid = self.lp_mid - self.lp_low;
            let high = x - self.lp_mid;
            self.acc[0] += f64::from(x * x);
            self.acc[1] += f64::from(low * low);
            self.acc[2] += f64::from(mid * mid);
            self.acc[3] += f64::from(high * high);
            self.frames += 1;
        }
    }

    /// 结算本块并清零（无样本 → 全零，用于暂停/停止时的自然回落）。
    pub fn finish_block(&mut self) -> LevelProbe {
        let out = if self.frames == 0 {
            LevelProbe::default()
        } else {
            let inv = 1.0 / self.frames as f64;
            let rms_of = |v: f64| (v * inv).sqrt() as f32;
            LevelProbe {
                rms: rms_of(self.acc[0]),
                low: rms_of(self.acc[1]),
                mid: rms_of(self.acc[2]),
                high: rms_of(self.acc[3]),
            }
        };
        self.acc = [0.0; 4];
        self.frames = 0;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    /// 定频正弦 → 期望落在对应频段。
    fn tone(freq: f32, frames: usize, amp: f32) -> Vec<f32> {
        (0..frames)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / SR as f32).sin())
            .collect()
    }

    #[test]
    fn silence_reads_zero() {
        let mut tap = LevelTap::new(SR, 2);
        tap.push(&vec![0.0; 2 * 512]);
        let p = tap.finish_block();
        assert!(
            p.rms.abs() < 1e-6 && p.low.abs() < 1e-6 && p.high.abs() < 1e-6,
            "{p:?}"
        );
    }

    #[test]
    fn empty_block_falls_back_to_zero() {
        let mut tap = LevelTap::new(SR, 2);
        tap.push(&[0.5; 4]);
        assert!(tap.finish_block().rms > 0.0);
        // 下一块无样本 → 回落 0（暂停/停止时画面必须暗下来）。
        assert_eq!(tap.finish_block(), LevelProbe::default());
    }

    #[test]
    fn bass_tone_dominates_low_band() {
        let mut tap = LevelTap::new(SR, 1);
        tap.push(&tone(60.0, 8192, 0.5));
        let p = tap.finish_block();
        assert!(p.low > p.mid && p.mid > p.high, "60Hz 应以低频为主: {p:?}");
        assert!(p.rms > 0.1 && p.rms < 0.5, "正弦 RMS ≈ amp/√2: {p:?}");
    }

    #[test]
    fn treble_tone_dominates_high_band() {
        let mut tap = LevelTap::new(SR, 1);
        tap.push(&tone(8000.0, 8192, 0.5));
        let p = tap.finish_block();
        assert!(p.high > p.mid && p.mid > p.low, "8kHz 应以高频为主: {p:?}");
    }

    #[test]
    fn non_finite_samples_do_not_poison_probe() {
        // 上游任何 NaN 都不得让电平永久卡死（原子量一旦写入 NaN 无法恢复）。
        let mut tap = LevelTap::new(SR, 1);
        tap.push(&[f32::NAN, f32::INFINITY, 0.4, -0.4]);
        let p = tap.finish_block();
        assert!(
            p.rms.is_finite() && p.low.is_finite() && p.mid.is_finite() && p.high.is_finite(),
            "{p:?}"
        );
    }

    #[test]
    fn probe_is_monotonic_in_amplitude() {
        let level_at = |amp: f32| {
            let mut tap = LevelTap::new(SR, 1);
            tap.push(&tone(220.0, 8192, amp));
            tap.finish_block().rms
        };
        assert!(level_at(0.1) < level_at(0.3), "更响必须读出更大");
        assert!(level_at(0.3) < level_at(0.6), "更响必须读出更大");
    }
}
