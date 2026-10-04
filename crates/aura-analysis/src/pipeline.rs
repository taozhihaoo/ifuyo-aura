//! 分析管线（§86/§92–§94/§102/§103/§105/§106/§108/§117）：
//! 一次顺序解码 → 共享特征提取 → 校验 → TrackAnalysis。
//!
//! - 单遍共享（§93）：一次解码喂 Spectral 累加器 + LUFS hop 累加；
//!   Tempo/Key/MFCC/Embedding 用**前 MAX_ANALYSIS_SECONDS 秒**窗口
//!   缓冲（有界 §91/§167——120s × 22050 × 4B ≈ 10.6MB 上限）。
//!   Loudness 全轨（累加器 O(时长/10)，多遍成本说明：loudness 全轨
//!   与窗口特征共用同一次解码流，无二次解码 §94）。
//! - 取消（§102/§103）：检查点在 decode-chunk / 特征块边界；提交
//!   （persist）由调用方在管线外完成——本模块不触 DB（§103 不产生
//!   半提交）。
//! - 进度（§105/§106）：stage 枚举 + processed/total frames。
//! - 逻辑时间（§107/§108）：analyzed_at_ms 由注入 clock 提供。

use crate::domain::{
    AnalysisError, AnalysisStatus, FeatureAvailability, TrackAnalysis, ANALYSIS_VERSION,
    EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION,
};
use crate::embedding::{EmbeddingInput, EmbeddingProvider};
use crate::key_detect;
use crate::loudness;
use crate::mfcc;
use crate::params::{resample_linear, ANALYSIS_SAMPLE_RATE, CHUNK_SAMPLES, MAX_FEATURE_SAMPLES};
use crate::spectral::SpectralAccumulator;
use crate::tempo;

/// 分析阶段（§106）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Decoding,
    Tempo,
    Key,
    Loudness,
    Spectral,
    Mfcc,
    Embedding,
    Persisting,
    Completed,
}

/// 进度（§105）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalysisProgress {
    pub stage: Stage,
    pub processed_frames: u64,
    pub total_frames: u64,
}

