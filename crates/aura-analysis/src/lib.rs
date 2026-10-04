//! ifuyo Aura 音频分析：BPM、beat grid、key/Camelot、LUFS、true peak、
//! energy、brightness、MFCC、embedding。
//!
//! 后台低优先级、可取消、可恢复、可缓存、content-key 绑定、逐位确定性。
//!
//! Phase 0 骨架已被 M8 上篇替换（§11 职责边界内）。

pub mod decode;
pub mod domain;
pub mod embedding;
pub mod fft;
pub mod key;
pub mod key_detect;
pub mod local_stream;
pub mod loudness;
pub mod mfcc;
pub mod params;
pub mod spectral;
pub mod store;
pub mod tempo;

pub use decode::{decode_file_to_mono, decode_to_mono, DecodedMono};
pub use domain::{
    AnalysisError, AnalysisStatus, Embedding, FeatureAvailability, KeyAnalysis, KeyMode,
    LoudnessAnalysis, MfccFeatures, MusicalKey, SpectralFeatures, TempoAnalysis, TrackAnalysis,
    ANALYSIS_VERSION, EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION,
};
pub use embedding::{
    DType, DistanceMetric, EmbeddingInput, EmbeddingProvider, Normalization, SpectralStatsProvider,
    EMBEDDING_DIMENSION,
};
pub use fft::{fft_forward, magnitude_half};
pub use key::analysis_key;
pub use key_detect::{analyze_key, camelot, chroma, KeyResult, KEY_ALGO_VERSION};
pub use loudness::{amplitude_for_target_lufs, analyze_loudness, true_peak_dbtp};
pub use mfcc::{analyze_mfcc, MFCC_ALGO_VERSION};
pub use params::{
    hann, resample_linear, ANALYSIS_CHANNELS, ANALYSIS_SAMPLE_RATE, CHUNK_SAMPLES, FRAME_LEN,
    HOP_LEN, MAX_ANALYSIS_SECONDS, MAX_BEATS, MAX_FEATURE_SAMPLES,
};
pub use spectral::{SpectralAccumulator, SpectralResult, SPECTRAL_ALGO_VERSION};
pub use store::{AnalysisStore, StoreError};
pub use tempo::{analyze_tempo, AnalysisErrorLike, TempoResult};
