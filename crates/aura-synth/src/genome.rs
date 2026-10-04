//! Dreamscape Genome（§23–§29）：控制世界基础形态的有界参数集合。
//!
//! - 每个参数有 min/max/default 与 genome 版本（§25 禁止参数无限增长）。
//! - seed → genome → world（§24）；genome 可序列化（快照/调试）。
//! - mutation 是对小范围参数的受控变化（§26），发生在明确时间尺度上
//!   （新 session / 长 idle / 事件 / 显式触发，§27/§28），且可记录、
//!   可重放（§29）。

use serde::{Deserialize, Serialize};

use crate::rng::{streams, Rng};
use crate::seed::DreamscapeSeed;

/// genome 语义版本（字段集合/边界变更 = 升版本，§25）。
pub const GENOME_VERSION: u16 = 1;

/// Genome 字段（边界集中定义，§25）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GenomeField {
    TerrainScale,
    FloraDensity,
    MistDensity,
    LightSoftness,
    WaterPresence,
    ColorTemperature,
    ObjectScaleBias,
    HorizonHeight,
}

impl GenomeField {
    /// (min, max, default)（§25/§32 边界即合同）。
    #[must_use]
    pub const fn bounds(self) -> (f32, f32, f32) {
        match self {
            Self::TerrainScale => (0.5, 4.0, 1.0),
            Self::FloraDensity => (0.0, 1.0, 0.5),
            Self::MistDensity => (0.0, 1.0, 0.3),
            Self::LightSoftness => (0.0, 1.0, 0.6),
            Self::WaterPresence => (0.0, 1.0, 0.4),
            Self::ColorTemperature => (-1.0, 1.0, 0.0),
            Self::ObjectScaleBias => (0.5, 2.0, 1.0),
            Self::HorizonHeight => (0.0, 1.0, 0.5),
        }
    }
}

/// 世界基础形态参数（全部有界，§23）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    pub genome_version: u16,
    pub terrain_scale: f32,
    pub flora_density: f32,
    pub mist_density: f32,
    pub light_softness: f32,
    pub water_presence: f32,
    pub color_temperature: f32,
    pub object_scale_bias: f32,
    pub horizon_height: f32,
}

impl Genome {
    /// 默认基因组（全部取 default）。
    #[must_use]
    pub fn default_genome() -> Self {
        Self::from_fields(|f| f.bounds().2)
    }

    /// seed → genome（§24/§45）：由 world 流决定基础形态；
    /// 同 seed 恒同 genome。
    #[must_use]
    pub fn from_seed(seed: &DreamscapeSeed) -> Self {
        let mut world = Rng::from_stream(seed, streams::WORLD);
        Self::from_fields(|f| {
            let (min, max, def) = f.bounds();
            // 以 default 为锚、流值决定偏移（保持分布集中在合理区间）。
            let spread = (max - min) * 0.5;
            let offset = (world.next_f32_unit() - 0.5) * spread;
            (def + offset).clamp(min, max)
        })
    }

    fn from_fields(mut derive: impl FnMut(GenomeField) -> f32) -> Self {
        Self {
            genome_version: GENOME_VERSION,
            terrain_scale: derive(GenomeField::TerrainScale),
            flora_density: derive(GenomeField::FloraDensity),
            mist_density: derive(GenomeField::MistDensity),
            light_softness: derive(GenomeField::LightSoftness),
            water_presence: derive(GenomeField::WaterPresence),
            color_temperature: derive(GenomeField::ColorTemperature),
            object_scale_bias: derive(GenomeField::ObjectScaleBias),
            horizon_height: derive(GenomeField::HorizonHeight),
        }
    }

    /// 读取字段值。
    #[must_use]
    pub fn get(&self, f: GenomeField) -> f32 {
        match f {
            GenomeField::TerrainScale => self.terrain_scale,
            GenomeField::FloraDensity => self.flora_density,
            GenomeField::MistDensity => self.mist_density,
            GenomeField::LightSoftness => self.light_softness,
            GenomeField::WaterPresence => self.water_presence,
            GenomeField::ColorTemperature => self.color_temperature,
            GenomeField::ObjectScaleBias => self.object_scale_bias,
            GenomeField::HorizonHeight => self.horizon_height,
        }
    }

