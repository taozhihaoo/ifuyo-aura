//! Radio Browser 目录 Source（§39–§51/§103/§124）。
//!
//! 发现：`/json/servers`（不硬编码单一服务器，§40）；镜像随机选择
//! （production）/ 注入确定性选择器（test，§41/§124）；失败逐镜像
//! fallback，上限可配（§42）；`url_resolved` 优先（§45）；codec 过滤
//! （§46）；click 上报默认关闭（§47）；离线全失败 → `SourceUnavailable`
//! 不换服务（§49）；响应体上限 + UA（§50/§43）。

use serde::Deserialize;

use crate::error::{Result, SourceError};
use crate::http::{HttpClient, USER_AGENT};
use crate::model::{
    Attribution, CachePolicy, ProviderMeta, Resolved, ResolvedLicense, ResolvedStream,
    SeekCapability, SourceKind, SourceMetadata, StreamMeta, StreamType, TrackMeta,
};

/// 发现端点基址（§40：`all.api.radio-browser.info`；mirrors 从此枚举）。
pub const DISCOVERY_BASE: &str = "https://all.api.radio-browser.info";

/// 默认最多尝试的镜像数（§42：有限重试）。
pub const DEFAULT_MAX_MIRRORS: usize = 3;

/// 文本响应上限：/json/servers 64 KiB；/json/stations/search 8 MiB（§50）。
const SERVERS_LIMIT: u64 = 64 << 10;
const STATIONS_LIMIT: u64 = 8 << 20;

/// 镜像选择器：production 随机 / test 注入固定（§41/§124）。
pub type MirrorSelector = Box<dyn Fn(usize) -> usize + Send>;

/// 随机选择器（生产）。
pub fn random_selector() -> MirrorSelector {
    Box::new(|n: usize| {
        use std::hash::{BuildHasher, Hasher, RandomState};
        let h = RandomState::new().build_hasher().finish();
        usize::try_from(h % n as u64).unwrap_or(0)
    })
}

/// 顺序选择器（测试注入：固定取第 k 个）。
pub fn fixed_selector(k: usize) -> MirrorSelector {
    Box::new(move |n: usize| k.min(n.saturating_sub(1)))
}

/// `/json/servers` 单条（§80：未知字段 serde 自动忽略；缺失字段默认）。
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RbServer {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub hostname: String,
}

/// 电台条目（§44：以真实 response 字段为准的常用子集）。
#[derive(Debug, Clone, PartialEq)]
pub struct Station {
    pub stationuuid: String,
    pub name: String,
    /// 播放 URL（原始）。
    pub url: String,
    /// 解析后 URL —— 播放一律优先用它（§45）。
    pub url_resolved: String,
    pub codec: String,
    pub bitrate: u64,
    pub country: String,
    pub language: String,
    pub homepage: String,
    pub tags: String,
}

impl Station {
    /// §45：播放 URL 的选择与记录。
    #[must_use]
    pub fn stream_url(&self) -> (&str, &'static str) {
        if !self.url_resolved.trim().is_empty() {
            (&self.url_resolved, "url_resolved")
        } else if !self.url.trim().is_empty() {
            (&self.url, "url")
        } else {
            ("", "none")
        }
    }
}

/// 电台搜索条件（§44）。
#[derive(Debug, Clone, Default)]
pub struct StationQuery {
    pub name: Option<String>,
    pub country: Option<String>,
    pub language: Option<String>,
    /// codec 过滤（§46）：如 ["MP3", "OGG", "FLAC"]；None = 不过滤（由
    /// 排序策略给出兼容性提示）。
    pub codecs: Option<Vec<String>>,
    pub limit: u32,
}

/// 纯解析：`/json/servers`（§80：truncated/invalid → Err，不 panic）。
///
/// # Errors
/// 非 JSON 数组/类型不符 → [`SourceError::ParseError`]。
pub fn parse_servers(json: &str) -> Result<Vec<RbServer>> {
    serde_json::from_str(json).map_err(|e| SourceError::ParseError {
        reason: crate::error::short(&e.to_string()),
    })
}

/// 纯解析：`/json/stations/search`（§80：逐条容错 —— 坏条目跳过并计数，
/// 其余保留；整体非数组 → ParseError）。
pub fn parse_stations(json: &str) -> Result<(Vec<Station>, u64)> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| SourceError::ParseError {
            reason: crate::error::short(&e.to_string()),
        })?;
    let Some(entries) = value.as_array() else {
        return Err(SourceError::ParseError {
            reason: "stations response is not an array".into(),
        });
    };
    let mut out = Vec::new();
    let mut skipped = 0u64;
    for entry in entries {
        match serde_json::from_value::<StationRaw>(entry.clone()) {
            Ok(raw) => out.push(raw.into_station()),
            Err(_) => skipped += 1, // 坏条目跳过（§80 fallback）
        }
    }
    Ok((out, skipped))
}

#[derive(Debug, Clone, Deserialize)]
struct StationRaw {
    #[serde(default)]
    stationuuid: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    url_resolved: String,
    #[serde(default)]
    codec: String,
    #[serde(default)]
    bitrate: serde_json::Value,
    #[serde(default)]
    country: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    homepage: String,
    #[serde(default)]
    tags: String,
}

