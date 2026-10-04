//! 单实例（§107–§116/§230–§231）：官方 `tauri-plugin-single-instance`。
//! 第二实例启动 → 插件令其退出 → 本回调在既有实例触发 → 仅执行最小
//! 允许动作：show + restore + focus 主窗（§113/§114）。argv 一律忽略
//! （§115/§231 不接受第二实例任意命令/参数路由）。
//!
//! §116：插件必须先于 SMTC 注册（Builder 链首位），保证不会出现两个
//! 实例同时注册 SMTC 的窗口期。

use tauri::{AppHandle, Wry};

/// 第二实例允许的唯一动作（§231 最小动作面）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondInstanceAction {
    /// 仅 show+restore+focus 主窗（§113/§114）。
    FocusMainWindow,
}

/// §230/§231：argv/cwd **不解析、不路由、不执行**——任何参数（含恶意
/// 构造）都只产生同一最小动作。纯函数，mock 可测（§264）。
#[must_use]
pub fn second_instance_action(args: &[String], cwd: &str) -> SecondInstanceAction {
    let _ = (args, cwd);
    SecondInstanceAction::FocusMainWindow
}

/// 构造 single-instance 插件（注册顺序见 lib.rs）。
#[must_use]
pub fn plugin() -> tauri::plugin::TauriPlugin<Wry> {
    tauri_plugin_single_instance::init(|app: &AppHandle<Wry>, args, cwd| {
        // §230/§231：动作面收敛；参数被显式丢弃。
        let _ = second_instance_action(&args, &cwd);
        super::tray::focus_main(app);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §264/§231：任意 argv/cwd → 唯一 Focus 动作（无命令执行面）。
    #[test]
    fn second_instance_args_never_route() {
        let plain = ["ifuyo-aura.exe".to_string()];
        assert_eq!(
            second_instance_action(&plain, "C:/x"),
            SecondInstanceAction::FocusMainWindow
        );
        let malicious = [
            "ifuyo-aura.exe".to_string(),
            "--exec".to_string(),
            "rm -rf /".to_string(),
            r#"file:///C:/Windows/System32/calc.exe"#.to_string(),
            r#"{"cmd":"exit"}"#.to_string(),
        ];
        assert_eq!(
            second_instance_action(&malicious, "C:/"),
            SecondInstanceAction::FocusMainWindow
        );
        assert_eq!(
            second_instance_action(&[], ""),
            SecondInstanceAction::FocusMainWindow
        );
    }
}
