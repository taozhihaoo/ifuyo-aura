# ifuyo Aura
# M8 中篇 — Feature Analysis Engine (§50–204)

> 本文档为原 M8 开发提示词的**中篇（特征分析引擎，§50–204）**。
> 上篇《ifuyo Aura — M8 上篇 分析基建（Domain Model、Identity、Persistence、Embedding Policy）开发提示词.md》承载 §0–49；下篇《ifuyo Aura — M8 下篇 运行时、集成与发布质量（Queue、Soak、Integration、Gates）开发提示词.md》承载 §205–390。
> 章节编号沿用原 M8 全局编号；跨篇引用按编号在三篇中查找。
> 阅读本文前，上篇所定义的基建必须已就绪。

# 50. BPM / Tempo

建立：

真实：

> tempo estimation pipeline。

不能：

只找一个最强频率：

就声称：

BPM。

---

# 51. Tempo Range

默认：

限制合理：

```text 20–300 BPM
```

具体范围：

可根据算法调整。

任何超界结果：

必须：

- 重新解释倍频 / 半频；
- 或标记 invalid。

不能：

出现：

```text 1200 BPM
```

并写入数据库。

---

# 52. Half / Double Tempo

BPM：

常见：

```text 60
↔
120
↔
240
```

算法：

必须：

具备：

> 合理倍频 / 半频处理策略。

至少：

在测试中：

明确：

> 选择规则。

---

# 53. BPM Confidence

必须：

输出：

```text bpm
bpm_confidence
```

confidence：

必须：

有：

明确：

```text [0,1]
```

语义：

必须：

写入：

`DECISIONS.md`

---

# 54. BPM Invalid Cases

例如：

- silence；
- pure noise；
- extremely short clip；
- spoken-only audio；
- malformed audio。

不能：

> 强行产生 BPM。

应该：

`Unavailable / LowConfidence / Failed`

根据实际模型。

---

# 55. BPM Fixture

建立：

确定性测试音频：

```text 60 BPM
90 BPM
120 BPM
128 BPM
140 BPM
180 BPM
```

至少：

使用：

> synthetic click / impulse train

以及：

> synthetic rhythmic audio。

---

# 56. BPM Real-World Fixture

可以：

加入：

极小的：

> 合法、可公开分发的短音频 fixture。

必须：

记录：

- source；
- license；
- expected range。

不得：

提交完整商业歌曲。

---

# 57. Beat Grid

[A]

至少：

返回：

```text beat_positions
beat_count
```

或：

> equivalent compact representation。

必须：

> frame-based / sample-based

而不是：

只保存浮点 seconds。

---

# 58. Beat Grid Version

Beat grid：

必须：

与：

analysis version：

绑定。

算法改变：

beat positions：

可能改变。

---

# 59. Beat Grid Bounds

不能：

因为错误输入：

生成：

> 数百万个 beats。

必须：

有：

- duration bound；
- max beat count；
- memory bound。

---

# 60. Beat Grid Determinism

相同：

```text audio
algorithm
version
```

必须：

得到：

> 相同 beat sequence。

---

# 61. Key Detection

建立：

真实：

> musical key estimation。

至少：

支持：

```text C
C#
D
...
B
```

以及：

```text Major
Minor
```

或：

等价 key model。

---

# 62. Chroma

Key detection：

优先使用：

> chroma / pitch-class energy

或：

可靠等价算法。

中间结果：

可以：

只保留 summary，

无需将完整 chromagram 永久写库。

---

# 63. Key Confidence

输出：

```text key
key_confidence
```

范围：

```text 0–1
```

必须：

定义。

---

# 64. No Key Guess

如果：

音频：

- silence；
- noise；
- speech；

算法：

不可靠：

必须：

允许：

`Unavailable / LowConfidence`

不得：

强行：

`C Major`

---

# 65. Camelot

如果：

key model：

可靠：

可以：

映射：

```text Camelot key
```

例如：

`8A`

但：

必须：

由真实 key algorithm 结果派生。

不能：

random。

---

# 66. Key Enharmonic Policy

必须定义：

例如：

```text C# major
Db major
```

如何：

