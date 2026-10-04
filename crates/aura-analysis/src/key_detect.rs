//! Key detection（§61–§66，D-101）：chroma（pitch-class energy）→
//! Krumhansl–Kessler 大/小调剖面相关 → key + confidence + Camelot。
//!
//! 不可靠语义（§64）：静音/平坦 chroma（噪声/语音）→ confidence 低于
//! 阈值 → Unavailable（不强行 C Major）。enharmonic（§66）：
//! canonical = 升号表示（C#/Db → Cs）。

use crate::domain::{KeyAnalysis, KeyMode, MusicalKey};
use crate::fft::magnitude_half;
use crate::params::{hann, CHROMA_FFT_LEN, CHROMA_HOP_LEN};

pub const KEY_ALGO_VERSION: u16 = 1;

/// Krumhansl–Kessler 剖面（12 音级；公开文献标准值）。
const KK_MAJOR: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
const KK_MINOR: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

/// 置信阈值：低于此 → Unavailable（§64）。语义见 D-101。
const MIN_CONFIDENCE: f32 = 0.20;

/// chroma（12 维 pitch-class 能量，帧均值）。
#[must_use]
#[allow(clippy::needless_range_loop)]
pub fn chroma(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let mut out = vec![0.0f32; 12];
    if samples.is_empty() || sample_rate == 0 {
        return out;
    }
    // 去均值：Hann 窗会把 DC 分量放大 window_sum 倍集中到最低 bins。
    let dc = samples.iter().sum::<f32>() / samples.len() as f32;
    let nyquist = sample_rate as f32 / 2.0;
    let mut windowed = vec![0.0f32; CHROMA_FFT_LEN];
    let mut frames = 0u64;
    let mut pos = 0usize;
    while pos + CHROMA_FFT_LEN <= samples.len() {
        for i in 0..CHROMA_FFT_LEN {
            windowed[i] = (samples[pos + i] - dc) * hann(i, CHROMA_FFT_LEN);
        }
        let mag = match magnitude_half(&windowed) {
            Ok(m) => m,
            Err(_) => break,
        };
        let total: f32 = mag.iter().sum();
        if total > f32::EPSILON {
            // 幅度（m¹）累积 + 跳过 <55 Hz（DC 泄漏集中区）。m² 的指数
            // 分布方差使噪声 chroma 假尖锐（实测 peak/mean 7.3），m¹ 更稳。
            for (bin, m) in mag.iter().enumerate().skip(1) {
                let freq = bin as f32 * nyquist / (mag.len() - 1) as f32;
                if (55.0..=nyquist).contains(&freq) {
                    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
                    let pc = (midi.round() as i64).rem_euclid(12) as usize;
                    out[pc] += m;
                }
            }
        }
        frames += 1;
        pos += CHROMA_HOP_LEN;
    }
    if frames > 0 {
        for v in &mut out {
            *v /= frames as f32;
        }
    }
    out
}

/// Pearson 相关。
fn pearson(a: &[f32], b: &[f32; 12]) -> f32 {
    let n = 12.0f32;
    let ma = a.iter().sum::<f32>() / n;
    let mb = b.iter().sum::<f32>() / n;
    let mut num = 0.0f32;
    let (mut da, mut db) = (0.0f32, 0.0f32);
    for i in 0..12 {
        num += (a[i] - ma) * (b[i] - mb);
        da += (a[i] - ma) * (a[i] - ma);
        db += (b[i] - mb) * (b[i] - mb);
    }
    if da <= f32::EPSILON || db <= f32::EPSILON {
        return 0.0;
    }
    num / (da.sqrt() * db.sqrt())
}

/// Camelot 映射（§65；Open Key 标准：大调=B，其关系小调=同号 A）。
/// 号数按五度圈：C=8, G=9, D=10, A=11, E=12, B=1, F#=2, Db=3, Ab=4,
/// Eb=5, Bb=6, F=7。小调 key 的号 = 其关系大调（+3 半音）的号。
#[must_use]
pub fn camelot(key: MusicalKey, mode: KeyMode) -> String {
    let major_pc = match mode {
        KeyMode::Major => key.pitch_class(),
        KeyMode::Minor => (key.pitch_class() + 3) % 12,
    };
    let num: u8 = match major_pc {
        0 => 8,  // C
        7 => 9,  // G
        2 => 10, // D
        9 => 11, // A
        4 => 12, // E
        11 => 1, // B
        6 => 2,  // F#
        1 => 3,  // C#/Db
        8 => 4,  // G#/Ab
        3 => 5,  // D#/Eb
        10 => 6, // A#/Bb
        5 => 7,  // F
        _ => 8,
    };
    let letter = match mode {
        KeyMode::Major => 'B',
        KeyMode::Minor => 'A',
    };
    format!("{num}{letter}")
}

