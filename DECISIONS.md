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

## D-010 src-tauri 接线时机（2026-10-02 建立，2026-10-03 已执行）

~~`src-tauri/` 在 Phase 0 仅建立脚手架（目录、`tauri.conf.json`、入口源码），**暂不加入 Cargo workspace members、不引入 tauri 依赖编译**。~~

**已落地（M0-3，2026-10-03）**：`src-tauri` 加入 workspace members，tauri 2.12 依赖树经 `cargo check/clippy/test` 全绿。占位图标集（`src-tauri/icons/`，品牌色占位图形）由 `@tauri-apps/cli icon` 生成，正式品牌 logo 按 BRAND.md 在 M6 前替换。`ui/dist/.gitkeep` 保证 fresh clone 上 `frontendDist` 路径存在。完整 `tauri build` 打包门禁仍待 UI（M6）激活。

理由（原始）：Phase 0 禁止实现 M1 之后业务功能，且本机 Rust 工具链未安装；提前把 tauri 拉进 workspace 会让 `cargo test` 拖入数百个依赖，违背"不得添加未必要的生产依赖"（§18）。M0 建立 IPC contracts 时再把 `src-tauri` 加入 members 并锁定 Tauri 2 版本。

## D-011 MSRV 与本机工具链（2026-10-02 建立，2026-10-03 更新）

- **工具链（2026-10-03 就绪）**：本机安装 VS Build Tools 2022（MSVC 14.44）+ rustup stable `x86_64-pc-windows-msvc`（rustc 1.99.0），`rust-toolchain.toml` 锁定 channel=stable，与 CI windows-latest 目标一致。首次 `cargo fmt --check` / `clippy -D warnings` / `test --workspace` 已全绿。
- **MSRV**：`1.87`（2026-10-03 以真实 1.87 工具链全 workspace check 验证通过；由 rubato 5.0.1 抬高，最初声明的 1.85 不成立）。

## D-013 M1 音频依赖锁定（2026-10-03）

`aura-core` 锁定 M1 四件套（crates.io 当前稳定版）：

| crate | 版本 | 用途 |
|---|---|---|
| `symphonia` | 0.6.1 | 解码；默认特性全开，覆盖 Charter §36 格式矩阵（M1 实装时可裁剪特性） |
| `cpal` | 0.18.2 | 音频输出（Windows WASAPI shared） |
| `rubato` | 5.0.1 | 重采样 |
| `rtrb` | 0.4.0 | lock-free SPSC ring buffer |

- `cargo deny check licenses bans sources` 全绿（127 包依赖树，无 GPL/AGPL/LGPL）。
- `cargo deny check advisories` 本机因无法连接 github.com（RustSec advisory-db clone 超时）NOT RUN，由 CI 执行。
- 只锁定到 minor 版本（如 `"0.6.1"`），`Cargo.lock` 精确固定；升级走显式决策。
- 备注：Windows-first 项目固定 MSVC 目标；不使用 windows-gnu。

## D-012 品牌色进入 brand.json（2026-10-02）

`brand.json` 除 Phase 0 §3 的四个必填字段外，附加 `version` / `copyright` / `colors`（bg、rose、violet、sky、mint），以满足总章程 §4.1 对 brand.json 的完整要求；`ui/src/design/tokens.ts` 与生成的 CSS variables 均以 `brand.json` 为单一事实来源，`scripts/generate-tokens.mjs` 负责生成与一致性校验。

## D-014 SPSC Ring Buffer 选型（2026-10-03，M1）

`rtrb` 0.4（M0 已锁定），外层包装帧语义 `AudioRingBuffer`（ProducerHandle/ConsumerHandle）。SPSC 由类型系统强制（句柄不可 Clone）。generation-aware 丢弃在 consumer 侧执行：producer bump 时以原子量记录缓冲中旧代帧数，consumer 检测到代切换后**精确丢弃**这些帧（不波及 bump 后写入的新代数据）。

## D-015 Decoder 选型（2026-10-03，M1）

直接使用 `symphonia_bundle_flac` 的具体类型 `FlacReader`/`FlacDecoder`（不经通用 probe/registry 层）。理由：M1 只支持 FLAC，具体类型 API 稳定且零漂移；symphonia 0.6 按 RFC 9639 实现的元数据/帧布局与旧版有差异，走通用层反而引入不确定。真实解码测试夹具由 libsndfile（soundfile 1.2.2）生成的 21KB 单声道 48k 1 秒 FLAC 文件提供（提交于 `crates/aura-core/tests/fixtures/`，自生成可公开分发，§41 合规）。

