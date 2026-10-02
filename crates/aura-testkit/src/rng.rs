//! 固定种子确定性 RNG：SplitMix64。
//!
//! 规格依据（Charter：固定随机种子）：测试与确定性逻辑禁止使用系统随机源；
//! 同种子必须产生相同序列。SplitMix64 体积小、统计性足够测试使用；
//! M7 Dreamscape 的生产级 RNG 体系（命名 stream 派生）在 aura-synth 内独立实现，
//! 不复用本模块。

/// SplitMix64 伪随机数生成器。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// 产生下一个 u64（SplitMix64，全周期覆盖）。
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 产生 [0, 1) 的 f32（24 bit 尾数精度）。
    #[must_use]
    pub fn next_f32_unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// 产生 [min, max) 的 i64。
    #[must_use]
    pub fn next_i64_range(&mut self, min: i64, max: i64) -> i64 {
        debug_assert!(max > min);
        let span = (max - min) as u64;
        min + (self.next_u64() % span) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = DeterministicRng::new(42);
        let mut b = DeterministicRng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seed_different_sequence() {
        let mut a = DeterministicRng::new(1);
        let mut b = DeterministicRng::new(2);
        let seq_a: Vec<u64> = (0..16).map(|_| a.next_u64()).collect();
        let seq_b: Vec<u64> = (0..16).map(|_| b.next_u64()).collect();
        assert_ne!(seq_a, seq_b);
    }

    #[test]
    fn unit_floats_in_range() {
        let mut rng = DeterministicRng::new(7);
        for _ in 0..10_000 {
            let f = rng.next_f32_unit();
            assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn range_respects_bounds() {
        let mut rng = DeterministicRng::new(9);
        for _ in 0..10_000 {
            let v = rng.next_i64_range(-5, 5);
            assert!((-5..5).contains(&v));
        }
    }

    #[test]
    fn sequence_is_not_degenerate() {
        let mut rng = DeterministicRng::new(123);
        let seen: HashSet<u64> = (0..10_000).map(|_| rng.next_u64()).collect();
        assert!(seen.len() > 9_900, "随机序列不应大量重复");
    }
}
