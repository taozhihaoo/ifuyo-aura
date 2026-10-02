# ifuyo Aura
# M2 — Gapless / Crossfade / Loudness / Hotplug / EQ

你现在进入 ifuyo Aura 项目的 **M2：播放质量与设备鲁棒性**。

项目正式名称：

`ifuyo Aura`

当前阶段目标：

> **在已经完成并通过审计的 M1 Audio Core 基础上，实现商业产品级的连续播放、交叉淡入淡出、响度归一化、输出设备热插拔，以及 [B] 级 10-band Parametric EQ。**

本阶段必须以 **M1 的真实实现为唯一基线**。

不得假设 M1 是理想实现。

必须先审计 M1，再在真实代码上增量实施。

---

# 0. 本阶段绝对边界

## 本阶段必须实现

[A]

1. 样本级 gapless playback。
2. 下一首预解码。
3. 当前曲目 → 下一曲目的无缝拼接。
4. crossfade，0~12 秒。
5. 等功率 crossfade 曲线。
6. gapless 与 crossfade 的互斥/协同策略。
7. loudness normalization 基础机制。
8. True Peak 限制机制。
9. 默认目标响度 `-16 LUFS`。
10. 输出设备变化检测。
11. 输出设备断开后的安全暂停。
12. 输出设备恢复后的可恢复状态。
13. 新设备采样率变化后的重新配置。
14. 设备切换期间不得 panic。
15. underrun / output error 状态保持可诊断。
16. 以上全部具备 deterministic / automated tests。

[B]

17. 10-band Parametric EQ。
18. preamp。
19. EQ presets。
20. 按输出设备记忆 EQ 配置的核心数据模型。

---

# 1. 本阶段明确禁止

不要实现：

- BPM analysis；
- Key detection；
- LUFS 文件扫描数据库；
- SQLite；
- FTS5；
- 音乐库；
- 文件夹扫描；
- filesystem watcher；
- artwork；
- playlist UI；
- queue UI；
- lyrics；
- Source system；
- Radio Browser；
- HTTP；
- HLS；
- cache；
- Dreamscape；
- Music Map；
- WebGL Aurora；
- SMTC；
- tray；
- installer；
- updater；
- website；
- Steam。

这些属于后续 Milestone。

---

# 2. 特别注意：不要偷渡 M3

M2 的 loudness normalization **不能因为最终产品需要 SQLite/音乐库而提前实现完整的 Library / Analysis 系统**。

M2 只负责：

> **音频核心如何消费“已经存在的响度元数据”。**

建立稳定的核心数据结构，例如：

```text
TrackLoudness
  integrated_lufs
  true_peak_db
  loudness_available
```

由未来 M3/M8 等阶段负责：

- 扫描；
- 分析；
- 持久化；
- 更新。

M2 可以使用：

- synthetic metadata；
- test fixtures；
- 手工构造的 TrackLoudness；

进行完整测试。

不要在 M2 引入 SQLite。

---

# 3. 开始前：Baseline Audit

在任何修改之前：

执行真实仓库审计。

至少：

```powershell
git status
git branch
git log --oneline -10
cargo metadata
cargo tree
cargo test --workspace
```

检查：

- M1 commit；
- M1 测试数量；
- M1 warnings；
- 当前 crate 结构；
- `aura-core`；
- `aura-testkit`；
- decoder；
- resampler；
- ring buffer；
- output；
- clock；
- generation；
- gain；
- NullOutput；
- CPAL adapter。

输出：

# M2 Baseline Audit

A. Git  
B. M1 Status  
C. Audio Pipeline  
D. Output Layer  
E. Test Infrastructure  
F. Dependencies  
G. Existing Limitations  
H. Risks

---

# 4. M1 不合格时怎么办

如果 M1 存在真正影响 M2 的缺陷：

不得直接绕过。

先修复最小必要缺陷。

例如：

- generation 逻辑错误；
- frame accounting 错误；
- output state 错误；
- callback 线程安全问题；
- ring buffer 所有权问题。

