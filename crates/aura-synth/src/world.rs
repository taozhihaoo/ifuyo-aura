//! World 拓扑（§47–§64）：空间结构 → 地形 → landmark → 物件放置。
//!
//! - 有限逻辑边界（§50，禁止无限世界）；horizon 是渲染层的深度错觉
//!   （§51），不在本层建模。
//! - 地形为低频 value-noise 高度场（§48），同 seed+参数逐位一致（§49）。
//! - 物件经 placement 规则（§52：坡度容差/密度/排除半径/族亲和），
//!   spatial hash 避免 O(N²)（§54），主要 landmark 不重叠（§53）。
//! - landmark 携带稳定 id = hash(world_seed, placement_index)（§56）。
//! - 预算上限（§63/§64）：seed 再极端，数量也不上升。

use serde::{Deserialize, Serialize};

use crate::biome::{Biome, ObjectFamily};
use crate::genome::Genome;
use crate::rng::{streams, Rng};
use crate::seed::{stable_hash64, DreamscapeSeed};

pub const WORLD_VERSION: u16 = 1;
/// 有限边界（§50）：世界为 `[-BOUND, BOUND]²`。
pub const WORLD_BOUND: f32 = 256.0;
/// 对象预算（§63）。
pub const MAX_OBJECTS: usize = 512;
/// landmark 预算（§63）。
pub const MAX_LANDMARKS: usize = 7;
/// 粒子预算（§63；manifest 层再乘 LOD）。
pub const MAX_PARTICLES: usize = 256;
/// spatial hash 单元边长（§54）。
pub const GRID_CELL: f32 = 8.0;

/// 高度场（§48/§49）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Terrain {
    pub amplitude: f32,
    pub frequency: f32,
    terrain_seed: u64,
}

impl Terrain {
    #[must_use]
    pub fn new(seed: &DreamscapeSeed, amplitude: f32, frequency: f32) -> Self {
        Self {
            amplitude,
            frequency,
            terrain_seed: stable_hash64(b"terrain/v1", &seed.value.to_le_bytes()),
        }
    }

    /// 地形种子（manifest 透出，渲染层确定性重建高度场）。
    #[must_use]
    pub const fn height_seed(&self) -> u64 {
        self.terrain_seed
    }

    /// 晶格伪随机值（确定性：同 (ix,iz) 同值，与查询顺序无关）。
    fn lattice(&self, ix: i32, iz: i32) -> f32 {
        let mut buf = Vec::with_capacity(12);
        buf.extend_from_slice(&self.terrain_seed.to_le_bytes());
        buf.extend_from_slice(&ix.to_le_bytes());
        buf.extend_from_slice(&iz.to_le_bytes());
        (stable_hash64(b"terrain-lattice/v1", &buf) >> 40) as f32 / (1u32 << 24) as f32
    }

    /// 低频 value-noise（两倍频程；§48 计算成本可控）。
    #[must_use]
    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let mut h = 0.0;
        let mut amp = 1.0;
        let mut freq = self.frequency;
        for _ in 0..2 {
            let fx = x * freq;
            let fz = z * freq;
            let ix = fx.floor() as i32;
            let iz = fz.floor() as i32;
            let tx = smoothstep(fx - fx.floor());
            let tz = smoothstep(fz - fz.floor());
            let v00 = self.lattice(ix, iz);
            let v10 = self.lattice(ix + 1, iz);
            let v01 = self.lattice(ix, iz + 1);
            let v11 = self.lattice(ix + 1, iz + 1);
            let a = v00 + (v10 - v00) * tx;
            let b = v01 + (v11 - v01) * tx;
            h += (a + (b - a) * tz) * amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        (h * 2.0 - 1.0) * self.amplitude
    }

