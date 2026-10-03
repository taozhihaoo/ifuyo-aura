//! Typed IPC commands（§11–§15）：DTO + 命令集 + 事件。
//! 所有命令返回 `Result<T, AppError>`（§15：结构化错误，非 Rust debug 串）。

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Wry};

use crate::audio::{self, AudioHandle, Control, PlayerSnapshot};
use crate::library::{AppSettings, LibraryState, ScanProgressDto, TrackSummary};

/// 结构化应用错误（§15）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl AppError {
    fn new(code: &str, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

type CmdResult<T> = Result<T, AppError>;

fn from_err(code: &str, e: impl std::fmt::Display, recoverable: bool) -> AppError {
    AppError::new(code, e.to_string(), recoverable)
}

/// 应用共享状态（R = Tauri Runtime；生产 Wry / 冒烟 MockRuntime）。
pub struct AppState {
    pub audio: Arc<AudioHandle>,
    pub library: Arc<LibraryState>,
    pub app: AppHandle<Wry>,
    /// M4 HTTP 客户端（Open URL 用；allow_private = 用户显式输入即确认，
    /// D-070；scheme 白名单/凭据拒绝/封锁表始终生效）。
    pub http: Arc<aura_sources::http::HttpClient>,
    pub blocklist: Arc<aura_sources::platform_block::PlatformBlocklist>,
}

/// 应用基础信息（M0 契约保留）。
#[tauri::command]
pub fn app_info() -> CmdResult<serde_json::Value> {
    let brand: serde_json::Value = serde_json::from_str(include_str!("../../brand.json"))
        .map_err(|e| AppError::new("INTERNAL", e.to_string(), false))?;
    Ok(serde_json::json!({
        "name": brand["name"],
        "version": brand["version"],
        "slogan": brand["slogan"],
        "phase": "M6",
    }))
}

/// §11 PlayerSnapshot。
#[tauri::command]
pub fn player_snapshot(state: State<'_, AppState>) -> CmdResult<PlayerSnapshot> {
    Ok(state.audio.snapshot())
}

/// 实时位置（秒；UI rAF 插值基准，§14）。
#[tauri::command]
pub fn player_position(state: State<'_, AppState>) -> CmdResult<f64> {
    Ok(state.audio.position_seconds())
}

/// 播放。
#[tauri::command]
pub fn play(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .audio
        .post(aura_core::AudioCommand::Play)
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))
}

/// 暂停。
#[tauri::command]
pub fn pause(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .audio
        .post(aura_core::AudioCommand::Pause)
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))
}

/// 停止。
#[tauri::command]
pub fn stop(state: State<'_, AppState>) -> CmdResult<()> {
    state
        .audio
        .post(aura_core::AudioCommand::Stop)
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))
}

/// seek（秒 → 帧；seekable 由快照驱动，§40）。
#[tauri::command]
pub fn seek(state: State<'_, AppState>, position_seconds: f64) -> CmdResult<()> {
    let frame = (position_seconds.max(0.0) * 48_000.0) as u64;
    state
        .audio
        .post(aura_core::AudioCommand::Seek { frame })
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))
}

/// §43：真实 SetVolume（M2 smoothing）。
#[tauri::command]
pub fn set_volume(state: State<'_, AppState>, volume: f32) -> CmdResult<()> {
    state
        .audio
        .post(aura_core::AudioCommand::SetVolume { volume })
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))
}

/// M4 SourceError → 结构化 AppError（§189 错误映射链的 Rust 侧起点）。
fn from_source_err(e: aura_sources::error::SourceError) -> AppError {
    use aura_sources::error::SourceError as E;
    let (code, recoverable) = match &e {
        E::UnsupportedInput { .. } | E::UnsupportedCodec { .. } | E::UnsupportedPlatform { .. } => {
            ("UNSUPPORTED_SOURCE", false)
        }
        E::InvalidUrl { .. } | E::PrivateNetworkBlocked { .. } => ("SOURCE_BLOCKED", false),
        E::Timeout => ("SOURCE_TIMEOUT", true),
        E::Cancelled => ("CANCELLED", true),
        E::ProtectedStream => ("UNSUPPORTED_SOURCE", false),
        E::Io { .. } => ("SOURCE_UNAVAILABLE", true),
        _ => ("SOURCE_UNAVAILABLE", true),
    };
    AppError::new(code, e.to_string(), recoverable)
}

/// §93：双击曲目 → application layer 解析 Track → Source → Audio Core。
#[tauri::command]
pub fn play_track(state: State<'_, AppState>, track_id: i64) -> CmdResult<()> {
    let path = state
        .library
        .track_path(track_id)
        .map_err(|e| from_err("TRACK_UNAVAILABLE", e, true))?;
    play_input(state, path).map(|_| ())
}

