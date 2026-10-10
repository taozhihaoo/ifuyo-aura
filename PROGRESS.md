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
- **tauri build 实测 PASS**（§304/§211）：`npx tauri build` →
  `target/release/aura-shell.exe` + `bundle/nsis/ifuyo Aura_0.1.0_x64-setup.exe`
  （8.02 MiB，unsigned——签名未配置，§211 不伪造 signed）

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

## M7 上篇（2026-10-05）——Dreamscape 世界生成核心（§0–127）

状态 = PASS（世界核心全部落地；渲染/集成/门禁在下篇 §128–330，未偷渡）

### Implemented

- aura-synth 正式建立（11 模块，§7 职责边界内）：seed（D-079）/ RNG 命名流（D-080）/ Genome+mutation（D-081）/ Environment+palette（D-082）/ Biome×5（§43–46）/ World 拓扑+预算（D-083）/ 稀有事件框架（D-084）/ SceneManifest+scene_hash（D-085）/ GeneratorV1（§108/109）/ Region 流式+取消（D-086）/ Snapshot+Session（D-087）
- shell：dreamscape.rs 装配 + `dreamscape.json` 逻辑态持久化（原子写、损坏回退）+ 9 个 typed IPC 命令；ui 错误码接线（INVALID_SEED/DREAMSCAPE_*）
- 明确禁止项未实现（§2）：无 BPM/Key/LUFS/MFCC/embedding/Map/SMTC 等

### Tests

- aura-synth 78 项（确定性合同/预算/过渡/重放/边界/serde 全覆盖）；shell 3 项（持久化往返/损坏回退/版本门）；workspace 全绿
- 关键不变量测试锁定：scene_hash 同输入相等（§114）、事件日志重放一致（§92）、mutation 小步（§26/83）、fog≤0.7（§39）、夜间环境光≥0.25（§99）、manifest<256KiB（§115）、区域切分完整（§119）

### Known Limitations

- 渲染（Three.js Dreamscape renderer/相机/LOD/上下文恢复/截图模式）、palette 命令接线、audio telemetry 反应、性能 bench——全部属于 M7 下篇（§128–330）
- 植被/地形仅逻辑数据（渲染表现待下篇验证）； vegetation RNG 流已定义但世界层尚未消费（对象密度走 placement 流——见 D-083，下篇渲染密度控制时接线）

## M7 下篇（2026-10-05）——渲染、集成、测试与发布门禁（§128–330）

M7-A = PASS（上篇世界核心 + 下篇渲染/集成全部落地）
M7-B = PASS（真实渲染+集成+测试；B 级如实列出，见下）

### Implemented（下篇）

- Three.js 渲染器（three 0.186，D-088）：程序化地形/InstancedMesh/landmark 原语组合/粒子；质量档 Low/Med/High/Auto（D-089）；节流 30/10/2/5fps（D-090）；context lost→恢复重建（§151）；pixelRatio cap（§150）
- 集成：Aurora 互斥（D-091）；Zustand 最小状态（§269）；manifest 走 ref（§270）；低频推进（§263/§264）；palette 四命令 + Esc 分层退出（§197/§198）；幂等命令 + revision（D-093）
- 后端加固：validate_world（§167/§182）；快照 integrity+原子写（D-094）；Frozen（§101）；历史上限（§219/§220）；serde 兼容（§229/§230）
- 真实构建截图 ×8（01–05/07/10/12，§320 建议项 11/13/14–18 未专门摆拍）；capture 脚本鼠标点击导航（绕 IME）

### Tests

- vitest 60（新增 engine 13 + dreamscape-integration 5：§187/§142-143/§258-261/§275/§277/§189/§190/§188 短循环）
- Rust：synth 85 lib + stress_fuzz 4（§249–252）+ shell 3；workspace 全绿
- 截图管线暴露并修复 4 个真实缺陷（body 居中/manifest serde 大小写/初始 setSize 缺失/WebGL 构造异常未捕获）

### Performance

见 docs/PERF.md「M7 Dreamscape」节（world 生成 9.9ms、manifest ser 996µs、snapshot save 125µs 等）

### Soak（§246–§248 如实声明）

- 确定性重放：事件日志重放一致性/快照往返/×400 演化有界——测试锁定
- 渲染器 enter/exit ×20（vitest）+ ×10 mount/unmount（keyboard 测试）
- 数小时真实桌面 soak 未执行（远程虚拟显示环境，同 M6 限制）；§246 动作循环已在截图管线与测试中部分覆盖

### Known Limitations

- 渲染为第一版视觉（Lambert 材质、无阴影/后处理栈；High 档 postEnabled 预留未实装特效）
- 11-night/13-mutation/14-seed/15-settings 等建议截图未摆拍（需环境手动覆盖 UI，B 级留待后续）
- Full-screen immersive（§199 B 级）未实现；用户手动 time-of-day/weather 预设（§100 B 级）未实现（Frozen/Auto 已可用）
- World slots 多世界（§214 B 级）未实现；快照备份 last-known-good（§226 B 级）未实现

## M8 上篇（2026-10-05）——分析基建（Domain/Identity/Persistence/Embedding Policy，§0–49）

状态 = PASS（基建落地；特征引擎在中篇 §50–204，运行时/集成/门禁在下篇 §205–390）

### Implemented

- aura-analysis 实装：domain model（§13–§18/§31–§33）+ analysis_key（§23/D-097）+ EmbeddingProvider 合同与真实本地 provider（D-095）+ M1 解码适配（§12/§34）+ AnalysisStore 持久化（D-096）
- M3 迁移 V2：track_analysis 重建（content binding/availability/算法版本/带元数据 BLOB/索引）
- §4 合规结构保证：无结果 → Pending/Unsupported/Stale 明确态；任何"Available"必有真实数据行

### Tests

- aura-analysis 28（domain 10 / store 8 / embedding 6 / decode 4）；aura-library 110（含迁移 v2）；workspace 全绿
- 关键不变量：all-or-nothing 完成语义（§31）、六态 availability 保留（§32）、替换→Stale（§20）、版本升级幂等 Stale（§18）、BLOB 完整性（§29）、嵌入逐位确定性（§46）、NaN/Inf 拒绝

### Known Limitations

- BPM/Key/LUFS/Spectral/MFCC 算法 = 中篇；worker/queue/cancellation 集成 = 下篇
- SpectralStatsProvider 的 DFT 为 O(N²)（离线可接受；中篇换 FFT 并升算法版本）
- 多声道 >2 无 downmix（M1 能力边界，明确报错不偷渡）

## M8 中篇（2026-10-05）——特征分析引擎（§50–204）

状态 = PASS（五族真实算法 + 管线 + 调度/合同；运行时/集成/发布门禁在下篇 §205–390）

### Implemented

- 规范预处理：22050Hz/mono/(L+R)÷2/线性重采样（D-098）；内部分析 FFT（radix-2，§80 自实现）
- LUFS：BS.1770-4 K-weighting/双门控/LRA/true peak 4× 过采样（D-099）；静音 below-measurable=None
- Tempo：flux onset→分数周期谐波 comb→[70,180) 折叠（D-100）；beat grid 帧位置有界
- Key：chroma 4096-FFT→KK 24 假设 Pearson→熵门（D-101）；Camelot（关系大小调映射修正）
- Spectral/MFCC：RMS/Peak/Centroid/Rolloff/Brightness/Energy（D-102）；pre-emph/mel26/DCT/13 系数（D-103）
- 管线：单遍共享解码/取消/进度/clock 注入/特征隔离（§92–§108/§134–136）
- 调度/合同：Scheduler（优先级/取消/有界重试）+ Repository trait + InMemory + TrackAnalysisSummary + cosine 原语（D-104）

### Tests

- aura-analysis 88 lib + integration_m8 10（silence/noise/相位/采样率矩阵/×10 确定性/batch/summary/clock/损坏音频）；workspace 全绿
- fixture 全部合成（§55/§73/§191–§195）；§196 真实 provider 运行（SpectralStatsProvider 非 mock）

### Known Limitations

- 真实音乐 BPM/Key 精度未评估（synthetic fixture 全过；§196 无第三方模型——自有谱统计嵌入即真实运行）
- 数小时 soak = 下篇 §246；MFCC DCT O(k·n)（26×513，可接受）；重采样线性插值（D-098 已知限制）

## M8 下篇（2026-10-05）——运行时、集成与发布质量（§205–390）

M8-A = PASS（上篇基建 + 中篇算法 + 下篇运行时/合同全部落地）
M8-B = PASS（真实运行时与合同；长 soak 如实声明未执行）

### Implemented（下篇）

- 查询/恢复：query_track_ids（参数化 SQL、NULL 不匹配 §225、数值范围 §227/§228、key canonical §226、limit 钳制 §241）+ recover_running_to_pending（§211）
- 运行时（D-105）：AnalysisService（batch 去重/有界 §206/§241）、dedup_active（§207）、commit_result stale 保护（§208–§210）、run_job_isolated panic 边界（§244/§245）、WorkerState 生命周期（§213/§212）
- M7 合同（D-106）：AnalysisVisualSummary（none/from_analysis，§231/§232）；M6 状态/summary IPC 三命令（§233/§117）
- 合成生成器 generator.rs（§261–§264 确定性 fixture + 元数据）
- PERF：分段耗时/端到端/10k 库 soak/队列 10k/存储预算 532B/首（§302–§309）

### Tests

- aura-analysis 103 lib + integration_m8 10 + perf_analysis 5（ignored 显式跑）；shell 3；workspace 527+（多轮全绿）

### Known Limitations

- 数小时 analysis/playback soak 未执行（远程环境限制，§352/§353 如实声明）；100-track 顺序分析已测
- Embedding O(N²) DFT 热点（debug 实测 26s/30s 音；换 FFT 可加速，升 model_version）
- 真实音乐 BPM/Key 精度未基准评估（synthetic 全过；§299 无 99.9% 宣称）

## M9 上篇（2026-10-05）——相似度内核、kNN 图、布局与聚类（§0–108）

状态 = PASS（内核层落地；渲染/交互 = 中篇 §109–269，门禁 = 下篇 §270–554）

### Implemented
- crates/aura-map（§13）：dataset 适配（兼容合同/向量校验/dataset hash §25–§34）+ 相似度内核（cosine/top-K D-109/D-110）+ kNN 图（D-112）+ 确定性聚类 LPA（D-113）+ 确定性布局 PCA+grid（D-114）+ 持久化（D-115 checksum/合同校验）+ 查询 API（neighbors/viewport/cluster_members）
- PERF（debug）：4k 节点 dataset 5ms、kNN 图 21s、聚类 106ms、布局 7.7s；存储=图 JSON

### Known Limitations
- exact scan ≤25k 节点（ANN = B 级 §44–§46 未引入，超界明确报错）
- 渲染/交互/搜索定位/LOD = 中篇；小时级 soak = 下篇
## M9 中篇（2026-10-05）——渲染、交互、构建与确定性（§109–269）

