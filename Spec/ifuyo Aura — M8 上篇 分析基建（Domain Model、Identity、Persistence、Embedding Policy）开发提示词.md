# ifuyo Aura
# M8 — Audio Analysis / BPM / Key / LUFS / MFCC / Embedding
> 本文档为原 M8 开发提示词的**上篇（分析基建，§0–49）**。
> 中篇《ifuyo Aura — M8 中篇 特征分析引擎（BPM、Key、LUFS、Spectral、MFCC）开发提示词.md》承载 §50–204；下篇《ifuyo Aura — M8 下篇 运行时、集成与发布质量（Queue、Soak、Integration、Gates）开发提示词.md》承载 §205–390。
> 章节编号沿用原 M8 全局编号；跨篇引用（如“见 #132”）按编号在三篇中查找。


你现在进入 ifuyo Aura 项目的 **M8：音频分析与音乐特征系统**。

项目正式名称：

`ifuyo Aura`

仓库 / 项目名称：

`ifuyo-aura`

品牌：

`ifuyo · 伊芙游`

品牌口号：

`以光与梦，编织每一次游玩。`

---

# 0. 当前阶段目标

在已经完成并通过审计的：

```text
Phase 0
M1 Audio Core
M2 Playback Quality
M3 Music Library
M4 Source System
M5 Persistent Cache
M6 Desktop UI / Now Playing / Library / Aurora
M7 Dreamscape
```

基础上，正式实现：

> **本地、确定性、可取消、可增量、可版本化的音乐分析系统。**

M8 是：

> **ifuyo Aura 的 Music Intelligence / Audio Feature Layer**

但：

不是：

- AI 推荐系统；
- 在线音乐数据库；
- 云端音乐分析；
- 社交系统；
- Music Map；
- Dreamscape；
- DSP playback engine。

本阶段真正负责：

```text
Audio / Track
    ↓
Analysis Pipeline
    ↓
BPM / Beat
Key / Chroma
LUFS / Loudness
Spectral Features
MFCC
Embedding
    ↓
Versioned Analysis Result
    ↓
M3 Library Database
    ↓
M6 / M7 / M9 Consumers
```

---

# 1. 本阶段核心原则

M8 必须满足：

```text
Local-first
Deterministic
Versioned
Incremental
Cancellable
Restartable
Privacy-preserving
Commercial-safe
No fake intelligence
No network dependency
```

尤其：

> **所有分析默认在本机完成。**

不得把音乐数据上传到：

- cloud API；
- AI API；
- analytics service；
- embedding API；
- third-party music analysis service。

---

# 2. 本阶段绝对边界

## A 级必须实现

1. `aura-analysis` crate。
2. analysis domain model。
3. analysis version。
4. algorithm version。
5. content identity binding。
6. analysis status model。
7. analysis job model。
8. cancellable analysis worker。
9. deterministic audio feature extraction。
10. BPM / tempo estimation。
11. beat / beat-grid 基础结果。
12. BPM confidence。
13. Key detection。
14. chroma / pitch-class representation。
15. key confidence。
16. Camelot-compatible key representation（如果最终算法可靠）。
17. integrated LUFS。
18. loudness range。
19. true peak。
20. loudness confidence / validity semantics。
21. RMS / peak 等基础能量特征。
22. spectral centroid。
23. spectral rolloff。
24. brightness / spectral balance 等稳定派生特征。
25. MFCC。
26. MFCC summary。
27. local embedding provider abstraction。
28. 至少一个真实、可运行、合法的本地 embedding provider。
29. embedding version。
30. embedding dimension contract。
31. deterministic embedding inference。
32. analysis result persistence。
33. analysis DB migrations。
34. analysis result invalidation。
35. incremental re-analysis。
36. content replacement detection。
37. analysis queue。
38. progress reporting。
39. cancellation。
40. failure recovery。
41. malformed / unsupported audio handling。
42. bounded CPU / memory usage。
43. deterministic fixtures。
44. numerical safety。
45. M3 integration。
46. M6 integration contract。
47. M7 integration contract。
48. M9 compatibility contract。
49. 完整自动化测试。
50. performance benchmark。
51. M1–M7 regression。

