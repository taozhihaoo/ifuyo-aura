# ifuyo Aura
# M8 下篇 — Runtime / Integration / Release Quality (§205–390)

> 本文档为原 M8 开发提示词的**下篇（运行时、集成与发布质量，§205–390）**。
> 上篇承载分析基建（§0–49）；中篇《ifuyo Aura — M8 中篇 特征分析引擎（BPM、Key、LUFS、Spectral、MFCC）开发提示词.md》承载特征分析引擎（§50–204）。
> 章节编号沿用原 M8 全局编号；跨篇引用按编号在三篇中查找。
> 阅读本文前，上篇与中篇所定义的实现必须已完成或在收尾。

# 205. Feature Independence

最终：

必须：

保证：

```text
BPM unavailable
```

不会导致：

```text LUFS unavailable
```

除非：

共享输入真的失败。

---

# 206. Analysis Batch API

建议：

```text analyze_tracks(track_ids)
```

必须：

- bounded；
- cancellable；
- progress-aware；
- no duplicate jobs。

---

# 207. Duplicate Job Deduplication

同一个：

```text analysis_key
```

如果：

已经：

`Running`

第二个请求：

不应：

启动第二份计算。

应该：

> join / coalesce。

---

# 208. Stale Job Protection

类似：

M1/M7：

generation：

Analysis：

也必须：

拥有：

```text job_id
run_id
revision
```

旧 job：

完成后：

不得：

覆盖：

更新版本结果。

---

# 209. Example

```text analysis v1 running
↓
algorithm updated to v2
↓
v2 starts
↓
v1 finishes late
```

必须：

> v1 不能覆盖 v2。

---

# 210. Analysis Revision

可以使用：

```text analysis_revision
```

或者：

`analysis_key`

作为：

比较依据。

---

# 211. Analysis Queue Persistence

[A]

分析 queue：

不要求：

完整持久任务队列。

至少：

> DB 状态可以从 Running 恢复到 Pending。

B：

可以：

持久化完整 batch queue。

---

# 212. Background Worker Shutdown

必须：

在：

application closing：

安全：

停止。

不得：

关闭窗口后：

worker：

继续：

分析：

数百首歌。

---

# 213. Worker Lifecycle

至少：

```text Start
Idle
Running
Stopping
Stopped
```

---

# 214. Worker Leak Test

循环：

```text start
stop
start
stop
```

几十次。

确认：

没有：

- orphan threads；
- runaway tasks；
- model duplicate loads；
- memory growth。

---

# 215. Analysis Soak

至少：

执行：

> 数小时 analysis soak。

例如：

连续分析：

100 / 500 / 1000 tracks。

观察：

- RAM；
- CPU；
- DB size；
- queue stability；
- failure count。

---

# 216. Long Track Soak

至少：

测试：

> 长音频 fixture。

确保：

内存：

不会：

随着：

duration：

无限增长。

---

# 217. Large Library Soak

使用：

> synthetic metadata + tiny audio fixtures

模拟：

```text 10k tracks
```

测试：

- query；
- enqueue；
- status；
- batch read；
- stale detection。

---

# 218. Analysis DB Size

记录：

每首：

analysis 平均大小。

特别：

关注：

embedding。

不要：

让：

一首歌：

analysis：

轻易达到：

数 MB。

---

# 219. Embedding Storage Budget

必须：

测量：

```text vector dimension
dtype
bytes per track
10k tracks total
```

例如：

不能：

只因为：

“Embedding 很酷”

就：

让：

10k tracks：

占用：

数 GB：

而没有明确设计。

---

# 220. MFCC Storage Budget

同样：

记录：

每 Track：

MFCC summary：

实际字节数。

---

# 221. SQLite Query Performance

测试：

```text get_analysis(single)
get_analysis_batch(100)
get_analysis_batch(1000)
filter by bpm
filter by key
filter by energy
```

如果：

M3 schema：

允许：

查询。

---

# 222. Indexes

为：

常用查询：

建立：

合理索引。

例如：

- bpm；
- key；
- analysis_version；
- status；

但：

不要：

所有列都建 index。

---

# 223. Search Integration

M3 FTS：

M8：

不要：

修改：

FTS tokenizer。

如果：

UI：

未来要：

```text bpm:120
key:8A
```

只需要：

> M3 filter AST

读取：

M8 analysis fields。

---

# 224. Filter Integration

最终：

应该：

允许：

application layer：

查询：

```text bpm:120-130
key:8A
energy:>0.6
```

但：

M8：

不要：

实现新的：

filter language。

复用：

M3 contract。

---

# 225. Query Semantics

如果：

analysis unavailable：

对：

```text bpm:120-130
```

结果：

必须：

> 不匹配。

不能：

当成：

0。

---

# 226. Key Query

key：

必须：

匹配：

canonical representation。

例如：

`A minor`

与：

`Am`

如果：

同时支持：

必须：

在 schema / query layer：

明确转换。

---

# 227. BPM Range Query

范围：

必须：

数值比较。

不要：

将：

BPM：

存成：

字符串：

`"128 BPM"`。

---

# 228. Energy Query

energy：

必须：

数值。

---

# 229. M8 → M7 Data Contract

建立：

例如：

```text AnalysisVisualSummary
```

只包含：

> Dreamscape 真正需要的最小数据。

例如：

```text bpm
energy
brightness
loudness
```

具体：

以 M7 现有扩展点为准。

---

# 230. No M7 Rewrite

不要：

因为 M8：

重写：

Dreamscape generator。

只：

注入：

真实 analysis。

---

# 231. M7 Null Analysis

必须测试：

```text no analysis
```

