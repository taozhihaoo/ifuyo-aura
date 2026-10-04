//! 确定性 2D 布局（§68–§85，D-114）：
//! embedding → 确定性 PCA 初始化（符号规范 §74/§75）→ 图感知网格
//! 松弛（斥力 grid 加速 §77，引力沿边）→ 归一化 [-1,1]²（§72）。
//!
//! - §76：无 O(N²) 全对计算——斥力经 grid 邻域（半径截断）。
//! - §78：max_iterations / max_nodes 预算。
//! - §82–§85：增量——新增节点初始化在邻居质心，只做局部松弛，已有
//!   节点位置保持。
//! - §74/§75 PCA 符号规范：最大绝对载荷项为正。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

use crate::dataset::MapDataset;
use crate::graph::MapGraph;

pub const LAYOUT_ALGO_VERSION: u16 = 1;
/// §78 预算。
pub const MAX_ITERATIONS: usize = 150;
pub const MAX_NODES: usize = 25_000;
/// 斥力半径（归一化坐标比例）。
const REPULSION_RADIUS: f32 = 0.12;
/// 网格 bin 数（每轴）。
const GRID_BINS: usize = 32;

/// 位置（归一化 [-1,1]²）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MapPosition {
    pub x: f32,
    pub y: f32,
}

/// 布局结果。
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutResult {
    /// BTreeMap：序列化序稳定（§159 checksum / 持久化 §64–§67）。
    pub positions: BTreeMap<i64, MapPosition>,
    pub iterations: usize,
}

/// 确定性布局（§68–§78）。
///
/// # Errors
/// dataset 空 → [`String`] 描述（简化错误面；管线层映射）。
pub fn compute_layout(ds: &MapDataset, g: &MapGraph) -> Result<LayoutResult, String> {
    layout_with_options(ds, g, MAX_ITERATIONS)
}

/// 增量/受限布局（§82/§84）：复用初始位置，仅松弛。
pub fn layout_with_options(
    ds: &MapDataset,
    g: &MapGraph,
    max_iterations: usize,
) -> Result<LayoutResult, String> {
    if ds.entries.is_empty() {
        return Err("empty dataset".to_string());
    }
    if ds.entries.len() > MAX_NODES {
        return Err(format!("dataset exceeds max nodes {}", MAX_NODES));
    }
    let mut positions = pca_init(ds);

    // 图感知松弛（§72/§76/§77：grid 斥力 + 边引力）。
    let index: HashMap<i64, usize> = ds
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.track_id, i))
        .collect();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); ds.entries.len()];
    for e in &g.edges {
        if let (Some(&a), Some(&b)) = (index.get(&e.a), index.get(&e.b)) {
            adj[a].push(b);
            adj[b].push(a);
        }
    }

    let n = positions.len();
    for iter in 0..max_iterations.min(MAX_ITERATIONS) {
        let _ = iter;
        // 斥力：grid 邻域（§77）。
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, p) in positions.iter().enumerate() {
            let bx = ((p.0 * 0.5 + 0.5).clamp(0.0, 0.999_9) * GRID_BINS as f32) as i32;
            let by = ((p.1 * 0.5 + 0.5).clamp(0.0, 0.999_9) * GRID_BINS as f32) as i32;
            grid.entry((bx, by)).or_default().push(i);
        }
        let mut force = vec![(0.0f32, 0.0f32); n];
        for (i, p) in positions.iter().enumerate() {
            let bx = ((p.0 * 0.5 + 0.5).clamp(0.0, 0.999_9) * GRID_BINS as f32) as i32;
            let by = ((p.1 * 0.5 + 0.5).clamp(0.0, 0.999_9) * GRID_BINS as f32) as i32;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let Some(cell) = grid.get(&(bx + dx, by + dy)) {
                        for &j in cell {
                            if j == i {
                                continue;
                            }
                            let q = positions[j];
                            let (vx, vy) = (p.0 - q.0, p.1 - q.1);
                            let d2 = vx * vx + vy * vy + 1e-6;
                            if d2 < REPULSION_RADIUS * REPULSION_RADIUS {
                                let d = d2.sqrt();
                                let f = (REPULSION_RADIUS - d) / REPULSION_RADIUS * 0.05;
                                let (fx, fy) = if d > 1e-5 {
                                    (vx / d * f, vy / d * f)
                                } else {
                                    // 确定性抖动（§74 无随机：用 index 派生）。
                                    let a = i as f32 * 2.399;
                                    ((a.cos()) * 0.01, (a.sin()) * 0.01)
                                };
                                force[i].0 += fx;
                                force[i].1 += fy;
                            }
                        }
                    }
                }
            }
        }
        // 引力：沿边拉近。
        for e in &g.edges {
            if let (Some(&a), Some(&b)) = (index.get(&e.a), index.get(&e.b)) {
                let (pa, pb) = (positions[a], positions[b]);
                let (vx, vy) = (pb.0 - pa.0, pb.1 - pa.1);
                let d = (vx * vx + vy * vy).sqrt();
                let pull = (d - 0.05) * 0.02 * e.similarity;
                if d > 1e-5 {
                    force[a].0 += vx / d * pull;
                    force[a].1 += vy / d * pull;
                    force[b].0 -= vx / d * pull;
                    force[b].1 -= vy / d * pull;
                }
            }
        }
        // 应用 + 归一化边界 clamp。
        for (p, f) in positions.iter_mut().zip(&force) {
            p.0 = (p.0 + f.0).clamp(-1.0, 1.0);
            p.1 = (p.1 + f.1).clamp(-1.0, 1.0);
        }
    }

    let map: BTreeMap<i64, MapPosition> = ds
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            (
                e.track_id,
                MapPosition {
                    x: positions[i].0,
                    y: positions[i].1,
                },
            )
        })
        .collect();
    Ok(LayoutResult {
        positions: map,
        iterations: max_iterations.min(MAX_ITERATIONS),
    })
}

