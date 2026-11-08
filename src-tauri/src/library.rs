//! Library 服务封装（M3）+ 应用设置持久化（§114–§116）。
//! UI 只见本模块 DTO；SQLite 细节留在 aura-library（§7 分层）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    /// 扫描失败原因（成功为 None）。`done` 只表示「这一轮结束」，
    /// 前端必须同时看 error 才能判定成功（§88 错误不得被吞）。
    pub error: Option<String>,
}

/// 应用设置（§114）。`version` 向后兼容缺省（§116）。
///
/// IPC/磁盘 JSON 契约统一为 camelCase，与 `ui/src/ipc/types.ts` 的
/// `AppSettings` 一致（此前缺 rename_all，前端读到的全是 undefined，
/// 而 E2E 脚本直接按 snake_case 调 IPC 因此假绿）。
/// 每个多词字段保留 snake_case `alias`，以便无损读取 M15b 之前写盘的
/// settings.json；下次保存即自动迁移为 camelCase。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub version: u32,
    #[serde(default = "default_language")]
    pub language: String,
    /// M11 §84：自动检查更新（默认 true；非阻塞、无遥测 §8/§44）。
    #[serde(default = "default_true", alias = "updates_auto_check")]
    pub updates_auto_check: bool,
    /// M15a：Latin-1 声明下 GBK 标签乱码自动修复（默认开；双轨防误判）。
    #[serde(default = "default_true", alias = "mojibake_fix")]
    pub mojibake_fix: bool,
    /// M8 分析接线：扫描后自动排队分析（默认开；Music Map 与响度归一
    /// 的数据源）。
    #[serde(default = "default_true", alias = "analysis_auto")]
    pub analysis_auto: bool,
    #[serde(default = "default_scale", alias = "ui_scale")]
    pub ui_scale: f32,
    #[serde(default, alias = "reduced_motion")]
    pub reduced_motion: bool,
    #[serde(default, alias = "loudness_enabled")]
    pub loudness_enabled: bool,
    #[serde(default, alias = "crossfade_seconds")]
    pub crossfade_seconds: f64,
    #[serde(default = "default_true", alias = "eq_enabled")]
    pub eq_enabled: bool,
    #[serde(default = "default_eq_preset", alias = "eq_preset")]
    pub eq_preset: usize,
    #[serde(default = "default_true", alias = "mica_enabled")]
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

impl AppSettings {
    /// IPC 边界统一规范化。`save_settings` 接收的是前端整包对象，若不在这里
    /// 钳制，就会绕过 `set_crossfade` / `set_eq_preset` 等 setter 的校验，
    /// 把超界值写进 settings.json，启动时又直接喂给音频引擎。
    pub fn normalized(mut self) -> Self {
        if !matches!(self.language.as_str(), "en" | "zh-CN") {
            self.language = default_language();
        }
        if !self.ui_scale.is_finite() || !(0.9..=1.4).contains(&self.ui_scale) {
            self.ui_scale = default_scale();
        }
        if !self.crossfade_seconds.is_finite() {
            self.crossfade_seconds = 0.0;
        }
        self.crossfade_seconds = self.crossfade_seconds.clamp(0.0, 12.0);
        self.eq_preset = self
            .eq_preset
            .min(aura_core::dsp::EQ_PRESETS.len().saturating_sub(1));
        self
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            version: 1,
            language: default_language(),
            updates_auto_check: true,
            mojibake_fix: true,
            analysis_auto: true,
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
    /// 库 DB 路径。扫描走独立 `LibraryService` 连接，避免长时间持有
    /// `service` Mutex 把列表/搜索/播放等所有曲库 IPC 一起阻塞。
    pub db_path: PathBuf,
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
        let db_path = app_dir.join("library.db");
        let service = LibraryService::open(&db_path).map_err(|e| e.to_string())?;
        let settings_path = app_dir.join("settings.json");
        let settings = match std::fs::read(&settings_path) {
            Ok(bytes) => match serde_json::from_slice::<AppSettings>(&bytes) {
                Ok(s) => s.normalized(),
                Err(e) => {
                    // 写盘中途崩溃/断电会留下截断 JSON。旧实现直接
                    // `.ok().unwrap_or_default()` 静默丢光全部偏好且不留现场；
                    // 这里先保留损坏文件供排查，再回落默认值。
                    let backup =
                        settings_path.with_extension(format!("json.corrupt-{}", Self::now_ms()));
                    let _ = std::fs::rename(&settings_path, &backup);
                    eprintln!(
                        "[settings] corrupt JSON ({e}); kept at {}",
                        backup.display()
                    );
                    AppSettings::default()
                }
            },
            Err(_) => AppSettings::default(), // 文件不存在 = 首次运行
        };
        Ok(Self {
            settings_path,
            db_path,
            settings: Mutex::new(settings),
            service: Mutex::new(service),
            cancel: Mutex::new(CancelToken::new()),
            scanning: AtomicBool::new(false),
        })
    }

