//! ifuyo Aura Tauri 壳（M6）：应用层装配 + typed IPC。
//! 分层（§7）：UI → typed IPC → 本模块 commands → aura-library/sources/core。

use tauri::Manager;

pub mod audio;
pub mod ipc;
pub mod library;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            app.manage(ipc::AppState {
                audio: std::sync::Arc::new(audio),
                library: std::sync::Arc::new(library),
                app: handle,
            });
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running ifuyo Aura");
}