但：

> 不得借 M2 机会重写整个 M1。

任何 M1 修复：

必须在 `DECISIONS.md` / `PROGRESS.md` 记录。

如果只是非阻塞的历史代码问题：

记录，不扩大范围。

---

# 5. Gapless 总体架构

M2 需要将 M1：

```text
Decoder
↓
Resampler
↓
Ring Buffer
↓
Output
```

扩展为：

```text
Current Track Decoder
        ↓
   Current Pipeline
        ↓
    Audio Buffer
        ↓
      Mixer
        ↑
Next Track Decoder
        ↓
Next Pipeline
```

Mixer 是 M2 的关键新层。

要求：

- 仍然满足 callback 铁律；
- mixer 所需状态预分配；
- callback 不做动态内存分配。

---

# 6. Next Track Predecode

下一首必须在当前曲目结束前开始准备。

最低要求：

> 当前曲目进入剩余 N 秒窗口时，提前建立下一曲目的 decode pipeline。

N 必须可配置。

默认：

`3 seconds`

具体策略可根据实际 decoder latency 调整。

---

# 7. Gapless 的目标

对于支持 gapless 的编码：

[A]

- MP3；
- FLAC；
- Vorbis；
- ALAC；

要求：

当前曲目有效 PCM 的最后一个 frame：

必须紧接：

下一曲目第一个有效 PCM frame。

不得：

- 插入静音；
- 插入额外 frame；
- 发生重复 frame；
- 发生 frame 丢失。

---

# 8. Gapless Frame Accounting

必须建立：

```text
TrackFrameRange
```

或等价结构。

明确：

- source frames；
- decoder padding；
- priming；
- valid frames；
- rendered frames。

对于 symphonia 实际暴露的 gapless metadata：

必须真实消费。

不要自己猜 trim 值。

---

# 9. Gapless Test Fixture

创建两个确定性 fixture：

## Track A

1 秒。

内容：

440Hz sine。

## Track B

1 秒。

内容：

880Hz sine。

通过 deterministic renderer：

```text
A + B
```

验证：

- 总有效 frame 数正确；
- A 最后 frame 与 B 第一 frame 相邻；
- 无额外 silence；
- 无重复；
- 无遗漏。

---

# 10. Impulse Gapless Test

除了 sine：

必须增加：

```text
Track A = impulse at final frame
Track B = impulse at first frame
```

渲染后：

两个 impulse 必须：

> 恰好出现在两个相邻 frame。

这个测试比“听起来没有间隙”更可靠。

---

# 11. Gapless 长时间测试

至少创建：

```text
A → B → A → B → A → B
```

连续循环。

确认：

- frame count；
- generation；
- track boundary；
- 不产生逐曲累计偏移。

---

# 12. Crossfade

实现：

```text
0.0 ~ 12.0 seconds
```

配置。

默认建议：

`6 seconds`

但具体默认值必须以产品行为实际验证后写入 `DECISIONS.md`。

---

# 13. Equal-Power Curve

使用等功率曲线。

不要使用简单：

```text
gain_a = 1 - t
gain_b = t
```

避免中心段整体能量明显下降。

目标：

```text
gain_a(t)
gain_b(t)
```

满足近似恒定功率关系。

所有曲线计算：

必须是：

- deterministic；
- allocation-free；
- 可单元测试。

---

# 14. Crossfade Frame Model

crossfade 必须基于：

> audio frames

而不是：

> wall-clock milliseconds

最终：

```text
crossfade_frames
```

由：

```text
duration × sample_rate
```

转换。

必须正确处理：

- 44.1kHz；
- 48kHz；
- 96kHz。

---

# 15. Gapless 与 Crossfade 的关系

必须定义明确规则。

建议：

## 情况 A

同专辑连续曲目：

默认：

> gapless，不 crossfade。

## 情况 B

不同来源 / 不同专辑：

默认：

