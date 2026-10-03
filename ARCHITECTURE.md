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

## 构建与工程

- Cargo workspace：7 个 aura crates + `src-tauri`（M0-3 起为正式 member，tauri 2.12）。
- `config/unsupported_platforms.json`：不支持平台识别表，来源系统必须据此拒绝请求。
- CI（`.github/workflows/ci.yml`）预留全部门禁命令，见 PROGRESS.md 的当前可运行状态。
- 工具链：`rust-toolchain.toml` 锁定 stable（MSVC 目标），MSRV 1.87（D-011）。
