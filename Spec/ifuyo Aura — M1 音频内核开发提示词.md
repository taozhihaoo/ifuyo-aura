# ifuyo Aura
# M1 — Audio Core

你现在进入 ifuyo Aura 项目的 **M1：音频内核**。

项目正式名称：

`ifuyo Aura`

当前阶段目标：

> **在 M0 已完成并通过审计的基础上，实现一个真正可测试、可确定性验证、可长期扩展的 Rust 音频核心。**

本阶段只实现：

- 音频解码；
- 重采样；
- lock-free SPSC ring buffer；
- NullOutput；
- 基础播放时钟；
- seek；
- generation；
- 音量；
- 基础播放控制；

并建立完整测试体系。

**禁止实现 M2 及之后的功能。**

---

# 0. 当前阶段边界

## 本阶段必须完成

[A]

1. `aura-core` crate 建立稳定结构。
2. decoder abstraction。
3. 至少一个真实音频格式的实际解码路径。
4. decoded PCM → resampler → ring buffer → output 的完整链路。
5. lock-free SPSC ring buffer。
6. NullOutput。
7. 可控虚拟时钟。
8. playback position。
9. pause / resume / stop。
10. seek。
11. generation-based stale sample rejection。
12. volume。
13. 10ms gain smoothing。
14. underrun detection。
15. 基础错误处理。
16. 完整确定性单元测试。
17. 关键行为集成测试。
18. `cargo fmt`、`clippy`、`test`、`deny` 等当前可执行质量门禁通过。

---

# 1. 本阶段明确禁止

不要实现：

- gapless；
- crossfade；
- loudness normalization；
- LUFS；
- true peak；
- EQ；
- A-B loop；
- WSOLA；
- auto mix；
- spectrum analyzer；
- BPM；
- beat grid；
- key detection；
- music map；
- Dreamscape；
- WebGL；
- Tauri UI；
- Windows SMTC；
- tray；
- online source；
- Radio Browser；
- HLS；
- cache；
- SQLite；
- lyrics；
- playlist UI。

这些属于后续 Milestone。

本阶段如果发现某项后续功能需要基础接口：

只建立最小、稳定、无业务污染的 abstraction。

不得偷偷实现后续能力。

---

# 2. 开始前：完整 Baseline Audit

在任何修改前，先检查当前仓库真实状态。

至少检查：

```text
git status
git branch
git log --oneline -10
cargo metadata
Cargo.toml
crates/
src-tauri/
ui/
```

确认：

- M0 是否已经完成；
- `aura-core` 是否已存在；
- workspace 是否正常；
- 当前 Rust toolchain；
- 当前 MSRV；
- 当前依赖；
- 当前 CI；
- 当前 `cargo-deny`；
- 当前测试数量。

如果 M0 尚未完成：

不要重做 M0。

先指出阻塞项。

如果 M0 已完成：

直接进入 M1。

输出：

# M1 Baseline Audit

A. Git  
B. Workspace  
C. Rust Toolchain  
D. aura-core  
E. Dependencies  
F. Existing Tests  
G. CI  
H. Risks

---

# 3. 依赖策略

优先使用 M0 已确定依赖。

音频核心相关优先考虑：

```text
symphonia
cpal
rubato
rtrb 或 ringbuf
```

具体版本必须：

- 使用当前 workspace 已锁定版本；
- 或在确有必要时选择当前兼容稳定版本；
- 不因为“最新版”而无意义升级整个 workspace。

每新增依赖必须检查：

1. license；
2. MSRV；
3. Windows compatibility；
4. CI compatibility；
5. maintenance；
6. 是否能被 `cargo-deny` 接受。

如果 API 与章程中的预期不同：

不要强行模仿旧 API。

根据当前实际版本适配，并把真实决策写入：

`DECISIONS.md`

---

# 4. 设计目标

音频核心必须满足：

> **业务无关、设备无关、UI 无关、网络无关、可确定性测试。**

`aura-core`：

不得依赖：

- Tauri；
- React；
- filesystem；
- HTTP；
- UI；
- Windows-specific application layer。

允许：

- Windows audio backend 的独立 adapter；
- 但核心逻辑不能直接绑定 Windows API。

---

# 5. 推荐核心分层

可以采用以下结构；如有更好的等价设计，可以自行调整，但必须记录理由：

```text
aura-core/
  src/
    lib.rs
    error.rs
    types.rs

    audio/
      mod.rs
      format.rs
      buffer.rs
      clock.rs
      gain.rs
      generation.rs
      position.rs

    decoder/
      mod.rs
      pcm.rs

    resampler/
      mod.rs

    output/
      mod.rs
      null.rs
      device.rs

    engine/
      mod.rs
      player.rs
      pipeline.rs
      command.rs
      state.rs

  tests/
```

