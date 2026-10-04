//! Map 持久化（§64–§67，D-115）：JSON（pretty）序列化 dataset hash/
//! 版本/位置/簇——derived data（§65 可全量重建）；损坏/版本不符 →
//! Err（调用方删除重建，不影响 M3 §50）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::clustering::MapCluster;
use crate::dataset::MapDataset;
use crate::graph::MapGraph;
use crate::layout::MapPosition;

/// 持久化结构（§66 metadata/hash/版本/位置/簇 id）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedMap {
    pub format_version: u16,
    pub map_algo_version: u16,
    pub analysis_version: u16,
    pub dataset_hash: String,
    pub model_id: String,
    pub model_version: u16,
    pub embedding_dimension: u32,
    pub k: usize,
    /// kNN 边随位置一并持久化（§174 恢复免重算图）。
    pub edges: Vec<crate::graph::MapEdge>,
    pub graph_algo_version: u16,
    pub cluster_algo_version: u16,
    pub positions: BTreeMap<i64, MapPosition>,
    pub clusters: Vec<MapCluster>,
    pub isolated: Vec<i64>,
    /// §159 精神：载荷校验和。
    pub checksum: u64,
}

pub const PERSIST_FORMAT_VERSION: u16 = 1;

/// 从当前构建序列化。
#[must_use]
pub fn persist(
    ds: &MapDataset,
    g: &MapGraph,
    clusters: &[MapCluster],
    positions: &BTreeMap<i64, MapPosition>,
) -> PersistedMap {
    let mut m = PersistedMap {
        format_version: PERSIST_FORMAT_VERSION,
        map_algo_version: ds.map_algo_version,
        analysis_version: ds.analysis_version,
        dataset_hash: ds.dataset_hash.clone(),
        model_id: ds.compatibility.model_id.clone(),
        model_version: ds.compatibility.model_version,
        embedding_dimension: ds.compatibility.dimension,
        k: g.k,
        edges: g.edges.clone(),
        graph_algo_version: g.graph_algo_version,
        cluster_algo_version: crate::clustering::CLUSTER_ALGO_VERSION,
        positions: positions.clone(),
        clusters: clusters.to_vec(),
        isolated: g.isolated.clone(),
        checksum: 0,
    };
    m.checksum = payload_checksum(&m);
    m
}

/// §159：载荷校验和（integrity 字段归零后计算）。
#[must_use]
pub fn payload_checksum(m: &PersistedMap) -> u64 {
    let mut copy = m.clone();
    copy.checksum = 0;
    let mut h = 0u64;
    for b in serde_json::to_vec(&copy).unwrap_or_default() {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01B3);
    }
    h
}

/// §322 占位：load 语义由 validate+调用方重建承担（derived data §65）。
pub fn load_map_placeholder() {}

/// 加载 + 校验（§67/§335/§337/§338/§339/§340）：
/// 格式版本/map 版本/analysis 版本/dataset hash/checksum 全部通过才
/// 返回 Some；否则 None（调用方删除重建——derived data §65）。
#[must_use]
pub fn validate(p: &PersistedMap, expected_dataset_hash: &str, ds: &MapDataset) -> bool {
    p.format_version == PERSIST_FORMAT_VERSION
        && p.map_algo_version == ds.map_algo_version
        && p.analysis_version == ds.analysis_version
        && p.graph_algo_version == crate::graph::GRAPH_ALGO_VERSION
        && p.cluster_algo_version == crate::clustering::CLUSTER_ALGO_VERSION
        && p.model_id == ds.compatibility.model_id
        && p.model_version == ds.compatibility.model_version
        && p.embedding_dimension == ds.compatibility.dimension
        && p.dataset_hash == expected_dataset_hash
        && p.checksum == payload_checksum(p)
}