Dreamscape：

依然：

完整工作。

---

# 232. M7 Real Analysis

必须测试：

```text analysis exists
```

Dreamscape：

能够：

接收：

真实 summary。

如果 M7 当前没有：

实际消费代码：

可以：

只建立：

> clean contract

但：

不要：

假装：

视觉已经根据 BPM 改变。

---

# 233. M6 Analysis State

M6：

如果需要：

可以：

得到：

```text analyzed
not analyzed
analysis failed
```

但：

M8：

不实现：

大型 UI。

---

# 234. No Analysis Spam

后台：

连续分析：

不得：

每首：

都弹 toast。

UI：

未来：

可以：

显示 aggregate progress。

---

# 235. Privacy

M8：

必须：

维持：

> 本地分析。

默认：

不上传：

- raw audio；
- metadata；
- vectors；
- analysis results；
- diagnostics。

---

# 236. Telemetry

绝对不要：

添加：

- analysis telemetry；
- model usage telemetry；
- embedding telemetry；
- cloud error reporting。

---

# 237. Logs

日志：

可以记录：

```text track_id
analysis_key
feature
duration
status
error_code
```

不要：

记录：

- raw PCM；
- full embedding vector；
- full metadata；
- sensitive path；
- audio bytes。

---

# 238. Diagnostics

本地 diagnostics：

可以：

显示：

```text analysis count
queued
running
completed
failed
stale
average duration
embedding model
version
```

仅：

> 本地。

---

# 239. No Raw Feature Logging

不要：

每首歌：

把：

13 × N：

MFCC：

打印到日志。

也不要：

把 embedding：

全部：

dump 到普通日志。

---

# 240. Security — Input Validation

任何：

analysis command：

必须：

验证：

- TrackId；
- jobId；
- version；
- batch size；
- priority；
- cancellation token。

---

# 241. Batch Limits

前端：

不能：

发送：

```text 1,000,000 track_ids
```

导致：

巨大内存。

必须：

> bounded batch size。

---

# 242. Analysis Abuse Prevention

malicious / malformed library data：

不能：

通过：

- extremely long path；
- impossible duration；
- huge metadata；
- corrupted audio；

制造：

unbounded CPU / memory。

---

# 243. Parser / Decoder Failure

任何：

decoder：

失败：

必须：

转换为：

`AnalysisError::Decode`

不能：

panic。

---

# 244. Panic Boundary

analysis worker：

不得：

因为：

单首坏歌曲：

panic：

导致：

> 整个 analysis service 死亡。

每个 job：

必须：

有：

> failure isolation。

---

# 245. Worker Panic Recovery

如果：

底层 provider：

unexpected panic：

必须：

尽可能：

被：

> worker boundary 捕获。

是否能：

catch_unwind：

取决于实际 Rust safety / unwind strategy。

如果不适合：

至少：

保证：

native panic：

不会：

来自：

普通 malformed user input。

---

# 246. Embedding Model Failure Isolation

一个：

model inference：

失败：

不能：

导致：

其他 worker：

全部：

进入：

dead state。

---

# 247. Analysis DB Failure

数据库：

暂时 unavailable：

analysis：

应：

进入：

recoverable error。

不要：

删除：

现有旧 analysis。

---

# 248. Read-Only DB

数据库：

只读：

必须：

允许：

已有：

analysis：

读取。

新 analysis：

明确：

失败 / deferred。

---

# 249. Disk Full

analysis：

写 DB：

disk full：

不能：

破坏：

Library DB。

SQLite transaction：

必须：

回滚。

---

# 250. Database Isolation

M8：

不得：

破坏：

M3：

- tracks；
- playlists；
- playback stats；
- artwork。

analysis failure：

必须：

> isolate。

---

# 251. Migration Backup

schema migration：

如果：

需要：

应：

遵循：

M3 existing migration policy。

不要：

创建：

不受控制：

database copies。

---

# 252. Dependency Audit

新增：

DSP / FFT / ML dependency：

必须检查：

```text license
MSRV
Windows support
CI
maintenance
binary size
runtime size
CPU requirements
security
commercial redistribution
```

---

# 253. No GPL / AGPL / LGPL

继续遵守：

M0 commercial whitelist。

任何：

- GPL；
- AGPL；
- LGPL；

不得：

进入：

正式产品依赖。

---

# 254. Model License Is Separate

注意：

即使：

runtime：

license 合法：

model weights：

也可能：

不合法。

必须：

分别审计：

```text runtime
code
weights
pretrained model
```

---

# 255. No Hidden Model License

不能：

只看：

GitHub repo：

README：

就：

认为：

model license：

可商业使用。

必须：

确认：

> weights / checkpoint 的实际 license。

---

# 256. Sources Documentation

更新：

`SOURCES.md`

记录：

M8：

如果有：

- algorithm source；
- model source；
- runtime source；
- standard reference；

必须：

记录：

- name；
- source；
- version；
- license；
- purpose；
- verification date；
- commercial status；
- known limitations。

---

# 257. THIRD_PARTY_LICENSES

必须增加：

所有：

M8：

新增：

- crates；
- model runtime；
- model；
- test assets；

的真实许可证信息。

不得虚构。

---

# 258. DECISIONS.md

至少记录：

- analysis architecture；
- analysis schema；
- analysis versioning；
- content identity binding；
- sample rate；
- channel policy；
- window size；
- hop size；
- BPM algorithm；
- beat grid；
- BPM confidence；
- key algorithm；
- chroma；
- key canonicalization；
- Camelot；
- LUFS standard；
- true peak；
- spectral features；
- MFCC configuration；
- embedding model；
- embedding runtime；
- model license；
- vector normalization；
- vector dtype；
- vector dimension；
- similarity metric；
- worker model；
- concurrency；
- cancellation；
- retry；
- persistence；
- stale results；
- M7 integration；
- M9 contract。

