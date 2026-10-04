//! Map 查询 API（§36–§38，M9 上篇后端层）：邻居/簇/视口——均基于
//! 已构建的 dataset+graph+layout 内存结构（IPC 桥接在应用层）。

use crate::clustering::ClusteringResult;
use crate::dataset::MapDataset;
use crate::graph::MapGraph;
use crate::layout::MapPosition;
use std::collections::HashMap;

use crate::similarity::top_k_neighbors;

/// §36：选中节点邻居查询（复用相似度内核的稳定性语义）。
///
/// # Errors
/// 相似度计算失败 / 未知 track。
pub fn neighbors(
    ds: &MapDataset,
    g: &MapGraph,
    track_id: i64,
    k: usize,
) -> Result<Vec<(i64, f32)>, crate::similarity::SimilarityError> {
    let in_graph: std::collections::HashSet<i64> = g
        .neighbors_of(track_id)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    // 图边已含阈值过滤；此处从内核取 rank 完整性。
    let ranked = top_k_neighbors(ds, track_id, k)?;
    let mut out = Vec::new();
    for (id, sim, _rank) in ranked {
        if in_graph.contains(&id) {
            out.push((id, sim));
        }
        if out.len() >= k {
            break;
        }
    }
    Ok(out)
}

/// §38：视口查询——返回 [-1,1]² 矩形内的节点位置（LOD 数据源）。
#[must_use]
pub fn viewport_nodes(
    positions: &HashMap<i64, MapPosition>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> Vec<(i64, MapPosition)> {
    positions
        .iter()
        .filter(|(_, p)| p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y)
        .map(|(id, p)| (*id, *p))
        .collect()
}

/// §98：簇查询——按 cluster_id 返回成员。
#[must_use]
pub fn cluster_members(clustering: &ClusteringResult, cluster_id: usize) -> Vec<i64> {
    clustering
        .clusters
        .get(cluster_id)
        .map(|c| c.track_ids.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::MapEntry;
    use aura_analysis::domain::{Embedding, EMBEDDING_MODEL_ID};

    fn emb(v: Vec<f32>) -> Embedding {
        Embedding::normalized(v, EMBEDDING_MODEL_ID, 1)
    }

    #[test]
    fn neighbors_intersect_graph() {
        let ds = MapDataset::build(
            vec![
                MapEntry {
                    track_id: 1,
                    content_identity: "a".into(),
                    analysis_key: "k1".into(),
                    embedding: emb(vec![1.0, 0.0]),
                },
                MapEntry {
                    track_id: 2,
                    content_identity: "b".into(),
                    analysis_key: "k2".into(),
                    embedding: emb(vec![0.9, 0.1]),
                },
                MapEntry {
                    track_id: 3,
                    content_identity: "c".into(),
                    analysis_key: "k3".into(),
                    embedding: emb(vec![0.0, 1.0]),
                },
            ],
            None,
        )
        .unwrap();
        let g = crate::graph::build_knn_graph(&ds, 1, 0.0).unwrap();
        let r = neighbors(&ds, &g, 1, 2).unwrap();
        assert!(!r.is_empty());
        assert!(r.iter().all(|(id, _)| *id != 1), "无自环");
    }

    #[test]
    fn viewport_filters() {
        let mut positions = HashMap::new();
        positions.insert(1, MapPosition { x: -0.5, y: -0.5 });
        positions.insert(2, MapPosition { x: 0.5, y: 0.5 });
        let r = viewport_nodes(&positions, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, 2);
    }

    #[test]
    fn cluster_members_by_id() {
        let clustering = ClusteringResult {
            assignment: HashMap::new(),
            clusters: vec![
                crate::clustering::MapCluster {
                    cluster_id: 0,
                    track_ids: vec![1, 2],
                    centroid: None,
                    radius: None,
                },
                crate::clustering::MapCluster {
                    cluster_id: 1,
                    track_ids: vec![3],
                    centroid: None,
                    radius: None,
                },
            ],
        };
        assert_eq!(cluster_members(&clustering, 1), vec![3]);
        assert!(cluster_members(&clustering, 9).is_empty());
    }
}