canonicalize。

同一 key：

必须：

使用：

> 稳定 canonical representation。

---

# 67. LUFS

M8：

必须：

真实计算：

> Integrated Loudness。

目标：

与：

M2：

`TrackLoudness`

兼容。

---

# 68. LUFS Standards

实际实现：

必须：

以：

工程选择的：

> 可验证 loudness standard / implementation

为依据。

必须记录：

- standard；
- gating；
- channel weighting；
- integration semantics；
- version。

---

# 69. LUFS Output

至少：

```text
integrated_lufs
loudness_range_lu
true_peak_dbtp
```

如果标准实现：

还可保存：

```text momentary
short_term
```

但：

不需要为了数量：

无限扩展。

---

# 70. True Peak

不能：

简单：

取 sample peak：

然后：

声称：

> true peak。

必须：

采用：

> 明确的 true-peak oversampling / equivalent standard-compliant method。

具体实现：

记录。

---

# 71. LUFS Numeric Safety

所有 loudness：

必须：

检查：

- finite；
- range；
- channel validity；
- duration validity。

无效：

不得：

写入 SQLite。

---

# 72. Silence

完全 silence：

LUFS：

可能：

> 不可定义 / extremely low。

必须：

有：

明确：

> invalid / below measurable threshold

行为。

不要：

产生：

`-0 LUFS`

之类错误值。

---

# 73. Loudness Fixture

至少：

建立：

确定性：

```text
-12 LUFS
-16 LUFS
-20 LUFS
-23 LUFS
```

附近的：

合法测试 fixture。

由于测试误差：

必须：

使用：

> explicit tolerance。

不要：

强行：

bit-exact。

---

# 74. M2 Cross-Validation

M8：

生成：

```text TrackLoudness
```

后：

必须：

通过：

M2 的：

loudness consumption tests。

确认：

```text M8 output
→ M2
```

行为：

> 正确。

---

# 75. Spectral Features

M8：

至少：

计算：

```text RMS
Peak
Spectral Centroid
Spectral Rolloff
Brightness
```

必要时：

增加：

```text Spectral Flux
Zero Crossing Rate
```

但：

不要：

为了“功能很多”无限添加。

---

# 76. Energy

定义：

> 一个明确的 energy metric。

例如：

> RMS-derived normalized energy

或者：

其它：

可解释方法。

必须：

有：

```text [0,1]
```

映射：

或明确其它范围。

---

# 77. Energy Semantics

必须记录：

energy：

到底代表：

- signal amplitude；
- perceived energy；
- spectral energy；

不能：

名称叫：

`energy`

却：

算法没有定义。

---

# 78. Brightness

定义：

> 可重复的 spectral brightness metric。

至少：

保证：

```text finite
bounded
deterministic
```

---

# 79. Spectral Feature Windowing

必须定义：

- sample rate；
- window type；
- FFT size；
- hop size；
- analysis duration；
- aggregation。

不能：

同一项目：

不同模块：

随意使用不同参数。

---

# 80. FFT

如果需要 FFT：

优先：

- 使用已有依赖；
- 或实现小型、可靠、经过测试的内部 FFT；

不要：

引入：

> 大型 DSP framework

只为了：

一个 FFT。

---

# 81. MFCC

实现真实：

> Mel-frequency cepstral coefficients。

必须明确：

- sample rate；
- pre-emphasis；
- frame window；
- FFT size；
- mel filter count；
- min/max frequency；
- DCT；
- coefficient count。

---

# 82. MFCC Count

建议：

使用：

```text
13 coefficients
```

或：

项目实际确认的稳定配置。

具体：

记录：

`DECISIONS.md`

---

# 83. MFCC Storage

完整 frame-level MFCC：

可能非常大。

M8：

默认：

可以保存：

```text
mean
stddev
min
max
```

等：

> compact summary。

如果 M9/B 级需要完整时间序列：

另行实现。

---

# 84. MFCC Determinism

固定：

- audio；
- sample rate；
- parameters；

必须：

> 输出一致。

考虑：

浮点 tolerance。

---

# 85. MFCC Numerical Safety

不得产生：

