//! Versioned generator（§108/§109）：seed + generation_version + genome +
//! environment → SceneManifest。
//!
//! 契约（§109）：输入输出均为纯数据；无 IO、无时钟、无全局状态。
//! 未来 GeneratorV2 并行共存，旧世界仍用旧规则打开（§106/§107）。

use crate::biome::Biome;
use crate::env::EnvironmentState;
use crate::genome::Genome;
use crate::manifest::SceneManifest;
use crate::seed::{world_identity, DreamscapeSeed};
use crate::world::World;

/// 生成分代（§108）。
pub const SUPPORTED_GENERATION_VERSION: u16 = 1;

/// 生成结果（逻辑态组合）。
#[derive(Debug, Clone, PartialEq)]
pub struct GenerationOutput {
    pub world_id: u64,
    pub world: World,
    pub manifest: SceneManifest,
}

/// V1 生成器（§108）。
pub struct GeneratorV1;

impl GeneratorV1 {
    pub const VERSION: u16 = 1;

    /// 生成（§109 契约）。
    #[must_use]
    pub fn generate(
        seed: &DreamscapeSeed,
        genome: &Genome,
        env: &EnvironmentState,
    ) -> GenerationOutput {
        let biome = Biome::select(seed, genome);
        let world_id = world_identity(seed, Self::VERSION, biome.kind.id().as_bytes());
        let world = World::generate(seed, genome, &biome);
        let manifest =
            SceneManifest::from_world(world_id, &world, env, biome.effective_mist_bias());
        GenerationOutput {
            world_id,
            world,
            manifest,
        }
    }
}

/// 种子输入解析结果（§105/§106：明确错误，不静默换世界）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedParseError {
    /// 非 DS- 人类格式且无法当作任意文本（不会发生——任意文本可 hash；
    /// 保留枚举位以便 UI 区分消息）。
    Invalid,
    /// generation version 超出本构建支持（§106：显示 World version
    /// unavailable 而非静默改世界）。
    UnsupportedGeneration(u16),
}

/// 打开世界：校验 generation version 兼容性（§106）。
#[must_use]
pub const fn generation_supported(v: u16) -> bool {
    v == SUPPORTED_GENERATION_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_deterministic() {
        let seed = DreamscapeSeed::from_input("gen");
        let genome = Genome::from_seed(&seed);
        let env = EnvironmentState::initial(&seed, 100);
        let a = GeneratorV1::generate(&seed, &genome, &env);
        let b = GeneratorV1::generate(&seed, &genome, &env);
        assert_eq!(a, b, "§109：同输入同输出");
        assert_eq!(a.manifest.scene_hash(), b.manifest.scene_hash());
        assert_eq!(a.world_id, b.world_id);
    }

    #[test]
    fn version_pinned_and_gate() {
        assert_eq!(GeneratorV1::VERSION, SUPPORTED_GENERATION_VERSION);
        assert!(generation_supported(1));
        assert!(
            !generation_supported(2),
            "§106：未知代 → 上层显示 unavailable"
        );
        assert_eq!(
            SeedParseError::UnsupportedGeneration(2),
            SeedParseError::UnsupportedGeneration(2)
        );
    }

    #[test]
    fn world_id_matches_components() {
        let seed = DreamscapeSeed::from_input("gen-id");
        let genome = Genome::from_seed(&seed);
        let biome = Biome::select(&seed, &genome);
        let env = EnvironmentState::initial(&seed, 0);
        let out = GeneratorV1::generate(&seed, &genome, &env);
        assert_eq!(
            out.world_id,
            world_identity(&seed, 1, biome.kind.id().as_bytes())
        );
    }

    use crate::env::EnvironmentState;
}
