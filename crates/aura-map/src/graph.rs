//! kNN 图（§51–§67，D-112）：top-K → 对称化无向图（§52/§63）→ 去重 →
//! 阈值过滤（§55/§56）→ 校验（§61）。
//!
//! - 边（§53）：source/target/similarity；§54 无自环、有限相似度。
//! - 负相似（§55）：cosine < MIN_SIMILARITY(0) 不成边。
//! - 边预算（§58/§59）：max_edges = N × K（对称化后仍 ≤ 2·N·K，K ≤ 16）。
//! - 孤立节点（§60）：允许——不强行连接。

use crate::dataset::MapDataset;
use crate::similarity::top_k_neighbors;
use serde::{Deserialize, Serialize};

pub const GRAPH_ALGO_VERSION: u16 = 1;
/// §57/§58：边预算系数（max_edges ≈ N × K）。
pub const MAX_EDGES_FACTOR: usize = 16;

/// 无向边（canonical：a < b；§63 对称化后单条存储）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MapEdge {
    pub a: i64,
    pub b: i64,
    /// cosine 相似度 [0,1]（阈值过滤后）。
    pub similarity: f32,
}

/// kNN 图（对称无向；孤立节点不在 edges 中但仍属于 dataset 节点集）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapGraph {
    pub nodes: Vec<i64>,
    pub edges: Vec<MapEdge>,
    /// 孤立（零边）节点。
    pub isolated: Vec<i64>,
    pub k: usize,
    pub graph_algo_version: u16,
}

impl MapGraph {
    /// 节点总数（dataset 全量——孤立节点也是节点）。
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 邻接查询（§36 相邻节点）。
    #[must_use]
    pub fn neighbors_of(&self, track_id: i64) -> Vec<(i64, f32)> {
        let mut out = Vec::new();
        for e in &self.edges {
            if e.a == track_id {
                out.push((e.b, e.similarity));
            } else if e.b == track_id {
                out.push((e.a, e.similarity));
            }
        }
        out
    }
}

/// 图校验错误（§61）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphInvalid {
    SelfLoop,
    UnknownNode,
    NonFiniteWeight,
    DuplicateEdge,
    NotSymmetric,
    EdgeBudget,
}

/// §61 validate_graph。
pub fn validate_graph(g: &MapGraph, k: usize) -> Result<(), GraphInvalid> {
    if g.edges.len() > g.nodes.len() * k.max(1) * 2 {
        return Err(GraphInvalid::EdgeBudget);
    }
    use std::collections::HashSet;
    let nodes: HashSet<i64> = g.nodes.iter().copied().collect();
    let mut seen: HashSet<(i64, i64)> = HashSet::new();
    for e in &g.edges {
        if e.a == e.b {
            return Err(GraphInvalid::SelfLoop);
        }
        if !nodes.contains(&e.a) || !nodes.contains(&e.b) {
            return Err(GraphInvalid::UnknownNode);
        }
        if !e.similarity.is_finite() {
            return Err(GraphInvalid::NonFiniteWeight);
        }
        let canon = if e.a < e.b { (e.a, e.b) } else { (e.b, e.a) };
        if !seen.insert(canon) {
            return Err(GraphInvalid::DuplicateEdge);
        }
    }
    Ok(())
}

