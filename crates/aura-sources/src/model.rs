//! 来源领域类型（§2/§10/§52–§54/§56/§57/§91/§108）。
//! 时间与网络都不过界：结构只携带数据，行为在 source 实现里。

/// 来源种类（§10）：强类型，禁止 free-form string。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    Local,
    DirectHttp,
    InternetRadio,
    RadioBrowser,
    Podcast,
    Hls,
}

/// 流形态（§57）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    /// 有确定长度（可下载文件）。
    Finite,
    /// 持续直播（internet radio）。
    Live,
    /// 有内容但长度未知。
    UnknownDuration,
}

/// seek 能力（§56）：不伪造。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekCapability {
    Seekable,
    NonSeekable,
}

/// 缓存政策（§2/§94）。未知一律 `SessionOnly`，绝不默认 `Allowed`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePolicy {
    /// 允许持久缓存（本地文件）。
    Allowed,
    /// 仅会话内内存。
    SessionOnly,
    /// 禁止缓存（internet radio：只允许 ≤30s 内存滚动缓冲，§37）。
    Forbidden,
}

/// Source 生命周期（§108）：单一状态机，不是一堆 bool。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    Created,
    Resolving,
    Ready,
    Streaming,
    Ended,
    Failed,
    Cancelled,
}

/// 解析出的单条流（§9）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStream {
    /// 最终流 URL（已通过 URL 校验；不含凭据，§15）。
    pub url: String,
    pub mime: Option<String>,
    /// codec 短名（如 mp3/ogg/flac）；最终以 decoder probing 为准（§77）。
    pub codec: Option<String>,
    pub bitrate_bps: Option<u64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub seekable: SeekCapability,
    pub stream_type: StreamType,
    /// 字节长度（有限流有；live/未知 None）。Range seek 的前提。
    pub byte_size: Option<u64>,
}

/// 轨级 metadata（§54）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackMeta {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_seconds: Option<u64>,
}

/// 流级 metadata（§54）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamMeta {
    pub station: Option<String>,
    pub homepage: Option<String>,
    pub genre: Option<String>,
    pub icy_name: Option<String>,
}

/// 提供方 metadata（§54）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderMeta {
    /// 例如 "Radio Browser" / "Local Filesystem"。
    pub name: Option<String>,
    /// 提供方 API/页面。
    pub url: Option<String>,
}

/// 三类 metadata 的合集（§54：不塞一个 HashMap）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceMetadata {
    pub track: TrackMeta,
    pub stream: StreamMeta,
    pub provider: ProviderMeta,
}

/// 许可（§52）：不猜测；只有明确证据才声明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedLicense {
    Unknown,
    Known { id: String, name: String },
}

/// 署名（§53）：没有明确要求就不编造。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attribution {
    pub provider: Option<String>,
    pub title: Option<String>,
    pub source_url: Option<String>,
    /// 要求展示的署名文本；`None` = 无明确要求。
    pub required_text: Option<String>,
}

/// Source 解析结果（§2/§9）。
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub kind: SourceKind,
    pub streams: Vec<ResolvedStream>,
    pub meta: SourceMetadata,
    pub license: ResolvedLicense,
    pub cache_policy: CachePolicy,
    pub attribution: Attribution,
}

/// 流级 metadata 事件（§35/§91）：backend 事件，UI 由 M6 接。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamMetadataEvent {
    pub station: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
    /// 原始 StreamTitle（已 UTF-8 安全转换；解析失败不产生事件）。
    pub raw: Option<String>,
}