状态 = 功能全部落地（构建服务/恢复/IPC/前端渲染交互）；门禁与最终审计 = 下篇 §270–554

### Implemented
- aura-map/build.rs：build_map（冻结 snapshot→图→簇→布局→持久化；取消/generation 检查点；结束 hash 变→Stale §245/§246）、BuildGeneration、choose_build_kind（30% 阈值 §171/§172）、BuildKind
- aura-map/persistence.rs：PersistedMap 增 kNN 边 + graph/cluster 算法版本；restore_map（§174 免重算；图校验/非有限检查 §189/§190）
- shell map_ipc.rs：MapHost（惰性 restore、冻结采集只取 Completed §239、后台线程 §237/§238、§165 去重、§186 原子写）+ 8 命令（§228）；smoke 装配
- 前端 ui/src/features/map/：mapStore（§145 白名单测试锁定）、mapData（typed arrays+spatial hash+稳定配色+相机纯函数 §146/§156/§113/§135）、MusicMapView（Canvas2D 单渲染器 §149；LOD §142；视口 culling §143；标签优先级+碰撞 §206–§209；选中邻域边 ≤30 §118/§119；hover 本地 §158；按需渲染零 idle §197/§198；reduced motion 即时 §133；当前播放双环 §225）、MapDetailPanel（§124–§128）、MapToolbar（搜索 focus §129–§132、过滤 chips+Clear §217–§219、配色 legend §210–§216、视图命令 §138、构建/重建/取消 §236/§164、状态行 §235）、MapPage（状态握手 §234、构建轮询、§180 视图恢复/§181 损坏回退）
- i18n（zh-CN/en 各 47 个 map.* 新键）+ CSS

### Tests
- aura-map 40 lib 全绿（build 5/persistence 5/query 3/聚类/布局/图/dataset）；前端 vitest 75（新增 map 15：§145 形状/§113 配色/§216 过滤/§156 hash/§135/§137/§139 相机/§233 失效/§191 守卫）；tsc/eslint 绿

### Known Limitations
- 地图真实数据端到端（真实库构建+截图）= 下篇门禁； PERF 基准复核（阈值 30%）留待下篇
- Source 配色模式未提供（M4 stream 不入 M3 库表——§215 为"可以"级）
- search-to-focus 走 library_search IPC 与地图交集（≥2 万节点大库按视口补数，loadAll 上限 2 万）
## M9 下篇（2026-10-05）——测试、性能与发布门禁（§270–554）

**M9-A = PASS**（§544 四十五项门槛全部满足；见 Final Audit）
**M9-B = NOT COMPLETED**（ANN/增量 kNN/高级聚类/局部化布局/簇标签/密度层/专辑封面节点/书签/保存视图/大库优化 = B 级，未混入 A）

### Implemented（下篇）
- 门禁测试矩阵（tests/m9_gate.rs，12 项）：相似度 §389/kNN K=1,3,5+tie §390/图 §391/簇 §392/布局 0–1000+病理 §393–394/相同向量 §395/版本失效 §279–282/损坏 §283–285/构建生命周期 §286–289/build-cancel-reopen 循环 §482/查询语义 §298/内存核算 §291
- 增量布局：layout_seeded 种子冻结语义（§82/§277/§278 已有节点位置精确保持）+ build_map seeds 参数；**修复真实缺陷**——相同向量重叠分离死分支（§396/§397 epsilon 判定）
- shell：choose_build_kind 30% 阈值增量接线（旧位置作种子、迭代 40）+ 真实 build_duration_ms（§471–§473）+ MapHost 损坏 map.json 清除测试（§285）
- 前端：键盘 +/-/F/Esc/Enter（§317/§315）、双击播放（§314）、dpr cap（§326）、LibrarySearchRunner 搜索竞态守卫（§345）+4 测试、Open on Map（§311）/Open in Library（§312）、palette 命令 Rebuild/Fit/Reset（§411）、视图持久化提取+损坏测试（§181/§331）
- PERF（release 实测，PERF.md M9 节）：1k/4k/10k 全链分段（kNN 9.7s@10k、布局 9.1s、恢复 34ms、top-k 1.13ms）+ 内存账面 + >25k DatasetTooLarge 显式拒绝（§290 B 级边界）
- 文档：SOURCES M9 来源表（siphasher 核验 §381）、THIRD_PARTY、SECURITY §549 清单、PRIVACY §547、README §546、DECISIONS D-120..D-122、ARCHITECTURE
- 截图管线修复：标题栏合成点击在本环境不达 webview（历史 02/05 号截图从未真正导航）；改剪贴板+palette 导航；05 号首次截到真实 Map 空态（§400/§529）

### Gates（全绿）
fmt/clippy -D warnings/workspace **598**×2 轮/deny 四项/ui tsc+eslint+vitest **79**/smoke/tauri build+NSIS 8.48MiB/产物独立加载验证

### Known Limitations
- **真实数据全图截图 BLOCKED**：M8 未暴露分析触发 IPC（§404/§405 以实际代码为准，不伪造入口）→ 图数据无法经 UI 生成；截图提供真实空态
- 数小时 soak（§481/§483/§487）未执行（远程环境限制，如实声明）；以 20 轮 build/cancel/reopen + 竞态守卫测试替代核心语义
- 浏览器级 FPS/GPU 内存（§299）未测——Canvas2D 按需渲染无持续帧循环，指标不适用（如实记录）
- M9-B 全部 NOT COMPLETED（§545 不混入 A）
## M10 上篇（2026-10-05）——SMTC、媒体键、系统托盘与单实例（§0–242）

状态 = A 级能力全部落地；B 级未实现（§2 不混入）；下篇（§243–411 测试/原生 QA/门禁）待续

### Implemented
- **integration 协调层**（纯逻辑+10 mock 测试 §47）：SystemSnapshot（§138/§139 单源）、revision 单调（§148/§149）、track 边界 vs status（§37/§91）、position 500ms 节流（§141/§144/§182–§184）、seek 验证（§69/§70）、ICY 去重（§55）、字符串限长+Unicode 保真（§133–§136）、title fallback（§43）、artwork 竞态门（§150）
- **SMTC**（windows 0.62 最小 feature+interop）：GetForWindow 绑定主窗、Play/Pause/Stop/Next/Previous 按钮（§20–§25）、按钮 enabled 随 queue 边界（§65–§67）、DisplayUpdater metadata+封面字节流（§36/§47–§52）、timeline 秒→100ns checked（§32/§233）、token 保存退出移除（§104/§105）、回调只投递（§74/§187）
- **托盘**（tauri tray-icon）：32px 程序生成图标（§79/§80）、稳定菜单 ID §156、动态 Play/Pause 文案 §85、enablement §86、当前曲目行 §84、locale 跟随 §93–§94、单例所有权 §106、失败降级 §120、左键菜单/双击聚焦 D-125 §88–§89
- **单实例**：官方插件先于 SMTC 注册（§116）；第二实例仅 focus（§109/§113/§230–§231）
- **命令路由**：全部进 Playback Authority（§7/§8）；next/previous 事件桥复用前端 queue（§23/§24）；queue 边界由前端上报（§65/§153）；媒体键=SMTC 按钮，未注册全局快捷键（§239/§241）
- **close 策略**：主窗 X=正式退出（§98/§174）；Tray Exit=正式退出（§102/§103）
- **修复**：setup 顺序（tray 在 manage 前 state() 导致启动 panic）——tauri build 产物运行时复验（加载/单实例/进程数 1）

### Tests
- Rust 608×2 轮全绿（含 integration 10 mock + shell 14）；vitest 79；smoke OK；deny 四项绿（windows 0.62/single-instance 审计 §237/§238）
- 运行时验证（tauri build 产物）：独立加载 ✓、第二实例退出+主实例存活 ✓、进程数 1 ✓

### Known Limitations
- **SMTC v1 无 position-request 通道**（D-125，以实际 API 为准 §27）——seek 面由应用内 UI 承担；SMTC 侧无可达 seek 事件
- 真实 SMTC 面板/托盘菜单的 GUI 交互验证（§191/§192 的桌面级 soak）= M10 下篇；本篇以确定性 mock+运行时健康验证为准
- B 级全部未实现（全局自定义快捷键/SMTC seek 增减/任务栏集成/托盘子菜单/后台模式设置等 §2）
## M10 下篇（2026-10-05）——测试、原生 QA 与发布门禁（§243–411）

**M10-A = PASS**（§401 五十项门槛；两项如实标注，见 Known Limitations）
**M10-B = NOT COMPLETED**（§402：全局自定义快捷键/高级 SMTC/任务栏集成/高级托盘/启动策略——未实现，未混入 A）

### Implemented（下篇）
- **纯逻辑测试矩阵**（§251–§267，26 shell lib 测试）：状态映射 §253（playing/paused/stopped/error/buffering/idle）、seek 边界 §254（zero/inside/exact/beyond/NaN/inf/unknown/non-seekable）、时间换算 §255（44.1/48/96k；0/1 帧；分钟/小时/超长不溢出）、按钮启用 §256、revision 序列 §257、track/metadata 竞态 §258/§260、artwork 门 §259、tray 文案/映射/菜单推导/本地化 §261–§263、单实例 argv 不路由 §264（tray_action/apply_menu_state/second_instance_action 纯函数化）
- **原生验证脚本**（scripts/m10-native-verify.ps1，tauri build 产物）：M10-SINGLE-01..03（B 退出 102–120ms/进程数 1）、**M10-SMTC-01（系统 GSMTC 会话管理器列出 aura-shell.exe——SMTC 注册的系统级证据）**、M10-CRASH-01（强杀重启自恢复）、**M10-ZOMBIE-01 PASS（WM_CLOSE 正式退出后 0 残留进程 §355/§361）**
- **PERF M10 节**（§291–§294/§398）：单实例 102–120ms、启动 421–606ms；WinRT 调用级/长时内存如实 NOT MEASURED
- 文档：README 功能矩阵（§397/§400 不夸大：Installer/Updater/Steam=Future）、CHANGELOG、SECURITY grep §370 零命中

### Gates（全绿）
fmt/clippy --workspace -D warnings/workspace **620**（一轮 aura-sources 网络压力偶发超时，重跑 7 连绿——已记录的既有 flake）/deny 四项/ui tsc+eslint+vitest **79**/smoke 389ms/tauri build+NSIS 8.93MiB/原生验证全证据行