> 如果 crossfade > 0，则 crossfade。

## 情况 C

用户明确关闭 crossfade：

> 一律优先 gapless。

记录实际决策到：

`DECISIONS.md`

---

# 16. Crossfade 测试

必须测试：

### 0 秒

行为等价于普通 track transition。

### 1 秒

正确 overlap。

### 6 秒

正确 overlap。

### 12 秒

正确 overlap。

### Crossfade > Track Duration

不能 panic。

必须按明确规则降级。

---

# 17. Crossfade Determinism

同一个：

- Track A；
- Track B；
- volume；
- crossfade duration；
- sample rate；

必须得到：

> 完全一致的 PCM。

不得依赖：

`Instant::now()`。

---

# 18. Crossfade Boundary Test

使用：

```text
A = 440Hz
B = 880Hz
```

截取过渡窗口。

验证：

- A gain 单调下降；
- B gain 单调上升；
- 无 overshoot；
- 不出现 NaN；
- 不出现 Infinity。

---

# 19. Loudness Normalization

实现：

```text
target_lufs = -16 LUFS
```

但 M2 不负责分析 LUFS。

M2 只消费：

```text
TrackLoudness
```

例如：

```text
integrated_lufs = -20
true_peak_db = -3
```

计算目标增益：

```text
gain_db = target_lufs - integrated_lufs
```

再转换为 linear gain。

---

# 20. Loudness Safety Limit

不得无限增加增益。

必须考虑：

True Peak。

例如：

歌曲：

`-24 LUFS`

但：

`true_peak = -0.1 dBTP`

如果直接增加 8 LU：

可能产生严重 clipping。

因此：

> loudness gain 必须受 peak safety 限制。

具体算法：

可以使用：

- peak-derived ceiling；
- soft limiter；

但必须：

- deterministic；
- no allocation；
- no panic。

---

# 21. Soft Limiter

实现最小可靠 limiter。

要求：

- 不产生明显硬 clipping；
- 输出 peak 小于安全上限；
- 不出现 NaN；
- 不 overshoot。

目标最终 peak：

优先保持：

`<= -1.0 dBFS`

如果具体实现选择其他安全余量：

必须写入 `DECISIONS.md`。

---

# 22. Loudness Test Fixtures

提供：

### Fixture 1

`-16 LUFS`

预期：

约：

`0 dB`

target gain。

### Fixture 2

`-20 LUFS`

预期：

约：

`+4 dB`

但如果 true peak ceiling 不允许：

必须正确限制。

### Fixture 3

`-12 LUFS`

预期：

约：

`-4 dB`

---

# 23. Loudness Off

必须允许：

```text
enabled = false
```

关闭时：

> 不对 track 做 loudness gain。

原始音量路径保持一致。

---

# 24. Loudness + User Volume

明确两种增益关系：

```text
Track Loudness Gain
        ×
User Volume
        ×
EQ / Preamp
        ×
Limiter
```

实际顺序必须结合 EQ / limiter 的 DSP 设计确定。

重点：

> 不允许 EQ preamp 把最终输出重新推爆。

记录：

`DECISIONS.md`

---

# 25. Loudness Metadata 缺失

如果：

```text
loudness_available = false
```

默认行为：

> 不进行响度归一。

不得：

- 猜测；
- 使用随机默认值；
- 阻塞播放。

保留：

`gain = 1.0`

或者产品确定的等价安全策略。

---

# 26. Hotplug

实现输出设备变化处理。

必须覆盖：

### Device Connected

### Device Removed

### Default Device Changed

### Device Format Changed

### Stream Error

---

# 27. Device Removal

设备被拔出时：

必须：

1. 停止/失效当前 output stream；
2. 保留当前 track；
3. 保留当前 frame position；
4. 进入安全暂停状态；
5. 记录状态；
6. 不 panic；
7. 不在 callback 阻塞等待。

UI 将由后续 M6 实现。

M2 只提供核心状态。

---

# 28. Device Recovery

设备重新出现：

