//! 确定性 RNG（§19–§22）：命名流 + 流隔离。
//!
//! - 全局无随机状态；一切流由 `master_seed + 流名` 经固定 hash 派生（§21，
//!   禁止 `master_seed + index` 式裸偏移）。
//! - 流隔离（§20）：消耗某流（如 vegetation）不影响其他流（terrain）的
//!   序列；调参只影响目标系统。
//! - 核心为 SplitMix64（全周期、无依赖），序列由初始状态唯一决定。

use crate::seed::{stable_hash64, DreamscapeSeed};

/// 独立命名 RNG 流。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rng {
    state: u64,
}

/// 标准流名（§19 建议集合；新增流名 = 追加，不改既有）。
pub mod streams {
    pub const WORLD: &str = "world";
    pub const TERRAIN: &str = "terrain";
    pub const VEGETATION: &str = "vegetation";
    pub const WEATHER: &str = "weather";
    pub const EVENT: &str = "event";
    pub const MUTATION: &str = "mutation";
    pub const PLACEMENT: &str = "placement";
    pub const PALETTE: &str = "palette";
}

impl Rng {
    /// 从 master seed 与流名派生（§21）。
    #[must_use]
    pub fn from_stream(master: &DreamscapeSeed, stream: &str) -> Self {
        let mut data = Vec::with_capacity(10 + stream.len());
        data.extend_from_slice(&master.value.to_le_bytes());
        data.extend_from_slice(&master.seed_version.to_le_bytes());
        data.extend_from_slice(stream.as_bytes());
        // 状态避开 0（SplitMix64 对 0 也能运行，但避开使「未播种」可检测）。
        let state = stable_hash64(b"rng-stream/v1", &data);
        Self {
            state: if state == 0 { 1 } else { state },
        }
    }

    /// 下一个 u64（SplitMix64）。
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// [0, 1) 的 f32（24 bit 尾数精度）。
    #[must_use]
    pub fn next_f32_unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// [min, max) 的 f32。
    #[must_use]
    pub fn next_f32_range(&mut self, min: f32, max: f32) -> f32 {
        debug_assert!(max > min);
        min + self.next_f32_unit() * (max - min)
    }

    /// [min, max] 的 i64（均匀，拒绝采样）。
    #[must_use]
    pub fn next_i64_range(&mut self, min: i64, max: i64) -> i64 {
        debug_assert!(max >= min);
        let span = (max - min + 1) as u64;
        let zone = u64::MAX - (u64::MAX % span) - 1;
        loop {
            let v = self.next_u64();
            if v <= zone {
                return min + (v % span) as i64;
            }
        }
    }

    /// 概率事件（§90 稀有度用）：`p ∈ [0,1]`。
    #[must_use]
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32_unit() < p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 序列锚点（首两个 u64；算法或派生规则变更会打破——有意为之，
    /// 更新需记录 DECISIONS）。
    const ANCHOR_A: u64 = 0x8698_a6fb_381f_ad07;
    const ANCHOR_B: u64 = 0x2c9c_b0ba_606c_2480;

    fn master(tag: &str) -> DreamscapeSeed {
        DreamscapeSeed::from_input(tag)
    }

    #[test]
    fn deterministic_same_seed_same_stream() {
        let a: Vec<u64> = {
            let mut r = Rng::from_stream(&master("dstest"), streams::TERRAIN);
            (0..32).map(|_| r.next_u64()).collect()
        };
        let b: Vec<u64> = {
            let mut r = Rng::from_stream(&master("dstest"), streams::TERRAIN);
            (0..32).map(|_| r.next_u64()).collect()
        };
        assert_eq!(a, b, "同 seed+同流 → 逐位一致（§22）");
    }

    #[test]
    fn streams_are_independent_and_isolated() {
        // 不同流名 → 不同序列。
        let t: Vec<u64> = {
            let mut r = Rng::from_stream(&master("dstest"), streams::TERRAIN);
            (0..8).map(|_| r.next_u64()).collect()
        };
        let v: Vec<u64> = {
            let mut r = Rng::from_stream(&master("dstest"), streams::VEGETATION);
            (0..8).map(|_| r.next_u64()).collect()
        };
        assert_ne!(t, v);
        // 流隔离（§20）：消耗 vegetation 不改变 terrain 序列。
        let mut terrain = Rng::from_stream(&master("dstest"), streams::TERRAIN);
        let baseline: Vec<u64> = (0..8).map(|_| terrain.next_u64()).collect();
        let mut vegetation = Rng::from_stream(&master("dstest"), streams::VEGETATION);
        let mut consumed = 0u64;
        for _ in 0..1000 {
            consumed = consumed.wrapping_add(vegetation.next_u64());
        }
        assert_ne!(consumed, 0, "vegetation 流确实被消费");

        let mut terrain2 = Rng::from_stream(&master("dstest"), streams::TERRAIN);
        let after: Vec<u64> = (0..8).map(|_| terrain2.next_u64()).collect();
        assert_eq!(baseline, after);
    }

    #[test]
    fn stream_derivation_not_index_offset() {
        // §21：流名相邻 ≠ 序列相邻/线性相关。
        let a = Rng::from_stream(&master("x"), "s1").next_u64();
        let b = Rng::from_stream(&master("x"), "s2").next_u64();
        assert_ne!(a.wrapping_sub(b), 1);
        assert_ne!(a, b);
    }

    #[test]
    fn different_masters_vary_reasonably() {
        let firsts: HashSet<u64> = (0..200)
            .map(|i| Rng::from_stream(&master(&format!("m{i}")), streams::WORLD).next_u64())
            .collect();
        assert_eq!(firsts.len(), 200, "不同 master 应合理变化（§22）");
    }

    #[test]
    fn sequence_anchor_pins_algorithm() {
        // 算法/派生规则变更会打破锚点——有意为之（跨版本可检测，§17 精神）。
        let mut r = Rng::from_stream(&DreamscapeSeed::from_value(0x1234_5678), streams::WORLD);
        let a = r.next_u64();
        let b = r.next_u64();
        assert_eq!(
            (a, b),
            (ANCHOR_A, ANCHOR_B),
            "算法/派生规则被改动——若为有意升级，更新锚点并记录 DECISIONS"
        );
    }

    #[test]
    fn helpers_bounded_and_deterministic() {
        let mut r = Rng::from_stream(&master("bounds"), streams::EVENT);
        for _ in 0..1000 {
            let f = r.next_f32_range(2.0, 5.0);
            assert!((2.0..5.0).contains(&f));
            let i = r.next_i64_range(-7, 7);
            assert!((-7..=7).contains(&i));
        }
        let mut r2 = Rng::from_stream(&master("bounds"), streams::EVENT);
        assert_eq!(
            r2.next_f32_range(2.0, 5.0),
            Rng::from_stream(&master("bounds"), streams::EVENT).next_f32_range(2.0, 5.0)
        );
    }

    use std::collections::HashSet;
}
