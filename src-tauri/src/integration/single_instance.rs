//! 单实例（§107–§116/§230–§231）：官方 `tauri-plugin-single-instance`。
//! 第二实例启动 → 插件令其退出 → 本回调在既有实例触发 → 仅执行最小
//! 允许动作：show + restore + focus 主窗（§113/§114）。argv 一律忽略
//! （§115/§231 不接受第二实例任意命令/参数路由）。
//!
//! §116：插件必须先于 SMTC 注册（Builder 链首位），保证不会出现两个
//! 实例同时注册 SMTC 的窗口期。

use tauri::{AppHandle, Wry};

/// 构造 single-instance 插件（注册顺序见 lib.rs）。
#[must_use]
pub fn plugin() -> tauri::plugin::TauriPlugin<Wry> {
    tauri_plugin_single_instance::init(|app: &AppHandle<Wry>, _args, _cwd| {
        // §230/§231：args/cwd 不路由、不解析、不执行。
        super::tray::focus_main(app);
    })
}
