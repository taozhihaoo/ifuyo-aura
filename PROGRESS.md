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