---

# 259. ARCHITECTURE.md

加入：

```text
M8 Analysis Architecture
Analysis Pipeline
Feature Extraction
Scheduler
Worker Model
Persistence
Versioning
Embedding Provider
Model Lifecycle
Cancellation
Failure Isolation
M3 Integration
M6/M7 Integration
M9 Contract
Performance
```

---

# 260. PROGRESS.md

开始：

```text
M8-A = IN PROGRESS
M8-B = NOT STARTED
```

完成后真实记录：

- feature support；
- algorithms；
- versions；
- model；
- tests；
- database migration；
- benchmark；
- soak；
- regression；
- known limitations。

---

# 261. Test Audio Fixture Policy

所有：

analysis fixture：

必须：

- tiny；
- deterministic；
- legal；
- redistributable；
- license documented。

禁止：

提交：

完整商业音乐。

---

# 262. Synthetic Audio Generator

建立：

测试生成器：

例如：

```text
sine
chord
click track
silence
noise
impulse
```

用于：

BPM / Key / spectral / MFCC：

测试。

---

# 263. Deterministic Generator

synthetic fixture：

必须：

使用：

固定：

seed / parameters。

禁止：

随机 waveform：

没有固定 seed。

---

# 264. Test Fixture Metadata

每个：

fixture：

必须：

记录：

```text
name
sample_rate
channels
duration
expected_feature
tolerance
```

---

# 265. Test — Analysis Lifecycle

至少：

```text enqueue
→ pending
→ running
→ completed
```

以及：

```text enqueue
→ running
→ cancel
→ cancelled
```

---

# 266. Test — Restart

流程：

```text running
↓
shutdown
↓
reopen
```

状态：

不能：

永久：

`Running`。

---

# 267. Test — Duplicate Queue

两次：

```text request_analysis(track)
request_analysis(track)
```

必须：

> 不产生重复 active job。

---

# 268. Test — Stale Job

```text v1 running
v2 running
v1 finishes late
```

必须：

> v2 remains authoritative。

---

# 269. Test — Content Identity

```text file A
analysis
rename
```

结果：

> analysis remains valid。

---

# 270. Test — Content Replacement

```text file A
analysis
replace audio with B
```

必须：

> old analysis stale / invalid。

---

# 271. Test — Metadata Only Change

```text audio unchanged
metadata changed
```

analysis：

必须：

> remain valid。

---

# 272. Test — BPM

至少：

```text 60
90
120
128
140
180
```

---

# 273. Test — BPM Negative Cases

至少：

```text silence
noise
speech-like fixture
too-short
malformed
```

---

# 274. Test — Key

至少：

```text C major
A minor
G major
E minor
D major
```

---

# 275. Test — Key Negative Cases

至少：

```text silence
noise
single-tone
very-short
```

---

# 276. Test — Loudness

至少：

```text -12
-16
-20
-23 LUFS class fixtures
```

测试：

- tolerance；
- finite；
- true peak；
- LRA semantics。

---

# 277. Test — Spectral

至少：

```text 100Hz
1kHz
8kHz
white-ish noise
silence
```

---

# 278. Test — MFCC

至少：

```text sine
chord
noise
silence
```

确认：

- finite；
- dimension；
- deterministic。

---

# 279. Test — Embedding

至少：

```text load
warmup
single inference
batch inference
determinism
dimension
finite
similarity
failure
shutdown
```

---

# 280. Test — Persistence

至少：

```text fresh DB
migration
save
load
reopen
update
stale
corrupt
rollback
```

---

# 281. Test — Batch

至少：

```text 1 track
10 tracks
100 tracks
```

并：

确认：

不会：

> 一次加载无限 PCM。

---

# 282. Test — Concurrency

至少：

```text same track
different tracks
embedding + BPM
analysis + playback
analysis + library scan
analysis + UI query
```

---

# 283. Playback Concurrency Test

必须：

证明：

```text background analysis
+
continuous playback
```

不会：

明显：

增加：

- underrun；
- callback violation；
- audio corruption。

---

# 284. Audio Callback Regression

M8 完成后：

再次：

运行：

M1/M2：

```text zero allocation
no lock
no blocking
```

必须：

全部 PASS。

---

# 285. M3 Regression

必须：

重新验证：

- scan；
- TrackId；
- rename；
- move；
- metadata update；
- DB migration；
- FTS5；
- playback stats。

尤其：

> analysis migration 不得损坏 M3 data。

---

# 286. M4 Regression

分析：

不得：

改变：

- source resolution；
- HTTP；
- ICY；
- Radio Browser；
- security；
- cache policy。

---

# 287. M5 Regression

确认：

analysis：

不会：

绕过：

```text CachePolicy
```

尤其：

`Forbidden`

不能：

因为 M8：

偷偷：

把 radio：

写盘。

---

# 288. M6 Regression

确认：

- UI launch；
- Now Playing；
- Library；
- Search；
- Queue；
- Aurora；
- Mini Player；
- Command Palette。

analysis：

不能：

阻塞 UI。

---

# 289. M7 Regression

确认：

- Dreamscape launch；
- seed determinism；
- world persistence；
- renderer lifecycle；
- mutation；
- reduced motion；
- no analysis 时正常；
- 有 analysis 时 contract 正常。

---

# 290. M8 No M9 Regression

M8：

不能：

偷偷：

实现：

