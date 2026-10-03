//! 等功率交叉淡化曲线（M2 §12–14）。
//!
//! 增益对：`gain_a = cos(t·π/2)`、`gain_b = sin(t·t·0 + t·π/2)` 的标准等功率
//! 形式（t ∈ [0,1]），满足近似恒定功率：cos²+sin² = 1。
//! 确定性、无分配、可单元测试。crossfade 帧数 = duration × sample_rate。

use crate::types::SampleRate;

/// 交叉淡化时长上限（秒，M2 §12）。
pub const CROSSFADE_MAX_SECONDS: f64 = 12.0;

/// 交叉淡化默认时长（秒）。0 = 关闭（纯 gapless）。产品默认由设置层决定；
/// 核心引擎默认 0（gapless 优先，DECISIONS D-028）。
pub const CROSSFADE_DEFAULT_SECONDS: f64 = 0.0;

/// 将秒转换为帧（四舍五入），并 clamp 到 [0, 12s] 上限。
#[must_use]
pub fn crossfade_frames(duration_seconds: f64, sample_rate: SampleRate) -> u64 {
    let d = duration_seconds.clamp(0.0, CROSSFADE_MAX_SECONDS);
    (d * f64::from(sample_rate.get())).round() as u64
}

/// 等功率增益对：t ∈ [0, 1]（0 = 全 A，1 = 全 B）。
/// 单调、无 overshoot、确定性。
#[must_use]
pub fn equal_power_gains(t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    let theta = t * std::f32::consts::FRAC_PI_2;
    (theta.cos(), theta.sin())
}

/// 交叉淡化一帧混合：`a·gain_a + b·gain_b`。
#[must_use]
pub fn mix_frame(a: f32, b: f32, t: f32) -> f32 {
    let (ga, gb) = equal_power_gains(t);
    a * ga + b * gb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_full_gain() {
        let (ga0, gb0) = equal_power_gains(0.0);
        let (ga1, gb1) = equal_power_gains(1.0);
        assert!((ga0 - 1.0).abs() < 1e-6 && gb0.abs() < 1e-6);
        assert!(ga1.abs() < 1e-6 && (gb1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn power_is_approximately_constant() {
        // 等功率：ga² + gb² ≈ 1（未叠加信号相关项，曲线本身满足恒功率）
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            let (ga, gb) = equal_power_gains(t);
            assert!((ga * ga + gb * gb - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn gains_are_monotonic() {
        let mut prev_a = 1.0f32;
        let mut prev_b = 0.0f32;
        for i in 0..=1000 {
            let t = i as f32 / 1000.0;
            let (ga, gb) = equal_power_gains(t);
            assert!(ga <= prev_a + 1e-6, "gain_a 非单调下降 @{t}");
            assert!(gb >= prev_b - 1e-6, "gain_b 非单调上升 @{t}");
            prev_a = ga;
            prev_b = gb;
        }
    }

    #[test]
    fn frames_conversion_clamps_and_rounds() {
        let sr = SampleRate::new(48_000).unwrap();
        assert_eq!(crossfade_frames(1.0, sr), 48_000);
        assert_eq!(crossfade_frames(6.0, sr), 288_000);
        assert_eq!(crossfade_frames(12.0, sr), 576_000);
        assert_eq!(crossfade_frames(60.0, sr), 576_000, "超过 12s 上限被钳制");
        assert_eq!(crossfade_frames(-1.0, sr), 0);
    }

    #[test]
    fn determinism() {
        let a = equal_power_gains(0.37);
        let b = equal_power_gains(0.37);
        assert_eq!(a, b);
    }
}
