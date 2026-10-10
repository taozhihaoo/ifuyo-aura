//! ifuyo 远程曲库清单（自托管音乐的客户端面）。
//!
//! 服务器侧是一份**静态 JSON**（`scripts/gen-ifuyo-manifest.py` 生成，cron 定时
//! 更新），清单由 Rust 拉取——不是 WebView `fetch`：应用 CSP 是 `connect-src ipc:`
//! （§IPC 边界），远程内容必须经过我们自己的校验链。
//!
//! 策略要点：清单里的 URL 由服务器给出，用户没有逐条确认过，因此曲目地址一律走
//! [`AppState::http_strict`]（deny_private + 凭据拒绝 + 平台封锁表）。这跟
//! "Open URL"（D-070：用户显式输入即视为确认私网可达）刻意不同——否则一份被改过
//! 的清单就能把客户端指向用户内网或云元数据端点。清单地址本身是编译期常量，
//! 不接受 UI/参数注入。

use serde::Deserialize;
use serde::Serialize;
use tauri::State;

use crate::ipc::{from_err, AppError, AppState, CmdResult};
use crate::security_limits::SecurityLimits;

/// 唯一的清单地址（与服务器 `--base-url` 对应的站点路径一致）。
pub const MANIFEST_URL: &str = "https://ifuyo.com/audio/aura-manifest.json";