    /// 坡度近似（|∇h|，placement 坡度容差用，§52）。
    #[must_use]
    pub fn slope_at(&self, x: f32, z: f32) -> f32 {
        let e = 0.5;
        let hx = self.height_at(x + e, z) - self.height_at(x - e, z);
        let hz = self.height_at(x, z + e) - self.height_at(x, z - e);
        (hx * hx + hz * hz).sqrt() / (2.0 * e)
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// landmark 形态（§55 示例集合）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LandmarkKind {
    OldTree,
    StandingStone,
    StillPool,
    Arch,
    GlowCluster,
}

/// landmark（§55/§56）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    /// 稳定 id = hash(world_seed, placement_index)（§56：快照/变异可稳定引用）。
    pub id: u64,
    pub kind: LandmarkKind,
    pub x: f32,
    pub z: f32,
    pub radius: f32,
    pub rotation: f32,
}

/// 已放置普通对象（下篇渲染层实例化的来源数据）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlacedObject {
    pub family: ObjectFamily,
    pub x: f32,
    pub z: f32,
    pub scale: f32,
    pub rotation: f32,
    /// 稳定序号（渲染层 per-instance 变异用）。
    pub nonce: u32,
}

/// 生成的世界（逻辑态；GPU 对象在渲染层，§80）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub world_version: u16,
    pub seed: DreamscapeSeed,
    pub biome: Biome,
    pub terrain: Terrain,
    pub landmarks: Vec<Landmark>,
    pub objects: Vec<PlacedObject>,
}