/// §275/§276/§194：Open File / Open URL 统一入口。
/// 全部经过 M4 Source Resolver（scheme 白名单 → 凭据拒绝 → 封锁表 →
/// 私网策略 → 探测），frontend 不自行 fetch。本地路径由 resolver 的
/// Local Source 分支处理（存在性/扩展名校验在 Rust 侧，§192）。
#[tauri::command]
pub fn play_input(state: State<'_, AppState>, input: String) -> CmdResult<String> {
    let resolved = aura_sources::resolver::resolve_input(&input, &state.http, &state.blocklist)
        .map_err(from_source_err)?;
    use aura_sources::model::SourceKind;
    let title = resolved
        .meta
        .track
        .title
        .clone()
        .or(resolved.meta.stream.icy_name.clone());
    let display: String = match resolved.kind {
        SourceKind::Local => {
            let path = resolved
                .streams
                .first()
                .map(|s| s.url.clone())
                .unwrap_or_else(|| input.clone());
            let (decoder, duration_frames) = audio::open_decoder(std::path::Path::new(&path))
                .map_err(|e| AppError::new("UNSUPPORTED_SOURCE", e.to_string(), true))?;
            state
                .audio
                .send(Control::LoadDecoder {
                    decoder,
                    duration_frames,
                    title,
                    display: path.clone(),
                })
                .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))?;
            path
        }
        SourceKind::InternetRadio => {
            let (net_stream, _icy_events) =
                aura_sources::radio::open_resolved(&state.http, &resolved)
                    .map_err(from_source_err)?
                    .into_parts();
            let url = resolved
                .streams
                .first()
                .map(|s| s.url.clone())
                .unwrap_or_else(|| input.clone());
            let decoder =
                aura_core::decoder::streaming::StreamingDecoder::open(Box::new(net_stream), None)
                    .map_err(|e| AppError::new("UNSUPPORTED_SOURCE", e.to_string(), true))?;
            state
                .audio
                .send(Control::LoadDecoder {
                    decoder: Box::new(decoder),
                    duration_frames: None, // Live：无确定时长（§57）
                    title,
                    display: url.clone(),
                })
                .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))?;
            url
        }
        _ => {
            // DirectHttp（Finite/UnknownDuration）：NetworkAudioStream → decoder。
            let stream = resolved
                .streams
                .first()
                .cloned()
                .ok_or_else(|| AppError::new("SOURCE_UNAVAILABLE", "no stream", true))?;
            let url: url::Url =
                url::Url::parse(&stream.url).map_err(|e| from_err("SOURCE_BLOCKED", e, false))?;
            let resp = state.http.get_stream(&url).map_err(from_source_err)?;
            let seekable = stream.seekable == aura_sources::model::SeekCapability::Seekable;
            let capacity = aura_sources::network_stream::buffer_capacity(
                stream.stream_type,
                stream.bitrate_bps,
            );
            let nas = aura_sources::network_stream::NetworkAudioStream::start(
                &state.http,
                url,
                resp,
                capacity,
                seekable,
                stream.byte_size,
                state.http.config().stream_idle_timeout,
            );
            // codec hint：URL 扩展名（最终以 decoder probing 为准，§77）。
            let hint = stream
                .url
                .split(['?', '#'])
                .next()
                .and_then(|p| p.rsplit('.').next())
                .map(str::to_ascii_lowercase)
                .filter(|ext| aura_library::scanner::SUPPORTED_EXTENSIONS.contains(&ext.as_str()));
            let decoder = aura_core::decoder::streaming::StreamingDecoder::open(
                Box::new(nas),
                hint.as_deref(),
            )
            .map_err(|e| AppError::new("UNSUPPORTED_SOURCE", e.to_string(), true))?;
            let duration_frames = aura_core::decoder::Decoder::duration_frames(&decoder);
            state
                .audio
                .send(Control::LoadDecoder {
                    decoder: Box::new(decoder),
                    duration_frames,
                    title,
                    display: stream.url.clone(),
                })
                .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))?;
            stream.url
        }
    };
    // 装载成功即播放（与此前 play_track 行为一致）。
    state
        .audio
        .post(aura_core::AudioCommand::Play)
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))?;
    Ok(display)
}

/// §45：crossfade 0–12s。
#[tauri::command]
pub fn set_crossfade(state: State<'_, AppState>, seconds: f64, persist: bool) -> CmdResult<()> {
    let s = seconds.clamp(0.0, 12.0);
    state
        .audio
        .send(Control::SetCrossfade(s))
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))?;
    if persist {
        let lib = &state.library;
        let mut settings = lib.settings.lock().expect("settings");
        settings.crossfade_seconds = s;
        lib.save_settings(&settings)
            .map_err(|e| from_err("SETTINGS_WRITE", e, true))?;
    }
    Ok(())
}

