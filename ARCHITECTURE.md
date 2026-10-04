# ARCHITECTURE

> 状态：Phase 0 基线。crate 均为空骨架，本文件描述的是**目标边界**，M0 起逐阶段填充实现。
> Status: Phase 0 baseline. All crates are empty skeletons; this file describes target boundaries, filled in from M0 onward.

## 总体分层

```text
UI (React 18 + TypeScript + Vite + Zustand + Three.js)
 ↓
Tauri IPC（typed commands + events，allowlist + input validation）
 ↓
Application Layer（src-tauri：会话、命令路由、策略执行）
 ↓
Domain / Services
 ↓
Crates（纯 Rust 领域内核）
```

## Rust Crates

| Crate | 职责 | 关键约束 |
|---|---|---|
| `aura-core` | 音频引擎：decode → resample → ring buffer → output；时钟、seek generation、gain | **不得依赖 Tauri / UI / filesystem / network**；必须可通过 `NullOutput` 独立测试 |
| `aura-sources` | Source trait 与来源实现（local / direct URL / radio / ICY / Radio Browser / 后续开放来源） | 统一 `search / resolve`；每个 Source 返回 `Allowed / SessionOnly / Forbidden` 缓存策略 |
| `aura-cache` | 分块网络缓存（256 KiB chunks、Range、LRU、pin、故障注入） | 只允许持久化 `Allowed` 来源；`Forbidden` 来源零写盘 |
| `aura-library` | 音乐库：扫描、标签（lofty）、SQLite（WAL + FTS5）、watcher、smart search | 中文 CJK 二元组检索 |
| `aura-analysis` | 后台低优先级分析：BPM、key、LUFS、true peak、energy、MFCC、embedding | 可取消、可恢复、可缓存、content-key 绑定、确定性 |
| `aura-synth` | Dreamscape 确定性生成音乐引擎 | 同 seed 同版本逐位一致 |
| `aura-testkit` | 测试基础设施：NullOutput、FaultFS、合成音频、固定随机种子、确定性时钟 | 仅用于测试，不进入生产依赖 |

## 依赖方向（强制）

```text
src-tauri ──► aura-core / aura-sources / aura-cache / aura-library / aura-analysis / aura-synth
aura-sources ──► aura-core
aura-cache ──► aura-core
aura-library ──► aura-core
aura-analysis ──► aura-core
aura-synth ──► aura-core
aura-testkit ──► （被测试引用，不反向依赖任何 crate 的内部）
```

- **`aura-core` 不依赖 Tauri、UI、filesystem、network。** 音频内核可在无窗口、无网络环境下独立测试。
- crates 之间不得相互污染职责；不允许出现环。
- `windows` crate 及一切平台 API 只允许出现在 `src-tauri`（M10 起），不得进入任何 aura crate 内核。
- UI 不得绕过 IPC 直接触达领域逻辑；高频数据（如可视化帧）走 Tauri channel，频率受控。

## UI 结构

```text
ui/
  src/
    design/tokens.ts        # 设计 token 唯一事实来源（品牌色等，镜像 brand.json）
    generated/              # 由 scripts/generate-tokens.mjs 生成，禁止手改
    ...
```

- CSS variables 只能从 tokens 生成（`scripts/generate-tokens.mjs`，读取 `brand.json` 并与 `tokens.ts` 校验一致）。
- 业务代码禁止裸色值；lint 规则在本阶段仅预留结构，M6 落地。


## Audio Core Architecture（M1）

```text
Decoder（symphonia FLAC / synthetic）
  ↓ 交错 f32（源格式）
Channel Map（mono→stereo / passthrough）
  ↓
Resampler（rubato sinc 256，采样率一致时旁路）
  ↓
SPSC Ring Buffer（rtrb，帧语义，generation-aware discard）
  ↓ consumer
OutputStage（回调路径唯一逻辑：弹出 → underrun 补静音 → PCM 净化 → 10ms 增益平滑 → 计数）
  ↓
NullOutput（CI 确定性渲染/捕获）或 CPAL 设备流（f32/i16）
```


