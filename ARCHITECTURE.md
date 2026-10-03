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
## 构建与工程

- Cargo workspace：7 个 aura crates + `src-tauri`（M0-3 起为正式 member，tauri 2.12）。
- `config/unsupported_platforms.json`：不支持平台识别表，来源系统必须据此拒绝请求。
- CI（`.github/workflows/ci.yml`）预留全部门禁命令，见 PROGRESS.md 的当前可运行状态。
- 工具链：`rust-toolchain.toml` 锁定 stable（MSVC 目标），MSRV 1.87（D-011）。
