//! ifuyo Aura 音频分析：BPM、beat grid、key/Camelot、LUFS、true peak、
//! energy、brightness、MFCC、embedding。
//!
//! 后台低优先级、可取消、可恢复、可缓存、content-key 绑定、逐位确定性。
//!
//! Phase 0 骨架已被 M8 上篇替换（§11 职责边界内）。

pub mod decode;
pub mod domain;
pub mod embedding;
pub mod key;
pub mod local_stream;
pub mod store;

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
pub use key::analysis_key;
pub use store::{AnalysisStore, StoreError};