### Transition Engine（M2）

```text
Current Decoder ──► Current Pipeline ──► Audio Ring Buffer ──► OutputStage ──► Output
Next Decoder    ──► (preload: 同管线顺序接管)
```

- **Gapless Model**：自然曲目结束（decode_next → 0）且有预装载 next → 同管线接管（不 bump generation、位置连续）。TrackFrameRange 记账每轨 source/valid/rendered 帧。
- **Crossfade Model**：cf ∈ [0,12s]；cf=0 → gapless 优先；进入条件 = remaining ≤ cf 且格式一致；等功率 cos/sin 曲线按帧推进；degenerate（cf ≥ remaining）降级 overlap=remaining。
- **Loudness Pipeline**：`TrackLoudness`（M3/M8 供给）→ gain = min(−16−LUFS, peak-safe ceiling) → 与 preamp 合并为 track_gain（静态）→ 与 user volume（动态平滑）相乘。
- **Limiter**：tanh 软限幅，−1dBFS 直通阈值，渐近 1.0；DSP 顺序末级（§44）。
- **EQ Architecture**：RBJ biquad ×10 频段，Direct Form I 级联；EqBus 原子字快照（无锁），回调本地 Copy 副本。
- **Output Device Lifecycle / Hotplug Recovery**：Connected/Removed 两态；断开 → 安全暂停（位置/曲目保留）；恢复 → 格式重建（重采样器/渲染缓冲/EQ 级联），不自动播放；流错误仅计数；检测归外部适配器（M10）。

### Thread Model

```text
Control Thread   ──post──►  Command Queue（rtrb SPSC，64 容量）
Engine Thread    ──owns──►  Engine（decoder / resampler / producer / 状态机）
Output Callback  ──owns──►  ConsumerHandle + OutputStage（或 NullOutput 确定性渲染）
```

M1 的 Engine 以同步确定性方式驱动（`render_for_frames` 显式推进）；真实线程接线（解码线程 + CPAL 回调）在后续里程碑引入，共享面仅原子量（SharedOutputState：position / underruns / target_gain）。

### Generation Model

load / seek 均 bump generation（u64，起 1）。producer bump 时记录缓冲内旧代帧数；consumer 检测代切换后精确丢弃这些帧（不波及新代数据）。旧代样本永不到达输出；并发残余最坏情况为单渲染块静音（D-019）。

### Clock Model

位置权威 = `consumed_frames / sample_rate`。暂停期间位置冻结；Stop 归零（D-023）；seek 直接设目标帧。测试经显式推进/确定性渲染，禁 wall-clock。

### Callback Rules（铁律）

回调路径（OutputStage::render + ring pop + capture）零分配、零锁、零 logging、零 syscall、零 panic——由计数分配器测试（tests/zero_alloc.rs）证明。错误回调仅递增原子计数。

### Buffering Model

ring buffer 容量按帧对齐（Engine 默认 buffer/8 = 渲染块）。overrun：producer 部分写入拒绝 + fill 背压（不覆盖不丢数据，D-022）。underrun：输出静音 + 计数 + 可恢复。

### Error Model

`AudioError` 枚举（InvalidFormat / DecodeError / UnsupportedFormat / ResampleError / OutputError / SeekError / InvalidCommand / InvalidParameter），Display 可诊断、无敏感数据、无 panic 路径。

### Music Library Architecture（M3）

`aura-library` 职责链（§5）：`Filesystem → Scanner → Metadata(lofty) → Normalization → Identity → SQLite → Search/Query`。不负责解码/播放/输出/网络/UI（§61：与 audio core 唯一交汇是上层以 Track 路径喂给 engine `load_track`）。

