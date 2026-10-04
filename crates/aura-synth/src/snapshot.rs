//! Snapshot 与 Session（§77–§81/§103–§106）。
//!
//! - Snapshot 只保存逻辑态（§79/§80：无 GPU/Mesh/renderer 对象）。
//! - 恢复路径（§81）：snapshot → 重新生成世界 → 场景（下篇渲染层）。
//! - 世界持久（§77）：同 seed 明天回来仍是同一世界；Regenerate 只能
//!   用户显式触发（§103）。
//! - seed 导入/导出（§104/§105）：人类格式；非法输入明确报错。

use serde::{Deserialize, Serialize};

use crate::biome::Biome;
use crate::env::EnvironmentState;
use crate::events::{self, EventEntry};
use crate::generator::{generation_supported, SeedParseError, SUPPORTED_GENERATION_VERSION};
use crate::genome::{
    apply_mutation, genome_hash, propose_mutation, Genome, MutationCategory, MutationRecord,
    MutationTrigger,
};
use crate::rng::{streams, Rng};
use crate::seed::{hash_input, world_identity, DreamscapeSeed};

pub const SNAPSHOT_VERSION: u16 = 1;

/// 相机逻辑态（§78；数值约束在渲染层消费，本层只保存）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.35,
            distance: 42.0,
        }
    }
}

/// 世界快照（§79：关键逻辑态）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DreamscapeSnapshot {
    pub snapshot_version: u16,
    pub world_id: u64,
    pub seed: DreamscapeSeed,
    pub generation_version: u16,
    pub biome_id: String,
    pub genome: Genome,
    pub environment: EnvironmentState,
    pub mutation_log: Vec<MutationRecord>,
    pub event_log: Vec<EventEntry>,
    pub camera: CameraState,
    pub saved_at_tick: u64,
}

/// 恢复错误（§105/§106：明确失败，不静默改世界）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreError {
    /// 快照版本不被支持。
    UnsupportedSnapshot(u16),
    /// generation version 不再支持（§106：World version unavailable）。
    UnsupportedGeneration(u16),
    /// 快照损坏（world_id 与内容不匹配）。
    Corrupt,
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSnapshot(v) => write!(f, "snapshot version {v} unsupported"),
            Self::UnsupportedGeneration(v) => {
                write!(f, "world generation version {v} unavailable")
            }
            Self::Corrupt => write!(f, "snapshot corrupt (world identity mismatch)"),
        }
    }
}

impl DreamscapeSnapshot {
    /// 从会话状态捕获（DTO 构造；字段即快照结构，不拆参数对象）。
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn capture(
        world_id: u64,
        seed: DreamscapeSeed,
        biome_id: &str,
        genome: &Genome,
        env: &EnvironmentState,
        mutation_log: &[MutationRecord],
        event_log: &[EventEntry],
        camera: CameraState,
        saved_at_tick: u64,
    ) -> Self {
        Self {
            snapshot_version: SNAPSHOT_VERSION,
            world_id,
            seed,
            generation_version: SUPPORTED_GENERATION_VERSION,
            biome_id: biome_id.to_string(),
            genome: *genome,
            environment: *env,
            mutation_log: mutation_log.to_vec(),
            event_log: event_log.to_vec(),
            camera,
            saved_at_tick,
        }
    }

    /// 校验快照一致性（§81/§106 门）。
    ///
    /// # Errors
    /// [`RestoreError`]：版本不支持 / world_id 不匹配。
    pub fn validate(&self) -> Result<(), RestoreError> {
        if self.snapshot_version != SNAPSHOT_VERSION {
            return Err(RestoreError::UnsupportedSnapshot(self.snapshot_version));
        }
        if !generation_supported(self.generation_version) {
            return Err(RestoreError::UnsupportedGeneration(self.generation_version));
        }
        // world_id 一致性：快照未被手改（§18 身份稳定）。
        let expect = world_identity(
            &self.seed,
            self.generation_version,
            self.biome_id.as_bytes(),
        );
        if expect != self.world_id {
            return Err(RestoreError::Corrupt);
        }
        Ok(())
    }
}

