//! 领域模型（§7）。类型只携带数据；持久化与查询在 `db`/`service`。
//! 时间一律 `i64` 毫秒 epoch，由调用方（应用层/测试）注入，核心逻辑不读 wall-clock。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::ErrorCategory;

macro_rules! opaque_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub i64);
    };
}

opaque_id!(
    /// `tracks` 行主键。路径不是身份（§8）；身份见 `TrackIdentity`。
    TrackId
);
opaque_id!(/// `artworks` 行主键。
    ArtworkId
);
opaque_id!(/// `library_roots` 行主键。
    RootId
);
opaque_id!(/// `playlists` 行主键。
    PlaylistId
);

/// 内容身份（§8/§9）：文件大小 + 首/尾 1MB 的 128 位指纹。
/// 改名/移动不变；标签改写不变（见 `audio_sig` 分辨规则，D-038）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackIdentity {
    /// `"<ver>:<size hex>:<fingerprint hex>"`，见 `identity` 模块。
    pub identity_key: String,
    /// 音频核心属性签名 `"<codec>/<sample_rate>/<channels>/<duration_frames>"`，
    /// 用于把「同路径标签改写」与「音频内容替换」区分开（§10）。
    pub audio_sig: String,
}

/// 标签解析产物（§22/§23/§24）：`None` 表示标签缺失；
/// 空字符串/纯空白在解析层折叠为 `None`。原始字节串不入库（privacy，§63）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub track_number: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_number: Option<u32>,
    pub disc_total: Option<u32>,
    /// 音频时长（帧 = 采样周期数）。
    pub duration_frames: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    /// 统一小写短名，如 `flac` / `mp3` / `vorbis` / `pcm_s16le`。
    pub codec: Option<String>,
    /// 统一小写短名，如 `flac` / `mpeg` / `ogg` / `riff`。
    pub container: Option<String>,
    /// M15a CUE 子轨：在整轨文件中的起点（秒）。
    pub cue_start_seconds: Option<f64>,
    /// M15a CUE 子轨：时长（秒）。
    pub cue_duration_seconds: Option<f64>,
}

impl TrackMetadata {
    /// §10 的音频签名：这四项在标签改写时不变，音频替换时几乎必变。
    pub fn audio_sig(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.codec.as_deref().unwrap_or("?"),
            self.sample_rate.unwrap_or(0),
            self.channels.unwrap_or(0),
            self.duration_frames.unwrap_or(0)
        )
    }
}

/// 从文件内嵌提取的封面字节（§27）。M3 只接受 JPEG/PNG。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artwork {
    pub bytes: Vec<u8>,
    /// `image/jpeg` 或 `image/png`（由字节签名判定，不信任标签声明）。
    pub mime: &'static str,
}

/// 一条已入库曲目（display 文本 = NFC 规范化的用户可见值，§24）。
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub id: TrackId,
    pub identity_key: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration_frames: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub codec: Option<String>,
    pub container: Option<String>,
    pub artwork_id: Option<ArtworkId>,
    pub file_size: u64,
    pub date_added_ms: i64,
    pub last_scanned_ms: i64,
}

/// 播放统计（§54）：逻辑身份绑定；文件删除后随 track 停用而保留（§47）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlaybackStats {
    pub play_count: u64,
    pub skip_count: u64,
    pub total_played_frames: u64,
    pub last_played_at_ms: Option<i64>,
}

/// 扫描中单文件的落点（§15）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Discovered,
    Readable,
    Recognized,
    Imported,
    Skipped,
    Failed,
}

/// 单文件扫描结果（诊断与测试用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOutcome {
    pub path: PathBuf,
    pub status: FileStatus,
    pub category: Option<ErrorCategory>,
}

/// 扫描汇总（§16）。错误只保留 path + category。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanResult {
    pub files_seen: u64,
    pub files_imported: u64,
    pub files_updated: u64,
    pub files_deleted: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
    pub duration_ms: u64,
    pub errors: Vec<(PathBuf, ErrorCategory)>,
}

/// 扫描进度（§68；M3 不接 UI）。
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub discovered: u64,
    pub processed: u64,
    pub imported: u64,
    pub failed: u64,
    pub current_path: PathBuf,
}

/// 扫描取消令牌（§67）。协作式：扫描器在目录/文件边界轮询。
#[derive(Debug, Default, Clone)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}
