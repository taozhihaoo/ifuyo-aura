//! Typed IPC commands（§11–§15）：DTO + 命令集 + 事件。
//! 所有命令返回 `Result<T, AppError>`（§15：结构化错误，非 Rust debug 串）。

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::audio::{AudioHandle, Control, PlayerSnapshot};
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

/// 应用共享状态。
pub struct AppState {
    pub audio: Arc<AudioHandle>,
    pub library: Arc<LibraryState>,
    pub app: AppHandle,
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

/// §93：双击曲目 → application layer 解析 Track → Source → Audio Core。
#[tauri::command]
pub fn play_track(state: State<'_, AppState>, track_id: i64) -> CmdResult<()> {
    let path = state
        .library
        .track_path(track_id)
        .map_err(|e| from_err("TRACK_UNAVAILABLE", e, true))?;
    state
        .audio
        .send(Control::Load {
            path: path.clone().into(),
            title: None,
        })
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))?;
    state
        .audio
        .post(aura_core::AudioCommand::Play)
        .map_err(|e| from_err("AUDIO_UNAVAILABLE", e, true))
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
    tracks
        .into_iter()
        .filter_map(|t| {
            let path: Option<String> = service.as_ref().and_then(|s| {
                s.database()
                    .connection()
                    .query_row(
                        "SELECT path FROM track_paths WHERE track_id = ?1 AND is_active = 1 ORDER BY id LIMIT 1",
                        rusqlite::params![t.id.0],
                        |r| r.get(0),
                    )
                    .ok()
            });
            Some(TrackSummary {
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
            })
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
pub fn scan_folder(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<()> {
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