---

## B 级

A 级全部稳定后，可以实现：

1. 更高精度 beat tracking。
2. bar / downbeat estimation。
3. richer tempo confidence。
4. multiple-key hypothesis。
5. key modulation detection。
6. section segmentation。
7. intro / verse / chorus / outro 等结构候选。
8. danceability-like derived feature。
9. acousticness-like derived feature。
10. instrumentalness-like derived feature。
11. vocal activity estimate。
12. richer embedding metadata。
13. embedding quantization / compression。
14. analysis diagnostics UI contract。
15. batch analysis scheduler。
16. analysis priority classes。
17. analysis CPU budget profiles。
18. background-only analysis mode。
19. pause-on-battery policy。
20. analysis history。

B 级不得破坏 A 级。

---

# 3. 本阶段明确禁止

不要实现：

- Music Map；
- similarity graph；
- graph layout；
- cluster visualization；
- recommendation engine；
- playlist recommendation；
- autoplay recommendation；
- social recommendation；
- Dreamscape world generation；
- Dreamscape mutation；
- new Aurora visual effects；
- SMTC；
- tray；
- installer；
- updater；
- Steam；
- cloud sync；
- account system；
- telemetry；
- remote analysis；
- online BPM API；
- online key API；
- online loudness API；
- cloud embedding；
- online lyrics；
- online metadata enrichment；
- online artwork；
- commercial-platform extraction；
- DRM extraction；
- protected stream analysis；
- persistent audio cache redesign；
- new Source provider；
- new codec。

特别强调：

> **M8 是分析，不是“智能产品层”。**

M8 输出：

> 数据。

M9 才决定：

> 如何把这些数据组织成 Music Map。

---

# 4. 不允许 Fake Analysis

这是本阶段最高优先级约束之一。

禁止：

```text
BPM = random()
Key = random()
Energy = random()
Embedding = random_vector()
```

也禁止：

```text
BPM = duration-based guess
Key = hash(track_id)
Embedding = deterministic noise
```

任何展示为：

> 真正音乐分析结果

的数据：

必须：

> 来自真实算法。

没有结果：

必须：

```text
Not analyzed
Unavailable
Unsupported
Failed
```

不得伪造。

---

# 5. M8 与 M2 Loudness 的关系

M2 已经支持：

```text
TrackLoudness
```

用于：

> 播放时响度归一化。

M8：

负责：

> 真正计算这些 loudness metadata。

因此：

```text
M8 Analysis
   ↓
TrackLoudness
   ↓
M2 Playback
```

必须建立：

> 干净的 DTO / domain contract。

不要让 M2 依赖：

`aura-analysis`。

推荐：

```text
aura-analysis
    ↓
shared/domain DTO
    ↓
application layer
    ↓
aura-core / M2
```

保持依赖方向。

---

# 6. M8 与 M3 的关系

M3 已经建立：

> Library database + analysis placeholder。

M8：

应该：

> 正式接管 analysis persistence。

不要重新建立：

> 第二套音乐库数据库。

原则：

```text
M3 Library DB
 ├─ tracks
 ├─ playback_stats
 ├─ artworks
 └─ track_analysis
```

M8：

> 负责分析数据的生成与维护。

M3：

> 负责 Track identity 与 Library truth。

---

# 7. M8 与 M7 Dreamscape 的关系

M7 已经预留：

> future analysis injection。

M8：

可以提供真实：

```text
bpm
energy
brightness
loudness
```

供 Dreamscape：

作为：

> optional visual influence。

但：

M8 不负责重新设计 Dreamscape。

M7 renderer：

必须：

在：

```text analysis absent
```

情况下：

> 正常运行。

---

# 8. M8 与 M9 的关系

M9 将使用：

```text
embedding
BPM
Key
energy
brightness
loudness
```

建立：

> Music Map。

因此 M8 必须提供：

> 稳定、版本化、可查询的 analysis contract。

但：