不得自动播放。

应进入：

> 可恢复暂停状态。

重新建立：

- sample rate；
- channel layout；
- output stream；
- resampler。

恢复后：

用户可以继续播放。

---

# 29. Default Device Change

即使旧设备仍然存在：

Windows 默认输出设备发生变化：

必须能被检测。

推荐：

> 外部管理线程 / polling / platform event adapter

而不是 callback 负责设备检测。

不得在 callback：

- 查询设备；
- 调用 Windows APIs。

---

# 30. Hotplug Detection

具体实现方式：

根据当前 CPAL / Windows API 的实际能力选择。

允许：

- event-based；
- 外部 polling；
- adapter-specific notification。

不允许：

> audio callback 内 device enumeration。

检测频率必须合理。

不得高频轮询到影响 CPU。

真实决策写入：

`DECISIONS.md`

---

# 31. Device Reinitialization

新设备：

可能与旧设备具有不同：

- sample rate；
- channel count；
- sample format。

必须重新建立 output pipeline。

例如：

```text
48000 stereo
        ↓
device removed
        ↓
44100 stereo device
        ↓
reinitialize resampler/output
```

不得假设：

> 所有输出设备都是 48kHz stereo。

---

# 32. Position Preservation

设备热插拔后：

当前 track position：

必须保持。

不得：

- 自动跳回 0；
- 丢失大量 position；
- 因设备重新初始化产生明显 duplicate samples。

允许：

极短恢复间隙。

但：

不得破坏逻辑 frame position。

---

# 33. Hotplug Tests

CI 中：

不能依赖真实 Windows 声卡。

必须使用：

`NullOutput`

模拟：

- device connected；
- device removed；
- device restored；
- sample rate changed；
- channel changed；
- stream error。

真实设备测试：

继续使用：

`#[ignore]`

并保持独立。

---

# 34. EQ — [B]

实现：

> 10-band Parametric EQ。

本阶段属于 B 级功能。

即：

如果实现质量受到阻碍，不能破坏 A 级功能。

但是：

如果当前环境正常，应完整实现。

---

# 35. EQ Architecture

EQ 必须位于：

> output callback 可安全执行的 DSP 路径。

因此：

所有 biquad 状态：

必须：

- 预先创建；
- 预分配；
- 不动态 allocation；
- 不加锁。

---

# 36. EQ Bands

建立 10 个默认 band。

不要硬编码在业务 UI。

核心只需要：

```text
EqBand
  frequency
  gain_db
  q
  enabled
```

UI 后续负责展示。

---

# 37. Parametric EQ

推荐使用：

RBJ biquad cookbook

或稳定的等价实现。

支持：

- Peak；
- 低架；
- 高架；

但如果本阶段要控制复杂度：

核心可以先使用 Peak bands。

具体选择：

记录 `DECISIONS.md`。

---

# 38. EQ Presets

至少：

```text
Flat
Bass Boost
Treble Boost
Vocal
Electronic
Acoustic
```

Preset 必须只是数据。

不得复制十套 DSP 实现。

---

# 39. Preamp

范围建议：

`-12 dB ~ +12 dB`

默认：

`0 dB`

Preamp 改变后：

不能产生未定义行为。

---

# 40. EQ Stability

必须测试：

- 全 Flat；
- 极端 boost；
- 极端 cut；
- 所有 band simultaneous；
- 高频；
- 低频；
- silence；
- DC input；
- impulse；
- sine；
- 长时间 render。

不得：

- NaN；
- Infinity；
- runaway；
- sample explosion。

---

# 41. EQ Parameter Update

用户修改 EQ：

不得重建整个 audio engine。

应：

> 更新目标参数，并在安全边界生效。

如果更新 filter coefficients 需要重算：

可在非 callback 线程计算。

callback：

只消费预计算后的 coefficient snapshot。

---

# 42. EQ Parameter Synchronization

不得：

```text
Mutex<Vec<Biquad>>
```