- **身份**（D-037..D-039）：`identity_key = v1:size + head/tail 1MiB SipHash-2-4-128`；路径行（track_paths）与身份行（tracks）分离，改名/移动合并、标签改写保留（audio_sig 分辨）、音频替换新身份；统计绑定身份，删除只停用不销毁（§47）。
- **扫描**：迭代 walk（无 reparse 跟随，D-041）、扩展名过滤 + 内容探针、单文件事务（崩溃只丢当前文件）、size+mtime 增量跳过（§17）、CancelToken 在条目边界生效（§67）、进度回调进 UI 不进日志（§68/§80）。
- **数据库**：SQLite bundled WAL，`user_version` 迁移（v1），所有写路径单事务（§49）；`tracks_fts`（FTS5）行在 track 事务内同步。
- **搜索**（D-042/D-046）：预分词（小写英文词 + CJK 单字/滑窗二元组）+ unicode61；查询侧引号失效化 + 前缀星；bm25 列权重 + id 决胜。
- **封面**：内嵌提取（front cover 优先、字节签名 MIME、8MB 限）→ artworks 内容哈希去重 → 64/256 box-filter 缩略图（原子写、解码限内存、可重建）。
- **watcher**（D-043）：notify → EventCoalescer（500ms 窗口、终态合并、注入时钟）→ rescan 候选；watcher 路径零 DB 写。
- **时间**：核心不读 wall-clock；LibraryService 接受注入时钟（`set_clock`），扫描/统计时间戳可确定性测试。

### Source Architecture（M4）

`aura-sources` 职责链（§7）：`Input → Detection → Resolution → Resolved(+License/Attribution/CachePolicy)`。不负责 audio output / persistent cache（M5）/ UI / SQLite 直写（§93）。

- **Source trait 面**（§8）：`search(query)` / `resolve(input)`；`Resolved { streams, meta(Track/Stream/Provider 三分), license(Unknown/Known 不猜测), cache_policy, attribution }`。`SourceKind` 强类型（Local/DirectHttp/InternetRadio/RadioBrowser/Podcast/Hls）。
- **Resolver 顺序**（§13）：本地存在文件 → URL 校验（scheme 白名单/凭据拒绝/封锁平台/私网，全部先于网络）→ 网络探测（icy 头→电台形态；Accept-Ranges+Content-Length→Seekable）→ 未知输入明确报错。
- **网络栈**（D-049/052/053）：reqwest blocking + rustls；重定向 ≤5 逐跳重校验；自定义 DNS 解析器逐 IP 私网复检（防 rebinding）；响应体上限；`NetworkAudioStream` = 有界管道 + feeder 线程 + 所有权交接取消协议 + 消费侧空闲检测 + Range 重请求 seek。
- **ICY 管线**：`Icy-MetaData: 1` → `IcyReader`（任意 read 边界安全）剥离 metadata → 音频逐字节透传 → `StreamMetadataEvent` mpsc 通道。
- **Radio Browser 流程**：`/json/servers` 发现 → 选择器（生产随机/测试注入）→ fallback ≤3 → search（name/country/language/codec）→ `url_resolved` 优先（记录字段）→ click 默认 OFF。
- **Source → Audio Core 边界**（§55/§58）：`AudioByteStream` trait（core 定义，sources 实现）→ `MediaSource` 适配 → `StreamingDecoder`（symphonia probe）→ M1 Decoder → Resampler → Buffer。core 不认识 reqwest，sources 不碰 CPAL。
- **取消与错误传播**（§22/§59）：`SourceError` 全链路可诊断、不 panic、不泄漏凭据；`Cancelled` 语义端到端。
### Cache Architecture（M5）

`aura-cache` 职责链（§2）：`Source → Network Stream → Cache Layer → Decoder → Audio Core`。cache 不知道 WASAPI/CPAL/EQ/UI；与 library DB 完全隔离（独立文件 + 结构性无路径知识，§28）。