- NaN；
- Infinity；
- negative infinity；
- overflow。

log：

输入：

不得：

`log(0)`。

必须：

> 使用稳定 epsilon / flooring。

---

# 86. Analysis Preprocessing

必须建立：

统一：

```text
decoder PCM
↓
channel policy
↓
sample rate policy
↓
analysis buffer
↓
feature extraction
```

---

# 87. Analysis Sample Rate

不要：

每个算法：

自己随便 resample。

应该：

统一：

> canonical analysis sample rate

例如：

```text 22050
44100
48000
```

具体值：

根据：

- BPM；
- key；
- MFCC；
- embedding；

综合决定。

记录：

`DECISIONS.md`

---

# 88. Analysis Channel Policy

至少定义：

```text mono
stereo
```

推荐：

> 分析使用稳定 mono fold-down

除非：

某算法明确需要 stereo。

不能：

不同算法：

默默使用不同 channel semantics。

---

# 89. Downmix

如果：

stereo → mono：

必须：

使用：

明确的安全 downmix policy。

例如：

```text (L + R) / 2
```

或：

等价方案。

必须：

测试：

- identical channels；
- opposite phase；
- silent channel。

---

# 90. Full-Track vs Windowed Analysis

长歌曲：

不能：

一次：

```text Vec<f32> entire 2-hour audio
```

全部载入内存。

必须：

> streaming / chunked / windowed analysis。

---

# 91. Analysis Memory Bound

分析：

内存：

必须：

有：

> 明确上限。

不应：

随：

歌曲长度：

线性无限增长。

---

# 92. Analysis Pipeline

推荐：

```text
Track
 ↓
Decoder
 ↓
Chunked PCM
 ↓
Canonical Analysis Stream
 ↓
┌───────────────┬───────────────┬───────────────┐
Tempo           Key             Loudness
│               │               │
Spectral        MFCC            Embedding
└───────────────┴───────────────┴───────────────┘
 ↓
Validated Result
 ↓
SQLite Transaction
```

---

# 93. Shared Decode

尽量避免：

BPM：

自己完整解码一次。

Key：

再完整解码一次。

MFCC：

又解码一次。

应尽量：

> 一次顺序扫描共享基础分析 features。

具体：

根据 decoder / algorithm constraints 实现。

---

# 94. Multi-Pass Analysis

如果：

某些算法必须：

多遍扫描：

允许：

multi-pass。

但：

必须：

明确：

- 为什么；
- 哪些 feature；
- 时间成本。

---

# 95. Analysis Scheduling

建立：

```text AnalysisScheduler
```

负责：

- pending jobs；
- concurrency；
- priority；
- cancellation；
- retry；
- status。

---

# 96. Priority

至少：

可以：

```text UserRequested
RecentlyPlayed
LibraryNew
Background
```

具体优先级：

根据实际产品决定。

不要：

为了复杂而做十几个等级。

---

# 97. Concurrency

分析：

不能：

> 无限制并发。

必须：

配置：

```text max_workers
```

推荐：

> CPU cores 的有限比例。

具体：

通过 benchmark 决定。

---

# 98. Audio Playback Priority

播放音乐：

优先级：

高于：

background analysis。

M8：

不能：

因为 CPU 分析：

导致：

M1/M2 underrun。

---

# 99. Analysis Worker Isolation

analysis worker：

不得：

进入：

audio callback。

严格禁止：

```text callback
 ↓
analysis
```

---

# 100. CPU Budget

允许：

设置：

```text analysis_cpu_budget
```

或：

worker count。

播放中：

可以：

> 降低分析并发。

暂停播放：

可以：

> 提高分析吞吐。

---

# 101. Battery / Power Policy

[B]

可以：

未来：

根据：

Windows power state：

降低：

background analysis。

本阶段：

可以：

预留：

policy interface。

不要：

实现复杂 power manager。

---

# 102. Cancellation

任何：

analysis job：

必须：

支持：

```text cancel()
```

取消后：

状态：

```text Cancelled
```

并且：

不能：

产生：

`Completed`。

---

# 103. Cancellation Boundary

取消：

可以发生在：

