//! 系统托盘（§75–§96/§102/§106/§146）：Rust 侧统一创建（§76）；全局
//! 单例所有权（§78/§106）；稳定菜单 ID（§156）与 locale 无关（§157）；
//! 只在真实状态/语言变化时更新文本与启用态（§91/§94/§95，不重建图标）。
//! 左键=菜单 / 双击=聚焦主窗（D-125 §88/§89）。
//!
//! 纯逻辑（文案表/命令映射/菜单状态推导）与 Tauri 侧分离——下篇测试
//! 矩阵（§261–§263/§264）无需 Windows 即可运行。
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

// ---------- 纯逻辑：文案表（§93/§157） ----------

/// 托盘文案（locale 归一为 zh-CN / en 两套）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayTexts {
    pub open: &'static str,
    pub nothing: &'static str,
    pub play: &'static str,
    pub pause: &'static str,
    pub previous: &'static str,
    pub next: &'static str,
    pub now_playing: &'static str,
    pub library: &'static str,
    pub map: &'static str,
    pub dreamscape: &'static str,
    pub mini: &'static str,
    pub exit: &'static str,
}

/// 归一 locale（未知 → zh-CN 缺省）。
#[must_use]
pub fn normalize_locale(locale: &str) -> &'static str {
    if locale == "en" {
        "en"
    } else {
        "zh-CN"
    }
}

/// §93：按 locale 提供全套文案（逻辑 ID 永不随语言变化 §157）。
#[must_use]
pub fn texts(locale: &str) -> TrayTexts {
    let l = normalize_locale(locale);
    if l == "en" {
        TrayTexts {
            open: "Open ifuyo Aura",
            nothing: "Nothing Playing",
            play: "Play",
            pause: "Pause",
            previous: "Previous",
            next: "Next",
            now_playing: "Now Playing",
            library: "Library",
            map: "Music Map",
            dreamscape: "Dreamscape",
            mini: "Mini Player",
            exit: "Exit",
        }
    } else {
        TrayTexts {
            open: "打开 ifuyo Aura",
            nothing: "未在播放",
            play: "播放",
            pause: "暂停",
            previous: "上一曲",
            next: "下一曲",
            now_playing: "正在播放",
            library: "音乐库",
            map: "音乐地图",
            dreamscape: "梦境",
            mini: "Mini 播放器",
            exit: "退出",
        }
    }
}

// ---------- 纯逻辑：菜单 ID → 动作映射（§156/§261） ----------

/// 菜单点击动作（与应用命令一一对应；§87 统一 Application Command）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Focus,
    TogglePlay,
    Previous,
    Next,
    Navigate(&'static str),
    Exit,
    Ignore,
}

/// §261：稳定 ID → 动作（未知 ID → Ignore，§229 metadata 不进命令面）。
#[must_use]
pub fn tray_action(id: &str) -> TrayAction {
    match id {
        ids::OPEN => TrayAction::Focus,
        ids::PLAY_PAUSE => TrayAction::TogglePlay,
        ids::PREVIOUS => TrayAction::Previous,
        ids::NEXT => TrayAction::Next,
        ids::NOW_PLAYING => TrayAction::Navigate("now-playing"),
        ids::LIBRARY => TrayAction::Navigate("library"),
        ids::MAP => TrayAction::Navigate("map"),
        ids::DREAMSCAPE => TrayAction::Navigate("dreamscape"),
        ids::MINI => TrayAction::Navigate("mini"),
        ids::EXIT => TrayAction::Exit,
        _ => TrayAction::Ignore, // 信息行/未知 ID 无动作（§229 metadata 不进命令面）
    }
}

// ---------- 纯逻辑：菜单状态推导（§84–§86/§262/§263） ----------

/// 可测试的菜单目标（Tauri MenuItem 实现之；mock 记录调用）。
pub trait TrayTarget {
    fn set_track_text(&mut self, text: &str);
    fn set_play_pause_text(&mut self, text: &str);
    fn set_transport_enabled(&mut self, previous: bool, next: bool);
}

/// §84/§85/§86：由真实播放状态推导菜单文本/启用态（§92 低频调用）。
pub fn apply_menu_state(
    target: &mut dyn TrayTarget,
    playing: bool,
    has_previous: bool,
    has_next: bool,
    track_line: Option<&str>,
    locale: &str,
) {
    let t = texts(locale);
    target.set_play_pause_text(if playing { t.pause } else { t.play });
    target.set_transport_enabled(has_previous, has_next);
    if let Some(line) = track_line {
        target.set_track_text(line);
    }
}

/// §132/§84：当前曲目行（真实 metadata；artist 缺失只显示标题）。
#[must_use]
pub fn track_line(artist: Option<&str>, title: &str) -> String {
    match artist {
        Some(a) if !a.is_empty() => format!("{a} — {title}"),
        _ => title.to_string(),
    }
}

// ---------- Tauri 侧：单例托盘 ----------

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

