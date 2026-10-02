//! 确定性虚拟时钟：以纳秒为单位的单调虚拟时间，替代 wall-clock。
//!
//! 规格依据（Charter §8.4 / 各里程碑确定性要求）：任何测试与确定性逻辑
//! 不得依赖 `SystemTime` / `Instant`；时间由测试显式推进。

/// 单调虚拟时钟，初始为 0。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualClock {
    nanos: u64,
}

impl VirtualClock {
    #[must_use]
    pub const fn new() -> Self {
        Self { nanos: 0 }
    }

    /// 当前虚拟时间（纳秒）。
    #[must_use]
    pub const fn nanos(&self) -> u64 {
        self.nanos
    }

    /// 显式推进时钟；时间只前进，不允许回退或暂停语义外的操作。
    pub fn advance_nanos(&mut self, nanos: u64) {
        self.nanos = self.nanos.saturating_add(nanos);
    }

    /// 按采样数推进（采样率换算，四舍五入到纳秒）。
    pub fn advance_frames(&mut self, frames: u64, sample_rate: u32) {
        let nanos = (frames * 1_000_000_000) / u64::from(sample_rate);
        self.advance_nanos(nanos);
    }
}

impl Default for VirtualClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero_and_advances_deterministically() {
        let mut a = VirtualClock::new();
        let mut b = VirtualClock::default();
        a.advance_nanos(1_500);
        b.advance_nanos(1_500);
        assert_eq!(a.nanos(), 1_500);
        assert_eq!(a, b, "相同操作序列必须产生相同时间（确定性）");
    }

    #[test]
    fn advance_frames_uses_sample_rate_math() {
        let mut clock = VirtualClock::new();
        clock.advance_frames(48_000, 48_000);
        assert_eq!(clock.nanos(), 1_000_000_000);
        clock.advance_frames(2_400, 48_000);
        assert_eq!(clock.nanos(), 1_050_000_000);
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        let mut clock = VirtualClock::new();
        clock.advance_nanos(u64::MAX);
        clock.advance_nanos(1);
        assert_eq!(clock.nanos(), u64::MAX);
    }
}