- decode chunk；
- feature extraction block；
- embedding inference；
- DB commit 前。

不能：

在：

> transaction commit 中途

制造：

半提交状态。

---

# 104. No Fake Progress

如果：

没有办法准确计算总工作量：

不要：

显示：

```text 63%
```

可以：

显示：

```text Analyzing…
Track 12 / 100
```

或者：

真实可计算的阶段进度。

---

# 105. Analysis Progress

建议：

提供：

```text AnalysisProgress
  job_id
  track_id
  stage
  processed_frames
  total_frames
  elapsed
  status
```

未来 M6：

可直接消费。

---

# 106. Analysis Stages

至少：

```text Loading
Decoding
Tempo
Key
Loudness
Spectral
MFCC
Embedding
Persisting
Completed
```

如果采用共享流水线：

实际 stage 可以：

合并。

---

# 107. Stage Determinism

stage：

不得：

依赖：

wall-clock：

决定分析结果。

时间：

只能：

> diagnostics。

---

# 108. Virtual Clock

测试：

应：

使用：

> VirtualClock / injected clock

处理：

- elapsed time；
- retry；
- scheduling timestamps。

不要：

让：

分析结果：

依赖：

real wall-clock。

---

# 109. Analysis Retry

可重试：

- transient filesystem read；
- temporary worker failure。

不得：

无限 retry。

---

# 110. Retry Semantics

如果：

音频损坏：

不是：

> retry 100 次。

应：

直接：

`Failed` / `Unsupported`。

---

# 111. Error Model

建立：

```text
AnalysisError
```

至少：

```text
Decode
UnsupportedFormat
InvalidAudio
InsufficientData
FeatureExtraction
Embedding
ModelUnavailable
ModelInvalid
Database
Cancelled
ResourceLimit
Internal
```

---

# 112. Error Persistence

数据库：

可以：

保存：

简短：

```text error_code
```

可选：

```text error_detail
```

不得：

保存：

- stack trace；
- secrets；
- full filesystem internals；

作为普通用户数据。

---

# 113. Analysis Status Event

后端：

可以：

发：

```text analysis_status_changed
analysis_progress
```

M6：

后续消费。

频率：

必须：

> 有界。

禁止：

每个 audio frame：

发送 IPC。

---

# 114. Frontend Boundary

M8：

不要：

实现完整 analysis dashboard UI。

允许：

提供：

typed DTO：

例如：

```text TrackAnalysisSummary
AnalysisStatus
AnalysisProgress
```

M6：

后续实际展示。

---

# 115. No M8 UI Explosion

不要：

新增：

- analysis explorer；
- BPM laboratory；
- waveform editor；
- spectral analyzer UI；

作为 M8 主功能。

本阶段：

> backend / domain / persistence first。

---

# 116. CLI / Diagnostic Tool

允许：

建立：

```text analysis-inspect
```

或：

等价 CLI / test command。

用于：

查看：

```text Track
BPM
Key
LUFS
MFCC
Embedding
Version
Status
```

这是：

> diagnostics，不是正式 UI。

---

# 117. Analysis Output Contract

最终：

建立：

```text TrackAnalysisSummary
```

至少：

```text analysis_version
content_identity
status

bpm
bpm_confidence

key
key_confidence
camelot_key

integrated_lufs
loudness_range
true_peak

energy
brightness
spectral_centroid
spectral_rolloff

mfcc_summary

embedding_metadata
```

具体字段：

根据实际 schema 调整。

---

# 118. Units

所有 numeric feature：

必须：

明确单位。

例如：

```text bpm           → BPM
lufs             → LUFS
true_peak        → dBTP
lra              → LU
frequency        → Hz
duration         → frames / seconds
energy           → [0,1]
brightness       → documented normalized range
```

不要：

出现：

```text peak = -1.4
```

却：

不说明单位。

---

# 119. Nullable Values

数据库：

正确使用：

`NULL`

表达：

> unavailable。

不要：

```text bpm = 0
```

表示：

> 未分析。

---

# 120. Zero Semantics

特别定义：

```text bpm = 0
```

必须：

> 不允许作为合法 BPM。

没有 BPM：

使用：