/// 调性检测结果。
#[derive(Debug, Clone, PartialEq)]
pub enum KeyResult {
    Analysis(KeyAnalysis),
    /// 静音/噪声/平坦 chroma（§64 不强行给 key）。
    Unavailable,
}

/// 检测（§61–§66）。
#[must_use]
pub fn analyze_key(samples: &[f32], sample_rate: u32) -> KeyResult {
    let chroma = chroma(samples, sample_rate);
    let total: f32 = chroma.iter().sum();
    if total <= f32::EPSILON {
        return KeyResult::Unavailable; // 静音
    }
    // §64 前置门：chroma Shannon 熵（归一化，max = ln12 ≈ 2.485）。
    // 均匀（噪声/语音）→ 熵≈max；有调性 → 熵显著更低。
    // 实测门限 2.30（4096-FFT chroma）：音阶 run ≈ 2.0–2.2，
    // 白噪声 ≈ 2.37。
    let mut entropy = 0.0f32;
    for &v in &chroma {
        if v > f32::EPSILON {
            let p = v / total;
            entropy -= p * p.ln();
        }
    }
    if entropy > 2.30 {
        return KeyResult::Unavailable;
    }

    let dbg = std::env::var("AURA_KEY_DEBUG").is_ok();
    let mut all_r: Vec<(f32, MusicalKey, KeyMode)> = Vec::new();
    let mut best_r = f32::NEG_INFINITY;
    let mut second_r = f32::NEG_INFINITY;
    let mut best_key = MusicalKey::C;
    let mut best_mode = KeyMode::Major;
    for rotate in 0..12 {
        let rot_major: [f32; 12] = std::array::from_fn(|p| KK_MAJOR[(p + 12 - rotate) % 12]);
        let rot_minor: [f32; 12] = std::array::from_fn(|p| KK_MINOR[(p + 12 - rotate) % 12]);
        let r_major = pearson(&chroma, &rot_major);
        let r_minor = pearson(&chroma, &rot_minor);
        let key = MusicalKey::ALL[rotate];
        for (r, mode) in [(r_major, KeyMode::Major), (r_minor, KeyMode::Minor)] {
            all_r.push((r, key, mode));
            if r > best_r {
                second_r = best_r;
                best_r = r;
                best_key = key;
                best_mode = mode;
            } else if r > second_r {
                second_r = r;
            }
        }
    }

    if dbg {
        all_r.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (r, k, m) in all_r.iter().take(4) {
            eprintln!("KEYDBG {k:?} {m:?} r={r}");
        }
    }
    // §63 confidence：最优相关强度 × 与次优分离度 → [0,1]（D-101）。
    let strength = best_r.max(0.0);
    let separation = (best_r - second_r).max(0.0) * 4.0;
    let confidence = (strength * (0.5 + separation).min(1.0)).clamp(0.0, 1.0);
    if confidence < MIN_CONFIDENCE {
        return KeyResult::Unavailable; // §64 噪声/语音不强行
    }

    KeyResult::Analysis(KeyAnalysis {
        key: best_key,
        mode: best_mode,
        confidence,
        camelot: Some(camelot(best_key, best_mode)),
        algorithm_version: KEY_ALGO_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::ANALYSIS_SAMPLE_RATE;

    /// 合成和弦（§192 fixture）：音级集合正弦叠加。
    fn tones(pitch_classes: &[u8], seconds: f32, sr: u32, amp: f32) -> Vec<f32> {
        let n = (sr as f32 * seconds) as usize;
        let freqs: Vec<f32> = pitch_classes
            .iter()
            .map(|&pc| 440.0 * 2f32.powf((pc as f32 - 9.0) / 12.0))
            .collect();
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

    /// 顺序音阶 run（§192 synthetic fixture）：任一时刻仅 1-2 音，
    /// 避免低频半音在 FFT 主瓣内重叠；主音置于首尾并加重。



    #[test]
    fn c_major_triad_detected() {
        let r = analyze_key(
            &tones(&[0, 4, 7, 12], 6.0, ANALYSIS_SAMPLE_RATE, 0.8),
            ANALYSIS_SAMPLE_RATE,
        );
        match r {
            KeyResult::Analysis(a) => {
                assert_eq!(a.key, MusicalKey::C, "got {:?}", a.key);
                assert_eq!(a.mode, KeyMode::Major);
                assert_eq!(a.camelot.as_deref(), Some("8B"));
                assert!(a.confidence > MIN_CONFIDENCE);
            }
            KeyResult::Unavailable => panic!("C major 应可检测"),
        }
    }



    #[test]
    fn a_minor_triad_detected() {
        let r = analyze_key(
            &tones(&[9, 0, 4, 21], 6.0, ANALYSIS_SAMPLE_RATE, 0.8),
            ANALYSIS_SAMPLE_RATE,
        );
        match r {
            KeyResult::Analysis(a) => {
                assert_eq!(a.key, MusicalKey::A);
                assert_eq!(a.mode, KeyMode::Minor);
                assert_eq!(a.camelot.as_deref(), Some("8A"));
            }
            KeyResult::Unavailable => panic!("A minor 应可检测"),
        }
    }

    #[test]
    fn g_major_e_minor_d_major_detected() {
        // §192：G major / E minor / D major。
        for (root, mode, expect_key) in [
            (7u8, KeyMode::Major, MusicalKey::G),
            (4u8, KeyMode::Minor, MusicalKey::E),
            (2u8, KeyMode::Major, MusicalKey::D),
        ] {
            // §192：合成三和弦（root/三度/五度 + root 八度）。
            let third = if mode == KeyMode::Major {
                (root + 4) % 12
            } else {
                (root + 3) % 12
            };
            let samples = tones(
                &[root, third, (root + 7) % 12, root + 12],
                6.0,
                ANALYSIS_SAMPLE_RATE,
                0.8,
            );
            let _ = third;
            let r = analyze_key(&samples, ANALYSIS_SAMPLE_RATE);
            match r {
                KeyResult::Analysis(a) => {
                    assert_eq!(a.key, expect_key, "{expect_key:?}");
                    assert_eq!(a.mode, mode);
                }
                KeyResult::Unavailable => panic!("{expect_key:?} 应可检测"),
            }
        }
    }

    #[test]
    fn silence_unavailable() {
        let r = analyze_key(
            &vec![0.0; ANALYSIS_SAMPLE_RATE as usize * 4],
            ANALYSIS_SAMPLE_RATE,
        );
        assert_eq!(r, KeyResult::Unavailable);
    }

    #[test]
    fn noise_low_confidence_not_forced() {
        // §64/§170：噪声不强行给 key。
        let noise: Vec<f32> = (0..ANALYSIS_SAMPLE_RATE as usize * 6)
            .map(|i| {
                let x = (i as u64)
                    .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                    .wrapping_shr(40);
                (x as f32 / (1u32 << 24) as f32) - 0.5
            })
            .collect();
        match analyze_key(&noise, ANALYSIS_SAMPLE_RATE) {
            KeyResult::Unavailable => {}
            KeyResult::Analysis(a) => {
                panic!("噪声不应给出可信 key：{:?} conf={}", a.key, a.confidence)
            }
        }
    }

    #[test]
    fn deterministic_ten_runs() {
        // §185：×10 一致。
        let samples = tones(&[0, 4, 7, 12], 4.0, ANALYSIS_SAMPLE_RATE, 0.8);
        let first = analyze_key(&samples, ANALYSIS_SAMPLE_RATE);
        for _ in 0..9 {
            assert_eq!(analyze_key(&samples, ANALYSIS_SAMPLE_RATE), first);
        }
    }

    #[test]
    fn camelot_table_complete() {
        let mut seen = std::collections::HashSet::new();
        for k in MusicalKey::ALL {
            for m in [KeyMode::Major, KeyMode::Minor] {
                seen.insert(camelot(k, m));
            }
        }
        assert_eq!(seen.len(), 24);
        assert!(seen.contains("8B"));
        assert!(seen.contains("8A"));
    }

    #[test]
    fn enharmonic_canonical_sharp() {
        // §66：canonical 用升号——枚举仅含 Cs（无 Db），结构保证。
        assert!(MusicalKey::ALL.contains(&MusicalKey::Cs));
    }
}
