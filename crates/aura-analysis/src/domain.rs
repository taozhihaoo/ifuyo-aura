//! Analysis domain model（§13–§18/§31–§33）。
//!
//! 身份（§15）：结果绑定 `TrackId + content_identity + analysis_version`
//! （embedding 另含 model_id/model_version）——不依赖裸 TrackId。
//! 版本（§16/§17）：analysis_version 为整体合同版本；各特征另有算法版本。
//! 完整性（§31/§32）：结果 all-or-nothing（单事务写入），但允许
//! feature-level availability 明确表达 partial 状态（§32）。

use serde::{Deserialize, Serialize};

use crate::key::analysis_key;

/// 整体分析 contract 版本（§16）。
pub const ANALYSIS_VERSION: u16 = 1;

/// 各特征算法/模型版本（§17：可定位结果由什么算法产生）。
pub const TEMPO_ALGORITHM_VERSION: u16 = 1;
pub const KEY_ALGORITHM_VERSION: u16 = 1;
pub const LOUDNESS_ALGORITHM_VERSION: u16 = 1;
pub const SPECTRAL_ALGORITHM_VERSION: u16 = 1;
pub const MFCC_ALGORITHM_VERSION: u16 = 1;
/// 嵌入模型（自有 DSP 统计嵌入，D-095：无第三方模型/运行时）。
pub const EMBEDDING_MODEL_ID: &str = "aura-spectral-stats";
pub const EMBEDDING_MODEL_VERSION: u16 = 1;

/// 分析状态机（§14：单一枚举，禁 bool 堆）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    /// 格式/来源明确不支持（如 internet radio §36）。
    Unsupported,
    /// content identity 或版本已过期（§18/§20）。
    Stale,
}

/// 单特征可用性（§32/§33）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureAvailability {
    /// 尚未分析（骨架初始态）。
    Pending,
    Available,
    /// 输入不足以计算（如过短音频）。
    Unavailable,
    /// 来源/格式明确不支持（radio/live）。
    Unsupported,
    /// 计算失败（保留错误信息于 failure 域）。
    Failed,
    /// 结果存在但由旧算法版本产生（§18）。
    Stale,
}

/// 调性（camelot 兼容表示在 [`KeyAnalysis`] 派生，§A-16）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MusicalKey {
    C,
    Cs,
    D,
    Ds,
    E,
    F,
    Fs,
    G,
    Gs,
    A,
    As,
    B,
}

impl MusicalKey {
    /// 全部 12 个（调性检测输出域）。
    pub const ALL: [MusicalKey; 12] = [
        Self::C,
        Self::Cs,
        Self::D,
        Self::Ds,
        Self::E,
        Self::F,
        Self::Fs,
        Self::G,
        Self::Gs,
        Self::A,
        Self::As,
        Self::B,
    ];

    /// 音级（C=0）。
    #[must_use]
    pub const fn pitch_class(self) -> u8 {
        match self {
            Self::C => 0,
            Self::Cs => 1,
            Self::D => 2,
            Self::Ds => 3,
            Self::E => 4,
            Self::F => 5,
            Self::Fs => 6,
            Self::G => 7,
            Self::Gs => 8,
            Self::A => 9,
            Self::As => 10,
            Self::B => 11,
        }
    }
}

/// 调性模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyMode {
    Major,
    Minor,
}

/// BPM/节奏分析结果（§A-10–12）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TempoAnalysis {
    /// BPM（合成/点击轨为精确值；真实音乐为估计值）。
    pub bpm: f32,
    /// 置信度 [0,1]。
    pub confidence: f32,
    /// 首拍偏移（秒；beat-grid 基础，§A-11）。
    pub first_beat_offset_seconds: f32,
    pub algorithm_version: u16,
}

/// 调性分析结果（§A-13–16）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyAnalysis {
    pub key: MusicalKey,
    pub mode: KeyMode,
    /// 置信度 [0,1]。
    pub confidence: f32,
    /// Camelot 兼容码（如 "8A"/"8B"；算法可靠时填充）。
    pub camelot: Option<String>,
    pub algorithm_version: u16,
}

/// 响度分析结果（§A-17–20；M2 `TrackLoudness` 的数据来源，§5）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoudnessAnalysis {
    /// 综合 LUFS（ITU-R BS.1770 链路，中篇实现）。
    pub integrated_lufs: f32,
    /// 响度范围 LU。
    pub lra: f32,
    /// 真峰值 dBTP。
    pub true_peak_dbtp: f32,
    pub algorithm_version: u16,
}