    /// 保存设置（写盘 + 同步内存缓存）。
    ///
    /// `get_settings` 读内存缓存——只写盘不更新缓存会让写后立即回读拿到
    /// 旧值（M12.5 L2 verify-settings 实测抓出）。
    ///
    /// # Errors
    /// 序列化/写盘失败。
    pub fn save_settings(&self, s: &AppSettings) -> Result<(), String> {
        let normalized = s.clone().normalized();
        // 全程持锁：与 set_crossfade 等「持锁 + write_settings」的命令共用
        // 同一把锁，避免两个写者同时写同一个临时文件、互相覆盖 rename。
        // 写盘失败时不改内存缓存，否则 UI 回读新值、重启却回滚旧值。
        let mut guard = self
            .settings
            .lock()
            .map_err(|_| "settings lock poisoned".to_string())?;
        self.write_settings(&normalized)?;
        *guard = normalized;
        Ok(())
    }

    /// 仅写盘。供**已持有 settings 锁**的调用方（set_crossfade 等
    /// 部分持久化命令）使用；save_settings 也会持锁后调用它。
    /// 本函数自身不取锁，避免持锁重入死锁。
    ///
    /// # Errors
    /// 序列化/写盘失败。
    pub fn write_settings(&self, s: &AppSettings) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
        // tmp → 原子 rename：裸 fs::write 在写入中途崩溃会留下半截 JSON，
        // 下次启动用户偏好被静默清空（Dreamscape/Map 早已采用此写法）。
        // 临时名必须唯一：Tauri 命令是线程池，两个并发写者共用固定的
        // settings.json.tmp 会互相截断/抢占 rename。
        static TMP_SEQ: AtomicU64 = AtomicU64::new(0);
        let tmp = self.settings_path.with_extension(format!(
            "json.{}.{}.tmp",
            std::process::id(),
            TMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.settings_path).map_err(|e| e.to_string())
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
        // 扫描是数秒到数分钟的 IO：不能持有共享 `service` Mutex，否则
        // library_list / library_search / track_artwork / play_track 等
        // 所有曲库 IPC 都在 lock() 上排队，UI 像假死。改用独立连接
        // （WAL + 5s busy_timeout，单文件事务），与 analysis_host 同模式。
        let mut service = LibraryService::open(&self.db_path).map_err(|e| e.to_string())?;
        service
            .add_root(root, Self::now_ms())
            .map_err(|e| e.to_string())?;
        let cancel = CancelToken::new();
        *self.cancel.lock().map_err(|_| "cancel lock")? = cancel.clone();
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
                        error: None,
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

#[cfg(test)]
mod tests {
    use super::AppSettings;

    /// IPC/磁盘契约：字段名必须是 camelCase，且不得混入 snake_case。
    /// 回归的是「前端读到全 undefined、E2E 却假绿」的 P0 缺陷。
    #[test]
    fn settings_json_is_camel_case() {
        let v = serde_json::to_value(AppSettings::default()).expect("serialize");
        for key in [
            "updatesAutoCheck",
            "mojibakeFix",
            "analysisAuto",
            "uiScale",
            "reducedMotion",
            "loudnessEnabled",
            "crossfadeSeconds",
            "eqEnabled",
            "eqPreset",
            "micaEnabled",
        ] {
            assert!(v.get(key).is_some(), "missing camelCase key {key}");
        }
        for key in ["updates_auto_check", "crossfade_seconds", "eq_preset"] {
            assert!(
                v.get(key).is_none(),
                "snake_case key {key} leaked into IPC payload"
            );
        }
    }

    /// camelCase 载荷是主路径：任何字段都不能静默回落 default。
    #[test]
    fn camel_case_payload_is_not_reset_to_defaults() {
        let json = r#"{"version":1,"language":"en","updatesAutoCheck":false,
            "mojibakeFix":false,"analysisAuto":false,"uiScale":1.25,
            "reducedMotion":true,"loudnessEnabled":true,"crossfadeSeconds":4.0,
            "eqEnabled":false,"eqPreset":2,"micaEnabled":false}"#;
        let s: AppSettings = serde_json::from_str(json).expect("deserialize");
        assert_eq!(s.language, "en");
        assert_eq!(s.ui_scale, 1.25);
        assert_eq!(s.crossfade_seconds, 4.0);
        assert_eq!(s.eq_preset, 2);
        assert!(!s.updates_auto_check);
        assert!(!s.eq_enabled);
        assert!(s.reduced_motion);
    }

    /// M15b 之前写盘的 snake_case settings.json 必须无损读取（alias），
    /// 且回写自动迁移为 camelCase。
    #[test]
    fn legacy_snake_case_settings_still_deserialize() {
        let legacy = r#"{"version":1,"language":"en",
            "updates_auto_check":false,"mojibake_fix":false,
            "analysis_auto":false,"ui_scale":0.9,"reduced_motion":true,
            "loudness_enabled":true,"crossfade_seconds":2.5,
            "eq_enabled":false,"eq_preset":3,"mica_enabled":false}"#;
        let s: AppSettings = serde_json::from_str(legacy).expect("legacy deserialize");
        assert!(!s.updates_auto_check);
        assert!(!s.mojibake_fix);
        assert_eq!(s.ui_scale, 0.9);
        assert_eq!(s.crossfade_seconds, 2.5);
        assert_eq!(s.eq_preset, 3);
        assert!(s.reduced_motion);
        let migrated = serde_json::to_value(&s).expect("serialize");
        assert_eq!(migrated["crossfadeSeconds"], 2.5);
        assert!(migrated.get("crossfade_seconds").is_none());
    }
}
