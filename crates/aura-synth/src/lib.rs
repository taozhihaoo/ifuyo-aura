//! ifuyo Aura Dreamscape 参数生成核心（M7 上篇，§0–127）。
//!
//! 职责边界（§7）：Seed → Genome → Environment → 光场参数（LightField）。
//! 不含 Three.js/WebGL/React/Tauri/audio callback/SQLite/HTTP。
//!
//! v2 起**不再做任何 3D 世界建模**（无地形高度场/无物体放置）：Dreamscape
//! 改为音乐驱动的极光光场，本 crate 只负责"这场光长什么样"的确定性参数。
//!
//! 确定性合同（§6/§22/§49/§114）：同 seed + generation_version +
//! environment 状态 → 逐位一致的输出（RNG 序列、manifest、scene_hash）。
//! 世界逻辑只接受逻辑时间（LogicalTick，§30），不读取 wall-clock。

pub mod biome;
pub mod env;
pub mod events;
pub mod generator;
pub mod genome;
pub mod manifest;
pub mod rng;
pub mod seed;
pub mod snapshot;

/// seed 语义版本（见 [`seed::SEED_VERSION`]）。
#[must_use]
pub const fn seed_version() -> u16 {
    seed::SEED_VERSION
}

/// 生成算法版本（GeneratorV1，§108）。
#[must_use]
pub const fn generation_version() -> u16 {
    seed::GENERATION_VERSION
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions_pinned() {
        // §16：版本必须显式固定；升级需走显式决策。
        assert_eq!(super::seed_version(), 1);
        assert_eq!(super::generation_version(), 1);
    }
}
