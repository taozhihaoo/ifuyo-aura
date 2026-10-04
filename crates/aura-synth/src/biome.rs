//! Biome 原型（§43–§46）：不只是配色——地形轮廓 / 物件族 / 密度轮廓 /
//! 氛围 / palette 锚 / 光照倾向（§44）。
//!
//! - 5 个原型（§43），命名克制。
//! - 选择由 seed → genome 决定（§45），不随打开次数变化。
//! - 同 biome 不同 seed 产生结构差异（§46：`variation` 给出 per-seed
//!   密度/幅度调制，世界层消费）。

use serde::{Deserialize, Serialize};

use crate::genome::Genome;
use crate::rng::{streams, Rng};
use crate::seed::{stable_hash64, DreamscapeSeed};

/// 物件族（§57：族集合由 biome 决定；几何形态在下篇渲染层）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectFamily {
    Flora,
    Stone,
    Water,
    Light,
    Particle,
    Landmark,
}

/// 光照倾向（§44）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightingTendency {
    /// 柔和漫射。
    Soft,
    /// 辉光偏重（夜里明显）。
    Glow,
    /// 暖金倾向。
    Amber,
}

/// Biome 原型（§44 六要素）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiomeKind {
    /// 宁静草甸。
    QuietMeadow,
    /// 雾林。
    MistyForest,
    /// 梦岸。
    DreamShore,
    /// 月园。
    MoonGarden,
    /// 琥珀谷。
    AmberValley,
}

impl BiomeKind {
    /// 稳定 id（palette 锚 / world_id 初始环境串使用）。
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::QuietMeadow => "quiet-meadow",
            Self::MistyForest => "misty-forest",
            Self::DreamShore => "dream-shore",
            Self::MoonGarden => "moon-garden",
            Self::AmberValley => "amber-valley",
        }
    }

    /// 地形轮廓：振幅 / 频率（§44 terrain profile；世界层据此生成）。
    #[must_use]
    pub const fn terrain_profile(self) -> (f32, f32) {
        match self {
            Self::QuietMeadow => (0.35, 0.8),
            Self::MistyForest => (0.6, 1.2),
            Self::DreamShore => (0.25, 0.6),
            Self::MoonGarden => (0.45, 1.0),
            Self::AmberValley => (0.85, 0.7),
        }
    }

    /// 物件族集合（§57）。
    #[must_use]
    pub const fn object_families(self) -> &'static [ObjectFamily] {
        match self {
            Self::QuietMeadow => &[
                ObjectFamily::Flora,
                ObjectFamily::Stone,
                ObjectFamily::Particle,
                ObjectFamily::Landmark,
            ],
            Self::MistyForest => &[
                ObjectFamily::Flora,
                ObjectFamily::Stone,
                ObjectFamily::Particle,
                ObjectFamily::Landmark,
            ],
            Self::DreamShore => &[
                ObjectFamily::Water,
                ObjectFamily::Stone,
                ObjectFamily::Light,
                ObjectFamily::Landmark,
            ],
            Self::MoonGarden => &[
                ObjectFamily::Flora,
                ObjectFamily::Light,
                ObjectFamily::Particle,
                ObjectFamily::Landmark,
            ],
            Self::AmberValley => &[
                ObjectFamily::Flora,
                ObjectFamily::Stone,
                ObjectFamily::Light,
                ObjectFamily::Landmark,
            ],
        }
    }

    /// 密度轮廓缩放（§44 density profile；世界预算内生效）。
    #[must_use]
    pub const fn density_scale(self) -> f32 {
        match self {
            Self::QuietMeadow => 0.8,
            Self::MistyForest => 1.2,
            Self::DreamShore => 0.6,
            Self::MoonGarden => 0.9,
            Self::AmberValley => 1.0,
        }
    }

    /// 氛围偏置（mist 追加；§44 atmosphere）。
    #[must_use]
    pub const fn mist_bias(self) -> f32 {
        match self {
            Self::QuietMeadow => 0.0,
            Self::MistyForest => 0.25,
            Self::DreamShore => 0.1,
            Self::MoonGarden => 0.15,
            Self::AmberValley => 0.05,
        }
    }

    /// 光照倾向（§44）。
    #[must_use]
    pub const fn lighting(self) -> LightingTendency {
        match self {
            Self::QuietMeadow => LightingTendency::Soft,
            Self::MistyForest => LightingTendency::Soft,
            Self::DreamShore => LightingTendency::Glow,
            Self::MoonGarden => LightingTendency::Glow,
            Self::AmberValley => LightingTendency::Amber,
        }
    }
}

