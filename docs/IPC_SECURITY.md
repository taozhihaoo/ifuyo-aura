# IPC Security（M12 §6/§63）

## 面（FACT）

50 个 typed IPC 命令（`#[tauri::command]` 宏 = 类型化反序列化，无 raw
JSON 参数）。清单见 ARCHITECTURE.md；枚举自 src-tauri/src/{ipc,library,
dreamscape,analysis_ipc,map_ipc,integration}.rs。

## 原则（§62/§64/§109）

- **最小权限**：capabilities 仅 `core:default` + 显式列出的
  window/webview/dialog/updater:default/process:allow-restart——无 `*`。
- **Rust 是安全权威**（§64）：前端只发请求；端点/签名/版本决定全在
  Rust/插件侧（updater 端点来自编译期配置，前端不可指定 §64）。
- **来源验证（§65）**：播放输入统一走 M4 resolver（scheme 白名单 →
  凭据拒绝 → 封锁表 → 私网策略）；前端永不 raw fetch。
- **失败关闭（§55）**：非法参数 → 命令 Err；路径不存在 → 拒绝；更新
  非法 → typed Failed。

## 分类核查（§38 IPC）

| 项 | 结果 |
| --- | --- |
| unauthorized command | capabilities 白名单外的 webview 调用被 core 拒绝 |
| invalid enum/JSON | serde 反序列化失败 → 命令错误（无 panic 路径；M10 §122） |
| invalid path | track_path 仅查库内 identity；artwork 仅 app_data 字节（§236） |
| invalid URL | play_input → M4 resolver 全链校验 |
| oversized input | resolver/command 层上限（play_input 4096 等 §54） |
| integer overflow | seek 帧换算 checked（§32）；时间线 u128→i64 上限钳制 |
| malformed JSON | invoke 层拒绝，无副作用 |

## 审计状态

STATICALLY VERIFIED（本文件为静态审计结论 + tests/security_m12.rs 回归锚定）。运行时 GUI 级 IPC 滥用验证 = MANUAL VERIFICATION REQUIRED。
