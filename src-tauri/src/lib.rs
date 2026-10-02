// ifuyo Aura Tauri 壳入口。
// Phase 0：仅最小 builder；IPC commands、capabilities 收敛、Mica 等按 M0/M6 落地。
// 注意：窗口尺寸遵循 Charter §21（默认 1180×760，最小 900×600，无边框自绘标题栏在 M6 启用）。

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running ifuyo Aura");
}