/// Dreamscape 会话（§77/§78）：世界持久 + 逻辑时间推进 + 显式再生。
#[derive(Debug, Clone)]
pub struct DreamscapeSession {
    pub seed: DreamscapeSeed,
    pub genome: Genome,
    pub biome: Biome,
    pub world_id: u64,
    pub environment: EnvironmentState,
    pub mutation_log: Vec<MutationRecord>,
    pub event_log: Vec<EventEntry>,
    pub camera: CameraState,
    /// 已应用的最大 mutation tick（触发去重）。
    last_mutation_tick: u64,
}

/// 缺省 seed（§77/§103：不自动 regenerate——所有用户默认进入同一
/// 「故乡」世界，除非显式导入/再生）。
pub const DEFAULT_SEED_INPUT: &str = "ifuyo-aura-default";

impl DreamscapeSession {
    /// 新会话（持久化不存在时的首启路径）。
    #[must_use]
    pub fn new(seed: DreamscapeSeed, tick: u64) -> Self {
        let genome = Genome::from_seed(&seed);
        let biome = Biome::select(&seed, &genome);
        let world_id = world_identity(
            &seed,
            SUPPORTED_GENERATION_VERSION,
            biome.kind.id().as_bytes(),
        );
        let environment = EnvironmentState::initial(&seed, tick);
        Self {
            seed,
            genome,
            biome,
            world_id,
            environment,
            mutation_log: Vec::new(),
            event_log: events::to_log(&events::simulate_events(&seed, 0, tick)),
            camera: CameraState::default(),
            last_mutation_tick: tick,
        }
    }

    /// 推进逻辑时间（§30）：环境演化 + 事件日志补记 + 长 idle mutation
    /// （§28：仅 LongIdle 触发，去重）。
    pub fn advance(&mut self, delta_ticks: u64) {
        let now = self.environment.tick + delta_ticks;
        self.environment.advance(&self.seed, delta_ticks);
        // 事件日志补记（确定性模拟；日志只增不改，§93）。
        let new_events =
            events::simulate_events(&self.seed, self.environment.tick - delta_ticks, now);
        let known: std::collections::HashSet<u64> =
            self.event_log.iter().map(|e| e.event_id).collect();
        for e in events::to_log(&new_events) {
            if !known.contains(&e.event_id) {
                self.event_log.push(e);
            }
        }
        // 长 idle mutation（§28）：每满 8 逻辑小时一次，mutation 流确定性。
        let idle_windows = now / crate::genome::IDLE_MUTATION_AFTER_TICKS;
        let prev_windows = self.last_mutation_tick / crate::genome::IDLE_MUTATION_AFTER_TICKS;
        if idle_windows > prev_windows {
            let mut rng = Rng::from_stream_bucket(&self.seed, streams::MUTATION, idle_windows);
            let record = propose_mutation(
                &self.seed,
                &self.genome,
                MutationCategory::Growth,
                MutationTrigger::LongIdle,
                now,
                &mut rng,
            );
            apply_mutation(&mut self.genome, &record);
            self.mutation_log.push(record);
        }
        self.last_mutation_tick = now;
    }

    /// 当前世界场景（manifest 视图；§81 路径的逻辑步）。
    #[must_use]
    pub fn manifest(&self) -> crate::manifest::SceneManifest {
        let world = crate::world::World::generate(&self.seed, &self.genome, &self.biome);
        crate::manifest::SceneManifest::from_world(
            self.world_id,
            &world,
            &self.environment,
            self.biome.effective_mist_bias(),
        )
    }

    /// 显式再生（§103：仅用户主动调用）。
    pub fn regenerate(&mut self, new_input: &str, tick: u64) {
        *self = Self::new(DreamscapeSeed::from_input(new_input), tick);
    }

    /// 导出 seed（§104：人类格式，仅安全内容）。
    #[must_use]
    pub fn export_seed(&self) -> String {
        self.seed.to_human()
    }

