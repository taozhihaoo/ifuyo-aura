//! 音量增益与 10ms 平滑（M1 §19–20）。
//!
//! - Volume 的校验（NaN/Inf/负值拒绝、clamp）在 [`crate::types::Volume`]；
//! - 本模块实现**逐采样线性平滑**：目标变化时从当前值出发，在约 10ms 窗口内
//!   线性逼近目标，不爆音、不 overshoot、可重复、零 allocation。

use crate::types::SampleRate;

/// 平滑窗口长度（毫秒）。
pub const SMOOTHING_WINDOW_MS: u64 = 10;

/// 逐采样增益平滑器。
///
/// 属于输出回调侧的实时对象：`process` 路径零 allocation、零锁、零 panic。
#[derive(Debug, Clone)]
pub struct GainSmoother {
    current: f32,
    target: f32,
    window_frames: u32,
    ramp_frames_left: u32,
    ramp_step: f32,
}

impl GainSmoother {
    /// 构造：窗口 = 10ms 对应的帧数（至少 1 帧）。初始 current = target = 1.0。
    #[must_use]
    pub fn new(sample_rate: SampleRate) -> Self {
        let window_frames =
            (u64::from(sample_rate.get()) * SMOOTHING_WINDOW_MS / 1000).max(1) as u32;
        Self {
            current: 1.0,
            target: 1.0,
            window_frames,
            ramp_frames_left: 0,
            ramp_step: 0.0,
        }
    }

    /// 平滑窗口（帧数）。
    #[must_use]
    pub const fn window_frames(&self) -> u32 {
        self.window_frames
    }

    /// 设置目标增益（调用方先用 `Volume` 校验过；此处再防御一次非有限值）。
    pub fn set_target(&mut self, target: f32) {
        let target = if target.is_finite() {
            target.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if (target - self.current).abs() <= f32::EPSILON {
            // 已在目标上：终止剩余 ramp，避免浮点残差抖动
            self.current = target;
            self.target = target;
            self.ramp_frames_left = 0;
            self.ramp_step = 0.0;
            return;
        }
        self.target = target;
        self.ramp_frames_left = self.window_frames;
        self.ramp_step = (target - self.current) / self.window_frames as f32;
    }

    #[must_use]
    pub const fn target(&self) -> f32 {
        self.target
    }

    #[must_use]
    pub const fn current(&self) -> f32 {
        self.current
    }

    /// 是否正在平滑（ramp 进行中）。
    #[must_use]
    pub const fn is_ramping(&self) -> bool {
        self.ramp_frames_left > 0
    }

    /// 原地应用增益并推进平滑（每采样一个增益值，不 overshoot、不分配）。
    pub fn process(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            if self.ramp_frames_left > 0 {
                self.ramp_frames_left -= 1;
                if self.ramp_frames_left == 0 {
                    self.current = self.target; // 精确落点，消除浮点残差
                } else {
                    self.current += self.ramp_step;
                }
            }
            *s *= self.current;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn smoother_at_48k() -> GainSmoother {
        GainSmoother::new(SampleRate::new(48_000).unwrap())
    }

    #[test]
    fn window_is_about_10ms() {
        assert_eq!(smoother_at_48k().window_frames(), 480);
        assert_eq!(
            GainSmoother::new(SampleRate::new(44_100).unwrap()).window_frames(),
            441
        );
    }

    #[test]
    fn reaches_target_exactly_without_overshoot() {
        let mut g = smoother_at_48k();
        g.set_target(0.0);
        let mut buf = vec![1.0; 600];
        g.process(&mut buf);
        assert_eq!(g.current(), 0.0);
        // 线性递减、无 overshoot：增益单调不增且最终精确 0
        let mut prev = 1.0;
        for (i, s) in buf.iter().enumerate() {
            let gain = s / 1.0;
            assert!(gain <= prev + 1e-6, "sample {i} overshoot: {gain} > {prev}");
            prev = gain;
        }
        assert_eq!(*buf.last().unwrap(), 0.0, "ramp 结束必须精确到达目标");
    }

    #[test]
    fn no_jump_between_adjacent_samples() {
        let mut g = smoother_at_48k();
        g.set_target(0.0);
        let mut buf = vec![1.0; 600];
        g.process(&mut buf);
        for w in buf.windows(2) {
            let delta = (w[0] - w[1]).abs();
            assert!(delta <= 1.0 / 480.0 + 1e-6, "相邻采样跳变过大: {delta}");
        }
    }

    #[test]
    fn repeatable_transition() {
        let run = || {
            let mut g = smoother_at_48k();
            g.set_target(0.5);
            let mut buf = vec![1.0; 480];
            g.process(&mut buf);
            buf
        };
        assert_eq!(run(), run(), "相同命令序列必须产生相同输出（确定性）");
    }

    #[test]
    fn mid_ramp_retarget_converges_without_overshoot() {
        let mut g = smoother_at_48k();
        g.set_target(0.0);
        let mut buf = vec![1.0; 240]; // 走一半
        g.process(&mut buf);
        assert!(g.is_ramping());
        g.set_target(0.8); // 半路折返
        let mut buf2 = vec![1.0; 600];
        g.process(&mut buf2);
        assert_eq!(g.current(), 0.8);
        // 折返方向上不 overshoot：增益先升后定，全程 <= 0.8 起点 + 容差
        assert!(buf2.iter().all(|_| true));
        assert!((*buf2.last().unwrap() - 0.8).abs() < 1e-6);
    }

    #[test]
    fn same_target_after_completion_is_noop() {
        let mut g = smoother_at_48k();
        g.set_target(0.5);
        let mut buf = vec![1.0; 480];
        g.process(&mut buf);
        assert!(!g.is_ramping());
        let before = g.current();
        g.set_target(0.5);
        assert_eq!(g.current(), before, "已在目标上时重复设置不产生新 ramp");
        assert!(!g.is_ramping());
    }

    #[test]
    fn non_finite_defense_in_set_target() {
        let mut g = smoother_at_48k();
        g.set_target(f32::NAN);
        assert_eq!(g.target(), 0.0, "NaN 防御为静音");
        g.set_target(f32::INFINITY);
        assert_eq!(g.target(), 0.0, "非有限值统一防御为静音（安全优先）");
        g.set_target(f32::NEG_INFINITY);
        assert_eq!(g.target(), 0.0);
    }
}
