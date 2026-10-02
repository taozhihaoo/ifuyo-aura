# DECISIONS

工程决策记录。新增决策追加编号，不覆写既有条目；推翻旧决策时保留原文并追加新条目说明。

## D-001 Product Name

正式产品名：

`ifuyo Aura`

理由：

产品名称直接绑定 ifuyo 品牌与视觉核心 Aura，便于未来商业化。

> 2026-10-02 章程变更：产品名由 `IFUYO Music`（内部工程代号 `Aura`）正式更名为 `ifuyo Aura`。原内部代号并入正式产品名称，不再作为独立工程代号使用。

## D-002 Technology

Tauri 2 + Rust + React + TypeScript + Three.js。

理由：Charter §5 固定技术栈；Tauri 2 提供最小体积与原生集成，Rust 承载音频与领域内核。

## D-003 Platform

Windows-first。

理由：官方首发目标为 Windows（WASAPI / SMTC / Mica）；代码保持可移植性，但非 Windows 平台不承诺官方支持。

## D-004 Privacy

No telemetry。

理由：Charter §33 零遥测是产品身份的一部分，商业化也不能改变。

## D-005 Content

No commercial-platform extraction。

理由：合法性优先（Charter 产品原则第 1 条）。只接受本地与合法开放来源。

## D-006 Licensing

默认 MIT OR Apache-2.0 双许可。

理由：最大兼容未来商业发行与开源协作；依赖侧禁止 GPL / AGPL / LGPL（Charter §7）。

## D-007 Codec

官方发行版 codec 策略按总章程执行：优先 MP3 / FLAC / Vorbis / WAV-PCM / OGG / MKV-WebM / ALAC；AAC 默认经 Windows Media Foundation，官方构建不内置 AAC decoder，自行编译可启用 `aac-builtin` feature。

## D-008 Website

备案信息不属于 ifuyo Aura 发布页要求，不实现相关字段或展示。

## D-009 Scope

M0→M14，逐阶段实施；一次只做一个 Milestone，门禁全绿才 commit。

## D-010 src-tauri 接线时机（2026-10-02）

`src-tauri/` 在 Phase 0 仅建立脚手架（目录、`tauri.conf.json`、入口源码），**暂不加入 Cargo workspace members、不引入 tauri 依赖编译**。

理由：Phase 0 禁止实现 M1 之后业务功能，且本机 Rust 工具链未安装；提前把 tauri 拉进 workspace 会让 `cargo test` 拖入数百个依赖，违背"不得添加未必要的生产依赖"（§18）。M0 建立 IPC contracts 时再把 `src-tauri` 加入 members 并锁定 Tauri 2 版本。

## D-011 MSRV 与本机工具链（2026-10-02 建立，2026-10-03 更新）

- **工具链（2026-10-03 就绪）**：本机安装 VS Build Tools 2022（MSVC 14.44）+ rustup stable `x86_64-pc-windows-msvc`（rustc 1.99.0），`rust-toolchain.toml` 锁定 channel=stable，与 CI windows-latest 目标一致。首次 `cargo fmt --check` / `clippy -D warnings` / `test --workspace` 已全绿。
- **MSRV**：声明为 `1.85`（edition 2021 + workspace lints 需 1.74+，留验证余量）。依赖锁定（M0-2）若抬高低限，将更新此处与 `Cargo.toml` 的 `rust-version`。
- 备注：Windows-first 项目固定 MSVC 目标；不使用 windows-gnu。

## D-012 品牌色进入 brand.json（2026-10-02）

`brand.json` 除 Phase 0 §3 的四个必填字段外，附加 `version` / `copyright` / `colors`（bg、rose、violet、sky、mint），以满足总章程 §4.1 对 brand.json 的完整要求；`ui/src/design/tokens.ts` 与生成的 CSS variables 均以 `brand.json` 为单一事实来源，`scripts/generate-tokens.mjs` 负责生成与一致性校验。