M8 不实现：

- graph；
- spatial layout；
- similarity UI；
- cluster generation；
- Map rendering。

---

# 9. 开始前 Baseline Audit

任何代码修改之前：

必须先审计真实仓库。

执行：

```powershell
git status
git branch
git log --oneline -20
cargo metadata
cargo tree
cargo test --workspace
cargo deny check
```

前端：

```powershell
npm run typecheck
npm run lint
npm test
```

检查：

```text
crates/aura-core
crates/aura-testkit
crates/aura-library
crates/aura-sources
crates/aura-cache
crates/aura-analysis
crates/aura-synth
src-tauri
ui
docs
config
```

确认：

- M1；
- M2；
- M3；
- M4；
- M5；
- M6；
- M7；
- 当前 HEAD；
- 当前 Rust version；
- current MSRV；
- SQLite schema；
- `track_analysis` 当前状态；
- existing analysis placeholders；
- existing feature extraction code；
- existing math / DSP dependencies；
- current Three.js integration；
- M7 analysis injection boundary；
- CI；
- license policy；
- current dependency lockfile。

输出：

# M8 Baseline Audit

A. Git  
B. M1 Status  
C. M2 Status  
D. M3 Status  
E. M4 Status  
F. M5 Status  
G. M6 Status  
H. M7 Status  
I. Existing Analysis Code  
J. Database Schema  
K. Dependencies  
L. Performance Infrastructure  
M. Security / Licensing  
N. Risks

---

# 10. 前序问题处理

如果 M1–M7：

存在真正阻塞 M8 的缺陷：

只做：

> 最小必要修复。

不得借 M8：

- 重写 M1；
- 重写 M2；
- 重写 M3；
- 重写 M4；
- 重写 M5；
- 重写 M6；
- 重写 M7。

所有必要修复：

必须记录：

`DECISIONS.md`

与：

`PROGRESS.md`

---

# 11. `aura-analysis` 职责

`aura-analysis`：

负责：

```text
Track / Audio Bytes
       ↓
Decode
       ↓
Analysis Preprocessing
       ↓
Feature Extraction
       ↓
Analysis Result
       ↓
Persistence
```

它不得负责：

- audio output；
- WASAPI；
- UI rendering；
- Three.js；
- Tauri window；
- network fetch；
- cloud API；
- source discovery；
- cache persistence。

---

# 12. 不要复制 Decoder

M8：

优先复用：

M1 已有：

> decoder abstraction。

如果分析需要：

seek / decode：

应：

复用现有 decoder capability。

不要：

建立：

> 第二套 codec parser。

如果现有 decoder 不适合离线批量分析：

建立：

> analysis-side adapter

而不是复制整套 decoder。

---

# 13. Analysis Domain Model

建立至少：

```text
TrackAnalysis
AnalysisId
AnalysisVersion
AnalysisStatus
AnalysisJob
AnalysisProgress
AnalysisError
TempoAnalysis
KeyAnalysis
LoudnessAnalysis
SpectralFeatures
MfccFeatures
Embedding
```

命名可以根据真实代码调整。

---

# 14. Analysis Status

至少：

```text
Pending
Running
Completed
Failed
Cancelled
Unsupported
Stale
```

不要使用：

十几个互相冲突的 bool。

---

# 15. Analysis Identity

一份分析结果：

不能只依赖：

`TrackId`

必须同时考虑：

```text
TrackId
+
content_identity
+
analysis_version
```

如果 embedding 使用模型：

还应考虑：

```text
embedding_model_id
embedding_model_version
```

---

# 16. Analysis Version

必须建立明确：

```text
analysis_version
```

例如：

```text
analysis_version = 1
```

它表示：

> 整体分析 contract / 算法结果版本。

不能只依赖 Git commit。

---

# 17. Algorithm Version

复杂分析：

应进一步区分：

```text
analysis_version
tempo_algorithm_version
key_algorithm_version
loudness_algorithm_version
mfcc_algorithm_version
embedding_model_version
```

如果不需要每个都独立：

至少：

