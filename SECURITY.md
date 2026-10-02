# SECURITY

> 安全基线。随里程碑落地逐步具体化；实现与本文冲突时，实现必须改。

## Secret Handling

- 仓库不得包含任何真实 API key、密码、令牌、证书私钥、签名私钥。
- 提交前审计清单：`api_key`、`apikey`、`client_secret`、`password`、`token`、`private_key`、`BEGIN PRIVATE KEY`、`.env`。
- 签名/更新私钥只允许存在于 CI Secret（见 [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md)）。
- 应用凭据只进 Windows Credential Manager（见 [PRIVACY.md](PRIVACY.md)）。

## Dependency Review

新增依赖前必须确认（与 Charter §20 一致）：必要性、许可证（禁 GPL/AGPL/LGPL）、维护状态、Windows 兼容、CI 可构建、架构耦合。`cargo deny` 与前端 license 检查在 CI 强制执行。

## Unsafe Rust Review

- 音频 callback 内禁止 allocation / lock / logging / syscall / blocking / IO / panic（Charter §8.2）。
- 任何 `unsafe` 块必须：有注释说明不变量、有测试覆盖、可在 review 中追溯；目标是 unsafe 面积最小化（ring buffer、SIMD 等少数热点）。

## IPC Validation

- Tauri command 走 allowlist；capabilities 最小化。
- 所有 IPC 输入必须校验（类型、范围、长度）；不可信输入不得直达领域层。
- CSP 禁止 remote scripts / styles / fonts / images。

## Path Traversal Protection

用户提供的路径必须规范化并限定在其授权根目录内；文件/目录访问不得被 `..`、符号链接或 UNC 形式逃逸出授权范围。

## Network URL Validation

- 仅允许 http/https scheme；最大 5 次重定向；connect/read 超时；响应体大小上限；描述性 User-Agent。
- `config/unsupported_platforms.json` 命中的平台直接拒绝、不发请求。
- 首次访问非公网地址需用户一次性确认。

## Credential Handling

Subsonic 等服务凭据只能存入 Windows Credential Manager；不明文写盘、不进日志、不进备份。

## Logging Redaction

`tracing` 日志在写入前过滤：password、token、API key、凭据、完整 URL 查询参数（含 key 的）一律脱敏。

## 问题反馈

当前为 pre-alpha：请通过 GitHub 仓库的 private security advisory（Security 标签页）报告安全问题，不要在公开 issue 中披露细节。
