//! 桌面窗口冒烟（A-37）：构建**真实 Wry 应用**（与 lib.rs 相同装配），
//! Windows 事件循环允许任意线程；验证 setup（audio + library + 设置）完成、
//! managed 状态可取、`player_snapshot` 命令函数经注入 State 返回合法快照，
//! 然后 `cleanup_before_exit()` 收尾。
//!
//! 运行：`cargo run -p aura-shell --bin smoke`
//! （桌面环境；不作为 CI 默认测试——webview 初始化依赖本机 WebView2 runtime。）
//!
//! 说明：tauri v2 的 `generate_handler!` 宏路径不接受泛型命令（E0283，
//! tauri issue #9290 同类），故 IPC 命令层以「直接调用命令函数 + 注入
//! State」覆盖；完整 webview IPC 序列化留待 M6 下篇 windowed 测试。
//! 零真实网络；音频子系统在桌面 Windows 可用。

use aura_shell_lib::ipc::{self, AppState};
use aura_shell_lib::{audio, library};
use std::sync::Arc;
use tauri::Manager;

fn fail(msg: &str) -> ! {
    eprintln!("SMOKE FAIL: {msg}");
    std::process::exit(1);
}

fn main() {
    let t_start = std::time::Instant::now();
    let tmp = std::env::temp_dir().join(format!("aura-m6-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let tmp_ref = tmp.clone();
    let tmp_ds = tmp.clone();

    let mut app = tauri::Builder::default()
        .any_thread()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ipc::app_info,
            ipc::player_snapshot
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let audio =
                audio::AudioHandle::start(|| {}).map_err(|e| format!("audio init failed: {e}"))?;
            let library = library::LibraryState::open(&tmp_ref)
                .map_err(|e| format!("library open failed: {e}"))?;
            let settings = library.settings.lock().expect("settings").clone();
            let _ = audio.send(audio::Control::SetCrossfade(settings.crossfade_seconds));
            let blocklist = Arc::new(
                aura_sources::platform_block::PlatformBlocklist::embedded()
                    .map_err(|e| format!("blocklist init failed: {e}"))?,
            );
            let http_cfg = aura_sources::http::HttpConfig {
                policy: aura_sources::url_policy::NetworkPolicy::allow_private(),
                ..aura_sources::http::HttpConfig::default()
            };
            let http = Arc::new(
                aura_sources::http::HttpClient::new(http_cfg, Arc::clone(&blocklist))
                    .map_err(|e| format!("http client init failed: {e}"))?,
            );
            let http_strict = aura_shell_lib::ipc::build_strict_http(&blocklist)
                .map_err(|e| format!("strict http init failed: {e}"))?;
            let dreamscape = aura_shell_lib::dreamscape::DreamscapeHost::open(&tmp_ds)
                .map_err(|e| format!("dreamscape init failed: {e}"))?;
            let map = aura_shell_lib::map_ipc::MapHost::open(&tmp_ref)
                .map_err(|e| format!("map init failed: {e}"))?;
            let analysis = Arc::new(aura_shell_lib::analysis_host::AnalysisHost::spawn(
                &handle,
                library
                    .service
                    .lock()
                    .expect("library lock")
                    .database()
                    .path(),
            ));
            app.manage(AppState {
                audio: Arc::new(audio),
                library: Arc::new(library),
                app: handle,
                http,
                http_strict,
                blocklist,
                dreamscape,
                map,
                analysis,
                integration: aura_shell_lib::integration::Shared::new(),
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("wry app build");

    let handle = app.handle().clone();

    // §155：run_iteration 触发 setup 并处理事件（有界 3 次迭代）。
    #[allow(deprecated)]
    {
        for _ in 0..3 {
            app.run_iteration(|_app, _event| {});
        }
    }

    // §203：启动基线（进程起 → setup 完成可查询）。
    println!("startup to ready: {:?}", t_start.elapsed());

    // player_snapshot 命令函数（与 UI invoke 走同一函数体）。
    let snap = match ipc::player_snapshot(handle.state::<AppState>()) {
        Ok(s) => s,
        Err(e) => fail(&format!("{e:?}")),
    };
    if !["idle", "stopped", "playing", "paused"].contains(&snap.state.as_str()) {
        fail(&format!("invalid state {}", snap.state));
    }

    // app_info 命令函数（无状态路径）。
    let info = match ipc::app_info() {
        Ok(v) => v,
        Err(e) => fail(&format!("{e:?}")),
    };
    if info["name"] != "ifuyo Aura" {
        fail("brand mismatch");
    }

    // library 打开且为空（临时目录）。
    let entries = handle
        .state::<AppState>()
        .library
        .service
        .lock()
        .expect("lib lock")
        .database()
        .connection()
        .query_row("SELECT count(*) FROM tracks", [], |r| r.get::<_, i64>(0))
        .unwrap_or(-1);
    if entries != 0 {
        fail(&format!("library not empty: {entries}"));
    }

    // 快照延迟（驱动线程 10Hz 上的读取成本）。
    let t0 = std::time::Instant::now();
    for _ in 0..100 {
        let _ = ipc::player_snapshot(handle.state::<AppState>()).unwrap();
    }
    println!("player_snapshot x100: {:?}/call", t0.elapsed() / 100);

    app.cleanup_before_exit();
    println!("SMOKE OK");

    let _ = std::fs::remove_dir_all(&tmp);
    std::process::exit(0);
}