### Known Limitations（§399/§400/§343 如实）
- **SMTC 按钮回环的状态读取 BLOCKED**：PS5.1 WinRT 投影无法解包 GSMTC 会话属性（会话注册本身已系统级验证）——人工按媒体键目测面板 = 手动 QA 清单
- **SMTC v1 无 position-request 通道**（D-125）——面板无用户 seek 面；seek 验证逻辑保留给应用内入口
- 数小时 desktop soak（§353）未执行（环境限制如实声明）；以 20 轮 build/cancel/reopen + 原生验证脚本替代核心语义
- Windows 版本矩阵（§337）：仅在本机 Windows 11 实测；Windows 10 未测——如实声明
- DPI/多显示器/主题矩阵（§338–§340）：未逐项实测——如实声明；托盘图标为深浅主题通用设计
- Tray 重连（§283）：Tauri tray 创建失败仅降级+diagnostics，不做后台重试（§365/§367 无忙修复循环）——limitation 如实记录
## M11（2026-10-05）——安装包、自动更新、代码签名与 Release Engineering

**M11-A = BLOCKED**（生产 Windows code-signing 证书与生产 updater 密钥对缺失 §25/§103——架构/脚本/配置模板/mock/local dry-run 全部完成；不伪造 PASS）
**M11-B = NOT COMPLETED**

### Implemented
- **版本权威 §A1**：version:check 五源一致（ui/tauri conf/Cargo/brand/app-v* tag）+ CHANGELOG 段检查（warning，pipeline 强制）
- **Windows Installer §3**：NSIS per-user（`ifuyo Aura_<v>_x64-setup.exe`）；**installer smoke 实测全 PASS**（静默装 1s/启动/卸载/用户数据保留 §4；注册表 DisplayName+Version 证据 §49）
- **Secure Updater §5–§15**：官方 tauri-plugin-updater/process；dev 配置（本地端点+dev 公钥+insecure 本地声明 §9 官方机制）与 release overlay（CI secrets 生成、HTTPS、createUpdaterArtifacts）严格隔离 §77；UpdateService typed 状态机 §7/错误分类 §65/降级拒绝 §47/清单校验 §46/进度节流 §99/检查策略 §8（15s 延迟+24h 间隔+设置开关 §84+手动检查 §85）；Settings 更新区 §34（i18n §66）
- **Release Pipeline §18–§23**：release.yml（tag `app-v*` → validate/test/build/verify/draft，fail-closed §21，secrets guard §78）+ prepare-release-config + release-verify（存在/版本/签名/路径残留 §92）+ gitleaks CI §29 + `--locked`/npm ci §32
- **更新集成测试 §59**：本地 fake server + CDP 观测（scripts/cdp-*.mjs）——真实 HTTP 回环实测（`GET /updates/windows/x86_64/0.1.0` target/arch/version 插值正确）、Phase A 同版本→null、Phase B 0.2.0→Update 对象、安装回环触发（Channel 需应用内插件，console 构造限制=工具边界如实记录）

### Gates
fmt/clippy/workspace（--locked）/deny 四项/ui 四件套 92/version:check/smoke/tauri build+NSIS 8.93MiB/updater dry-run（签名产物 .sig 实生成+release-verify PASS）

### Known Limitations（§103 分类）
- **BLOCKED**：生产 Authenticode 证书未申请；生产 updater 密钥对未创建；GitHub Release 发布未经真实远端验证（无凭据）——均已完成代码/脚本/CI 侧准备
- GUI 级（托盘/SMTC 面板/Settings 按钮点击）自动化在本环境不可用（§336 人工 QA 清单见 docs/RELEASE_CHECKLIST.md）；update 按钮点击回环由 CDP 工具部分覆盖（检查回环实测、安装触发受 Channel 构造限制）
- 数小时 update soak（§353）未执行；Windows 10 版本矩阵（§337）未实测
- 升级 smoke（N→N+1 §58）未跑双版本（当前仅 0.1.0 一个版本；机制上用户数据 SQLite/设置已由 M3 迁移+安装器实测保留覆盖）
## M12（2026-10-05）——安全、隐私、许可证与商业发布合规加固

**M12-A = PASS**（§77 MUST PASS 全项；一处 BLOCKED 如实：GitHub 账户级安全能力核对=MANUAL）
**M12-B = NOT COMPLETED**

### Implemented（§69 顺序）
- **Step1 Baseline Audit**（§70）：cargo audit（0 漏洞/3 warnings 分类接受）+ npm audit（5 项全 dev-only，非破坏 fix 后剩 2 项需破坏性升级=DEFER 分类 §40）+ gitleaks CI + 密钥全 history 扫描（17 处 PRIVATE KEY 命中全为审计文档文本，零真实密钥 §43）+ CSP/capabilities/IPC 50 命令面枚举
- **SecurityLimits 集中化**（§54）：security_limits.rs（更新/SMTC 元数据/map/IPC 上限单点）+ const 编译期自洽断言
- **安全回归矩阵**（§38/§60）：src-tauri/tests/security_m12.rs——凭据 URL/非 https scheme/超长 URL/路径穿越/UNC/不存在路径 resolver 拒绝 + metadata sanitize 回归 + diagnostics 无 secret 字段
- **文档套件**（§47/§50）：SECURITY_THREAT_MODEL.md（边界图+STRIDE 表+不变量）、IPC_SECURITY.md（50 命令分类核查 §38）、NETWORK_BEHAVIOR.md（出网面清单 §45/§87）、SUPPLY_CHAIN.md（cargo/npm audit 分类表 §40、SBOM-lite 命令、license 策略 §59）、DIAGNOSTICS.md（§46）、FUZZING.md（§39 targets 登记+smoke 命令）、SECURITY.md 漏洞报告节（§48）、M12_SECURITY_CHECKLIST.md（§61 手动清单）
- **CI 安全门**（§41/§58）：permissions contents:read 最小化、cargo audit job、npm audit --omit=dev 生产依赖门（0 漏洞）、cargo-deny 改 install-action

### Gates
fmt/clippy/workspace --locked 478+478（http_matrix 既有 flake 隔离 15/15）/deny 四项/ui 全套 92+build/version:check/ smoke /tauri build

### Known Limitations（§79/§103）
- GitHub 账户级能力（Secret scanning/Push protection/CodeQL 开关）= MANUAL CONFIGURATION REQUIRED（无法从代码库内核实 §42）
- 数小时 soak / Windows 10 实机 / DPI·多显示器·主题矩阵 = MANUAL VERIFICATION REQUIRED（同 M10/M11）
- dev 工具链 vite6/vitest4 升级（含 1 critical dev-only）= DEFER 已分类（docs/SUPPLY_CHAIN.md）
- GUI 级 IPC 滥用验证 = MANUAL（本环境输入注入受限；typed/权限层已 STATICALLY+RUNTIME 验证）

## M12.5（2026-10-05）——传输淡变：播放/暂停/停止全命令 1s 淡入淡出（用户口头需求，消灭"突刺"）

**状态 = PASS（单元 4/4 + CDP 端到端 3 场景 RUNTIME VERIFIED）**

### Implemented
- **淡变引擎**（aura-core output）：SharedOutputState 增 `fade_cmd`/`fade_progress_frames`/`fade_total_frames`/`fade_start_gain_bits`/`fade_gain_bits` 原子（driver→callback 命令、callback→driver 进度/当前增益）；OutputStage::render 末端线性增益缩放（起点增益=请求时定格、进度按帧域——交错采样按声道折算）
- **相位钳位**：淡变完成后相位不清除——OUT 钳在增益 0（静音）、IN 钳在 1（直通）。防 driver 应用挂起暂停前的 ≤1 循环（100ms）间隙全增益回跳爆音；`clear_fade` 保留 total、progress 推满以维持钳位
- **增益连续反转**：淡出中按播放（或淡入中按暂停）→ 新淡变从最近发布的块末增益出发（OUT: g→0 / IN: g→1），方向反转不产生增益跳变；IN 消费不设状态守卫（回调 state 快照可能滞后一拍）
- **driver 编排**（src-tauri audio.rs）：暂停/停止在 Playing 时挂起（`pending_transport`），淡出完成（progress≥total 或淡出中 EOF→Stopped）后生效；Play 立即生效、在 `set_playback_state(Playing)` 之后请求淡入（want_fade_in 武装）；`[fade-probe]` stderr 探针日志
- **测试**：transport_fade.rs×5（淡出线性+钳位静音+clear 保持、淡入至满+直通、中途反转衔接、cancel 立即恢复、**暂停落地切换块残余静音**）
- **暂停渐静后突刺修复**（用户实测反馈）：1s 淡出完成后 driver 才落地暂停，Playing→Paused 切换块把 ring 里从未被衰减的缓冲残余（暂停不清缓冲）以 1.0 起点做 10ms 淡出播放=渐静后蹦出全幅度声；停止路径无此问题（bump_generation 丢弃残余）。修复=切换块淡出起点取已发布传输增益（完成=0 静音；硬暂停=1.0 保留 10ms 保护）

### 验证（CDP 端到端，tauri build 独立产物 + --remote-debugging-port=9223）
- stop 探针：淡出期间 state 仍 playing 位置 1.31→1.84 前进；1.5s 后 stopped 归零；重播 0.51s 从头起步（fading/stopped/replay 三断言 OK）
- pause 探针：淡出→3.06s 冻结，其后 2.8s drift 0.000
- 反转实测：淡出中 play → state 保持 playing 继续出声，driver 日志 `reversal fade-in requested`

### Gates
fmt/clippy 干净/aura-core 94 全绿/workspace 全绿（watcher `event_driven_rescan_preserves_identity` 既有 flake 隔离重跑即绿）/tauri build + NSIS 8.98MiB

### Known Limitations
- 淡变期间（1s）UI 状态显示 playing、位置继续前进（淡出听得见=真实时间语义，符合设计）
- 设备采样率非 48k 时淡变时长按 48k 帧数换算（44.1k 设备 ≈1.088s，可感知差异可忽略；engine 固定 48k 输出侧一致）
- 淡变增益为线性（非等响度曲线）；人耳听感如需更自然可后续升级 S 曲线/指数
- WASAPI loopback 录音量化验证（淡出曲线数值实测）未做——工具链已备（scripts/capture_loopback.py，sounddevice 无 loopback 支持=工具边界）

## M12.6（2026-10-05）——播放模式（顺序/列表循环/单曲/随机）+ 曲终假播放修复

**状态 = PASS（引擎/前端单测 + CDP 端到端 RUNTIME VERIFIED）**

### 用户报告
1. "歌曲没有循环机制吗？顺序循环或者随机循环？"——确无（queue 只有游标制 next/previous）
2. "歌曲播完后，进度条还在计数，但是已经没有声音了"——曲终假播放

