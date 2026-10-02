// ifuyo Aura Tauri 壳入口。
// Phase 0/M0：最小 builder + 最小 typed IPC（app_info）。
// 窗口尺寸遵循 Charter §21（默认 1180×760，最小 900×600；无边框自绘标题栏在 M6 启用）。

mod ipc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ipc::app_info])
        .run(tauri::generate_context!())
        .expect("error while running ifuyo Aura");
}