```text
similarity graph
```

即使：

已经：

有 embedding。

最多：

提供：

```text cosine_similarity()
```

这种：

> math primitive。

---

# 291. Analysis Visualization Boundary

如果开发时：

需要查看：

BPM / Key / spectrum：

可以：

用：

diagnostic tool。

不要：

将：

diagnostic：

变成：

正式 M8 UI。

---

# 292. No Waveform Editor

M8：

可以：

内部读取：

PCM。

但：

不要：

实现：

- waveform editor；
- beat editor；
- sample editor。

---

# 293. No Manual BPM Editing

M8：

不要：

实现：

完整：

> BPM editor。

未来：

UI 可以：

允许：

用户：

override analysis，

但：

属于：

独立产品功能。

---

# 294. User Overrides

如果：

以后：

需要：

用户修改：

BPM / Key：

不要：

污染：

analysis result。

预留：

```text calculated_value
user_override
```

或：

等价分离。

本阶段：

只建立：

最小 schema compatibility。

---

# 295. Analysis vs User Metadata

必须区分：

```text algorithm_result
user_override
```

不要：

用户：

手动：

改了：

BPM：

然后：

下一次 reanalysis：

失去：

原始分析结果。

---

# 296. Feature Confidence

每个：

可能不可靠的 feature：

应该：

允许：

confidence。

至少：

```text BPM
Key
```

其它：

可以：

按算法需要。

---

# 297. Confidence Semantics

必须：

记录：

```text 0.0 = no confidence
1.0 = highest confidence
```

或者：

等价：

明确定义。

不要：

不同算法：

0.8：

代表：

完全不同含义。

---

# 298. Threshold Policy

如果：

M6/M9：

未来：

需要：

filter：

例如：

`bpm_confidence > 0.7`

M8：

应该：

提供：

> 明确 confidence semantics。

不要：

只产生：

神秘数字。

---

# 299. Analysis Quality

M8：

必须：

记录：

> known limitations。

尤其：

BPM / Key：

不可能：

对所有：

音乐类型：

100% 正确。

不要：

README：

声称：

“准确率 99.9%”

除非：

有：

真实：

benchmark data。

---

# 300. Benchmark Dataset

允许：

建立：

小型：

合法：

benchmark suite。

至少：

覆盖：

- electronic；
- rock；
- acoustic；
- ambient；
- classical；
- speech-like；
- silence；
- noisy。

如果：

没有：

合法足够数据：

使用：

synthetic：

并：

明确：

局限。

---

# 301. Benchmark Metrics

至少：

记录：

### BPM

- absolute error；
- tolerance hit rate；
- confidence。

### Key

- exact canonical match；
- relative key errors；
- confidence。

### LUFS

- absolute error vs reference。

### Embedding

- determinism；
- inference latency；
- memory。

不要：

声称：

模型：

“理解情绪”

除非：

有：

真实：

验证。

---

# 302. Performance Baseline

更新：

`docs/PERF.md`

至少：

记录：

```text decode throughput
analysis throughput
BPM time
Key time
LUFS time
MFCC time
Embedding inference time
end-to-end time
peak RAM
DB write latency
batch query latency
```

---

# 303. Performance by Track Length

至少：

```text 30s
3min
5min
10min
60min
```

如果：

fixture：

无法合法提供：

使用：

synthetic audio。

---

# 304. Performance by Worker Count

至少：

测试：

```text 1
2
4
```

或：

符合机器：

实际：

worker configurations。

观察：

是否：

> throughput 增加但 playback 稳定性下降。

---

# 305. Playback Priority Benchmark

必须：

记录：

播放中：

analysis：

是否导致：

- underrun；
- increased callback time；
- UI stall。

---

# 306. Memory Benchmark

记录：

```text baseline app
+ analysis worker
+ model loaded
+ 1 track
+ batch
```

---

# 307. Model Loading Benchmark

记录：

```text model load time
warmup time
single inference
batch inference
unload
```

---

# 308. DB Growth Benchmark

至少：

模拟：

```text 1k
10k
100k
```

分析：

记录：

- DB size；
- query time；
- batch read；
- migration time。

---

# 309. Analysis Queue Benchmark

至少：

模拟：

```text enqueue 10k
```

观察：

- scheduling latency；
- memory；
- duplicate deduplication；
- cancellation。

---

# 310. No Unbounded Queue

队列：

必须：

有：

> bounded memory representation。

不要：

100k jobs：

每个：

复制整个：

Track / metadata / PCM。

---

# 311. Queue Item

只保存：

```text TrackId
AnalysisKey
Priority
JobId
```

而不是：

> 整首 PCM。

---

# 312. Progress Memory

progress：

不能：

为：

100k jobs：

积累：

无限 event history。

---

# 313. Analysis Event Coalescing

UI：

不需要：

每个：

feature：

一个 event。

可以：

聚合：

```text stage completed
```

然后：

发送：

低频状态。

---

# 314. IPC Frequency

M8：

不得：

通过 Tauri：

发送：

高频：

analysis frame data。

例如：

MFCC：

不能：

> 每个 frame 每次 IPC。

---

# 315. Local Diagnostics Transport

如需：

展示：

feature：

优先：

> command/query

而不是：

> continuous push。

---

# 316. Frontend State

如果：

M8 增加：

Zustand：

只能保存：

```text analysis status
analysis progress
selected track analysis summary
```

不要：

把：

整个：

MFCC matrix / embedding dataset：

放入：

React reactive state。

---

# 317. Large Vector Handling

Embedding：

后续：

M9：

读取：

应：

> backend batch query

不要：

把：

所有：

embedding：

发送：