`NULL / unavailable`

---

# 121. Analysis Query API

建立：

```text get_analysis(track_id)
get_analysis_batch(track_ids)
get_analysis_status(track_id)
request_analysis(track_id)
cancel_analysis(job_id)
```

具体命名：

根据当前 application architecture 调整。

---

# 122. Batch Query

M9：

可能：

一次需要：

> 数千首歌曲 embedding。

因此：

必须：

支持：

> batch query。

不要：

让 M9：

一首一首：

IPC round-trip。

---

# 123. Batch Analysis Read

支持：

```text
get_analysis_batch(...)
```

需要：

考虑：

- pagination；
- bounded batch size；
- memory limits。

不要：

一次：

加载百万向量。

---

# 124. Embedding Batch

Embedding provider：

可以：

支持 batch inference。

但：

必须：

有：

> bounded batch size。

不能：

一次：

将整个音乐库全部放进内存。

---

# 125. Embedding Similarity Primitive

M8 可以提供：

一个：

> 纯数学 similarity primitive

例如：

```text cosine_similarity(a,b)
```

但：

不实现：

> Music Map graph。

这属于：

> M9 可复用基础能力。

---

# 126. Similarity Safety

向量：

如果：

包含：

NaN / Inf：

similarity：

必须：

返回：

> Invalid / error

而不是：

产生：

NaN。

---

# 127. Cosine Similarity Tests

至少：

```text same vector → 1
orthogonal → ~0
opposite → -1
zero vector → invalid
invalid vector → invalid
```

具体：

根据 normalized embedding policy 调整。

---

# 128. Embedding Dimension Safety

比较：

两个 vector：

必须：

```text dimension(A) == dimension(B)
```

否则：

返回：

`DimensionMismatch`

不能：

静默 truncate。

---

# 129. Embedding Version Compatibility

不同：

```text model_id
model_version
analysis_version
```

不得：

直接混合计算。

M9：

必须：

能够知道：

> 两个 vector 是否可比较。

---

# 130. Multiple Embedding Models

[B]

未来：

可以：

支持：

```text model A
model B
```

当前：

至少：

一个。

架构：

应该：

通过：

provider abstraction：

可扩展。

---

# 131. Model Lifecycle

embedding model：

生命周期：

```text Load
Validate
Warmup
Infer
Unload
```

必须：

避免：

每首歌曲：

重复加载模型。

---

# 132. Model Memory

模型：

可能：

占用大量 RAM。

必须：

记录：

> actual memory cost。

并：

只维护：

有限：

> active model instances。

---

# 133. Model Warmup

可以：

首次分析：

进行：

一次：

> warmup。

但：

不要：

人为：

制造：

“加载中” fake delay。

状态：

必须：

来自：

真实 model initialization。

---

# 134. Model Failure

模型加载失败：

不应该：

导致：

整个 library 无法使用。

结果：

```text embedding = Unavailable
```

但：

BPM / Key / LUFS：

仍可以：

> 独立完成。

---

# 135. Embedding Independence

Embedding：

必须：

与其它 feature：

> failure-isolated。

不能：

```text embedding failed
→ entire analysis failed
```

除非：

产品实际将 embedding 定义为：

> mandatory。

默认：

不建议。

---

# 136. Analysis Feature Isolation

同理：

```text BPM failed
```

不应该：

阻塞：

```text LUFS
MFCC
Embedding
```

除非：

基础 audio decode：

本身失败。

---

# 137. Dependency Graph

推荐：

```text
Decoded Audio
    ↓
Shared Analysis Stream
    ├─ Tempo
    ├─ Key
    ├─ Loudness
    ├─ Spectral
    ├─ MFCC
    └─ Embedding preprocessing
```

---

# 138. No Circular Dependency

不要：

```text aura-analysis
 ↔
aura-library
```

形成：

循环依赖。

建议：

```text aura-library
      ↑
application layer
      ↑
aura-analysis
```

或者：

通过：

shared domain types。

实际决定：

记录。

---

# 139. Analysis Persistence Ownership

`aura-analysis`：

可以：

拥有：

> analysis repository abstraction。

但：

不要：

