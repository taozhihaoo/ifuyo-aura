//! TrackAnalysisSummary（§117 输出合同）：M6/M7/M9 消费的稳定 DTO。
//! 单位（§118）：bpm=BPM、lufs=LUFS、lra=LU、true_peak=dBTP、
//! centroid/rolloff=Hz、energy/brightness/confidence=[0,1]。
//! NULL 语义（§119/§120）：未分析 → None（绝不用 0 表示）。

use serde::{Deserialize, Serialize};

use crate::domain::{AnalysisStatus, TrackAnalysis};

/// 稳定输出合同（§117）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackAnalysisSummary {
    pub analysis_version: u16,
    pub content_identity: String,
    pub status: AnalysisStatus,
    pub bpm: Option<f32>,
    pub bpm_confidence: Option<f32>,
    /// 首个检测拍的偏移（秒）；可视化据此对齐节拍相位（None = 未分析）。
    pub first_beat_offset_seconds: Option<f32>,
    pub musical_key: Option<String>,
    pub key_confidence: Option<f32>,
    pub camelot_key: Option<String>,
    pub integrated_lufs: Option<f32>,
    pub loudness_range: Option<f32>,
    pub true_peak: Option<f32>,
    pub energy: Option<f32>,
    pub brightness: Option<f32>,
    pub spectral_centroid: Option<f32>,
    pub spectral_rolloff: Option<f32>,
    /// §83：MFCC 摘要（mean 前 3 项诊断展示；完整 mean/variance 在
    /// TrackAnalysis）。
    pub mfcc_summary: Option<[f32; 3]>,
    /// §117/§49：嵌入元数据（向量本体不进 summary——M9 经仓库取）。
    pub embedding_metadata: Option<EmbeddingMetadata>,
}

/// 嵌入元数据（§28/§48）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingMetadata {
    pub dimension: u32,
    pub dtype: String,
    pub model_id: String,
    pub model_version: u16,
    pub normalization: String,
    pub distance_metric: String,
}

impl From<&TrackAnalysis> for TrackAnalysisSummary {
    fn from(a: &TrackAnalysis) -> Self {
        let key_str = a.key.as_ref().map(|k| format!("{:?} {:?}", k.key, k.mode));
        Self {
            analysis_version: a.analysis_version,
            content_identity: a.content_identity.clone(),
            status: a.status,
            bpm: a.tempo.as_ref().map(|t| t.bpm),
            bpm_confidence: a.tempo.as_ref().map(|t| t.confidence),
            first_beat_offset_seconds: a.tempo.as_ref().map(|t| t.first_beat_offset_seconds),
            musical_key: key_str,
            key_confidence: a.key.as_ref().map(|k| k.confidence),
            camelot_key: a.key.as_ref().and_then(|k| k.camelot.clone()),
            integrated_lufs: a.loudness.as_ref().map(|l| l.integrated_lufs),
            loudness_range: a.loudness.as_ref().map(|l| l.lra),
            true_peak: a.loudness.as_ref().map(|l| l.true_peak_dbtp),
            energy: a.spectral.as_ref().map(|s| s.energy),
            brightness: a.spectral.as_ref().map(|s| s.brightness),
            spectral_centroid: a.spectral.as_ref().map(|s| s.centroid_hz),
            spectral_rolloff: a.spectral.as_ref().map(|s| s.rolloff_hz),
            mfcc_summary: a.mfcc.as_ref().map(|m| {
                [
                    m.mean.first().copied().unwrap_or(0.0),
                    m.mean.get(1).copied().unwrap_or(0.0),
                    m.mean.get(2).copied().unwrap_or(0.0),
                ]
            }),
            embedding_metadata: a.embedding.as_ref().map(|e| EmbeddingMetadata {
                dimension: e.dimension,
                dtype: "f32-le".to_string(),
                model_id: e.model_id.clone(),
                model_version: e.model_version,
                normalization: "l2".to_string(),
                distance_metric: "cosine".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        FeatureAvailability, KeyAnalysis, KeyMode, LoudnessAnalysis, MusicalKey, SpectralFeatures,
        TempoAnalysis,
    };

    /// 可视化对齐节拍需要 first-beat 偏移随 summary 透出。
    #[test]
    fn summary_carries_first_beat_offset() {
        use crate::domain::{AnalysisStatus, FeatureAvailability, TempoAnalysis};
        let mut a = TrackAnalysis::skeleton(7, "c-beat");
        a.status = AnalysisStatus::Completed;
        a.tempo = Some(TempoAnalysis {
            bpm: 128.0,
            confidence: 0.9,
            first_beat_offset_seconds: 0.42,
            algorithm_version: 1,
        });
        a.tempo_availability = FeatureAvailability::Available;
        let s = TrackAnalysisSummary::from(&a);
        assert_eq!(s.bpm, Some(128.0));
        assert_eq!(s.first_beat_offset_seconds, Some(0.42));
        // 未分析曲目不得伪造偏移。
        let empty = TrackAnalysisSummary::from(&TrackAnalysis::skeleton(8, "c-none"));
        assert_eq!(empty.first_beat_offset_seconds, None);
    }

    #[test]
    fn summary_maps_fields_and_units() {
        let mut a = TrackAnalysis::skeleton(1, "c");
        a.status = AnalysisStatus::Completed;
        a.tempo = Some(TempoAnalysis {
            bpm: 128.0,
            confidence: 0.9,
            first_beat_offset_seconds: 0.0,
            algorithm_version: 1,
        });
        a.tempo_availability = FeatureAvailability::Available;
        a.key = Some(KeyAnalysis {
            key: MusicalKey::Fs,
            mode: KeyMode::Minor,
            confidence: 0.7,
            camelot: Some("11A".into()),
            algorithm_version: 1,
        });
        a.key_availability = FeatureAvailability::Available;
        a.loudness = Some(LoudnessAnalysis {
            integrated_lufs: -14.0,
            lra: 5.0,
            true_peak_dbtp: -1.0,
            algorithm_version: 1,
        });
        a.loudness_availability = FeatureAvailability::Available;
        let s = TrackAnalysisSummary::from(&a);
        assert_eq!(s.bpm, Some(128.0), "bpm=BPM");
        assert_eq!(s.integrated_lufs, Some(-14.0), "lufs=LUFS");
        assert_eq!(s.loudness_range, Some(5.0), "lra=LU");
        assert_eq!(s.true_peak, Some(-1.0), "true_peak=dBTP");
        assert_eq!(s.camelot_key.as_deref(), Some("11A"));
        assert_eq!(s.musical_key.as_deref(), Some("Fs Minor"));
        // Embedding 缺失 → 元数据 None（§117 nullable）。
        assert!(s.embedding_metadata.is_none());
    }

    #[test]
    fn summary_serde_camel_case_roundtrip() {
        let mut a = TrackAnalysis::skeleton(2, "ser");
        a.status = AnalysisStatus::Completed;
        a.spectral = Some(SpectralFeatures {
            rms: 0.3,
            peak: 0.9,
            centroid_hz: 2500.0,
            rolloff_hz: 9_000.0,
            brightness: 0.4,
            energy: 0.5,
            algorithm_version: 1,
        });
        a.spectral_availability = FeatureAvailability::Available;
        let s = TrackAnalysisSummary::from(&a);
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"analysisVersion\""));
        assert!(json.contains("\"spectralCentroid\""));
        let back: TrackAnalysisSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}