/// 构建 kNN 对称图（§51–§63）。
///
/// # Errors
/// 相似度计算失败（dataset 层已校验，罕见）。
pub fn build_knn_graph(
    ds: &MapDataset,
    k: usize,
    min_similarity: f32,
) -> Result<MapGraph, crate::similarity::SimilarityError> {
    let k = k.clamp(1, 32);
    let mut nodes = Vec::with_capacity(ds.entries.len());
    let mut isolated = Vec::new();
    // §59：边预算 N × K。
    let mut edges: Vec<MapEdge> = Vec::with_capacity(ds.entries.len() * k);
    let mut degree: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    let mut seen: std::collections::HashSet<(i64, i64)> = std::collections::HashSet::new();

    for e in &ds.entries {
        nodes.push(e.track_id);
        let neighbors = top_k_neighbors(ds, e.track_id, k)?;
        let mut has_edge = false;
        for (nbr, sim, _rank) in neighbors {
            // §55/§56：负/零相似不成边。
            if sim <= min_similarity {
                continue;
            }
            // canonical 无向键 + 去重（§62）。
            let (a, b) = if e.track_id < nbr {
                (e.track_id, nbr)
            } else {
                (nbr, e.track_id)
            };
            if !seen.insert((a, b)) {
                continue;
            }
            edges.push(MapEdge {
                a,
                b,
                similarity: sim,
            });
            *degree.entry(a).or_insert(0) += 1;
            *degree.entry(b).or_insert(0) += 1;
            has_edge = true;
        }
        if !has_edge {
            isolated.push(e.track_id);
        }
    }
    let _ = &mut degree;
    edges.sort_by_key(|e| (e.a, e.b));

    Ok(MapGraph {
        nodes,
        edges,
        isolated,
        k,
        graph_algo_version: GRAPH_ALGO_VERSION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::MapEntry;
    use aura_analysis::domain::Embedding;
    use aura_analysis::domain::EMBEDDING_MODEL_ID;

    fn emb(v: Vec<f32>) -> Embedding {
        Embedding::normalized(v, EMBEDDING_MODEL_ID, 1)
    }

    fn ds_with(vectors: Vec<(i64, Vec<f32>)>) -> MapDataset {
        let entries: Vec<MapEntry> = vectors
            .into_iter()
            .map(|(id, v)| MapEntry {
                track_id: id,
                content_identity: format!("c{id}"),
                analysis_key: format!("k{id}"),
                embedding: emb(v),
            })
            .collect();
        MapDataset::build(entries, None).unwrap()
    }

    #[test]
    fn two_clusters_form_symmetric_edges() {
        // 双簇：x 轴两团 → 簇内互连。
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0, 0.0]),
            (2, vec![0.9, 0.1, 0.0]),
            (3, vec![0.95, 0.05, 0.0]),
            (4, vec![0.0, 1.0, 0.0]),
            (5, vec![0.1, 0.9, 0.0]),
            (6, vec![0.0, 0.95, 0.05]),
        ]);
        let g = build_knn_graph(&ds, 2, 0.0).unwrap();
        assert_eq!(validate_graph(&g, 2), Ok(()));
        for e in &g.edges {
            // §63：canonical 无向（a < b）。
            assert!(e.a < e.b);
            // §63：对称一致——对称边不重复出现（去重 §62）。
            assert!(!g.edges.iter().any(|o| o.a == e.b && o.b == e.a));
        }
        // 簇内边存在。
        assert!(g
            .edges
            .iter()
            .any(|e| (e.a == 1 && e.b == 2) || (e.a == 1 && e.b == 3) || (e.a == 2 && e.b == 3)));
        assert!(g
            .edges
            .iter()
            .any(|e| (e.a == 4 && e.b == 5) || (e.a == 4 && e.b == 6) || (e.a == 5 && e.b == 6)));
    }

    #[test]
    fn isolated_node_allowed() {
        // §60：不强行连接。
        // 单位圆正交分布 + 一个零信息节点无法避免——改用高维：2 节点 + 1 孤立。
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0]),
            (2, vec![1.0, 0.05]),
            (3, vec![-1.0, 0.0]),
        ]);
        let g = build_knn_graph(&ds, 1, 0.5).unwrap();
        // 3 与 1/2 相似度为负/低 → 被阈值过滤 → 孤立。
        assert!(g.isolated.contains(&3), "isolated={:?}", g.isolated);
        assert!(validate_graph(&g, 1).is_ok(), "validate");
    }

    #[test]
    fn self_loop_never() {
        let ds = ds_with(vec![(1, vec![1.0, 0.0]), (2, vec![0.0, 1.0])]);
        let g = build_knn_graph(&ds, 1, 0.0).unwrap();
        assert!(g.edges.iter().all(|e| e.a != e.b), "§54");
    }

    #[test]
    fn edge_budget_respected() {
        let n = 200usize;
        let ds = ds_with((0..n as i64).map(|i| (i, vec![1.0, 0.0])).collect());
        let g = build_knn_graph(&ds, 16, 0.0).unwrap();
        assert!(g.edges.len() <= n * MAX_EDGES_FACTOR, "§58 边预算");
    }

    #[test]
    fn deterministic() {
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0]),
            (2, vec![0.9, 0.1]),
            (3, vec![0.0, 1.0]),
        ]);
        let g1 = build_knn_graph(&ds, 2, 0.0).unwrap();
        let g2 = build_knn_graph(&ds, 2, 0.0).unwrap();
        assert_eq!(g1, g2);
    }

    #[test]
    fn threshold_filters_weak_edges() {
        // §56：正交对 (sim≈0) 被阈值 0.3 过滤 → 全孤立。
        let ds = ds_with(vec![(1, vec![1.0, 0.0]), (2, vec![0.0, 1.0])]);
        let g = build_knn_graph(&ds, 1, 0.3).unwrap();
        assert!(g.edges.is_empty());
        assert_eq!(g.isolated.len(), 2);
    }
}