- **CachePolicy 硬门禁**（§3–§7）：`put_chunk` 是唯一持久写入口 —— Forbidden 拒绝（零写盘）、SessionOnly 有界内存（32MiB）、Allowed 才落盘；live 流双保险拒绝（§75）。
- **Chunk Storage**（D-056/057）：256KiB 固定分块；`chunks/<ck[0..2]>/<ck>/<idx>.bin` 固定深度映射；tmp 写 → 校验 → 原子 rename → SHA-256 → DB 事务；FaultFS 注入 write/truncate/rename 故障（§65）。
- **Cache Index**：独立 SQLite（WAL、`user_version` 迁移、5s busy timeout）；entries/chunks/pins 三表；access-time 内存聚合批量落库（§62）。
- **Range Reader**（§29–§37）：`CachedRangeReader`（Read+Seek+AudioByteStream）cache-first；miss → 206 按块拉取（Content-Range 逐项校验）；200 → non-range + bounded sequential fill；seek 限已填充区间；离线 miss → Network 错误不伪装 EOS（§80）。
- **Revalidation**（§40–§43/§84）：open 时 If-None-Match；Fresh/Validated/Stale/Unknown 显式状态；内容变更作废全 key chunks。
- **Retry/Dedup**（§45–§48/§53/§95/§136）：3 次指数退避（仅可恢复错误）；in-flight 注册表按 FetchKey 去重（leader 在临界区持久化，waiter 读缓存）。
- **Prefetch**（§49–§51）：读取触发 + 显式两种；epoch 计数器 seek 即取消。
- **LRU/Pin**（§55–§61）：容量只计有效 chunk bytes（默认 2GiB 可配）；LRU 淘汰跳过 pinned 与 in-flight lease；全 pinned → `CacheFullPinned`。
- **Crash Recovery**（§27/§66–§69）：DB 损坏 → 备份 + 重建；orphan/missing/corrupt → `verify_cache` 报告并修复；FaultFS 全场景测试。
### Desktop UI Architecture（M6 上篇）

分层（§7）：`UI Components → Zustand feature stores → Typed IPC → Tauri commands → aura-shell app layer → M1–M5`。React 不触碰 audio callback/ring/decoder/SQLite/reqwest（§7/§128）。

- **状态分离**（§8/§9）：domain（player snapshot/library query）来自 Rust 快照；UI（view/panels/palette）来自分域 Zustand stores + selectors。
- **高频边界**（§10/§14）：快照 250ms 轮询 + 位置走共享原子 `player_position`，rAF 本地插值；Aurora 以 ref telemetry 驱动，不经 React 渲染。
- **Shell**：frameless + 自绘 titlebar；window controls 经 capabilities 最小授权（core:window-* + dialog:default，无 shell §23）；CSP 全自托管（§24/§25 零远程资源）。
- **命令架构**（§99）：`Command { id, title, keywords, shortcut, execute }` 注册表 + 子序列模糊匹配；Ctrl+K/Space/Ctrl+F/Esc 全局快捷键（输入态不拦截 Space §103）。
- **Aurora**（§56–§75）：单一 WebGL canvas；GLSL fbm 流场（品牌 palette 派生）；playing 30fps / idle 8fps / reduced-motion 2fps 节流；稳定 per-track seed（非 M7 seed）；WebGL 失败 → CSS 渐变 fallback；分辨率 cap + resize debounce（§134–136）。
- **多窗口**（§157）：mini player 为同 dist `?view=mini` WebviewWindow，共享单一应用层/音频引擎；关闭 = 隐藏不停止（§113）。
### Desktop UI Architecture（M6 下篇）

