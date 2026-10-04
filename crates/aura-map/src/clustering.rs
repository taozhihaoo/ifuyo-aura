//! 确定性聚类（§98–§108，D-113）：图上的 label propagation
//! （确定性遍历/tie-break），canonical cluster 身份 + 统计。
//!
//! - §99：使用真实图结构（非 hash(track)%k）。
//! - §100：同 dataset + 同版本 + 同参数 → 同分配（固定遍历序 + 确定
//!   tie-break：标签取邻域中（计数最大，计数同取标签最小）者；异步
//!   更新按节点 id 升序）。
//! - §101/§102：canonical 重编号——按（size DESC, 最小 track_id ASC）。
//! - §104：不伪造语义标签（只 Cluster 01/02… 由 UI 层按 canonical 序
//!   命名）。

use crate::graph::MapGraph;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub const CLUSTER_ALGO_VERSION: u16 = 1;

/// 单簇统计（§103/§105：count/centroid/radius/density——来自图与位置，
/// metadata 统计（genre/artist）由应用层补充，缺失不猜）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapCluster {
    /// canonical 序号（0 起，重编号规则见模块注释）。
    pub cluster_id: usize,
    pub track_ids: Vec<i64>,
    /// 质心（布局坐标；布局前为 None）。
    pub centroid: Option<(f32, f32)>,
    /// 半径 = 质心到成员最大距离。
    pub radius: Option<f32>,
}

/// 聚类结果：track_id → cluster_id + 簇列表（canonical 序）。
#[derive(Debug, Clone, PartialEq)]
pub struct ClusteringResult {
    pub assignment: HashMap<i64, usize>,
    pub clusters: Vec<MapCluster>,
}

/// 确定性 label propagation（§100）。
///
/// 输入图（对称无向）；孤立节点自成单簇。遍历按节点 id 升序，标签
/// tie-break 取最小标签——全确定性。
#[must_use]
pub fn detect_clusters(g: &MapGraph, max_iterations: usize) -> ClusteringResult {
    let nodes: Vec<i64> = {
        let mut v = g.nodes.clone();
        v.sort_unstable();
        v
    };
    let index: HashMap<i64, usize> = nodes
        .iter()
        .copied()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();

    // 邻接表。
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for e in &g.edges {
        let (ai, bi) = (index[&e.a], index[&e.b]);
        adj[ai].push(bi);
        adj[bi].push(ai);
    }

    // 初始：每节点独立标签 = 自身 index。
    let mut labels: Vec<usize> = (0..nodes.len()).collect();
    for _ in 0..max_iterations {
        let mut changed = false;
        // §100：异步 LPA，节点 id 升序。
        for i in 0..nodes.len() {
            if adj[i].is_empty() {
                continue; // 孤立节点保持自身标签
            }
            let mut counts: HashMap<usize, usize> = HashMap::new();
            for &j in &adj[i] {
                *counts.entry(labels[j]).or_insert(0) += 1;
            }
            // tie-break：计数最大 → 标签最小。
            let best = counts
                .iter()
                .map(|(label, count)| (*label, *count))
                .min_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)))
                .map(|(label, _)| label);
            if let Some(best) = best {
                if labels[i] != best {
                    labels[i] = best;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    // §101/§102：canonical 重编号——按（size DESC, 最小 track_id ASC）。
    let mut members: HashMap<usize, Vec<i64>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        members.entry(label).or_default().push(nodes[i]);
    }
    for v in members.values_mut() {
        v.sort_unstable();
    }
    let mut order: Vec<(usize, Vec<i64>)> = members.into_iter().collect();
    order.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.1[0].cmp(&b.1[0])));
    let assignment: HashMap<i64, usize> = order
        .iter()
        .enumerate()
        .flat_map(|(cid, (_, ids))| ids.iter().map(move |id| (*id, cid)))
        .collect();
    let clusters = order
        .into_iter()
        .enumerate()
        .map(|(cid, (_, ids))| MapCluster {
            cluster_id: cid,
            track_ids: ids,
            centroid: None,
            radius: None,
        })
        .collect();

    ClusteringResult {
        assignment,
        clusters,
    }
}