impl StationRaw {
    fn into_station(self) -> Station {
        // bitrate 兼容 "128"/128/null（§80：已知字段类型错误 → fallback 值 0）。
        let bitrate = match &self.bitrate {
            serde_json::Value::Number(n) => n.as_u64().unwrap_or(0),
            serde_json::Value::String(s) => s.trim().parse().unwrap_or(0),
            _ => 0,
        };
        Station {
            stationuuid: self.stationuuid,
            name: self.name,
            url: self.url,
            url_resolved: self.url_resolved,
            codec: self.codec,
            bitrate,
            country: self.country,
            language: self.language,
            homepage: self.homepage,
            tags: self.tags,
        }
    }
}

/// §46/§90：codec 兼容与排序评分。M4-A 支持解码的 codec 优先（高码率优先），
/// AAC 在 WMF（B 级）缺席时标记不支持 —— 不 panic、不自动换源（§78）。
#[must_use]
pub fn codec_supported(codec: &str) -> bool {
    let c = codec.trim().to_ascii_uppercase();
    matches!(c.as_str(), "MP3" | "OGG" | "FLAC" | "VORBIS" | "")
}

/// 流选择评分：支持 > 高码率；稳定（同分按名称再按 uuid）。
#[must_use]
pub fn station_sort_key(s: &Station) -> (u8, u64, String, String) {
    let supported = u8::from(codec_supported(&s.codec));
    (
        8 - supported,
        s.bitrate,
        s.name.to_lowercase(),
        s.stationuuid.clone(),
    )
}

/// Radio Browser 目录 Source。
pub struct RadioBrowserSource {
    http: HttpClient,
    discovery_base: String,
    /// 镜像 API scheme（生产 `https`；测试注入 `http` 以使用 fake server）。
    mirror_scheme: String,
    max_mirrors: usize,
    selector: MirrorSelector,
    /// §47：click 上报默认 OFF；开启才发送 `/json/url/<uuid>`。
    pub click_reporting_enabled: bool,
}

impl std::fmt::Debug for RadioBrowserSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioBrowserSource")
            .field("discovery_base", &self.discovery_base)
            .field("max_mirrors", &self.max_mirrors)
            .field("click_reporting_enabled", &self.click_reporting_enabled)
            .finish_non_exhaustive()
    }
}

impl RadioBrowserSource {
    /// 生产构造：随机镜像选择。
    ///
    /// # Errors
    /// HttpClient 构建失败。
    pub fn new(http: HttpClient) -> Self {
        Self::with_parts(
            http,
            DISCOVERY_BASE.to_string(),
            "https".to_string(),
            DEFAULT_MAX_MIRRORS,
            random_selector(),
        )
    }

    /// 测试构造：注入发现基址与确定性选择器（§89/§124）。
    #[must_use]
    pub fn with_parts(
        http: HttpClient,
        discovery_base: String,
        mirror_scheme: String,
        max_mirrors: usize,
        selector: MirrorSelector,
    ) -> Self {
        Self {
            http,
            discovery_base,
            mirror_scheme,
            max_mirrors: max_mirrors.max(1),
            selector,
            click_reporting_enabled: false, // §47：默认 OFF
        }
    }

    /// §40：发现镜像列表（`GET {base}/json/servers`）。
    ///
    /// # Errors
    /// 全部失败 → [`SourceError::SourceUnavailable`]（§49：不换未知服务）。
    pub fn discover_mirrors(&self) -> Result<Vec<String>> {
        let url = format!("{}/json/servers", self.discovery_base);
        let parsed = self.http.validate(&url)?;
        // §49：发现阶段的网络/超时/超限失败统一归为 SourceUnavailable
        // （应用层显示"目录不可用"；不静默换第三方服务）。
        let text = self
            .http
            .get_text(&parsed, SERVERS_LIMIT)
            .map_err(|e| match e {
                SourceError::ParseError { reason } => SourceError::ParseError { reason },
                other => SourceError::SourceUnavailable {
                    reason: crate::error::short(&other.to_string()),
                },
            })?;
        let servers = parse_servers(&text)?;
        let hosts: Vec<String> = servers
            .into_iter()
            .map(|s| s.hostname.trim().to_string())
            .filter(|h| !h.is_empty())
            .collect();
        if hosts.is_empty() {
            return Err(SourceError::SourceUnavailable {
                reason: "server list empty".into(),
            });
        }
        Ok(hosts)
    }

