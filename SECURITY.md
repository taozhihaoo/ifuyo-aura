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

## Source / Network Security（M4 实装，与代码一致）

### URL Validation（url_policy.rs）
- 只允许 `http` / `https` scheme；`file:` `ftp:` `data:` `javascript:` `blob:` `ws:` 等一律拒绝。
- host 必须存在且非空；WHATWG 形态的畸形 authority 走完整校验链。
- **带凭据的 URL（user:password@host）默认拒绝**（§15），不进日志/UI/存储。

### Redirect Validation
- 上限 5 跳；**每一跳**重新执行 scheme/凭据/封锁平台/私网全套校验（§16）——首跳合法不等于目的地合法（有测试）。

### SSRF / Private Network Policy
- host 字符串层：`localhost`/`*.localhost`/`*.local` + IPv4/IPv6 私网/环回/链路本地/CGNAT/组播字面量默认拒绝；用户明确确认局域网来源后由应用层放开（`NetworkPolicy::allow_private`）。
- **DNS 层复检**（防 rebinding，§18）：自定义解析器把解析结果逐 IP 过私网检查，命中即整个解析失败（不给连接层任何可用地址；有 DNS-rebind 仿真测试）。
- 禁止内网探测/localhost 扫描：来源系统不存在任何主动扫描行为。

### Unsupported Platforms
- `config/unsupported_platforms.json` 命中 → resolve 前拒绝，**零请求**（DNS spy 测试断言 0 次解析，§125）。

### Response Size Limits
- 文本响应（JSON/RSS/m3u8/ICY 头）：8 MiB 上限（servers 64 KiB），Content-Length 预检 + 读取超限双保险（§21）。
- 音频流：有界内存缓冲（finite 1 MiB / live ≤30s 滚动），结构上排除无限累积（§29/§100）。

### ICY Limits
- metadata 块协议上限 4080 字节；畸形/非 UTF-8 块仅丢弃，音频继续（§33/§36）。

### XML Safety
- M4-A 不含 XML 解析（RSS 为 B 级）；将来实现必须禁用外部实体（XXE 不得可能）。

### Cancellation
- 所有网络请求可取消；取消后消费侧即时返回，feeder 线程不成为孤儿（D-052，有测试）。

### Logging
- URL 日志脱敏：仅 scheme://host/path，query 与 userinfo 剥离（`redact_url`，§96）。