让：

feature extraction：

直接知道：

具体 SQL。

推荐：

```text Analyzer
 ↓
AnalysisRepository
 ↓
SQLite implementation
```

---

# 140. Repository Interface

建立：

```text save_result()
load_result()
mark_stale()
list_pending()
```

或：

等价接口。

这样：

unit tests：

可以：

不用真实 SQLite。

---

# 141. In-Memory Repository

测试：

必须：

提供：

> in-memory analysis repository

用于：

验证：

scheduler / cancellation / status。

---

# 142. SQLite Repository

集成测试：

再使用：

> temporary SQLite database。

确认：

实际：

- migration；
- WAL；
- transaction；
- reopen。

---

# 143. Migration

M8 必须：

正式增加：

analysis schema migration。

不能：

手工删除旧数据库。

---

# 144. Migration Safety

必须测试：

```text fresh DB
old M3 DB
M8 upgrade
reopen
```

以及：

```text migration failure
rollback
retry
```

---

# 145. Legacy Placeholder Migration

如果 M3 已经：

存在：

`track_analysis`

placeholder：

必须：

先审计：

真实字段。

不要：

直接：

drop/recreate

破坏现有数据。

---

# 146. Analysis Schema Compatibility

未来：

M9：

需要读取：

analysis。

所以：

analysis schema：

必须：

> backward-compatible where feasible。

新增字段：

使用：

nullable/default。

---

# 147. Analysis Version Migration

schema migration：

和：

algorithm version：

必须：

区分。

例如：

```text DB schema v3
analysis algorithm v2
```

这是两个不同概念。

---

# 148. Stale Analysis

算法升级：

不要：

立刻：

删除旧结果。

可以：

标记：

`Stale`

然后：

background reanalysis。

这样：

用户：

仍可以看到：

> 上一版本结果

如果产品 UI：

明确标识版本。

---

# 149. Reanalysis

必须支持：

```text reanalyze(track)
reanalyze(stale)
reanalyze(all)
```

但：

本阶段：

至少：

> 单 Track + stale batch

能力。

---

# 150. Reanalysis Scheduling

不要：

算法升级：

启动：

立刻：

分析用户所有 100,000 首歌曲。

必须：

> 用户/设置可控。

默认：

可以：

> background low priority。

---

# 151. Newly Imported Track

M3：

扫描新 Track 后：

M8：

应该能够：

> 标记 analysis pending。

最终：

由：

application layer：

将：

```text Track imported
↓
AnalysisScheduler.enqueue
```

接起来。

---

# 152. Do Not Put Analyzer in Scanner

M3 scanner：

不要：

直接：

执行：

> BPM / Key / Embedding。

保持：

```text Library Import
↓
Analysis Queue
```

解耦。

---

# 153. Background Analysis

默认：

可以：

> 后台分析。

但：

播放时：

降低：

concurrency。

---

# 154. User-Initiated Analysis

用户：

明确请求：

“Analyze now”

则：

允许：

提升优先级。

仍然：

不得：

阻塞：

playback。

---

# 155. Analysis Cancellation on Shutdown

应用退出：

必须：

- stop accepting jobs；
- cancel active jobs；
- flush committed analysis；
- don't start new jobs；
- close model safely；
- close DB cleanly。

---

# 156. Analysis Resume

重启后：

如果：

上次：

```text Running
```

但：

实际没有：

Completed：

必须：

转为：

`Pending` / `Cancelled`

重新处理。

不能：

一直：

`Running`。

---

# 157. Crash Recovery

模拟：

```text decode
↓
feature extraction
↓
crash
↓
restart
```

DB：

不能：

显示：

> Completed

除非：

transaction 已成功提交。

---

# 158. Corrupted Result

如果：

DB 中 embedding：

坏：

必须：

> 标记 invalid / stale。

不能：

M9：

读取：

损坏向量继续计算。

---

# 159. Analysis Checksum

[B]

可以：

对：

analysis payload：

增加：

checksum / hash

检测：

意外 corruption。

---

# 160. Source Changes

如果：

M4 Source：

同一个 URL：

返回：

不同内容。

M8：

如果：

