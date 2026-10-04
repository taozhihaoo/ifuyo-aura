//! SceneManifest（§110–§115）：renderer-friendly 但 domain-neutral 的
//! 场景描述。
//!
//! - 不含任何 Three.js/WebGL 概念（§111）：只有数值与族枚举。
//! - 可序列化 + 可 hash（§112/§113）：`scene_hash` 用于确定性测试
//!   （同 seed+版本+genome+environment → hash 相等，§114）。
//! - 尺寸有界（§115）：对象数受世界预算约束，manifest 不会因 seed
//!   膨胀到 MB 级。

use serde::{Deserialize, Serialize};

use crate::biome::ObjectFamily;
use crate::env::{EnvironmentState, Palette};
use crate::seed::{stable_hash64, DreamscapeSeed};
use crate::world::{Landmark, LandmarkKind, PlacedObject, Terrain, World};

pub const MANIFEST_VERSION: u16 = 1;

/// 地形参数补丁（渲染层据此确定性重建高度场，§49 精神）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainPatch {
    pub amplitude: f32,
    pub frequency: f32,
    pub terrain_seed: u64,
}

/// 抽象光态（§40 → manifest 透出，渲染层映射 Three.js）。
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

/// landmark 节点。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LandmarkNode {
    pub id: u64,
    pub kind: LandmarkKind,
    pub x: f32,
    pub z: f32,
    pub y: f32,
    pub radius: f32,
    pub rotation: f32,
}

/// 实例组（SoA；渲染层 → InstancedMesh，§61）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceGroup {
    pub family: ObjectFamily,
    pub x: Vec<f32>,
    pub z: Vec<f32>,
    pub scale: Vec<f32>,
    pub rotation: Vec<f32>,
    pub nonce: Vec<u32>,
}

impl InstanceGroup {
    fn from_objects(family: ObjectFamily, objs: &[PlacedObject], terrain: &Terrain) -> Self {
        let mut g = Self {
            family,
            x: Vec::new(),
            z: Vec::new(),
            scale: Vec::new(),
            rotation: Vec::new(),
            nonce: Vec::new(),
        };
        for o in objs {
            if o.family == family {
                g.x.push(o.x);
                g.z.push(o.z);
                g.scale.push(o.scale);
                g.rotation.push(o.rotation);
                g.nonce.push(o.nonce);
            }
        }
        let _ = terrain;
        g
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.x.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }
}

/// 场景清单（§110）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneManifest {
    pub manifest_version: u16,
    /// 世界稳定身份（§18）。
    pub world_id: u64,
    pub seed: DreamscapeSeed,
    pub terrain: TerrainPatch,
    pub palette: Palette,
    pub light: LightState,
    pub atmosphere: AtmosphereState,
    /// landmark 数 ≤ MAX_LANDMARKS；实例组按族分组。
    pub landmarks: Vec<LandmarkNode>,
    pub instances: Vec<InstanceGroup>,
}

impl SceneManifest {
    /// 世界 + 环境 → manifest（§109 generator 输出）。
    #[must_use]
    pub fn from_world(
        world_id: u64,
        world: &World,
        env: &EnvironmentState,
        mist_bias: f32,
    ) -> Self {
        let mut families: Vec<ObjectFamily> =
            world.objects.iter().map(|o| o.family).collect::<Vec<_>>();
        families.sort();
        families.dedup();
        let landmark_nodes = world
            .landmarks
            .iter()
            .map(|l: &Landmark| LandmarkNode {
                id: l.id,
                kind: l.kind,
                x: l.x,
                z: l.z,
                y: world.terrain.height_at(l.x, l.z),
                radius: l.radius,
                rotation: l.rotation,
            })
            .collect();
        Self {
            manifest_version: MANIFEST_VERSION,
            world_id,
            seed: world.seed,
            terrain: TerrainPatch {
                amplitude: world.terrain.amplitude,
                frequency: world.terrain.frequency,
                terrain_seed: world.terrain.height_seed(),
            },
            palette: Palette::derive(&world.seed, world.biome.kind.id(), env),
            light: LightState {
                sun_intensity: env.sun_intensity,
                ambient_intensity: env.ambient_intensity,
                horizon_glow: env.horizon_glow,
            },
            atmosphere: AtmosphereState {
                fog: env.fog,
                wind: env.wind,
                mist_bias,
            },
            landmarks: landmark_nodes,
            instances: families
                .into_iter()
                .map(|f| InstanceGroup::from_objects(f, &world.objects, &world.terrain))
                .collect(),
        }
    }