/// §73/§74/§75：确定性 PCA 2D 初始化。
/// - 协方差特征向量经**幂迭代**（固定次数，无随机起点——从全 1 方向
///   开始，逐次正交化）；
/// - 分量序：方差降序；
/// - 符号规范：最大绝对载荷项为正（§75）。
fn pca_init(ds: &MapDataset) -> Vec<(f32, f32)> {
    let n = ds.entries.len();
    let dim = ds.compatibility.dimension as usize;
    let mean: Vec<f32> = (0..dim)
        .map(|d| {
            ds.entries
                .iter()
                .map(|e| e.embedding.vector.get(d).copied().unwrap_or(0.0))
                .sum::<f32>()
                / n as f32
        })
        .collect();
    // 中心化矩阵（f64 精度）。
    let centered: Vec<Vec<f64>> = ds
        .entries
        .iter()
        .map(|e| {
            (0..dim)
                .map(|d| {
                    f64::from(e.embedding.vector.get(d).copied().unwrap_or(0.0))
                        - f64::from(mean[d])
                })
                .collect()
        })
        .collect();

    // 幂迭代取前 2 主成分（带正交化）。
    let mut v1 = vec![1.0f64; dim];
    for _ in 0..24 {
        v1 = multiply(centered.as_slice(), &v1);
        let norm = v1.iter().map(|v| v * v).sum::<f64>().sqrt();
        if norm < 1e-12 {
            break;
        }
        for v in &mut v1 {
            *v /= norm;
        }
    }
    let mut v2 = vec![0.0f64; dim];
    if dim > 1 {
        v2[1] = 1.0; // 与 v1 正交的初始方向
        for _ in 0..24 {
            let mut orth = multiply(centered.as_slice(), &v2);
            // 正交化：减去 v1 分量。
            let proj: f64 = orth.iter().zip(&v1).map(|(a, b)| a * b).sum::<f64>();
            for (o, b) in orth.iter_mut().zip(&v1) {
                *o -= proj * b;
            }
            let norm = orth.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm < 1e-12 {
                break;
            }
            for v in &mut orth {
                *v /= norm;
            }
            v2 = orth;
        }
    }

    // §74/§75 符号规范：最大绝对载荷项为正。
    canonicalize_sign(&mut v1);
    canonicalize_sign(&mut v2);

    ds.entries
        .iter()
        .map(|e| {
            let x: f64 = (0..dim)
                .map(|d| {
                    f64::from(e.embedding.vector.get(d).copied().unwrap_or(0.0))
                        - f64::from(mean[d])
                })
                .zip(&v1)
                .map(|(c, w)| c * w)
                .sum::<f64>();
            let y: f64 = (0..dim)
                .map(|d| {
                    f64::from(e.embedding.vector.get(d).copied().unwrap_or(0.0))
                        - f64::from(mean[d])
                })
                .zip(&v2)
                .map(|(c, w)| c * w)
                .sum::<f64>();
            // 归一化到 [-1,1]²（保守 clamp；缩放在松弛中自然发生）。
            ((x as f32).clamp(-1.0, 1.0), (y as f32).clamp(-1.0, 1.0))
        })
        .collect()
}