/// 频谱派生特征（§A-21–24；标量族）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpectralFeatures {
    /// RMS 能量 [0,1]。
    pub rms: f32,
    /// 峰值 [0,1]。
    pub peak: f32,
    /// 频谱质心（Hz）。
    pub centroid_hz: f32,
    /// 频谱滚降（Hz，85% 能量点）。
    pub rolloff_hz: f32,
    /// 亮度 = 质心 / (采样率/2)，[0,1]。
    pub brightness: f32,
    /// §76/§77（D-102）：RMS dBFS → [0,1]（[-60,0] 线性映射）。
    pub energy: f32,
    pub algorithm_version: u16,
}

/// MFCC 结果（§A-25–26）：系数向量 + 摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MfccFeatures {
    /// 均值系数（13 维标准；dtype f32 LE）。
    pub mean: Vec<f32>,
    /// 方差系数（同维）。
    pub variance: Vec<f32>,
    pub algorithm_version: u16,
}

impl MfccFeatures {
    /// 标准维数合同。
    pub const DIMENSION: usize = 13;
}

/// 嵌入合同（§47/§48）：M9 只消费此结构及其相似度语义（§49）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Embedding {
    /// L2 归一化 f32 LE 向量（§47：M9 用 cosine/dot 等价）。
    pub vector: Vec<f32>,
    pub dimension: u32,
    pub model_id: String,
    pub model_version: u16,
    pub analysis_version: u16,
}

impl Embedding {
    /// L2 归一化（构造入口统一执行；零向量保持零）。
    #[must_use]
    pub fn normalized(mut vector: Vec<f32>, model_id: &str, model_version: u16) -> Self {
        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > f32::EPSILON {
            for v in &mut vector {
                *v /= norm;
            }
        }
        Self {
            dimension: vector.len() as u32,
            vector,
            model_id: model_id.to_string(),
            model_version,
            analysis_version: ANALYSIS_VERSION,
        }
    }

    /// 二进制序列化（§28：f32 LE 紧凑格式，非 JSON 字符串数组）。
    #[must_use]
    pub fn to_blob(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.vector.len() * 4);
        for v in &self.vector {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// 从 BLOB 恢复（§29：expected_bytes == dimension × 4，否则 Corrupt）。
    ///
    /// # Errors
    /// 长度不符 → [`AnalysisError::CorruptEmbedding`]。
    pub fn from_blob(
        blob: &[u8],
        dimension: u32,
        model_id: &str,
        model_version: u16,
    ) -> Result<Self, AnalysisError> {
        let expected = dimension as usize * 4;
        if blob.len() != expected {
            return Err(AnalysisError::CorruptEmbedding {
                expected_bytes: expected,
                actual_bytes: blob.len(),
            });
        }
        let vector = blob
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        Ok(Self {
            dimension,
            vector,
            model_id: model_id.to_string(),
            model_version,
            analysis_version: ANALYSIS_VERSION,
        })
    }
}

/// 完整分析结果（§13 TrackAnalysis 聚合；§31 all-or-nothing 单事务写入）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackAnalysis {
    /// M3 TrackId（持久化 join 用；身份判定另依赖 content_identity，§15）。
    pub track_id: i64,
    /// M3 content identity（identity_key；§19 改名/移动不变）。
    pub content_identity: String,
    /// §23：hash(content_identity + versions)，稳定派生键。
    pub analysis_key: String,
    pub analysis_version: u16,
    pub status: AnalysisStatus,
    pub tempo: Option<TempoAnalysis>,
    pub tempo_availability: FeatureAvailability,
    pub key: Option<KeyAnalysis>,
    pub key_availability: FeatureAvailability,
    pub loudness: Option<LoudnessAnalysis>,
    pub loudness_availability: FeatureAvailability,
    pub spectral: Option<SpectralFeatures>,
    pub spectral_availability: FeatureAvailability,
    pub mfcc: Option<MfccFeatures>,
    pub mfcc_availability: FeatureAvailability,
    pub embedding: Option<Embedding>,
    pub embedding_availability: FeatureAvailability,
    /// 逻辑时间戳（分析完成时刻；来源时钟语义由应用层决定）。
    pub analyzed_at_ms: i64,
}