> 能够定位结果由什么算法 / 模型产生。

---

# 18. Why Versioning Matters

以后：

BPM 算法改变。

同一 Track：

可能：

从：

```text 122 BPM
```

变成：

```text 123 BPM
```

这是合法的。

因此：

数据库必须能够：

> 判断现有结果是否过期。

不能把旧值当永久真相。

---

# 19. Content Identity Binding

分析：

应该绑定：

M3 的：

> content identity。

文件：

重命名：

```text
old.mp3
→
new.mp3
```

如果音频内容不变：

分析结果：

> 继续有效。

---

# 20. Content Replacement

如果：

用户：

用另一首音频：

覆盖同一路径：

必须：

检测：

content identity 改变。

旧 analysis：

必须：

标记：

`Stale`

或删除。

不得：

将旧 BPM：

继续关联新音频。

---

# 21. Metadata-Only Change

如果：

只修改：

- title；
- artist；
- album；
- genre；

音频 PCM：

没有改变。

则：

分析结果：

> 不需要重算。

---

# 22. File Move

文件移动：

analysis：

必须：

保持。

因为：

Track identity / content identity：

保持。

---

# 23. Analysis Key

建议：

使用：

```text
analysis_key =
hash(
content_identity
+
analysis_version
+
algorithm/model versions
)
```

或者等价机制。

必须：

deterministic。

具体方案：

记录到：

`DECISIONS.md`

---

# 24. Analysis Persistence

分析结果：

必须：

持久保存。

默认：

> 使用 M3 Library SQLite。

不要：

将每次分析结果：

仅保存在内存。

---

# 25. SQLite Policy

继续使用：

M3：

- SQLite；
- WAL；
- migrations；
- transactions。

M8：

不要：

再创建第二个数据库。

---

# 26. Analysis Schema

至少需要支持：

```text
track_analysis
```

可以拆分：

```text
track_analysis
track_tempo
track_key
track_loudness
track_spectral
track_mfcc
track_embedding
```

或者：

合理的单表 + BLOB schema。

不要：

为了“看起来专业”建立几十张表。

---

# 27. Scalar vs Blob

标量：

例如：

```text
bpm
confidence
lufs
lra
true_peak
energy
brightness
```

适合：

> 独立 numeric columns。

高维：

```text
MFCC
Embedding
```

可以：

> BLOB / compact serialized representation。

但必须：

记录：

- dtype；
- dimension；
- byte order；
- normalization；
- version。

---

# 28. Embedding Storage

Embedding：

不能：

直接：

```text JSON array of strings
```

进行长期存储。

优先：

> Float32 / Float16 binary representation

或等价紧凑格式。

必须：

有：

```text dimension
dtype
model_id
model_version
normalization
```

---

# 29. Embedding Integrity

数据库读取：

必须验证：

```text expected_bytes
==
dimension × dtype_size
```

如果不一致：

> Corrupt / invalid。

不得：

越界读取。

---

# 30. Analysis Database Transaction

一首歌：

完整分析：

推荐：

```text
analysis run
↓
validate all features
↓
single transaction
↓
commit
```

避免：

只保存了：

BPM：

却没有 Key / embedding：

然后：

UI：

误以为整条分析完成。

---

# 31. Atomic Analysis Result

分析结果：

应该：

具有：

> all-or-nothing semantics

至少：

对于一次正式 analysis version：

不能出现：

```text Completed
```

但：

embedding 缺失。

除非：

该 feature 明确：

`Unavailable`。

---

# 32. Partial Feature Availability

某文件：

可能：

- BPM 成功；
- Key 失败；
- Embedding unsupported。

允许：

> feature-level availability。

例如：

```text BPM = Available
Key = Failed
Embedding = Unsupported
```

但：

必须：

明确表示。

不要：

整个 Track：

简单标记：

`Failed`

然后丢失其它成功分析。

---

# 33. Feature Availability Model

推荐：

```text
Available
Unavailable
Unsupported
Failed
Stale
```

或等价模型。

---

# 34. Audio Decode

分析：

