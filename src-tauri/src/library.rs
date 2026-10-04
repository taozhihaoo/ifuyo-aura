//! Library 服务封装（M3）+ 应用设置持久化（§114–§116）。
//! UI 只见本模块 DTO；SQLite 细节留在 aura-library（§7 分层）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use aura_library::service::LibraryService;
use aura_library::types::{CancelToken, ScanProgress};

/// UI 侧 track 摘要 DTO（§11/§12：最小 DTO，不泄漏 SQLite 行）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSummary {
    pub id: i64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_seconds: Option<f64>,
    pub year: Option<i32>,
    pub path: String,
    pub has_artwork: bool,
}

/// 扫描进度 DTO（§88）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressDto {
    pub discovered: u64,
    pub processed: u64,
    pub imported: u64,
    pub failed: u64,
    pub current_path: String,
    pub done: bool,
}

/// 应用设置（§114）。`version` 向后兼容缺省（§116）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub version: u32,
    #[serde(default = "default_language")]
    pub language: String,
    /// M11 §84：自动检查更新（默认 true；非阻塞、无遥测 §8/§44）。
    #[serde(default = "default_true")]
    pub updates_auto_check: bool,
    #[serde(default = "default_scale")]
    pub ui_scale: f32,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default)]
    pub loudness_enabled: bool,
    #[serde(default)]
    pub crossfade_seconds: f64,
    #[serde(default = "default_true")]
    pub eq_enabled: bool,
    #[serde(default = "default_eq_preset")]
    pub eq_preset: usize,
    #[serde(default = "default_true")]
    pub mica_enabled: bool,
}

fn default_language() -> String {
    "zh-CN".into()
}
fn default_scale() -> f32 {
    1.0
}
fn default_true() -> bool {
    true
}
fn default_eq_preset() -> usize {
    0
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: 1,
            language: default_language(),
            updates_auto_check: true,
            ui_scale: default_scale(),
            reduced_motion: false,
            loudness_enabled: false,
            crossfade_seconds: 0.0,
            eq_enabled: true,
            eq_preset: default_eq_preset(),
            mica_enabled: true,
        }
    }
}

/// 共享 library 状态。
pub struct LibraryState {
    pub settings_path: PathBuf,
    pub settings: Mutex<AppSettings>,
    pub service: Mutex<LibraryService>,
    pub cancel: Mutex<CancelToken>,
    pub scanning: AtomicBool,
}

impl LibraryState {
    /// 打开 library DB + 加载设置（§53：失败返回字符串错误）。
    ///
    /// # Errors
    /// 目录创建/DB 打开失败。
    pub fn open(app_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        let service =
            LibraryService::open(&app_dir.join("library.db")).map_err(|e| e.to_string())?;
        let settings_path = app_dir.join("settings.json");
        let settings = std::fs::read(&settings_path)
            .ok()
            .and_then(|b| serde_json::from_slice::<AppSettings>(&b).ok())
            .unwrap_or_default();
        Ok(Self {
            settings_path,
            settings: Mutex::new(settings),
            service: Mutex::new(service),
            cancel: Mutex::new(CancelToken::new()),
            scanning: AtomicBool::new(false),
        })
    }

    /// 保存设置。
    ///
    /// # Errors
    /// 序列化/写盘失败。
    pub fn save_settings(&self, s: &AppSettings) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
        std::fs::write(&self.settings_path, json).map_err(|e| e.to_string())
    }

    /// 同步扫描（后台线程调用）；进度经回调上抛（§88）。
    ///
    /// # Errors
    /// 已有扫描在跑 / 扫描失败。
    pub fn scan_root_blocking(
        &self,
        root: &Path,
        on_progress: impl Fn(ScanProgressDto),
    ) -> Result<aura_library::types::ScanResult, String> {
        if self.scanning.swap(true, Ordering::SeqCst) {
            return Err("scan already running".into());
        }
        let result = self.scan_inner(root, on_progress);
        self.scanning.store(false, Ordering::SeqCst);
        result
    }

    fn scan_inner(
        &self,
        root: &Path,
        on_progress: impl Fn(ScanProgressDto),
    ) -> Result<aura_library::types::ScanResult, String> {
        // &mut LibraryService 经 std::sync::MutexGuard 的 DerefMut 获得。
        let mut service = self.service.lock().map_err(|_| "library lock")?;
        service
            .add_root(root, Self::now_ms())
            .map_err(|e| e.to_string())?;
        let cancel = CancelToken::new();
        *self.cancel.lock().expect("cancel lock") = cancel.clone();
        service
            .scan_root(
                root,
                &cancel,
                Some(&mut |p: &ScanProgress| {
                    on_progress(ScanProgressDto {
                        discovered: p.discovered,
                        processed: p.processed,
                        imported: p.imported,
                        failed: p.failed,
                        current_path: p.current_path.to_string_lossy().into_owned(),
                        done: false,
                    });
                }),
            )
            .map_err(|e| e.to_string())
    }

    /// TrackId → 主路径（播放管线入口，§93）。
    ///
    /// # Errors
    /// track 不存在/路径不可用。
    pub fn track_path(&self, track_id: i64) -> Result<String, String> {
        let service = self.service.lock().map_err(|_| "library lock")?;
        let track = service
            .get_track(aura_library::types::TrackId(track_id))
            .map_err(|e| e.to_string())?
            .ok_or("track not found")?;
        // 主路径：该 track 的第一条活跃 path（M3 track_paths 表）。
        let path: Option<String> = service
            .database()
            .connection()
            .query_row(
                "SELECT path FROM track_paths WHERE track_id = ?1 AND is_active = 1
                 ORDER BY id LIMIT 1",
                rusqlite::params![track_id],
                |r| r.get(0),
            )
            .ok();
        path.ok_or_else(|| format!("track {} has no active path", track.identity_key))
    }

    pub fn now_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0)
    }
}

/// 共享句柄别名。
pub type SharedLibrary = Arc<LibraryState>;