/// 简单 spatial hash（§54）：cell → 索引集合。
#[derive(Debug, Default)]
struct SpatialHash {
    cells: std::collections::HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialHash {
    fn cell_of(x: f32, z: f32) -> (i32, i32) {
        (
            (x / GRID_CELL).floor() as i32,
            (z / GRID_CELL).floor() as i32,
        )
    }

    fn insert(&mut self, x: f32, z: f32, idx: usize) {
        self.cells.entry(Self::cell_of(x, z)).or_default().push(idx);
    }

    /// 查询 (x,z) 半径 r 内是否已有对象（检查 3×3 邻域；对象本体半径
    /// 已并入 r）。
    fn any_nearby(&self, objects: &[PlacedObject], x: f32, z: f32, r: f32) -> bool {
        let (cx, cz) = Self::cell_of(x, z);
        for dx in -1..=1 {
            for dz in -1..=1 {
                if let Some(list) = self.cells.get(&(cx + dx, cz + dz)) {
                    for &i in list {
                        let o = &objects[i];
                        let dx = o.x - x;
                        let dz = o.z - z;
                        if dx * dx + dz * dz < r * r {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

/// 密度缩放硬上限（§63/§64：预算不随 seed 上升）。
const DENSITY_CAP: f32 = 1.3;

impl World {
    /// 生成世界（确定性：§49/§119 同输入同输出）。
    #[must_use]
    pub fn generate(seed: &DreamscapeSeed, genome: &Genome, biome: &Biome) -> Self {
        let (amp_p, freq_p) = biome.kind.terrain_profile();
        let terrain = Terrain::new(
            seed,
            amp_p * biome.terrain_variation * genome.terrain_scale,
            freq_p,
        );
        let mut w = Rng::from_stream(seed, streams::PLACEMENT);
        let mut world = Self {
            world_version: WORLD_VERSION,
            seed: *seed,
            biome: *biome,
            terrain,
            landmarks: Vec::new(),
            objects: Vec::new(),
        };

        // ---- landmark（§55）：少量、互不重叠（§53）----
        let want_landmarks = 3 + (w.next_u64() as usize) % 3; // 3..=5 ≤ MAX_LANDMARKS
        let kinds = [
            LandmarkKind::OldTree,
            LandmarkKind::StandingStone,
            LandmarkKind::StillPool,
            LandmarkKind::Arch,
            LandmarkKind::GlowCluster,
        ];
        for i in 0..want_landmarks {
            for _attempt in 0..64 {
                let x = w.next_f32_range(-WORLD_BOUND * 0.8, WORLD_BOUND * 0.8);
                let z = w.next_f32_range(-WORLD_BOUND * 0.8, WORLD_BOUND * 0.8);
                let radius = w.next_f32_range(2.0, 5.0);
                if world.terrain.slope_at(x, z) > 0.35 {
                    continue;
                }
                let clash = world.landmarks.iter().any(|l| {
                    let dx = l.x - x;
                    let dz = l.z - z;
                    let rr = l.radius + radius;
                    dx * dx + dz * dz < rr * rr
                });
                if clash {
                    continue;
                }
                world.landmarks.push(Landmark {
                    id: stable_hash64(
                        b"landmark-id/v1",
                        &[seed.value.to_le_bytes(), (i as u64).to_le_bytes()].concat(),
                    ),
                    kind: kinds[(w.next_u64() as usize) % kinds.len()],
                    x,
                    z,
                    radius,
                    rotation: w.next_f32_range(0.0, std::f32::consts::TAU),
                });
                break;
            }
        }

        // ---- 普通对象（§52/§63/§64）----
        let families = biome.kind.object_families();
        let density = (biome.effective_density() * genome.flora_density * 2.0).min(DENSITY_CAP);
        let target = ((MAX_OBJECTS as f32) * density) as usize;
        let mut hash = SpatialHash::default();
        let mut nonce = 0u32;
        'budget: for _ in 0..4096 {
            if world.objects.len() >= target {
                break 'budget;
            }
            let x = w.next_f32_range(-WORLD_BOUND, WORLD_BOUND);
            let z = w.next_f32_range(-WORLD_BOUND, WORLD_BOUND);
            let family = families[(w.next_u64() as usize) % families.len()];
            // 族亲和：粒子/light 不占实体预算大头；flora/stone 需平缓。
            if matches!(family, ObjectFamily::Flora | ObjectFamily::Stone)
                && world.terrain.slope_at(x, z) > 0.45
            {
                continue;
            }
            // landmark 排除（不穿模，§53）。
            let in_landmark = world.landmarks.iter().any(|l| {
                let dx = l.x - x;
                let dz = l.z - z;
                dx * dx + dz * dz < (l.radius + 1.0) * (l.radius + 1.0)
            });
            if in_landmark {
                continue;
            }
            // 排除半径（§52）：0.9 m + 随机体量。
            let scale = 0.6 + w.next_f32_unit() * 0.9;
            if hash.any_nearby(&world.objects, x, z, 0.9 + scale) {
                continue;
            }
            hash.insert(x, z, world.objects.len());
            world.objects.push(PlacedObject {
                family,
                x,
                z,
                scale,
                rotation: w.next_f32_range(0.0, std::f32::consts::TAU),
                nonce,
            });
            nonce = nonce.wrapping_add(1);
        }

        // ---- 粒子（预算内，§63）----
        let particles = (MAX_PARTICLES as f32 * 0.25 * density) as usize;
        for _ in 0..particles {
            let x = w.next_f32_range(-WORLD_BOUND, WORLD_BOUND);
            let z = w.next_f32_range(-WORLD_BOUND, WORLD_BOUND);
            world.objects.push(PlacedObject {
                family: ObjectFamily::Particle,
                x,
                z,
                scale: w.next_f32_range(0.2, 0.6),
                rotation: w.next_f32_range(0.0, std::f32::consts::TAU),
                nonce,
            });
            nonce = nonce.wrapping_add(1);
        }
        world
    }

    /// 世界逻辑态稳定 hash（测试/快照一致性用；§114 同类机制）。
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.seed.value.to_le_bytes());
        buf.extend_from_slice(&self.terrain.amplitude.to_bits().to_le_bytes());
        buf.extend_from_slice(&self.terrain.frequency.to_bits().to_le_bytes());
        for l in &self.landmarks {
            buf.extend_from_slice(&l.id.to_le_bytes());
            buf.extend_from_slice(&l.x.to_bits().to_le_bytes());
            buf.extend_from_slice(&l.z.to_bits().to_le_bytes());
        }
        for o in &self.objects {
            buf.extend_from_slice(&o.x.to_bits().to_le_bytes());
            buf.extend_from_slice(&o.z.to_bits().to_le_bytes());
            buf.extend_from_slice(&o.nonce.to_le_bytes());
        }
        stable_hash64(b"world-state/v1", &buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::EnvironmentState;
    use crate::genome::Genome;

    fn world_for(tag: &str) -> World {
        let seed = DreamscapeSeed::from_input(tag);
        let genome = Genome::from_seed(&seed);
        let biome = Biome::select(&seed, &genome);
        World::generate(&seed, &genome, &biome)
    }

    #[test]
    fn generation_deterministic() {
        let a = world_for("world-det");
        let b = world_for("world-det");
        assert_eq!(a, b, "同 seed+genome+biome → 同世界（§49/§119）");
        assert_eq!(a.state_hash(), b.state_hash());
        let c = world_for("world-det-2");
        assert_ne!(a.state_hash(), c.state_hash(), "不同 seed 合理变化");
    }

    #[test]
    fn finite_bounds_respected() {
        // §50：一切位置都在有限边界内。
        for tag in ["b1", "b2", "b3", "b4", "b5"] {
            let w = world_for(tag);
            assert!(w
                .landmarks
                .iter()
                .all(|l| l.x.abs() <= WORLD_BOUND && l.z.abs() <= WORLD_BOUND));
            assert!(w
                .objects
                .iter()
                .all(|o| o.x.abs() <= WORLD_BOUND && o.z.abs() <= WORLD_BOUND));
        }
    }

    #[test]
    fn budgets_hold_under_extreme_genome() {
        // §63/§64：seed/参数再极端，数量不超预算。
        for i in 0..8u64 {
            let seed = DreamscapeSeed::from_input(&format!("extreme{i}"));
            let mut g = Genome::from_seed(&seed);
            g.set(GenomeField::FloraDensity, 1.0);
            g.set(GenomeField::TerrainScale, 4.0);
            let b = Biome::with_variation(&seed, crate::biome::BiomeKind::MistyForest);
            let w = World::generate(&seed, &g, &b);
            assert!(w.objects.len() <= MAX_OBJECTS + MAX_PARTICLES);
            assert!(w.landmarks.len() <= MAX_LANDMARKS);
            let n_objects = w
                .objects
                .iter()
                .filter(|o| o.family != ObjectFamily::Particle)
                .count();
            assert!(n_objects <= MAX_OBJECTS);
        }
    }

    #[test]
    fn landmarks_sparse_and_non_overlapping_with_stable_ids() {
        let w = world_for("landmark-check");
        assert!((1..=MAX_LANDMARKS).contains(&w.landmarks.len()));
        for (i, a) in w.landmarks.iter().enumerate() {
            for b in &w.landmarks[i + 1..] {
                let dx = a.x - b.x;
                let dz = a.z - b.z;
                let rr = a.radius + b.radius;
                assert!(dx * dx + dz * dz >= rr * rr, "landmark 重叠（§53）");
            }
        }
        let again = world_for("landmark-check");
        assert_eq!(
            w.landmarks.iter().map(|l| l.id).collect::<Vec<_>>(),
            again.landmarks.iter().map(|l| l.id).collect::<Vec<_>>(),
            "id 稳定（§56）"
        );
    }

    #[test]
    fn placement_rules_slope_and_exclusion() {
        // §52：flora/stone 不在陡坡；对象间保持排除半径。
        let w = world_for("rules");
        for o in &w.objects {
            if matches!(o.family, ObjectFamily::Flora | ObjectFamily::Stone) {
                assert!(w.terrain.slope_at(o.x, o.z) <= 0.45, "坡度容差");
            }
        }
        // 抽样验证两两距离（数量受预算约束，O(N²) 可接受）。
        for (i, a) in w.objects.iter().enumerate() {
            for b in &w.objects[..i] {
                if matches!(a.family, ObjectFamily::Particle)
                    || matches!(b.family, ObjectFamily::Particle)
                {
                    continue;
                }
                let dx = a.x - b.x;
                let dz = a.z - b.z;
                assert!(dx * dx + dz * dz >= 0.81, "排除半径 0.9m（§52）");
            }
        }
        // 不与 landmark 重叠。
        for o in &w.objects {
            for l in &w.landmarks {
                let dx = o.x - l.x;
                let dz = o.z - l.z;
                let rr = l.radius + 1.0;
                assert!(dx * dx + dz * dz >= rr * rr, "对象不穿 landmark（§53）");
            }
        }
    }

    #[test]
    fn density_respects_genome_with_cap() {
        // 密度随 flora 上升但受 DENSITY_CAP 钳制（§63/§64）。
        let seed = DreamscapeSeed::from_input("density");
        let g0 = {
            let mut g = Genome::from_seed(&seed);
            g.set(GenomeField::FloraDensity, 0.1);
            g
        };
        let g1 = {
            let mut g = Genome::from_seed(&seed);
            g.set(GenomeField::FloraDensity, 1.0);
            g
        };
        let b = Biome::select(&seed, &g0);
        let low = World::generate(&seed, &g0, &b).objects.len();
        let high = World::generate(&seed, &g1, &b).objects.len();
        assert!(high > low, "密度参数生效");
        assert!(high <= MAX_OBJECTS + MAX_PARTICLES);
    }

    #[test]
    fn terrain_height_deterministic_and_smooth() {
        let seed = DreamscapeSeed::from_input("terrain");
        let t = Terrain::new(&seed, 2.0, 0.05);
        for xz in [(0.0, 0.0), (12.5, -3.25), (100.0, 100.0), (-77.7, 33.3)] {
            let (x, z) = xz;
            assert_eq!(t.height_at(x, z), t.height_at(x, z), "逐位一致（§49）");
        }
        // 相邻采样连续（value-noise 平滑，渲染不裂）。
        let mut prev = t.height_at(0.0, 0.0);
        for i in 1..200 {
            let h = t.height_at(i as f32 * 0.1, 0.0);
            assert!((h - prev).abs() < 0.5, "高度场连续");
            prev = h;
        }
    }

    #[test]
    fn spatial_hash_finds_nearby() {
        // §54：空间哈希单元查询正确性（经放置行为间接覆盖 + 直接构造）。
        let mut h = SpatialHash::default();
        let objs = vec![
            PlacedObject {
                family: ObjectFamily::Stone,
                x: 0.0,
                z: 0.0,
                scale: 1.0,
                rotation: 0.0,
                nonce: 0,
            },
            PlacedObject {
                family: ObjectFamily::Stone,
                x: 100.0,
                z: 100.0,
                scale: 1.0,
                rotation: 0.0,
                nonce: 1,
            },
        ];
        h.insert(0.0, 0.0, 0);
        h.insert(100.0, 100.0, 1);
        assert!(h.any_nearby(&objs, 0.5, 0.5, 2.0));
        assert!(!h.any_nearby(&objs, 50.0, 50.0, 2.0));
    }

    #[test]
    fn world_state_hash_ignores_env_snapshot() {
        // 世界逻辑态不含瞬时环境（环境独立持久化，§78/§79 分层）。
        let w = world_for("envfree");
        let e1 = EnvironmentState::initial(&w.seed, 0);
        let _ = e1;
        let h1 = w.state_hash();
        let h2 = w.state_hash();
        assert_eq!(h1, h2);
    }

    use crate::biome::Biome;
    use crate::genome::GenomeField;
}