/// §44：loudness 开关（UI 不算 LUFS）。
#[tauri::command]
pub fn set_loudness(state: State<'_, AppState>, enabled: bool, persist: bool) -> CmdResult<()> {
    state
        .audio
        .send(Control::SetLoudness(enabled))
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))?;
    if persist {
        let lib = &state.library;
        let mut settings = lib.settings.lock().expect("settings");
        settings.loudness_enabled = enabled;
        lib.save_settings(&settings)
            .map_err(|e| from_err("SETTINGS_WRITE", e, true))?;
    }
    Ok(())
}

/// §46：EQ preset（10 band DSP 在 M2）。
#[tauri::command]
pub fn set_eq_preset(state: State<'_, AppState>, preset: usize, persist: bool) -> CmdResult<()> {
    state
        .audio
        .send(Control::SetEqPreset(preset))
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))?;
    if persist {
        let lib = &state.library;
        let mut settings = lib.settings.lock().expect("settings");
        settings.eq_preset = preset;
        lib.save_settings(&settings)
            .map_err(|e| from_err("SETTINGS_WRITE", e, true))?;
    }
    Ok(())
}

fn summarize(state: &AppState, tracks: Vec<aura_library::types::Track>) -> Vec<TrackSummary> {
    let service = state.library.service.lock().ok();
    let service_guard = service.as_ref();
    tracks
        .into_iter()
        .map(|t| {
            let path: Option<String> = service_guard.and_then(|s| {
                s.database()
                    .connection()
                    .query_row(
                        "SELECT path FROM track_paths WHERE track_id = ?1 AND is_active = 1 ORDER BY id LIMIT 1",
                        rusqlite::params![t.id.0],
                        |r| r.get(0),
                    )
                    .ok()
            });
            TrackSummary {
                id: t.id.0,
                title: t.title,
                artist: t.artist,
                album: t.album,
                duration_seconds: t
                    .duration_frames
                    .zip(t.sample_rate)
                    .map(|(f, r)| f as f64 / f64::from(r.max(1))),
                year: t.year,
                path: path.unwrap_or_default(),
                has_artwork: t.artwork_id.is_some(),
            }
        })
        .collect()
}

/// §84/§85：分页列表（sort 直接映射 M3）。
#[tauri::command]
pub fn library_list(
    state: State<'_, AppState>,
    sort: String,
    limit: i64,
    offset: i64,
) -> CmdResult<Vec<TrackSummary>> {
    let service = state
        .library
        .service
        .lock()
        .map_err(|_| from_err("LIBRARY_DB_UNAVAILABLE", "lock", true))?;
    let sort_key = match sort.as_str() {
        "artist" => aura_library::query::SortKey::Artist,
        "album" => aura_library::query::SortKey::Album,
        "dateAdded" => aura_library::query::SortKey::DateAdded,
        "duration" => aura_library::query::SortKey::Duration,
        "year" => aura_library::query::SortKey::Year,
        _ => aura_library::query::SortKey::Title,
    };
    let tracks = service
        .list_tracks(
            sort_key,
            aura_library::query::Page::new(limit.clamp(1, 500), offset.max(0)),
        )
        .map_err(|e| from_err("LIBRARY_DB_UNAVAILABLE", e, true))?;
    drop(service);
    Ok(summarize(&state, tracks))
}

/// §78：真实 FTS5 搜索。
#[tauri::command]
pub fn library_search(
    state: State<'_, AppState>,
    query: String,
    limit: i64,
    offset: i64,
) -> CmdResult<Vec<TrackSummary>> {
    let service = state
        .library
        .service
        .lock()
        .map_err(|_| from_err("LIBRARY_DB_UNAVAILABLE", "lock", true))?;
    let hits = service
        .search_tracks(
            &query,
            aura_library::query::Page::new(limit.clamp(1, 500), offset.max(0)),
        )
        .map_err(|e| from_err("LIBRARY_DB_UNAVAILABLE", e, true))?;
    drop(service);
    Ok(summarize(&state, hits))
}

/// §35/§36/§37：按 TrackId 读 M3 artwork → data URL（Tauri-safe 通道，
/// 不把任意本地路径交给 webview）。`None` = 无封面（missing）。
#[tauri::command]
pub fn track_artwork(state: State<'_, AppState>, track_id: i64) -> CmdResult<Option<String>> {
    let service = state
        .library
        .service
        .lock()
        .map_err(|e| from_err("LIBRARY_DB_UNAVAILABLE", e, true))?;
    let track = service
        .get_track(aura_library::types::TrackId(track_id))
        .map_err(|e| from_err("LIBRARY_DB_UNAVAILABLE", e, true))?;
    let Some(aid) = track.and_then(|t| t.artwork_id) else {
        return Ok(None);
    };
    let art = service
        .get_artwork(aid)
        .map_err(|e| from_err("LIBRARY_DB_UNAVAILABLE", e, true))?;
    let Some((mime, bytes)) = art else {
        return Ok(None); // 引用悬空 = missing（§36）
    };
    use base64::Engine as _;
    Ok(Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )))
}

