# PROGRESS

## Milestone Status

```text
Phase 0 = COMPLETE (2026-10-02)
M0  = COMPLETE (2026-10-03)
M1  = COMPLETE (2026-10-03)  (Audio Engine)
M2  = COMPLETE (2026-10-03)  (Playback Quality)
M3  = NOT STARTED   (Music Library)
M4  = NOT STARTED   (Sources)
M5  = NOT STARTED   (Cache)
M6  = NOT STARTED   (UI Foundation)
M7  = NOT STARTED   (Dreamscape)
M8  = NOT STARTED   (Analysis)
M9  = NOT STARTED   (Music Map)
M10 = NOT STARTED   (Windows Integration)
M11 = NOT STARTED   (Online Sources)
M12 = NOT STARTED   (Advanced Experience)
M13 = NOT STARTED   (Distribution)
M14 = NOT STARTED   (Final Release Audit)
```

无任何伪造完成状态：除 Phase 0 与 M0 外全部 NOT STARTED。



## M2 Completion（2026-10-03）

- A 级 16 项全部实现（§0）：样本级 gapless（同管线连续解码）、下一曲预解码（3s 窗口）、无缝拼接、crossfade 0–12s 等功率、gapless/crossfade 互斥策略、loudness 归一（−16 LUFS 目标）+ 峰值安全、设备变化检测/断开安全暂停/恢复可续/格式重配置/无 panic/可诊断 + 确定性测试。
- B 级 4 项全部实现：10-band RBJ 参数 EQ（Peak/Shelf）、preamp ±12dB、6 预设（纯数据）、按设备记忆 EQ 的数据模型基座（EqBus 快照可按设备键扩展）。
- 测试：workspace **108 通过**（M2 新增：gapless 5 + crossfade 5 + loudness/EQ/hotplug 9 + 集成/确定性/soak 5+1）。
- §51 十项 M1 证明全部回归通过；零分配证明升级为全 DSP 链（stage 级，独立二进制）。
- 性能：全 DSP 回调 avg 44.6µs / p95 49.3µs / p99 54.6µs（预算占用 0.21%）；1h 等效 soak PASS；见 docs/PERF.md。
- 已知限制：真实声卡收听验证待 M6；symphonia gapless trim 的真实 MP3/Vorbis 夹具待 M3 真实文件接入后补充；设备检测的 Windows 事件适配器归 M10。
## M1 Completion（2026-10-03）

- 结构：`aura-core` 分层（types/error/audio{buffer,gain,position}/decoder{mod,flac,pcm}/resampler/output{mod,null,device}/engine{command,player}）。
- A 级 18 项全部实现（§0）：decoder abstraction + FLAC 真实路径、重采样（rubato）、SPSC ring buffer、NullOutput、播放时钟/位置/状态机、seek、generation 陈旧样本拒绝、音量 + 10ms 平滑、underrun 检测、错误模型、完整测试。
- 测试：lib 25 + 集成（ring 13 / engine 9 / flac 2 / zero-alloc 1）= **50 通过**。
- **§51 十项证明**：真实 FLAC 解码 ✓（libsndfile 夹具逐样本一致）；resampler ✓（4 组采样率矩阵 + 频率保持）；ring buffer ✓；NullOutput 确定性消费 ✓；Play/Pause/Stop ✓；seek ✓；**无 stale generation** ✓（ramp provenance：seek 后样本全部来自新代值域）；音量平滑 ✓（无相邻跳变 + 可重复）；underrun 静音+计数 ✓；**callback 零分配** ✓（计数分配器，度量窗口 0 次分配）。
- 性能基线：见 docs/PERF.md（decode 3668×、resample 285×、回调 5.1µs/1024 帧块）。
- 已知限制：真实设备收听验证待 M6（当前仅编译级 + 代码审阅）；escape 残差合成 FLAC 与 symphonia 0.6 demuxer 兼容性问题已绕过（改用 libsndfile 生成夹具）。
## M0 Completion（2026-10-03）

Charter §51 M0 定义 8 项对照：

| 项 | 状态 | 说明 |
|---|---|---|
| workspace | ✅ | 7 crates + src-tauri（M0-3 接入），cargo check/clippy/test 全绿 |
| CI | ✅ | ci.yml 全命令就绪（tauri-build job 待 M6 激活） |
| deny | ✅ | cargo-deny 0.20.2；licenses/bans/sources 全绿（127 包）；advisories 本机网络受限 NOT RUN → CI |
| brand | ✅ | brand.json 单一事实（Phase 0） |
| design tokens | ✅ | tokens.ts + 生成管线（Phase 0） |
| IPC contracts | ✅ | M0-4：`app_info` 命令 + TS 镜像 + 双侧契约测试 |
| testkit | ✅ | M0-5：VirtualClock / SplitMix64 RNG / 合成 PCM / NullOutput（FaultFS 留 M3/M5） |
| project docs | ✅ | 12 份治理文档（Phase 0） |

