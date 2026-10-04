//! 规范分析参数（§79/§87/§88）：全项目分析共用一套，禁止各算法自选。

/// 规范分析采样率（§87）：22 050 Hz——Nyquist 11 025 Hz 覆盖节奏/调性/
/// 倒谱需要的全部频段；内存与 DFT 成本减半。D-098。
pub const ANALYSIS_SAMPLE_RATE: u32 = 22_050;

/// 声道策略（§88）：稳定 mono fold-down（(L+R)/2，§89），所有算法统一。
pub const ANALYSIS_CHANNELS: u16 = 1;

/// 频谱帧参数（§79：窗 1024 / hop 512 / Hann；全部算法共享）。
pub const FRAME_LEN: usize = 1024;
pub const HOP_LEN: usize = 512;

/// 分析时长上限（§166 max analysis duration）：tempo/key/spectral/mfcc
/// 只分析开头该秒数（有界内存 §91/§167）；loudness 全轨流式累加。
pub const MAX_ANALYSIS_SECONDS: u32 = 120;
/// 解码 chunk（样本数，mono）——流式解码步长（§90）。
pub const CHUNK_SAMPLES: usize = 65_536;
/// 单曲最大解码样本数（§166 max decoded frames；= MAX_ANALYSIS_SECONDS
/// × rate，但 loudness 除外——loudness 无帧级内存，可超）。
pub const MAX_FEATURE_SAMPLES: usize =
    MAX_ANALYSIS_SECONDS as usize * ANALYSIS_SAMPLE_RATE as usize;

/// beat grid 上限（§59 max beat count）。
pub const MAX_BEATS: usize = 20_000;

/// chroma 专用 FFT 长度（§79：每算法 FFT 尺寸集中定义于此）：
/// 4096 @ 22050 = 5.4Hz/bin——低频半音（261/277Hz 差 15.6Hz ≈ 2.9 bins）
/// 可分辨，降低 pitch-class 串扰。
pub const CHROMA_FFT_LEN: usize = 4096;
/// chroma hop。
pub const CHROMA_HOP_LEN: usize = 2048;

/// Hann 窗（样本级；与 M7 synth 的实现同公式，此处独立用于频谱帧）。
#[must_use]
pub fn hann(i: usize, n: usize) -> f32 {
    0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos()
}

/// 简单线性插值重采样到规范采样率（D-098：分析用线性插值——
/// 关键特征（节奏/调性/倒谱统计）对轻微混叠不敏感；记录为已知限制）。
#[must_use]
pub fn resample_linear(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = f64::from(to) / f64::from(from);
    let out_len = ((input.len() as f64) * ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let i0 = src.floor() as usize;
        let frac = (src - i0 as f64) as f32;
        let a = input[i0];
        let b = input.get(i0 + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identity_and_ratio() {
        let x: Vec<f32> = (0..100).map(|i| i as f32).collect();
        assert_eq!(resample_linear(&x, 22_050, 22_050).len(), 100);
        // 2:1 下采样 → 约一半长度。
        let half = resample_linear(&x, 44_100, 22_050);
        assert_eq!(half.len(), 50);
        assert!((half[10] - x[20]).abs() < 1e-4);
        // 上采样放大。
        let dbl = resample_linear(&x, 22_050, 44_100);
        assert_eq!(dbl.len(), 200);
    }

    #[test]
    fn resample_empty_safe() {
        assert!(resample_linear(&[], 48_000, 22_050).is_empty());
    }

    #[test]
    fn hann_symmetric() {
        assert!((hann(0, 8) - 0.0).abs() < 1e-6);
        assert!((hann(4, 8) - 1.0).abs() < 1e-4);
        assert!((hann(1, 8) - hann(7, 8)).abs() < 1e-6);
    }
}