- **来源入口统一**（§275/§276/§194）：Open File / Open URL 一律经 `play_input` IPC → M4 `resolve_input`（scheme 白名单→凭据拒绝→封锁表→私网策略→探测）→ 按 kind 装载：Local→`open_decoder`；InternetRadio→`radio::open_resolved().into_parts()`（NetworkAudioStream 即 AudioByteStream，live/non-seekable）；DirectHttp→`NetworkAudioStream`（容量按形态/码率 D-051）→StreamingDecoder。前端零 fetch（§195 无浏览器音频）。
- **错误打开即回传**（D-071）：decoder 在 IPC 命令线程打开，`Control::LoadDecoder` 只携带就绪 decoder；驱动线程无吞错路径。
- **错误映射链**（§189/D-073）：`AppError.code` → `ipc/errors.ts` 静态表 → i18n（zh/en）；`FeatureBoundary` 页面级隔离（§227–229），单页挂掉不白屏，重试仅重挂载本 feature。
- **queue 单一权威**（§197/D-072）：模块单例游标模型（`playWithQueueContext`/`playNext`/`playPrevious`/`removeAt`），边界返回 false（§257）。
- **数值安全**（§253–§256）：`ui/format.ts` 统一 `formatDuration`（非法→null，调用方展示 Live/Unknown）+ `clampNumber`/`safeProgress`。
- **测试模式**（§179/§180/D-074）：`?visualTest` → Aurora 固定 seed/time/telemetry 单帧。
- **测试栈**（§181–§183）：vitest + jsdom + @testing-library（渲染/a11y/键盘矩阵/挂载卫生 42 项）；桌面冒烟 `--bin smoke`（真实 Wry 装配 + 启动计时）。
- **键盘图**（D-078）：Ctrl+K/Esc/Space/Ctrl+F/Alt+1/2/3/Ctrl+Q/Ctrl+M；Esc 全局可退出任一浮层（§183 无死路）。

### Dreamscape 世界生成核心（M7 上篇）

`crates/aura-synth`（§7 职责边界）：`seed → genome → environment → procedural generation → scene manifest`；无 Three.js/WebGL/React/Tauri/audio/SQLite/HTTP。

```text
src/
  seed.rs       # 60bit 规范 seed + DS 人类格式 + world_id（D-079）
  rng.rs        # SplitMix64 命名流 + 桶派生（D-080）
  genome.rs     # 有界参数 + 小步 mutation 记录（D-081）
  env.rs        # 绝对 tick → 相位/天气/光态 + 约束 palette（D-082）
  biome.rs      # 五原型六要素 + per-seed 变异（§43–46）
  world.rs      # 地形/放置/spatial hash/landmark/预算（D-083）
  events.rs     # 稀有事件 + 日志重放过期（D-084）
  manifest.rs   # SceneManifest + scene_hash（D-085）
  generator.rs  # GeneratorV1 版本化契约（§108/§109）
  region.rs     # 区域流式：worker/epoch 取消/LRU（D-086）
  snapshot.rs   # Snapshot/Session/缺省世界（D-087）
```

- 确定性合同：同 seed+generation_version+environment → RNG 序列/manifest/scene_hash 逐位一致；世界逻辑只接受逻辑 tick。
- 分层：synth 只做「世界是什么」；shell `dreamscape.rs` 装配会话与 `dreamscape.json` 逻辑态持久化（§127）+ 9 个 typed IPC 命令；渲染（下篇 Three.js）消费 manifest/DTO。
- 版本轴：seed_version / generation_version / genome·env·world·manifest·snapshot 各自版本常量；升级 = 显式 DECISIONS。

### Dreamscape 渲染与集成（M7 下篇）

```text
ui/src/visual/dreamscape/
  engine.ts             # 纯逻辑：质量档/AutoQuality/节流/GenerationGate/Lifecycle
  scene.ts              # manifest→GL：程序化地形/InstancedMesh/landmark/粒子 + Disposables
  DreamscapeRenderer.ts # 生命周期编排（create/loadWorld/start/pause/resume/resize/dispose）
ui/src/features/dreamscape/DreamscapeView.tsx  # React 集成壳
```

- **单渲染器规则**（§130/§131/D-091）：dreamscape 视图卸载 AuroraCanvas；Mini 永不渲染 Dreamscape。
- **状态分层**（§269–§273）：Zustand 只存 dreamscapeOpen/status；manifest 走 renderer ref；相机渲染器本地、持久化仅 compact CameraState。
- **频率边界**（§263/§264/§265–§267）：IPC 低频推进（2s=30 逻辑分钟）+ 按需拉取；插值/平滑全在渲染器本地。
- **失败路径**（§274–§277）：FeatureBoundary + CSS fallback 场景 + 用户世界状态不污染；GL 不可用 → create 返回 false → 立即 fallback（不悬挂 loading §278）。
- **确定性**（§171/§172）：逻辑 manifest/scene_hash 为准；GPU 视觉允许像素级差异（地形由 terrain patch 参数在渲染端确定性重建）。
- **键盘**（§198/§285）：Esc 分层退出（palette→dreamscape→抽层）；palette 四命令（open/regenerate/copy seed/save snapshot §197）。