/// 生成器当前产出的版本；不认识的版本 fail-closed（不猜字段含义）。
const SUPPORTED_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTrack {
    id: String,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    url: String,
    duration_seconds: Option<f64>,
    size_bytes: Option<u64>,
    format: Option<String>,
    etag: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawManifest {
    version: u32,
    generated_at: Option<String>,
    tracks: Vec<RawTrack>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IfuyoTrackDto {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub url: String,
    /// `null` 是真实数据形态（无标签占位文件），UI 必须显示 `--:--` 而不是 0:00。
    pub duration_seconds: Option<f64>,
    pub size_bytes: Option<u64>,
    pub format: Option<String>,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IfuyoListDto {
    pub generated_at: Option<String>,
    pub tracks: Vec<IfuyoTrackDto>,
    /// 被丢弃的条数（不可播 URL / 重复 URL），如实上报而不是静默消失。
    pub skipped: usize,
    /// 超过上限被截断（大清单在 UI 上必须看得出来是不完整的）。
    pub truncated: bool,
}

/// 标题缺失时的回退：URL 末段（percent-decode）去扩展名。与 M3"无标签曲目以
/// 路径回退显示"同口径，不伪造元数据。
fn stem_from_url(url: &str) -> String {
    let last = url
        .split(['?', '#'])
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .unwrap_or("");
    let decoded = percent_decode(last);
    let trimmed = decoded.rsplit_once('.').map_or_else(
        || decoded.clone(),
        |(head, _)| {
            // 只有"看起来像扩展名"才截断（≤5 个 ASCII 字母数字）
            if head.is_empty() || extension_like(decoded.rsplit_once('.').map_or("", |(_, t)| t)) {
                head.to_string()
            } else {
                decoded.clone()
            }
        },
    );
    if trimmed.trim().is_empty() {
        decoded
    } else {
        trimmed
    }
}

fn extension_like(ext: &str) -> bool {
    !ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 纯函数（可单测，不碰网络）：版本门 → 逐条校验/去重/截断 → DTO。
fn build_list(
    raw: &RawManifest,
    mut is_playable: impl FnMut(&str) -> bool,
    cap: usize,
) -> Result<IfuyoListDto, String> {
    if raw.version != SUPPORTED_VERSION {
        return Err(format!(
            "清单版本 {} 不支持（客户端支持 v{SUPPORTED_VERSION}）",
            raw.version
        ));
    }
    let mut tracks: Vec<IfuyoTrackDto> = Vec::with_capacity(raw.tracks.len().min(cap));
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut skipped = 0usize;
    let mut truncated = false;
    for t in &raw.tracks {
        if t.url.trim().is_empty() || !is_playable(&t.url) || !seen.insert(t.url.as_str()) {
            // 不可播（私网/凭据/非法 scheme/封锁表）与重复 URL 都不进列表。
            skipped += 1;
            continue;
        }
        if tracks.len() >= cap {
            truncated = true;
            break;
        }
        tracks.push(IfuyoTrackDto {
            id: t.id.clone(),
            title: t
                .title
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| stem_from_url(&t.url)),
            artist: t.artist.clone().filter(|s| !s.trim().is_empty()),
            album: t.album.clone().filter(|s| !s.trim().is_empty()),
            url: t.url.clone(),
            duration_seconds: t.duration_seconds,
            size_bytes: t.size_bytes,
            format: t.format.clone(),
            etag: t.etag.clone(),
        });
    }
    Ok(IfuyoListDto {
        generated_at: raw.generated_at.clone(),
        tracks,
        skipped,
        truncated,
    })
}

/// 拉取并校验远程清单。错误面：网络/解析/版本分开报，`recoverable=true` 让 UI
/// 可以重试（清单是幂等 GET）。
#[tauri::command]
pub fn ifuyo_list(state: State<'_, AppState>) -> CmdResult<IfuyoListDto> {
    let parsed = state
        .http_strict
        .validate(MANIFEST_URL)
        .map_err(|e| AppError::new("SOURCE_BLOCKED", e.to_string(), false))?;
    let text = state
        .http_strict
        .get_text(&parsed, SecurityLimits::IFUYO_MANIFEST_MAX_BYTES as u64)
        .map_err(|e| from_err("IFUYO_UNAVAILABLE", e, true))?;
    let raw: RawManifest =
        serde_json::from_str(&text).map_err(|e| from_err("IFUYO_MANIFEST", e, true))?;
    let blocklist = std::sync::Arc::clone(&state.blocklist);
    build_list(
        &raw,
        move |u| {
            aura_sources::url_policy::validate_url(
                u,
                &aura_sources::url_policy::NetworkPolicy::deny_private(),
                &blocklist,
            )
            .is_ok()
        },
        SecurityLimits::IFUYO_TRACKS_MAX,
    )
    .map_err(|e| from_err("IFUYO_MANIFEST", e, true))
}

/// 播放远程曲目（直连流式；本地缓存优先是下一批的面）。
#[tauri::command]
pub fn ifuyo_play(state: State<'_, AppState>, url: String) -> CmdResult<String> {
    // 先用 Arc 副本判定，避免把 state 借出去后又移动它。
    let blocklist = std::sync::Arc::clone(&state.blocklist);
    let policy = aura_sources::url_policy::NetworkPolicy::deny_private();
    if aura_sources::url_policy::validate_url(&url, &policy, &blocklist).is_err() {
        return Err(AppError::new(
            "SOURCE_BLOCKED",
            "远程清单中的该地址未通过安全校验（私网/凭据/非法地址）",
            false,
        ));
    }
    crate::ipc::play_input_impl(
        &state,
        url,
        None,
        None,
        aura_core::TrackLoudness::missing(),
        &state.http_strict,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(items: Vec<(&str, &str, &str)>) -> RawManifest {
        RawManifest {
            version: 1,
            generated_at: Some("2026-10-10T07:04:08+00:00".into()),
            tracks: items
                .into_iter()
                .map(|(id, title, url)| RawTrack {
                    id: id.into(),
                    title: if title.is_empty() {
                        None
                    } else {
                        Some(title.into())
                    },
                    artist: None,
                    album: None,
                    url: url.into(),
                    duration_seconds: None,
                    size_bytes: Some(1024),
                    format: Some("mp3".into()),
                    etag: None,
                })
                .collect(),
        }
    }

    /// 版本门：不认识的版本必须 fail-closed，不能"尽力解析"。
    #[test]
    fn unsupported_version_fails_closed() {
        let mut m = raw(vec![("a", "A", "https://x/a.mp3")]);
        m.version = 2;
        let e = build_list(&m, |_| true, 100).expect_err("v2 必须拒绝");
        assert!(e.contains("不支持"), "{e}");
    }

    /// 不可播地址与重复地址不进列表，但必须**如实计数**（静默消失会让人以为
    /// 服务器少给了歌）。
    #[test]
    fn unplayable_and_duplicate_urls_are_skipped_and_counted() {
        let m = raw(vec![
            ("a", "A", "https://music.ifuyo.com/a.mp3"),
            ("b", "B", "http://127.0.0.1/b.mp3"),
            ("c", "C", "http://192.168.1.9/c.mp3"),
            ("d", "D", "https://music.ifuyo.com/a.mp3"), // 与 a 同 URL
            ("e", "E", "file:///e:/x.mp3"),
            ("f", "F", ""),
        ]);
        let out = build_list(
            &m,
            |u| {
                !u.contains("127.0.0.1")
                    && !u.contains("192.168.")
                    && !u.starts_with("file:")
                    && !u.trim().is_empty()
            },
            100,
        )
        .expect("list");
        assert_eq!(out.tracks.len(), 1, "只剩 https 公网那条");
        assert_eq!(out.skipped, 5, "私网×2 + 重复 + file: + 空 URL");
        assert!(!out.truncated);
    }

    /// 真实数据形态：artist/duration 为 null 必须原样保留（UI 显示 `--:--`），
    /// 不能填 0 让人以为曲目是 0 秒。
    #[test]
    fn null_metadata_stays_null() {
        let m = raw(vec![("a", "A", "https://x/a.mp3")]);
        let out = build_list(&m, |_| true, 100).expect("list");
        assert!(out.tracks[0].duration_seconds.is_none());
        assert!(out.tracks[0].artist.is_none());
        assert_eq!(out.tracks[0].size_bytes, Some(1024));
    }

    /// 标题缺失（服务器只有文件名时）回退到 URL 末段，且要还原 percent-encoding。
    #[test]
    fn title_falls_back_to_decoded_url_stem() {
        let m = raw(vec![("a", "", "https://x/%E9%9D%92%E8%8A%B1%E7%93%B7.mp3")]);
        let out = build_list(&m, |_| true, 100).expect("list");
        assert_eq!(
            out.tracks[0].title, "青花瓷",
            "实际 {}",
            out.tracks[0].title
        );
    }

    /// 空白标题也算缺失（不能被当成有效元数据）。
    #[test]
    fn blank_title_is_treated_as_missing() {
        let m = raw(vec![("a", "   ", "https://x/song.mp3")]);
        let out = build_list(&m, |_| true, 100).expect("list");
        assert_eq!(out.tracks[0].title, "song");
    }

    /// 超出上限：截断并置位标志，UI 才能显示"列表不完整"。
    #[test]
    fn cap_marks_truncated() {
        let urls: Vec<String> = (0..10).map(|i| format!("https://x/{i}.mp3")).collect();
        let items: Vec<(&str, &str, &str)> = urls.iter().map(|u| ("id", "T", u.as_str())).collect();
        let out = build_list(&raw(items), |_| true, 4).expect("list");
        assert_eq!(out.tracks.len(), 4);
        assert!(out.truncated, "必须如实报告被截断");
    }

    /// 扩展名判定不能把带点的艺人名截成"艺人名"（例如 "周杰伦.新".mp3）。
    #[test]
    fn stem_only_strips_real_looking_extension() {
        assert_eq!(stem_from_url("https://x/a.b.mp3"), "a.b");
        assert_eq!(stem_from_url("https://x/a.mp4a"), "a");
        assert_eq!(stem_from_url("https://x/dir"), "dir");
        assert_eq!(stem_from_url("https://x/a.mp3?tk=1"), "a");
    }
}
