//! M7 数据合同（§229–§232，D-106）：AnalysisVisualSummary ——
//! Dreamscape 真正需要的最小数据（bpm/energy/brightness/loudness）。
//!
//! - M7 依赖方向（§230）：M7 不被重写；本 DTO 只是注入合同。
//! - Null analysis（§231/§206）：`AnalysisVisualSummary::none()`——
//!   Dreamscape 必须在无分析时完整工作。
//! - 不声称视觉已随 BPM 改变（§232：M7 当前无消费代码，仅 clean
//!   contract）。

use serde::{Deserialize, Serialize};

use crate::domain::TrackAnalysis;

/// M7 视觉影响注入合同（§205 M7 扩展点）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisVisualSummary {
    /// 无分析语义时为 None（M7 用缺省行为）。
    pub bpm: Option<f32>,
    pub energy: Option<f32>,
    pub brightness: Option<f32>,
    pub loudness: Option<f32>,
}

impl AnalysisVisualSummary {
    /// §231：无分析缺省。
    #[must_use]
    pub fn none() -> Self {
        Self {
            bpm: None,
            energy: None,
            brightness: None,
            loudness: None,
        }
    }

    /// 从 TrackAnalysis 提取（§232 真实 summary）。
    #[must_use]
    pub fn from_analysis(a: &TrackAnalysis) -> Self {
        Self {
            bpm: a.tempo.as_ref().map(|t| t.bpm),
            energy: a.spectral.as_ref().map(|s| s.energy),
            brightness: a.spectral.as_ref().map(|s| s.brightness),
            loudness: a.loudness.as_ref().map(|l| l.integrated_lufs),
        }
    }

    /// §206 语义：是否包含任何真实分析数据。
    #[must_use]
    pub fn has_data(&self) -> bool {
        self.bpm.is_some()
            || self.energy.is_some()
            || self.brightness.is_some()
            || self.loudness.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AnalysisStatus, FeatureAvailability, LoudnessAnalysis, SpectralFeatures, TempoAnalysis,
    };

    /// §231：no analysis → Dreamscape 完整工作（合同层：none() 有效且
    /// has_data=false——M7 用缺省视觉参数）。
    #[test]
    fn null_analysis_contract() {
        let none = AnalysisVisualSummary::none();
        assert!(!none.has_data());
        assert!(none.bpm.is_none());
        let json = serde_json::to_string(&none).unwrap();
        let back: AnalysisVisualSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(back, none);
    }

    /// §232：analysis exists → 真实 summary 被接受（数值来自真实分析）。
    #[test]
    fn real_analysis_contract() {
        let mut a = TrackAnalysis::skeleton(1, "m7");
        a.status = AnalysisStatus::Completed;
        a.tempo = Some(TempoAnalysis {
            bpm: 120.0,
            confidence: 0.9,
            first_beat_offset_seconds: 0.0,
            algorithm_version: 1,
        });
        a.tempo_availability = FeatureAvailability::Available;
        a.spectral = Some(SpectralFeatures {
            rms: 0.3,
            peak: 0.9,
            centroid_hz: 2000.0,
            rolloff_hz: 9000.0,
            brightness: 0.5,
            energy: 0.6,
            algorithm_version: 1,
        });
        a.spectral_availability = FeatureAvailability::Available;
        a.loudness = Some(LoudnessAnalysis {
            integrated_lufs: -14.0,
            lra: 4.0,
            true_peak_dbtp: -1.0,
            algorithm_version: 1,
        });
        a.loudness_availability = FeatureAvailability::Available;

        let s = AnalysisVisualSummary::from_analysis(&a);
        assert!(s.has_data());
        assert_eq!(s.bpm, Some(120.0));
        assert_eq!(s.energy, Some(0.6));
        assert_eq!(s.brightness, Some(0.5));
        assert_eq!(s.loudness, Some(-14.0));
    }

    /// §205 特征独立（M7 合同视角）：BPM 缺失不影响 energy/brightness 注入。
    #[test]
    fn partial_analysis_partial_summary() {
        let mut a = TrackAnalysis::skeleton(2, "partial");
        a.status = AnalysisStatus::Completed;
        a.spectral = Some(SpectralFeatures {
            rms: 0.2,
            peak: 0.4,
            centroid_hz: 1000.0,
            rolloff_hz: 4000.0,
            brightness: 0.3,
            energy: 0.4,
            algorithm_version: 1,
        });
        a.spectral_availability = FeatureAvailability::Available;
        // BPM/Loudness 不可用。
        let s = AnalysisVisualSummary::from_analysis(&a);
        assert!(s.bpm.is_none());
        assert_eq!(s.energy, Some(0.4));
        assert_eq!(s.brightness, Some(0.3));
        assert!(s.has_data());
    }
}
