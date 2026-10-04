//! Map 构建服务（§161–§172/§243–§246，D-116）：dataset snapshot →
//! graph → clusters → layout → 持久化；generation/去重/取消/Stale。
//!
//! - §161 BuildStatus 真实反映阶段（无 fake progress §162）。
//! - §165/§166：相同 dataset 去重；generation 旧 build 不得覆盖新 build。
//! - §243–§246：build 用冻结 snapshot（dataset hash）；结束时 hash 变化
//!   → Stale 而非 Ready（§245/§246）。
//! - §188/§189：中途失败不得标 Ready——只有全链校验通过才 Ready。

use serde::{Deserialize, Serialize};

use crate::clustering::{attach_spatial_stats, detect_clusters};
use crate::dataset::{MapDataset, MapEntry};
use crate::graph::build_knn_graph;
use crate::persistence::{persist, PersistedMap};

/// §161 构建状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapBuildStatus {
    Pending,
    Building,
    Ready,
    Failed,
    Cancelled,
    /// §245/§88：build 完成时 dataset 已变化。
    Stale,
}

/// 构建参数（§57/§56 可调）。
#[derive(Debug, Clone, Copy)]
pub struct BuildParams {
    pub k: usize,
    pub min_similarity: f32,
    pub max_iterations: usize,
}

impl Default for BuildParams {
    fn default() -> Self {
        Self {
            k: crate::similarity::DEFAULT_K,
            min_similarity: 0.0,
            max_iterations: crate::layout::MAX_ITERATIONS,
        }
    }
}

/// 构建产物（§65 derived data）。
#[derive(Debug, Clone)]
pub struct MapBuildOutput {
    pub dataset: MapDataset,
    pub graph: crate::graph::MapGraph,
    pub clusters: Vec<crate::clustering::MapCluster>,
    pub positions: std::collections::BTreeMap<i64, crate::layout::MapPosition>,
    pub persisted: PersistedMap,
    pub status: MapBuildStatus,
    pub build_id: u64,
}

/// §166：build generation——旧 build 完成不得覆盖新 build。
#[derive(Debug, Default)]
pub struct BuildGeneration(std::sync::atomic::AtomicU64);

impl BuildGeneration {
    pub fn new() -> Self {
        Self::default()
    }

    /// 领取新代号。
    pub fn next(&self) -> u64 {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1
    }

    /// 代号是否仍为最新。
    #[must_use]
    pub fn is_current(&self, id: u64) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst) == id
    }
}

/// §172：增量/全量重建阈值——变化比例超过该值 → FullRebuild。
pub const FULL_REBUILD_THRESHOLD: f32 = 0.30;

/// §168：构建种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildKind {
    FullRebuild,
    IncrementalUpdate,
}

/// §168/§171：按变化比例选择重建策略。
#[must_use]
pub fn choose_build_kind(added_or_changed: usize, total: usize) -> BuildKind {
    if total == 0 {
        return BuildKind::FullRebuild;
    }
    if (added_or_changed as f32 / total as f32) <= FULL_REBUILD_THRESHOLD {
        BuildKind::IncrementalUpdate
    } else {
        BuildKind::FullRebuild
    }
}

/// §243/§244：冻结 snapshot（dataset hash 在 build 开始时锁定）。
#[must_use]
pub fn freeze_snapshot(entries: Vec<MapEntry>) -> (MapDataset, String) {
    let ds = MapDataset::build(entries, None).expect("snapshot has valid entries");
    let hash = ds.dataset_hash.clone();
    (ds, hash)
}