注意：

> 不要为了遵守这个目录而制造空模块。

结构应该服务于真实职责。

---

# 6. 核心数据模型

建立稳定的领域类型。

至少考虑：

```rust
Sample
SampleRate
Channels
FrameCount
AudioFormat
TrackPosition
PlaybackState
Volume
Generation
AudioCommand
```

推荐：

- 时间内部优先使用 frame；
- sample position 不使用浮点秒作为权威状态；
- seconds 只作为 UI / API presentation。

例如：

```text
position_frames
sample_rate
```

由此派生：

```text
position_seconds
```

---

# 7. PCM 表示

统一内部 PCM 表示。

原则：

- 明确 sample type；
- 明确 interleaved / planar；
- 明确 channel count；
- 明确 sample rate。

内部格式必须在整个 pipeline 中一致。

推荐：

`f32`

范围：

理论上音频 pipeline 可允许超出 [-1, 1]，但输出边界必须明确处理。

必须禁止：

NaN。

必须测试：

- NaN 输入；
- Infinity 输入；
- 极大值；
- 负极大值。

系统不得因为异常 PCM 导致 panic。

---

# 8. Decoder abstraction

建立：

```rust
trait Decoder
```

职责只负责：

> compressed audio → decoded PCM frames

不得负责：

- output device；
- UI；
- playback clock；
- seek policy；
- volume；
- visualizer；
- caching。

至少需要：

```text
format()
sample_rate()
channels()
duration()
decode_next(...)
seek(...)
```

具体接口可以根据 symphonia 实际模型调整。

---

# 9. 第一阶段真实格式

M1 不需要一次实现所有 codec。

至少实现：

> 一个真实、本地、确定的压缩音频解码格式。

优先：

`FLAC`

或项目当前环境最稳定的合法格式。

同时必须提供：

> synthetic PCM source

用于核心测试。

这样测试不依赖真实音乐文件。

---

# 10. Test Audio Fixtures

`aura-testkit` 或 `aura-core` 测试中提供确定性合成音频。

至少：

## Fixture A

1 秒：

440Hz sine wave。

参数：

```text
sample_rate = 48000
channels = 2
duration = 1s
```

## Fixture B

固定长度：

不同振幅的 sine wave。

## Fixture C

impulse。

## Fixture D

带编号 ramp：

用于验证：

seek 后旧 generation 数据是否泄漏。

例如：

```text
sample[n] = n
```

这样可以精确判断：

> 输出的数据到底来自哪个 generation。

---

# 11. SPSC Ring Buffer

使用：

`rtrb` 或 `ringbuf`

或 M0 已批准的等价实现。

要求：

- single producer；
- single consumer；
- lock-free；
- bounded；
- preallocated。

禁止：

生产路径动态扩容。

禁止：

audio callback 内 allocation。

---

# 12. Ring Buffer API

围绕项目需求建立轻量 wrapper。

例如：

```text
AudioRingBuffer
ProducerHandle
ConsumerHandle
```

要求：

- capacity 可测试；
- push；
- pop；
- available frames；
- free frames；
- clear；
- generation-aware discard。

不要让业务层直接依赖第三方 ring buffer 类型。

---

# 13. Ring Buffer 测试

必须测试：

1. 空 buffer。
2. 满 buffer。
3. 单帧写入。
4. 大块写入。
5. 跨边界写入。
6. 跨边界读取。
7. producer / consumer 数量违约。
8. 长时间循环。
9. 高频 push/pop。
10. underrun。
11. overrun。
12. clear。
13. 多 generation。

如果使用线程测试：

必须保证测试确定性。

不得依赖：

“sleep 100ms 后应该完成”

之类脆弱测试。

---

# 14. Audio Clock

建立独立 clock abstraction。

必须支持：

```text
VirtualClock
```

测试环境使用 virtual clock。

真实输出设备：

后续由 hardware timing 驱动。

不要让核心播放位置依赖：

```text
SystemTime::now()
Instant::now()
```

作为唯一权威。

---

# 15. 权威播放位置

播放位置：

```text
consumed_frames / sample_rate
```

或者等价的 frame-based representation。

必须保证：

暂停期间：

position 不继续增长。

停止后：

position 按定义归零或保持，必须明确记录。

seek：

直接更新目标 frame。

---

# 16. Playback State

至少：

```text
Stopped
Paused
Playing
```

必要时：

```text
Buffering
Error
```

但是：

不要为了未来功能提前创建复杂状态机。