    /// 导入 seed（§105：非法 → Err；任意文本输入也可）。
    pub fn import_seed(&mut self, input: &str, tick: u64) -> Result<(), SeedParseError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(SeedParseError::Invalid);
        }
        let seed = DreamscapeSeed::from_input(trimmed);
        *self = Self::new(seed, tick);
        Ok(())
    }

    /// 捕获快照。
    #[must_use]
    pub fn snapshot(&self) -> DreamscapeSnapshot {
        DreamscapeSnapshot::capture(
            self.world_id,
            self.seed,
            self.biome.kind.id(),
            &self.genome,
            &self.environment,
            &self.mutation_log,
            &self.event_log,
            self.camera,
            self.environment.tick,
        )
    }

    /// 从快照恢复会话（§81；§106 版本门）。
    ///
    /// # Errors
    /// [`RestoreError`]。
    pub fn restore(snapshot: &DreamscapeSnapshot) -> Result<Self, RestoreError> {
        snapshot.validate()?;
        let kind =
            crate::biome::BiomeKind::from_id(&snapshot.biome_id).ok_or(RestoreError::Corrupt)?;
        Ok(Self {
            seed: snapshot.seed,
            genome: {
                let mut g = snapshot.genome;
                g.sanitize();
                g
            },
            biome: Biome::with_variation(&snapshot.seed, kind),
            world_id: snapshot.world_id,
            environment: {
                let mut e = snapshot.environment;
                e.sanitize();
                e
            },
            mutation_log: snapshot.mutation_log.clone(),
            event_log: snapshot.event_log.clone(),
            camera: snapshot.camera,
            last_mutation_tick: snapshot.saved_at_tick,
        })
    }
}

/// genome parent hash 透出（诊断用）。
#[must_use]
pub fn session_genome_hash(s: &DreamscapeSession) -> u64 {
    genome_hash(&s.genome)
}

