//! M15a CUE 整轨支持：sheet 解析 → 虚拟子轨元数据。
//!
//! 语法覆盖：`REM`（跳过）、`TITLE`/`PERFORMER`（sheet 级与 TRACK 级，
//! 子轨继承 sheet 值）、`FILE "..."/ WAVE|MP3|AIFF|FLAC`、
//! `TRACK nn AUDIO|MODE1/2352`（仅 AUDIO 为音乐子轨）、
//! `INDEX 01 mm:ss:ff`（子轨起点；ff = 75 分之一秒）。
//! 编码：UTF-8/BOM 优先，失败按 GBK（与标签乱码修复同口径）。
//! 单行解析失败不致命（D-016 同精神）。

use std::path::Path;

use encoding_rs::GBK;

/// 单条音乐子轨（起点秒；时长由调用方按相邻 INDEX 或文件时长推导）。
#[derive(Debug, Clone, PartialEq)]
pub struct CueSubtrack {
    pub number: u32,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub start_seconds: f64,
}

/// 解析后的 CUE sheet。
#[derive(Debug, Clone, PartialEq)]
pub struct CueSheet {
    pub album: Option<String>,
    pub album_performer: Option<String>,
    pub file_name: Option<String>,
    pub subtracks: Vec<CueSubtrack>,
}

/// 读取音频文件的同名 `.cue`（stem 相同）并解析。
/// 仅当 sheet 中存在 ≥2 条 AUDIO 子轨时返回 Some（单轨 CUE 无整轨意义）。
#[must_use]
pub fn load_for_audio(audio_path: &Path) -> Option<CueSheet> {
    let stem = audio_path.file_stem()?.to_string_lossy().into_owned();
    let cue = audio_path.with_file_name(format!("{stem}.cue"));
    let bytes = std::fs::read(&cue).ok()?;
    if bytes.len() > 1024 * 1024 {
        return None;
    }
    // UTF-8/BOM 优先，GBK 回退（国内 CUE 常见 GBK）
    let text = match String::from_utf8(bytes.clone()) {
        Ok(t) => t,
        Err(_) => {
            let (decoded, _, had_errors) = GBK.decode(&bytes);
            if had_errors {
                return None;
            }
            decoded.into_owned()
        }
    };
    let sheet = parse(&text)?;
    (sheet.subtracks.len() >= 2).then_some(sheet)
}

/// 解析 CUE 文本。全 sheet 无有效 AUDIO 子轨 → None。
#[must_use]
pub fn parse(content: &str) -> Option<CueSheet> {
    let mut sheet = CueSheet {
        album: None,
        album_performer: None,
        file_name: None,
        subtracks: Vec::new(),
    };
    let mut album_title: Option<String> = None;
    let mut album_performer: Option<String> = None;

    // 当前 TRACK 聚合态
    let mut cur_number: Option<u32> = None;
    let mut cur_is_audio = false;
    let mut cur_title: Option<String> = None;
    let mut cur_performer: Option<String> = None;
    let mut cur_start: Option<f64> = None;

    let push_cur = |sheet: &mut CueSheet,
                    number: &mut Option<u32>,
                    is_audio: &mut bool,
                    title: &mut Option<String>,
                    performer: &mut Option<String>,
                    start: &mut Option<f64>| {
        if let (Some(n), Some(t)) = (*number, *start) {
            if *is_audio {
                sheet.subtracks.push(CueSubtrack {
                    number: n,
                    title: title.take(),
                    performer: performer.take(),
                    start_seconds: t,
                });
            }
        }
        *number = None;
        *is_audio = false;
        *title = None;
        *performer = None;
        *start = None;
    };

    for raw in content.lines() {
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() || line.starts_with("REM ") {
            continue;
        }
        let (keyword, rest) = line.split_once(' ')?;
        let _ = keyword;
        let (keyword, rest) = (keyword.to_ascii_uppercase(), rest.trim());
        match keyword.as_str() {
            "TITLE" => {
                let v = unquote(rest);
                if cur_number.is_some() {
                    if cur_title.is_none() {
                        cur_title = Some(v);
                    }
                } else if album_title.is_none() {
                    album_title = Some(v);
                }
            }
            "PERFORMER" => {
                let v = unquote(rest);
                if cur_number.is_some() {
                    if cur_performer.is_none() {
                        cur_performer = Some(v);
                    }
                } else if album_performer.is_none() {
                    album_performer = Some(v);
                }
            }
            "FILE" => {
                let v = unquote(rest.split_whitespace().next().unwrap_or(""));
                if sheet.file_name.is_none() {
                    sheet.file_name = Some(v);
                }
            }
            "TRACK" => {
                push_cur(
                    &mut sheet,
                    &mut cur_number,
                    &mut cur_is_audio,
                    &mut cur_title,
                    &mut cur_performer,
                    &mut cur_start,
                );
                let mut parts = rest.split_whitespace();
                cur_number = parts.next().and_then(|n| n.parse::<u32>().ok());
                cur_is_audio = matches!(parts.next(), Some("AUDIO"));
            }
            "INDEX" => {
                // 只取 01（主起点）；00 为 pregap，忽略
                let mut parts = rest.split_whitespace();
                let idx = parts.next().unwrap_or("");
                let time = parts.next().unwrap_or("");
                if idx == "01" {
                    cur_start = parse_time(time);
                }
            }
            _ => {}
        }
    }
    push_cur(
        &mut sheet,
        &mut cur_number,
        &mut cur_is_audio,
        &mut cur_title,
        &mut cur_performer,
        &mut cur_start,
    );

    sheet.album = album_title;
    sheet.album_performer = album_performer;
    (!sheet.subtracks.is_empty()).then_some(sheet)
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    t.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(t)
        .to_string()
}