附加产出：MSRV 实测锁定 **1.87**（1.85 声明被 rubato 5.0.1 推翻，以真实工具链验证）；M1 四件套依赖锁定（symphonia 0.6.1 / cpal 0.18.2 / rubato 5.0.1 / rtrb 0.4.0）。

## Phase 0 Baseline Audit（2026-10-02）

### A. Environment

| 项 | 结果 |
|---|---|
| OS | Windows 11 (10.0.26200), x64 |
| Shell | Git Bash (MINGW64), PowerShell 5.1 备用 |
| Rust / Cargo | ✅ 已安装（2026-10-03）：rustup stable `x86_64-pc-windows-msvc`，rustc/cargo 1.99.0，VS Build Tools 2022 (MSVC 14.44)；见 DECISIONS D-011 |
| Node / npm | node v24.19.0 / npm 11.17.0 |
| Tauri CLI | 未安装（按需经 cargo tauri / npm 引入） |
| cargo-deny | 未安装（M0-2 安装并首跑） |

### B. Git

- 仓库：`E:\WorkSpace\ifuyo-aura`，分支 `main`，Phase 0 之前**零提交**。
- remote：`origin = https://github.com/taozhihaoo/ifuyo-aura.git`（已配置，本阶段不 push）。
- 用户：taozhihaoo / zihaotao@qq.com。
- Phase 0 前未跟踪文件：`.zcodeignore`（ZCode 工作区本地配置，不入库）、`Spec/`（项目规格文档，入库）。

### C. Existing Files

仅 `.zcodeignore` 与 `Spec/`（Charter + Phase 0 + M1–M14 规格）。无代码、无旧项目文档、无音乐文件、无 `.env`、无密钥/证书文件。

### D. Existing Dependencies

无（无 package.json、无 Cargo.toml）。Phase 0 新增依赖均为前端脚手架的必要 devDependencies 与 react/react-dom，见 `ui/package.json`。

### E. Security

对仓库执行 `api_key|apikey|client_secret|password|token|private_key|BEGIN PRIVATE KEY` 扫描：命中的仅为 Spec 中的**审计清单条目**（要求确保这些内容不存在），无真实凭据。PASS。

### F. Licensing

Phase 0 前无 LICENSE 文件。已创建 `LICENSE-MIT`、`LICENSE-APACHE`、`THIRD_PARTY_LICENSES.md`（机制说明；依赖清单待依赖解析后生成）。PASS。

### G. Architecture

Phase 0 前无架构文档。已创建 `ARCHITECTURE.md`（分层、crate 职责、依赖方向、aura-core 隔离约束）。PASS。

### H. Risks

1. 本机无 Rust 工具链 → cargo 类门禁本地不可运行（见下节），由 CI 与 M0 承接。
2. Tauri CLI 未安装 → `tauri build` 门禁延后至 M0 接线后。
3. `Spec/` 中保留少量旧名称（`IFUYO Music` / `Aura Music` 等）——均为刻意的旧名称审计清单与章程变更记录，非冲突。

## M6 上篇 Status（2026-10-05）

**M6-A（上篇 §0–171）= PASS　M6-B（下篇 §172–318）= NOT STARTED（属独立里程碑）**

- Tauri shell：frameless + 自绘标题栏 + window controls（capabilities 最小化，无 shell 权限 §23）；CSP 保持严格（§24）；1180×760/900×600（§17）。
- 应用层（src-tauri）：AudioHandle 驱动线程（M1 Engine fill+process @10Hz 快照 + CPAL 真实输出，消费端移交 D-064）；LibraryService 接线（list/search/scan/progress 事件/cancel）；设置 JSON v1（§114–116）。
- UI：React+TS+Zustand 分域 stores（§8/§9）；typed IPC + DTO guards（§11）；i18n zh-CN/en（§117）；Now Playing（真实播放状态/进度插值/音量/seek 禁用逻辑，§33–45）；Library（真实 FTS5 搜索 debounce/虚拟化列表/排序映射 M3/扫描进度+取消/空错态，§76–94）；Map 壳（§95–97）；Command Palette 注册架构 + Ctrl+K + 键盘导航（§98–102）；Settings panel（语言/缩放/reducedMotion/loudness/crossfade/EQ，§114）；Queue drawer（in-memory，引用 TrackId，§48–49）；Mini Player WebviewWindow（共享应用层，§109–113）；Aurora Three.js GLSL（单一 canvas/节流 30-8fps/reduced-motion/稳定 seed/context-loss fallback，§56–75）。
- 测试：ui vitest **16 通过**（stores/i18n/命令注册/DTO guards/seed 确定性）；Rust ipc 单测保留；workspace **339 通过 / 0 失败**（M1–M5 全回归）。
- 已知限制：Aurora FPS 预算为运行时实测项（§132）留待 M6 下篇窗口化验证；folder dialog 依赖 tauri-plugin-dialog（前端 npm 包 + Rust 插件均已接）；queue reorder/undo 为 B 级；Mica 材质待 Win11 实机验证（solid fallback 已实现）。
- M6 上篇 Final Audit：见 2026-10-05 会话输出。