    /// 场景稳定 hash（§113；域分离的规范字节序）。
    #[must_use]
    pub fn scene_hash(&self) -> u64 {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.manifest_version.to_le_bytes());
        buf.extend_from_slice(&self.world_id.to_le_bytes());
        buf.extend_from_slice(&self.seed.value.to_le_bytes());
        for v in [
            self.terrain.amplitude,
            self.terrain.frequency,
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
        ] {
            buf.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        buf.extend_from_slice(&self.terrain.terrain_seed.to_le_bytes());
        for l in &self.landmarks {
            buf.extend_from_slice(&l.id.to_le_bytes());
            for v in [l.x, l.z, l.y, l.radius, l.rotation] {
                buf.extend_from_slice(&v.to_bits().to_le_bytes());
            }
        }
        for g in &self.instances {
            buf.extend_from_slice(&(g.family as u32).to_le_bytes());
            for pair in g.x.iter().zip(&g.z) {
                buf.extend_from_slice(&pair.0.to_bits().to_le_bytes());
                buf.extend_from_slice(&pair.1.to_bits().to_le_bytes());
            }
        }
        stable_hash64(b"scene-hash/v1", &buf)
    }

    /// 实例总数（预算审计用）。
    #[must_use]
    pub fn instance_count(&self) -> usize {
        self.instances.iter().map(InstanceGroup::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biome::Biome;
    use crate::genome::Genome;
    use crate::world::WORLD_BOUND;

    fn manifest_for(tag: &str, tick: u64) -> SceneManifest {
        let seed = DreamscapeSeed::from_input(tag);
        let genome = Genome::from_seed(&seed);
        let biome = Biome::select(&seed, &genome);
        let env = EnvironmentState::initial(&seed, tick);
        let world = World::generate(&seed, &genome, &biome);
        let world_id = crate::seed::world_identity(
            &seed,
            crate::generation_version(),
            biome.kind.id().as_bytes(),
        );
        SceneManifest::from_world(world_id, &world, &env, biome.effective_mist_bias())
    }

    #[test]
    fn scene_hash_deterministic_same_inputs() {
        // §114：同 seed+version+genome+environment → hash 相等。
        let a = manifest_for("hash-det", 1000);
        let b = manifest_for("hash-det", 1000);
        assert_eq!(a, b);
        assert_eq!(a.scene_hash(), b.scene_hash());
    }

    #[test]
    fn scene_hash_changes_with_inputs() {
        let base = manifest_for("hash-in", 1000);
        let other_seed = manifest_for("hash-in-2", 1000);
        assert_ne!(base.scene_hash(), other_seed.scene_hash());
        // 半日偏移 → 相位不同 → 光态不同 → 场景变化（整日偏移相位相同，
        // 场景一致是正确行为）。
        let other_env = manifest_for("hash-in", 1000 + 24 * 60 / 2);
        assert_ne!(
            base.scene_hash(),
            other_env.scene_hash(),
            "环境变化 → 场景变化"
        );
        // 不同 world_id（同 manifest 其余相同）→ hash 变化。
        let mut moved = base.clone();
        moved.world_id ^= 1;
        assert_ne!(base.scene_hash(), moved.scene_hash());
    }

    #[test]
    fn manifest_bounded_size() {
        // §115：极端 seed 下 manifest 序列化体积有界（< 256 KiB）。
        for i in 0..4u64 {
            let m = manifest_for(&format!("size{i}"), 0);
            let json = serde_json::to_string(&m).unwrap();
            assert!(
                json.len() < 256 * 1024,
                "manifest {} bytes 过大",
                json.len()
            );
            assert!(m.instance_count() <= crate::world::MAX_OBJECTS + crate::world::MAX_PARTICLES);
            assert!(m.landmarks.len() <= crate::world::MAX_LANDMARKS);
            // 位置在有限边界内（§50 传递到 manifest）。
            for g in &m.instances {
                for x in &g.x {
                    assert!(x.abs() <= WORLD_BOUND);
                }
            }
        }
    }

    #[test]
    fn manifest_serde_roundtrip_and_no_three_leak() {
        // §111/§112：纯数值 DTO；serde 往返一致，hash 不变。
        let m = manifest_for("ser", 500);
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("THREE"), "无 Three.js 类型泄漏（§111）");
        let back: SceneManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.scene_hash(), m.scene_hash());
    }

    #[test]
    fn landmark_y_on_terrain() {
        // y 由地形采样（§81 恢复路径需要；不悬空）。
        let m = manifest_for("landmark-y", 0);
        assert!(!m.landmarks.is_empty());
        for l in &m.landmarks {
            assert!(l.y.is_finite());
        }
    }

    const _: () = {
        const fn assert_send_sync<T: Send + Sync>() {}
        let _ = assert_send_sync::<SceneManifest>;
    };
}