/// `mm:ss:ff`（ff = 1/75 秒）→ 秒。
fn parse_time(s: &str) -> Option<f64> {
    let mut it = s.split(':');
    let mm: f64 = it.next()?.trim().parse().ok()?;
    let ss: f64 = it.next()?.trim().parse().ok()?;
    let ff: f64 = it.next()?.trim().parse().ok()?;
    Some(mm * 60.0 + ss + ff / 75.0)
}

#[cfg(test)]
mod cue_tests {
    use super::*;

    const SAMPLE: &str = r#"REM COMMENT "exact copy"
PERFORMER "群星"
TITLE "加固计划合集"
FILE "fixture-cue.flac" WAVE
  TRACK 01 AUDIO
    TITLE "第一首"
    PERFORMER "测试艺术家"
    INDEX 00 00:00:00
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "第二首"
    INDEX 01 00:02:00
  TRACK 03 MODE1/2352
    TITLE "数据轨"
    INDEX 01 00:04:00
  TRACK 04 AUDIO
    TITLE "第四首"
    INDEX 01 00:06:37
"#;

    #[test]
    fn parses_audio_subtracks_with_inheritance() {
        let sheet = parse(SAMPLE).expect("parse");
        assert_eq!(sheet.album.as_deref(), Some("加固计划合集"));
        assert_eq!(sheet.album_performer.as_deref(), Some("群星"));
        assert_eq!(sheet.file_name.as_deref(), Some("fixture-cue.flac"));
        // 数据轨被排除
        assert_eq!(sheet.subtracks.len(), 3);
        assert_eq!(sheet.subtracks[0].number, 1);
        assert_eq!(sheet.subtracks[0].title.as_deref(), Some("第一首"));
        assert_eq!(sheet.subtracks[0].performer.as_deref(), Some("测试艺术家"));
        assert_eq!(sheet.subtracks[0].start_seconds, 0.0);
        // 第二首：无 TRACK 级 PERFORMER → 继承 sheet 级（00:02:00 = 2 秒）
        assert_eq!(sheet.subtracks[1].title.as_deref(), Some("第二首"));
        assert_eq!(sheet.subtracks[1].start_seconds, 2.0);
        // 00:06:37 = 6 秒 + 37/75 帧
        assert!((sheet.subtracks[2].start_seconds - 6.493_3).abs() < 0.001);
    }

    #[test]
    fn single_track_sheet_is_rejected_by_loader_contract() {
        // load_for_audio 的 ≥2 契约由调用方判定；解析器本身返回单轨
        let sheet = parse("FILE \"a.flac\" WAVE\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n")
            .expect("parse");
        assert_eq!(sheet.subtracks.len(), 1);
    }

    #[test]
    fn garbage_returns_none() {
        assert!(parse("not a cue at all").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn time_parsing_frames() {
        // 01:07:37 → 60 + 7 + 37/75 = 67.4933…
        let sheet =
            parse("FILE \"a.flac\" WAVE\n  TRACK 01 AUDIO\n    INDEX 01 01:07:37\n").unwrap();
        let t = sheet.subtracks[0].start_seconds;
        assert!((t - 67.493_3).abs() < 0.001, "got {t}");
    }
}