/// 布局完成后回填 cluster centroid/radius（§106）。
pub fn attach_spatial_stats(
    clusters: &mut [MapCluster],
    positions: &BTreeMap<i64, crate::layout::MapPosition>,
) {
    for c in clusters {
        let pts: Vec<(f32, f32)> = c
            .track_ids
            .iter()
            .filter_map(|id| positions.get(id).map(|p| (p.x, p.y)))
            .collect();
        if pts.is_empty() {
            continue;
        }
        let cx = pts.iter().map(|p| p.0).sum::<f32>() / pts.len() as f32;
        let cy = pts.iter().map(|p| p.1).sum::<f32>() / pts.len() as f32;
        let radius = pts
            .iter()
            .map(|(x, y)| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt())
            .fold(0.0f32, f32::max);
        c.centroid = Some((cx, cy));
        c.radius = Some(radius);
    }
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

    fn ds_with(vectors: Vec<(i64, Vec<f32>)>) -> crate::dataset::MapDataset {
        let entries: Vec<MapEntry> = vectors
            .into_iter()
            .map(|(id, v)| MapEntry {
                track_id: id,
                content_identity: format!("c{id}"),
                analysis_key: format!("k{id}"),
                embedding: emb(v),
            })
            .collect();
        crate::dataset::MapDataset::build(entries, None).unwrap()
    }

    #[test]
    fn two_communities_detected() {
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0, 0.0]),
            (2, vec![0.9, 0.1, 0.0]),
            (3, vec![0.95, 0.05, 0.0]),
            (4, vec![0.0, 0.1, 0.9]),
            (5, vec![0.0, 0.9, 0.1]),
            (6, vec![0.05, 0.9, 0.05]),
        ]);
        let g = crate::graph::build_knn_graph(&ds, 2, 0.0).unwrap();
        let r = detect_clusters(&g, 20);
        assert_eq!(r.clusters.len(), 2, "§98 两簇：{:?}", r.clusters);
        // 同簇成员一致（确定性 §100）。
        let c0: &Vec<i64> = &r.clusters[0].track_ids;
        let c1: &Vec<i64> = &r.clusters[1].track_ids;
        let is_group = |c: &Vec<i64>, ids: &[i64]| ids.iter().all(|id| c.contains(id));
        assert!(
            (is_group(c0, &[1, 2, 3]) && is_group(c1, &[4, 5, 6]))
                || (is_group(c0, &[4, 5, 6]) && is_group(c1, &[1, 2, 3]))
        );
        // §101/§102：canonical 重编号——size 相同 → 最小 track_id 小者在前。
        assert_eq!(r.clusters[0].track_ids[0], 1);
        assert_eq!(r.clusters[1].track_ids[0], 4);
    }

    #[test]
    fn deterministic_across_runs() {
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0]),
            (2, vec![0.9, 0.1]),
            (3, vec![0.8, 0.2]),
            (4, vec![0.0, 1.0]),
            (5, vec![0.1, 0.9]),
        ]);
        let g = crate::graph::build_knn_graph(&ds, 2, 0.0).unwrap();
        let a = detect_clusters(&g, 20);
        for _ in 0..9 {
            assert_eq!(detect_clusters(&g, 20), a, "§100");
        }
    }

    #[test]
    fn isolated_nodes_own_clusters() {
        // §60/§102：孤立节点自成单簇（canonical 序按最小 track_id）。
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0]),
            (2, vec![1.0, 0.05]),
            (3, vec![-1.0, 0.0]),
        ]);
        let g = crate::graph::build_knn_graph(&ds, 1, 0.5).unwrap();
        let r = detect_clusters(&g, 10);
        assert!(r.clusters.len() >= 2);
    }
}