进入 callback。

推荐：

- immutable snapshot；
- atomic parameter generation；
- preallocated double buffer；

具体方式自行选择。

重点：

> callback 不锁。

---

# 43. EQ Zero Allocation

增加到 M1 的 callback zero-allocation 测试。

证明：

以下同时开启：

- volume smoothing；
- loudness gain；
- limiter；
- EQ；

callback：

仍然零 allocation。

---

# 44. DSP Order

最终建立明确：

```text
Input PCM
   ↓
Track Gain / Loudness
   ↓
User Volume / Preamp
   ↓
EQ
   ↓
Limiter
   ↓
Output
```

如果经过实际测试后必须调整：

必须记录：

- 为什么；
- 修改前后；
- 对 clipping / headroom 的影响。

---

# 45. Headroom

EQ + loudness + volume：

组合情况下：

不得简单：

```text
sample *= gain
```

之后直接输出。

必须有最终安全处理。

测试：

- loudness +12dB；
- EQ +12dB；
- preamp +12dB；
- volume 1.0；

仍不得出现非法 PCM。

---

# 46. Audio State Extension

在 M1 基础上扩展状态：

例如：

```text
PlaybackState
OutputState
TransitionState
LoudnessState
EqState
```

但不要建立巨型：

`GlobalAudioState`

优先按职责拆分。

---

# 47. Transition State

gapless / crossfade：

建议建立：

```text
Normal
PreparingNext
GaplessTransition
Crossfading
```

或者等价模型。

不得引入复杂状态机框架。

---

# 48. End-of-Track Handling

必须精确区分：

### Natural End

正常播放至末尾。

### Stop

用户主动停止。

### Seek Near End

跳到接近结尾。

### Seek To End

需要明确处理。

### Device Failure

不是曲目结束。

不得因为 output error：

误触发 next track。

---

# 49. Seek During Crossfade

必须定义行为。

建议：

> seek 会立即产生新 generation，并取消当前 transition。

旧：

- current;
- next;
- crossfade samples；

全部失效。

seek 后：

重新从目标位置构建 playback pipeline。

这是 M1 generation 机制在 M2 的正式扩展。

---

# 50. Stop During Crossfade

Stop：

必须：

- 停止当前 output；
- 取消 transition；
- 清除 next-track state；
- generation++;
- 不保留 stale samples。

---

# 51. Pause During Crossfade

Pause：

必须：

- 停止消费；
- 保留 transition state；
- 不继续推进 position。

Resume：

从暂停位置继续。

不得因为 pause：

重新开始 crossfade。

---

# 52. Crossfade + Underrun

如果 transition 期间发生 underrun：

必须：

- 输出静音；
- 记录 underrun；
- 保持 transition state；
- 不 panic。

不得因为一次 underrun：

损坏 generation。

---

# 53. Tests — Gapless

至少：

```text
test_gapless_total_frames
test_gapless_no_duplicate_frames
test_gapless_no_missing_frames
test_gapless_impulse_boundary
test_gapless_repeated_transitions
test_gapless_generation_integrity
```

---

# 54. Tests — Crossfade

至少：

```text
test_crossfade_zero
test_crossfade_one_second
test_crossfade_six_seconds
test_crossfade_twelve_seconds
test_crossfade_longer_than_track
test_crossfade_gain_monotonic
test_crossfade_no_nan
test_crossfade_deterministic
```

---

# 55. Tests — Loudness

至少：

```text
test_loudness_disabled
test_loudness_minus16
test_loudness_quieter_track
test_loudness_louder_track
test_loudness_peak_safety
test_loudness_missing_metadata
test_loudness_deterministic
```

---

# 56. Tests — Hotplug

至少：

```text
test_device_removed
test_device_restored
test_device_format_change
test_default_device_change
test_device_error_does_not_panic
test_position_survives_device_reinitialization
```

---

# 57. Tests — EQ

至少：