impl TrackAnalysis {
    /// 新建骨架（全 Pending；status/时间由调用方定）。
    #[must_use]
    pub fn skeleton(track_id: i64, content_identity: &str) -> Self {
        let analysis_key = analysis_key(
            content_identity,
            ANALYSIS_VERSION,
            EMBEDDING_MODEL_ID,
            EMBEDDING_MODEL_VERSION,
        );
        Self {
            track_id,
            content_identity: content_identity.to_string(),
            analysis_key,
            analysis_version: ANALYSIS_VERSION,
            status: AnalysisStatus::Pending,
            tempo: None,
            tempo_availability: FeatureAvailability::Pending,
            key: None,
            key_availability: FeatureAvailability::Pending,
            loudness: None,
            loudness_availability: FeatureAvailability::Pending,
            spectral: None,
            spectral_availability: FeatureAvailability::Pending,
            mfcc: None,
            mfcc_availability: FeatureAvailability::Pending,
            embedding: None,
            embedding_availability: FeatureAvailability::Pending,
            analyzed_at_ms: 0,
        }
    }

    /// §31：Completed 语义校验——至少一个特征 Available，且每个非 Pending
    /// 特征要么有数据要么明确 Unavailable/Unsupported/Failed/Stale。
    #[must_use]
    pub fn is_valid_completed(&self) -> bool {
        if self.status != AnalysisStatus::Completed {
            return false;
        }
        let any_available = [
            self.tempo_availability,
            self.key_availability,
            self.loudness_availability,
            self.spectral_availability,
            self.mfcc_availability,
            self.embedding_availability,
        ]
        .contains(&FeatureAvailability::Available);
        let pairs = [
            (self.tempo_availability, self.tempo.is_some()),
            (self.key_availability, self.key.is_some()),
            (self.loudness_availability, self.loudness.is_some()),
            (self.spectral_availability, self.spectral.is_some()),
            (self.mfcc_availability, self.mfcc.is_some()),
            (self.embedding_availability, self.embedding.is_some()),
        ];
        any_available
            && pairs.iter().all(|(avail, present)| match avail {
                FeatureAvailability::Available => *present,
                FeatureAvailability::Pending => false,
                _ => !*present || true,
            })
    }
}

