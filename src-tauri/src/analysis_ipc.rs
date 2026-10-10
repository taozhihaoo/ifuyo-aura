//! M8 分析 IPC（§233/§354）：M6 可获取 analyzed/not-analyzed/failed
//! 状态与 summary；不实现大型 analysis UI（§114/§115）。
//! 安全（§240）：TrackId 参数化；路径不经前端（查 M3 库内路径）。

use aura_analysis::domain::AnalysisStatus;
use aura_analysis::{AnalysisVisualSummary, TrackAnalysisSummary};
use rusqlite::OptionalExtension;

use crate::ipc::{from_err, AppError, CmdResult};

fn store_err(e: aura_analysis::store::StoreError) -> AppError {
    AppError::new("ANALYSIS_DB", e.to_string(), true)
}

/// §233/§117：单曲分析 summary（None = 尚未分析）。
#[tauri::command]
pub fn track_analysis_summary(
    state: tauri::State<'_, crate::ipc::AppState>,
    track_id: i64,
) -> CmdResult<Option<TrackAnalysisSummary>> {
    let service = state
        .library
        .service
        .lock()
        .map_err(|_| from_err("ANALYSIS_DB", "lock", true))?;
    let conn = service.database().connection();
    let exists: Option<i64> = conn
        .query_row(
            "SELECT id FROM tracks WHERE id = ?1",
            rusqlite::params![track_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| from_err("ANALYSIS_DB", e, true))?;
    if exists.is_none() {
        return Err(from_err("TRACK_UNAVAILABLE", "track not in library", false));
    }
    let row =
        aura_analysis::store::AnalysisStore::load_by_track(conn, track_id).map_err(store_err)?;
    Ok(row.map(|a| TrackAnalysisSummary::from(&a)))
}

/// §229/§232：M7 视觉注入合同（bpm/energy/brightness/loudness 最小集；
/// 未分析 → 全 None 缺省）。
#[tauri::command]
pub fn analysis_visual_summary(
    state: tauri::State<'_, crate::ipc::AppState>,
    track_id: i64,
) -> CmdResult<AnalysisVisualSummary> {
    let service = state
        .library
        .service
        .lock()
        .map_err(|_| from_err("ANALYSIS_DB", "lock", true))?;
    let conn = service.database().connection();
    let row =
        aura_analysis::store::AnalysisStore::load_by_track(conn, track_id).map_err(store_err)?;
    Ok(row
        .map(|a| AnalysisVisualSummary::from_analysis(&a))
        .unwrap_or_else(AnalysisVisualSummary::none))
}

/// §233：分析状态（analyzed / not_analyzed / failed / unsupported）。
#[tauri::command]
pub fn track_analysis_state(
    state: tauri::State<'_, crate::ipc::AppState>,
    track_id: i64,
) -> CmdResult<String> {
    let service = state
        .library
        .service
        .lock()
        .map_err(|_| from_err("ANALYSIS_DB", "lock", true))?;
    let conn = service.database().connection();
    let row =
        aura_analysis::store::AnalysisStore::load_by_track(conn, track_id).map_err(store_err)?;
    Ok(match row {
        None => "not_analyzed".to_string(),
        Some(a) => match a.status {
            AnalysisStatus::Completed => "analyzed".to_string(),
            AnalysisStatus::Failed => "failed".to_string(),
            AnalysisStatus::Unsupported => "unsupported".to_string(),
            other => format!("{other:?}").to_lowercase(),
        },
    })
}
