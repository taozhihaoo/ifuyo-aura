# DIAGNOSTICS（M12 §46/§227/§56）

> 原则：No telemetry / Local only / User initiated（§46）。
> 当前应用**不写任何运行时日志文件**；诊断全部按需、仅本地。

## What is logged（当前全部诊断面）

| 诊断 | 位置 | 内容 | 获取方式 |
| --- | --- | --- | --- |
| SMTC/Tray/Updater 健康 | 内存（get_integration_diagnostics） | smtc_available/smtc_state/last_revision/last_error(WinRT hr 文本)/tray_initialized | 设置页/IPC 查询（§225–§227） |
| Windows 集成错误 | 同上 last_error | WinRT 错误文本（无 secret） | 同上 |
| 启动耗时 | smoke 脚本输出 | startup-to-ready ms | `cargo run --bin smoke` |
| 更新检查时间 | webview localStorage | lastCheckAt 时间戳（§43 不含 token/URL 参数） | devtools |
| 原生验证证据 | `scripts/m10-native-verify.ps1` 输出 | M10-* 证据行 | 手动运行 |

## What is NOT logged

- 用户播放行为/曲目历史（§44/§130）
- audio 路径/ PCM/嵌入向量/分析数据（§237/§239）
- 任何 secret（token/密码/私钥——§56 安全事件只记 kind/reason/
  subsystem/timestamp，例如"updater signature mismatch"，不含凭据值）
- 无遥测端点、无上报（§44 grep 零命中）

## How to enable verbose logs

当前无 verbose 开关（应用零日志输出，§237）。开发诊断：
`tauri dev` 的控制台输出仅本地开发可见，不随产物分发。

## How to redact / export

- 诊断数据仅存在于进程内存（重启即清）；如需分享 IPC 查询结果，
  请人工检查后去除个人曲目名等字段（§132）。
- 更新失败的 `last_error` 仅含 WinRT/网络错误文本，无 URL 参数、
  无凭据（§43）。

## 安全事件记录（§56）

SECURITY.md 列举的拒绝类事件（无效 scheme/穿越/签名不符）当前策略：
拒绝并返回 typed 错误；**不写持久日志**（零日志=零泄露面）。如未来
引入本地安全日志，将遵循 §56（不记 secret 值）并在本文件更新。