/// 选中结果 = 原型 + per-seed 变异（§46）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Biome {
    pub kind: BiomeKind,
    /// 变异：密度调制 [0.7,1.3]（同 biome 不同 seed 结构不同）。
    pub density_variation: f32,
    /// 变异：地形振幅调制 [0.7,1.3]。
    pub terrain_variation: f32,
    /// 变异：氛围调制 [0.7,1.3]。
    pub atmosphere_variation: f32,
}

impl Biome {
    /// seed → genome → biome（§45）。确定性；genome 倾向主导，world 流
    /// 仅作同倾向下的稳定决胜（非随机换 biome）。
    #[must_use]
    pub fn select(seed: &DreamscapeSeed, genome: &Genome) -> Self {
        // genome 倾向打分（§45：不能每次打开随机换）。
        let scores = [
            (BiomeKind::QuietMeadow, 1.0),
            (
                BiomeKind::MistyForest,
                0.6 + genome.flora_density + genome.mist_density * 0.5,
            ),
            (BiomeKind::DreamShore, 0.4 + genome.water_presence * 1.6),
            (
                BiomeKind::MoonGarden,
                0.5 + genome.light_softness * 0.8 + genome.mist_density * 0.4,
            ),
            (
                BiomeKind::AmberValley,
                0.5 + (genome.color_temperature + 1.0) * 0.7,
            ),
        ];
        let mut best = scores[0];
        for s in &scores[1..] {
            if s.1 > best.1 {
                best = *s;
            }
        }
        // 同分稳定决胜（world 流，非每次打开变化）。
        let mut tie = scores
            .iter()
            .filter(|s| (s.1 - best.1).abs() < f32::EPSILON);
        if tie.next().is_some() && tie.next().is_some() {
            let mut r = Rng::from_stream(seed, streams::WORLD);
            best = scores[(r.next_u64() as usize) % scores.len()];
        }
        Self::with_variation(seed, best.0)
    }

    /// per-seed 变异（§46）：三轴独立调制。
    #[must_use]
    pub fn with_variation(seed: &DreamscapeSeed, kind: BiomeKind) -> Self {
        let mut v = Rng::from_stream_bucket(seed, streams::WORLD, 0xA11CE);
        Self {
            kind,
            density_variation: v.next_f32_range(0.7, 1.3),
            terrain_variation: v.next_f32_range(0.7, 1.3),
            atmosphere_variation: v.next_f32_range(0.7, 1.3),
        }
    }

    /// 变异后的有效密度缩放（预算内消费，§64）。
    #[must_use]
    pub fn effective_density(self) -> f32 {
        self.kind.density_scale() * self.density_variation
    }

    /// 变异后的有效雾偏置（env fog 约束仍在上层生效，§39）。
    #[must_use]
    pub fn effective_mist_bias(self) -> f32 {
        (self.kind.mist_bias() * self.atmosphere_variation).min(0.35)
    }

