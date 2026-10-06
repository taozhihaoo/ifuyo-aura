//! 歌词解析（M13.3）：本地曲目歌词读取与 LRC 解析。
//!
//! 来源优先级：同目录同名 `.lrc` 文件（带时间戳，可逐行高亮）→ 音频
//! 标签歌词（ID3 USLT / Vorbis LYRICS，通常纯文本）。读取与解析都在
//! 后端——本地路径不进 webview（与封面同一安全口径，§35–§37）。
//!
//! 编码：本轮只支持 UTF-8/BOM（`fs::read_to_string` 对非 UTF-8 返回
//! Err → 视为无歌词；GBK 老文件按需后置，需引入 encoding_rs 走供应链
//! 审计）。

use std::path::Path;

use lofty::prelude::*;
use lofty::tag::ItemKey;
use serde::Serialize;

use crate::security_limits;

/// 单行歌词。`time_seconds = None` 表示无时间戳（纯文本歌词）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    pub time_seconds: Option<f64>,
    pub text: String,
}

/// 曲目歌词。`synced` = LRC 带时间戳（可逐行高亮/点击跳转）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub synced: bool,
    pub lines: Vec<LyricLine>,
}

/// 解析结果：None = 无歌词（无 .lrc 且标签无歌词）。
pub type MaybeLyrics = Option<Lyrics>;

/// 曲目歌词查询入口：track 文件路径 → `.lrc` 优先，回退标签歌词。
#[must_use]
pub fn lyrics_for_path(path: &str) -> MaybeLyrics {
    if let Some(lrc) = read_lrc(path) {
        return Some(lrc);
    }
    read_tag_lyrics(path)
}

/// 同目录同名 `.lrc`（stem 相同，扩展名小写匹配）。
fn lrc_path(path: &str) -> Option<std::path::PathBuf> {
    let p = Path::new(path);
    let stem = p.file_stem()?.to_string_lossy();
    let lrc = p.with_file_name(format!("{stem}.lrc"));
    lrc.is_file().then_some(lrc)
}

fn read_lrc(path: &str) -> Option<Lyrics> {
    let lrc = lrc_path(path)?;
    let content = std::fs::read(&lrc).ok()?;
    if content.len() > security_limits::SecurityLimits::MAX_LYRICS_BYTES {
        return None;
    }
    let text = String::from_utf8(content).ok()?;
    parse_lrc(&text)
}