### Implemented
- **引擎曲终检测**（aura-core player）：fill_and_process 在 EOF + ring 被回调耗尽 + Playing 时置 `track_finished`（`take_track_finished` 取走即复位；有 next 时 gapless 接管不触发）——修复前引擎曲终停留 Playing，回调持续 underrun 静音而位置照常推进
- **driver 编排**：take_track_finished → 投递 Stop（复用归零语义）+ `on_track_ended` 回调（lib.rs 经 app.emit 发 `track-ended` 事件）
- **seek 帧空间换算**（E2E 挖出的存量 bug）：Seek 目标是输出帧空间、解码器按源采样率寻址——44.1k 源收 48k 帧号=时间×48/44.1（CDP 实测：seek(222s) 无效、seek(60s) 实播 65.3s 内容错位）。修复=输出帧→源帧换算 + resample_in 残片清除
- **时长链路按秒**：duration_frames→duration_seconds（open_decoder/3 个 LoadDecoder 点按源采样率换算）——修复 44.1k MP3 快照时长显示 206s（实际 224.4s）
- **前端播放模式**：queue 权威增 PlayMode（order/repeatAll/repeatOne/shuffle）+ advanceOnTrackEnded（末曲回绕/单曲重播/随机避当前/order 停止）；`track-ended` 事件监听自动前进；NowPlayingPage 新增模式按钮（→/🔁/🔂/🔀，zh/en i18n）；修复 ⏮/⏭ 无 onClick 死按钮

### 验证
- 单测：aura-core 97（natural_track_finish + seek_converts_output_frames_to_source_rate 新增）；vitest 94（queue 模式 ×5 新增）
- CDP：duration 224.39（修复前 206.16）、seek(222) 生效（修复前无效）、曲终 stopped 归零 + driver 日志 `track finished -> stop + ended event`

### Gates
fmt/clippy 干净/workspace 全绿（dns_rebind 既有 flake 隔离重跑即绿）/tauri build + NSIS 8.97MiB

### Known Limitations
- 播放模式为内存态（不持久化；重启回顺序播放）——queue 本身 §48 即 in-memory，持久化留待 queue store 演进
- shuffle 为纯随机（无随机种子列表/避免重复历史）；repeatAll 循环以双击库列表形成的队列上下文为准
- 单曲/列表循环的曲终前进走前端事件（后端已 Stop 再 play_track），曲目间有 ≤1 个快照周期的间隙（非无缝；无缝循环需引擎级 queue，超出口径）

## M12.5 加固执行（2026-10-05/06）——测试方案落地：Phase 0–5 第一轮

**状态 = PASS（三层套件全部固化并实跑；抓出并修复 3 个存量 bug；30min soak 通过）**

按 Spec/ifuyo Aura — M12.5 播放与真实旅程加固计划.md 执行（用户 /goal：加固稳定性和强健性）。

### 产出（全部入库）
- **L1 真实设备**：crates/aura-core/tests/real_device.rs（#[ignore]×4：位置推进/暂停冻结/恢复、seek 联动、曲终检测、10Hz 供数零 underrun）——本机实跑 4/4；漂移说明：外部输出接管消费端后 capture 不可见，以位置/underrun/状态断言替代（计划原文 capture 断言）。首跑即抓 harness 漏 set_playback_state——回调按 Stopped 静音，再次印证状态传播是真实接缝
- **L2 播放矩阵**：scripts/verify-playback.mjs（C01+P01-P09）10/10
- **L2 库矩阵**：scripts/gen-fixtures.py（ffmpeg lavfi：ID3v2.3/2.4+封面 mp3、无标签、8KB 长标签、Unicode🎵、FLAC PICTURE、OGG → tests/fixtures/×7）+ scripts/verify-library.mjs（L01-L13）13/13
- **L2 设置回环**：scripts/verify-settings.mjs（S01-S05：读→改→写后回读→硬杀→重启回读）5/5
- **Phase 4 soak**：scripts/soak.mjs（随机传输操作流+30s 采样 RSS/句柄→JSONL）——30min 实跑 **446 ops、RSS 40→42MB、句柄 437→437、零操作错误、进程存活**=无泄漏无崩溃
- **QA 隔离**：lib.rs 增 AURA_DATA_DIR 环境变量覆盖（QA 全走临时资料目录）；scripts/lib/qa-app.mjs（spawn+CDP attach 探测，规避启动竞态空页 origin 报错）

### 实跑抓出的存量 bug（3 个，均带回归）
1. **seek 帧空间不换算**（engine）：44.1k 源收 48k 帧号=时间×48/44.1——近端 seek 超界失败、远端内容与进度错位（已随 M12.6 提交 98054f2）
2. **library_stats 列名错误恒 0**：规范化 schema 后无 artist/album 列，`COUNT(DISTINCT artist)` 报错被 unwrap_or(0) 吞——artists/albums 自 M3 迁移以来恒 0。修正 + aura-library/tests/schema_contract.rs 契约测试锁列形态
3. **设置写后回读旧值**：save_settings 只写盘不更新内存缓存；set_* 持锁调用点拆出 write_settings（防重入死锁），save_settings 写盘+同步缓存

### Gates
fmt/clippy 干净/workspace 全绿（dns_rebind 既有 flake 隔离重跑即绿）/aura-core 99+94 vitest/schema_contract 2/real_device 4（ignored 套件本机实跑）

### Known Limitations（如实）
- 嫌疑区未勾项保留：crossfade/gapless 真实输出 capture 断言、Radio/HTTP 流接真实输出、设备热插拔实测、SMTC 真实面板、播放列表重启持久、升级 N→N+1、Mini Player、睡眠/恢复
- EQ/响度真实回调逐项 PCM 断言未做（DSP 单测已有）
- soak 第一轮用户 Music 因脚本时序（SCAN_RUNNING 并发拒绝）未装载，仅 fixtures 7 首高周转；脚本已修（顺序扫+等待），下一轮覆盖真实曲库
- 流程教训入计划文档 §6：构建失败未察觉导致 QA 跑旧 exe 把 fixture 扫入真实曲库（已清理还原）；plain cargo build 产物 origin 报错（D-075 复发）

## M12.7（2026-10-06）——界面交互反馈全面优化（用户需求：界面死板/交互差/要给用户反馈）

**状态 = PASS（tsc/eslint/vitest 95 全绿 + tauri build + CDP 实测交互生效 + 截图目测）**

### 全局层
- notice → toast 三态（success✓/infoℹ/error⚠）：语义 role（status/alert）、success/info 3.5s 自动消失、error 停留；noticeSeq 重放动画+重置计时；toast() 便捷函数
- CSS 交互层：所有可点元素统一 hover/active/focus 三态+按压缩放+过渡；主播放键悬停光晕、播放/暂停切换弹跳动画；range 滑块 accent 主题+拇指悬停放大；palette/settings/queue 抽屉入场动画；视图切换淡入；侧栏激活项左强调条；扫描条不定长流光；补齐 ghost-btn/primary-btn 缺失样式

### 组件层
- **曲库**：行 cursor:pointer（原 default！）、悬停浮现 ▶ 播放键（单击即播）、正在播放行玫瑰色+跳动 EQ 条（暂停静止；路径归一化匹配）、省略文本 tooltip、扫描完成 toast 三态（成功/部分失败/全失败）
- **正在播放**：播放模式切换即时 toast、传输键 title 悬停提示
- **队列**：清空成功 toast、当前曲强调色+悬停浮现移除键
- **设置**：保存成功/失败 toast

### 验证
vitest 95（+1 clearQueue 反馈断言）/CDP 实测：模式 toast 文案出现、cursor:pointer、悬停键渲染、playing 行+EQ 生效；capture-screens 截图目测布局无破损

### Known Limitations
- toast 单条（新 toast 替换旧的）——堆叠留待有真实需求再做
- 音量/进度滑块无数值气泡（accent+拇指反馈已够；气泡可后续加）

## M13.1 第一轮（2026-10-06）——对齐 QQ 音乐的界面优化（用户需求）

**状态 = PASS（vitest 95 + workspace（dns_rebind 既有 flake 隔离绿）+ clippy 0 + tauri build + CDP 实测 + 截图目测）**

### 范式补齐：全局播放条（PlayerBar）
- 常驻底栏（除正在播放页——该页已有全尺寸控件）：封面缩略 44px + 标题/状态（点击进正在播放页）+ ⏮⏸⏭（应用层队列语义）+ 迷你 seek（rAF 本地插值，可拖）+ 播放模式（toast）+ 音量
- queue 抽屉与 toast 经 data-view 选择器自动让出底栏高度
- 首版两行叠放被 64px 裁切（CDP 实测抓出）→ 单行布局修复

