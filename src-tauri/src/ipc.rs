//! Typed IPC contracts。
//!
//! M0 范围：最小契约（app_info）。M1 起按"向 M6 只留最小 Tauri command 契约"
//! 的纪律逐个里程碑追加；DTO 命名在 Rust 侧 snake_case 字段 + camelCase 序列化，
//! 与 ui/src/ipc 的 TS 镜像一一对应。

use serde::{Deserialize, Serialize};

/// brand.json 在编译期内嵌（单一品牌事实来源），运行时不需要额外文件。
const BRAND_JSON: &str = include_str!("../../brand.json");

#[derive(Debug, Deserialize)]
struct Brand {
    name: String,
    version: String,
    slogan: String,
}

/// 应用基础信息 DTO（ui/src/ipc/appInfo.ts 镜像此结构）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub slogan: String,
    pub phase: String,
}

/// 读取内嵌品牌信息。brand.json 损坏属编译期即可发现的仓库错误，
/// 运行时以显式错误返回，不 panic。
#[tauri::command]
pub fn app_info() -> Result<AppInfo, String> {
    let brand: Brand = serde_json::from_str(BRAND_JSON)
        .map_err(|e| format!("embedded brand.json is invalid: {e}"))?;
    Ok(AppInfo {
        name: brand.name,
        version: brand.version,
        slogan: brand.slogan,
        phase: "M0".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reflects_brand_json() {
        let info = app_info().expect("brand.json must parse");
        assert_eq!(info.name, "ifuyo Aura");
        assert_eq!(info.version, "0.1.0");
        assert_eq!(info.slogan, "以光与梦，编织每一次游玩。");
    }

    #[test]
    fn app_info_serializes_camel_case() {
        let info = app_info().unwrap();
        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("name").is_some());
        assert!(json.get("version").is_some());
        assert!(json.get("slogan").is_some());
        assert!(json.get("phase").is_some());
    }
}
