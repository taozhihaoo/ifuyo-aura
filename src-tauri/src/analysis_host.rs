//! 分析宿主（M8 运行时 → 应用层）。
//!
//! 后台单 worker 顺序分析未分析曲目，结果经 `AnalysisStore` 写入同一
//! `library.db`，进度走 `analysis-progress` 事件。接线前
//! `AnalysisService::analyze_tracks` 只被自身单测调用——Music Map 与响度
//! 归一在真实应用里没有任何数据源。
//!
//! 连接策略：worker 持有自己的 DB 连接（WAL + busy_timeout 允许与扫描/
//! 查询并发）；IPC 侧只读统计另开短连接，避免与 worker 抢同一把锁
//! （单曲分析耗时秒级，共享连接会让状态查询卡住）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use aura_analysis::domain::{AnalysisStatus, TrackAnalysis};
use aura_analysis::embedding::SpectralStatsProvider;
use aura_analysis::local_stream::LocalFileStream;
use aura_analysis::pipeline::{self, CancelToken};
use aura_analysis::runtime::run_job_isolated;
use aura_analysis::store::AnalysisStore;
use rusqlite::{Connection, OptionalExtension};
use tauri::{Emitter, State};

use crate::ipc::{from_err, CmdResult};

/// 进度 DTO（与 `ScanProgressDto` 同形态：只计数，不外泄路径）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisProgressDto {
    pub processed: u64,
    pub total: u64,
    pub failed: u64,
    pub running: bool,
}

/// 状态 DTO：进度 + 曲库分析覆盖度。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisStatusDto {
    pub progress: AnalysisProgressDto,
    pub analyzed: u64,
    pub pending: u64,
}

pub struct AnalysisHost {
    db_path: PathBuf,
    tx: Sender<Vec<i64>>,
    cancel: Arc<Mutex<CancelToken>>,
    progress: Arc<Mutex<AnalysisProgressDto>>,
    busy: Arc<AtomicBool>,
}

impl AnalysisHost {
    /// 启动 worker 线程。DB 打开失败 → 线程立即退出并降级为"分析不可用"
    /// （状态面如实显示 pending，不假装在跑）。
    pub fn spawn(app: &tauri::AppHandle, db_path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel::<Vec<i64>>();
        let cancel = Arc::new(Mutex::new(CancelToken::new()));
        let progress = Arc::new(Mutex::new(AnalysisProgressDto {
            processed: 0,
            total: 0,
            failed: 0,
            running: false,
        }));
        let busy = Arc::new(AtomicBool::new(false));
        let thread = {
            let app = app.clone();
            let db_path = db_path.clone();
            let cancel = Arc::clone(&cancel);
            let progress = Arc::clone(&progress);
            let busy = Arc::clone(&busy);
            std::thread::Builder::new()
                .name("aura-analysis".into())
                .spawn(move || worker_loop(rx, &app, &db_path, &cancel, &progress, &busy))
        };
        if thread.is_err() {
            eprintln!("analysis worker unavailable: thread spawn failed");
        }
        Self {
            db_path,
            tx,
            cancel,
            progress,
            busy,
        }
    }

    /// 入队全部未分析曲目，返回入队数。已有批次在跑 → 拒绝（不排队，
    /// 否则同一曲目会被分析两遍）。
    ///
    /// # Errors
    /// 只读连接打开/查询失败，或已有批次在跑。
    pub fn request_pending(&self) -> Result<u64, String> {
        // CAS 认领 busy：load-then-send 的窗口里两个并发请求会双双通过。
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("analysis already running".into());
        }
        // 认领之后任何早退路径都必须归还 busy，否则状态永远停在"在跑"。
        match self.plan_pending() {
            Ok(None) => {
                self.release();
                Ok(0)
            }
            Ok(Some(ids)) => {
                // 取消是"一次性"的：新一轮必须换上新令牌。
                *self.cancel.lock().expect("analysis cancel lock") = CancelToken::new();
                let queued = ids.len() as u64;
                match self.tx.send(ids) {
                    Ok(()) => Ok(queued),
                    Err(_) => {
                        self.release();
                        Err("analysis worker stopped".to_string())
                    }
                }
            }
            Err(e) => {
                self.release();
                Err(e)
            }
        }
    }

    fn plan_pending(&self) -> Result<Option<Vec<i64>>, String> {
        let conn = open_readonly(&self.db_path)?;
        let ids = pending_ids(&conn)?;
        Ok(if ids.is_empty() { None } else { Some(ids) })
    }

    fn release(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }

    /// 取消当前批次（活动曲目在检查点退出）。
    pub fn cancel(&self) {
        self.cancel.lock().expect("analysis cancel lock").cancel();
    }

    /// 状态快照（进度 + 覆盖度）。
    ///
    /// # Errors
    /// 只读连接打开/查询失败。
    pub fn status(&self) -> Result<AnalysisStatusDto, String> {
        let progress = self
            .progress
            .lock()
            .map(|p| p.clone())
            .unwrap_or(AnalysisProgressDto {
                processed: 0,
                total: 0,
                failed: 0,
                running: false,
            });
        let conn = open_readonly(&self.db_path)?;
        Ok(AnalysisStatusDto {
            progress,
            analyzed: count(&conn, ANALYZED_COUNT_SQL)?,
            pending: pending_count(&conn)?,
        })
    }
}