该内容对应：

新的 content identity：

必须：

重新分析。

不要：

按 URL：

复用旧分析。

---

# 161. Cache Boundary

M5：

负责：

> audio byte persistence。

M8：

负责：

> analysis result persistence。

不要：

在 M8：

重新建立：

> audio cache。

分析输入：

若需要：

从：

M3 local file

或：

M5 已缓存合法内容

读取。

---

# 162. No Analysis Cache Duplication

不要：

在：

```text %APPDATA%\analysis-cache
```

再偷偷保存一份：

全套 audio。

可以存在：

轻量 intermediate artifact，

但必须：

- 必要；
- bounded；
- atomic；
- 可清理；
- 不替代 M5。

优先：

> streaming analysis，避免 intermediate audio files。

---

# 163. Temporary Data

如需要：

temporary analysis files：

必须：

位于：

> application temp directory。

不得：

写入：

用户音乐目录。

---

# 164. Security — File Input

analysis：

只能访问：

> application layer 已批准的音频路径 / source。

不得：

接受：

任意 frontend path：

然后：

直接：

`File::open()`。

---

# 165. Path Validation

任何 analysis file path：

必须：

经过：

M3 / application layer：

合法化。

防止：

- `..`;
- reparse point；
- path escape；
- UNC surprises。

---

# 166. Resource Limits

必须设置：

- max analysis duration；
- max decoded frames；
- max PCM memory；
- max MFCC frames retained；
- max embedding batch size；
- max concurrent jobs；
- max DB payload。

---

# 167. Extremely Long Track

例如：

12 小时录音。

不得：

因为：

MFCC / beat grid：

直接：

分配：

巨型内存。

必须：

> chunked processing + bounded representation。

---

# 168. Extremely Short Track

例如：

100ms。

算法：

必须：

能够：

> graceful unavailable。

不得：

panic。

---

# 169. Silence

必须测试：

> 完全 silence。

预期：

- BPM unavailable / low confidence；
- Key unavailable / low confidence；
- LUFS handled according to loudness semantics；
- MFCC finite；
- embedding finite 或明确 unsupported。

---

# 170. Noise

随机噪声：

必须：

不会：

- panic；
- NaN；
- impossible BPM；
- invalid key；
- invalid vector。

算法：

可以：

> low confidence。

---

# 171. Speech

语音：

不能：

因为：

算法输出了一个 BPM：

就认为：

> 这是歌曲节奏。

允许：

> 低 confidence。

---

# 172. Mono

测试：

mono：

必须：

正常。

---

# 173. Stereo

测试：

stereo：

必须：

正常。

---

# 174. Phase

测试：

左右相位：

特殊情况：

不得：

导致：

分析：

NaN / infinite。

---

# 175. Sample Rate Matrix

至少测试：

```text
22050
44100
48000
96000
```

如果：

M1 实际支持：

更广范围：

可扩展。

---

# 176. Bit Depth Matrix

如果 decoder：

支持：

- 16-bit；
- 24-bit；
- 32-bit；

必须：

统一成：

> analysis PCM representation

并测试。

---

# 177. Channel Matrix

至少：

```text
mono
stereo
```

如果 M1 支持更多：

> 明确 analysis downmix policy。

---

# 178. Numeric Safety

所有 feature：

必须检查：

```text is_finite()
```

向量：

也必须：

全部：

finite。

---

# 179. Clamp Policy

需要范围：

必须：

明确：

> clamp / reject

不能：

每个 feature：

偷偷：

不同处理。

---

# 180. Floating Point Tests

不要：

要求：

所有分析结果：

bit-identical

如果：

算法天然：

浮点敏感。

应：

使用：

> deterministic tolerance。

---

# 181. Deterministic Hash

可以：

建立：

```text analysis_result_hash
```

对：

canonicalized result：

计算 hash。

用于：

回归测试。

---

# 182. Canonical Serialization

必须：

保证：

analysis result serialization：

字段顺序 / binary layout：

稳定。

JSON：

如用于 diagnostics：

不能作为：

唯一 canonical storage。

---

# 183. Analysis Determinism

以下固定：

