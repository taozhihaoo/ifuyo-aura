//! 系统托盘（§75–§96/§102/§106/§146）：Rust 侧统一创建（§76）；全局
//! 单例所有权（§78/§106）；稳定菜单 ID（§156）与 locale 无关（§157）；
//! 只在真实状态/语言变化时更新文本与启用态（§91/§94/§95，不重建图标）。
//! 左键=菜单 / 双击=聚焦主窗（D-125 §88/§89）。
//!
//! 失败隔离（§120）：托盘创建失败只记录 diagnostics，不终止应用。

use std::sync::Mutex;
use std::sync::OnceLock;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

/// 稳定菜单命令 ID（§156：显示文本永不作为逻辑 ID）。
pub mod ids {
    pub const OPEN: &str = "tray.open";
    pub const TRACK: &str = "tray.track";
    pub const PLAY_PAUSE: &str = "tray.playPause";
    pub const PREVIOUS: &str = "tray.previous";
    pub const NEXT: &str = "tray.next";
    pub const NOW_PLAYING: &str = "tray.nowPlaying";
    pub const LIBRARY: &str = "tray.library";
    pub const MAP: &str = "tray.map";
    pub const DREAMSCAPE: &str = "tray.dreamscape";
    pub const MINI: &str = "tray.miniPlayer";
    pub const EXIT: &str = "tray.exit";
}

/// §93 托盘文案表（zh-CN / en；跟随应用语言）。
struct Texts;
impl Texts {
    fn open(l: &str) -> &'static str {
        if l == "en" {
            "Open ifuyo Aura"
        } else {
            "打开 ifuyo Aura"
        }
    }
    fn nothing(l: &str) -> &'static str {
        if l == "en" {
            "Nothing Playing"
        } else {
            "未在播放"
        }
    }
    fn play(l: &str) -> &'static str {
        if l == "en" {
            "Play"
        } else {
            "播放"
        }
    }
    fn pause(l: &str) -> &'static str {
        if l == "en" {
            "Pause"
        } else {
            "暂停"
        }
    }
    fn previous(l: &str) -> &'static str {
        if l == "en" {
            "Previous"
        } else {
            "上一曲"
        }
    }
    fn next(l: &str) -> &'static str {
        if l == "en" {
            "Next"
        } else {
            "下一曲"
        }
    }
    fn now_playing(l: &str) -> &'static str {
        if l == "en" {
            "Now Playing"
        } else {
            "正在播放"
        }
    }
    fn library(l: &str) -> &'static str {
        if l == "en" {
            "Library"
        } else {
            "音乐库"
        }
    }
    fn map(l: &str) -> &'static str {
        if l == "en" {
            "Music Map"
        } else {
            "音乐地图"
        }
    }
    fn dreamscape(l: &str) -> &'static str {
        if l == "en" {
            "Dreamscape"
        } else {
            "梦境"
        }
    }
    fn mini(l: &str) -> &'static str {
        if l == "en" {
            "Mini Player"
        } else {
            "Mini 播放器"
        }
    }
    fn exit(l: &str) -> &'static str {
        if l == "en" {
            "Exit"
        } else {
            "退出"
        }
    }
}

/// 托盘单例句柄（§106 singleton ownership；跨线程可见）。
pub struct TrayPresenter {
    pub track: MenuItem<Wry>,
    pub play_pause: MenuItem<Wry>,
    pub previous: MenuItem<Wry>,
    pub next: MenuItem<Wry>,
    locale: Mutex<String>,
    /// §225/§226 本地 diagnostics（§227 不作为 source of truth）。
    pub initialized: bool,
}

static TRAY: OnceLock<TrayPresenter> = OnceLock::new();