## D-016 内部 PCM 格式（2026-10-03，M1）

全 pipeline 统一**交错 f32**。源格式由 PcmBuffer 携带的 AudioFormat 显式标注。非有限样本值在输出渲染阶段确定性替换为 0（D-025）；有限极值允许通过（限幅属 M2）。单包解码失败不致命：跳包继续（容错策略）。

## D-017 重采样选型（2026-10-03，M1）

rubato 5 `Async::new_sinc`（256 阶 sinc、Blackman2 窗、1024 帧输入块、FixedAsync::Input、相对可调带宽 2.0——必须 ≥1 覆盖 44.1k↔48k 的 ratio 1.088）。采样率一致时旁路。吞吐基线 285× 实时（见 PERF.md）。EOF 尾块以 `partial_len` 喂入（rubato 以静音补齐缺帧）。

## D-018 时钟设计（2026-10-03，M1）

无独立 wall-clock 依赖：位置权威 = 输出侧消费帧数（SharedOutputState.position 原子量）。测试确定性由 aura-testkit 的 VirtualClock 与显式 `render_for_frames(n)` 推进保证；禁止 `SystemTime::now()/Instant::now()` 作为播放位置依据。

## D-019 Generation 设计（2026-10-03，M1）

load/seek 均 bump generation（u64，起 1）。ring buffer producer 在 bump 时记录缓冲内旧代帧数；consumer 检测代切换后精确丢弃。**并发残余竞态**：弹出过程中发生 bump 时，该块整体丢弃返回 0（输出静音）——最坏情况为单个渲染块的静音间隙，不会输出旧代样本；确定性单线程路径下零泄漏。逐块大小由 Engine 的 chunk_frames 控制（默认 buffer/8）。

## D-020 NullOutput 设计（2026-10-03，M1）

NullOutput = OutputStage（渲染）+ SharedOutputState（原子位置/underrun/目标增益）+ 捕获缓冲。`render_for_frames(n)` 以固定块推进：命令 → 填充（背压感知）→ 渲染 → 捕获。CI 无声卡全链路可跑。捕获缓冲容量预分配，零分配测量窗口内不增长。

## D-021 输出抽象与 CPAL 适配（2026-10-03，M1）

`AudioOutput` trait 最小化（format/start/pause/stop/consumed_frames/underruns）。真实设备走 `CpalOutput`（默认设备、设备默认格式，f32/i16 两路径；错误回调仅递增原子计数）。已知限制：设备格式与生产格式不一致时的重采样接线属 M6+；pause 语义由 Engine 驱动（渲染静音），设备流保持运行。

## D-022 Overrun 策略（2026-10-03，M1）

producer 遇缓冲满即**部分写入拒绝**（返回实际写入帧数），fill 循环背压暂停——不覆盖、不丢已缓冲数据。underrun（consumer 侧）：输出静音 + 计数 + 管线可恢复，不 panic/不阻塞/不分配。

## D-023 Stop 语义（2026-10-03，M1）

Stop：状态回 Stopped、**播放位置归零**、缓冲清空（generation bump）。暂停（Pause）位置保持。seek：位置设为目标帧（同块 generation bump）。

## D-024 EOF 尾块重采样策略（2026-10-03，M1）

流结束时剩余输入不足重采样器一个完整块：以 `partial_len` 喂入，rubato 以静音补齐缺失输入帧。尾部补齐不影响有效输出样本（调用方按帧数取用）。

## D-025 PCM 安全净化（2026-10-03，M1）

输出渲染阶段：非有限样本值（NaN/±Inf）确定性替换为 0，随后应用平滑增益。合成/解码路径产生的有限极值原样通过（限幅属 M2 loudness/limiter）。

## D-026 零分配承诺边界（2026-10-03，M1）

零分配承诺覆盖 **OutputStage::render 回调路径**（真实线程模型中即 audio callback：ring pop + 增益平滑 + 计数），由计数分配器测试证明。M1 单线程引擎中 decode/fill 属解码面，分配允许（真实线程模型中移入解码线程）；resampler 分块输出缓冲同理。真实线程接线（M6+）时保持此边界。