/// 未分析 = 活跃、有可用路径、且 `track_analysis` 无行。CUE 虚拟子轨排除
/// （其路径是合成的 `file#NN`，磁盘上不存在；整轨主行在子轨展开后被停用，
/// 因此整轨文件当前不参与分析——如实记为已知限制）。
///
/// 谓词只此一份：入队清单与待分析计数共用（两份谓词会漂移）。
const PENDING_WHERE: &str = "t.is_active = 1 AND t.cue_start_seconds IS NULL \
     AND (SELECT p.path FROM track_paths p WHERE p.track_id = t.id AND p.is_active = 1 \
           ORDER BY p.id LIMIT 1) IS NOT NULL \
     AND NOT EXISTS (SELECT 1 FROM track_analysis a WHERE a.track_id = t.id)";

const ANALYZED_COUNT_SQL: &str = "SELECT COUNT(*) FROM track_analysis WHERE status = 'completed'";

fn open_readonly(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

fn count(conn: &Connection, sql: &str) -> Result<u64, String> {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0))
        .map(|v| v.max(0) as u64)
        .map_err(|e| e.to_string())
}

fn pending_ids(conn: &Connection) -> Result<Vec<i64>, String> {
    let sql = format!("SELECT t.id FROM tracks t WHERE {PENDING_WHERE} ORDER BY t.id");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn pending_count(conn: &Connection) -> Result<u64, String> {
    let sql = format!("SELECT COUNT(*) FROM tracks t WHERE {PENDING_WHERE}");
    count(conn, &sql)
}

fn track_source(conn: &Connection, track_id: i64) -> Result<Option<(String, String)>, String> {
    let row = conn
        .query_row(
            "SELECT t.identity_key,
                    (SELECT p.path FROM track_paths p
                      WHERE p.track_id = t.id AND p.is_active = 1 ORDER BY p.id LIMIT 1)
             FROM tracks t WHERE t.id = ?1",
            [track_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(row.and_then(|(identity, path)| path.map(|p| (identity, p))))
}

fn worker_loop(
    rx: mpsc::Receiver<Vec<i64>>,
    app: &tauri::AppHandle,
    db_path: &Path,
    cancel: &Mutex<CancelToken>,
    progress: &Mutex<AnalysisProgressDto>,
    busy: &AtomicBool,
) {
    let db = match aura_library::db::Database::open(db_path) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("analysis worker offline: {e}");
            return;
        }
    };
    let conn = db.connection();
    while let Ok(ids) = rx.recv() {
        let token = cancel.lock().expect("analysis cancel lock").clone();
        busy.store(true, Ordering::SeqCst);
        let total = ids.len() as u64;
        update(progress, app, |p| {
            p.processed = 0;
            p.total = total;
            p.failed = 0;
            p.running = true;
        });
        for id in ids {
            if token.is_cancelled() {
                break;
            }
            let failed = analyze_one(conn, id, &token).is_err();
            let done = {
                let mut guard = progress.lock().expect("analysis progress lock");
                guard.processed += 1;
                if failed {
                    guard.failed += 1;
                }
                guard.processed
            };
            // 每 5 首或收尾时上抛一次，避免千首库刷出千个事件。
            if done % 5 == 0 || done == total {
                emit(progress, app);
            }
        }
        busy.store(false, Ordering::SeqCst);
        update(progress, app, |p| p.running = false);
    }
}

fn update(
    progress: &Mutex<AnalysisProgressDto>,
    app: &tauri::AppHandle,
    f: impl FnOnce(&mut AnalysisProgressDto),
) {
    if let Ok(mut p) = progress.lock() {
        f(&mut p);
    }
    emit(progress, app);
}

fn emit(progress: &Mutex<AnalysisProgressDto>, app: &tauri::AppHandle) {
    if let Ok(p) = progress.lock() {
        let _ = app.emit("analysis-progress", &*p);
    }
}

/// 单曲分析：解码 → 特征 → 落库。返回 Err = 本轮失败计数（不落库，
/// 下次请求会重试）。
fn analyze_one(conn: &Connection, track_id: i64, cancel: &CancelToken) -> Result<(), String> {
    let (identity, path) = track_source(conn, track_id)?.ok_or("track has no source path")?;
    let stream = LocalFileStream::open(Path::new(&path)).map_err(|e| e.to_string())?;
    let token = cancel.clone();
    let result = run_job_isolated(|| {
        pipeline::analyze_stream(
            Box::new(stream),
            track_id,
            &identity,
            &SpectralStatsProvider,
            &token,
            now_ms,
        )
    });
    match result {
        Ok(analysis) => persist(conn, &analysis),
        Err(e) => Err(e.to_string()),
    }
}

fn persist(conn: &Connection, analysis: &TrackAnalysis) -> Result<(), String> {
    if analysis.status == AnalysisStatus::Cancelled {
        return Ok(());
    }
    // §335/§336 同门：Completed 必须至少有一个可用特征，否则不覆盖既有结果。
    if analysis.status == AnalysisStatus::Completed && !analysis.is_valid_completed() {
        return Err("completed result has no available feature".into());
    }
    AnalysisStore::save(conn, analysis).map_err(|e| e.to_string())
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 分析状态：进度 + 曲库覆盖度（已分析/待分析）。
#[tauri::command]
pub fn analysis_status(state: State<'_, crate::ipc::AppState>) -> CmdResult<AnalysisStatusDto> {
    state
        .analysis
        .status()
        .map_err(|e| from_err("ANALYSIS_DB", e, true))
}

/// 入队全部未分析曲目，返回入队数。
#[tauri::command]
pub fn request_analysis(state: State<'_, crate::ipc::AppState>) -> CmdResult<u64> {
    state
        .analysis
        .request_pending()
        .map_err(|e| from_err("ANALYSIS_UNAVAILABLE", e, true))
}

/// 取消当前分析批次（活动曲目在下一个检查点退出）。
#[tauri::command]
pub fn cancel_analysis(state: State<'_, crate::ipc::AppState>) -> CmdResult<()> {
    state.analysis.cancel();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_db(tag: &str) -> (PathBuf, aura_library::db::Database) {
        let dir = std::env::temp_dir().join(format!("aura-analysis-host-{tag}-{}", now_ms()));
        std::fs::create_dir_all(&dir).expect("tempdir");
        let db = aura_library::db::Database::open(&dir.join("library.db")).expect("open db");
        db.connection()
            .execute_batch(
                "INSERT INTO library_roots (id, path, date_added_ms) VALUES (1, 'M:/music', 0);
                 INSERT INTO artists (id, name, norm) VALUES (1, 'a', 'a');
                 INSERT INTO tracks (id, identity_key, audio_sig, title, artist_id, file_size,
                                     modified_ms, date_added_ms, last_scanned_ms, is_active)
                 VALUES (1, 'k1', 's', 'one', 1, 10, 0, 0, 0, 1),
                        (2, 'k2', 's', 'two', 1, 10, 0, 0, 0, 1),
                        (3, 'k3', 's', 'cue', 1, 10, 0, 0, 0, 1);
                 INSERT INTO track_paths (id, track_id, root_id, path, is_active,
                                          first_seen_ms, last_seen_ms)
                 VALUES (1, 1, 1, 'M:/music/a.flac', 1, 0, 0),
                        (2, 2, 1, 'M:/music/b.flac', 1, 0, 0),
                        (3, 3, 1, 'M:/music/c.flac#02', 1, 0, 0);
                 UPDATE tracks SET cue_start_seconds = 2.0 WHERE id = 3;",
            )
            .expect("seed");
        (dir, db)
    }

    /// 队列边界：CUE 虚拟子轨与已有分析行都不进队；"已分析"只数 Completed。
    #[test]
    fn pending_excludes_cue_subtracks_and_analyzed_rows() {
        let (dir, db) = seeded_db("pending");
        let conn = db.connection();
        assert_eq!(pending_ids(conn).expect("pending"), vec![1, 2]);
        assert_eq!(pending_count(conn).expect("count"), 2);

        AnalysisStore::save(conn, &TrackAnalysis::skeleton(1, "k1")).expect("save");
        assert_eq!(pending_ids(conn).expect("pending2"), vec![2]);
        assert_eq!(
            count(conn, ANALYZED_COUNT_SQL).expect("analyzed"),
            0,
            "Pending 行不得计入已分析"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 分析源解析：路径取活跃 path，CUE 子轨的合成路径不会被当成文件去开。
    #[test]
    fn track_source_resolves_active_path_only() {
        let (dir, db) = seeded_db("source");
        let (identity, path) = track_source(db.connection(), 1)
            .expect("source")
            .expect("track 1");
        assert_eq!(identity, "k1");
        assert_eq!(path, "M:/music/a.flac");
        assert!(track_source(db.connection(), 999).expect("miss").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// §335 门：无可用特征的 Completed 结果不落库（不覆盖既有结果）。
    #[test]
    fn completed_without_features_is_not_persisted() {
        let (dir, db) = seeded_db("gate");
        let mut skeleton = TrackAnalysis::skeleton(1, "k1");
        skeleton.status = AnalysisStatus::Completed;
        assert!(
            persist(db.connection(), &skeleton).is_err(),
            "空特征的 Completed 必须被拒绝"
        );
        assert!(pending_ids(db.connection()).expect("pending").contains(&1));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
