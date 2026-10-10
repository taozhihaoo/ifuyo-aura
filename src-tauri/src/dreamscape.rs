//! Dreamscape 应用服务（M7 上篇）：会话装配 + 逻辑态持久化 + typed IPC。
//! 分层（§7/§8）：aura-synth 域层负责「世界是什么」；本层只装配与存取，
//! 渲染（下篇 Three.js）负责「世界怎么被看见」。
//! 持久化边界（§127）：只存逻辑态快照 JSON；不缓存 mesh，不动 M5。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use aura_synth::seed::DreamscapeSeed;
use aura_synth::snapshot::{CameraState, DreamscapeSession, RestoreError};
use serde::Serialize;

use crate::ipc::{from_err, AppError, CmdResult};

/// 持久化文件名（app_data 下）。
const SNAPSHOT_FILE: &str = "dreamscape.json";

/// 会话宿主（AppState 持有）。
pub struct DreamscapeHost {
    pub session: Mutex<DreamscapeSession>,
    path: PathBuf,
    /// §262 revision：状态变更单调递增；UI 只接受最新（§259/§260 陈旧代保护）。
    revision: std::sync::atomic::AtomicU64,
    /// §257 幂等：open/close 重复调用不报错（渲染器在 UI 侧单例）。
    opened: std::sync::atomic::AtomicBool,
}