    /// §42/§44：镜像循环执行 `f(mirror_base)`；选中→失败→下一个，≤ max。
    fn with_fallback<T>(
        &self,
        mirrors: &[String],
        mut f: impl FnMut(&str) -> Result<T>,
    ) -> Result<T> {
        if mirrors.is_empty() {
            return Err(SourceError::SourceUnavailable {
                reason: "no mirrors discovered".into(),
            });
        }
        let start = (self.selector)(mirrors.len()).min(mirrors.len() - 1);
        let mut tried = 0;
        let mut last_err = SourceError::SourceUnavailable {
            reason: "unreachable".into(),
        };
        for i in 0..mirrors.len() {
            if tried >= self.max_mirrors {
                break;
            }
            tried += 1;
            let host = &mirrors[(start + i) % mirrors.len()];
            let base = format!("{}://{host}", self.mirror_scheme);
            match f(&base) {
                Ok(v) => return Ok(v),
                Err(e) => last_err = e,
            }
        }
        Err(SourceError::SourceUnavailable {
            reason: crate::error::short(&format!("all {tried} mirrors failed; last: {last_err}")),
        })
    }

    /// §44：基本搜索（name/country/language/codec 过滤 + 限制）。
    ///
    /// # Errors
    /// 发现失败/全部镜像失败/解析失败。
    pub fn search(&self, query: &StationQuery) -> Result<Vec<Station>> {
        let mirrors = self.discover_mirrors()?;
        let limit = if query.limit == 0 { 100 } else { query.limit };
        let mut params: Vec<(String, String)> = vec![
            ("limit".into(), limit.to_string()),
            ("hidebroken".into(), "true".into()),
            ("order".into(), "votes".into()),
        ];
        if let Some(n) = &query.name {
            params.push(("name".into(), n.clone()));
        }
        if let Some(c) = &query.country {
            params.push(("country".into(), c.clone()));
        }
        if let Some(l) = &query.language {
            params.push(("language".into(), l.clone()));
        }
        // codec 过滤：Radio Browser 用逗号分隔（多个 codec 参数）。
        if let Some(codecs) = &query.codecs {
            if !codecs.is_empty() {
                params.push(("codec".into(), codecs.join(",")));
            }
        }
        let mut enc = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in &params {
            enc.append_pair(k, v);
        }
        let query_string = enc.finish();

        self.with_fallback(&mirrors, |base| {
            let url_text = format!("{base}/json/stations/search?{query_string}");
            let parsed = self.http.validate(&url_text)?;
            let text = self.http.get_text(&parsed, STATIONS_LIMIT)?;
            let (stations, _skipped) = parse_stations(&text)?;
            Ok(stations)
        })
    }

    /// §45/§46/§52/§53/§94：电台 → Resolved。
    /// `url_resolved` 优先，回退 `url`（D-054：`stream_url` 返回实际使用的
    /// 字段名，不静默）。
    #[must_use]
    pub fn resolve_station(&self, s: &Station) -> Option<Resolved> {
        let (stream_url, _url_field) = s.stream_url();
        if stream_url.is_empty() {
            return None;
        }
        let stream = ResolvedStream {
            url: stream_url.to_string(),
            mime: None,
            codec: Some(s.codec.to_lowercase()),
            bitrate_bps: Some(s.bitrate * 1000),
            sample_rate: None,
            channels: None,
            seekable: SeekCapability::NonSeekable,
            stream_type: StreamType::Live,
        };
        Some(Resolved {
            kind: SourceKind::RadioBrowser,
            streams: vec![stream],
            meta: SourceMetadata {
                track: TrackMeta {
                    title: Some(s.name.clone()),
                    ..TrackMeta::default()
                },
                stream: StreamMeta {
                    station: Some(s.name.clone()),
                    homepage: if s.homepage.is_empty() {
                        None
                    } else {
                        Some(s.homepage.clone())
                    },
                    genre: if s.tags.is_empty() {
                        None
                    } else {
                        Some(s.tags.clone())
                    },
                    icy_name: None,
                },
                provider: ProviderMeta {
                    name: Some("Radio Browser".into()),
                    url: Some(self.discovery_base.clone()),
                },
            },
            // §51/§52：目录 metadata ≠ 音频许可；不假定（Unknown）。
            license: ResolvedLicense::Unknown,
            // §94：Radio Browser 目录本身——电台音频仍按电台政策（Forbidden）。
            cache_policy: CachePolicy::Forbidden,
            attribution: Attribution {
                provider: Some("Radio Browser".into()),
                title: Some(s.name.clone()),
                source_url: if s.homepage.is_empty() {
                    None
                } else {
                    Some(s.homepage.clone())
                },
                // §53：无明确署名要求 → 不编造。
                required_text: None,
            },
        })
    }

    /// §47：click 上报（仅当开关开启；关闭 = 零请求）。
    ///
    /// # Errors
    /// 全镜像失败/解析失败。
    pub fn report_click(&self, stationuuid: &str) -> Result<()> {
        if !self.click_reporting_enabled {
            return Ok(()); // 关闭：不发任何请求（§47 不能偷偷上报）
        }
        let mirrors = self.discover_mirrors()?;
        self.with_fallback(&mirrors, |base| {
            let url_text = format!("{base}/json/url/{stationuuid}");
            let parsed = self.http.validate(&url_text)?;
            self.http.get_text(&parsed, SERVERS_LIMIT)?;
            Ok(())
        })
    }

    /// UA 透出（§43：不得隐藏身份）。
    #[must_use]
    pub fn user_agent() -> &'static str {
        USER_AGENT
    }
}