### 分析基建（M8 上篇）

`crates/aura-analysis`（§11 职责边界）：domain / key / embedding / decode / store。

```text
src/
  domain.rs      # AnalysisStatus/FeatureAvailability/TrackAnalysis 聚合/各特征结构/AnalysisError（§13–§33）
  key.rs         # analysis_key = hash(content_identity+versions)（§23，D-097）
  embedding.rs   # EmbeddingProvider 合同 + SpectralStatsProvider（D-095，§41–§49）
  decode.rs      # M1 StreamingDecoder 适配 → 单声道 f32（§12/§34）
  local_stream.rs# 本地文件 analysis-side adapter（§12）
  store.rs       # AnalysisStore：M3 库单事务持久化/替换检测/版本 Stale（D-096）
```

- **身份**（§15）：TrackId + content_identity（M3 identity_key，D-038 改名/移动/标签改写不变）+ analysis_version；§20 替换检测 → Stale；§21 metadata-only 不重算；§22 移动保持。
- **持久化**（§24–§31）：M3 SQLite 单表（迁移 V2），save 单事务 all-or-nothing，feature availability 列存，BLOB 完整性读回校验。
- **M2 契约**（§5）：LoudnessAnalysis → TrackLoudness 的 DTO 由应用层转换（M2 不依赖 aura-analysis）；M7/M9 消费 contract 见 domain/embedding（§205/§49）。
- **禁止**（§3/§4/§38）：无网络、无 fake analysis、无 Map/推荐；中篇承载 BPM/Key/LUFS/Spectral/MFCC 算法，下篇承载 worker/queue/集成门禁。

### 特征分析引擎（M8 中篇）

aura-analysis 增：fft / params / loudness / tempo / key_detect / spectral / mfcc / pipeline / scheduler / similarity / summary。

- **规范预处理**（§86–§89，D-098）：mono fold-down (L+R)/2 → 线性重采样 22050 → 分析缓冲（窗口 ≤120s 有界；loudness 全轨流式 O(时长/10)）。
- **算法**：LUFS（BS.1770-4 K-weighting+双门控+LRA+true peak 4× 过采样，D-099）；Tempo（spectral flux→分数周期谐波 comb→[70,180) 折叠，D-100）；Key（chroma 4096-FFT→KK 剖面 24 假设 Pearson→熵门，D-101）；Spectral（RMS/Peak/Centroid/Rolloff/Brightness/Energy dBFS 映射，D-102）；MFCC（pre-emph/mel26/DCT-II/13 系数/ln floor，D-103）。
- **管线**（§92–§94）：单遍共享解码（spectral+loudness 全轨流式；tempo/key/mfcc/embedding 用 120s 窗口）→ 特征隔离（§134–136 各自 Failed/Unavailable）→ Completed 校验 → 仓库。取消检查点在 chunk/特征边界（§102/§103）；clock 注入（§107/§108）。
- **调度/合同**（§95–§129）：AnalysisScheduler（优先级/取消/有界重试）+ AnalysisRepository trait（InMemory 测试 / SQLite 桥接）+ TrackAnalysisSummary（§117 单位显式 §118、None=未分析 §119/§120）+ cosine 相似度原语（§125–§129 错误分支：维度/NaN/零向量/版本不兼容）。

## 构建与工程

- Cargo workspace：7 个 aura crates + `src-tauri`（M0-3 起为正式 member，tauri 2.12）。
- `config/unsupported_platforms.json`：不支持平台识别表，来源系统必须据此拒绝请求。
- CI（`.github/workflows/ci.yml`）预留全部门禁命令，见 PROGRESS.md 的当前可运行状态。
- 工具链：`rust-toolchain.toml` 锁定 stable（MSVC 目标），MSRV 1.87（D-011）。
