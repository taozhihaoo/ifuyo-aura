//! 合成音频生成器（§261–§264，D-107）：测试专用确定性 fixture。
//!
//! - 全部固定参数（§263 无 seed 随机——噪声用固定 seed PRNG）。
//! - tiny/legal/redistributable（§261：程序生成，无第三方素材）。
//! - fixture 元数据（§264）：[`FixtureMeta`]。

/// fixture 元数据（§264）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixtureMeta {
    pub name: &'static str,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_seconds: f32,
    /// 预期特征摘要（tolerance 由各测试定义）。
    pub expected: ExpectedFeature,
}

/// §264 预期特征。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExpectedFeature {
    /// BPM ≈ 期望值（±3）。
    Tempo { bpm: f32 },
    /// 能量 ≈ 0（静音）。
    Silence,
    /// 频谱质心 ≈ 频率（±400Hz）。
    Tone { freq: f32 },
}

/// 正弦（§262）。
#[must_use]
pub fn sine(freq: f32, seconds: f32, sr: u32, amp: f32) -> Vec<f32> {
    let n = (sr as f32 * seconds) as usize;
    (0..n)
        .map(|i| (std::f32::consts::TAU * freq * i as f32 / sr as f32).sin() * amp)
        .collect()
}

/// 和弦（多正弦叠加，归一）。
#[must_use]
pub fn chord(freqs: &[f32], seconds: f32, sr: u32, amp: f32) -> Vec<f32> {
    let n = (sr as f32 * seconds) as usize;
    (0..n)
        .map(|i| {
            freqs
                .iter()
                .map(|f| (std::f32::consts::TAU * f * i as f32 / sr as f32).sin() * amp)
                .sum::<f32>()
                / freqs.len() as f32
        })
        .collect()
}

/// click track（节奏 fixture，§55）：每拍衰减 sine burst。
#[must_use]
pub fn click_track(bpm: f32, seconds: f32, sr: u32) -> Vec<f32> {
    let period = (60.0 / bpm * sr as f32) as usize;
    let burst = (0.03 * sr as f32) as usize;
    let mut out = vec![0.0f32; (sr as f32 * seconds) as usize];
    let mut t = 0usize;
    while t + burst < out.len() {
        for i in 0..burst {
            let env = 1.0 - i as f32 / burst as f32;
            out[t + i] += env * (std::f32::consts::TAU * 1_000.0 * i as f32 / sr as f32).sin();
        }
        t += period;
    }
    out
}

/// 静音（§262）。
#[must_use]
pub fn silence(seconds: f32, sr: u32) -> Vec<f32> {
    vec![0.0; (sr as f32 * seconds) as usize]
}

/// 白噪声（固定 seed PRNG，§263 确定性）。
#[must_use]
pub fn noise(seconds: f32, sr: u32, seed: u64) -> Vec<f32> {
    let n = (sr as f32 * seconds) as usize;
    let mut state = seed;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 40) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect()
}

/// 脉冲串（§262 impulse）。
#[must_use]
pub fn impulse_train(period_samples: usize, count: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; period_samples * count];
    for k in 0..count {
        if k * period_samples < out.len() {
            out[k * period_samples] = 1.0;
        }
    }
    out
}

/// 标准 fixture 集（§264 元数据+内容；D-107）。
#[must_use]
pub fn standard_fixtures() -> Vec<(FixtureMeta, Vec<f32>)> {
    let sr = ANALYSIS_SAMPLE_RATE;
    let mk = |name: &'static str, expected: ExpectedFeature, v: Vec<f32>| {
        (
            FixtureMeta {
                name,
                sample_rate: sr,
                channels: 1,
                duration_seconds: v.len() as f32 / sr as f32,
                expected,
            },
            v,
        )
    };
    vec![
        mk("silence-6s", ExpectedFeature::Silence, silence(6.0, sr)),
        mk(
            "sine-440-4s",
            ExpectedFeature::Tone { freq: 440.0 },
            sine(440.0, 4.0, sr, 0.5),
        ),
        mk(
            "click-120bpm-12s",
            ExpectedFeature::Tempo { bpm: 120.0 },
            click_track(120.0, 12.0, sr),
        ),
        mk(
            "noise-8s",
            ExpectedFeature::Tone { freq: 0.0 },
            noise(8.0, sr, 0xfeed),
        ),
    ]
}

use crate::params::ANALYSIS_SAMPLE_RATE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generators_deterministic() {
        // §263：固定参数 → 逐位一致。
        assert_eq!(sine(440.0, 0.5, 22_050, 0.5), sine(440.0, 0.5, 22_050, 0.5));
        assert_eq!(noise(1.0, 22_050, 7), noise(1.0, 22_050, 7));
        assert_ne!(
            noise(1.0, 22_050, 7),
            noise(1.0, 22_050, 8),
            "不同 seed 不同输出"
        );
        assert_eq!(
            click_track(120.0, 2.0, 22_050),
            click_track(120.0, 2.0, 22_050)
        );
    }

    #[test]
    fn fixtures_have_metadata_and_content() {
        for (meta, samples) in standard_fixtures() {
            assert_eq!(meta.sample_rate, ANALYSIS_SAMPLE_RATE);
            assert_eq!(meta.channels, 1);
            assert!(
                (samples.len() as f32 / meta.sample_rate as f32 - meta.duration_seconds).abs()
                    < 0.1
            );
            assert!(!samples.is_empty());
        }
    }

    #[test]
    fn silence_fixture_is_silent() {
        for (meta, samples) in standard_fixtures() {
            if meta.expected == ExpectedFeature::Silence {
                assert!(samples.iter().all(|v| *v == 0.0));
            }
        }
    }
}