必须定义合法转换：

```text
Stopped → Playing
Playing → Paused
Paused → Playing
Playing → Stopped
Paused → Stopped
```

无效操作：

必须安全处理。

不得 panic。

---

# 17. Audio Command

建立控制命令：

```text
Play
Pause
Stop
Seek
SetVolume
```

未来允许扩展：

```text
LoadTrack
```

但 M1 不实现 queue / playlist。

---

# 18. Command 通道

播放控制不能通过：

> 在 output callback 中直接执行控制逻辑。

推荐：

```text
Control Thread
      ↓
Command Queue
      ↓
Audio Engine
      ↓
Audio Ring Buffer
      ↓
Output
```

Audio callback 只负责：

> 拿数据 → gain → 输出 → meter / counter。

本阶段 meter 只保留最小内部计数结构，不实现 UI 数据通道。

---

# 19. Gain / Volume

实现基础音量：

范围：

```text
0.0 ~ 1.0
```

允许内部扩展更高精度。

必须：

- clamp；
- reject NaN；
- reject Infinity；
- 不允许负 volume。

---

# 20. 10ms Gain Smoothing

不能：

```text
volume = new_volume
```

直接瞬间切换。

根据 sample rate：

计算约 10ms 的 smoothing window。

例如：

48kHz：

约 480 frames。

实现：

```text
current_gain
target_gain
```

每 frame / sample 平滑逼近。

要求：

- 不爆音；
- 不 overshoot；
- target 改变可重复；
- 零 allocation。

---

# 21. Generation System

这是 M1 核心要求。

每次：

- load track；
- seek；

必须产生新 generation。

例如：

```text
Generation(1)
Generation(2)
Generation(3)
```

旧 generation 音频：

即使已经进入某个异步路径：

也不能继续输出。

---

# 22. Seek 逻辑

执行 seek：

```text
1. increment generation
2. clear / invalidate stale decoded state
3. seek decoder
4. refill buffer for new generation
5. resume output from target frame
```

具体顺序可以根据真实 pipeline 调整。

关键要求：

> **旧 generation 永远不能到达 output。**

---

# 23. Seek Determinism Test

制作：

两段明显不同的音频。

例如：

Generation 1：

440Hz。

Generation 2：

880Hz。

执行：

```text
play
seek
render
```

断言：

seek 后输出中：

不存在旧 generation 数据。

更进一步：

通过固定 ramp / tagged fixture：

直接验证 sample provenance。

不能只测“最终频率似乎对”。

---

# 24. NullOutput

必须实现：

```text
NullOutput
```

它不访问真实声卡。

必须支持：

- configurable sample rate；
- configurable channels；
- virtual clock；
- callback invocation；
- consumed frame counting；
- captured PCM；
- underrun counting。

---

# 25. NullOutput 的意义

NullOutput 是整个项目后续测试基础。

它必须能够：

```text
load synthetic audio
→ decode
→ resample
→ buffer
→ output
→ capture
```

全程在 CI 运行。

无需：

- Windows sound card；
- speaker；
- user interaction。

---

# 26. Deterministic Render

提供：

```text
render_for_frames(n)
```

或等价接口。

同一个：

- 输入；
- volume；
- sample rate；
- commands；
- virtual clock；

必须产生：

> 完全相同的 PCM。

这将成为后续：

- gapless；
- crossfade；
- Dreamscape；
- auto mix

测试的基础。

---

# 27. Underrun

如果 ring buffer 无数据：

不得：

- panic；
- block；
- allocate；
- logging；
- 无限等待。

必须：

1. 输出静音；
2. increment underrun counter；
3. 保持 playback pipeline 可恢复。

---

# 28. Overrun

producer 写满：

不得静默覆盖未知数据。

必须有明确策略：

- reject；
- backpressure outside callback；
- drop policy；

但不能破坏未消费音频。

记录决定。

---

# 29. Output Abstraction

建立：

```rust
trait AudioOutput
```

但必须保持最小。

至少表达：

- format；
- start；
- stop；
- pause；
- callback；
- consumed frames；
- error。

真实 `CPAL/WASAPI` adapter 可以建立基础骨架。

但是：

> **本阶段重点是 NullOutput。**

如果真实 WASAPI output 的 API 需要大量 Windows-specific 工程，先建立最小可运行 adapter，不为了追求功能数量污染 `aura-core`。

---

# 30. CPAL

使用 CPAL 作为真实输出设备 adapter。

要求：

- 默认设备；
- 根据实际设备格式创建 stream；
- 基础 f32/i16 等格式适配；
- 错误回调安全；
- stream start / pause / stop。

不得：