    /// 写入字段值（钳制到边界）。
    pub fn set(&mut self, f: GenomeField, v: f32) {
        let (min, max, _) = f.bounds();
        let v = v.clamp(min, max);
        match f {
            GenomeField::TerrainScale => self.terrain_scale = v,
            GenomeField::FloraDensity => self.flora_density = v,
            GenomeField::MistDensity => self.mist_density = v,
            GenomeField::LightSoftness => self.light_softness = v,
            GenomeField::WaterPresence => self.water_presence = v,
            GenomeField::ColorTemperature => self.color_temperature = v,
            GenomeField::ObjectScaleBias => self.object_scale_bias = v,
            GenomeField::HorizonHeight => self.horizon_height = v,
        }
    }

    /// 反序列化后必须调用的边界矫正（手改 JSON 不产生越界世界）。
    pub fn sanitize(&mut self) {
        for f in [
            GenomeField::TerrainScale,
            GenomeField::FloraDensity,
            GenomeField::MistDensity,
            GenomeField::LightSoftness,
            GenomeField::WaterPresence,
            GenomeField::ColorTemperature,
            GenomeField::ObjectScaleBias,
            GenomeField::HorizonHeight,
        ] {
            let v = self.get(f);
            self.set(f, v);
        }
        if self.genome_version != GENOME_VERSION {
            self.genome_version = GENOME_VERSION;
        }
    }
}

/// mutation 类别（§84）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MutationCategory {
    Growth,
    Atmosphere,
    Color,
    Placement,
    RarePhenomenon,
}

/// 单条受控变化（§26：小步）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MutationDelta {
    pub field: GenomeField,
    pub delta: f32,
}

/// mutation 触发来源（§27/§28：明确时间尺度，无每帧随机）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationTrigger {
    /// 新 session 打开。
    NewSession,
    /// 长 idle（连续 [`IDLE_MUTATION_AFTER_TICKS`] tick 无交互）。
    LongIdle,
    /// 世界事件连带。
    WorldEvent,
    /// 用户显式触发。
    Explicit,
}

/// 长 idle 阈值（逻辑 tick；1 tick = 1 逻辑分钟，§30）。
pub const IDLE_MUTATION_AFTER_TICKS: u64 = 8 * 60; // 8 小时逻辑时间

/// 单次 mutation 的可重放记录（§29）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutationRecord {
    pub mutation_id: u64,
    pub source_seed: DreamscapeSeed,
    /// 变化前 genome 的稳定 hash。
    pub parent_state_hash: u64,
    pub deltas: Vec<MutationDelta>,
    pub category: MutationCategory,
    pub trigger: MutationTrigger,
    /// 逻辑时间（LogicalTick，1 tick = 1 逻辑分钟，§30）。
    pub logical_tick: u64,
}

/// 单次 mutation 的最大字段数（§26 小范围）。
pub const MAX_MUTATION_FIELDS: usize = 3;
/// 单字段最大步长（占参数区间的比例，§26/§83 小步）。
pub const MAX_MUTATION_STEP: f32 = 0.03;

/// genome 状态稳定 hash（parent_state_hash / 测试用）。
#[must_use]
pub fn genome_hash(g: &Genome) -> u64 {
    crate::seed::stable_hash64(b"genome/v1", &genome_bytes(g))
}

fn genome_bytes(g: &Genome) -> Vec<u8> {
    // 按字段字节序 hash（不引入文本序列化形态，稳定且免依赖）。
    let mut buf = Vec::with_capacity(9 * 4 + 2);
    buf.extend_from_slice(&g.genome_version.to_le_bytes());
    for f in [
        GenomeField::TerrainScale,
        GenomeField::FloraDensity,
        GenomeField::MistDensity,
        GenomeField::LightSoftness,
        GenomeField::WaterPresence,
        GenomeField::ColorTemperature,
        GenomeField::ObjectScaleBias,
        GenomeField::HorizonHeight,
    ] {
        buf.extend_from_slice(&g.get(f).to_bits().to_le_bytes());
    }
    buf
}