```text input audio
analysis version
algorithm versions
model version
preprocessing
settings
```

重复运行：

必须：

得到：

> same logical result

或：

在：

> documented floating-point tolerance

范围内一致。

---

# 184. BPM Deterministic Test

相同：

synthetic click track

运行：

10 次。

结果：

必须：

> 一致。

---

# 185. Key Deterministic Test

相同：

synthetic chord progression

运行：

10 次。

结果：

必须：

> same canonical key / confidence tolerance。

---

# 186. LUFS Deterministic Test

相同：

fixture：

运行：

多次：

必须：

> same result within tolerance。

---

# 187. MFCC Deterministic Test

必须：

比较：

- coefficient count；
- means；
- stddev；
- min；
- max。

---

# 188. Embedding Deterministic Test

同：

audio + model：

必须：

> same dimension。

向量：

数值：

应：

> 在明确 tolerance 内一致。

---

# 189. Embedding Similarity Test

相同：

音频：

embedding：

应：

高度一致。

不同：

明显不同的 synthetic fixture：

应：

能够：

产生：

> 可观察差异。

但：

不要把：

“不同一定低 similarity”

作为绝对算法保证。

---

# 190. Feature Correlation Test

不需要：

做机器学习评估。

但：

必须：

做：

基本 sanity：

例如：

```text louder sine
→ RMS higher

higher-frequency tone
→ centroid higher

silent
→ energy near zero
```

---

# 191. BPM Synthetic Validation

至少：

验证：

```text 60 BPM
90 BPM
120 BPM
128 BPM
140 BPM
180 BPM
```

允许：

合理 tolerance。

---

# 192. Key Synthetic Validation

至少：

生成：

```text C major
A minor
G major
E minor
D major
```

测试：

canonical key。

---

# 193. LUFS Synthetic Validation

至少：

具有：

明确 reference level：

并：

验证：

scale relation。

---

# 194. Spectral Validation

至少：

```text 100Hz tone
1kHz tone
8kHz tone
```

验证：

centroid：

符合：

> expected ordering。

---

# 195. MFCC Validation

至少：

测试：

不同频谱：

MFCC：

产生：

> finite + deterministic + distinguishable features。

不要：

要求：

手工精确值，

除非：

数学 fixture：

足够简单。

---

# 196. Embedding Validation

必须：

实际：

运行：

真实 model。

不能：

只 mock provider：

然后：

宣布：

embedding 完成。

Mock：

只用于：

scheduler / integration tests。

---

# 197. Model Smoke Test

至少：

```text load model
↓
warmup
↓
analyze tiny fixture
↓
validate dimension
↓
validate finite
↓
shutdown
```

必须：

在：

CI 或：

明确可运行的开发环境：

真实执行。

---

# 198. Model License Audit

最终必须提供：

```text model name
model version
runtime
license
weight license
redistribution status
commercial status
source
```

写入：

`THIRD_PARTY_LICENSES.md`

---

# 199. Model Reproducibility

记录：

- model hash；
- model version；
- preprocessing version；
- runtime version。

如果模型文件：

变化：

必须：

> analysis invalidation。

---

# 200. Model File Integrity

如果：

模型：

本地文件：

启动加载：

必须：

验证：

> expected checksum

或：

等价 integrity mechanism。

---

# 201. Model Path Policy

模型：

只能：

来自：

> approved application data / bundled resource location。

禁止：

frontend：

提供：

任意模型路径：

然后：

Rust：

加载任意文件。

---

# 202. Model Loading Security

不得：

加载：

- `.exe`;
- arbitrary DLL；
- arbitrary script；

作为：

“模型”。

仅允许：

> 明确规定格式。

---

# 203. ONNX / Runtime Safety

如果：

使用：

ONNX：

必须：

避免：

模型：

通过：

动态外部：

资源：

执行任意代码。

模型加载：

> local + validated。

---

# 204. No Runtime Model Download

再次强调：

默认：

完全断网：

M8：

必须：

能够：

运行：

> 除非用户没有安装已批准的 embedding model。

这种情况下：

embedding：

标记：

`ModelUnavailable`

而其他分析：

仍然可运行。

---

