//! ifuyo Aura Tauri 壳（M6）：应用层装配 + typed IPC。
//! 分层（§7）：UI → typed IPC → 本模块 commands → aura-library/sources/core。

use tauri::Manager;

pub mod analysis_host;
pub mod analysis_ipc;
pub mod audio;
pub mod dreamscape;
pub mod ifuyo;
pub mod integration;
pub mod ipc;
pub mod library;
pub mod lyrics;
pub mod map_ipc;
pub mod security_limits;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // §116/§118：单实例最先建立（先于 SMTC/托盘注册窗口期）。
        .plugin(integration::single_instance::plugin())
        .plugin(tauri_plugin_dialog::init())
        // M11：官方 updater（§5/§6 架构：Update Service → Tauri Updater
        // Adapter → 已签名 artifact；Rust 侧只注册，策略面在前端
        // UpdateService + release 配置隔离 §77）。
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .on_window_event(|window, event| {
            // §97/§98/§174 默认策略：主窗 X = 正式退出（后台模式为 B 级，
            // 未实现——不得悄悄后台驻留）。
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    window.app_handle().exit(0);
                }
            }
        })
        .setup(|app| {
            let handle = app.handle().clone();
            // AURA_DATA_DIR：QA 钩子（M12.5 L2/L3/soak 走隔离临时资料目录，
            // 零用户数据污染；缺省仍为系统 app_data_dir）。仅影响本机数据
            // 位置，无权限/安全边界变化。
            let data_dir = std::env::var("AURA_DATA_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    handle
                        .path()
                        .app_data_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                });
            // 曲目自然播完 → 前端事件（队列按播放模式自动前进，M12.6）。
            let audio = audio::AudioHandle::start({
                let handle = handle.clone();
                move || {
                    use tauri::Emitter;
                    let _ = handle.emit("track-ended", ());
                }
            })
            .map_err(|e| format!("audio init failed: {e}"))?;
            let library = library::LibraryState::open(&data_dir)?;
            let settings = library.settings.lock().expect("settings").clone();
            // M8 分析宿主：worker 自带一条 DB 连接（WAL + busy_timeout，与
            // 扫描/查询并发）；启动时补齐未分析曲目，扫描后再补一次。
            let db_path = library
                .service
                .lock()
                .map_err(|_| "library state lock")?
                .database()
                .path();
            let analysis =
                std::sync::Arc::new(analysis_host::AnalysisHost::spawn(&handle, db_path));
            if settings.analysis_auto {
                let _ = analysis.request_pending();
            }
            // M15a：标签乱码修复开关（扫描线程读取原子量）。
            aura_library::metadata::mojibake::set_mojibake_fix(settings.mojibake_fix);
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
            // 严格策略客户端：服务侧给出的 URL（远程曲库清单）专用。
            let http_strict = ipc::build_strict_http(&blocklist).map_err(|e| e.to_string())?;
            let dreamscape = dreamscape::DreamscapeHost::open(&data_dir)
                .map_err(|e| format!("dreamscape init failed: {e}"))?;
            let map =
                map_ipc::MapHost::open(&data_dir).map_err(|e| format!("map init failed: {e}"))?;
            let integration = integration::Shared::new();
            app.manage(ipc::AppState {
                audio: std::sync::Arc::new(audio),
                library: std::sync::Arc::new(library),
                app: handle.clone(),
                http,
                http_strict,
                blocklist,
                dreamscape,
                map,
                analysis,
                integration: integration.clone(),
            });
            // §118 初始化顺序：… → Windows Integration worker → SMTC → Tray。
            // §120：托盘失败只降级（Tray=unavailable），应用继续运行。
            let st = app.state::<ipc::AppState>();
            integration::worker::spawn(
                handle.clone(),
                st.audio.clone(),
                st.library.clone(),
                integration,
            );
            let lang = st
                .library
                .settings
                .lock()
                .expect("settings")
                .language
                .clone();
            if let Err(e) = integration::tray::create(&handle, &lang) {
                eprintln!("tray unavailable: {e}");
            }
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
            ipc::player_level,
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
            ipc::lyrics_for_track,
            ipc::track_meta,
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
            ifuyo::ifuyo_list,
            ifuyo::ifuyo_play,
            analysis_host::analysis_status,
            analysis_host::request_analysis,
            analysis_host::cancel_analysis,
            map_ipc::get_map_status,
            map_ipc::get_map_overview,
            map_ipc::get_map_nodes,
            map_ipc::get_map_clusters,
            map_ipc::get_map_neighbors,
            map_ipc::get_map_track,
            map_ipc::request_map_build,
            map_ipc::cancel_map_build,
            ipc::report_queue_state,
            ipc::get_integration_diagnostics,
        ]) // probe-isolated
        .run(tauri::generate_context!())
        .expect("error while running ifuyo Aura");
}