前端。

---

# 318. No Browser-side ML

正式 embedding：

不得：

使用：

browser-side：

JS ML model：

作为：

默认正式管线。

除非：

架构审计明确证明：

它符合：

- privacy；
- memory；
- licensing；
- commercial packaging；

并且：

不会：

重复：

Rust analysis engine。

默认：

优先：

> Rust/local backend inference。

---

# 319. No Duplicate Analysis Engines

不要：

Rust：

分析一遍。

React：

又分析一遍。

整个系统：

必须：

只有：

> 一个权威 analysis pipeline。

---

# 320. No Randomness

M8：

核心分析：

不得：

依赖：

randomness。

即使：

某个 ML runtime：

有：

随机优化：

测试：

必须：

固定：

seed / inference settings。

---

# 321. Reproducibility Record

每条 analysis：

尽量能够：

追溯：

```text content_identity
analysis_version
algorithm_versions
model_id
model_version
preprocessing_version
```

---

# 322. Analysis Export

[B]

可以：

提供：

```text export analysis JSON
```

用于：

debug / bug report。

但：

默认：

不上传。

---

# 323. Export Safety

导出：

不得：

包含：

- raw audio；
- credentials；
- unrelated local paths；
- personal account information。

---

# 324. Bug Reproduction Bundle

[B]

未来：

可以：

支持：

> tiny synthetic reproduction bundle。

本阶段：

只需：

保持 analysis：

具有：

足够：

version metadata。

---

# 325. Documentation Evidence

最终：

README：

只描述：

真实：

M8：

能力。

不要：

宣称：

```text AI understands your music
```

除非：

产品：

有：

真实：

验证。

更准确：

可以写：

> 本地分析 BPM、Key、响度、频谱特征、MFCC 与音乐向量。

---

# 326. No Marketing Metrics

禁止：

伪造：

- accuracy；
- emotional understanding；
- recommendation quality；
- similarity score。

所有：

benchmark：

必须：

真实：

可复现。

---

# 327. Security Audit

重新扫描：

```text api_key
apikey
password
token
client_secret
Authorization
Cookie
BEGIN PRIVATE KEY
.env
```

并检查：

- model paths；
- arbitrary file access；
- batch abuse；
- malformed analysis data；
- BLOB size limits；
- integer overflow；
- float overflow；
- NaN；
- Infinity；
- model loading；
- dynamic library loading；
- SQLite transactions；
- no network access。

---

# 328. Offline Audit

必须：

在：

> 完全断网环境

运行：

至少：

```text local track
↓
analysis
↓
save
↓
reopen
↓
read
```

确认：

BPM / Key / LUFS / MFCC / embedding：

按实际：

> 全部可运行。

如果 embedding model：

没有：

正确安装：

应该：

明确：

`ModelUnavailable`

而不是：

假装：

PASS。

---

# 329. Dependency Lock

M8：

完成后：

确认：

```text Cargo.lock
package lock
```

没有：

不必要：

大规模升级。

---

# 330. Full Dependency Audit

执行：

```powershell
cargo tree
cargo deny check
```

检查：

- 新 DSP crate；
- FFT；
- ML runtime；
- model tooling；
- image / utility dependencies；

是否：

符合商业许可证。

---

# 331. Architecture Review

确认：

依赖方向：

不得：

```text aura-analysis
 → Tauri
 → React
 → Three.js
```

正确：

```text aura-analysis
      ↓
Application
      ↓
Tauri / UI
```

---

# 332. No Tauri in Analysis Core

`aura-analysis`：

尽量：

保持：

> framework independent。

Tauri：

只在：

application adapter。

---

# 333. No SQLite in Feature Algorithms

BPM：

Key：

MFCC：

Embedding：

本身：

不要：

直接：

SQL。

---

# 334. Feature Algorithm Purity

尽可能：

例如：

```text estimate_bpm(audio)
detect_key(audio)
compute_mfcc(audio)
compute_loudness(audio)
```

输入：

> deterministic data

输出：

> deterministic result。

方便：

单元测试。

---

# 335. Analysis Result Validation

建立：

```text validate_analysis_result()
```

至少检查：

### BPM

finite + range。

### Key

valid enum。

### LUFS

finite / valid semantics。

### MFCC

dimension / finite。

### Embedding

dimension / dtype / finite。

---

# 336. Validation Before Persistence

任何：

analysis：

必须：

先：

```text compute
↓
validate
↓
persist
```

不能：

直接：

把：

invalid value：

写：

SQLite。

---

# 337. Validation After Load

数据库：

读：

analysis：

同样：

```text load
↓
validate
↓
return
```

避免：

corrupt BLOB：

直接：

进入：

M9。

---

# 338. Unknown Version

读取：

未知：

`analysis_version`

必须：

> graceful unsupported

不能：

静默当成：

当前版本。

---

# 339. Old Version

如果：

旧 analysis：

仍：

兼容：

可以：

读取。

否则：

标记：

`Stale`

重新分析。

---

# 340. Future Version

未来：

analysis：

版本更高：

当前客户端：

不得：

覆盖。

---

# 341. Model Version Compatibility

同理：

当前 model：

不能：

无意识：

重新解释：

另一个：

embedding layout。

---

# 342. Analysis Hash

[B]

可以：

对：

canonical result：

计算：

hash。

用于：

- regression；
- cache invalidation；
- integrity。

---

# 343. Feature-Level Hash

[B]

可以：

进一步：

为：

```text tempo
key
loudness
mfcc
embedding
```

分别：

建立：

hash。

本阶段：

不强制。

---

# 344. Analysis Repro Test

最少：