应该：

使用：

M1 decoder。

支持：

M1 已经支持的格式。

不要：

因为分析：

去偷渡新的 codec。

---

# 35. Offline Input

M8-A：

分析对象：

优先：

```text local library tracks
```

网络音频：

必须：

符合已有：

M4/M5 source/cache policy。

但：

不要：

为 M8 添加任何新的网络机制。

---

# 36. Internet Radio

Internet Radio：

默认：

> 不进行永久 Track Analysis。

原因：

它是：

- live；
- continuous；
- non-seekable。

如果应用层：

未来允许：

analysis session：

必须：

另行设计。

M8：

默认：

`Unsupported`

---

# 37. Direct HTTP

Direct HTTP：

只有：

> 有限、可重复、合法、可 seek 的 finite content

并且：

内容：

> 已存在于本地 / 合法缓存

时：

才可分析。

不要：

为了 M8：

自动下载整首网络歌曲。

---

# 38. No Hidden Network

M8：

运行时：

默认：

> 不产生网络请求。

分析：

不能：

偷偷：

- 查询 metadata；
- 下载 model；
- 调用 API；
- 上传音频。

---

# 39. Embedding Model Policy

这是 M8 最重要的商业许可证检查之一。

必须先确定：

> 一个真实、可运行、可商业发行、可本地执行的 embedding 模型。

必须检查：

- model license；
- code license；
- weight license；
- redistribution rights；
- commercial use；
- attribution；
- derivative restrictions；
- repository compatibility；
- installer redistribution；
- model update policy。

---

# 40. Embedding Model 不允许默认偷下

禁止：

程序第一次分析：

自动下载：

```text model.bin
weights.onnx
checkpoint
```

没有：

> 用户明确行为和产品设计：

不得联网下载模型。

---

# 41. Embedding Provider

建立：

```text
trait EmbeddingProvider
```

或等价 abstraction。

至少：

```text model_id()
model_version()
dimension()
analyze(input)
```

具体：

根据实际运行时决定。

---

# 42. Local Embedding Provider

必须至少有：

> 一个真实 provider。

它必须：

> 在完全断网环境运行。

如果模型许可证不符合：

不能使用。

如果最终无法找到满足商业发行条件的合法模型：

必须：

`BLOCKED`

而不是：

> 用随机 vector 冒充。

---

# 43. Embedding Provider Isolation

Embedding provider：

不得：

依赖：

- UI；
- Tauri；
- SQLite internals；
- network；
- Music Map。

它只负责：

```text PCM / Features
 ↓
Vector
```

---

# 44. Model Runtime

如果使用：

ONNX / Candle / Burn / tract / 等价本地推理方案：

必须：

先审计：

- license；
- binary size；
- CPU support；
- Windows；
- SIMD；
- memory；
- packaging；
- CI。

不要：

机械引入大型 ML framework。

---

# 45. Model Distribution

模型：

如果属于仓库：

需要：

> 明确是否允许 Git 仓库存储。

如果模型过大：

可以：

作为：

> release asset / installer asset

但：

必须：

> 离线可安装 / 不依赖 runtime cloud download。

最终策略：

记录：

`DECISIONS.md`

---

# 46. Embedding Determinism

固定：

```text audio input
model
model version
preprocessing
inference settings
```

必须：

产生：

> 可重复向量。

如果不同 CPU 指令集造成：

微小浮点差异：

必须：

定义：

> acceptable tolerance

而不是：

偷偷 rounding 成完全一样。

---

# 47. Embedding Normalization

必须明确：

向量是否：

```text L2 normalized
```

或者使用：

其它规范化方式。

M9：

必须能够：

明确知道：

> 相似度到底应该使用 cosine / dot / distance。

---

# 48. Embedding Contract

至少定义：

```text dimension
dtype
distance_metric
normalization
model_id
model_version
analysis_version
```

---

# 49. M9 Compatibility

M9：

不要：

依赖：

具体 ML runtime。

M9：

只应该消费：

```text EmbeddingVector
```

及其：

> similarity contract。

---