/// LRC 文本 → 歌词行。全部时间戳无效时降级为纯文本（synced=false）。
/// 语法：`[mm:ss.xx]`/`[mm:ss]`（可多时间戳前缀）；`[offset:±ms]` 全局
/// 偏移；`[ti:/ar:/al:...]` 等元数据标签跳过；非法行跳过（D-016 同精神：
/// 单行损坏不致命）。
#[must_use]
pub fn parse_lrc(content: &str) -> Option<Lyrics> {
    let mut offset_ms: f64 = 0.0;
    let mut lines: Vec<LyricLine> = Vec::new();
    let mut saw_timestamp = false;

    for raw in content.lines() {
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with('[') {
            // 无括号的裸文本行：仅当已有时间戳行时视为其歌词（简单 LRC 变体）
            if let Some(last) = lines.last_mut() {
                if last.text.is_empty() && saw_timestamp {
                    last.text = line.to_string();
                }
            }
            continue;
        }
        // 逐个提取行首 [..] 标记
        let mut rest = line;
        let mut times: Vec<f64> = Vec::new();
        while let Some(end) = rest.find(']') {
            let tag = &rest[1..end];
            rest = &rest[end + 1..];
            if let Some(t) = parse_lrc_time(tag) {
                times.push((t - offset_ms).max(0.0));
            } else if let Some(v) = tag.strip_prefix("offset:") {
                // LRC offset 标签单位为毫秒；行内时间以秒运算——换算
                offset_ms = v.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
            }
            // 非 [mm:ss] 非 [offset:] 的标记（[ti:..] 等）→ 跳过
            if !rest.starts_with('[') {
                break;
            }
        }
        let text = rest.trim().to_string();
        if times.is_empty() {
            continue; // 纯元数据行或非法行
        }
        saw_timestamp = true;
        for t in times {
            lines.push(LyricLine {
                time_seconds: Some(t),
                text: text.clone(),
            });
        }
    }

    if !saw_timestamp {
        // 整段无有效时间戳 → 纯文本降级
        let text: Vec<LyricLine> = content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('['))
            .map(|l| LyricLine {
                time_seconds: None,
                text: l.to_string(),
            })
            .collect();
        return (!text.is_empty()).then_some(Lyrics {
            synced: false,
            lines: text,
        });
    }

    lines.retain(|l| !l.text.is_empty() || true); // 空行保留（间奏占位）
    lines.sort_by(|a, b| {
        a.time_seconds
            .partial_cmp(&b.time_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(Lyrics {
        synced: true,
        lines,
    })
}

/// `[mm:ss]` / `[mm:ss.xx]` / `[mm:ss.xxx]` → 秒。
fn parse_lrc_time(tag: &str) -> Option<f64> {
    let (mm, rest) = tag.split_once(':')?;
    let minutes: f64 = mm.trim().parse().ok()?;
    let (ss, fraction) = match rest.split_once('.') {
        Some((s, f)) => (s, f),
        None => (rest, ""),
    };
    let seconds: f64 = ss.trim().parse().ok()?;
    let frac = match fraction.len() {
        0 => 0.0,
        1..=3 => {
            let f = fraction.parse::<f64>().ok()?;
            f / 10f64.powi(fraction.len() as i32)
        }
        _ => return None,
    };
    if !(0.0..=5999.0).contains(&minutes) || !(0.0..=59.0).contains(&seconds) {
        return None;
    }
    Some(minutes * 60.0 + seconds + frac)
}

/// 音频标签歌词（ID3 USLT / Vorbis LYRICS → lofty ItemKey::Lyrics）。
fn read_tag_lyrics(path: &str) -> MaybeLyrics {
    let Ok(tagged) = lofty::read_from_path(path) else {
        return None;
    };
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    let text = tag.get_string(&ItemKey::Lyrics)?;
    if text.len() > security_limits::SecurityLimits::MAX_LYRICS_BYTES {
        return None;
    }
    // 标签歌词通常无时间戳；若内容本身是 LRC 格式则按 LRC 解析。
    parse_lrc(text).or_else(|| {
        let lines: Vec<LyricLine> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| LyricLine {
                time_seconds: None,
                text: l.to_string(),
            })
            .collect();
        (!lines.is_empty()).then_some(Lyrics {
            synced: false,
            lines,
        })
    })
}

#[cfg(test)]
mod lyrics_tests {
    use super::*;

    #[test]
    fn lrc_timestamps_and_sorting() {
        let l = parse_lrc("[ti:test]\n[00:03.00]第三行\n[00:01.5]第二行\n[00:00]第一行").unwrap();
        assert!(l.synced);
        assert_eq!(l.lines.len(), 3);
        assert_eq!(l.lines[0].text, "第一行");
        assert_eq!(l.lines[0].time_seconds, Some(0.0));
        assert_eq!(l.lines[1].time_seconds, Some(1.5));
        assert_eq!(l.lines[2].time_seconds, Some(3.0));
    }

    #[test]
    fn lrc_multi_timestamp_and_offset() {
        let l = parse_lrc("[offset:500]\n[00:02.00][00:10.00]重复段").unwrap();
        assert_eq!(l.lines.len(), 2);
        // offset 500ms：2.00-0.5=1.5
        assert_eq!(l.lines[0].time_seconds, Some(1.5));
        assert_eq!(l.lines[1].time_seconds, Some(9.5));
    }

    #[test]
    fn lrc_no_timestamp_degrades_to_plain() {
        let l = parse_lrc("[ti:纯文本]\n第一段\n第二段").unwrap();
        assert!(!l.synced);
        assert_eq!(l.lines.len(), 2);
        assert_eq!(l.lines[0].time_seconds, None);
    }

    #[test]
    fn lrc_garbage_yields_none() {
        assert!(parse_lrc("[ti:x]\n[ar:y]").is_none());
        assert!(parse_lrc("").is_none());
    }

    #[test]
    fn tag_lyrics_falls_back_to_plain_lines() {
        let l = parse_lrc("第一行\n第二行").unwrap();
        assert!(!l.synced);
        assert_eq!(l.lines.len(), 2);
    }

    #[test]
    fn tag_lyrics_content_that_is_lrc_parses_synced() {
        let l = parse_lrc("[00:01.0]词").unwrap();
        assert!(l.synced);
    }
}
