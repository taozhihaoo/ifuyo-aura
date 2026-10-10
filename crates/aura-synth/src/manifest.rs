//! 光场清单（v2）：Dreamscape 的渲染契约——**纯参数，无几何**。
//!
//! v1 是 3D 世界（terrain 高度场 + landmark + instance 数组）；v2 起
//! Dreamscape 改为音乐驱动的极光/星野光场，因此几何全部撤除。
//!
//! 关键约束：
//! - `Genome` 的持久化字段**一个都不删**，而是 1:1 重映射为光场参数
//!   （`FieldParams`）——删字段会让 `payload_hash` 失配，旧
//!   `dreamscape.json` 被判 Corrupt 并静默回退默认世界。
//! - 不含 Three.js/WebGL 概念（§111 精神延续）：只有数值与枚举。
//! - 确定性：同 seed + genome + environment → 同 `scene_hash`（§114）。

use serde::{Deserialize, Serialize};

use crate::biome::Biome;
use crate::env::{EnvironmentState, Palette};
use crate::genome::Genome;
use crate::seed::{stable_hash64, DreamscapeSeed};

/// v2：几何字段全部移除，manifest 语义从"场景"变为"光场"。
pub const MANIFEST_VERSION: u16 = 2;

/// 视觉母题（取代 v1 的 landmark 种类；决定极光幕的主导形态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldMotif {
    /// 垂落帷幕
    Veil,
    /// 倾泻光瀑
    Cascade,
    /// 放射光晕
    Radiance,
    /// 缓慢漂移的薄雾带
    Drift,
}

impl FieldMotif {
    const ALL: [Self; 4] = [Self::Veil, Self::Cascade, Self::Radiance, Self::Drift];

    /// seed → 母题（确定性，与 genome/biome 无关，便于"同 seed 同画面"）。
    #[must_use]
    pub fn from_seed(seed: &DreamscapeSeed) -> Self {
        let h = stable_hash64(b"field-motif/v2", &seed.value.to_le_bytes());
        Self::ALL[usize::try_from(h % Self::ALL.len() as u64).unwrap_or(0)]
    }
}

/// 抽象光态（§40 透出；渲染层映射为曝光/辉光强度）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightState {
    pub sun_intensity: f32,
    pub ambient_intensity: f32,
    pub horizon_glow: f32,
}

/// 大气状态。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtmosphereState {
    pub fog: f32,
    pub wind: f32,
    /// biome 氛围偏置（§44）。
    pub mist_bias: f32,
}

/// 光场形态参数（genome 语义重映射；全部归一化到 `[0,1]`）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldParams {
    /// 幕帘纵深与层叠幅度 ← `terrain_scale`
    pub curtain_depth: f32,
    /// 光尘密度 ← `flora_density`
    pub mote_density: f32,
    /// 雾幕不透明度与柔化 ← `mist_density`
    pub veil: f32,
    /// 辉光扩散半径 ← `light_softness`
    pub glow_softness: f32,
    /// 地平镜面强度 ← `water_presence`
    pub mirror: f32,
    /// 星野尺度与疏密 ← `object_scale_bias`
    pub star_scale: f32,
    /// 地平线在画面中的高度 ← `horizon_height`
    pub horizon: f32,
}

impl FieldParams {
    #[must_use]
    pub fn from_genome(genome: &Genome) -> Self {
        Self {
            curtain_depth: genome.terrain_scale,
            mote_density: genome.flora_density,
            veil: genome.mist_density,
            glow_softness: genome.light_softness,
            mirror: genome.water_presence,
            star_scale: genome.object_scale_bias,
            horizon: genome.horizon_height,
        }
    }

    #[must_use]
    pub fn as_scalars(&self) -> [f32; 7] {
        [
            self.curtain_depth,
            self.mote_density,
            self.veil,
            self.glow_softness,
            self.mirror,
            self.star_scale,
            self.horizon,
        ]
    }
}

/// 光场清单（渲染层唯一输入）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightField {
    pub manifest_version: u16,
    /// 世界稳定身份（§18；仍绑定 biome id，旧快照才不失效）。
    pub world_id: u64,
    pub seed: DreamscapeSeed,
    /// 场噪声种子（取代 v1 `terrain.terrain_seed`）。
    pub field_seed: u64,
    pub motif: FieldMotif,
    pub palette: Palette,
    pub light: LightState,
    pub atmosphere: AtmosphereState,
    pub params: FieldParams,
}

impl LightField {
    /// seed + genome + biome + environment → 光场（§109 契约不变）。
    #[must_use]
    pub fn new(
        world_id: u64,
        seed: &DreamscapeSeed,
        genome: &Genome,
        biome: &Biome,
        env: &EnvironmentState,
    ) -> Self {
        Self {
            manifest_version: MANIFEST_VERSION,
            world_id,
            seed: *seed,
            field_seed: stable_hash64(b"field-seed/v2", &seed.value.to_le_bytes()),
            motif: FieldMotif::from_seed(seed),
            palette: Palette::derive(seed, biome.kind.id(), env),
            light: LightState {
                sun_intensity: env.sun_intensity,
                ambient_intensity: env.ambient_intensity,
                horizon_glow: env.horizon_glow,
            },
            atmosphere: AtmosphereState {
                fog: env.fog,
                wind: env.wind,
                mist_bias: biome.effective_mist_bias(),
            },
            params: FieldParams::from_genome(genome),
        }
    }