- 在 callback 中 lock；
- 在 callback 中 allocation；
- 在 callback 中 logging；
- 在 callback 中执行 decoder IO。

---

# 31. Callback 铁律

这是 M1 最重要的工程门禁之一。

Callback 内：

允许：

- 从 ring buffer 读取；
- sample copy；
- volume gain；
- 10ms smoothing；
- underrun counter；
- 简单原子计数。

禁止：

- malloc；
- Box；
- Vec grow；
- Mutex；
- RwLock；
- filesystem；
- network；
- decoder API；
- tracing；
- println；
- syscall；
- sleep；
- blocking；
- panic。

---

# 32. Zero Allocation Test

NullOutput 测试必须能证明：

> callback execution path 不产生 allocation。

可以使用：

- counting allocator；
- 专用 test allocator；
- 已预分配 scratch memory；

具体实现自行决定。

测试不能只检查“代码看起来没有 Vec”。

必须有真实机制。

---

# 33. Error Handling

定义：

```text
AudioError
```

至少考虑：

- InvalidFormat；
- DecodeError；
- UnsupportedFormat；
- ResampleError；
- OutputError；
- SeekError；
- InvalidCommand；
- InvalidParameter。

错误：

- 必须可诊断；
- 不允许 panic；
- 不应该包含敏感数据。

---

# 34. Resampler

使用：

`rubato`

目标：

让：

```text
decoder sample rate
```

转换到：

```text
output sample rate
```

例如：

44.1kHz → 48kHz。

要求：

- 正确 frame count；
- channel count 不损坏；
- 无明显 discontinuity；
- 测试确定性。

---

# 35. Resampler Test

至少：

```text
44100 → 48000
48000 → 44100
22050 → 48000
48000 → 22050
```

测试：

- 输出 frame 数符合预期；
- 末尾误差在合理范围；
- 左右声道保持一致；
- sine frequency 不发生明显变化。

不要添加主观“听起来不错”测试。

---

# 36. Channel Mapping

M1 只做最基础 channel policy。

至少：

```text
mono → stereo
stereo → stereo
```

mono → stereo：

左右一致。

后续多声道 downmix：

可以暂留 M1 interface，但不得扩张功能范围。

---

# 37. PCM Safety

所有进入 output 的 PCM：

必须处理：

- NaN；
- Infinity；
- unexpected values。

根据工程设计：

可以：

- clamp；
- sanitize；
- 将非有限值替换为 0。

行为必须确定性。

---

# 38. Thread Ownership

明确记录：

```text
Decoder Thread
Producer
Audio Consumer
Output Callback
Control Thread
```

每个对象只能有一个 owner。

尽量通过：

- channels；
- atomics；
- lock-free queue；

通信。

避免 shared mutable state。

---

# 39. Atomic State

对于 callback 所需实时参数：

允许使用：

```text
AtomicU32
AtomicU64
```

例如：

- target gain；
- playback generation；
- underrun counter。

不要用：

`Mutex<PlaybackState>`

作为 callback 热路径共享状态。

---

# 40. Testing Matrix

M1 至少必须拥有：

## Unit Tests

- format；
- sample conversion；
- gain；
- gain smoothing；
- generation；
- clock；
- position；
- channel mapping。

## Integration Tests

- synthetic PCM pipeline；
- decoder → PCM；
- resampler；
- ring buffer；
- NullOutput；
- play；
- pause；
- stop；
- seek；
- underrun。

## Deterministic Tests

- identical input → identical output；
- identical command sequence → identical output；
- seek → no stale generation；
- volume transition reproducible。

---

# 41. Real Audio Decoder Test

提供一个最小 fixture。

不得把大型真实音乐文件提交到仓库。

可以：

- 在测试运行时生成；
- 使用极小、合法、可公开分发的 fixture；
- 或使用项目自己的 deterministic audio encoder fixture。

文件必须非常小。

license / origin 明确。

---

# 42. No Network Tests

M1 测试：

禁止访问真实网络。

禁止：

- HTTP；
- Radio Browser；
- Jamendo；
- Internet Archive；
- Subsonic。

---

# 43. Test Failure Debugging

如果测试失败：

先定位：

- API misuse；
- race；
- ownership；
- frame accounting；
- generation；
- resampling；
- callback behavior；
- test flakiness。

不得直接：

- 放宽 assertion；
- 增加 sleep；
- ignored test；
- 重试到通过；
- 删除 test。

---

# 44. Performance Baseline

M1 建立基础 benchmark：

至少测：

- ring buffer throughput；
- decode throughput；
- resample throughput；
- NullOutput render throughput；
- callback execution time。