同一 Track：

完整：

运行：

两次。

第二次：

必须：

得到：

相同：

`analysis_result_hash`

或：

documented floating tolerance。

---

# 345. Cross-Machine Determinism

至少：

尝试：

相同：

fixture：

在：

CI / local：

比较：

logical result。

如果：

embedding：

存在：

minor SIMD difference：

必须：

记录：

实际范围。

---

# 346. Cross-Platform

目标：

Windows：

优先。

如果：

Rust：

analysis core：

能够：

在：

non-Windows：

编译：

尽量：

不要：

写死：

Windows API。

Embedding：

也尽量：

通过：

portable provider abstraction。

---

# 347. Windows-specific Model Runtime

如果：

embedding runtime：

Windows-specific：

应：

隔离：

到：

platform adapter。

不要：

污染：

`aura-analysis` domain。

---

# 348. Test Parallelism

analysis tests：

如果：

共享：

模型：

必须：

避免：

多个 test：

重复：

加载：

几百 MB 模型。

使用：

- fixture singleton；
- serial test；
- test cache；

具体：

根据：

实际 runtime。

---

# 349. Test Isolation

但是：

测试之间：

不得：

依赖：

前一个测试：

修改：

SQLite。

每个：

integration test：

优先：

temporary DB。

---

# 350. Real Model CI Policy

如果：

real model：

太大：

不适合：

每次：

PR CI：

可以：

建立：

```text fast tests
extended local tests
```

但：

不能：

把：

“没有运行”

写：

`PASS`。

---

# 351. Extended Analysis Test

至少：

建立：

本地：

extended command：

能够：

真正：

分析：

一组：

> 10–100 tracks

检查：

完整：

end-to-end。

---

# 352. Final Long Soak

至少：

执行：

> 数小时级 analysis soak。

循环：

```text enqueue
analyze
cancel
reanalyze
query
reopen DB
reanalyze stale
```

检查：

- RAM；
- DB size；
- worker count；
- queue；
- model lifecycle；
- corruption。

---

# 353. Analysis + Playback Soak

同时：

```text playback
+
background analysis
```

持续：

> 长时间。

重点：

检查：

- audio underrun；
- CPU saturation；
- UI stall；
- memory growth。

---

# 354. Final Visual Contract Test

M6：

如果已有：

analysis summary：

必须：

确认：

UI：

能够：

> 正确显示：

```text analyzed
not analyzed
failed
```

但：

不要：

强制：

M8：

开发：

大型：

analysis page。

---

# 355. M7 Integration Test

必须：

确认：

```text analysis absent
→ Dreamscape works

analysis present
→ typed summary accepted
```

不得：

出现：

fake analysis。

---

# 356. M9 Contract Test

至少：

模拟：

```text get_analysis_batch(100)
→ embedding vectors
→ BPM
→ Key
→ energy
```

确认：

M9 future engine：

> 可以直接使用。

不要：

实现：

真正：

Map。

---

# 357. M9 No Graph

必须：

在：

scope audit：

明确：

不存在：

- graph；
- cluster；
- map layout；
- spatial similarity。

---

# 358. Final Scope Audit

完成后：

执行：

```powershell
git diff --stat
git diff
```

重点检查：

- 是否偷渡 M9 Music Map；
- 是否偷渡 recommendation；
- 是否偷渡 M7 Dreamscape；
- 是否偷渡 M10 SMTC/tray；
- 是否新增 network API；
- 是否下载 embedding model；
- 是否上传 audio；
- 是否添加 telemetry；
- 是否使用 fake BPM；
- 是否使用 fake Key；
- 是否使用 fake LUFS；
- 是否使用 random embedding；
- 是否引入第二套 decoder；
- 是否重新实现 SQLite；
- 是否把 MFCC / embedding 放入 Zustand；
- 是否分析阻塞 audio callback；
- 是否造成 unbounded memory；
- 是否造成 unbounded DB growth；
- 是否加入 GPL/AGPL/LGPL；
- 是否加入 model with incompatible license；
- 是否有 TODO；
- 是否 ignored tests；
- 是否弱化 assertion；
- 是否提交商业音乐；
- 是否提交大模型但没有 license evidence；
- 是否提交本地 analysis database；
- 是否提交 raw audio。

---

# 359. Frontend Quality Gate

必须：

```powershell
npm run typecheck
npm run lint
npm test
npm run build
```

如果有：

```powershell
npm run test:e2e
```

也必须运行。

---

# 360. Rust Quality Gate

必须：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

---

# 361. Tauri Build

当前环境：

如果：

可执行：

必须：

```powershell
tauri build
```

如果：

环境：

无法：

明确：

`BLOCKED`

不得：

写：

PASS。

---

# 362. Full Regression

M8 完成后：

必须：

重新：

### M1

- decoder；
- resampler；
- ring buffer；
- seek；
- generation；
- volume；
- callback zero allocation。

### M2

- gapless；
- crossfade；
- loudness；
- limiter；
- hotplug；
- EQ。

### M3

- scanner；
- metadata；
- FTS5；
- identity；
- artwork；
- watcher；
- migrations；
- stats。

### M4

- local；
- HTTP；
- HTTPS；
- ICY；
- Radio Browser；
- security；
- cache policy。

### M5

- Allowed；
- SessionOnly；
- Forbidden；
- cache persistence；
- Range；
- recovery；
- LRU；
- pin。

### M6

- shell；
- Now Playing；
- Library；
- Queue；
- Command Palette；
- Mini Player；
- Aurora；
- accessibility。

### M7

- seed；
- generation；
- biome；
- environment；
- mutation；
- replay；
- snapshot；
- renderer；
- reduced motion。