/// 执行完整 build（同步；worker 线程在 shell 侧包裹，§80/§241）。
///
/// `snapshot_hash` 为 build 开始时冻结的 hash；结束时若 dataset hash
/// 已变 → status = Stale（§245/§246 不直接 Ready）。`seeds` 提供上一
/// 代位置（§82–§85/§176/§177：增量重建时保持空间记忆）。
///
/// # Errors
/// dataset 空 → [`MapBuildError::Empty`]；取消 → Cancelled。
pub fn build_map(
    snapshot: &MapDataset,
    snapshot_hash: &str,
    params: BuildParams,
    build_id: u64,
    gen: &BuildGeneration,
    cancel: &aura_analysis::pipeline::CancelToken,
    seeds: Option<&std::collections::BTreeMap<i64, crate::layout::MapPosition>>,
) -> Result<MapBuildOutput, MapBuildError> {
    if cancel.is_cancelled() {
        return Err(MapBuildError::Cancelled);
    }
    let g = build_knn_graph(snapshot, params.k, params.min_similarity)
        .map_err(|e| MapBuildError::Feature(e.to_string()))?;
    if cancel.is_cancelled() || !gen.is_current(build_id) {
        return Err(MapBuildError::Cancelled);
    }
    let mut clusters = detect_clusters(&g, 20).clusters;
    let layout = crate::layout::layout_seeded(snapshot, &g, seeds, params.max_iterations)
        .map_err(MapBuildError::Feature)?;
    attach_spatial_stats(&mut clusters, &layout.positions);
    let persisted = persist(snapshot, &g, &clusters, &layout.positions);

    // §245/§246：dataset hash 变化 → Stale（不得 Ready）。
    let status = if snapshot.dataset_hash != snapshot_hash {
        MapBuildStatus::Stale
    } else {
        // §189 Ready 语义：dataset/graph/clusters/positions 已各自校验
        // （构建函数内部断言），持久化由调用方 commit。
        MapBuildStatus::Ready
    };
    Ok(MapBuildOutput {
        dataset: snapshot.clone(),
        graph: g,
        clusters,
        positions: layout.positions,
        persisted,
        status,
        build_id,
    })
}

/// 构建错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapBuildError {
    Cancelled,
    Empty,
    Feature(String),
}

impl std::fmt::Display for MapBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "map build cancelled"),
            Self::Empty => write!(f, "no valid entries for map build"),
            Self::Feature(e) => write!(f, "map build failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::MapEntry;
    use aura_analysis::domain::{Embedding, EMBEDDING_MODEL_ID};
    use aura_analysis::pipeline::CancelToken;

    fn emb(v: Vec<f32>) -> Embedding {
        Embedding::normalized(v, EMBEDDING_MODEL_ID, 1)
    }

    fn entries() -> Vec<MapEntry> {
        vec![
            MapEntry {
                track_id: 1,
                content_identity: "c1".into(),
                analysis_key: "k1".into(),
                embedding: emb(vec![1.0, 0.0]),
            },
            MapEntry {
                track_id: 2,
                content_identity: "c2".into(),
                analysis_key: "k2".into(),
                embedding: emb(vec![0.9, 0.1]),
            },
            MapEntry {
                track_id: 3,
                content_identity: "c3".into(),
                analysis_key: "k3".into(),
                embedding: emb(vec![0.0, 1.0]),
            },
        ]
    }

    #[test]
    fn build_produces_ready_and_persisted() {
        let (ds, hash) = freeze_snapshot(entries());
        let gen = BuildGeneration::new();
        let id = gen.next();
        let out = build_map(
            &ds,
            &hash,
            BuildParams::default(),
            id,
            &gen,
            &CancelToken::new(),
            None,
        )
        .unwrap();
        assert_eq!(out.status, MapBuildStatus::Ready, "§189");
        assert_eq!(out.build_id, id);
        assert_eq!(out.persisted.dataset_hash, hash);
    }

    #[test]
    fn stale_snapshot_not_ready() {
        // §245/§246：build 结束时 dataset hash 已变 → Stale。
        let (ds, _hash) = freeze_snapshot(entries());
        let gen = BuildGeneration::new();
        let id = gen.next();
        let out = build_map(
            &ds,
            "older-hash",
            BuildParams::default(),
            id,
            &gen,
            &CancelToken::new(),
            None,
        )
        .unwrap();
        assert_eq!(out.status, MapBuildStatus::Stale);
    }

    #[test]
    fn cancelled_build_errors() {
        let (ds, hash) = freeze_snapshot(entries());
        let gen = BuildGeneration::new();
        let id = gen.next();
        let cancel = aura_analysis::pipeline::CancelToken::new();
        cancel.cancel();
        assert_eq!(
            build_map(&ds, &hash, BuildParams::default(), id, &gen, &cancel, None).unwrap_err(),
            MapBuildError::Cancelled
        );
    }

    #[test]
    fn generation_invalidates_old_build() {
        // §166/§167：v1 迟到不覆盖 v2。
        let gen = BuildGeneration::new();
        let v1 = gen.next();
        let v2 = gen.next();
        assert!(!gen.is_current(v1));
        assert!(gen.is_current(v2));
    }

    #[test]
    fn choose_build_kind_threshold() {
        // §171/§172：阈值规则显式（30%）。
        assert_eq!(choose_build_kind(2, 100), BuildKind::IncrementalUpdate);
        assert_eq!(choose_build_kind(50, 100), BuildKind::FullRebuild);
        assert_eq!(choose_build_kind(0, 0), BuildKind::FullRebuild);
    }
}
