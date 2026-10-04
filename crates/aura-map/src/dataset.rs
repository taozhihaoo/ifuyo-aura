//! Map dataset 适配（§15–§35/§25–§27）：M8 embedding → 兼容校验 →
//! 有序数据集 + dataset hash（D-108）。
//!
//! - 兼容性（§28/§29/§30/§31）：同 model_id/model_version/
//!   analysis_version/dimension 的嵌入才可入图；不兼容 → Unavailable
//!   统计，不静默投影/截断。
//! - 向量校验（§32/§33/§34）：dimension/finite/zero-vector 排除。
//! - 顺序（§27）：按 TrackId 升序进入管线（不依赖枚举序）。
//! - dataset hash（§26）：canonical（TrackId 升序）输入的 SipHash。

use serde::{Deserialize, Serialize};
use siphasher::sip128::SipHasher13;

use aura_analysis::domain::Embedding;

/// map 算法版本（§23：与 M8 analysis_version 完全独立）。
pub const MAP_ALGO_VERSION: u16 = 1;
/// dataset 适配版本（§22 记录 analysis_version 之外自己的轴）。
pub const DATASET_ALGO_VERSION: u16 = 1;

/// 数据集错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetError {
    /// 混合不兼容模型/版本（§29/§30：不静默投影）。
    IncompatibleEmbedding { reason: String },
    /// 没有任何可用嵌入（空图）。
    Empty,
}

impl std::fmt::Display for DatasetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompatibleEmbedding { reason } => {
                write!(f, "incompatible embedding: {reason}")
            }
            Self::Empty => write!(f, "no compatible embeddings"),
        }
    }
}

/// 兼容嵌入的期望合同（§28）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingCompatibility {
    pub dimension: u32,
    pub model_id: String,
    pub model_version: u16,
    pub analysis_version: u16,
}

impl EmbeddingCompatibility {
    /// 从第一枚有效嵌入建立期望（§28：首个有效样本定义合同）。
    #[must_use]
    pub fn of(e: &Embedding) -> Self {
        Self {
            dimension: e.dimension,
            model_id: e.model_id.clone(),
            model_version: e.model_version,
            analysis_version: e.analysis_version,
        }
    }

    /// §28/§31：全字段相容才可比较（不投影/不截断）。
    #[must_use]
    pub fn accepts(&self, e: &Embedding) -> bool {
        self.dimension == e.dimension
            && self.model_id == e.model_id
            && self.model_version == e.model_version
            && self.analysis_version == e.analysis_version
    }
}

/// 入图条目（§16：TrackId + content_identity + analysis_key + embedding；
/// 不复制完整 TrackAnalysis）。
#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub track_id: i64,
    pub content_identity: String,
    pub analysis_key: String,
    pub embedding: Embedding,
}

/// 适配统计（§8/§30：真实计数，不伪造）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DatasetStats {
    pub mapped: usize,
    pub excluded_not_analyzed: usize,
    pub excluded_incompatible: usize,
    pub excluded_invalid_vector: usize,
}

/// 就绪数据集（TrackId 升序；§27 确定性顺序）。
#[derive(Debug, Clone, PartialEq)]
pub struct MapDataset {
    pub entries: Vec<MapEntry>,
    pub compatibility: EmbeddingCompatibility,
    pub stats: DatasetStats,
    /// §26 dataset hash（canonical 输入）。
    pub dataset_hash: String,
    pub analysis_version: u16,
    pub map_algo_version: u16,
}

impl MapDataset {
    /// 从候选条目构建（§28 兼容检查 + §32 向量校验 + §27 排序 + §26 hash）。
    /// `expected` 给定时严格按其筛选（增量重建场景）；否则以首个有效嵌入
    /// 定义合同。
    ///
    /// # Errors
    /// 全部不可用 → [`DatasetError::Empty`]。
    pub fn build(
        candidates: Vec<MapEntry>,
        expected: Option<&EmbeddingCompatibility>,
    ) -> Result<Self, DatasetError> {
        let mut stats = DatasetStats::default();
        let mut compatibility: Option<EmbeddingCompatibility> = expected.cloned();

        let mut valid: Vec<MapEntry> = Vec::new();
        for e in candidates {
            // §32：向量校验。
            if e.embedding.vector.is_empty()
                || e.embedding.vector.len() != e.embedding.dimension as usize
                || e.embedding.vector.iter().any(|v| !v.is_finite())
            {
                stats.excluded_invalid_vector += 1;
                continue;
            }
            // §33：零向量 → No similarity 语义，不入图。
            if e.embedding.vector.iter().all(|v| *v == 0.0) {
                stats.excluded_invalid_vector += 1;
                continue;
            }
            // §28–§31：兼容性。
            match &compatibility {
                None => {
                    compatibility = Some(EmbeddingCompatibility::of(&e.embedding));
                }
                Some(c) if !c.accepts(&e.embedding) => {
                    stats.excluded_incompatible += 1;
                    continue;
                }
                _ => {}
            }
            valid.push(e);
        }
        stats.mapped = valid.len();
        let Some(compat) = compatibility else {
            return Err(DatasetError::Empty);
        };
        if valid.is_empty() {
            return Err(DatasetError::Empty);
        }

        // §27：TrackId 升序稳定排序。
        valid.sort_by_key(|e| e.track_id);

        let analysis_version = compat.analysis_version;
        let hash = dataset_hash(&valid, &compat);
        Ok(Self {
            stats,
            compatibility: compat,
            dataset_hash: hash,
            entries: valid,
            analysis_version,
            map_algo_version: MAP_ALGO_VERSION,
        })
    }