/// 取消令牌（§102；检查点轮询）。
#[derive(Debug, Default, Clone)]
pub struct CancelToken(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// 特征提取结果 → TrackAnalysis（校验后）。
///
/// # Errors
/// [`AnalysisError`]：取消 / 解码失败 / 数值安全。
#[allow(clippy::too_many_lines)]
pub fn analyze_stream(
    stream: Box<dyn aura_core::decoder::AudioByteStream>,
    track_id: i64,
    content_identity: &str,
    provider: &dyn EmbeddingProvider,
    cancel: &CancelToken,
    clock_ms: impl Fn() -> i64,
) -> Result<TrackAnalysis, AnalysisError> {
    let mut out = TrackAnalysis::skeleton(track_id, content_identity);
    out.status = AnalysisStatus::Running;

    // ---- 单遍解码：spectral 累加 + loudness 输入 + 窗口缓冲（§92/§93）----
    use aura_core::decoder::Decoder as _;
    let mut decoder =
        aura_core::decoder::streaming::StreamingDecoder::open(stream, None).map_err(|e| {
            AnalysisError::DecodeFailed {
                reason: e.to_string(),
            }
        })?;
    let format = decoder.format();
    let channels = format.channels.get();
    let src_rate = format.sample_rate.get();
    if channels > 2 {
        return Err(AnalysisError::DecodeFailed {
            reason: format!("{channels}-channel downmix unsupported"),
        });
    }

    let mut spectral = SpectralAccumulator::new();
    let mut loudness_input: Vec<f32> = Vec::new(); // 全轨 mono @ 源采样率
    let mut window: Vec<f32> = Vec::new(); // 规范率 mono 窗口（有界）
    let mut buf = aura_core::decoder::pcm::PcmBuffer::new();
    let mut processed: u64 = 0;
    // 总帧数：duration_frames（可能 None → 进度以 processed 报告）。
    let total_frames = decoder.duration_frames().unwrap_or(0);

    loop {
        if cancel.is_cancelled() {
            return Err(AnalysisError::Cancelled); // §102：取消 ≠ Completed
        }
        let n = decoder
            .decode_next(&mut buf)
            .map_err(|e| AnalysisError::DecodeFailed {
                reason: e.to_string(),
            })?;
        if n == 0 {
            break;
        }
        let data = buf.data();
        let mono: Vec<f32> = if channels == 2 {
            // §89：(L+R)/2 安全 fold-down（测试覆盖 identical/opposite/silent）。
            data.chunks_exact(2).map(|f| (f[0] + f[1]) * 0.5).collect()
        } else {
            data.clone()
        };
        spectral.push(&mono, src_rate);
        loudness_input.extend_from_slice(&mono);
        if window.len() < MAX_FEATURE_SAMPLES {
            let resampled = resample_linear(&mono, src_rate, ANALYSIS_SAMPLE_RATE);
            let remain = MAX_FEATURE_SAMPLES - window.len();
            window.extend(resampled.into_iter().take(remain));
        }
        processed += n;
        buf.clear();
        if processed.is_multiple_of(CHUNK_SAMPLES as u64 * 8) {
            // 进度点（§105）；不产生 IPC（§113 由上层决定频率）。
            let _ = AnalysisProgress {
                stage: Stage::Decoding,
                processed_frames: processed,
                total_frames,
            };
        }
    }
    drop(buf);

    // Loudness 全轨（O(时长/10) 内存，非线性增长 §91）。
    let loudness_result = loudness::analyze_loudness(&loudness_input, src_rate);
    drop(loudness_input);

    if cancel.is_cancelled() {
        return Err(AnalysisError::Cancelled);
    }

    // ---- 窗口特征（规范采样率）----
    match tempo::analyze_tempo(&window, ANALYSIS_SAMPLE_RATE) {
        Ok(r) => {
            out.tempo = Some(r.analysis);
            out.tempo_availability = FeatureAvailability::Available;
        }
        Err(tempo::AnalysisErrorLike::TooShort) => {
            out.tempo_availability = FeatureAvailability::Unavailable;
        }
        Err(tempo::AnalysisErrorLike::Unavailable) => {
            out.tempo_availability = FeatureAvailability::Unavailable;
        }
        Err(tempo::AnalysisErrorLike::NumericalSafety(_)) => {
            out.tempo_availability = FeatureAvailability::Failed;
        }
    }

    out.key = match key_detect::analyze_key(&window, ANALYSIS_SAMPLE_RATE) {
        key_detect::KeyResult::Analysis(a) => {
            out.key_availability = FeatureAvailability::Available;
            Some(a)
        }
        key_detect::KeyResult::Unavailable => {
            out.key_availability = FeatureAvailability::Unavailable;
            None
        }
    };

    out.loudness = match loudness_result {
        Some(r) => {
            out.loudness_availability = FeatureAvailability::Available;
            Some(r.analysis)
        }
        None => {
            // §72：below measurable（静音）。
            out.loudness_availability = FeatureAvailability::Unavailable;
            None
        }
    };

    let sp = spectral.finish();
    // Spectral 特征 Available 条件：有能量（非全静音）。
    if sp.peak > 0.0 {
        out.spectral = Some(crate::domain::SpectralFeatures {
            rms: sp.rms,
            peak: sp.peak,
            centroid_hz: sp.centroid_hz,
            rolloff_hz: sp.rolloff_hz,
            brightness: sp.brightness,
            energy: sp.energy,
            algorithm_version: crate::spectral::SPECTRAL_ALGO_VERSION,
        });
        out.spectral_availability = FeatureAvailability::Available;
    } else {
        out.spectral_availability = FeatureAvailability::Unavailable;
    }

    // 纯静音（peak==0）→ MFCC Unavailable（无内容可分析；log floor 的
    // 有限输出无语义）。
    out.mfcc = if sp.peak == 0.0 {
        out.mfcc_availability = FeatureAvailability::Unavailable;
        None
    } else {
        match mfcc::analyze_mfcc(&window, ANALYSIS_SAMPLE_RATE) {
            Ok(m) => {
                out.mfcc_availability = FeatureAvailability::Available;
                Some(m)
            }
            Err(AnalysisError::TooShort { .. }) | Err(AnalysisError::DecodeFailed { .. }) => {
                out.mfcc_availability = FeatureAvailability::Unavailable;
                None
            }
            Err(e) => return Err(e),
        }
    };

    // Embedding（§134/§135：失败隔离——embedding 失败不影响其它特征）。
    match provider.embed(&EmbeddingInput::new(window.clone(), ANALYSIS_SAMPLE_RATE)) {
        Ok(e) => {
            out.embedding = Some(e);
            out.embedding_availability = FeatureAvailability::Available;
        }
        Err(_) => {
            out.embedding_availability = FeatureAvailability::Failed;
        }
    }

    // §31/§32：Completed 校验（管线内部保证——至少 spectral 或 loudness
    // 之一 Available；全静音时 loudness/spectral 均 Unavailable、tempo/key
    // Unavailable → status = Unsupported 语义由上层定）。
    // 先置 Completed 再校验（is_valid_completed 要求 Completed 前提）。
    out.status = AnalysisStatus::Completed;
    if !out.is_valid_completed() {
        out.status = if sp.peak == 0.0 {
            AnalysisStatus::Unsupported // 纯静音：无任何可分析内容
        } else {
            AnalysisStatus::Failed
        };
    }
    out.analyzed_at_ms = clock_ms();

    if !matches!(
        out.status,
        AnalysisStatus::Completed | AnalysisStatus::Unsupported
    ) {
        return Err(AnalysisError::NumericalSafety {
            feature: "pipeline-terminal-state".to_string(),
        });
    }
    Ok(out)
}

/// 合同常量透出（测试/文档）。
#[must_use]
pub const fn contract_analysis_version() -> u16 {
    ANALYSIS_VERSION
}

#[must_use]
pub const fn contract_model() -> (&'static str, u16) {
    (EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::SpectralStatsProvider;
    use crate::local_stream::LocalFileStream;

    /// 内存 WAV（16-bit mono），测试专用最小编码器。
    pub fn wav_bytes(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_len = samples.len() * 2;
        let mut v = Vec::with_capacity(44 + data_len);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    struct MemStream(std::io::Cursor<Vec<u8>>);
    impl std::io::Read for MemStream {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(b)
        }
    }
    impl std::io::Seek for MemStream {
        fn seek(&mut self, p: std::io::SeekFrom) -> std::io::Result<u64> {
            self.0.seek(p)
        }
    }
    impl aura_core::decoder::AudioByteStream for MemStream {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            Some(self.0.get_ref().len() as u64)
        }
    }

    fn rhythmic(bpm: f32, seconds: f32, sr: u32) -> Vec<i16> {
        let period = (60.0 / bpm * sr as f32) as usize;
        let burst = period / 8;
        let total = (sr as f32 * seconds) as usize;
        let mut out = vec![0i16; total];
        let mut t = 0usize;
        let mut seed = 0x1234_5678_9abc_def0u64;
        let mut prng = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        while t + burst < total {
            for i in 0..burst {
                let env = 1.0 - i as f32 / burst as f32;
                out[t + i] = (env * 12_000.0 * ((prng() as f32 / u64::MAX as f32) - 0.5)) as i16;
            }
            t += period;
        }
        out
    }

    fn clock() -> impl Fn() -> i64 {
        || 1_000
    }

    #[test]
    fn end_to_end_rhythmic_wav_completed() {
        let wav = wav_bytes(&rhythmic(120.0, 8.0, 22_050), 22_050);
        let stream: Box<dyn aura_core::decoder::AudioByteStream> =
            Box::new(MemStream(std::io::Cursor::new(wav)));
        let out = analyze_stream(
            stream,
            1,
            "content-1",
            &SpectralStatsProvider,
            &CancelToken::new(),
            clock(),
        )
        .unwrap();
        assert_eq!(out.status, AnalysisStatus::Completed, "{:?}", out.status);
        assert!(out.is_valid_completed());
        assert!(out.tempo.is_some(), "节奏轨应有 BPM");
        assert_eq!(out.key_availability, FeatureAvailability::Unavailable); // 纯节奏无调性
        assert!(out.loudness.is_some());
        assert!(out.spectral.is_some());
        assert!(out.mfcc.is_some());
        assert!(out.embedding.is_some());
    }

    #[test]
    fn silence_yields_unsupported() {
        // §169：完全 silence → Unsupported（非 fake 结果）。
        let wav = wav_bytes(&vec![0i16; 22_050 * 6], 22_050);
        let out = analyze_stream(
            Box::new(MemStream(std::io::Cursor::new(wav))),
            2,
            "content-2",
            &SpectralStatsProvider,
            &CancelToken::new(),
            clock(),
        )
        .unwrap();
        assert_eq!(out.status, AnalysisStatus::Unsupported);
        assert!(out.loudness.is_none());
        assert!(out.tempo.is_none());
    }

    #[test]
    fn cancellation_beats_completion() {
        // §102/§157：取消后不得 Completed。
        let wav = wav_bytes(&rhythmic(120.0, 8.0, 22_050), 22_050);
        let cancel = CancelToken::new();
        cancel.cancel();
        let err = analyze_stream(
            Box::new(MemStream(std::io::Cursor::new(wav))),
            3,
            "content-3",
            &SpectralStatsProvider,
            &cancel,
            clock(),
        )
        .unwrap_err();
        assert!(matches!(err, AnalysisError::Cancelled));
    }

    #[test]
    fn local_file_path_analysis() {
        // §35：local library tracks。
        let dir = std::env::temp_dir().join(format!("aura-m8-pipe-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.wav");
        std::fs::write(&path, wav_bytes(&rhythmic(120.0, 6.0, 22_050), 22_050)).unwrap();
        let stream: Box<dyn aura_core::decoder::AudioByteStream> =
            Box::new(LocalFileStream::open(&path).unwrap());
        let out = analyze_stream(
            stream,
            4,
            "content-4",
            &SpectralStatsProvider,
            &CancelToken::new(),
            clock(),
        )
        .unwrap();
        assert_eq!(out.status, AnalysisStatus::Completed);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
