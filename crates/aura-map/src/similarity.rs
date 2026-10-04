//! 相似度内核（§35–§45，D-110）：严格复用 M8 cosine 合同；
//! top-K 稳定 tie-break（similarity DESC，TrackId ASC）；k 有界。
//!
//! 搜索策略（§44/§45，D-111）：exact vector scan（小/中库）。
//! EXACT_SCAN_MAX_NODES 以上的库 → 返回 [`SimilarityError::DatasetTooLarge`]，
//! ANN 索引为 B 级（§46 审计后引入）；不盲目 O(N²) 于超大库。

use crate::dataset::MapDataset;
use aura_analysis::domain::Embedding;

/// §43：k 上界。
pub const MAX_K: usize = 1_000;
/// §44/§45：exact scan 上限（超过则需 ANN；D-111 记录阈值依据）。
pub const EXACT_SCAN_MAX_NODES: usize = 25_000;
/// §57：图构建默认 k。
pub const DEFAULT_K: usize = 12;
/// §56：最低相似度阈值（cosine ≤0 视为不相似，§55 负相似不成边）。
pub const MIN_SIMILARITY: f32 = 0.0;

/// 相似度错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimilarityError {
    /// §33/§38：零向量（cosine 无定义）。
    ZeroVector,
    /// §34/§38：NaN/Inf。
    InvalidVector,
    /// 维数不匹配。
    DimensionMismatch,
    /// §44：超出 exact scan 能力（需 ANN，当前未启用）。
    DatasetTooLarge { nodes: usize },
    /// TrackId 不在数据集中。
    UnknownTrack,
}

/// §40 纯函数相似度：cosine（M8 合同）。不依赖数据库。
///
/// # Errors
/// [`SimilarityError`]：零向量/非法向量/维度不匹配。
pub fn similarity(a: &Embedding, b: &Embedding) -> Result<f32, SimilarityError> {
    if a.dimension != b.dimension || a.vector.len() != b.vector.len() {
        return Err(SimilarityError::DimensionMismatch);
    }
    if a.vector.iter().any(|v| !v.is_finite()) || b.vector.iter().any(|v| !v.is_finite()) {
        return Err(SimilarityError::InvalidVector);
    }
    let dot: f64 = a
        .vector
        .iter()
        .zip(&b.vector)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum();
    let na: f64 = a
        .vector
        .iter()
        .map(|v| f64::from(*v) * f64::from(*v))
        .sum::<f64>()
        .sqrt();
    let nb: f64 = b
        .vector
        .iter()
        .map(|v| f64::from(*v) * f64::from(*v))
        .sum::<f64>()
        .sqrt();
    if na <= f64::EPSILON || nb <= f64::EPSILON {
        return Err(SimilarityError::ZeroVector); // §33：不得悄悄返回 0.0
    }
    let cos = (dot / (na * nb)) as f32;
    debug_assert!((-1.0..=1.0).contains(&cos) || cos.is_nan());
    if cos.is_nan() {
        return Err(SimilarityError::InvalidVector);
    }
    Ok(cos.clamp(-1.0, 1.0))
}

/// §41 top-K 邻居：`{track_id, similarity, rank}`，相似度 DESC、
/// TrackId ASC tie-break（§42）。自身排除。
///
/// # Errors
/// [`SimilarityError`]。
pub fn top_k_neighbors(
    ds: &MapDataset,
    track_id: i64,
    k: usize,
) -> Result<Vec<(i64, f32, usize)>, SimilarityError> {
    let k = k.clamp(1, MAX_K);
    if ds.entries.len() > EXACT_SCAN_MAX_NODES {
        return Err(SimilarityError::DatasetTooLarge {
            nodes: ds.entries.len(),
        });
    }
    let self_entry = ds.entry(track_id).ok_or(SimilarityError::UnknownTrack)?;

    let mut scored: Vec<(i64, f32)> = Vec::with_capacity(ds.entries.len());
    for e in &ds.entries {
        if e.track_id == track_id {
            continue; // 无自环（§54）
        }
        // 合同一致的向量才可比（dataset 构建已保证）。
        scored.push((e.track_id, similarity(&self_entry.embedding, &e.embedding)?));
    }
    // §42：similarity DESC，TrackId ASC。
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    Ok(scored
        .into_iter()
        .take(k)
        .enumerate()
        .map(|(rank, (id, s))| (id, s, rank))
        .collect())
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
    fn cosine_same_orthogonal_opposite() {
        // §36。
        let a = emb(vec![1.0, 0.0]);
        assert!((similarity(&a, &a).unwrap() - 1.0).abs() < 1e-5);
        let b = emb(vec![0.0, 1.0]);
        assert!(similarity(&a, &b).unwrap().abs() < 1e-5);
        let c = emb(vec![-1.0, 0.0]);
        assert!((similarity(&a, &c).unwrap() + 1.0).abs() < 1e-5);
    }

    #[test]
    fn zero_vector_rejected_not_zero() {
        // §33：不得悄悄返回 0.0。
        let z = emb(vec![0.0, 0.0]);
        let a = emb(vec![1.0, 0.0]);
        assert_eq!(similarity(&z, &a), Err(SimilarityError::ZeroVector));
    }

    #[test]
    fn nan_rejected() {
        // §38。
        let mut bad = emb(vec![1.0, 0.0]);
        bad.vector[0] = f32::NAN;
        let a = emb(vec![1.0, 0.0]);
        assert_eq!(similarity(&bad, &a), Err(SimilarityError::InvalidVector));
    }

    #[test]
    fn top_k_order_and_tie_break() {
        // §41/§42：等相似度 → TrackId ASC。
        let ds = ds_with(vec![
            (1, vec![1.0, 0.0]),
            (4, vec![1.0, 0.0]), // 与 1 相似度 1.0（同向）
            (3, vec![1.0, 0.0]), // 同上，id 更小
            (2, vec![0.0, 1.0]), // 正交 → 0
        ]);
        let r = top_k_neighbors(&ds, 1, 3).unwrap();
        let ids: Vec<i64> = r.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(ids, vec![3, 4, 2], "sim DESC + id ASC");
        assert_eq!(r[0].2, 0, "rank 从 0");
        assert!((r[0].1 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn self_excluded_and_k_bounded() {
        let ds = ds_with(vec![(1, vec![1.0, 0.0]), (2, vec![1.0, 0.0])]);
        let r = top_k_neighbors(&ds, 1, MAX_K).unwrap();
        assert_eq!(r.len(), 1, "自身排除（§54）");
        let big = ds_with((0..500).map(|i| (i, vec![1.0, 0.0])).collect());
        let r = top_k_neighbors(&big, 0, 3).unwrap();
        assert_eq!(r.len(), 3, "§43 k 钳制");
    }

    #[test]
    fn unknown_track() {
        let ds = ds_with(vec![(1, vec![1.0, 0.0])]);
        assert_eq!(
            top_k_neighbors(&ds, 99, 5),
            Err(SimilarityError::UnknownTrack)
        );
    }
}