## M5 Status（2026-10-04）

**M5-A = PASS（A 级 41/41）　M5-B = NOT COMPLETED（智能 prefetch/coalescing/部分离线/diagnostics/clear/priority 留待后续）**

- 实现：aura-cache（keys/URL 规范化、256KiB 原子分块存储、SQLite WAL 索引、CachedRangeReader（Range 拉取 + sequential fill 降级 + revalidation + retry/backoff + 离线可用）、LRU/pin/容量 2GiB、fetch dedup、bounded prefetch、integrity/recovery、FaultFS 全场景）。
- 测试：aura-cache **52 通过**（store 10 / manager 10 / range 9 / concurrency 7 / integration 4 / keys+error 5 / FaultFS 4 / testkit 17）；workspace 累计 ~345。
- 性能：PERF.md M5 节（chunk 读 1423 MB/s、写+DB 244 MB/s、SHA-256 2429 MB/s、lookup 6.7µs）。
- 已知限制：部分离线播放为 B 级未实现（miss → 错误，不误报）；TTL 周期 revalidation 未做（open 时一次）；orphan 直接删除不 quarantine；SessionOnly→Allowed 升级不静默迁移（§73 有测试）。
- M5 Final Audit：见 2026-10-04 会话输出。

## M4 Status（2026-10-04）

**M4-A = PASS（A 级 33/33）　M4-B = NOT COMPLETED（RSS/HLS/WMF AAC 留待后续）**

- 实现：Source trait 面 + resolver（顺序/校验/封锁先于网络）+ Local/Direct HTTP/Internet Radio/Radio Browser 四来源 + ICY 剥离与事件 + NetworkAudioStream（有界缓冲/Range seek/取消/空闲检测）+ AudioByteStream→StreamingDecoder 适配 + UnsupportedPlatform 零请求。
- 测试：aura-sources **85 通过**（HTTP 矩阵 15 / ICY 9+property / resolver 8 / network_stream 8 / radio 6 / RB 12 / 集成 5 / perf 5）；workspace 累计 ~300。
- 安全：零请求封锁测试、DNS-rebind 仿真、重定向逐跳校验、凭据 URL 拒绝、body 上限、零写盘 helper。
- 性能：PERF.md M4 节（ICY 剥离 1818 MB/s、URL 校验 0.96µs、live 缓冲上限表）。
- 已知限制：B 级未实现（.m3u8 明确提示 tier-B）；AAC 电台 → UnsupportedCodec；自动重连未实现（§111）；reqwest blocking 取消的 stalled-read 边界依赖 keepalive（D-052）。
- M4 Final Audit：见 2026-10-04 会话输出。

## M3 Status（2026-10-04）

**M3 = COMPLETE（A 级 29/29，B 级 4/4）**

- 实现：scanner / metadata(lofty) / identity(SipHash-2-4-128) / SQLite(WAL+migrations) / FTS5(CJK 二元组) / filter AST / artwork+thumbnails / incremental scan / watcher(coalescer) / query API / [B] playlist+queue 模型。
- 测试：aura-library **110 通过**（workspace 累计 198+）；迁移测试含 fresh/upgrade/rerun/failure-rollback/reopen/WAL/并发读/损坏。
- 性能（release）：扫描 3484 文件/s；增量重扫 40×；10k 搜索 avg 1.1ms（§79 交互级达标）；PERF.md M3 节。
- 已知限制：reparse point 不跟随（D-041）；音频替换且音频签名完全不变时判为标签改写（D-038）；queue Clear 不可恢复（D-047）；permission-denied 文件矩阵在 Windows CI 上不可稳定模拟（分类逻辑由 unit test 覆盖）。
- M3 Final Audit：见 2026-10-04 会话输出（A–V 全 PASS）。

## Phase 0 Gate Status（§19 规则：不伪造 PASS）

