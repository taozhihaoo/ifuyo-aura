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
