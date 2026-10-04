//! ifuyo Aura Tauri 壳（M6）：应用层装配 + typed IPC。
//! 分层（§7）：UI → typed IPC → 本模块 commands → aura-library/sources/core。

use tauri::Manager;

pub mod analysis_ipc;
pub mod audio;
pub mod dreamscape;
pub mod ipc;
pub mod library;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let audio =
                audio::AudioHandle::start().map_err(|e| format!("audio init failed: {e}"))?;
            let library = library::LibraryState::open(&data_dir)?;
            let settings = library.settings.lock().expect("settings").clone();
            // 启动即应用持久化设置（§114/§155 启动优先级 3）。
            let _ = audio.send(audio::Control::SetCrossfade(settings.crossfade_seconds));
            let _ = audio.send(audio::Control::SetLoudness(settings.loudness_enabled));
            let _ = audio.send(audio::Control::SetEqPreset(settings.eq_preset));
            // M4 HTTP 客户端（D-070：用户显式输入 = 私网确认；scheme/凭据/
            // 封锁表校验始终生效，§278 不因 UI 入口降低 M4 保护）。
            let blocklist = std::sync::Arc::new(
                aura_sources::platform_block::PlatformBlocklist::embedded()
                    .map_err(|e| format!("blocklist init failed: {e}"))?,
            );
            let http_cfg = aura_sources::http::HttpConfig {
                policy: aura_sources::url_policy::NetworkPolicy::allow_private(),
                ..aura_sources::http::HttpConfig::default()
            };
            let http = std::sync::Arc::new(
                aura_sources::http::HttpClient::new(http_cfg, std::sync::Arc::clone(&blocklist))
                    .map_err(|e| format!("http client init failed: {e}"))?,
            );
            let dreamscape = dreamscape::DreamscapeHost::open(&data_dir)
                .map_err(|e| format!("dreamscape init failed: {e}"))?;
            app.manage(ipc::AppState {
                audio: std::sync::Arc::new(audio),
                library: std::sync::Arc::new(library),
                app: handle,
                http,
                blocklist,
                dreamscape,
            });
            // §19：Win11 Mica（设置开关，失败自动保持 solid fallback §20）。
            #[cfg(target_os = "windows")]
            {
                use tauri::Manager as _;
                let mica_enabled = app
                    .state::<ipc::AppState>()
                    .library
                    .settings
                    .lock()
                    .expect("settings")
                    .mica_enabled;
                if mica_enabled {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = window_vibrancy::apply_mica(&win, None);
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::app_info,
            ipc::player_snapshot,
            ipc::player_position,
            ipc::play,
            ipc::pause,
            ipc::stop,
            ipc::seek,
            ipc::set_volume,
            ipc::play_track,
            ipc::set_crossfade,
            ipc::set_loudness,
            ipc::set_eq_preset,
            ipc::library_list,
            ipc::library_search,
            ipc::track_path,
            ipc::scan_folder,
            ipc::cancel_scan,
            ipc::get_settings,
            ipc::save_settings,
            ipc::track_artwork,
            ipc::set_mica,
            ipc::track_id_by_path,
            ipc::play_input,
            ipc::library_stats,
            dreamscape::dreamscape_state,
            dreamscape::dreamscape_advance,
            dreamscape::dreamscape_manifest,
            dreamscape::dreamscape_regenerate,
            dreamscape::dreamscape_export_seed,
            dreamscape::dreamscape_import_seed,
            dreamscape::dreamscape_save_snapshot,
            dreamscape::dreamscape_restore_snapshot,
            dreamscape::dreamscape_set_camera,
            dreamscape::dreamscape_open,
            dreamscape::dreamscape_close,
            dreamscape::dreamscape_set_quality,
            dreamscape::dreamscape_set_environment,
            analysis_ipc::track_analysis_summary,
            analysis_ipc::analysis_visual_summary,
            analysis_ipc::track_analysis_state,
        ]) // probe-isolated
        .run(tauri::generate_context!())
        .expect("error while running ifuyo Aura");
}