```text
test_eq_flat
test_eq_bass_boost
test_eq_treble_boost
test_eq_extreme_gain
test_eq_silence
test_eq_impulse
test_eq_stability
test_eq_deterministic
```

---

# 58. Integration Tests

必须至少测试：

```text
Track A
 ↓
Gapless
 ↓
Track B
```

以及：

```text
Track A
 ↓
Crossfade
 ↓
Track B
```

以及：

```text
Track
 ↓
Loudness
 ↓
EQ
 ↓
Limiter
 ↓
Output
```

以及：

```text
Playing
 ↓
Device Removed
 ↓
Paused / Recoverable
 ↓
Device Restored
 ↓
Resume
```

---

# 59. Full DSP Determinism

固定：

- source；
- commands；
- volume；
- loudness metadata；
- EQ parameters；
- sample rate；
- crossfade duration；
- virtual clock。

执行：

```text
render()
render()
```

输出必须：

> bit-identical。

---

# 60. Numerical Safety

对所有 DSP 输出检查：

```text
is_finite()
```

必须测试：

- NaN；
- +Infinity；
- -Infinity。

任何非有限数据：

不得到达最终 output。

---

# 61. No Allocation Gate

扩展 M1 原来的 zero-allocation test。

必须在以下全部开启时验证：

```text
volume smoothing
+
loudness
+
limiter
+
EQ
+
crossfade
```

callback：

> 零动态内存分配。

---

# 62. No Lock Gate

同样验证：

callback 热路径：

不存在：

- Mutex；
- RwLock；
- parking_lot lock；
- channel blocking receive；
- synchronous wait。

允许：

- atomic；
- lock-free structures；
- preallocated memory。

---

# 63. Callback Timing

增加：

callback execution benchmark。

测试：

```text
Flat EQ
EQ active
Crossfade active
Limiter active
Full DSP chain
```

记录：

- average；
- p95；
- p99；
- worst observed。

不能伪造。

---

# 64. Long Soak Test

至少执行：

> 1 小时等价 playback soak。

如果 CI 不适合每次跑完整 1 小时：

建立：

- normal CI test；
- extended local soak test。

不得把“未运行”写成 PASS。

---

# 65. Long Soak 内容

循环：

```text
A
B
A
B
```

同时：

- gapless；
- crossfade；
- loudness；
- EQ。

验证：

- no panic；
- no memory growth beyond expected bounds；
- no stale generation；
- no progressive drift；
- no invalid PCM；
- no accumulating underrun in NullOutput。

---

# 66. M2 性能基线

更新：

`docs/PERF.md`

至少记录：

- gapless transition cost；
- crossfade DSP cost；
- EQ cost；
- limiter cost；
- full DSP callback cost；
- device recovery cost；
- memory footprint。

使用真实测试结果。

---

# 67. Dependency Review

如果添加 DSP 依赖：

必须检查：

- license；
- MSRV；
- Windows；
- CI；
- maintenance；
- binary impact；
- cargo-deny。

优先：

> 不添加额外 DSP crate，除非必要。

已有依赖优先复用。

---

# 68. Documentation

更新：

`ARCHITECTURE.md`

增加：

```text
Transition Engine
Gapless Model
Crossfade Model
Loudness Pipeline
Limiter
EQ Architecture
Output Device Lifecycle
Hotplug Recovery
```

更新：

`DECISIONS.md`

至少记录：

- next-track predecode window；
- gapless implementation；
- gapless metadata handling；
- crossfade curve；
- gapless/crossfade policy；
- loudness gain formula；
- true peak safety strategy；
- limiter；
- EQ implementation；
- EQ synchronization；
- hotplug strategy；
- position preservation。

更新：

`PROGRESS.md`

M2：

记录：

- 实现项；
- 测试项；
- 性能；
- known limitations；
- A/B 状态。

---

# 69. M2 不修改产品 UI

本阶段原则上：

React UI 不实现播放器界面。

如果需要：

只能建立：

> 未来 M6 使用的最小 backend state / command contract。