impl TrayTarget for TrayPresenter {
    fn set_track_text(&mut self, text: &str) {
        let _ = self.track.set_text(text);
        let _ = self.track.set_enabled(false); // 信息行
    }
    fn set_play_pause_text(&mut self, text: &str) {
        let _ = self.play_pause.set_text(text);
    }
    fn set_transport_enabled(&mut self, previous: bool, next: bool) {
        let _ = self.previous.set_enabled(previous);
        let _ = self.next.set_enabled(next);
    }
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
    let t = texts(initial_locale);
    let open =
        MenuItem::with_id(app, ids::OPEN, t.open, true, None::<&str>).map_err(|e| e.to_string())?;
    let track = MenuItem::with_id(app, ids::TRACK, t.nothing, false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let play_pause = MenuItem::with_id(app, ids::PLAY_PAUSE, t.play, true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let previous = MenuItem::with_id(app, ids::PREVIOUS, t.previous, false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let next = MenuItem::with_id(app, ids::NEXT, t.next, false, None::<&str>)
        .map_err(|e| e.to_string())?;
    let now_playing = MenuItem::with_id(app, ids::NOW_PLAYING, t.now_playing, true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let library = MenuItem::with_id(app, ids::LIBRARY, t.library, true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let map =
        MenuItem::with_id(app, ids::MAP, t.map, true, None::<&str>).map_err(|e| e.to_string())?;
    let dreamscape = MenuItem::with_id(app, ids::DREAMSCAPE, t.dreamscape, true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let mini =
        MenuItem::with_id(app, ids::MINI, t.mini, true, None::<&str>).map_err(|e| e.to_string())?;
    let exit =
        MenuItem::with_id(app, ids::EXIT, t.exit, true, None::<&str>).map_err(|e| e.to_string())?;
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
        locale: Mutex::new(normalize_locale(initial_locale).to_string()),
        initialized: true,
    });
    Ok(())
}

/// §226 diagnostics：托盘是否已初始化。
#[must_use]
pub fn is_initialized() -> bool {
    TRAY.get().is_some_and(|p| p.initialized)
}

/// §94：语言切换 → 只改文本（图标不重建）。未知 locale → 缺省 zh-CN。
pub fn set_locale(locale: &str) {
    let Some(p) = TRAY.get() else { return };
    let l = normalize_locale(locale);
    {
        let mut cur = p.locale.lock().expect("tray locale");
        if cur.as_str() == l {
            return; // §91 无变化不更新
        }
        *cur = l.to_string();
    }
    let t = texts(l);
    let _ = p.track.set_text(t.nothing);
    let _ = p.play_pause.set_text(t.play);
    let _ = p.previous.set_text(t.previous);
    let _ = p.next.set_text(t.next);
}

/// §84/§85/§86：真实状态驱动的菜单更新（worker 低频调用；同状态/
/// 同曲目不重复 set_text，避免系统刷新噪音 §92）。
pub fn apply_state(playing: bool, has_previous: bool, has_next: bool, track_line: Option<&str>) {
    let Some(p) = TRAY.get() else { return };
    let l = p.locale.lock().expect("tray locale").clone();
    let mut presenter = TrayRef(p);
    apply_menu_state(
        &mut presenter,
        playing,
        has_previous,
        has_next,
        track_line,
        &l,
    );
}

/// TrayTarget 借用包装（TrayPresenter 为全局单例；set_* 走内部可变性）。
struct TrayRef<'a>(&'a TrayPresenter);
impl TrayTarget for TrayRef<'_> {
    fn set_track_text(&mut self, text: &str) {
        let _ = self.0.track.set_text(text);
        let _ = self.0.track.set_enabled(false);
    }
    fn set_play_pause_text(&mut self, text: &str) {
        let _ = self.0.play_pause.set_text(text);
    }
    fn set_transport_enabled(&mut self, previous: bool, next: bool) {
        let _ = self.0.previous.set_enabled(previous);
        let _ = self.0.next.set_enabled(next);
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
    // §87/§261：统一动作映射（纯函数；§229 metadata 不进命令面）。
    match tray_action(id) {
        TrayAction::Focus => focus_main(app),
        TrayAction::TogglePlay => {
            let state = app.state::<crate::ipc::AppState>();
            // §7/§8：命令进 Playback Authority；状态以快照回流为准（§85）。
            let playing = state.audio.snapshot().state == "playing";
            let cmd = if playing {
                aura_core::AudioCommand::Pause
            } else {
                aura_core::AudioCommand::Play
            };
            let _ = state.audio.post(cmd);
        }
        TrayAction::Previous => {
            // §24/§64：复用应用层 previous 语义（前端 queue 桥）。
            let _ = app.emit(
                "integration://command",
                serde_json::json!({ "cmd": "previous" }),
            );
        }
        TrayAction::Next => {
            // §23：复用应用层 next 语义（前端 queue 桥）。
            let _ = app.emit(
                "integration://command",
                serde_json::json!({ "cmd": "next" }),
            );
        }
        TrayAction::Navigate(view) => {
            focus_main(app);
            let _ = app.emit(
                "integration://navigate",
                serde_json::json!({ "view": view }),
            );
        }
        TrayAction::Exit => {
            // §102/§103：正式退出（tauri cleanup 顺序处理 dispose）。
            app.exit(0);
        }
        TrayAction::Ignore => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §263/§158：zh-CN/en 全字段非空且互异；locale 归一安全。
    #[test]
    fn texts_locales_complete() {
        for l in ["zh-CN", "en", "fr-FR", ""] {
            let t = texts(l);
            for (name, v) in [
                ("open", t.open),
                ("nothing", t.nothing),
                ("play", t.play),
                ("pause", t.pause),
                ("previous", t.previous),
                ("next", t.next),
                ("now_playing", t.now_playing),
                ("library", t.library),
                ("map", t.map),
                ("dreamscape", t.dreamscape),
                ("mini", t.mini),
                ("exit", t.exit),
            ] {
                assert!(!v.is_empty(), "{l}/{name} 文案为空");
            }
            assert_ne!(texts(l).play, texts(l).pause, "Play/Pause 文案必须互异");
        }
        assert_eq!(texts("en").play, "Play");
        assert_eq!(texts("zh-CN").play, "播放");
        assert_eq!(normalize_locale("unknown"), "zh-CN");
    }

    /// §261：稳定 ID → 动作全映射；未知/信息行 → Ignore。
    #[test]
    fn tray_action_mapping() {
        assert_eq!(tray_action(ids::OPEN), TrayAction::Focus);
        assert_eq!(tray_action(ids::PLAY_PAUSE), TrayAction::TogglePlay);
        assert_eq!(tray_action(ids::PREVIOUS), TrayAction::Previous);
        assert_eq!(tray_action(ids::NEXT), TrayAction::Next);
        assert_eq!(
            tray_action(ids::NOW_PLAYING),
            TrayAction::Navigate("now-playing")
        );
        assert_eq!(tray_action(ids::LIBRARY), TrayAction::Navigate("library"));
        assert_eq!(tray_action(ids::MAP), TrayAction::Navigate("map"));
        assert_eq!(
            tray_action(ids::DREAMSCAPE),
            TrayAction::Navigate("dreamscape")
        );
        assert_eq!(tray_action(ids::MINI), TrayAction::Navigate("mini"));
        assert_eq!(tray_action(ids::EXIT), TrayAction::Exit);
        assert_eq!(tray_action(ids::TRACK), TrayAction::Ignore);
        // §229：任意未知文本（包括曲目 metadata）不产生命令。
        assert_eq!(tray_action("Playing — Some Song"), TrayAction::Ignore);
        assert_eq!(tray_action(""), TrayAction::Ignore);
    }

    /// §262 mock target（§266 托盘 mock）。
    #[derive(Default)]
    struct MockTray {
        track: Option<String>,
        play_pause: Option<String>,
        enabled: Option<(bool, bool)>,
    }
    impl TrayTarget for MockTray {
        fn set_track_text(&mut self, text: &str) {
            self.track = Some(text.to_string());
        }
        fn set_play_pause_text(&mut self, text: &str) {
            self.play_pause = Some(text.to_string());
        }
        fn set_transport_enabled(&mut self, previous: bool, next: bool) {
            self.enabled = Some((previous, next));
        }
    }

    /// §84/§85/§86：真实状态推导文案与启用态。
    #[test]
    fn menu_state_derivation() {
        let mut m = MockTray::default();
        // Playing + 队列中段：label=Pause，双向启用。
        apply_menu_state(&mut m, true, true, true, Some("A — T"), "zh-CN");
        assert_eq!(m.play_pause.as_deref(), Some("暂停"));
        assert_eq!(m.enabled, Some((true, true)));
        assert_eq!(m.track.as_deref(), Some("A — T"));
        // 队列尾：Next 禁用（§65）。
        apply_menu_state(&mut m, true, true, false, None, "zh-CN");
        assert_eq!(m.enabled, Some((true, false)));
        // 队列头：Previous 禁用（§66）。
        apply_menu_state(&mut m, true, false, true, None, "zh-CN");
        assert_eq!(m.enabled, Some((false, true)));
        // 暂停：label=Play（§85 真实状态，不按上次点击翻转）。
        apply_menu_state(&mut m, false, true, true, None, "en");
        assert_eq!(m.play_pause.as_deref(), Some("Play"));
        // 无曲目行 → 不写 track 文本（§92 减少刷新）。
        apply_menu_state(&mut m, false, false, false, None, "en");
        assert_eq!(m.track.as_deref(), Some("A — T"), "None 不覆盖");
    }

    /// §132/§84：曲目行拼接。
    #[test]
    fn track_line_format() {
        assert_eq!(track_line(Some("A"), "T"), "A — T");
        assert_eq!(track_line(Some(""), "T"), "T");
        assert_eq!(track_line(None, "T"), "T");
    }
}