### 修复
- **无标签曲目显示完整路径**（用户截图实锤）：曲库行/播放条/正在播放大标题统一回退文件名 stem（fileBaseName 去目录+verbatim `\?\` 前缀+扩展名）；tooltip 保留全路径；后端 play_input_impl Local 分支同口径回退（SMTC/快照受益），title_fallback_tests×3
- **⌖（Open on Map）按钮隐式 grid 行溢出**：行网格 5 列 6 元素——按钮落到 52px 行高外不可见；改 6 列
- **截图"空行"排查**：真实 DB 仅 2 条干净记录；当前构建 CDP 数 DOM 无空行——判定为旧构建/扫描瞬态，非存量 bug

### 曲库专业化
- 表头（标题/艺术家/专辑/时长，网格与行对齐）
- useCurrentTrackId 提取共用（PlayerBar/NowPlayingPage）

### Known Limitations / 后续轮次
- 表头暂不可点排序（排序仍走下拉）——点表头排序留待下轮
- 封面模糊背景/主色沉浸、歌词（LRC/ID3）、封面真实缩略图列 = 第二/三轮

## M13.2 第二轮（2026-10-07）——封面与沉浸感（对齐 QQ 音乐）

**状态 = PASS（vitest 100 + verify-library 15/15 含 L14/L15 新断言 + 截图目测）**

### Implemented
- **曲库封面真实缩略图列**：artworkCache（Map+负缓存+并发去重+512 FIFO+library-changed 全量失效，vitest×5）；ArtworkThumb 32px 组件仅对 hasArtwork 行发 IPC、虚拟列表可视行自然限流；▣ 字符占位退役（无封面=♪）
- **正在播放封面模糊沉浸背景**：当前封面 blur(72px)+saturate(1.25)+scale(1.35) 铺满+径向暗化遮罩+600ms 淡入（QQ 式 ambient）；无封面回退 Aurora；页面内容 z-index 提层
- **表头按钮化排序**：标题/艺术家/专辑/时长点击切换字段+活动列 ◆；下拉排序保留（year/dateAdded 仍走下拉）

### 验证
verify-library L14（5 个带封面 fixture → 5 个 `<img>` 渲染）/L15（播放后 .np-ambient 渲染）新增，15/15；截图目测 ambient 光晕生效（fixture 纯色封面=蓝晕，真实封面更丰富）

### Known Limitations / 后续
- 表头排序暂无方向反转（后端 SortKey 无 desc——需后端扩展）
- 封面主色驱动 Aurora 渐变（进阶沉浸）未做
- 歌词（LRC/ID3 USLT）= 第三轮，开始前定 lofty crate vs 自研解析
- 教训：缩略图功能提交前忘跑 tauri build，旧 exe 使 verify-library L14/L15 假 FAIL——QA 前必须确认 exe 与源码同代

## M13.3 第三轮（2026-10-07）——歌词显示

**状态 = PASS（Rust 单测×6 + verify-library 18/18 含 L16/L17/L18 + DOM 实测：4 行渲染、当前位置高亮、meta 行生效）**

### Implemented
- **后端 lyrics.rs**：lyrics_for_path——同目录同名 `.lrc` 优先（带时间戳，synced=true）→ lofty `ItemKey::Lyrics` 标签回退（通常纯文本 synced=false）→ None；LRC 解析覆盖多时间戳/offset(毫秒→秒)/元数据行跳过/无时间戳降级/单行损坏不致命；UTF-8/BOM only（GBK 后置需 encoding_rs 供应链审批）；MAX_LYRICS_BYTES=512KB 入 SecurityLimits
- **IPC**：`lyrics_for_track` + `track_meta`（无歌词回退面板）
- **前端**：正在播放页左右分栏（有歌词时）——右歌词面板：同步行随进度高亮（rAF 插值位置）+ 平滑居中滚动（reducedMotion 降级）+ 点击行 seek；纯文本静态滚动；无歌词回退"艺术家 · 专辑"meta 行；窄屏上下叠放
- **取舍落地**：复用依赖树内 lofty 0.22（M3 已引入、M12 已审计）——自研解析零收益；SYLT 不做（lofty 支持弱+罕见）；ffmpeg `lyrics=` 元数据实测不落 USLT 帧 → fixture 用应用同款 lofty 写入（examples/set_uslt，往返一致）

### 验证
Rust lyrics_tests×6（时间戳排序/多时间戳+offset/纯文本降级/垃圾输入/标签回退/LRC 内容标签）；verify-library L16（LRC synced 4 行）/L17（USLT 3 行）/L18（无歌词 null）+ DOM 实测（pane/4 行/active 第三句@2.7s）+ 截图目测分栏布局

### Known Limitations
- GBK 编码 .lrc 不支持（乱码兜底=视为无歌词）；SYLT 同步歌词标签不做
- 歌词无缓存（每次播放重读，本地 FS 开销可忽略）
- 逐字卡拉OK式 LRC（<mm:ss.xx> 词级时间戳）按行级处理

## M15a（2026-10-07）——格式与兼容（音乐管理器升级计划第一期）

**状态 = PASS（verify-library 22/22 + Rust 单测：cue×4/mojibake×5/gbk_lrc×1/title_fallback×3 + workspace 全绿（dns_rebind 既有 flake 隔离绿）+ clippy 0）**

### 1. 格式支持矩阵（symphonia feature 开启）
- `aac`/`alac`/`isomp4`/`aiff` feature → **M4A(AAC)/ALAC(M4A)/AIFF** 真实解码；扫描器补 aac/aiff 扩展名
- 此前状态：扫描器已列 m4a/alac 扩展名但解码 feature 未开（扫到放不出=半接线）
- fixtures：fixture.m4a/fixture-alac.m4a/fixture.aiff；verify L19 播放矩阵（轮询断言 3s 容忍首播初始化）
- Opus：symphonia 0.6 无 codec（registry 实测），后置

### 2. 编码检测（encoding_rs，已登记 SUPPLY_CHAIN：audit 复扫 0 漏洞/deny 过）
- **GBK 标签乱码修复**：ID3v2.3 Latin-1 声明按字节透明 → GBK 二次解码；启发式双轨（无错误 + ≥2 CJK 才采用，欧洲语文不命中）；AppSettings.mojibake_fix 默认开（启动/save_settings 同步原子量）+ 设置面板开关（zh/en）
- **GBK .lrc**：UTF-8 失败按 GBK 解码（M13.3 后置项并入）
- fixture-gbk.mp3（set_tag_latin1 示例写 Latin-1 形态 GBK 字节标题）+ verify L20：扫描后标题=周杰伦

### 3. CUE 整轨虚拟子轨
- **cue.rs 解析器**：TRACK/INDEX01/TITLE/PERFORMER/FILE/REM、数据轨排除、sheet 级继承、UTF-8→GBK 回退、≥2 AUDIO 子轨才展开；单测×4
- **导入**：schema V3（tracks.cue_start/cue_duration_seconds）；import_file 展开虚拟子轨（identity=父#轨道号、合成路径 path#NN、元数据继承、时长=相邻 INDEX 差/文件尾、复用主轨封面）；子轨成功→整轨主行 is_active=0 隐藏
- **mark_missing 前缀覆盖规则**：`整轨路径#` 开头的合成路径由整轨 seen 覆盖（否则未变化跳过轮子轨被误停用——L21 实测抓出）
- **SubRangeDecoder**（aura-core）：任意 Decoder 限制到 [start, start+total) 帧区间
- **播放**：open_decoder_sub（秒×源采样率包装，时长取 CUE 声明）+ play_track 识别合成路径
- fixture-cue.flac(12s)+.cue（6 子轨×2s 中文标题）+ verify L21/L22

### 4. 过程中抓出的存量 bug
- **换曲吞掉新曲目播放**（真用户 bug）：play_track 在上一首停止淡出未落地窗口内到达，新 Play 被"淡出中反转"分支吞掉（e79436b）——L19 探针序列实测抓出

### Gates
fmt/clippy 0/workspace 全绿（dns_rebind 既有 flake 隔离绿）/aura-core 103 + aura-library 52（cue×4+mojibake×5+gbk_lrc×1+cue_import×1 新增）/vitest 100/verify-library **22/22**/tauri build

### Known Limitations
- GBK .lrc 已支持；SYLT/TTML 不做（口径同 M13.3）
- CUE 仅 INDEX 01 主起点（pregap 00 忽略）；跨 FILE 多文件 CUE 不支持（单文件整轨主流场景）
- 子轨 seek 精度受容器 seek 粒度影响（Coarse）；设备选择/输出设备跟随默认设备（M15b/M15c 范围）

## M15a.1（2026-10-10）——第 1 批止血：设计 token 链 / 分析接线 / 响度真喂 / 曲库分页 / Mini Player / 文案

**状态 = PASS（verify-library 23/23 含新 L23 + vitest 104 + typecheck/eslint 干净 + fmt/clippy -D warnings 0 + workspace 全绿（一个既有 load-sensitive flake 见下）+ tauri build + 真实构建截图）**

### 1. 设计 token 生成链（根因级）
- 症状：样式表引用 15 个 CSS 变量（`--color-text/-muted`、`--surface-low/high/border`、`--radius-sm/md`、`--font-size-xs..xl`、`--motion-fast/normal`、`--ease-out`）**从未定义且无 fallback** → 按 CSS 级联语义整条声明失效：面板背景/边框透明、圆角归 0、字号阶梯塌平、M12.7 那轮 transition/animation 全部静默不跑
- 根因：`generate-tokens.mjs` 只遍历 `brand.colors` 输出 5 个变量，`tokens.ts` 的 surface/text/radius/typeScale/motion 组从未进 CSS；`tokens.test.ts` 只断言 TS 侧 → 门禁全绿而消费端断裂
- 修复：生成器解析 tokens.ts 全部组 + 显式「CSS 变量 ← token 组.键」映射表（缺任一 token fail-closed）；补 `radius.sm=8px`、`ease.out`；base.css 硬编码 rgba/px 改吃 token（同值，不改视觉）
- 新门禁：tokens.test.ts 增"样式表引用的每个 var(--x) 必须已定义"（`import.meta.glob ?raw` 扫全部 CSS，注释不计，glob 空转有断言防护）——首轮即抓到 `var(--color-*)` 注释误报并修正

### 2. M8 分析接线（Music Map 与响度归一的数据源）
- 事实：`AnalysisService::analyze_tracks` 全仓只被自身单测调用，src-tauri 只有 3 个只读查询命令 → **应用从不分析**，README「Analysis ✅ / Music Map ✅」在真实产品里不可达（D-122 只记了"无分析按钮"，未记"无分析"）
- 新增 `src-tauri/src/analysis_host.rs`：后台单 worker（自有 DB 连接，WAL+busy_timeout 与扫描/查询并发）+ 队列 + 一次性取消令牌（每轮换 `CancelToken`）+ `analysis-progress` 事件（每 5 首/收尾上抛）
- 队列口径：活跃 + 有真实路径 + `track_analysis` 无行；**CUE 虚拟子轨排除**（合成路径 `file#NN` 磁盘上不存在，整轨主行已随子轨展开停用）
- 落库门：Cancelled 不记账；Completed 必须 `is_valid_completed()`（§335 同门，空特征结果不覆盖既有）
- 触发：启动（`settings.analysis_auto` 默认开）+ 扫描完成后 + 设置面板手动；取消经 `cancel_analysis`
- IPC：`analysis_status`（进度 + analyzed/pending 覆盖度）/`request_analysis`/`cancel_analysis`；设置面板「音乐分析」区（覆盖度/进度 + 分析/停止 + 自动开关）；Music Map 空态按"库为空"（`map.addMusicFirst` 此前定义了从未被引用）与"未分析"分文案并给一键分析（顺带用上零调用的 `library_stats`）
- 单测×3：CUE 子轨与已分析行不入队 / 路径解析只取活跃 path / 空特征 Completed 不落库

### 3. 响度标准化真喂 LUFS
- 症状：`audio.rs` 唯一的 `set_track_loudness` 调用传 `TrackLoudness::missing()`（integrated_lufs=TARGET_LUFS）→ 增益恒 0dB，设置里"响度归一化"是假开关；分析出的 LUFS 此前只流向 Music Map
- 修复：引擎增 `set_loudness_enabled`（只翻开关、保留元数据）+ `loudness_enabled()`；`Control::LoadDecoder` 携带 `TrackLoudness`（换曲必须一并更新——`load_track` 不重置元数据，沿用上一首会按错曲算增益）；`play_track` 取 Completed + loudness Available 的实测 LUFS/true peak，其余一律 missing()（不猜测）
- 回归：`loudness_toggle_applies_metadata_instead_of_wiping_it`（-20 LUFS → +4dB 实测；捕获缓冲是累加的，断言前须 `clear_capture`）
- 端到端证据：verify L23 真实 fixture LUFS=-24.82 / true peak=-21.06 / key=A Minor(8A) 入库可读

### 4. 曲库分页
- `LibraryPage` 此前恒 `offset=0, limit=200` → 200 首之外永久不可见（后端 `library_list/library_search` 一直支持 offset，clamp 1..500）；改为续拉 +「加载更多曲目」（满一页才露入口；失败保留已加载页可重试）