不要实现：

- EQ 面板；
- crossfade UI；
- device selector UI；
- loudness toggle UI；
- Now Playing；
- Aurora。

---

# 70. M2 不修改 Source / Library

不要为了测试方便提前添加：

- SQLite；
- real music scanner；
- HTTP source；
- Radio Browser；
- cache。

所有测试：

继续使用：

- synthetic fixtures；
- local fixtures；
- NullOutput。

---

# 71. M2 Final Quality Gate

必须执行：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

如果 M0/M1 已经建立前端质量门禁：

继续执行：

```powershell
npm run typecheck
npm run lint
npm test
```

然后：

```powershell
cargo test --workspace
```

确保所有测试重新通过。

---

# 72. 重点审计：M1 回归

M2 完成后必须再次验证 M1：

```text
decoder
resampler
ring buffer
play/pause/stop
seek
generation
volume
smoothing
NullOutput
zero allocation
```

任何 M1 regression：

都视为 M2 FAIL。

---

# 73. Diff Review

完成实现后：

先不要 commit。

执行：

```powershell
git diff --stat
git diff
```

逐项检查：

- 是否修改了 M1 不必要部分；
- 是否偷渡 M3；
- 是否新增无必要依赖；
- callback 是否 lock-free；
- callback 是否 allocation-free；
- 是否新增 wall-clock 依赖；
- 是否存在真实网络测试；
- 是否存在 debug print；
- 是否存在 TODO；
- 是否存在 ignored test；
- 是否弱化 assertion；
- 是否存在 unsafe；
- 是否存在秘密；
- 是否提交真实音乐；
- 是否出现平台耦合泄漏到 `aura-core`。

---

# 74. 最终 M2 Audit

输出：

# M2 Final Audit

## A. Baseline

## B. Gapless

## C. Crossfade

## D. Loudness

## E. Limiter

## F. Hotplug

## G. EQ

## H. Generation / Seek Regression

## I. Callback Safety

## J. Determinism

## K. Tests

## L. Performance

## M. Dependencies

## N. Documentation

## O. Scope Compliance

每一项：

`PASS / FAIL / BLOCKED`

禁止：

“基本完成”

“应该没有问题”

“预计通过”

---

# 75. 必须提供的关键证据

最终输出必须明确提供：

### Gapless

- total frame count；
- boundary test；
- repeated transition test。

### Crossfade

- 0s；
- 1s；
- 6s；
- 12s；
- deterministic test。

### Loudness

- target；
- gain calculation；
- peak safety；
- missing metadata behavior。

### Hotplug

- remove；
- restore；
- format change；
- position preservation。

### EQ

- flat；
- preset；
- extreme；
- numerical stability。

### Callback

- zero allocation；
- no lock；
- benchmark。

---

# 76. Git Commit

只有：

- 所有 A 级功能 PASS；
- B 级 EQ 如果声明完成则必须 PASS；
- M1 regression 全 PASS；
- quality gate 全绿；
- final audit 全 PASS；
- diff review 干净；

才允许：

```powershell
git add .
git commit -m "M2: implement playback quality"
```

不要 push。

---

# 77. PROGRESS

更新：

```text
M2 = PASS
```

并记录：

- A 功能完成项；
- B 功能是否完成；
- 测试数量；
- 性能基线；
- 已知限制。

如果 EQ 没完成：

必须明确：

```text
M2-A = PASS
M2-B(EQ) = NOT COMPLETED
```

不能伪装成完整完成。

---

# 78. 最终输出

最终只输出真实结果：

```text
# M2 Final Audit

A. Baseline
B. Gapless
C. Crossfade
D. Loudness
E. Hotplug
F. EQ
G. Regression
H. Callback Safety
I. Tests
J. Performance
K. Dependencies
L. Documentation
M. Scope

M2 = PASS / BLOCKED
COMMIT = <hash>
NEXT = M3
```

如果：

`M2 = PASS`

立即停止。

**不要开始 M3。**