    /// 按 TrackId 查找条目（§17 node identity）。
    #[must_use]
    pub fn entry(&self, track_id: i64) -> Option<&MapEntry> {
        // entries 按 track_id 升序 → 二分。
        self.entries
            .binary_search_by_key(&track_id, |e| e.track_id)
            .ok()
            .map(|i| &self.entries[i])
    }
}

/// §26 dataset hash：TrackId 升序 canonical 输入的 128-bit SipHash hex。
#[must_use]
pub fn dataset_hash(entries: &[MapEntry], compat: &EmbeddingCompatibility) -> String {
    let mut buf = Vec::with_capacity(entries.len() * 16 + 64);
    buf.extend_from_slice(b"map-dataset/v1");
    buf.push(0xff);
    buf.extend_from_slice(compat.model_id.as_bytes());
    buf.push(0xff);
    buf.extend_from_slice(&compat.dimension.to_le_bytes());
    buf.extend_from_slice(&compat.model_version.to_le_bytes());
    buf.extend_from_slice(&compat.analysis_version.to_le_bytes());
    for e in entries {
        // canonical：TrackId 升序（调用方保证）。
        buf.extend_from_slice(&e.track_id.to_le_bytes());
        buf.extend_from_slice(e.analysis_key.as_bytes());
    }
    let out = SipHasher13::new_with_keys(0x6d39_6461_7461_7365, 0x0074_315f_7369_7068).hash(&buf);
    let v = out.h1 ^ out.h2;
    format!("{v:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emb(v: Vec<f32>) -> Embedding {
        Embedding::normalized(v, "aura-spectral-stats", 1)
    }

    fn entry(id: i64, v: Vec<f32>) -> MapEntry {
        MapEntry {
            track_id: id,
            content_identity: format!("c{id}"),
            analysis_key: format!("k{id}"),
            embedding: emb(v),
        }
    }

    #[test]
    fn build_sorts_and_counts() {
        let ds = MapDataset::build(
            vec![
                entry(3, vec![1.0, 0.0]),
                entry(1, vec![0.0, 1.0]),
                entry(2, vec![1.0, 1.0]),
            ],
            None,
        )
        .unwrap();
        // §27：TrackId 升序。
        assert_eq!(
            ds.entries.iter().map(|e| e.track_id).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(ds.stats.mapped, 3);
        assert_eq!(ds.compatibility.dimension, 2);
    }

    #[test]
    fn incompatible_model_excluded_not_projected() {
        // §29/§31：不同维度 → 排除（不截断/不随机投影）。
        let mut bad = entry(2, vec![1.0]);
        bad.embedding.dimension = 1;
        let ds = MapDataset::build(vec![entry(1, vec![1.0, 0.0]), bad], None).unwrap();
        assert_eq!(ds.stats.mapped, 1);
        assert_eq!(ds.stats.excluded_incompatible, 1);
    }

    #[test]
    fn zero_and_nan_excluded() {
        // §33/§34。
        let mut nan = entry(2, vec![f32::NAN, 0.0]);
        nan.embedding.dimension = 2;
        let ds = MapDataset::build(
            vec![entry(1, vec![1.0, 0.0]), nan, entry(3, vec![0.0, 0.0])],
            None,
        )
        .unwrap();
        assert_eq!(ds.stats.mapped, 1);
        assert_eq!(ds.stats.excluded_invalid_vector, 2);
    }

    #[test]
    fn expected_contract_strict() {
        let want = EmbeddingCompatibility::of(&emb(vec![1.0, 0.0]));
        // 同合同 → 收。
        let ds = MapDataset::build(vec![entry(1, vec![0.0, 1.0])], Some(&want)).unwrap();
        assert_eq!(ds.stats.mapped, 1);
        // 不同 model → 排除。
        let mut other = entry(2, vec![0.5, 0.5]);
        other.embedding.model_id = "other".into();
        // 全部不兼容 → Empty（§30 计数语义在 stats 中体现）。
        assert_eq!(
            MapDataset::build(vec![other], Some(&want)).unwrap_err(),
            DatasetError::Empty
        );
    }

    #[test]
    fn dataset_hash_stable_and_content_sensitive() {
        let e2 = entry(2, vec![1.0, 1.0]);
        let ds1 = MapDataset::build(vec![entry(1, vec![1.0, 0.0]), e2.clone()], None).unwrap();
        let ds2 = MapDataset::build(vec![entry(1, vec![1.0, 0.0]), e2.clone()], None).unwrap();
        assert_eq!(ds1.dataset_hash, ds2.dataset_hash, "§26 同输入同 hash");
        // 成员/键变化 → hash 变化。
        let mut e2_changed = e2.clone();
        e2_changed.analysis_key = "k2-changed".into();
        let ds3 = MapDataset::build(vec![entry(1, vec![1.0, 0.0]), e2_changed], None).unwrap();
        assert_ne!(ds1.dataset_hash, ds3.dataset_hash);
        // 顺序无关（canonical by TrackId）。
        let ds4 = MapDataset::build(vec![e2, entry(1, vec![1.0, 0.0])], None).unwrap();
        assert_eq!(ds1.dataset_hash, ds4.dataset_hash, "顺序无关 §26");
    }

    #[test]
    fn empty_dataset_rejected() {
        assert_eq!(
            MapDataset::build(vec![], None).unwrap_err(),
            DatasetError::Empty
        );
    }

    #[test]
    fn entry_lookup_binary() {
        let ds = MapDataset::build(
            vec![entry(1, vec![1.0, 0.0]), entry(2, vec![0.0, 1.0])],
            None,
        )
        .unwrap();
        assert!(ds.entry(2).is_some());
        assert!(ds.entry(9).is_none());
    }
}