### 5. Mini Player
- `MiniPlayer.tsx` 的 7 个 class 在 app.css **一个都没有**（340×110 块级竖排必溢出）；补齐样式（封面位/信息列 ellipsis/控制组/焦点环）
- ⏮/⏭ 此前**没有 onClick**（点了没反应的死按钮）：经 `emitTo('main','integration://command')` 复用 SMTC/托盘同一条队列语义路径（D-123）——mini 与主窗是不同 JS realm，队列真相在主窗；状态行由裸 ▶/⏸ 字符改文案
  - **更正（M15a.3 全量复查）**：本条初版写的事件名是 `integration-command`，而监听侧（`ipc.listenIntegrationCommand`）与 Rust 侧（tray/worker）用的是 `integration://command` ⇒ 当批"已接通"结论为**假**，⏮/⏭ 实际仍无响应。现已对齐事件名；仍未在真机验证（需主窗常驻的 QueueDrawer 监听 + release 构建）。

### 6. 文案与 i18n 门禁
- 去里程碑黑话：`map.analyzeFirst`"先在 M8 分析"→ 指向设置「音乐分析」；`map.similarityMetric`"（M8 嵌入）"→"（本地音频特征）"
- 补缺失键：`common.close`、`library.openUrl`（此前 zh/en 均未定义 → key 字符串直接渲染给用户/读屏）、en `settings.mojibake`（此前只有 zh）
- 新门禁 `ui/src/i18n/i18n.test.ts`×3：zh/en 键集一致 / 代码里 188 处字面 `t('key')` 全部存在 / 用户可见文案不得含 `\bM\d+\b`

### 过程中抓出的既有 bug
- `analysis_ipc.rs:29` 存在性检查写成 `SELECT track_id FROM tracks`（该表列名是 `id`）→ `track_analysis_summary` 自写下起从未成功执行过；因分析从不存在而无人调用，新 L23 断言暴露并修复
- `dns_rebind_to_private_ip_denied` **确认为真实 load-sensitive flake**（单包 15/15 过；`cargo test --workspace` 并行下挂于 http_matrix.rs:163）——PROGRESS 此前"隔离重跑即绿"的记录成立，但根因仍未定位（100ms 超时预算 vs CPU 争用），列入第 3 批

### Gates
fmt / clippy --workspace --all-targets -D warnings 0 / workspace 45 suites ok（新增 analysis_host×3 + core loudness×1，除上述既有 flake）/ vitest 104（+4）/ typecheck + eslint 干净 / **verify-library 23/23**（L23 = 分析端到端新断言）/ tauri build + capture-screens 真实构建截图（修复前后对比：nav 活动态药丸底、封面卡面、主键圆底、字号层级恢复）

### Known Limitations
- CUE 整轨专辑不进分析 → 其子轨在 Music Map 与响度归一上缺席（需按整轨文件分析 + 子轨共享父响度，未做）
- 分页只有"加载更多"（无自动续拉、无总数）；>500 首需多次点击（后端每页 clamp 500）
- 响度开关对**未分析**曲目仍无效果（数据源缺失，如实不归一）；不读文件内 ReplayGain 标签
- Mini ⏮/⏭ 依赖 `core:default` 的 emit-to 授权与主窗 QueueDrawer 常驻监听（抽屉关闭时组件仍在，监听有效）
- 分析为单 worker 顺序、无优先级/无断点续跑；已完成行不重算，失败行下次请求重试
- `capture-screens.ps1` 产出的 02-library.png 实际拍到"正在播放"页（导航未生效即截图）——docs/screenshots 刷新归第 3 批

## M15a.2（2026-10-10）——Dreamscape 去建模改光场 + 沉浸/桌面歌词

**状态 = 部分 PASS（Rust 门禁全绿 + vitest 107 + verify-library 23/23 基线；Y06/Y07 播放态反应与悬浮歌词实机点击穿透 **未验证**，见 Known Limitations）**

### 1. Dreamscape 撤除 3D 建模，保留确定性参数源
- 判断依据：真实 manifest 数据显示"画面差"一半不在渲染层——`sunIntensity=0`（光照曲线长相位无光）、`amplitude 0.65 / 世界尺度 ±200`（相对起伏 <0.5% ⇒ 一张纸）、`landmarks=4`、`saturation .43 / luminance .46`（§42 品牌安全区）。用户决定：移除建模，只做光影/星野/节奏画面。
- 删除：`world.rs`(634，高度场/放置规则/SpatialHash/Landmark/预算)、`region.rs`(449，**crate 外零调用者**=净收益)；`biome.rs` 剥掉 `ObjectFamily`/`object_families`/`terrain_profile`/`density_scale`/`effective_density`，保留 kind/id/mist/lighting/select/anchor。
- `SceneManifest` → **`LightField`**，`MANIFEST_VERSION` 1→2：`{manifestVersion, worldId, seed, fieldSeed, motif, palette, light, atmosphere, params}`；`landmarks`/`instances`/`terrain` 几何全部撤除。
- **`Genome` 持久化字段一个不删**，1:1 重映射为 `FieldParams`：`terrain_scale→curtainDepth`、`flora_density→moteDensity`、`mist_density→veil`、`light_softness→glowSoftness`、`water_presence→mirror`、`object_scale_bias→starScale`、`horizon_height→horizon`。原因：`payload_hash` 是对已解析结构体**重新序列化**再比对（snapshot.rs:145-150/169），删字段 ⇒ 旧 `dreamscape.json` 判 Corrupt ⇒ dreamscape.rs:36-43 只打一行 stderr 就静默回退默认世界（用户世界无声丢失）。**规则：持久化结构只加不删。**
- `biome_id` 取值集与 `GENERATION_VERSION` 未动（`world_identity` 绑定它们，动了旧快照全量失效）。
- 单测：manifest×5（hash 确定性/输入敏感/params 镜像 genome/serde 往返且无几何残留/母题稳定）、fuzz 与 perf 测试改写为光场口径；aura-synth 65+4 全绿。
- 渲染层：删 `scene.ts`+`DreamscapeRenderer.ts`（Three.js），新增 `visual/dreamscape/LightFieldRenderer.ts`——裸 WebGL2 单张全屏片元（极光幕 fbm 流苏 / 星野 / 光尘 / 地平镜面 / ACES + 暗角 + 抖动去色带）；`engine.ts`（质量档/AutoQuality/节流/生命周期，零 WebGL）原样保留并续用，`terrainSegments/particleScale/postEnabled` 更名 `fieldDetail/moteDensity/glowEnabled`。**`three` 与 `@types/three` 已从依赖移除**（仅这两个文件用过），产物无 three 标记。

### 2. 实跑抓出的 4 个 bug（原型截图看不出来，必须接进产品跑）
- **节流记账错**：`start()` 每帧把 `lastFrameMs` 刷成"上次 rAF 回调时间"，而 `shouldDraw` 要"上次真正绘制时间" ⇒ `now-last`≈16ms 永远过不了 100ms 门槛 ⇒ **光场只画一帧**。证据：`loopTicks` 卡在 1。改为 `shouldDraw` 通过时自己记账。
- **promise 链无 catch**：循环内异步异常被静默吞，画面不画而状态是 ready，无从查起。已补 catch 并写进 `canvas.dataset.fieldError`。
- **`resize()` 在重写时丢失**：画布停在默认 300×150。
- **fps 诊断是假的**：读的是 GPU 提交耗时，恒显示 1000。改为 1 秒滑窗实测绘制帧率（现显示 idle 10 / 播放 27）。
- 另：我自己的一轮视觉"改进"把色相改成最短弧插值，348°→108° 穿过橙 ⇒ 低明度下糊成锈红，星星也放大成白块；**已回退**，走 base→accent 的下降弧（紫/蓝侧）。

### 3. 电平 tap（B 档）与零门控语义
- 新增 `aura-core/audio/level.rs`：三条一极 IIR 分频（LP150 / LP2500−LP150 / x−LP2500）+ 块内 RMS；取样点在 `flush_pending` 唯一推送处（含 EQ/响度结果，不含用户音量）；经 `SharedOutputState` 新增 4 原子量发布（非有限值写入前归 0，防 NaN 永久毒化）；新 IPC `player_level`。零分配零锁，回调路径未动。
- 单测 6（静音/空块回落/60Hz 偏低频/8kHz 偏高频/NaN 不毒化/振幅单调）+ 引擎集成 2。
- **语义 = 零门控**：音量 0 ⇒ 电平归零（画面静止）；任何非零音量 ⇒ 电平不被衰减。理由：音量是用户为房间设的，不该指挥画面强弱；按幅度缩放的初版实测让 2% 音量下 level≈0.003，画面直接死掉（Y07 当时 FAIL 暴露）。
- **纠正一个假通过的测试**：原断言"半音量 < 满音量"既非期望语义、又会因取样相位不同偶然通过。改为断言"非零音量下电平不被衰减" + "再次静音必须归零"。

### 4. 遥测接通与"晃眼"修复
- `TrackAnalysisSummary` 补 `first_beat_offset_seconds`（+单测）；前端 `trackFeatures` 投影 bpm/首拍偏移/energy/brightness。
- 节拍相位 `beat=(pos−firstBeatOffset)×bpm/60`，包络 `exp(−3.2·frac)`；bpm 缺失或不在 [40,220] ⇒ **pulse=0，不假装踩点**；未分析特征 ⇒ 中性 0.5（不伪造）。
- 色相不绑调性：真实音乐上 `musical_key/camelot_key` 两首都是 null（D-101 熵门偏严），改绑 centroid/brightness。
- **晃眼根因 = 逐块 RMS 原值直接当曝光**（10–30fps 采样下就是噪声）。修复：遥测进着色器前非对称跟随（level 攻击 140ms/释放 380ms；频段各 100–150ms/280–400ms；曲目级特征 1.2s），并整体压调制深度（boost `pulse*1.15+level*0.85`→`0.50+0.30`；全局明度 ±25%→±10%；自主流速降到约 1/3）。

### 5. 歌词
- **梦境页地面舞台**：正在唱的一句偏左上、下一句预读偏右下（经典卡拉 OK 对角），行切换命令式重排（`loopTicks` 同族手法，高频不进 React）。verify 断言 Y01–Y05 **PASS**：`seek 0.6s`→「第一句/第二句」、`seek 2.6s`→「第三句/第四句」、无歌词曲目显示"这首歌没有歌词"而非留白、光场 27fps 无编译错误。
- **桌面悬浮歌词**（`?view=lyrics` 第二窗，复用 Mini Player 的多窗形态）：透明+无边框+置顶+跳过任务栏；播放条"总时长"与播放模式之间新增「词」按钮开/关；未锁定可拖（`data-tauri-drag-region`）、锁定不可拖；关闭=隐藏不停播放。**锁定不用 `setIgnoreCursorEvents`**——整窗穿透会让锁定态下的解锁按钮也点不到（正是要避免的）；改依赖分层窗口的逐像素 alpha 命中（透明区天然穿透、文字与按钮可点）。
- 命名坦白：用户可见文案已回到"梦境/Dreamscape"，但**内部标识仍是 dreamscape**（view id / stores / IPC / 目录）——纯改名无行为变更，待确认后再做。