/// 提议一次 mutation（确定性：同 rng 状态 + 同 genome → 同记录，§29）。
/// 步长受 [`MAX_MUTATION_STEP`] 约束；字段数 1..=[`MAX_MUTATION_FIELDS`]。
#[must_use]
pub fn propose_mutation(
    seed: &DreamscapeSeed,
    parent: &Genome,
    category: MutationCategory,
    trigger: MutationTrigger,
    logical_tick: u64,
    rng: &mut Rng,
) -> MutationRecord {
    let fields: &[GenomeField] = match category {
        MutationCategory::Growth => &[GenomeField::FloraDensity],
        MutationCategory::Atmosphere => &[GenomeField::MistDensity, GenomeField::HorizonHeight],
        MutationCategory::Color => &[GenomeField::ColorTemperature, GenomeField::LightSoftness],
        MutationCategory::Placement => &[GenomeField::ObjectScaleBias, GenomeField::TerrainScale],
        MutationCategory::RarePhenomenon => {
            &[GenomeField::LightSoftness, GenomeField::WaterPresence]
        }
    };
    let count = 1 + (rng.next_u64() as usize) % MAX_MUTATION_FIELDS.min(fields.len());
    let mut deltas = Vec::with_capacity(count);
    for i in 0..count {
        let field = fields[(rng.next_u64() as usize + i) % fields.len()];
        let (min, max, _) = field.bounds();
        let step = (max - min) * MAX_MUTATION_STEP;
        let delta = rng.next_f32_range(-step, step);
        deltas.push(MutationDelta { field, delta });
    }
    MutationRecord {
        mutation_id: crate::seed::stable_hash64(
            b"mutation-id/v1",
            &[
                seed.value.to_le_bytes(),
                genome_hash(parent).to_le_bytes(),
                logical_tick.to_le_bytes(),
                (rng.next_u64()).to_le_bytes(),
            ]
            .concat(),
        ),
        source_seed: *seed,
        parent_state_hash: genome_hash(parent),
        deltas,
        category,
        trigger,
        logical_tick,
    }
}