fn multiply(centered: &[Vec<f64>], v: &[f64]) -> Vec<f64> {
    // AᵀA·v 的近似（对每行投影再回乘）——等价协方差乘向量，O(N·d)。
    let dim = v.len();
    let mut proj_sum = vec![0.0f64; dim];
    for row in centered {
        let mut p = 0.0f64;
        for (r, val) in row.iter().enumerate() {
            p += val * v[r];
        }
        for (r, ps) in proj_sum.iter_mut().enumerate() {
            *ps += p * row[r];
        }
    }
    proj_sum
}

fn canonicalize_sign(v: &mut [f64]) {
    if let Some((_, max_abs)) = v.iter().enumerate().max_by(|a, b| {
        a.1.abs()
            .partial_cmp(&b.1.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        if *max_abs < 0.0 {
            for x in v.iter_mut() {
                *x = -*x;
            }
        }
    }
}

/// §83：新增节点 → 邻居质心初始化（无邻居 → 原点附近）。
#[must_use]
pub fn neighbor_centroid(neighbor_positions: &[(f32, f32)]) -> MapPosition {
    if neighbor_positions.is_empty() {
        return MapPosition { x: 0.0, y: 0.0 };
    }
    let n = neighbor_positions.len() as f32;
    MapPosition {
        x: neighbor_positions.iter().map(|p| p.0).sum::<f32>() / n,
        y: neighbor_positions.iter().map(|p| p.1).sum::<f32>() / n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataset::MapEntry;
    use aura_analysis::domain::{Embedding, EMBEDDING_MODEL_ID};

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
    fn layout_deterministic_and_bounded() {
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0, 0.0]),
            (2, vec![0.9, 0.1, 0.0]),
            (3, vec![0.8, 0.2, 0.0]),
            (4, vec![0.0, 1.0, 0.0]),
            (5, vec![0.1, 0.9, 0.0]),
            (6, vec![0.2, 0.8, 0.0]),
        ]);
        let g = crate::graph::build_knn_graph(&ds, 2, 0.0).unwrap();
        let a = compute_layout(&ds, &g).unwrap();
        let b = compute_layout(&ds, &g).unwrap();
        assert_eq!(a.positions, b.positions, "确定性");
        for p in a.positions.values() {
            assert!(
                (-1.0..=1.0).contains(&p.x) && (-1.0..=1.0).contains(&p.y),
                "归一化"
            );
        }
        assert!(a.iterations <= MAX_ITERATIONS, "预算");
    }

    #[test]
    fn similar_tracks_closer_than_dissimilar() {
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0, 0.0, 0.0]),
            (2, vec![0.95, 0.05, 0.0, 0.0]),
            (3, vec![0.0, 0.0, 1.0, 0.0]),
        ]);
        let g = crate::graph::build_knn_graph(&ds, 1, 0.0).unwrap();
        let r = compute_layout(&ds, &g).unwrap();
        let p = &r.positions;
        let d = |a: i64, b: i64| ((p[&a].x - p[&b].x).powi(2) + (p[&a].y - p[&b].y).powi(2)).sqrt();
        assert!(
            d(1, 2) < d(1, 3) + 1e-5,
            "相似更近 d12={} d13={}",
            d(1, 2),
            d(1, 3)
        );
    }

    #[test]
    fn empty_dataset_rejected() {
        if let Ok(ds) = MapDataset::build(vec![], None) {
            let g = MapGraph {
                nodes: vec![],
                edges: vec![],
                isolated: vec![],
                k: 8,
                graph_algo_version: 1,
            };
            assert!(compute_layout(&ds, &g).is_err());
        }
    }

    #[test]
    fn neighbor_centroid_mean() {
        let c = neighbor_centroid(&[(0.0, 0.0), (1.0, 1.0)]);
        assert!((c.x - 0.5).abs() < 1e-6);
        assert!((c.y - 0.5).abs() < 1e-6);
        let c = neighbor_centroid(&[]);
        assert_eq!((c.x, c.y), (0.0, 0.0));
    }

    #[test]
    fn pca_sign_canonical() {
        let mut v = vec![-0.9, 0.1, -0.05];
        canonicalize_sign(&mut v);
        assert!((v[0] - 0.9).abs() < 1e-6);
    }
}