### 6. 自托管远程库（服务器侧先行）
- 新增 `scripts/gen-ifuyo-manifest.py`：扫目录 → ffprobe/mutagen 取元数据 → 产出 `aura-manifest.json`（v1：id/title/artist/album/url/durationSeconds/sizeBytes/format/mimeType/etag）。实测 fixture 13/13 走 ffprobe；修了 Windows 下按 gbk 解码 ffprobe 输出的崩溃。
- 客户端未做。硬约束：CSP `connect-src ipc: http://ipc.localhost` ⇒ 前端不能直接 fetch，列表必须走 Rust 命令；`.m3u8` 会被 resolver 当 HLS 拒（`resolver.rs:61-66`），故清单用 JSON 而非 m3u8。
- 缓存现状澄清（回应用户提问）：`aura-cache` 在 `src-tauri` 零引用、直链策略是 `SessionOnly`（`resolver.rs:80`，只允许 32MiB 内存、硬门禁拒绝落盘）⇒ "先查本地缓存/断网重播"目前**不成立**；缓存在 `app_data` 内容哈希分块 + 独立 SQLite，默认 2GiB，**不在**音乐库文件夹里（混进去会被扫描器当音乐）。

### Gates
`cargo fmt --check` / `clippy --workspace --all-targets -D warnings` 0 / aura-synth 65+4 / aura-analysis 107 / aura-core engine 16（含电平 2 条）/ ui typecheck+lint 干净 / vitest 107（含 i18n 三门禁与 CSS 变量门禁）/ verify-library 基线 23/23 + 歌词 Y01–Y05 PASS。**唯一失败仍是既有 load-sensitive flake `dns_rebind_to_private_ip_denied`**（单包 15/15 过、全量并行挂）。

### Known Limitations（如实）
- **Y06/Y07 未重跑**：零门控与平滑改动的播放态反应（电平确实在变、逐帧抖动确实变小）尚无实测数据。
- **悬浮歌词的"锁定后不挡下方点击"未实机验证**——依赖 WebView2 是否真做逐像素 alpha 命中；若不做，需退到整窗 `setIgnoreCursorEvents` + 把解锁入口移到播放条。
- 光场视觉仍是"夜空+淡极光"，未达"炫酷"；地平线仍偏硬、下半屏偏空。音乐反应只在播放态成立。
- 调性检测在真实音乐上未出结果（两首均 null），Music Map 的 key 配色与未来 Auto-DJ 的调性连贯因此缺数据源。
- CUE 整轨专辑不进分析 ⇒ 其子轨缺席 Music Map 与响度归一。
- 内部命名仍为 dreamscape（歌词视图/悬浮歌词宿主）。

## M15a.3（2026-10-10）——全量复查：假结论纠正 + 光场上下文恢复 + 并发/陈旧态止血

来源：用户要求"全面检查还有哪些代码问题需要优化、修复，软件本身与用户体验上还有什么可优化"。逐条**回源码验证**后修复，未验证的只登记不宣称。

### 1. 纠正自己写过的假结论（最高优先）
- **Mini Player ⏮/⏭ 其实还是死的**：M15a.1 记录的 `emitTo('main','integration-command')` 与监听/Rust 侧的 `integration://command` 不是同一个事件名（`ui/src/ipc/client.ts:155`、`integration/tray.rs:372/379`、`integration/worker.rs:258/265`）——名字错一个冒号，"已复用 SMTC 同一条路径"的结论就是假的。已对齐（真机验证仍待 release 构建）。
- **电平取样点注释撒谎**：`player.rs` `flush_pending` 注释写"含 EQ/响度/限幅结果"，实际 EQ/`track_gain`/limiter 全在输出级（`output/mod.rs:370/396`，消费侧），tap 取的是解码→声道映射→重采样后的信号。已如实改为"取样在输出级之前，不含 EQ/响度/用户音量"，与 `fill_and_process` 的零门控语义一致（画面不跟音量走，但也不跟 EQ 走——这是有意的取舍，不是遗漏）。
- **README/ARCHITECTURE 仍写 Three.js**：v2 已删 three 依赖与 3D 重建，文档是最后一处还在宣称旧架构的地方。`README.md:31/60` + `ARCHITECTURE.md:9` 与两节 Dreamscape（生成核心/渲染集成）按当前文件树重写：`world.rs`/`region.rs` 退役、`manifest.rs`→`LightField`（`field-hash/v2`、MANIFEST_VERSION=2）、渲染器→`LightFieldRenderer.ts`、新增"音乐驱动/频率边界/上下文丢失"三条。截图 `docs/screenshots/10・12` 仍是 v1 3D 世界——**已在 README 显式标注待重截**（不悄悄留着骗人）。

### 2. 光场上下文丢失恢复（§151，v1 有、v2 我写漏了）
- 现象：TDR/显卡重置/睡眠唤醒后 WebGL 上下文丢失，旧代码只在 `create()` 建一次 program/VAO，丢失后**静默冻在最后一帧**，而 IPC 轮询照常——用户看到的是"梦境不动了"。
- `LightFieldRenderer`：`program/vao` 改可空可重建，抽出 `build()`（首建与恢复共用：编译/链接/VAO/uniform 解析/viewport）；`webglcontextlost` → `preventDefault()`（否则浏览器不再发 restored）+ 停循环 + 记 `resumeOnRestore` + `data-field-error=context-lost`；`webglcontextrestored` → `build()` 成功即清错误、按原意愿自动续跑 rAF，失败则留在 fallback。`render()` 在丢失期直接返回；`dispose()` 摘监听并置空句柄。
- `DreamscapeView`：`onContextChange` → 丢失期切 CSS `fallback` 态（有氛围而不是黑屏），恢复回 `ready`。

### 3. 并发与陈旧态
- `analysis_host.request_pending`：`load(busy)`→send 之间两个并发请求会双双通过 → 同一批曲目被排队分析两遍。改 **CAS 认领** `busy`，认领后所有早退路径（空清单/DB 错误/worker 已停）显式归还，避免状态永久卡"在跑"。
- `PENDING_COUNT_SQL` 与 `PENDING_WHERE` 是同一谓词的两份拷贝（会漂移）→ 计数改由 `PENDING_WHERE` 组装（`pending_count()`），单一来源。
- `LibraryPage.loadMore`：切排序/搜索/重扫会重置第 0 页，而在途的"加载更多"仍会把**旧条件的尾巴**拼进新结果。加 `pageGenRef` 代数，回包时代码不符即丢弃（含 `setLoadingMore`）。
- `SettingsPanel`：`analyzed/pending` 只在开面板时取一次 → 跑批过程中"已分析 N 首"钉住不动，看起来像没在分析。按进度事件节流重取（后端已 5 首一发，这里 25 首一刷 + 收尾必刷），不引入轮询。

### 4. 死代码与诊断残留
- `engine.ts`：`Lifecycle`/`GenerationGate`/`SmoothedValue`/`lerp` 只被自己的单测引用，生产侧无一调用（v1 三渲染器的遗产）→ 删除，连带 6 条只测自身的用例（107→101）。
- `LightFieldRenderer`：`lastRenderMs`/`averageFrameMs`/`resetStats` 及 `frameTotalMs`/`lastMs`/`frameCount` 记账删除——正是"用 GPU 提交耗时驱动 AutoQuality"那个错判的来源，留着只会诱使人再用它。
- `DreamscapeView`：删 `canvas.dataset.loopTicks`（M15a.2 排查期的一次性计数器）。
- `DesktopLyrics` 锁按钮：`🔒/🔓` 加 VS15（`\uFE0E`）强制单色文本呈现，与 `⏮/⏭` 同一 glyph 体系，不引入彩底 emoji（`aria-label` 已是 i18n 文案，读屏不受影响）。

### 5. 门禁与工具
- `verify-library.mjs` L23 的 `analysis_status` 轮询裸奔：一次瞬态 DB/IPC 抛错会中止整条断言 → 包 try/catch，失败等下一轮。
- `cargo fmt --all` + `clippy --workspace --all-targets -- -D warnings` 通过；`typecheck`/`lint` 通过；vitest **101/101**；`cargo test --workspace` 见下。

### Known Limitations（本批如实登记，未修）
- `dreamscape_set_quality` 是**校验后丢弃**的桩（`let _ = state;` 直接 Ok）：既没落库也没渲染器读回，UI 也没有任何入口调它。要么接成真设置（Settings 档位 + 视图消费 + 持久化），要么删命令——当前状态是"契约存在但不兑现"。
- 播放态光场每帧拉 `player_level`/`player_position`（30fps），而电平数据源是 100ms 驱动块 ⇒ 约 2/3 的取回是重复值。平滑器能吃重复样本，故只是浪费 IPC 往返，未改（改前先测）。
- 跨语言常量对（Rust `MANIFEST_VERSION` ↔ TS `manifestVersion !== 2`、字段名 ↔ `between(...)`）只靠注释与人工同步，没有 codegen/契约测试兜底。
- 未做（用户已知的既有项）：调性检测在真实音乐上仍出 null；CUE 整轨仍不进分析；悬浮歌词"锁定后逐像素穿透"未经实机验证；release 产物仍未重建（另一个 Aura 实例占着单实例锁与 exe），故 Y06/Y07 反应数据继续缺席。
- 仓库以 CRLF 存储（全局 `core.autocrlf=true`），无 `.gitattributes`；新增文件为 LF ⇒ 行尾混杂。属外观问题，且引入归一化会造成全库 diff，**未动**，需要时单独一次 `git add --renormalize` 处理。

## M15a.4（2026-10-10）——运行时验证面建立 + 电平取样迁到消费侧

### 0. 先纠正一条文档假结论
`§M15a.2` 记的 "verify 断言 Y01–Y05 **PASS**" 是**无据结论**：全仓 `grep "Y0"` 零命中，`verify-library` 只有 L 系列、`verify-playback` 只有 C01/P01–P09。那批结论出自不落仓库的临时探针。本批把 Y/W 两条面真正落成可重跑脚本。

### 1. 新验证面（三个脚本 + 共享 QA 能力）
- `scripts/verify-visual.mjs`：**Y00–Y09**（歌词舞台换行 / 无词提示 / 光场在画 / 播放中电平 / 静音零门控 / 平滑收敛与抖动 / WebGL 上下文丢失恢复）。实测 **10/10 PASS**。
- `scripts/verify-windows.mjs`：**W00–W13**（Mini 开窗、跨 realm 状态同步、`integration://command` 事件桥换曲、▶/⏸、歌词窗锁定与出词）。实测 **11/13 PASS + 2 诚实 SKIP**（OS 层鼠标穿透只能目测；隐藏窗口保留 CDP target 属预期，写成 SKIP 而非恒真断言）。
- `scripts/lib/qa-app.mjs` 增 `connectCdpByUrl()`：Mini/歌词都是独立 webview，各有独立 target——**只有主窗可被 `connectCdp` 挑中，这正是事件桥断裂在单窗测试里完全看不见的原因**。
- `verify-playback.mjs` 的用法漂移记录在案：它**不自启应用**（头注释要求手动带 `--remote-debugging-port` 起），且首个 `fetch` 无 try/catch ⇒ 空跑时报 "FATAL fetch failed"，看起来像应用崩了。本批未改它（改法应统一走 `startApp`）。

