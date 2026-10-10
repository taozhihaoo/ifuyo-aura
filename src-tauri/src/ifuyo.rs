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
    /// 同名 .lrc 的远程地址（由服务器生成器填写）；无则 None。
    lyrics_url: Option<String>,
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
    pub lyrics_url: Option<String>,
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

/// http(s) 且非空——清单里任何要交给客户端访问的地址都先过这一关。
fn is_http_url(u: &str) -> bool {
    u.starts_with("http://") || u.starts_with("https://")
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
        let lyrics = t
            .lyrics_url
            .as_ref()
            // 与曲目地址同一判定：只查 scheme 会把私网/封锁的歌词地址放进列表，
            // UI 一旦去取就是内网探测（测试 lyrics_url_* 正是抓在这）。
            .filter(|u| is_http_url(u) && is_playable(u))
            .cloned();
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
            lyrics_url: lyrics,
        });
    }
    Ok(IfuyoListDto {
        generated_at: raw.generated_at.clone(),
        tracks,
        skipped,
        truncated,
    })
}

/// UI 传入的元数据字符串按 SMTC 面同一上限截断（char 边界，不切半个汉字）。
/// 空白视为缺失——不往媒体面板写一串空格。
fn clip_meta(v: Option<String>) -> Option<String> {
    let t = v?.trim().to_string();
    if t.is_empty() {
        return None;
    }
    let end = t
        .char_indices()
        .nth(SecurityLimits::MAX_METADATA_CHARS)
        .map_or(t.len(), |(i, _)| i);
    Some(t[..end].to_string())
}

/// 取远程歌词：清单 `lyricsUrl` 指向的 .lrc 文本。
///
/// 与曲目地址同一策略（严格客户端：deny_private + 封锁表 + 体积上限），因为
/// 这个 URL 同样是服务器给的。返回形状复用本地歌词的 `Lyrics`，两个歌词面
/// （梦境舞台 / 桌面悬浮）无需分支。
#[tauri::command]
pub fn ifuyo_lyrics(
    state: State<'_, AppState>,
    url: String,
) -> CmdResult<Option<crate::lyrics::Lyrics>> {
    let parsed = state
        .http_strict
        .validate(&url)
        .map_err(|e| AppError::new("SOURCE_BLOCKED", e.to_string(), false))?;
    let text = state
        .http_strict
        .get_text(&parsed, SecurityLimits::MAX_LYRICS_BYTES as u64)
        .map_err(|e| from_err("IFUYO_UNAVAILABLE", e, true))?;
    Ok(crate::lyrics::parse_lrc(&text))
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
///
/// `title`/`artist` 来自清单：不传的话播放条只能拿 URL 末段回退显示（远程路径是
/// percent-encoded，用户会看到 `%E7%8E%B0…`）。UI 传入的字符串按 SMTC 面同一
/// 上限截断，不信任长度。
#[tauri::command]
pub fn ifuyo_play(
    state: State<'_, AppState>,
    url: String,
    title: Option<String>,
    artist: Option<String>,
) -> CmdResult<String> {
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
    let (title, artist) = (clip_meta(title), clip_meta(artist));
    let meta =
        (title.is_some() || artist.is_some()).then_some(crate::integration::CurrentTrackMeta {
            track_id: None,
            title,
            artist,
            album: None,
            album_artist: None,
            artwork_id: None,
        });
    crate::ipc::play_input_impl(
        &state,
        url,
        meta,
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
                    lyrics_url: None,
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

    /// 歌词地址：与曲目同一判定（scheme + 私网/封锁），只查 scheme 会漏。
    #[test]
    fn lyrics_url_uses_the_same_policy_as_track_url() {
        // 与 ifuyo_list 实际注入的判定同形：http(s) 且公网。
        let playable = |u: &str| {
            (u.starts_with("http://") || u.starts_with("https://"))
                && !u.contains("127.0.0.1")
                && !u.contains("192.168.")
        };
        let mut m = raw(vec![("a", "A", "https://x/a.mp3")]);
        m.tracks[0].lyrics_url = Some("https://x/a.lrc".into());
        let out = build_list(&m, playable, 100).expect("list");
        assert_eq!(out.tracks.len(), 1, "曲目本身被过滤掉了，测不到歌词字段");
        assert_eq!(out.tracks[0].lyrics_url.as_deref(), Some("https://x/a.lrc"));

        for bad in [
            "http://127.0.0.1/a.lrc",
            "http://192.168.1.1/a.lrc",
            "file:///c:/a.lrc",
            "",
        ] {
            let mut m2 = raw(vec![("a", "A", "https://x/a.mp3")]);
            m2.tracks[0].lyrics_url = Some(bad.into());
            let o2 = build_list(&m2, playable, 100).expect("list2");
            assert_eq!(o2.tracks.len(), 1, "曲目本身被过滤掉了");
            assert_eq!(o2.tracks[0].lyrics_url, None, "{bad} 不该被接受");
        }
    }

    /// 扩展名判定不能把带点的艺人名截成"艺人名"（例如 "周杰伦.新".mp3）。
    #[test]
    fn stem_only_strips_real_looking_extension() {
        assert_eq!(stem_from_url("https://x/a.b.mp3"), "a.b");
        assert_eq!(stem_from_url("https://x/a.mp4a"), "a");
        assert_eq!(stem_from_url("https://x/dir"), "dir");
        assert_eq!(stem_from_url("https://x/a.mp3?tk=1"), "a");
    }

    /// UI 传入的标题/艺术家会写进 SMTC 与快照：必须按 char 边界截断，
    /// 且空白视为缺失（否则媒体面板出现一串空格）。
    #[test]
    fn clip_meta_bounds_and_blank_handling() {
        assert_eq!(clip_meta(None), None);
        assert_eq!(clip_meta(Some("   ".into())), None);
        assert_eq!(
            clip_meta(Some(" 青花瓷 ".into())).as_deref(),
            Some("青花瓷")
        );
        let long = "周".repeat(SecurityLimits::MAX_METADATA_CHARS + 50);
        let clipped = clip_meta(Some(long)).expect("clip");
        assert_eq!(
            clipped.chars().count(),
            SecurityLimits::MAX_METADATA_CHARS,
            "必须正好截在上限，且不切出半个字符"
        );
        // ASCII 长串同样按字符数截，不按字节（字节切会 panic 在字符中间）
        let ascii = "a".repeat(SecurityLimits::MAX_METADATA_CHARS + 10);
        assert_eq!(
            clip_meta(Some(ascii)).expect("clip2").len(),
            SecurityLimits::MAX_METADATA_CHARS
        );
    }
}
