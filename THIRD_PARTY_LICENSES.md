# THIRD_PARTY_LICENSES

> Dependency license inventory will be generated after dependency resolution.

本文件当前**不包含**任何依赖许可证清单——Phase 0 不虚构不存在的依赖列表。

## 生成机制

依赖首次解析后（M0 起）：

1. **Rust 侧**：`cargo deny check` 读取 `deny.toml`，对全部 crate 依赖生成/校验许可证清单与禁用列表；结果由 CI 强制。
2. **前端侧**：对 `ui/package.json` 的依赖树运行许可证扫描（如 license-checker / npm ls 管线），禁止 GPL / AGPL / LGPL。
3. 清单（依赖名、版本、许可证、项目地址）写入本文件或生成物，并由 CI 校验与真实依赖树一致（Charter §52-3）。

## 允许的许可证（Charter §7）

MIT、Apache-2.0、BSD-2-Clause、BSD-3-Clause、ISC、MPL-2.0、Zlib、CC0-1.0、Unicode-3.0、CDLA-Permissive-2.0、OFL-1.1。

## 禁止的许可证

GPL、AGPL、LGPL（含其衍生形式）。新增任何其他许可证类型必须先记录到 [DECISIONS.md](DECISIONS.md)。

## 本项目自身许可证

MIT OR Apache-2.0，见 [LICENSE-MIT](LICENSE-MIT) / [LICENSE-APACHE](LICENSE-APACHE)。

## M8（2026-10-05）

M8 新增运行时依赖：**无第三方 DSP/FFT/ML runtime/模型**（D-095：自有 radix-2 FFT、自有谱统计嵌入）。新增 crates 均为本仓库 workspace 内（aura-analysis）或已在上篇记录的 serde/siphasher/rusqlite/aura-core（MIT/Apache-2.0）。测试 fixture 全部程序生成（§261 无第三方音频素材）。无隐藏模型许可（§255）。

## M9（2026-10-05）

M9 新增第三方依赖：**siphasher 1.x**（MIT OR Apache-2.0；dataset hash 用，M9 上篇引入、Cargo.lock 锁定；deny bans/licenses/sources 全绿）。M9 下篇零新增依赖（§494/§495：渲染用平台 Canvas2D，无第二渲染库、无大型图框架）。全部其余 crates 均为本仓库 workspace 内（aura-map 等）或此前已记录的 serde/serde_json/rusqlite/tauri 栈（MIT/Apache-2.0）。
## M10（2026-10-05）

M10 新增第三方依赖：**windows 0.62**（MIT OR Apache-2.0；仅 4 个最小 feature 面——Media/Storage_Streams/Win32_Foundation/Win32_System_WinRT + windows-future 0.3 传递，§16/§237 审计通过：MSRV 兼容、官方维护、Windows 原生 API 绑定）；**tauri-plugin-single-instance 2**（MIT OR Apache-2.0，Tauri 官方插件 §238）；tauri 核心加 tray-icon/image-png feature（同 crate，无新依赖）。无 GPL/AGPL/LGPL（§379）。