### 2. 电平取样点迁移（真产品缺陷，Y06 抓到）
`LevelTap` 原挂在**生产侧** `flush_pending`：环形缓冲预满时那一侧根本没有新样本可推，读数长期为 0 ⇒ 画面几乎不跟音乐走。迁到**消费侧** `OutputStage::render`（每个回调块都在真出声）：
- 取样在"任何增益之前"：用户音量、响度归一、preamp、EQ 都不参与；零门控语义不变（增益 0 → 发布全零）。
- 非播放态在 `render` 的早退分支发布全零并清累加器（暂停/停止必须暗下来，不冻结末值）。
- 门禁：`callback_render_path_is_allocation_free` 仍通过（回调零分配不变量未被破坏）；`aura-core` 全绿。
- **诚实记录**：最初"2/24 → 6/24 非零"的改善数字**不能**当修复证据——两版测量都落在曲终之后（fixture 只有 3s，而我在播放后 ~3.7s 才采样）。计时修正后才是首份真证据：播放中 16/16 非零，rms ∈ [0.05875, 0.05995]。

### 3. 本批踩到并修掉的"为错误原因通过/失败"（五处，全部有实测支撑）
1. **3s fixture 反复坑人**：采样窗口跨过曲终 ⇒ 把曲终静音读成引擎缺陷（Y06）或把正确行为读成错（W10 曾期望"第三句"而实际位置已归 0 显示"第一句"，那是正确行为）。改法：起播即采样 + 需要确定值时用 `pause`+`seek` 冻结位置。
2. **暂停有 1s 传输淡变**：淡出期间引擎刻意保持 `Playing`，固定 900ms 等待把可用功能判成 FAIL（W06）。改为轮询到状态翻转（实测 ~1250ms 落地）。
3. **`PlayerSnapshot` 没有 `trackId`**：曲目身份是 `trackPath`；且两个 fixture 共用标题"Fixture曲目" ⇒ 只比 title 会把换曲判成没换。
4. **`.dl-pill` 在未锁定且未悬停时是 `display:none`（设计如此）**，第一版在错误状态下测出 0×0 还把结论写成通过。改为在**锁定态**断言可见+自身命中（实测 `flex 26×26 自身命中=true`）。
5. **verify-windows 只拷音频扩展名 ⇒ 没拷 `fixture-lrc.lrc`**，悬浮窗按设计回退显示曲名，而"文本非空"断言照样 PASS——典型的假绿。已补拷 `.lrc` 并把断言改成匹配具体行文本。
另：我曾假设"歌词窗不在 capabilities 的 `windows: ["main","mini"]` 里，IPC 会被拒"——**该假设被实测否证**（自定义命令不受该列表门控；`invoke=OK` 且能出词）。

### 4. 待查（本批未归因，如实登记）
- ~~`http_matrix::dns_rebind_to_private_ip_denied` 全量并行 3/3 失败~~ → **已查明并修复**（`fix(security) HTTP 客户端显式禁用环境代理`）：不是抖动，是环境里存在 `ALL_PROXY/HTTP_PROXY` 时私网复检整层被绕过（复检挂在自定义解析器上，走代理则目标主机名由代理解析，解析器一次都不被调用；那个 502 来自公网，本机无人监听 127.0.0.1:80）。修法为拒绝式默认 `no_proxy()`；需要经代理出网时要另做显式开关并重写威胁模型。
- 平滑抑制抖动只在带动态的信号上可证；当前 fixture 是稳态正弦（原始抖动 1.86e-3，平滑 1.41e-4，收敛性可证）。需要一个有强动态的 fixture 才能把这条做严。
- 未做：`docs/screenshots/10・12` 仍是 v1 3D 世界；`dreamscape_set_quality` 仍是校验后丢弃的桩；真实音乐调性仍出 null；CUE 整轨仍不进分析。

## M15b 分片一（2026-10-10）——ifuyo 远程曲库：清单拉取 + 列表页 + 严格策略播放

### 做了什么
- **服务器给出的 URL 不享受 D-070 的"输入即确认私网"**：新增 `AppState::http_strict`
  （deny_private）与 `build_strict_http()`（lib/smoke 共用，不留第三份策略字面量）；
  `play_input_impl` 改为接收要用的 http 客户端，`play_track`/`play_input` 仍走
  `state.http`（既有语义不变），只有 `ifuyo_*` 走严格客户端。清单地址是编译期常量，
  不接受 UI/参数注入；CSP 是 `connect-src ipc:`，所以清单由 Rust 取，JS 不 fetch。
- `src-tauri/src/ifuyo.rs`：`ifuyo_list` / `ifuyo_play`，纯函数 `build_list` 做
  版本门（非 v1 fail-closed）→ 逐条 URL 校验 → 去重 → 上限截断；被丢弃的条数与
  truncated 一律如实上报（静默少歌会让人以为服务器少给了）。标题缺失按 URL 末段
  percent-decode 回退，与 M3"无标签以路径回退"同口径，不伪造 artist/时长。
- `ui/src/ipc/ifuyo.ts` 校验面 + `IfuyoPage` + 导航项 + palette 命令 + zh/en 文案。
  无标签曲目 `durationSeconds: null` 显示 `--:--`（不是 0:00）——这是线上真实形态。
- `scripts/verify-remote.mjs`（R 系列）打**真实服务器**跑完整链路。实测 **8/8 PASS**：
  R01 清单 2 首；R02 列表页 2/2 行、无伪造 0:00；**R03 点真实远端曲目 → 流式播放
  pos 0.40→1.92s**；R04/R05/R05b/R05c 私网回环/局域网/云元数据端点/带凭据 URL 全部
  `SOURCE_BLOCKED`；R06 对照组 `play_input` 同一私网地址得 `SOURCE_UNAVAILABLE`
  （策略放行、只是没人监听）——证明两个入口的策略确实不同，而不是我嘴上说不同。

### 本批抓到并纠正的自己的错
1. 新 CSS 用了 5 个不存在的 token（`--space-3`/`--text-muted`/`--surface-hover`/
   `--brand-accent`/`--text-primary`）——正是上一批修掉的那类"未定义 var 让整条声明
   静默失效"。token 门禁直接打红，改用真实存在的变量后过。
2. 组件测试只 mock 了 2 个 store hook，漏掉 `useTranslation` 依赖的第三个
   （`useSettingsStore`）⇒ 整棵组件树抛错、5 条全红；改为不 mock store、在真实
   zustand 上断言。
3. `ifuyo.test.ts` 里 URL 判定我写成 `startsWith('https://') === true === false`，
   会把所有 https 判成不合格——自查时发现的逻辑错，改成显式 `isHttpUrl`。
4. R04–R06 首轮 4 条 FAIL 是**探针自身的语法错**：`String(e && e.message ?? e)`
   混用 `&&` 与 `??` 不加括号在 V8 里是 SyntaxError，导致全部拿不到值；而 R06 当时
   写作 `!== 'SOURCE_BLOCKED'`，于是**因这个错误而假通过**。修语法 + 让 R06 必须
   真的取到判定值才算 PASS。qa-app 因此补了 `awaitJs`（`evalJs` 不带 awaitPromise，
   取回的是 pending Promise 序列化成 {}，这个坑本会话已踩第二次）。
5. R03 首轮 FAIL 也不是产品问题：清单第一行是占位的 `test.mp3`（1MB 无标签假数据，
   本来就解不开）。改为挑真正有时长的那条。

### 已知限制（未做，不是遗漏）
- 远程播放**不落盘**：直接 HTTP 源仍是 `SessionOnly`，"播过就有本地缓存、离线可重播"
  需要缓存保留策略的用户可见开关 + DECISIONS（分片二）。
- 列表没有分页/搜索（当前清单规模无需）；封面未从远端取（清单 v1 无 artwork 字段）。
- 清单地址硬编码；换主机需要重跑生成器（url 前缀已烘进清单）。

## M15b.1（2026-10-10）——用户实机反馈修三处（远程标题 / 桌面歌词残留 / 拖不动+居中）

用户跑真实曲库后报的三条，全部先定位再修，并各配一条可重跑断言：

1. **远程播放显示 `%E7%8E%B0…`**：两个原因叠在一起——(a) `known_meta` 只喂 SMTC 元数据，
   快照标题走 `resolved.meta`，远程 URL 没有标签就是 null；(b) UI 的 `fileBaseName` 回退
   从不 percent-decode。修法：`play_input_impl` 的标题优先级改为
   `known_meta.title → resolver track.title → icy_name`（仍不猜），`ifuyo_play` 把清单的
   title/artist 传进去（并按 SMTC 上限做 char 边界截断，UI 传入长度不可信），
   `fileBaseName` 补 percent-decode + 去 query/fragment（畸形转义退回原串，不崩）。
   断言 R03b：`snapshot.title === 清单 title` → PASS（修前 FAIL 拿到 null）。
2. **"未在播放"永久刻在桌面上**：paint 的无词分支用了 `title || t('np.noTrack')`，停止播放
   后不清空。新规则：无曲目或 stopped → 两行清空；paused 保留当前行（暂停是为了看词）。
   断言 W12：stop 后 `now===''&&next===''` → PASS。
3. **拖不动 + 没居中**：`data-tauri-drag-region` 只标在容器上，而文本 `<p>` 盖满窗口——
   Tauri 只认 mousedown 目标元素自身的属性（TitleBar 早就踩过同一个坑并写了注释）。
   改为未锁定时容器与两行文本都带该属性，锁定后全部撤掉；文字居中（含 padding 对称化）。
   断言 W11：未锁定 `dl-now` 有 drag 属性 + `text-align: center` + 锁定后属性消失 → PASS。

### 这一轮我又写错的两处测试（都被自己的断言抓到）
- W10 先 `pause` 再等 1.3s 去读"第三句"：暂停含 1s 淡出，淡出期间引擎仍在消费缓冲、
  **位置会继续走**（实测停在 3.61s → 正确显示"第四句"）。写死期望值把正确行为判成 FAIL，
  改为按实际位置从 LRC 推导期望行。
- 同一批里"停止后清空"这条新规则也让旧的 W10 取词流程失效（fixture 只有 3s，早已进
  stopped）——两条规则叠加时，先改测试的取数方式再谈产品是否有 bug。

矩阵（与源码同代 release 产物）：**Y 10/10、W 13/15（2 诚实 SKIP）、R 9/9**；
`cargo fmt`/`clippy -D warnings`/workspace 66 套件 0 失败、UI 116/116。