/// 初始化托盘（应用 setup 调用一次；失败 = unavailable，不 panic §122）。
///
/// # Errors
/// 菜单/托盘创建失败 → [`String`]（调用方记录 diagnostics 并继续）。
pub fn create(app: &AppHandle, initial_locale: &str) -> Result<(), String> {
    if TRAY.get().is_some() {
        return Ok(()); // §106 禁止重复创建
    }
    let l = initial_locale;
    let open = MenuItem::with_id(app, ids::OPEN, Texts::open(l), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let track = MenuItem::with_id(app, ids::TRACK, Texts::nothing(l), false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let play_pause = MenuItem::with_id(app, ids::PLAY_PAUSE, Texts::play(l), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let previous = MenuItem::with_id(app, ids::PREVIOUS, Texts::previous(l), false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let next = MenuItem::with_id(app, ids::NEXT, Texts::next(l), false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let now_playing = MenuItem::with_id(
        app,
        ids::NOW_PLAYING,
        Texts::now_playing(l),
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let library = MenuItem::with_id(app, ids::LIBRARY, Texts::library(l), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let map = MenuItem::with_id(app, ids::MAP, Texts::map(l), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let dreamscape = MenuItem::with_id(
        app,
        ids::DREAMSCAPE,
        Texts::dreamscape(l),
        true,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let mini = MenuItem::with_id(app, ids::MINI, Texts::mini(l), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let exit = MenuItem::with_id(app, ids::EXIT, Texts::exit(l), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let sep1 = tauri::menu::PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sep2 = tauri::menu::PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sep3 = tauri::menu::PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sep4 = tauri::menu::PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;

    let menu = Menu::with_items(
        app,
        &[
            &open,
            &sep1,
            &track,
            &play_pause,
            &previous,
            &next,
            &sep2,
            &now_playing,
            &library,
            &map,
            &dreamscape,
            &mini,
            &sep3,
            &exit,
            &sep4,
        ],
    )
    .map_err(|e| e.to_string())?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../../icons/tray.png"))
        .map_err(|e| e.to_string())?
        .to_owned();

    let _tray = TrayIconBuilder::with_id("aura-tray")
        .icon(icon)
        .tooltip("ifuyo Aura") // §81 静态 tooltip（不随歌曲刷新）
        .menu(&menu)
        .show_menu_on_left_click(true) // D-125 §89 左键=菜单
        .on_menu_event(|app, event| handle_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            // §88 双击 = 聚焦主窗（隐藏时 show+focus）。
            if let TrayIconEvent::DoubleClick { .. } = event {
                focus_main(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    let _ = TRAY.set(TrayPresenter {
        track,
        play_pause,
        previous,
        next,
        locale: Mutex::new(initial_locale.to_string()),
        initialized: true,
    });
    Ok(())
}

/// §226 diagnostics：托盘是否已初始化。
#[must_use]
pub fn is_initialized() -> bool {
    TRAY.get().is_some_and(|p| p.initialized)
}

/// §94：语言切换 → 只改文本（图标不重建）。未知 locale → 保持现状。
pub fn set_locale(locale: &str) {
    let Some(p) = TRAY.get() else { return };
    let l = if locale == "en" {
        "en"
    } else if locale == "zh-CN" {
        "zh-CN"
    } else {
        return;
    };
    let mut cur = p.locale.lock().expect("tray locale");
    if *cur == l {
        return; // §91 无变化不更新
    }
    *cur = l.to_string();
    drop(cur);
    let _ = p.track.set_text(Texts::nothing(l));
    let _ = p.play_pause.set_text(Texts::play(l));
    let _ = p.previous.set_text(Texts::previous(l));
    let _ = p.next.set_text(Texts::next(l));
}

/// §84/§85/§86：真实状态驱动的菜单更新（由 integration worker 低频调用；
/// 同状态/同曲目不重复 set_text，避免系统刷新噪音 §92）。
pub fn apply_state(playing: bool, has_previous: bool, has_next: bool, track_line: Option<&str>) {
    let Some(p) = TRAY.get() else { return };
    let l = p.locale.lock().expect("tray locale").clone();
    let label = if playing {
        Texts::pause(&l)
    } else {
        Texts::play(&l)
    };
    let _ = p.play_pause.set_text(label);
    let _ = p.previous.set_enabled(has_previous);
    let _ = p.next.set_enabled(has_next);
    if let Some(line) = track_line {
        let _ = p.track.set_text(line);
        let _ = p.track.set_enabled(false); // 信息行
    }
}

/// §132/§84：当前曲目行（真实 metadata；无曲目 = 空状态文案）。
#[must_use]
pub fn track_line(artist: Option<&str>, title: &str) -> String {
    match artist {
        Some(a) if !a.is_empty() => format!("{a} — {title}"),
        _ => title.to_string(),
    }
}

/// §167/§113：show + unminimize + set_focus（幂等 §169）。
pub fn focus_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn handle_menu(app: &AppHandle, id: &str) {
    let state = app.state::<crate::ipc::AppState>();
    match id {
        ids::OPEN => focus_main(app),
        ids::PLAY_PAUSE => {
            // §7/§8：命令进 Playback Authority；状态以快照回流为准（§85）。
            let playing = state.audio.snapshot().state == "playing";
            let cmd = if playing {
                aura_core::AudioCommand::Pause
            } else {
                aura_core::AudioCommand::Play
            };
            let _ = state.audio.post(cmd);
        }
        ids::PREVIOUS => {
            // §24/§64：复用应用层 previous 语义（前端 queue 桥）。
            let _ = app.emit(
                "integration://command",
                serde_json::json!({ "cmd": "previous" }),
            );
        }
        ids::NEXT => {
            // §23：复用应用层 next 语义（前端 queue 桥）。
            let _ = app.emit(
                "integration://command",
                serde_json::json!({ "cmd": "next" }),
            );
        }
        ids::NOW_PLAYING | ids::LIBRARY | ids::MAP | ids::DREAMSCAPE | ids::MINI => {
            let view = match id {
                ids::NOW_PLAYING => "now-playing",
                ids::LIBRARY => "library",
                ids::MAP => "map",
                ids::DREAMSCAPE => "dreamscape",
                _ => "mini",
            };
            focus_main(app);
            let _ = app.emit(
                "integration://navigate",
                serde_json::json!({ "view": view }),
            );
        }
        ids::EXIT => {
            // §102/§103：正式退出（tauri cleanup 顺序处理 dispose）。
            app.exit(0);
        }
        _ => {}
    }
}