任何 regression：

> M8 FAIL。

---

# 363. Documentation Finalization

最终必须更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
SECURITY.md
PRIVACY.md
SOURCES.md
THIRD_PARTY_LICENSES.md
CHANGELOG.md
docs/PERF.md
```

---

# 364. README

只能描述：

> 实际已完成的分析能力。

例如：

```text
BPM / Tempo
Key
Loudness
Spectral Features
MFCC
Local Embedding
```

如果某项：

未完成：

必须：

标：

`Not available`

---

# 365. PRIVACY

明确：

M8：

分析：

> entirely local by default。

不得：

上传：

- audio；
- vector；
- metadata；
- results。

如果：

future：

存在：

network model update：

那是：

后续：

另行设计。

---

# 366. SECURITY

增加：

```text
Analysis Input Validation
Model Loading
Model File Integrity
Embedding BLOB Validation
Resource Limits
Batch Limits
Cancellation
Worker Isolation
No Network
```

---

# 367. PROGRESS

最终：

```text
M8-A = PASS
M8-B = PASS / NOT COMPLETED
```

并记录：

- actual algorithms；
- actual versions；
- actual model；
- model license；
- test count；
- benchmark；
- DB size；
- soak；
- regressions；
- known limitations。

---

# 368. Screenshot / Evidence

M8：

不是视觉阶段。

不要求：

大量 marketing screenshots。

但：

至少可以准备：

```text
m8-analysis-diagnostics.png
```

或：

等价 diagnostics evidence。

截图：

必须：

来自：

> 当前真实 build / diagnostic tool。

不得：

用：

PPT / mock。

---

# 369. Final Evidence — BPM

必须提供：

- algorithm；
- BPM range；
- confidence；
- synthetic fixtures；
- negative cases；
- deterministic test。

---

# 370. Final Evidence — Key

必须提供：

- algorithm；
- canonicalization；
- confidence；
- major/minor；
- synthetic fixtures；
- deterministic test。

---

# 371. Final Evidence — Loudness

必须提供：

- integrated LUFS；
- LRA；
- true peak；
- standard；
- tolerance；
- M2 compatibility。

---

# 372. Final Evidence — Spectral

必须提供：

- RMS；
- centroid；
- rolloff；
- brightness；
- normalization/range；
- deterministic test。

---

# 373. Final Evidence — MFCC

必须提供：

- coefficient count；
- preprocessing；
- summary format；
- storage bytes；
- deterministic test。

---

# 374. Final Evidence — Embedding

必须提供：

- model name；
- model version；
- runtime；
- dimension；
- dtype；
- normalization；
- similarity metric；
- model checksum；
- license；
- offline inference evidence；
- deterministic test；
- memory/inference benchmark。

---

# 375. Final Evidence — Persistence

必须提供：

```text fresh DB
migration
save
reopen
stale
replacement
rollback
corruption
```

---

# 376. Final Evidence — Scheduler

必须提供：

- enqueue；
- deduplication；
- cancellation；
- restart recovery；
- concurrency；
- stale job protection。

---

# 377. Final Evidence — Performance

必须提供：

```text per-track analysis time
throughput
RAM
model memory
DB growth
batch query latency
playback coexistence
```

---

# 378. Final Evidence — Offline

必须明确：

> 断网状态：

能够：

```text local track
↓
decode
↓
BPM
↓
Key
↓
LUFS
↓
MFCC
↓
Embedding
↓
SQLite
```

其中：

embedding：

如果：

模型未安装：

明确：

`ModelUnavailable`

而不是：

假装：

成功。

---

# 379. Final Evidence — No Telemetry

必须确认：

代码中：

没有：

- analytics；
- remote logs；
- model usage upload；
- audio upload。

---

# 380. Final M8 Audit

最终输出：

# M8 Final Audit

## A. Baseline
## B. aura-analysis
## C. Analysis Domain
## D. Versioning
## E. Content Identity
## F. Scheduler
## G. Cancellation
## H. Decode Pipeline
## I. BPM
## J. Beat Grid
## K. Key
## L. Chroma
## M. Camelot
## N. Loudness
## O. True Peak
## P. Spectral Features
## Q. Energy
## R. MFCC
## S. Embedding
## T. Model Runtime
## U. Model License
## V. Persistence
## W. Migrations
## X. Stale / Reanalysis
## Y. IPC / DTO
## Z. M6 Integration
## AA. M7 Integration
## AB. M9 Contract
## AC. Security
## AD. Performance
## AE. Determinism
## AF. Soak
## AG. M1 Regression
## AH. M2 Regression
## AI. M3 Regression
## AJ. M4 Regression
## AK. M5 Regression
## AL. M6 Regression
## AM. M7 Regression
## AN. Documentation
## AO. Scope Compliance

每项只能：

`PASS / FAIL / BLOCKED`

禁止：

- 基本完成；
- 大致可用；
- 应该没问题；
- 理论支持；
- 模型以后再补；
- 结果看起来合理。

---

# 381. M8-A 完成门槛

必须同时满足：

1. `aura-analysis` 正式可用。
2. Analysis schema 正常。
3. Analysis version 正常。
4. Content identity 正确绑定。
5. Incremental analysis 正常。
6. Cancellation 正常。
7. Scheduler 正常。
8. Duplicate job dedup 正常。
9. Stale job protection 正常。
10. BPM 真实可用。
11. BPM confidence 正常。
12. Beat grid 正常。
13. Key 真实可用。
14. Key confidence 正常。
15. Chroma pipeline 正常。
16. LUFS 真实可用。
17. LRA 正常。
18. True Peak 正常。
19. Spectral features 正常。
20. Energy 定义明确。
21. MFCC 真实可用。
22. MFCC summary 正常。
23. Embedding provider abstraction 正常。
24. 至少一个真实 local embedding provider 正常。
25. Embedding license 合规。
26. Embedding offline 可运行。
27. Embedding dimension / dtype / normalization 明确。
28. Embedding deterministic。
29. Analysis persistence 正常。
30. Migration 正常。
31. Reanalysis 正常。
32. Corruption handling 正常。
33. Numerical validation 正常。
34. Memory bound 正常。
35. CPU concurrency 有限制。
36. Playback 不被 background analysis 破坏。
37. M6 contract 正常。
38. M7 contract 正常。
39. M9 batch contract 正常。
40. M1–M7 regression PASS。
41. Security PASS。
42. Quality gates PASS。
43. Performance evidence PASS。
44. Offline audit PASS。
45. Documentation PASS。

---

# 382. M8-B

如果 B 级完成：

必须：

> 单独列出。

例如：

```text
Beat / Downbeat = PASS
Section Detection = PASS
Richer Features = PASS
Advanced Embedding Metadata = PASS
Background Scheduler = PASS
Power Policy = PASS
```

未完成：

明确：

`NOT COMPLETED`

不得：

把：

B 级：

混入：

A 级 completion。

---

# 383. Scope Compliance

最终必须明确：

### M9 没有被偷渡

无：

- Music Map；
- graph；
- cluster；
- spatial layout；
- similarity visualization。

### M10 没有被偷渡

无：

- SMTC；
- tray；
- Windows media integration。

### M11+ 没有被偷渡

无：

- Steam；
- updater；
- installer；
- cloud；
- account；
- telemetry。

### M8 不得反向扩张 M7

无：

- Dreamscape rewrite；
- new procedural world engine；
- new visual renderer。

---

# 384. Dependency Final Audit

再次：

```powershell
cargo tree
cargo deny check
```

并特别：

检查：

> model / runtime / DSP / FFT：

实际许可证。

---

# 385. Diff Review

完成：

但：

不要立即：

commit。

执行：

```powershell
git diff --stat
git diff
```

检查：

- 是否加入网络 API；
- 是否有 remote model download；
- 是否有 audio upload；
- 是否有 fake metrics；
- 是否有 random embeddings；
- 是否有 unbounded memory；
- 是否有 unbounded queue；
- 是否有 unbounded BLOB；
- 是否有 SQL string concatenation；
- 是否有 arbitrary file read；
- 是否有 model arbitrary path；
- 是否有 dynamic library loading；
- 是否有 callback access；
- 是否有 React storing huge vectors；
- 是否有 giant analysis DB;
- 是否有 duplicate decoder；
- 是否有第二套 cache；
- 是否有 telemetry；
- 是否有 GPL/AGPL/LGPL；
- 是否有 undocumented model license；
- 是否有 TODO；
- 是否有 ignored tests；
- 是否弱化 assertion；
- 是否提交真实音乐；
- 是否提交本地 analysis database；
- 是否提交不允许分发的模型文件。

---

# 386. Git Commit

只有：

- M8-A 全部 PASS；
- 已声明的 M8-B 全部 PASS；
- M1 regression PASS；
- M2 regression PASS；
- M3 regression PASS；
- M4 regression PASS；
- M5 regression PASS；
- M6 regression PASS；
- M7 regression PASS；
- security PASS；
- deterministic tests PASS；
- performance PASS；
- offline audit PASS；
- dependency audit PASS；
- final audit PASS；
- diff review 干净；

才允许：

```powershell
git add .
git commit -m "M8: implement audio analysis"
```

**不要 push。**

---

# 387. Commit Hygiene

commit：

不得包含：

- user music；
- commercial audio；
- local library DB；
- local cache；
- analysis DB；
- raw PCM dumps；
- giant debug exports；
- private model files；
- credentials；
- secrets；
- target；
- node_modules；
- dist；
- build artifacts；

除非：

文件：

明确属于：

> 正式、合法、可分发的项目资产。

---

# 388. PROGRESS

最终：

```text
M8-A = PASS
M8-B = PASS / NOT COMPLETED
```

记录：

- algorithms；
- analysis versions；
- model；
- model hash；
- licenses；
- test counts；
- benchmark；
- DB growth；
- memory；
- soak；
- known limitations。

---

# 389. 最终输出

完成后严格输出：

```text
# M8 Final Audit

A. Baseline
B. aura-analysis
C. Versioning
D. Scheduler
E. BPM
F. Beat Grid
G. Key
H. Chroma
I. Loudness
J. True Peak
K. Spectral
L. MFCC
M. Embedding
N. Model / License
O. Persistence
P. Reanalysis
Q. Cancellation
R. Determinism
S. Performance
T. Security
U. M6 Integration
V. M7 Integration
W. M9 Contract
X. M1 Regression
Y. M2 Regression
Z. M3 Regression
AA. M4 Regression
AB. M5 Regression
AC. M6 Regression
AD. M7 Regression
AE. Documentation
AF. Scope

M8-A = PASS / BLOCKED
M8-B = PASS / NOT COMPLETED
COMMIT = <hash>
NEXT = M9
```

---

# 390. 最终停止条件

如果：

```text
M8-A = PASS
```

并且：

`M8-B` 已声明的项目全部通过：

立即：

> 停止。

不要开始：

- M9；
- Music Map；
- similarity graph；
- graph layout；
- clustering；
- spatial music visualization。

下一阶段只能由我单独下达：

> **M9 提示词。**