/// §284/§285：库统计（M3 真实计数；Map 展示用，不伪造数字）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub tracks: i64,
    pub artists: i64,
    pub albums: i64,
}

#[tauri::command]
pub fn library_stats(state: State<'_, AppState>) -> CmdResult<LibraryStats> {
    let service = state
        .library
        .service
        .lock()
        .map_err(|_| from_err("LIBRARY_DB_UNAVAILABLE", "lock", true))?;
    let conn = service.database().connection();
    let one = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0) };
    Ok(LibraryStats {
        tracks: one("SELECT COUNT(*) FROM tracks"),
        artists: one("SELECT COUNT(DISTINCT artist) FROM tracks"),
        albums: one("SELECT COUNT(DISTINCT album) FROM tracks"),
    })
}

/// path → TrackId（Now Playing artwork 绑定用）。
#[tauri::command]
pub fn track_id_by_path(state: State<'_, AppState>, path: String) -> CmdResult<Option<i64>> {
    let service = state
        .library
        .service
        .lock()
        .map_err(|e| from_err("LIBRARY_DB_UNAVAILABLE", e, true))?;
    let id: Option<i64> = service
        .database()
        .connection()
        .query_row(
            "SELECT track_id FROM track_paths WHERE path = ?1 AND is_active = 1 LIMIT 1",
            rusqlite::params![path],
            |r| r.get(0),
        )
        .ok();
    Ok(id)
}

/// §19/§78：Win11 Mica 材质开关（失败自动保持 solid fallback，§20）。
#[tauri::command]
pub fn set_mica(
    app: AppHandle<Wry>,
    state: State<'_, AppState>,
    enabled: bool,
    persist: bool,
) -> CmdResult<()> {
    #[cfg(target_os = "windows")]
    {
        let Some(win) = app.get_webview_window("main") else {
            return Err(from_err("INTERNAL", "main window missing", false));
        };
        let result = if enabled {
            window_vibrancy::apply_mica(&win, None).map_err(|e| e.to_string())
        } else {
            window_vibrancy::clear_mica(&win).map_err(|e| e.to_string())
        };
        if let Err(reason) = result {
            // 非 Win11 / 合成器不支持：静默保持 solid fallback（§20）。
            let _ = reason;
        }
    }
    if persist {
        let lib = &state.library;
        let mut settings = lib.settings.lock().expect("settings");
        settings.mica_enabled = enabled;
        lib.save_settings(&settings)
            .map_err(|e| from_err("SETTINGS_WRITE", e, true))?;
    }
    Ok(())
}

/// §93：播放管线路径解析。
#[tauri::command]
pub fn track_path(state: State<'_, AppState>, track_id: i64) -> CmdResult<String> {
    state
        .library
        .track_path(track_id)
        .map_err(|e| from_err("TRACK_UNAVAILABLE", e, true))
}

/// §87/§88：Add Music Folder → 后台扫描 + scan-progress 事件。
#[tauri::command]
pub fn scan_folder(app: AppHandle<Wry>, state: State<'_, AppState>, path: String) -> CmdResult<()> {
    if state
        .library
        .scanning
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        return Err(from_err("SCAN_RUNNING", "scan already running", true));
    }
    let lib = Arc::clone(&state.library);
    let app2 = app.clone();
    std::thread::spawn(move || {
        let _ = lib.scan_root_blocking(std::path::Path::new(&path), |p: ScanProgressDto| {
            let _ = app2.emit("scan-progress", &p);
        });
        let _ = app2.emit(
            "scan-progress",
            ScanProgressDto {
                discovered: 0,
                processed: 0,
                imported: 0,
                failed: 0,
                current_path: String::new(),
                done: true,
            },
        );
        let _ = app2.emit("library-changed", ());
    });
    Ok(())
}

/// §89：取消扫描（真实 M3 CancelToken）。
#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) -> CmdResult<()> {
    state.library.cancel.lock().expect("cancel lock").cancel();
    Ok(())
}

/// §114：读设置。
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> CmdResult<AppSettings> {
    Ok(state.library.settings.lock().expect("settings").clone())
}

/// §114：写设置。
#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: AppSettings) -> CmdResult<()> {
    state
        .library
        .save_settings(&settings)
        .map_err(|e| from_err("SETTINGS_WRITE", e, true))
}

/// tests 引用保活。
#[allow(unused_imports)]
use crate::audio::Control as _ControlAlias;
#[allow(dead_code)]
fn _touch(h: &AudioHandle) -> PlayerSnapshot {
    h.snapshot()
}