/// §174/§175：dataset 未变时从持久化直接恢复完整快照（图/聚类/位置
/// 不重算）；§189 Ready 语义在此复核（图校验 + §190 有限数值）。
///
/// # Errors
/// 校验不过（版本/hash/checksum/图/NaN）→ [`String`]（调用方按 §187
/// 删除重建，不影响 M3）。
pub fn restore_map(
    p: &PersistedMap,
    expected_dataset_hash: &str,
    ds: &MapDataset,
) -> Result<crate::build::MapBuildOutput, String> {
    if !validate(p, expected_dataset_hash, ds) {
        return Err("persisted map failed validation (version/hash/checksum)".to_string());
    }
    let g = crate::graph::MapGraph {
        nodes: ds.entries.iter().map(|e| e.track_id).collect(),
        edges: p.edges.clone(),
        isolated: p.isolated.clone(),
        k: p.k,
        graph_algo_version: p.graph_algo_version,
    };
    crate::graph::validate_graph(&g, g.k).map_err(|e| format!("graph invalid: {e:?}"))?;
    // §190/§193：非有限位置/簇统计不得进入渲染层。
    for pos in p.positions.values() {
        if !pos.x.is_finite() || !pos.y.is_finite() {
            return Err("non-finite position".to_string());
        }
    }
    for c in &p.clusters {
        if let Some((cx, cy)) = c.centroid {
            if !cx.is_finite() || !cy.is_finite() || !c.radius.unwrap_or(0.0).is_finite() {
                return Err("non-finite cluster stats".to_string());
            }
        }
    }
    Ok(crate::build::MapBuildOutput {
        dataset: ds.clone(),
        graph: g,
        clusters: p.clusters.clone(),
        positions: p.positions.clone(),
        persisted: p.clone(),
        status: crate::build::MapBuildStatus::Ready,
        build_id: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clustering::detect_clusters;
    use crate::dataset::MapEntry;
    use aura_analysis::domain::{Embedding, EMBEDDING_MODEL_ID};

    fn emb(v: Vec<f32>) -> Embedding {
        Embedding::normalized(v, EMBEDDING_MODEL_ID, 1)
    }

    fn fixture() -> (
        MapDataset,
        MapGraph,
        Vec<MapCluster>,
        BTreeMap<i64, MapPosition>,
        PersistedMap,
    ) {
        let ds = MapDataset::build(
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
            ],
            None,
        )
        .unwrap();
        let g = crate::graph::build_knn_graph(&ds, 1, 0.0).unwrap();
        let mut clusters = detect_clusters(&g, 10).clusters;
        let layout = crate::layout::compute_layout(&ds, &g).unwrap();
        let positions: BTreeMap<i64, MapPosition> =
            layout.positions.iter().map(|(k, v)| (*k, *v)).collect();
        crate::clustering::attach_spatial_stats(&mut clusters, &positions);
        let persisted = persist(&ds, &g, &clusters, &positions);
        (ds, g, clusters, positions, persisted)
    }

    #[test]
    fn roundtrip_and_validation() {
        // §280：save → load → validate。
        let (ds, _g, _c, _p, persisted) = fixture();
        let json = serde_json::to_vec(&persisted).unwrap();
        let back: PersistedMap = serde_json::from_slice(&json).unwrap();
        assert!(validate(&back, &ds.dataset_hash, &ds), "§67 合同校验通过");
    }

    #[test]
    fn dataset_hash_mismatch_rejected() {
        // §67：dataset hash 不符 → 重建。
        let (ds, _g, _c, _p, persisted) = fixture();
        assert!(!validate(&persisted, "deadbeef", &ds));
    }

    #[test]
    fn corrupted_checksum_rejected() {
        // §158/§249：损坏 → 标记无效（调用方重建），不 crash。
        let (ds, _g, _c, _p, mut persisted) = fixture();
        persisted.checksum ^= 0xFF;
        assert!(!validate(&persisted, &ds.dataset_hash, &ds));
    }

    #[test]
    fn malformed_json_never_panics() {
        let bad = b"{ not json";
        let parsed: Result<PersistedMap, _> = serde_json::from_slice(bad);
        assert!(parsed.is_err());
    }

    #[test]
    fn restore_without_recompute() {
        // §174：dataset 未变 → 直接恢复 Ready 快照（图/簇/位置保真）。
        let (ds, g, _c, positions, persisted) = fixture();
        let out = restore_map(&persisted, &ds.dataset_hash, &ds).unwrap();
        assert_eq!(out.status, crate::build::MapBuildStatus::Ready, "§189");
        assert_eq!(out.graph.edges, g.edges, "图边保真");
        assert_eq!(out.positions, positions, "位置保真");
        // §175：dataset hash 不符 → 拒绝（重建路径）。
        assert!(restore_map(&persisted, "other-hash", &ds).is_err());
    }
}
