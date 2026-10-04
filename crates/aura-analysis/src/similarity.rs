//! 相似度原语（§125–§129）：纯数学 cosine——不做 Music Map（§125）。

use crate::domain::Embedding;

/// 相似度错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimilarityError {
    /// 维数不等（§128：不静默 truncate）。
    DimensionMismatch { a: u32, b: u32 },
    /// NaN/Inf 向量（§126：返回 Invalid 而非 NaN）。
    InvalidVector,
    /// 零向量（无方向）。
    ZeroVector,
    /// 模型/版本不兼容（§129：不混合计算）。
    IncompatibleVersion,
}

/// §129：两个向量是否可比较。
#[must_use]
pub fn comparable(a: &Embedding, b: &Embedding) -> bool {
    a.dimension == b.dimension
        && a.model_id == b.model_id
        && a.model_version == b.model_version
        && a.analysis_version == b.analysis_version
}

/// cosine 相似度（L2 归一化下 cosine=dot）。
///
/// # Errors
/// [`SimilarityError`]。
pub fn cosine_similarity(a: &Embedding, b: &Embedding) -> Result<f32, SimilarityError> {
    if a.dimension != b.dimension || a.vector.len() != b.vector.len() {
        return Err(SimilarityError::DimensionMismatch {
            a: a.dimension,
            b: b.dimension,
        });
    }
    if !comparable(a, b) {
        return Err(SimilarityError::IncompatibleVersion);
    }
    if a.vector.iter().any(|v| !v.is_finite()) || b.vector.iter().any(|v| !v.is_finite()) {
        return Err(SimilarityError::InvalidVector);
    }
    let norm_a = a.vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    let norm_b = b.vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm_a <= f32::EPSILON || norm_b <= f32::EPSILON {
        return Err(SimilarityError::ZeroVector);
    }
    let dot: f32 = a.vector.iter().zip(&b.vector).map(|(x, y)| x * y).sum();
    Ok(dot / (norm_a * norm_b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::EMBEDDING_MODEL_ID;

    fn emb(v: Vec<f32>) -> Embedding {
        Embedding::normalized(v, EMBEDDING_MODEL_ID, 1)
    }

    #[test]
    fn same_orthogonal_opposite() {
        // §127。
        let a = emb(vec![1.0, 0.0]);
        assert!(
            (cosine_similarity(&a, &a).unwrap() - 1.0).abs() < 1e-5,
            "same → 1"
        );
        let b = emb(vec![0.0, 1.0]);
        assert!(
            cosine_similarity(&a, &b).unwrap().abs() < 1e-5,
            "orthogonal → ~0"
        );
        let c = emb(vec![-1.0, 0.0]);
        assert!(
            (cosine_similarity(&a, &c).unwrap() + 1.0).abs() < 1e-5,
            "opposite → -1"
        );
    }

    #[test]
    fn zero_vector_invalid() {
        let z = emb(vec![0.0, 0.0]);
        let a = emb(vec![1.0, 0.0]);
        assert_eq!(cosine_similarity(&z, &a), Err(SimilarityError::ZeroVector));
    }

    #[test]
    fn nan_inf_invalid() {
        // §126：NaN 向量 → Invalid 而非 NaN 结果。
        let mut bad = emb(vec![1.0, 0.0]);
        bad.vector[0] = f32::NAN;
        let a = emb(vec![1.0, 0.0]);
        assert_eq!(
            cosine_similarity(&bad, &a),
            Err(SimilarityError::InvalidVector)
        );
        let mut inf = emb(vec![1.0, 0.0]);
        inf.vector[1] = f32::INFINITY;
        assert_eq!(
            cosine_similarity(&inf, &a),
            Err(SimilarityError::InvalidVector)
        );
    }

    #[test]
    fn dimension_mismatch() {
        // §128：不静默 truncate。
        let a = emb(vec![1.0, 0.0]);
        let b = emb(vec![1.0, 0.0, 0.0]);
        assert_eq!(
            cosine_similarity(&a, &b),
            Err(SimilarityError::DimensionMismatch { a: 2, b: 3 })
        );
    }

    #[test]
    fn version_incompatibility() {
        // §129：不同 model/version 不可比较。
        let a = Embedding::normalized(vec![1.0, 0.0], "m1", 1);
        let b = Embedding::normalized(vec![1.0, 0.0], "m2", 1);
        let c = Embedding::normalized(vec![1.0, 0.0], "m1", 2);
        let d = Embedding::normalized(vec![1.0, 0.0], "m1", 1);
        // analysis_version 差异（手改模拟旧结果）。
        let mut e = d.clone();
        e.analysis_version = 2;
        assert_eq!(
            cosine_similarity(&a, &b),
            Err(SimilarityError::IncompatibleVersion)
        );
        assert_eq!(
            cosine_similarity(&a, &c),
            Err(SimilarityError::IncompatibleVersion)
        );
        assert_eq!(
            cosine_similarity(&a, &e),
            Err(SimilarityError::IncompatibleVersion)
        );
        assert!(comparable(&a, &d));
    }
}