| 门禁命令 | 状态 | 说明 |
|---|---|---|
| `cargo fmt --check` | **PASS**（2026-10-03 本地 msvc） | 首次真实执行，零 diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**（2026-10-03 本地 msvc） | 零告警 |
| `cargo test --workspace` | **PASS**（2026-10-03 本地 msvc） | 7 crates 骨架测试全过 |
| `cargo deny check` | **部分 PASS**（2026-10-03 本地）：licenses / bans / sources 全绿（127 包）；advisories 因本机无法连接 github.com **NOT RUN**，由 CI 执行 |
| `npm run typecheck` | **PASS**（2026-10-02 本地） | tsc --noEmit 零错误 |
| `npm run lint` | **PASS**（2026-10-02 本地） | eslint 零告警 |
| `npm test` | **PASS**（2026-10-02 本地） | vitest 2/2（design tokens 契约） |
| `npm run build`（vite） | **PASS**（2026-10-02 本地） | 31 modules，dist 产出正常 |
| `tauri build` | **DEFERRED**（打包级）；src-tauri 已于 M0-3 接入 workspace 并通过 `cargo check/clippy`（2026-10-03）；完整打包待 UI（M6）激活，CI 中该 job 保留命令（`if: false` 占位） |

由对应 Milestone 修复项：M0-2 安装 cargo-deny 并首跑；M0-3 接线 src-tauri 后激活 tauri build 门禁。

## Phase 0 Final Audit（2026-10-02，§21）

- **Identity**：全仓库扫描旧名称（Aura Player / Aura Music / IFY Music / IFUYO Music / IFUYO Player），仓库代码与文档零命中；Spec 中的命中均为刻意的旧名称审计清单与章程变更记录。`brand.json` 为唯一品牌事实。PASS。
- **Git**：分支 `main`；本阶段前零提交；remote 已配置但**未 push**（§22）。`.zcodeignore` 已忽略；node_modules/dist/target 均被忽略。PASS。
- **Security**：对全部新增文件执行 `api_key|apikey|client_secret|password=|private_key|BEGIN PRIVATE KEY` 扫描，零命中；无 `.env`；无音乐文件入库。PASS。
- **Licensing**：LICENSE-MIT / LICENSE-APACHE / THIRD_PARTY_LICENSES.md（机制 + 待生成声明）存在。PASS。
- **Architecture**：ARCHITECTURE.md 含分层、crate 职责与依赖方向、aura-core 隔离约束。PASS。
- **Documentation**：§5 全部 12 份文档存在。PASS。
- **Build**：npm 侧全绿（见上表）；cargo 侧 NOT RUN（工具链缺失，已如实记录，命令保留，CI/M0 承接）。PASS（带已记录限制）。

## M6（2026-10-05）

M6-A = PASS（上篇：shell/Now Playing/Library/Aurora/palette/mini/IPC 真连 M1–M5 + 验证器补强 artwork/Mica/smoke）
M6-B = PASS（下篇，全部为真实实现；无「声明完成未验证」项）

### Implemented（下篇）

- `play_input` 统一来源入口（M4 resolver → Local/Radio/DirectHttp 装载；D-070/D-071）+ `library_stats` 真实统计
- 错误映射链（code→i18n）+ FeatureBoundary 页面隔离 + NoticeBar；时长/数值安全工具（§253–256）
- queue 游标模型（next/previous/remove/current，单一权威 §197）；palette Open Folder/File/URL（filter 与 M3 同步）
- Aurora VISUAL_TEST_MODE + 运行时 FPS 计数（data-fps）；titlebar 双击最大化；键盘图 Alt+1/2/3、Ctrl+Q/M
- capabilities 补 mini 窗口权限（D-076）；custom-protocol 发布构建 feature（D-075）

### Tests

- Rust：workspace 339+（新增 perf_ui 显式基线）；aura-shell/aclippy 0 警告
- 前端：vitest 42 项（含 jsdom 渲染/a11y/键盘矩阵/挂载卫生 ×10/×20）
- 桌面冒烟：`--bin smoke` SMOKE OK + startup-to-ready 560.6ms（debug）

### Screenshots（§308，真实构建）

01-now-playing / 02-library / 03-command-palette / 04-mini-player / 05-map-shell / 07-queue（docs/screenshots/，capture-screens.ps1 可复现；06 EQ/08 空库/09 错误态为建议项——08 可复用 02（本环境库为空即空态），06/09 未截）

### Performance

见 docs/PERF.md「M6 UI」节（10k 行 list avg 6.6ms、搜索 worst 3.1ms、snapshot 188ns；硬件/OS/构建/方法已注明）

### Accessibility

按钮可访问名/dialog 语义/slider 标签/焦点环/Esc 无死路/键盘矩阵自动化（§181–183）；125%/150% DPI 由 root font-size 缩放（§107）+ 系统值实测依赖实机（限制如实声明）

### Security

§191/§300 审计全过（capabilities/CSP/无 shell/零远程资产/零凭据/IPC 校验）；SECURITY.md M6 节

### Known Limitations

- 数小时 soak 未执行（远程虚拟显示环境；§296 如实声明，脚本= capture/门禁复跑）
- 06/09 两张建议截图未专门摆拍；Mica 视觉需 Win11 实机确认
- queue 持久化（M3-B 表）未接线（§50 允许，应用层 in-memory）