    /// 光场稳定 hash（域分离的规范字节序；§113/§114）。
    #[must_use]
    pub fn scene_hash(&self) -> u64 {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.manifest_version.to_le_bytes());
        buf.extend_from_slice(&self.world_id.to_le_bytes());
        buf.extend_from_slice(&self.seed.value.to_le_bytes());
        buf.extend_from_slice(&self.field_seed.to_le_bytes());
        buf.extend_from_slice(&self.motif_discriminator().to_le_bytes());
        for v in [
            self.light.sun_intensity,
            self.light.ambient_intensity,
            self.light.horizon_glow,
            self.atmosphere.fog,
            self.atmosphere.wind,
            self.atmosphere.mist_bias,
            self.palette.base_hue,
            self.palette.accent_hue,
            self.palette.saturation,
            self.palette.luminance,
            self.palette.accent_frequency,
        ]
        .into_iter()
        .chain(self.params.as_scalars())
        {
            buf.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        stable_hash64(b"field-hash/v2", &buf)
    }

    /// 母题的可哈希判别值（避免依赖 `as u32` 对带判别枚举的不稳定布局）。
    #[must_use]
    const fn motif_discriminator(&self) -> u32 {
        match self.motif {
            FieldMotif::Veil => 0,
            FieldMotif::Cascade => 1,
            FieldMotif::Radiance => 2,
            FieldMotif::Drift => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::Genome;

    fn field_for(tag: &str, tick: u64) -> LightField {
        let seed = DreamscapeSeed::from_input(tag);
        let genome = Genome::from_seed(&seed);
        let biome = Biome::select(&seed, &genome);
        let env = EnvironmentState::initial(&seed, tick);
        let world_id = crate::seed::world_identity(
            &seed,
            crate::generation_version(),
            biome.kind.id().as_bytes(),
        );
        LightField::new(world_id, &seed, &genome, &biome, &env)
    }

    #[test]
    fn scene_hash_deterministic_same_inputs() {
        let a = field_for("hash-det", 1000);
        let b = field_for("hash-det", 1000);
        assert_eq!(a, b);
        assert_eq!(a.scene_hash(), b.scene_hash());
    }

    #[test]
    fn scene_hash_changes_with_inputs() {
        let base = field_for("hash-in", 1000);
        assert_ne!(base.scene_hash(), field_for("hash-in-2", 1000).scene_hash());
        // 半日偏移 → 相位不同 → 光态不同（整日循环相位相同是正确行为）。
        assert_ne!(
            base.scene_hash(),
            field_for("hash-in", 1000 + 24 * 60 / 2).scene_hash()
        );
        let mut moved = base.clone();
        moved.world_id ^= 1;
        assert_ne!(base.scene_hash(), moved.scene_hash());
        let mut motif_changed = base.clone();
        motif_changed.motif = if base.motif == FieldMotif::Veil {
            FieldMotif::Drift
        } else {
            FieldMotif::Veil
        };
        assert_ne!(
            base.scene_hash(),
            motif_changed.scene_hash(),
            "母题变化 → 光场变化"
        );
    }

    #[test]
    fn params_mirror_genome() {
        // genome 是唯一形态来源：不同 seed → 参数不同，且全部有限。
        let a = field_for("params-a", 0);
        let b = field_for("params-b", 0);
        let g = Genome::from_seed(&DreamscapeSeed::from_input("params-a"));
        assert_eq!(a.params, FieldParams::from_genome(&g));
        for v in a.params.as_scalars() {
            assert!(v.is_finite(), "光场参数必须有限");
        }
        assert_ne!(a.params.as_scalars(), b.params.as_scalars());
    }

    #[test]
    fn serde_roundtrip_and_no_renderer_leak() {
        // §111：纯数值 DTO；serde 往返一致，hash 不变。
        let m = field_for("ser", 500);
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("THREE"), "无渲染器类型泄漏");
        assert!(!json.contains("landmarks"), "几何字段已撤除");
        assert!(!json.contains("instances"), "几何字段已撤除");
        assert!(json.len() < 4096, "光场清单必须极小：{} bytes", json.len());
        let back: LightField = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.scene_hash(), m.scene_hash());
    }

    #[test]
    fn motif_is_stable_per_seed() {
        for tag in ["m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8"] {
            let seed = DreamscapeSeed::from_input(tag);
            assert_eq!(FieldMotif::from_seed(&seed), FieldMotif::from_seed(&seed));
        }
    }

    const _: () = {
        const fn assert_send_sync<T: Send + Sync>() {}
        let _ = assert_send_sync::<LightField>;
    };
}