    /// biome 派生锚（palette/区域生成用；§41/§118）。
    #[must_use]
    pub fn anchor(self, seed: &DreamscapeSeed) -> u64 {
        stable_hash64(b"biome-anchor/v1", self.kind.id().as_bytes()) ^ seed.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_archetypes_complete() {
        // §43 五个；§44 六要素齐备（编译期常量函数保证，此处运行期校验）。
        let all = [
            BiomeKind::QuietMeadow,
            BiomeKind::MistyForest,
            BiomeKind::DreamShore,
            BiomeKind::MoonGarden,
            BiomeKind::AmberValley,
        ];
        for k in all {
            let (amp, freq) = k.terrain_profile();
            assert!(amp > 0.0 && freq > 0.0);
            assert!(!k.object_families().is_empty());
            assert!(
                k.object_families().contains(&ObjectFamily::Landmark),
                "{k:?} 需可放 landmark"
            );
            assert!(k.density_scale() > 0.0);
            assert!((0.0..=0.4).contains(&k.mist_bias()));
            let _ = k.lighting();
            assert!(!k.id().is_empty());
        }
        // id 唯一。
        let ids: Vec<_> = all.iter().map(|k| k.id()).collect();
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn selection_deterministic_and_genome_coupled() {
        let g = Genome::from_seed(&DreamscapeSeed::from_input("biome"));
        let a = Biome::select(&DreamscapeSeed::from_input("biome"), &g);
        let b = Biome::select(&DreamscapeSeed::from_input("biome"), &g);
        assert_eq!(a, b, "§45 同 seed+genome 不变");

        // 高 water_presence 世界倾向 DreamShore（多 seed 统计）。
        let mut shore = 0;
        for i in 0..40u64 {
            let seed = DreamscapeSeed::from_input(&format!("shore{i}"));
            let mut g = Genome::default_genome();
            g.set(GenomeField::WaterPresence, 1.0);
            let _ = seed;
            if Biome::select(&seed, &g).kind == BiomeKind::DreamShore {
                shore += 1;
            }
        }
        assert!(shore >= 36, "water 主导应稳定选中 shore：{shore}/40");

        // 暖色温倾向 AmberValley。
        let mut amber = 0;
        for i in 0..40u64 {
            let seed = DreamscapeSeed::from_input(&format!("amber{i}"));
            let mut g = Genome::default_genome();
            g.set(GenomeField::ColorTemperature, 1.0);
            g.set(GenomeField::WaterPresence, 0.0);
            if Biome::select(&seed, &g).kind == BiomeKind::AmberValley {
                amber += 1;
            }
        }
        assert!(amber >= 36, "暖色温主导应稳定选中 amber：{amber}/40");

        let _ = GenomeField::FloraDensity;
    }

    use crate::genome::GenomeField;

    #[test]
    fn same_biome_different_seed_varies_structure() {
        // §46：同为 MistyForest，不同 seed → 密度/地形/氛围调制不同。
        let mut seen = Vec::new();
        for i in 0..6u64 {
            let seed = DreamscapeSeed::from_input(&format!("forest{i}"));
            let b = Biome::with_variation(&seed, BiomeKind::MistyForest);
            seen.push((
                b.density_variation,
                b.terrain_variation,
                b.atmosphere_variation,
            ));
            for v in [
                b.density_variation,
                b.terrain_variation,
                b.atmosphere_variation,
            ] {
                assert!((0.7..=1.3).contains(&v));
            }
        }
        let mut distinct = 0;
        for (i, a) in seen.iter().enumerate() {
            for b in &seen[i + 1..] {
                if (a.0 - b.0).abs() > 1e-4 || (a.1 - b.1).abs() > 1e-4 {
                    distinct += 1;
                }
            }
        }
        assert!(distinct >= 10, "不同 seed 应产生不同结构（§46）：{seen:?}");
    }

    #[test]
    fn effective_values_bounded() {
        let seed = DreamscapeSeed::from_input("eff");
        for i in 0..50u64 {
            let s = DreamscapeSeed::from_input(&format!("eff{i}"));
            let b = Biome::select(&s, &Genome::from_seed(&s));
            assert!(b.effective_density() > 0.0);
            assert!(b.effective_density() <= 1.3 * 1.3);
            assert!(b.effective_mist_bias() <= 0.35);
            assert_ne!(b.anchor(&seed), 0);
        }
    }

    #[test]
    fn biome_serde_roundtrip() {
        let seed = DreamscapeSeed::from_input("bser");
        let b = Biome::select(&seed, &Genome::from_seed(&seed));
        let json = serde_json::to_string(&b).unwrap();
        let back: Biome = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);
    }
}