/// 应用 mutation 记录（钳制边界；§83 不毁掉原世界）。
pub fn apply_mutation(g: &mut Genome, record: &MutationRecord) {
    for d in &record.deltas {
        let v = g.get(d.field) + d.delta;
        g.set(d.field, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_complete_and_ordered() {
        for f in [
            GenomeField::TerrainScale,
            GenomeField::FloraDensity,
            GenomeField::MistDensity,
            GenomeField::LightSoftness,
            GenomeField::WaterPresence,
            GenomeField::ColorTemperature,
            GenomeField::ObjectScaleBias,
            GenomeField::HorizonHeight,
        ] {
            let (min, max, def) = f.bounds();
            assert!(min < max, "{f:?}");
            assert!((min..=max).contains(&def), "{f:?}");
        }
    }

    #[test]
    fn from_seed_deterministic_and_varying() {
        let a = Genome::from_seed(&DreamscapeSeed::from_input("genome-a"));
        let b = Genome::from_seed(&DreamscapeSeed::from_input("genome-a"));
        assert_eq!(a, b, "同 seed → 同 genome（§24）");
        let c = Genome::from_seed(&DreamscapeSeed::from_input("genome-b"));
        assert_ne!(a, c);
        // 全字段在界内。
        for f in [
            GenomeField::TerrainScale,
            GenomeField::FloraDensity,
            GenomeField::MistDensity,
            GenomeField::LightSoftness,
            GenomeField::WaterPresence,
            GenomeField::ColorTemperature,
            GenomeField::ObjectScaleBias,
            GenomeField::HorizonHeight,
        ] {
            let (min, max, _) = f.bounds();
            assert!((min..=max).contains(&a.get(f)), "{f:?}");
        }
    }

    #[test]
    fn set_clamps_to_bounds() {
        let mut g = Genome::default_genome();
        g.set(GenomeField::FloraDensity, 5.0);
        assert_eq!(g.flora_density, 1.0);
        g.set(GenomeField::FloraDensity, -3.0);
        assert_eq!(g.flora_density, 0.0);
        g.set(GenomeField::ColorTemperature, 99.0);
        assert_eq!(g.color_temperature, 1.0);
    }

    #[test]
    fn sanitize_repairs_out_of_bounds() {
        let json = r#"{"genome_version":1,"terrain_scale":99.0,"flora_density":-1.0,
            "mist_density":0.3,"light_softness":0.6,"water_presence":0.4,
            "color_temperature":0.0,"object_scale_bias":1.0,"horizon_height":0.5}"#;
        let mut g: Genome = serde_json::from_str(json).unwrap();
        assert_eq!(g.terrain_scale, 99.0); // 反序列化不钳制（数据保真）
        g.sanitize();
        assert_eq!(g.terrain_scale, 4.0);
        assert_eq!(g.flora_density, 0.0);
    }

    #[test]
    fn mutation_small_steps_and_clamped() {
        let seed = DreamscapeSeed::from_input("mut");
        let parent = Genome::default_genome();
        let mut rng = Rng::from_stream(&seed, streams::MUTATION);
        for i in 0..200 {
            let cat = [
                MutationCategory::Growth,
                MutationCategory::Atmosphere,
                MutationCategory::Color,
                MutationCategory::Placement,
                MutationCategory::RarePhenomenon,
            ][i % 5];
            let rec = propose_mutation(
                &seed,
                &parent,
                cat,
                MutationTrigger::NewSession,
                i as u64,
                &mut rng,
            );
            assert!(!rec.deltas.is_empty());
            assert!(rec.deltas.len() <= MAX_MUTATION_FIELDS, "小范围（§26）");
            let mut g = parent;
            apply_mutation(&mut g, &rec);
            for d in &rec.deltas {
                let (min, max, _) = d.field.bounds();
                let step = (max - min) * MAX_MUTATION_STEP + f32::EPSILON;
                assert!(d.delta.abs() <= step, "小步长（§26）: {:?}", d);
                assert!((min..=max).contains(&g.get(d.field)), "边界钳制");
            }
        }
    }

    #[test]
    fn mutation_record_replayable() {
        let seed = DreamscapeSeed::from_input("replay");
        let parent = Genome::from_seed(&seed);
        let mk = |rng: &mut Rng| {
            propose_mutation(
                &seed,
                &parent,
                MutationCategory::Growth,
                MutationTrigger::LongIdle,
                777,
                rng,
            )
        };
        let mut r1 = Rng::from_stream(&seed, streams::MUTATION);
        let mut r2 = Rng::from_stream(&seed, streams::MUTATION);
        let a = mk(&mut r1);
        let b = mk(&mut r2);
        assert_eq!(a, b, "同 rng 状态 → 同记录（§29 可重放）");
        assert_eq!(a.parent_state_hash, genome_hash(&parent));

        // §92 精神：同一事件日志按序重放两次 → 相同 logical state。
        let log = vec![a, b];
        let mut g1 = parent;
        for rec in &log {
            apply_mutation(&mut g1, rec);
        }
        let mut g2 = parent;
        for rec in &log {
            apply_mutation(&mut g2, rec);
        }
        assert_eq!(g1, g2, "重放得到相同 logical state");
    }

    #[test]
    fn mutation_never_flips_the_world() {
        // §83：连续多次 mutation 后仍与 parent 接近（每字段 ≤ 步长×次数）。
        let seed = DreamscapeSeed::from_input("slow");
        let parent = Genome::from_seed(&seed);
        let mut rng = Rng::from_stream(&seed, streams::MUTATION);
        let mut g = parent;
        for tick in 0..50u64 {
            let cat = [MutationCategory::Growth, MutationCategory::Atmosphere][(tick % 2) as usize];
            let rec = propose_mutation(
                &seed,
                &g,
                cat,
                MutationTrigger::WorldEvent,
                tick * 600,
                &mut rng,
            );
            apply_mutation(&mut g, &rec);
        }
        for f in [
            GenomeField::FloraDensity,
            GenomeField::MistDensity,
            GenomeField::HorizonHeight,
        ] {
            let (min, max, _) = f.bounds();
            let drift = (g.get(f) - parent.get(f)).abs();
            assert!(drift <= (max - min) * 0.35, "{f:?} 漂移过大（§83）");
        }
    }

    #[test]
    fn genome_hash_stable() {
        let g = Genome::from_seed(&DreamscapeSeed::from_input("hash"));
        let mut h = g;
        assert_eq!(genome_hash(&g), genome_hash(&h));
        h.set(GenomeField::FloraDensity, g.flora_density + 0.01);
        assert_ne!(genome_hash(&g), genome_hash(&h));
    }

    #[test]
    fn genome_serde_roundtrip() {
        let g = Genome::from_seed(&DreamscapeSeed::from_input("ser"));
        let json = serde_json::to_string(&g).unwrap();
        let back: Genome = serde_json::from_str(&json).unwrap();
        assert_eq!(g, back);
    }
}