不要为了达到某个数字过度优化。

先获取：

> 真实基线。

写入：

`docs/PERF.md`

注明：

- machine；
- OS；
- Rust version；
- commit；
- test command；
- measured result。

---

# 45. Audio Callback Instrumentation

禁止在 callback 内 logging。

可以：

- atomic counters；
- preallocated diagnostics counters。

测试阶段：

由外部线程读取。

---

# 46. Logging

M1 的普通线程允许：

`tracing`

但 callback：

绝对禁止。

错误日志：

不要包含：

- filesystem secrets；
- tokens；
- URLs containing credentials；
- API keys。

---

# 47. Documentation

更新：

`ARCHITECTURE.md`

必须加入：

```text
Audio Core Architecture
Thread Model
Audio Pipeline
Generation Model
Clock Model
Callback Rules
Buffering Model
Error Model
```

更新：

`DECISIONS.md`

记录至少：

- ring buffer 选型；
- decoder 选型；
- internal PCM format；
- resampler 选择；
- clock 设计；
- generation 设计；
- NullOutput 设计；
- output abstraction；
- overrun policy。

更新：

`PROGRESS.md`

M1：

- IN PROGRESS；
- 完成项；
- 测试结果；
- 性能基线；
- 已知限制。

---

# 48. M1 不修改产品 UI

本阶段：

React UI 不需要加入完整播放器界面。

如确有必要：

可以只建立非常薄的 IPC placeholder / integration boundary。

但：

不得实现：

- Now Playing；
- player controls UI；
- Aurora；
- queue UI；
- command palette。

这些属于 M6。

---

# 49. Tauri 边界

如果需要 Tauri command：

只建立：

```text
load / play / pause / stop / seek
```

所需的最小契约。

但 audio engine 必须保持：

> 不依赖 Tauri。

如果无需修改 Tauri：

不要修改。

---

# 50. M1 完成门禁

必须全部通过：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

如果存在当前阶段可执行的前端门禁：

也运行：

```powershell
npm run typecheck
npm run lint
npm test
```

最终：

```powershell
cargo test --workspace
```

必须全绿。

---

# 51. 必须重点证明的 10 项

最终输出必须明确给出以下结果：

### 1
真实 decoder 可以解码。

### 2
PCM 可以经过 resampler。

### 3
PCM 可以进入 SPSC ring buffer。

### 4
NullOutput 可以确定性消费。

### 5
Play / Pause / Stop 正常。

### 6
seek 正确。

### 7
seek 后不存在 stale generation。

### 8
volume smoothing 正常。

### 9
underrun 输出静音且计数。

### 10
callback path 零分配。

---

# 52. M1 质量要求

不能接受：

- “大概可以播放”；
- “本地机器能响”；
- “测试没发现问题”；
- “理论上 callback 无 allocation”。

必须提供：

> **可重复、可自动化、可审计的证据。**

---

# 53. 最终 Audit

实现完成后：

先不要 commit。

进行：

# M1 Final Audit

## A. Architecture

## B. Decoder

## C. Resampler

## D. Ring Buffer

## E. Playback State

## F. Clock

## G. Generation / Seek

## H. Volume

## I. NullOutput

## J. Zero Allocation

## K. Error Handling

## L. Tests

## M. Performance

## N. Documentation

## O. Security / Dependency

每一项明确：

`PASS / FAIL / BLOCKED`

不得使用模糊表述。

---

# 54. Diff Review

执行：

```powershell
git diff --stat
git diff
```

检查：

- 是否偷渡 M2 功能；
- 是否引入无必要依赖；
- 是否存在 TODO；
- 是否存在 debug print；
- 是否存在 commented-out code；
- 是否存在 test weakening；
- 是否存在死代码；
- 是否存在 unsafe；
- 是否存在 callback violation；
- 是否存在平台耦合泄漏到 `aura-core`；
- 是否存在秘密；
- 是否存在真实音乐文件。

---

# 55. Git

只有：

- 所有 M1 测试通过；
- audit 全 PASS；
- diff review 干净；

才允许：

```text
git add
git commit
```

commit message：

```text
M1: implement audio core
```

**不要 push。**

---

# 56. 最终输出格式

完成后输出：

# M1 Final Audit

### A. Baseline
### B. Architecture
### C. Implemented
### D. Tests
### E. Zero Allocation
### F. Seek / Generation
### G. NullOutput
### H. Performance
### I. Dependencies
### J. Documentation
### K. Git

然后：

```text
M1 = PASS / BLOCKED
COMMIT = <hash>
NEXT = M2
```

如果 M1 = PASS：

**停止。**

不要开始 M2。