impl DreamscapeHost {
    /// 启动装载：存在快照 → 恢复（损坏则回退默认并继续启动，§265 精神）；
    /// 不存在 → 缺省世界（§77/§103 不自动 regenerate）。
    ///
    /// # Errors
    /// 目录不可创建等 IO 失败。
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        let path = data_dir.join(SNAPSHOT_FILE);
        let session = load_snapshot(&path).unwrap_or_else(|e| {
            // 诊断走 stderr（桌面应用无日志管道；不弹 UI）。
            eprintln!("dreamscape snapshot load failed ({e}); using default world");
            DreamscapeSession::new(
                DreamscapeSeed::from_input(aura_synth::snapshot::DEFAULT_SEED_INPUT),
                0,
            )
        });
        Ok(Self {
            session: Mutex::new(session),
            path,
            revision: std::sync::atomic::AtomicU64::new(1),
            opened: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// §262：状态修订号（每次变更 +1）。
    fn bump(&self) -> u64 {
        self.revision
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1
    }

    /// 写快照（§225 原子替换 temp→rename；§295 非 per-mutation 写盘，
    /// 由命令边界/退出路径调用；只含逻辑态 §127）。
    ///
    /// # Errors
    /// 序列化/写入失败。
    pub fn persist(&self, session: &DreamscapeSession) -> Result<(), String> {
        let mut snap = session.snapshot();
        snap.seal();

        let json = serde_json::to_vec_pretty(&snap).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

fn load_snapshot(path: &Path) -> Result<DreamscapeSession, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let snap: aura_synth::snapshot::DreamscapeSnapshot =
        serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    DreamscapeSession::restore(&snap).map_err(|e| e.to_string())
}

/// UI 状态 DTO（typed；§110 精神——逻辑态数值，无渲染概念）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DreamscapeStateDto {
    /// §262 revision：UI 只接受最新状态（陈旧回调丢弃）。
    pub revision: u64,
    pub world_id: String,
    pub seed_human: String,
    pub generation_version: u16,
    pub biome_id: String,
    pub tick: u64,
    pub time_phase: f32,
    pub weather: String,
    pub fog: f32,
    pub wind: f32,
    pub sun_intensity: f32,
    pub ambient_intensity: f32,
    pub horizon_glow: f32,
    pub mutation_count: usize,
    pub event_count: usize,
}

fn to_dto(s: &DreamscapeSession) -> DreamscapeStateDto {
    DreamscapeStateDto {
        revision: 0,
        world_id: format!("{:016x}", s.world_id),
        seed_human: s.export_seed(),
        generation_version: aura_synth::generation_version(),
        biome_id: s.biome.kind.id().to_string(),
        tick: s.environment.tick,
        time_phase: s.environment.time_phase().0,
        weather: format!("{:?}", s.environment.weather),
        fog: s.environment.fog,
        wind: s.environment.wind,
        sun_intensity: s.environment.sun_intensity,
        ambient_intensity: s.environment.ambient_intensity,
        horizon_glow: s.environment.horizon_glow,
        mutation_count: s.mutation_log.len(),
        event_count: s.event_log.len(),
    }
}

fn to_dto_rev(s: &DreamscapeSession, revision: u64) -> DreamscapeStateDto {
    let mut dto = to_dto(s);
    dto.revision = revision;
    dto
}

fn restore_err(e: RestoreError) -> AppError {
    match e {
        // §106：World version unavailable（明确，不静默换世界）。
        RestoreError::UnsupportedGeneration(_) | RestoreError::UnsupportedSnapshot(_) => {
            AppError::new("DREAMSCAPE_VERSION_UNAVAILABLE", e.to_string(), false)
        }
        RestoreError::Corrupt => AppError::new("DREAMSCAPE_CORRUPT", e.to_string(), true),
    }
}

/// 当前 Dreamscape 状态（typed DTO 通道 + §262 revision）。
#[tauri::command]
pub fn dreamscape_state(
    state: tauri::State<'_, crate::ipc::AppState>,
) -> CmdResult<DreamscapeStateDto> {
    let host = &state.dreamscape;
    let s = host.session.lock().expect("dreamscape lock");
    Ok(to_dto_rev(
        &s,
        host.revision.load(std::sync::atomic::Ordering::SeqCst),
    ))
}

/// §256/§257 open：幂等——重复 open 不报错、不建第二渲染器
/// （渲染器单例在 UI 侧；此处仅置位 + 返回状态）。
#[tauri::command]
pub fn dreamscape_open(
    state: tauri::State<'_, crate::ipc::AppState>,
) -> CmdResult<DreamscapeStateDto> {
    let host = &state.dreamscape;
    host.opened.store(true, std::sync::atomic::Ordering::SeqCst);
    let s = host.session.lock().expect("dreamscape lock");
    Ok(to_dto_rev(
        &s,
        host.revision.load(std::sync::atomic::Ordering::SeqCst),
    ))
}

/// §256/§257 close：幂等——重复 close 不报错；§293 渲染器 dispose
/// 由 UI 卸载路径负责，此处只置位（不销毁世界状态 §75）。
#[tauri::command]
pub fn dreamscape_close(
    state: tauri::State<'_, crate::ipc::AppState>,
) -> CmdResult<DreamscapeStateDto> {
    let host = &state.dreamscape;
    host.opened
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let s = host.session.lock().expect("dreamscape lock");
    Ok(to_dto_rev(
        &s,
        host.revision.load(std::sync::atomic::Ordering::SeqCst),
    ))
}

/// §100/§101/§256 set_environment：用户显式环境控制——冻结/解冻
/// （B 级手动 time-of-day/weather 预设留待后续；Auto/Frozen 已可用）。
#[tauri::command]
pub fn dreamscape_set_environment(
    state: tauri::State<'_, crate::ipc::AppState>,
    frozen: bool,
) -> CmdResult<DreamscapeStateDto> {
    let host = &state.dreamscape;
    let rev = host.bump();
    let mut s = host.session.lock().expect("dreamscape lock");
    s.frozen = frozen;
    Ok(to_dto_rev(&s, rev))
}

/// 推进逻辑时间（1 tick = 1 逻辑分钟，§30；调用频率由下篇集成决定）。
#[tauri::command]
pub fn dreamscape_advance(
    state: tauri::State<'_, crate::ipc::AppState>,
    ticks: u64,
) -> CmdResult<DreamscapeStateDto> {
    let host = &state.dreamscape;
    let rev = host.bump();
    let mut s = host.session.lock().expect("dreamscape lock");
    s.advance(ticks.min(24 * 60 * 365)); // 单次上限：一年逻辑时间
    let dto = to_dto_rev(&s, rev);
    let _ = host.persist(&s);
    Ok(dto)
}

/// 光场清单（渲染层唯一输入；§109 输出。命令名保留 `manifest` 语义）。
#[tauri::command]
pub fn dreamscape_manifest(
    state: tauri::State<'_, crate::ipc::AppState>,
) -> CmdResult<aura_synth::manifest::LightField> {
    let host = &state.dreamscape;
    let s = host.session.lock().expect("dreamscape lock");
    Ok(s.field())
}

/// 显式再生（§103：仅用户主动）。
#[tauri::command]
pub fn dreamscape_regenerate(
    state: tauri::State<'_, crate::ipc::AppState>,
    input: String,
) -> CmdResult<DreamscapeStateDto> {
    let host = &state.dreamscape;
    let rev = host.bump();
    let mut s = host.session.lock().expect("dreamscape lock");
    let tick = s.environment.tick;
    s.regenerate(&input, tick);
    let dto = to_dto_rev(&s, rev);
    let _ = host.persist(&s);
    Ok(dto)
}

/// 导出 seed（§104）。
#[tauri::command]
pub fn dreamscape_export_seed(state: tauri::State<'_, crate::ipc::AppState>) -> CmdResult<String> {
    let host = &state.dreamscape;
    let s = host.session.lock().expect("dreamscape lock");
    Ok(s.export_seed())
}

/// 导入 seed（§105：非法 → INVALID_SEED，不 crash）。
#[tauri::command]
pub fn dreamscape_import_seed(
    state: tauri::State<'_, crate::ipc::AppState>,
    input: String,
) -> CmdResult<DreamscapeStateDto> {
    let host = &state.dreamscape;
    let rev = host.bump();
    let mut s = host.session.lock().expect("dreamscape lock");
    let tick = s.environment.tick;
    s.import_seed(&input, tick)
        .map_err(|_| AppError::new("INVALID_SEED", "invalid dreamscape seed", false))?;
    let dto = to_dto_rev(&s, rev);
    let _ = host.persist(&s);
    Ok(dto)
}

/// 保存快照（§79；手动「Save Snapshot」）。
#[tauri::command]
pub fn dreamscape_save_snapshot(state: tauri::State<'_, crate::ipc::AppState>) -> CmdResult<()> {
    let host = &state.dreamscape;
    let s = host.session.lock().expect("dreamscape lock");
    host.persist(&s)
        .map_err(|e| from_err("SETTINGS_WRITE", e, true))
}

/// 恢复快照（§81；缺失/损坏/版本不支持 → 各自结构化错误码）。
#[tauri::command]
pub fn dreamscape_restore_snapshot(
    state: tauri::State<'_, crate::ipc::AppState>,
) -> CmdResult<DreamscapeStateDto> {
    let host = &state.dreamscape;
    let bytes =
        std::fs::read(&host.path).map_err(|e| from_err("DREAMSCAPE_NO_SNAPSHOT", e, true))?;
    let snap: aura_synth::snapshot::DreamscapeSnapshot =
        serde_json::from_slice(&bytes).map_err(|e| from_err("DREAMSCAPE_CORRUPT", e, true))?;
    snap.validate().map_err(restore_err)?;
    let restored = DreamscapeSession::restore(&snap).map_err(restore_err)?;
    let rev = host.bump();
    let mut s = host.session.lock().expect("dreamscape lock");
    *s = restored;
    Ok(to_dto_rev(&s, rev))
}

/// 相机逻辑态写入（§78；数值由下篇相机控制器产生）。
#[tauri::command]
pub fn dreamscape_set_camera(
    state: tauri::State<'_, crate::ipc::AppState>,
    yaw: f32,
    pitch: f32,
    distance: f32,
) -> CmdResult<()> {
    let host = &state.dreamscape;
    let mut s = host.session.lock().expect("dreamscape lock");
    s.camera = CameraState {
        yaw: yaw.rem_euclid(std::f32::consts::TAU),
        pitch: pitch.clamp(-0.2, 1.2),
        distance: distance.clamp(8.0, 160.0),
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aura_synth::seed::DreamscapeSeed;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("aura-m7-ds-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn default_world_stable_and_persists() {
        // §77/§127：首启缺省世界；persist → open 恢复同一逻辑态。
        let dir = tmp_dir("persist");
        let host = DreamscapeHost::open(&dir).unwrap();
        let s0 = host.session.lock().expect("lock");
        let id0 = s0.world_id;
        let seed0 = s0.export_seed();
        drop(s0);
        {
            let mut s = host.session.lock().expect("lock");
            s.advance(5_000);
        }
        host.persist(&host.session.lock().expect("lock")).unwrap();

        let host2 = DreamscapeHost::open(&dir).unwrap();
        let s2 = host2.session.lock().expect("lock");
        assert_eq!(s2.world_id, id0, "同一世界（§77）");
        assert_eq!(s2.export_seed(), seed0);
        assert!(s2.environment.tick >= 5_000, "逻辑时间被持久化");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_snapshot_falls_back_to_default() {
        // §265/§266 精神：损坏的 dreamscape.json 不崩启动，回退默认。
        let dir = tmp_dir("corrupt");
        std::fs::write(dir.join(SNAPSHOT_FILE), b"{ not json").unwrap();
        let host = DreamscapeHost::open(&dir).unwrap();
        let s = host.session.lock().expect("lock");
        assert_eq!(
            s.seed,
            DreamscapeSeed::from_input(aura_synth::snapshot::DEFAULT_SEED_INPUT)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restore_version_mismatch_surfaces() {
        // §106：版本不再支持 → 结构化错误（Host 恢复路径）。
        let dir = tmp_dir("version");
        let host = DreamscapeHost::open(&dir).unwrap();
        {
            let mut s = host.session.lock().expect("lock");
            s.advance(1_000);
        }
        let snap = host.session.lock().expect("lock").snapshot();
        host.persist(&host.session.lock().expect("lock")).unwrap();
        // 手改文件为未来代（模拟旧世界与新代不匹配的反向场景：未知代）。
        let mut v = serde_json::to_value(&snap).unwrap();
        v["generation_version"] = serde_json::json!(99);
        std::fs::write(dir.join(SNAPSHOT_FILE), serde_json::to_vec(&v).unwrap()).unwrap();
        let err = load_snapshot(&dir.join(SNAPSHOT_FILE)).unwrap_err();
        assert!(err.contains("generation version 99"), "明确版本错误：{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