/// 输入归一化复用（shell 侧 seed 解析一致性）。
#[must_use]
pub fn normalize_seed_input(input: &str) -> u64 {
    hash_input(input) & crate::seed::SEED_VALUE_MASK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_persists_across_reopen() {
        // §77：明天回来仍是同一世界（同 seed → 同 world_id）。
        let a = DreamscapeSession::new(DreamscapeSeed::from_input("home"), 0);
        let b = DreamscapeSession::new(DreamscapeSeed::from_input("home"), 999_999);
        assert_eq!(a.world_id, b.world_id);
        assert_eq!(a.genome, b.genome, "genome 同（世界形态不变）");
    }

    #[test]
    fn snapshot_roundtrip_restores_same_world() {
        // §79–§81：捕获 → 序列化 → 恢复 → 同一世界 + 环境。
        let mut s = DreamscapeSession::new(DreamscapeSeed::from_input("snap"), 500);
        s.advance(10_000);
        let snap = s.snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        let back: DreamscapeSnapshot = serde_json::from_str(&json).unwrap();
        let restored = DreamscapeSession::restore(&back).unwrap();
        assert_eq!(restored.world_id, s.world_id);
        assert_eq!(restored.seed, s.seed);
        assert_eq!(restored.genome, s.genome);
        assert_eq!(restored.environment, s.environment);
        assert_eq!(restored.mutation_log, s.mutation_log);
        assert_eq!(restored.event_log, s.event_log);
        assert_eq!(restored.biome.kind, s.biome.kind);
        // 逻辑态 → 场景一致（scene_hash 相等，§81/§114）。
        assert_eq!(restored.manifest().scene_hash(), s.manifest().scene_hash());
    }

    #[test]
    fn snapshot_has_no_gpu_objects() {
        // §80：快照 JSON 只含逻辑态数值/枚举（结构性保证：全部字段为
        // 数值/字符串/枚举；此处锁 JSON 不含渲染对象键）。
        let s = DreamscapeSession::new(DreamscapeSeed::from_input("nogpu"), 0);
        let json = serde_json::to_string(&s.snapshot()).unwrap();
        for banned in ["mesh", "Mesh", "texture", "Texture", "renderer", "webgl"] {
            assert!(!json.contains(banned), "{banned} 不应在快照中");
        }
    }

    #[test]
    fn restore_rejects_unknown_generation_and_corruption() {
        let mut s = DreamscapeSession::new(DreamscapeSeed::from_input("gate"), 0);
        s.advance(100);
        let mut snap = s.snapshot();
        snap.generation_version = 99;
        assert_eq!(
            DreamscapeSession::restore(&snap).unwrap_err(),
            RestoreError::UnsupportedGeneration(99),
            "§106：不再支持的版本明确报错"
        );
        let mut snap2 = s.snapshot();
        snap2.world_id ^= 0xff;
        assert_eq!(
            DreamscapeSession::restore(&snap2).unwrap_err(),
            RestoreError::Corrupt,
            "world_id 不匹配 = 损坏"
        );
        let mut snap3 = s.snapshot();
        snap3.snapshot_version = 42;
        assert_eq!(
            DreamscapeSession::restore(&snap3).unwrap_err(),
            RestoreError::UnsupportedSnapshot(42)
        );
    }

    #[test]
    fn advance_evolution_is_slow_and_deterministic() {
        // §82/§83/§28：演化缓慢、确定性。
        let mut a = DreamscapeSession::new(DreamscapeSeed::from_input("evo"), 0);
        let mut b = DreamscapeSession::new(DreamscapeSeed::from_input("evo"), 0);
        for _ in 0..30 {
            a.advance(24 * 60); // 30 逻辑日
            b.advance(24 * 60);
        }
        assert_eq!(a.genome, b.genome, "确定性");
        assert!(!a.mutation_log.is_empty(), "长 idle 触发 mutation");
        assert!(
            a.mutation_log.len() <= 30,
            "每 idle 窗口至多一次（§27/§28）"
        );
        // §83：漂移有界（每字段 ≤ 若干小步）。
        let base = Genome::from_seed(&DreamscapeSeed::from_input("evo"));
        for f in [
            crate::genome::GenomeField::FloraDensity,
            crate::genome::GenomeField::MistDensity,
        ] {
            let drift = (a.genome.get(f) - base.get(f)).abs();
            let (min, max, _) = f.bounds();
            assert!(drift <= (max - min) * 0.5, "演化不毁世界（§83）");
        }
    }

    #[test]
    fn event_log_only_grows_and_dedups() {
        let mut s = DreamscapeSession::new(DreamscapeSeed::from_input("elog"), 0);
        let before = s.event_log.len();
        s.advance(200 * 24 * 60);
        s.advance(0); // 零推进不重复记录
        let after = s.event_log.len();
        assert!(after >= before);
        let ids: std::collections::HashSet<u64> = s.event_log.iter().map(|e| e.event_id).collect();
        assert_eq!(ids.len(), s.event_log.len(), "去重（§93 日志不重复）");
    }

    #[test]
    fn seed_export_import_roundtrip() {
        // §104/§105：导出人类格式 → 导入 → 同一世界。
        let s = DreamscapeSession::new(DreamscapeSeed::from_input("voyage"), 0);
        let exported = s.export_seed();
        assert!(exported.starts_with("DS-"));
        assert!(
            !exported.contains(['/', '\\', ':']),
            "无路径/凭据内容（§104）"
        );
        let mut t = DreamscapeSession::new(DreamscapeSeed::from_input("other"), 0);
        t.import_seed(&exported, 0).unwrap();
        assert_eq!(t.world_id, s.world_id);
        // 空输入拒绝（§105 不 crash）。
        assert_eq!(
            t.import_seed("   ", 0).unwrap_err(),
            SeedParseError::Invalid
        );
        // 任意文本也能成为 seed（normalize 路径）。
        t.import_seed("  Quiet Harbor  ", 0).unwrap();
        assert_eq!(t.seed, DreamscapeSeed::from_input("Quiet Harbor"));
    }

    #[test]
    fn regenerate_explicit_only() {
        // §103：只有显式 regenerate 改变世界。
        let mut s = DreamscapeSession::new(DreamscapeSeed::from_input("regen"), 0);
        let id0 = s.world_id;
        s.advance(50_000);
        assert_eq!(s.world_id, id0, "推进/重开不换世界");
        s.regenerate("a-new-place", 0);
        assert_ne!(s.world_id, id0);
    }

    #[test]
    fn default_session_stable() {
        let a = DreamscapeSession::new(DreamscapeSeed::from_input(DEFAULT_SEED_INPUT), 0);
        let b = DreamscapeSession::new(DreamscapeSeed::from_input(DEFAULT_SEED_INPUT), 7);
        assert_eq!(a.world_id, b.world_id, "首启缺省世界稳定（§77）");
    }
}