/// 分析错误（§13 AnalysisError）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisError {
    /// 解码失败/格式不支持。
    DecodeFailed { reason: String },
    /// 音频过短无法分析。
    TooShort { samples: usize },
    /// 来源类型不支持（radio/live，§36）。
    UnsupportedSource { reason: String },
    /// 嵌入 BLOB 完整性失败（§29）。
    CorruptEmbedding {
        expected_bytes: usize,
        actual_bytes: usize,
    },
    /// 已取消（§A-39）。
    Cancelled,
    /// 数值安全失败（NaN/Inf 进入结果，§168 精神）。
    NumericalSafety { feature: String },
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecodeFailed { reason } => write!(f, "decode failed: {reason}"),
            Self::TooShort { samples } => write!(f, "audio too short ({samples} samples)"),
            Self::UnsupportedSource { reason } => write!(f, "unsupported source: {reason}"),
            Self::CorruptEmbedding {
                expected_bytes,
                actual_bytes,
            } => {
                write!(
                    f,
                    "embedding corrupt: expected {expected_bytes} bytes, got {actual_bytes}"
                )
            }
            Self::Cancelled => write!(f, "analysis cancelled"),
            Self::NumericalSafety { feature } => write!(f, "numerical safety: {feature}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempo() -> TempoAnalysis {
        TempoAnalysis {
            bpm: 120.0,
            confidence: 0.9,
            first_beat_offset_seconds: 0.0,
            algorithm_version: TEMPO_ALGORITHM_VERSION,
        }
    }

    #[test]
    fn analysis_key_deterministic_and_content_bound() {
        let a = analysis_key(
            "id-1",
            ANALYSIS_VERSION,
            EMBEDDING_MODEL_ID,
            EMBEDDING_MODEL_VERSION,
        );
        let b = analysis_key(
            "id-1",
            ANALYSIS_VERSION,
            EMBEDDING_MODEL_ID,
            EMBEDDING_MODEL_VERSION,
        );
        assert_eq!(a, b, "§23 确定性");
        assert_ne!(
            a,
            analysis_key(
                "id-2",
                ANALYSIS_VERSION,
                EMBEDDING_MODEL_ID,
                EMBEDDING_MODEL_VERSION
            )
        );
        assert_ne!(
            a,
            analysis_key("id-1", 2, EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION),
            "版本变化 → 键变化（§18）"
        );
        assert_ne!(
            a,
            analysis_key(
                "id-1",
                ANALYSIS_VERSION,
                "other-model",
                EMBEDDING_MODEL_VERSION
            ),
            "模型变化 → 键变化（§15）"
        );
    }

    #[test]
    fn skeleton_binds_identity_and_starts_pending() {
        let s = TrackAnalysis::skeleton(7, "id-1");
        assert_eq!(s.track_id, 7);
        assert_eq!(s.content_identity, "id-1");
        assert_eq!(s.status, AnalysisStatus::Pending);
        assert_eq!(s.tempo_availability, FeatureAvailability::Pending);
        assert!(!s.is_valid_completed());
    }

    #[test]
    fn completed_requires_all_decided_and_one_available() {
        let mut s = TrackAnalysis::skeleton(1, "id");
        s.status = AnalysisStatus::Completed;
        // 全 Pending（未决定）→ 不算完成。
        assert!(!s.is_valid_completed());
        // 决定其余特征为 Unavailable（输入不足），tempo Available。
        for set in [
            &mut s.key_availability,
            &mut s.loudness_availability,
            &mut s.spectral_availability,
            &mut s.mfcc_availability,
            &mut s.embedding_availability,
        ] {
            *set = FeatureAvailability::Unavailable;
        }
        s.tempo = Some(tempo());
        s.tempo_availability = FeatureAvailability::Available;
        assert!(s.is_valid_completed(), "§31：全部已决定 + ≥1 Available");
        // Available 但无数据 → 无效。
        s.key_availability = FeatureAvailability::Available;
        s.key = None;
        assert!(!s.is_valid_completed());
        // 明确 Unsupported 无数据 → 合法（§32 partial）。
        s.key_availability = FeatureAvailability::Unsupported;
        assert!(s.is_valid_completed());
        // 任一特征回到 Pending（未决定）→ 无效。
        s.key_availability = FeatureAvailability::Pending;
        assert!(!s.is_valid_completed());
    }

    #[test]
    fn embedding_l2_normalized() {
        let e = Embedding::normalized(vec![3.0, 4.0], EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION);
        let norm: f32 = e.vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "§47 L2 归一化");
        assert_eq!(e.dimension, 2);
        assert_eq!(e.model_id, EMBEDDING_MODEL_ID);
        // 零向量保持零（不产生 NaN）。
        let z = Embedding::normalized(vec![0.0; 4], "m", 1);
        assert!(z.vector.iter().all(|v| *v == 0.0));
    }

    #[test]
    fn embedding_blob_roundtrip_and_integrity() {
        let e = Embedding::normalized(
            vec![0.1, -0.2, 0.3, 0.4],
            EMBEDDING_MODEL_ID,
            EMBEDDING_MODEL_VERSION,
        );
        let blob = e.to_blob();
        assert_eq!(blob.len(), 16);
        let back =
            Embedding::from_blob(&blob, 4, EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION).unwrap();
        assert_eq!(back.vector, e.vector, "f32 LE 往返（§28）");
        // §29：字节数不符 → Corrupt，不越界读。
        assert_eq!(
            Embedding::from_blob(&blob[..12], 4, "m", 1).unwrap_err(),
            AnalysisError::CorruptEmbedding {
                expected_bytes: 16,
                actual_bytes: 12
            }
        );
        assert_eq!(
            Embedding::from_blob(&blob, 5, "m", 1).unwrap_err(),
            AnalysisError::CorruptEmbedding {
                expected_bytes: 20,
                actual_bytes: 16
            }
        );
    }

    #[test]
    fn mfcc_dimension_contract() {
        let m = MfccFeatures {
            mean: vec![0.0; MfccFeatures::DIMENSION],
            variance: vec![1.0; MfccFeatures::DIMENSION],
            algorithm_version: MFCC_ALGORITHM_VERSION,
        };
        assert_eq!(m.mean.len(), 13);
        let json = serde_json::to_string(&m).unwrap();
        let back: MfccFeatures = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn key_pitch_classes_complete() {
        // 12 音级互异且覆盖 0..12。
        let mut seen = std::collections::HashSet::new();
        for k in MusicalKey::ALL {
            seen.insert(k.pitch_class());
        }
        assert_eq!(seen.len(), 12);
    }

    #[test]
    fn error_display_non_empty() {
        let errs = [
            AnalysisError::DecodeFailed { reason: "x".into() },
            AnalysisError::TooShort { samples: 5 },
            AnalysisError::UnsupportedSource {
                reason: "radio".into(),
            },
            AnalysisError::CorruptEmbedding {
                expected_bytes: 4,
                actual_bytes: 0,
            },
            AnalysisError::Cancelled,
            AnalysisError::NumericalSafety {
                feature: "bpm".into(),
            },
        ];
        for e in errs {
            assert!(!e.to_string().is_empty());
        }
    }
}
