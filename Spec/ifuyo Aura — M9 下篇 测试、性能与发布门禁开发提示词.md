# ifuyo Aura
# M9 下篇 — Testing / Performance / Release Gates (§270–554)

> 本文档为原 M9 开发提示词的**下篇（测试、性能与发布门禁，§270–554）**。
> 上篇承载地图计算内核（§0–108）；中篇《ifuyo Aura — M9 中篇 渲染、交互、构建与确定性开发提示词.md》承载渲染、交互、构建与确定性（§109–269）。
> 章节编号沿用原 M9 全局编号；跨篇引用按编号在三篇中查找。
> 阅读本文前，上篇与中篇所定义的实现必须已完成或在收尾。

# 270. Deterministic Synthetic Dataset

Map tests：

必须：

使用：

> deterministic synthetic embeddings。

例如：

预定义：

```text cluster A vectors
cluster B vectors
cluster C vectors
```

而不是：

生产：

随机：

`Math.random()`。

---

# 271. Synthetic Dataset Design

至少：

测试：

```text 3 clusters
10 nodes
100 nodes
1000 nodes
```

---

# 272. Similarity Synthetic Test

构造：

明显：

相似：

vector：

验证：

> Top-K：

正确。

---

# 273. kNN Test

至少：

验证：

```text K = 1
K = 3
K = 5
```

以及：

tie-break。

---

# 274. Graph Test

至少：

检查：

- edge count；
- no self-loop；
- no duplicate；
- isolated nodes；
- symmetric policy。

---

# 275. Cluster Test

至少：

使用：

明显：

3 cluster：

数据：

确认：

> cluster：

不会：

随机漂移。

---

# 276. Layout Test

至少：

确认：

- finite；
- bounded；
- deterministic；
- no duplicate exact positions unless legitimate；
- no extreme overlap；
- no runaway coordinates。

---

# 277. Incremental Layout Test

流程：

```text 100 nodes
↓
save layout
↓
add 5 nodes
↓
incremental update
```

检查：

> 原 100 nodes：

位置：

变化：

应该：

在：

明确：

小范围：

而不是：

全图：

随机重排。

---

# 278. Removal Test

```text 100 nodes
↓
remove 3
```

其余：

node：

应：

尽可能：

保持：

position。

---

# 279. Replacement Test

```text Track A
↓
replace audio
```

旧：

node：

必须：

invalidate。

---

# 280. Metadata-only Test

```text title changed
```

Map：

position：

保持。

---

# 281. Model Version Test

```text model v1
↓
map
↓
model v2
```

必须：

> map stale / rebuild。

---

# 282. Analysis Version Test

```text analysis v1
↓
map
↓
analysis v2
```

必须：

> compatibility check：

失败：

则：

rebuild。

---

# 283. Corrupt Index Test

ANN / graph store：

故意：

破坏：

必须：

> detect + rebuild。

---

# 284. Corrupt Layout Test

一个：

node：

position：

NaN：

启动：

必须：

> recover / rebuild。

---

# 285. Corrupt Map Store

数据库 / file：

损坏：

不能：

> 主应用 crash。

---

# 286. Map Build Crash Test

模拟：

```text build
↓
crash
↓
restart
```

不得：

将：

partial build：

标记：

Ready。

---

# 287. Concurrent Builds

测试：

```text build v1
build v2
```

确认：

> v1 无法覆盖 v2。

---

# 288. Cancellation Test

```text build
↓
cancel
```

必须：

- worker stops；
- no partial ready map；
- no leaked tasks；
- no corrupted store。

---

# 289. Duplicate Build Test

同：

dataset：

连续：

```text request
request
request
```

应该：

> coalesce / deduplicate。

---

# 290. Large Dataset

至少：

synthetic：

测试：

```text 1k
10k
100k
```

---

# 291. Large Dataset Vector Memory

假设：

embedding dimension：

D。

必须：

测：

```text N × D × dtype_size
```

实际：

memory。

---

# 292. No Duplicate Vector Copies

M9：

尽量：

避免：

同时：

存在：

```text DB vector
+
Rust Vec<Vec<f32>>
+
ANN copy
+
frontend JS objects
```

四份：

巨大：

向量。

---

# 293. Vector Data Ownership

推荐：

明确：

```text SQLite/source
↓
bounded analysis batch
↓
index
```

不要：

长时间：

重复：

复制。

---

# 294. ANN Memory Benchmark

如果：

使用：

ANN：

记录：

```text raw vectors
index memory
build memory
query memory
```

---

# 295. Graph Memory Benchmark

记录：

```text nodes
edges
adjacency
clusters
positions
```

---

# 296. Layout Benchmark

至少：

测：

```text 1k
10k
50k
100k
```

如果：

100k：

超过：

可接受：

时间：

必须：

记录：

事实：

并：

使用：

B 级：

progressive / approximate：

策略。

不得：

伪造：

瞬间完成。

---

# 297. Build Throughput

更新：

`docs/PERF.md`

记录：

```text dataset extraction
index build
kNN query
graph construction
clustering
layout
persistence
```

---

# 298. Query Performance

至少：

测：

```text top-k neighbor
track detail
cluster detail
viewport query
search-to-focus
```

---

# 299. Frontend FPS

实际：

测：

```text 1k nodes
10k nodes
50k nodes
100k nodes
```

至少：

记录：

- FPS；
- frame time；
- draw calls；
- GPU memory；
- CPU usage。

---

# 300. Visible Node Budget

即使：

100k：

总 nodes：

屏幕上：

实际：

只需：

显示：

> 可视：

node subset。

必须：

利用：

LOD / viewport。

---

# 301. Visible Edge Budget

同样：

设置：

> 最大可视 edges。

---

# 302. Cluster-first LOD

地图：

缩小：

可以：

优先：

显示：

cluster：

而不是：

每个：

track。

---

# 303. Cluster Expand

Zoom：

进入：

cluster：

再：

逐渐：

显示：

node。

---

# 304. Progressive Rendering

Map：

首次：

加载：

应该：

可以：

先：

显示：

cluster：

然后：

逐步：

显示：

nodes。

---

# 305. No Fake Loading

progressive render：

必须：

来自：

实际：

dataset：

加载：

状态。

不能：

只是：

定时器：

切：

几张：

假：

loading screen。

---

# 306. No Full-Map UI Blocking

打开：

Map：

不得：

让：

主：

React app：

完全：

freeze。

---

# 307. Worker Isolation

昂贵：

操作：

应：

在：

Rust worker：

或者：

适合的：

background worker。

---

# 308. Frontend Main Thread

不得：

执行：

- 100k node clustering；
- full layout；
- all-pairs similarity；
- giant sort；
- full graph generation。

---

# 309. Search Main Thread

Map：

搜索：

复用：

M3：

query。

不要：

把：

100k metadata：

全部加载：

到：

JS：

然后：

过滤。

---

# 310. Search Result Focus

搜索：

结果：

通过：

TrackId：

定位。

---

# 311. Library ↔ Map Sync

从：

Library：

选中：

Track：

可以：

执行：

```text Open on Map
```

Map：

应该：

focus：

该 Track。

---

# 312. Map ↔ Library Sync

从：

Map：

选中：

Track：

可以：

执行：

```text Open in Library
```

使用：

真实：

TrackId。

---

# 313. Map ↔ Now Playing

Map：

选中：

当前：

播放：

track：

可以：

高亮：

current playing。

---

# 314. Double Click

Double click：

node：

可以：

播放。

但：

不自动：

修改：

queue。

---

# 315. Keyboard Accessibility

Map：

即使：

是：

canvas/WebGL：

也必须：

提供：

keyboard-accessible：

替代：

interaction。

例如：

selected node：

通过：

detail panel：

使用：

keyboard：

访问。

---

# 316. Canvas Accessibility

不要：

把：

整个：

canvas：

变成：

唯一：

内容：

来源。

必须：

提供：

selected node：

语义：

详情：

以及：

neighbor list：

DOM：

支持：

keyboard。

---

# 317. Keyboard Map Controls

至少：

可以：

支持：

```text + / -
F = Fit
Esc = clear selection
Enter = open selected
```

具体：

避免：

与：

全局快捷键：

冲突。

---

# 318. Reduced Motion

必须：

遵循：

M6：

`reducedMotion`。

---

# 319. Reduced Motion Map

开启：

后：

- zoom transitions；
- cluster expansion；
- search focus；

应该：

变成：

instant / shortened。

---

# 320. No Excessive Glow

Map：

不要：

变成：

高亮：

过量：

霓虹：

背景。

---

# 321. Map Visual Density

即使：

很多：

节点：

仍然：

要：

保持：

视觉：

呼吸：

空间。

---

# 322. No Gamer Radar

不要：

做成：

```text sci-fi radar
```

风格。

---

# 323. No 3D Fake Depth

如果：

使用：

Three.js：

仍然：

保持：

2D semantic map。

不要：

通过：

随机 z：

制造：

“3D 音乐银河”。

这会：

变成：

Dreamscape：

而不是：

Music Map。

---

# 324. Camera

使用：

> Orthographic Camera

或：

等价：

二维：

camera。

---

# 325. Camera Bounds

必须：

限制：

- zoom；
- pan；
- transform。

---

# 326. Device Pixel Ratio

高 DPI：

必须：

有：

> pixel ratio cap。

---

# 327. Map Resize

resize：

必须：

coalesce。

不要：

每个：

pixel：

重建：

dataset。

---

# 328. Context Loss

WebGL：

context lost：

必须：

重新：

> 建 renderer layers。

Map data：

不能：

丢。

---

# 329. Renderer Dispose

退出 Map：

必须：

释放：

- buffers；
- materials；
- render targets；
- event listeners；
- animation loop。

---

# 330. Map Renderer Leak Test

循环：

```text open map
close map
open map
close map
```

几十：

到：

上百：

次。

检查：

> GPU / JS memory：

没有：

持续：

不可解释：

增长。

---

# 331. Map View Restore Test

```text open
zoom/pan
close
reopen
```

确认：

> viewport：

正确：

恢复。

---

# 332. Map Search Test

至少：

```text title
artist
album
Chinese
English
special chars
```

---

# 333. Map Filter Test

至少：

```text BPM range
Key
Energy
Genre
Artist
```

根据：

M3/M8：

真实：

字段：

测试。

---

# 334. Missing Analysis Filter

过滤：

```text BPM > 120
```

没有：

BPM：

应该：

> not match。

---

# 335. Missing Energy Color

Color by Energy：

没有：

Energy：

> neutral。

---

# 336. Similarity Query Test

对：

selected Track：

查询：

top 5。

验证：

排序：

真实：

similarity。

---

# 337. Cluster Detail Test

选择：

cluster：

返回：

真实：

统计：

不得：

fabricate。

---

# 338. Viewport LOD Test

至少：

测试：

```text far zoom
mid zoom
near zoom
```

确认：

LOD：

正确：

切换。

---

# 339. Hover Test

鼠标：

在：

node：

应该：

显示：

正确：

Track。

---

# 340. Hit Test Accuracy

在：

缩放：

后：

node：

click：

仍然：

定位：

正确：

TrackId。

---

# 341. Pan + Hover

拖动：

地图：

过程中：

不得：

产生：

大量：

backend IPC。

---

# 342. Selection Race

快速：

```text click A
click B
click C
```

最终：

只有：

C：

为：

selected。

旧：

detail response：

不得：

覆盖：

C。

---

# 343. Detail Revision

详情：

query：

建议：

带：

```text request_id
revision
```

或：

等价：

stale-response protection。

---

# 344. Neighbor Query Race

例如：

```text A selected
↓
neighbor request A
↓
B selected
↓
neighbor request B
↓
A response returns late
```

A：

不得：

覆盖：

B：

详情：

状态。

---

# 345. Search Race

```text query A
query B
query C
```

旧：

query：

返回：

不得：

覆盖：

最新：

query。

---

# 346. Map Build Race

同样：

遵循：

M8：

generation：

思路。

---

# 347. IPC DTO

建立：

```text MapStatus
MapOverview
MapNodeSummary
MapClusterSummary
MapNeighbor
MapSelection
MapFilter
MapViewport
MapBuildProgress
```

具体：

以：

应用：

architecture：

为准。

---

# 348. IPC Minimum Data

不要：

把：

完整：

Rust MapNode：

全部：

发送：

前端。

只：

发送：

renderer：

当前：

需要：

的：

数据。

---

# 349. Batch DTO

例如：

```text MapNodeBatch
```

包含：

typed arrays：

或：

compact arrays：

也可以：

---

# 350. Large Dataset IPC

不得：

一次：

把：

100k：

节点：

作为：

100k：

JSON objects：

全部：

塞：

进：

Tauri IPC。

---

# 351. Binary Transport

[B]

如果：

实际：

测量：

JSON：

成为：

瓶颈：

可以：

使用：

compact binary / typed array：

transport。

但：

只有：

实际：

有价值：

才：

实现。

---

# 352. Data Chunking

如果：

需要：

将：

大：

Map dataset：

分块：

传输：

必须：

支持：

cancellation。

---

# 353. No IPC Flood

地图：

panning：

不得：

产生：

每帧：

IPC。

---

# 354. Local Renderer Cache

前端：

可以：

保留：

当前：

dataset：

但：

必须：

bounded。

---

# 355. Map Build Notifications

后台：

build：

完成：

可以：

发送：

```text map_status_changed
```

不要：

每个：

graph edge：

发送：

event。

---

# 356. No Progress Spam

Map build：

progress：

必须：

coalesced。

---

# 357. Map Diagnostics

可以：

提供：

本地：

diagnostics：

```text dataset tracks
mapped
clusters
edges
build time
layout time
memory
renderer FPS
visible nodes
visible edges
```

仅：

本地。

---

# 358. No Telemetry

绝对：

不要：

上传：

- map usage；
- selected tracks；
- similarity queries；
- cluster interactions；
- map screenshots；
- performance metrics。

---

# 359. Privacy

M9：

所有：

map：

在：

本地：

生成。

不会：

把：

音乐 library：

上传：

云端：

建图。

---

# 360. No Cloud Vector Store

不得：

添加：

- Pinecone；
- hosted vector DB；
- cloud similarity service；

或：

任何：

类似：

云服务。

---

# 361. Offline Map

没有：

网络：

情况下：

如果：

M8 analysis：

已经：

存在：

Map：

应该：

正常：

运行。

---

# 362. No Online Dependency

Map：

启动：

不得：

需要：

HTTP：

获取：

position / graph / cluster。

---

# 363. Search Offline

Map：

搜索：

也：

必须：

本地：

工作。

---

# 364. Security — Backend

检查：

- TrackId validation；
- cluster ID validation；
- viewport bounds；
- k bounds；
- filter size；
- dataset version；
- build generation；
- BLOB validation。

---

# 365. Security — Frontend

检查：

- malformed IPC；
- NaN；
- Infinity；
- negative counts；
- huge arrays；
- giant strings；
- invalid positions；
- invalid cluster IDs。

---

# 366. Map Query Abuse

例如：

```text k=999999999
```

必须：

> reject / clamp。

例如：

```text 10MB filter string
```

必须：

> reject。

---

# 367. Graph Abuse

任何：

来自：

persistent：

graph：

必须：

校验：

> edge count：

不能：

超过：

configured limit。

---

# 368. Node Abuse

不能：

数据库：

声称：

```text 10,000,000 nodes
```

然后：

frontend：

试图：

分配：

巨大：

buffers。

---

# 369. Safe Counts

所有：

count：

必须：

使用：

适当：

integer：

并：

检查：

overflow。

---

# 370. Safe Coordinates

所有：

position：

必须：

finite。

---

# 371. Safe Similarity

similarity：

必须：

finite。

---

# 372. SQLite / Store Safety

如果：

Map store：

使用：

SQLite：

必须：

使用：

参数化：

SQL。

禁止：

字符串：

拼接。

---

# 373. Derived Data Isolation

Map store：

任何：

错误：

不得：

写：

M3：

tracks。

---

# 374. Backup

Map derived data：

不需要：

像：

Library DB：

一样：

复杂：

backup。

因为：

它：

可：

重建。

但：

必须：

有：

> crash-safe commit。

---

# 375. Cache vs Map

M5：

是：

network cache。

M9：

Map：

是：

derived computation store。

两者：

完全：

分离。

---

# 376. Analysis vs Map

M8：

analysis：

是：

source truth。

M9：

map：

是：

derived view。

因此：

M8 删除：

旧：

embedding：

后：

M9：

应：

invalidate。

---

# 377. Model License

M9：

如果：

不新增：

model：

无需：

重复：

引入：

license。

但：

如果：

引入：

ANN / dimensionality reduction：

dependency：

必须：

审计：

license。

---

# 378. Dependency Policy

新增：

graph / layout / ANN：

依赖：

必须：

检查：

```text
license
MSRV
Windows
binary size
memory
maintenance
CI
commercial redistribution
```

---

# 379. No GPL / AGPL / LGPL

继续：

M0：

许可：

硬规则：

不得：

引入：

```text GPL
AGPL
LGPL
```

---

# 380. No Algorithm by Name Without Audit

不要：

因为：

某算法：

“业界常用”

就：

直接：

引入：

实现：

必须：

先：

确认：

实际：

crate / repository：

的：

许可证。

---

# 381. SOURCES.md

更新：

记录：

M9：

所有：

- graph algorithm sources；
- layout references；
- ANN library；
- math library；
- renderer utility；

包括：

- name；
- source；
- version；
- license；
- purpose；
- verification date；
- commercial status；
- limitations。

---

# 382. THIRD_PARTY_LICENSES.md

新增：

所有：

M9：

依赖：

必须：

真实：

记录。

---

# 383. ARCHITECTURE.md

加入：

```text
M9 Music Map Architecture
Analysis Dataset
Embedding Compatibility
Similarity
Nearest Neighbor Index
kNN Graph
Clustering
Layout
Incremental Update
Map Persistence
Map Renderer
LOD
Viewport
IPC
Performance
```

---

# 384. DECISIONS.md

至少记录：

- Map crate；
- analysis contract；
- dataset identity；
- embedding compatibility；
- similarity metric；
- K；
- threshold；
- graph symmetrization；
- edge weighting；
- ANN strategy；
- index parameters；
- clustering algorithm；
- cluster parameters；
- PCA / dimensionality reduction；
- sign canonicalization；
- layout algorithm；
- force parameters；
- iteration limit；
- incremental layout；
- position persistence；
- rebuild thresholds；
- map store；
- renderer；
- LOD；
- node/edge budgets；
- viewport query；
- color modes；
- cluster color policy；
- accessibility；
- reduced motion；
- IPC data strategy。

---

# 385. PROGRESS.md

开始：

```text
M9-A = IN PROGRESS
M9-B = NOT STARTED
```

完成后：

记录：

- map engine；
- similarity；
- graph；
- clusters；
- layout；
- persistence；
- renderer；
- LOD；
- filters；
- tests；
- performance；
- known limitations。

---

# 386. Test Fixture Policy

M9：

测试：

embedding：

允许：

使用：

> synthetic vectors。

但：

必须：

明确：

```text TEST ONLY
```

不得：

将：

synthetic：

Map：

混入：

production。

---

# 387. Synthetic Embedding Fixture

建立：

固定：

vectors：

例如：

```text Group A
Group B
Group C
```

向量：

必须：

确定性：

> explicitly defined / generated from fixed seed。

---

# 388. No Production Random Vectors

任何：

production：

code：

搜索：

必须：

证明：

没有：

```text random_vector()
Math.random()
```

用于：

Map：

data。

---

# 389. Similarity Test Matrix

至少：

```text same
orthogonal
opposite
nearby
zero
invalid dimension
NaN
Infinity
```

---

# 390. Neighbor Test Matrix

至少：

```text K=1
K=3
K=5
ties
duplicate similarity
isolated
```

---

# 391. Graph Test Matrix

至少：

```text empty
1 node
2 nodes
10 nodes
disconnected
dense
sparse
```

---

# 392. Cluster Test Matrix

至少：

```text 1 cluster
2 clusters
3 clusters
noise / isolated nodes
uneven clusters
```

---

# 393. Layout Test Matrix

至少：

```text 0 nodes
1 node
2 nodes
10
100
1000
```

---

# 394. Layout Pathological Cases

测试：

- all vectors identical；
- all vectors orthogonal-ish；
- one dominant cluster；
- many isolated nodes；
- extremely close nodes；
- duplicate vectors。

不得：

> explode。

---

# 395. Identical Embeddings

如果：

两个：

Track：

embedding：

完全：

相同：

similarity：

必须：

`1`。

Map：

可以：

把：

它们：

放：

非常：

接近。

但：

不能：

生成：

NaN。

---

# 396. Identical Positions

相同：

embedding：

可能：

产生：

相同：

initial position。

layout：

应该：

处理：

> overlap / collision

否则：

用户：

看不出：

节点。

---

# 397. Collision Resolution

至少：

对：

显著：

overlap：

进行：

轻量：

separation。

---

# 398. Layout Stability

separation：

必须：

deterministic。

不能：

使用：

未：

seed：

的：

random jitter。

---

# 399. Deterministic Jitter

如果：

必须：

使用：

微小：

jitter：

必须：

由：

稳定：

node identity：

派生：

seed。

---

# 400. Empty Dataset

Music Library：

没有：

compatible analysis：

Map：

必须：

显示：

> No analyzed tracks available for Music Map.

不要：

显示：

假地图。

---

# 401. One Node

只有：

一首：

歌曲：

Map：

应该：

显示：

> 单节点：

并：

提供：

Track detail。

---

# 402. Two Nodes

两个：

相似：

歌曲：

显示：

真实：

edge。

两个：

不相似：

允许：

无边。

---

# 403. Map Intro

空状态：

可以：

给出：

简短：

说明：

> Music Map uses your locally analyzed music to reveal relationships between tracks.

但：

不：

写：

“AI knows your taste”。

---

# 404. Analysis Coverage Prompt

如果：

大量：

未分析：

可以：

显示：

```text 4,812 / 9,003 tracks mapped
```

以及：

> Analyze more tracks

按钮：

调用：

M8：

真实：

analysis request。

---

# 405. Analyze More

点击：

必须：

进入：

M8：

真实：

scheduler。

M9：

不实现：

analysis。

---

# 406. Map Build Trigger

当：

新：

analysis：

完成：

M9：

可以：

提示：

> Map data changed; rebuild available。

或：

后台：

低优先级：

更新。

---

# 407. No Nagging

禁止：

不断：

弹：

“Map updated!”

toast。

---

# 408. Current Track

正在：

播放：

Track：

必须：

有：

> current indicator。

---

# 409. Playback Integration

Map：

选中：

Track：

可以：

显示：

真实：

playback state。

---

# 410. Queue Integration

M9：

不要：

自动：

重排：

queue。

---

# 411. Command Palette

M9：

可以：

注册：

```text
Open Music Map
Rebuild Music Map
Fit Music Map
Reset Music Map View
Focus Current Track
```

具体：

根据：

M6：

command architecture。

---

# 412. No Command Explosion

不要：

为：

100：

种：

Map：

微操作：

全部：

注册：

global command。

---

# 413. Context Actions

Map：

节点：

可以：

提供：

- Play；
- Play Next；
- Add to Queue；
- Open in Library；
- Show Similar Tracks。

这些：

必须：

调用：

真实：

backend/application commands。

---

# 414. No Automatic Recommendation

`Show Similar Tracks`

只：

打开：

真实：

邻居：

数据。

不能：

同时：

修改：

queue。

---

# 415. Similarity Panel

可以：

提供：

一个：

独立：

panel：

显示：

```text Similarity: 0.93
Metric: Cosine
```

以及：

neighbors。

---

# 416. Search / Selection State

必须：

区分：

```text search state
selection state
hover state
playback state
```

不要：

混成：

一个：

global object。

---

# 417. Map State Store

建议：

```text mapViewStore
mapInteractionStore
```

而：

Map data：

保持：

> renderer-owned。

---

# 418. No Giant Map Store

禁止：

```text mapStore.nodes
mapStore.edges
mapStore.positions
mapStore.clusters
mapStore.labels
```

全部：

放：

Zustand：

然后：

每次：

interaction：

导致：

巨大：

rerender。

---

# 419. React Integration

Map：

可以：

使用：

```text ref
imperative renderer
typed state
```

---

# 420. Renderer Events

renderer：

向：

React：

报告：

低频：

事件：

例如：

```text nodeSelected
clusterSelected
mapReady
```

---

# 421. High Frequency Renderer Data

以下：

不要：

进入：

React：

state：

```text camera x/y every frame
mouse x/y every frame
visible node every frame
```

---

# 422. Camera Local State

camera：

最好：

保持：

renderer：

本地。

需要：

持久化：

才：

snapshot。

---

# 423. Scroll / Wheel

Map：

必须：

不会：

意外：

滚动：

整个：

窗口。

Wheel：

在：

Map：

区域：

正确：

拦截：

但：

键盘：

accessibility：

仍：

保持。

---

# 424. Touchpad

至少：

考虑：

trackpad：

scroll：

与：

zoom：

行为。

---

# 425. Context Menu

右键：

可：

显示：

真实：

node：

actions。

如果：

没有：

真实：

功能：

不要：

显示：

空：

menu。

---

# 426. Map Toolbar

建议：

只保留：

核心：

```text Fit
Search
Filter
Color By
Rebuild
```

不要：

做：

几十：

按钮。

---

# 427. Map Sidebar

详情：

可以：

放：

右侧：

panel。

但：

地图：

仍：

占：

主要：

视觉：

空间。

---

# 428. Map Header

建议：

显示：

```text Music Map
8,412 tracks
42 clusters
```

这些：

来自：

真实：

data。

---

# 429. Map Status Indicator

如果：

Map：

正在：

build：

可以：

显示：

```text Updating map…
```

如果：

stale：

显示：

```text Map update available
```

---

# 430. Map Build UI

用户：

可：

点击：

```text Rebuild
```

来：

主动：

生成：

最新：

Map。

---

# 431. Background Rebuild UI

如果：

background：

生成：

进行：

不要：

阻塞：

主：

Map：

查看：

除非：

产品：

必须：

这样：

做。

---

# 432. Old Map During Rebuild

推荐：

> 保留：

旧：

Map：

可用。

状态：

显示：

> Updating in background。

新：

Map：

完成：

再：

atomic：

替换。

---

# 433. Stale Map Semantics

旧：

Map：

可以：

继续：

作为：

> last known good。

但：

必须：

标记：

stale。

---

# 434. Crash During Replacement

如果：

旧：

Map：

+ 新：

Map：

替换：

中：

崩溃：

重启：

必须：

至少：

恢复：

> last known good：

或者：

安全：

重建。

---

# 435. Double Buffer Map Dataset

推荐：

```text
active dataset
building dataset
```

两个：

逻辑：

状态：

隔离。

---

# 436. Build Output

新：

Map：

写：

到：

> temp / staging。

验证：

后：

原子：

替换：

active。

---

# 437. No Half-Ready Map

再次：

强调：

不能：

将：

中途：

数据：

暴露：

给：

renderer：

作为：

完成：

Map。

---

# 438. Map Build Lock

如果：

多个：

build：

请求：

必须：

有：

> single active build / coalescing。

---

# 439. Build Queue

不需要：

支持：

复杂：

持久：

Map build queue。

只需：

能够：

> 保证：

最新：

requested dataset：

最终：

成为：

active。

---

# 440. Map Build Resume

应用：

重启：

如果：

build：

上次：

Running：

应该：

> 重新判断：

active dataset：

与：

library dataset：

是否：

匹配。

不要：

永久：

显示：

`Building`。

---

# 441. Full Rebuild Scheduling

大型：

Library：

Map rebuild：

应：

低：

priority。

不得：

拖垮：

M8：

analysis。

---

# 442. M8 vs M9 Priority

```text
Audio playback
>
UI interaction
>
M8 analysis
>
M9 map build
```

---

# 443. CPU Budget

M9：

必须：

有限：

worker：

不能：

无限：

占满：

所有：

CPU。

---

# 444. GPU Budget

Map：

不能：

与：

Aurora：

同时：

维持：

两套：

高负载：

renderer。

---

# 445. Map / Aurora Lifecycle

进入：

Map：

不需要：

Aurora：

持续：

running。

可以：

暂停：

Aurora：

后台：

render loop。

---

# 446. Map / Dreamscape Lifecycle

打开：

Map：

不得：

启动：

Dreamscape。

打开：

Dreamscape：

不得：

启动：

Map：

renderer。

---

# 447. One Active Heavy Visualizer

原则：

```text Aurora
OR
Dreamscape
OR
Music Map
```

一次：

默认：

只：

有：

一个：

重型：

视觉：

renderer：

高负载运行。

---

# 448. Visual Resource Coordination

如果：

当前：

Map：

打开：

Aurora：

必须：

进入：

低负载：

或：

pause。

具体：

根据：

M6：

renderer：

manager。

---

# 449. Renderer Manager

如果：

当前：

没有：

统一：

visual renderer manager：

M9：

可以：

建立：

最小：

manager。

负责：

```text activeVisual
pause others
resume previous
```

不要：

建立：

巨大：

framework。

---

# 450. Renderer Manager 不拥有 World

它：

只：

协调：

生命周期。

不得：

知道：

Dreamscape：

内部：

generator。

---

# 451. M7 Regression

进入：

Map：

不：

破坏：

Dreamscape。

退出：

Map：

不：

破坏：

Aurora。

---

# 452. M6 Regression

M9：

完成：

必须：

重新：

验证：

- Tauri；
- Now Playing；
- Library；
- Queue；
- Command Palette；
- Mini Player；
- Aurora；
- settings；
- accessibility。

---

# 453. M8 Regression

尤其：

确认：

M9：

没有：

修改：

analysis：

结果：

格式。

---

# 454. M8 Batch Query

M9：

必须：

使用：

M8：

existing：

batch：

contract：

而：

不是：

直接：

读取：

M8：

内部：

struct。

---

# 455. No Internal Coupling

不要：

```text aura-map
 → aura-analysis private module
```

只：

依赖：

公开：

domain：

contract。

---

# 456. M3 Metadata

Map：

要：

显示：

title/artist/album：

应该：

读取：

M3：

Library：

数据。

不要：

Map：

复制：

完整：

metadata：

数据库。

---

# 457. Track Join

如果：

Map：

需要：

将：

analysis：

与：

metadata：

合并：

应：

通过：

TrackId：

join。

---

# 458. No Duplicate Metadata Store

M9：

不创建：

第二套：

artist/album：

事实：

库。

---

# 459. Performance Query Strategy

Map：

overview：

可以：

预计算。

详细：

track metadata：

按需：

查询。

---

# 460. No Full Metadata IPC

不得：

打开：

Map：

就：

向：

frontend：

发送：

整个：

library：

metadata。

---

# 461. Cluster Detail

cluster：

详情：

可以：

batch：

读取：

必要：

metadata。

---

# 462. Search / Map

搜索：

输入：

应该：

由：

M3：

执行。

Map：

接收：

TrackId：

结果。

---

# 463. Search Result Mapping

如果：

Track：

没有：

map node：

UI：

应该：

显示：

```text Track found, but not analyzed / mapped
```

而：

不是：

将：

它：

定位：

到：

随机位置。

---

# 464. Current Playing Not Mapped

如果：

当前：

正在：

播放：

Track：

没有：

embedding：

Map：

可以：

显示：

> `Current track is not mapped`

不要：

给：

它：

临时：

假：

位置。

---

# 465. Neighbor Cache

selected track：

最近：

查询：

可以：

短期：

缓存：

neighbors。

但：

必须：

在：

dataset_hash：

变化时：

失效。

---

# 466. Map Query Cache

Map：

query：

可以：

cache：

```text track_id + k + dataset_hash
```

---

# 467. No Stale Similarity

dataset：

变化：

后：

旧：

neighbor：

不能：

继续：

当：

最新：

数据：

显示：

---

# 468. Similarity Cache Persistence

[B]

可以：

持久：

保存：

top-K。

但：

必须：

版本化：

并：

可：

完全：

重建。

---

# 469. Cluster Cache Persistence

同理：

可以：

保存。

---

# 470. Layout Cache Persistence

优先：

保存：

positions。

---

# 471. Map Build Summary

最终：

Map build：

应该：

产生：

```text
nodes
edges
clusters
positions
statistics
hash
versions
```

---

# 472. Map Build Result

建议：

```text MapBuildResult
  dataset_hash
  map_version
  analysis_version
  model_id
  model_version
  node_count
  edge_count
  cluster_count
  build_duration
  layout_duration
  memory_peak
```

---

# 473. No User-visible Fake Build Metrics

所有：

duration / count：

必须：

真实。

---

# 474. Performance Evidence

更新：

`docs/PERF.md`

必须：

记录：

### Backend

- dataset extraction；
- vector validation；
- index build；
- kNN；
- graph；
- clustering；
- layout；
- persistence。

### Frontend

- initial map load；
- cluster LOD；
- node render；
- hover；
- zoom；
- pan；
- search focus；
- detail opening。

---

# 475. Hardware Record

每次：

benchmark：

记录：

- CPU；
- RAM；
- GPU；
- OS；
- build type；
- Rust；
- Node；
- dataset size。

---

# 476. Benchmark Dataset

至少：

```text
1k
10k
100k
```

synthetic：

dataset。

---

# 477. Benchmark Dimensions

如果：

M8 embedding：

当前：

dimension：

例如：

768。

Benchmark：

必须：

使用：

实际：

dimension。

---

# 478. Map Build Memory

必须：

记录：

峰值：

RAM。

---

# 479. Frontend Memory

必须：

记录：

JS heap / GPU：

在：

10k / 100k：

规模：

的：

实际：

行为。

---

# 480. No Memory Hand-waving

不要：

写：

> “内存应该没问题”。

必须：

实测。

---

# 481. Soak

至少：

进行：

> 数小时级：

Map：

使用：

soak。

循环：

```text open map
zoom
pan
select
search
filter
fit
open detail
play
return library
return map
```

---

# 482. Rebuild Soak

同时：

循环：

```text build
cancel
build
reopen
rebuild
```

检查：

- memory；
- CPU；
- DB；
- renderer；
- worker；
- no orphan tasks。

---

# 483. Large Library Soak

使用：

synthetic：

10k / 100k：

tracks：

执行：

- query；
- map build；
- rebuild；
- filter；
- selection。

---

# 484. Renderer Leak

至少：

```text open / close × 50+
```

---

# 485. Search Race Soak

连续：

快速：

改变：

搜索词：

观察：

> stale response：

不会：

覆盖：

最新。

---

# 486. Selection Race Soak

连续：

快速：

点击：

多个：

node。

---

# 487. Build Race Soak

不断：

library：

变化：

并：

触发：

map rebuild。

确认：

> 只有：

最新：

dataset：

成为：

active。

---

# 488. Corruption Soak

反复：

模拟：

- corrupt layout；
- corrupt graph;
- corrupt index；
- interrupted build。

确认：

> rebuild：

恢复。

---

# 489. Offline Soak

完全：

断网：

执行：

整个：

Map：

功能。

---

# 490. No Network Audit

M9：

运行：

必须：

不能：

访问：

外网：

来：

计算：

map。

可以：

用：

OS-level：

或：

test：

证明：

网络：

请求：

数量：

为：

0。

---

# 491. Telemetry Audit

搜索：

```text analytics
telemetry
tracking
sentry
posthog
segment
```

确认：

没有：

M9：

新增：

遥测。

---

# 492. Security Scan

重新：

检查：

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

- vector BLOB；
- map file；
- derived store；
- IPC；
- viewport；
- k；
- filter；
- positions；
- cluster IDs；
- dataset hash；
- corruption handling。

---

# 493. Dependency Audit

执行：

```powershell
cargo tree
cargo deny check
npm ls
```

重点：

- ANN；
- graph；
- layout；
- math；
- Three.js utilities；
- map rendering helpers。

---

# 494. No Duplicate Rendering Libraries

不要：

同时：

引入：

多个：

- graph renderer；
- canvas library；
- WebGL graph library；

除非：

真实：

必要。

优先：

复用：

当前：

Three.js：

体系。

---

# 495. No Giant Graph Framework

不要：

为了：

一个：

Music Map：

引入：

> 数 MB / 数十 MB：

大型：

graph framework。

---

# 496. Simple First

优先：

选择：

> 能满足：

10k–100k：

数据：

需求：

且：

可：

测试：

的：

最小：

架构。

---

# 497. Graph Algorithm Documentation

必须：

说明：

为什么：

选择：

当前：

graph：

算法。

不能：

只：

写：

> “popular”。

---

# 498. Layout Algorithm Documentation

同理：

必须：

说明：

- deterministic；
- scalability；
- stability；
- visual quality；
- incremental update；
- limitations。

---

# 499. Clustering Documentation

必须：

说明：

- 输入；
- 参数；
- deterministic；
- isolated nodes；
- cluster stability；
- limitations。

---

# 500. Product Limitation

最终：

README：

应：

如实：

说明：

> Map represents computational similarity based on local audio analysis; it is not a statement of genre truth or human judgment.

中文：

可以：

表达：

> 地图展示的是本地音频分析得到的相似关系，并不等同于绝对的音乐类型判断或人工评价。

---

# 501. No Claim of Musical Truth

禁止：

宣传：

```text
这首歌和那首歌本质上就是同一种音乐
```

更准确：

> 根据当前分析模型，它们在地图中被计算为较接近。

---

# 502. Model Limitation

如果：

embedding：

来自：

通用：

音频：

模型：

必须：

记录：

它：

实际：

适用范围：

不要：

声称：

它：

“真正理解音乐”。

---

# 503. No Marketing AI Hype

不要：

在：

UI：

加入：

```text
AI knows your music
AI understands you
```

---

# 504. Cluster Interpretation

UI：

cluster：

可以：

写：

```text 312 tracks
```

而：

不要：

自动：

写：

```text Happy Music
```

---

# 505. User Agency

Map：

必须：

是：

> exploration tool。

不是：

> decision engine。

---

# 506. No Forced Interaction

不得：

打开：

Map：

后：

自动：

- 播放；
- 切换歌曲；
- 改 queue；
- 启动 analysis；
- 启动网络请求。

---

# 507. Explicit Actions

Map：

只：

在：

用户：

明确：

- 点击；
- 搜索；
- 筛选；
- Rebuild；
- Play；

时：

改变：

相应：

状态。

---

# 508. Map Empty State

如果：

没有：

compatible：

analysis：

显示：

真实：

状态：

并：

提供：

`Analyze tracks`

如果：

M8：

已：

拥有：

对应：

action。

---

# 509. Partial Coverage UX

如果：

只有：

30%：

library：

有：

embedding：

Map：

仍然：

可以：

显示：

这：

30%。

并：

明确：

coverage。

---

# 510. No Hidden Exclusion

不要：

为了：

让：

地图：

看起来：

漂亮：

偷偷：

排除：

难分析：

歌曲。

---

# 511. Unsupported Analysis

例如：

embedding model：

不支持：

某音频：

该：

歌曲：

> Unmapped。

---

# 512. Map Statistics

必须：

是真实：

```text mapped
unmapped
clusters
edges
```

---

# 513. Library Update

M3：

新增：

Track：

Map：

应该：

最终：

发现：

dataset：

dirty。

---

# 514. Library Delete

M3：

删除：

Track：

Map：

最终：

应该：

移除：

对应：

node。

---

# 515. Watcher

M3 watcher：

不应该：

直接：

调用：

Map layout。

应用：

层：

负责：

> map dirty scheduling。

---

# 516. Architecture Boundary

推荐：

```text M3 Library Changed
        ↓
Application Event
        ↓
M9 Map Dirty
        ↓
Background Rebuild / Incremental Update
```

---

# 517. No Circular Dependency

不要：

```text aura-map ↔ aura-library
```

形成：

循环。

---

# 518. Repository Boundary

Map：

应该：

使用：

```text MapRepository
```

或：

等价：

interface。

---

# 519. Backend Query Boundary

Application：

只：

暴露：

需要：

的：

Map API。

---

# 520. Renderer Data Boundary

Renderer：

只：

接受：

```text positions
node IDs
visual metadata
cluster metadata
```

不要：

接触：

SQLite。

---

# 521. No SQL in Renderer

严格：

禁止：

frontend：

直接：

执行：

SQL。

---

# 522. No Filesystem in Renderer

也：

禁止：

renderer：

直接：

读取：

本地：

map file。

---

# 523. No Network in Renderer

同样：

禁止：

renderer：

`fetch()`：

map。

---

# 524. Renderer Error Isolation

Map：

renderer：

错误：

不得：

导致：

整个：

Tauri：

window：

白屏。

---

# 525. Fallback

如果：

WebGL：

失败：

至少：

显示：

```text
Map list / cluster list / selected track detail
```

保证：

用户：

仍然：

可以：

访问：

数据。

---

# 526. Fallback 不伪造空间关系

如果：

没有：

renderer：

不要：

用：

CSS：

random：

点：

伪装：

地图。

可以：

改成：

> Cluster / Similar Tracks list。

---

# 527. Visual Test Mode

建立：

```text VISUAL_TEST_MODE
```

固定：

- dataset fixture；
- layout version；
- camera；
- color mode；
- filter；
- DPR；
- animations。

---

# 528. Screenshot Evidence

至少：

准备：

```text 20-map-overview.png
21-map-clusters.png
22-map-selected-track.png
23-map-similar-tracks.png
24-map-filtered.png
25-map-search-focus.png
```

实际：

编号：

可：

调整。

截图：

必须：

来自：

> current real build。

---

# 529. Screenshot 不使用 Fake Data

截图：

必须：

来自：

合法：

synthetic fixture：

或者：

用户：

实际：

本地：

library：

如果：

后者：

不得：

提交：

音乐：

到：

仓库。

---

# 530. Visual Regression

固定：

test dataset：

执行：

截图：

比较：

> layout：

不能：

随机：

改变。

---

# 531. Screenshot Tolerance

GPU：

存在：

微小：

差异：

应：

使用：

合理：

pixel tolerance。

不要：

过严：

导致：

CI：

无意义：

失败。

---

# 532. UI Quality Gate

执行：

```powershell
npm run typecheck
npm run lint
npm test
npm run build
```

如果：

存在：

```powershell
npm run test:e2e
```

也：

必须：

执行。

---

# 533. Rust Quality Gate

执行：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

---

# 534. Tauri Build

当前：

Windows：

环境：

如果：

能够：

执行：

```powershell
tauri build
```

必须：

实际：

运行。

如果：

不能：

明确：

`BLOCKED`。

---

# 535. Full Regression

M9：

完成：

必须：

重新：

验证：

### M1

- decoder；
- resampler；
- ring buffer；
- seek；
- generation；
- volume；
- callback safety。

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
- identity；
- SQLite；
- FTS5；
- artwork；
- watcher；
- pagination；
- playback stats。

### M4

- local；
- HTTP；
- HTTPS；
- ICY；
- Radio Browser；
- source security；
- cache policy。

### M5

- Allowed；
- SessionOnly；
- Forbidden；
- Range；
- persistence；
- revalidation；
- LRU；
- pin；
- recovery。

### M6

- Tauri shell；
- Now Playing；
- Library；
- Queue；
- Command Palette；
- Mini Player；
- Aurora；
- settings；
- accessibility。

### M7

- seed；
- world；
- biome；
- environment；
- mutation；
- replay；
- snapshot；
- renderer；
- reduced motion。

### M8

- BPM；
- Key；
- LUFS；
- spectral；
- MFCC；
- embedding；
- persistence；
- model validation；
- analysis scheduler；
- deterministic tests。

任何：

regression：

> M9 FAIL。

---

# 536. M8 Analysis Must Remain Authoritative

如果：

M9：

需要：

修复：

M8：

只能：

最小：

fix。

禁止：

在：

M9：

内部：

复制：

analysis。

---

# 537. M7 Must Remain Independent

Music Map：

不得：

使用：

Dreamscape：

逻辑。

---

# 538. M10 Must Remain Untouched

不要：

实现：

- SMTC；
- tray；
- Windows media integration。

---

# 539. M11+ Must Remain Untouched

不要：

实现：

- updater；
- installer；
- Steam；
- cloud；
- account；
- telemetry。

---

# 540. Final Diff Review

完成：

实现：

后：

先：

不要：

commit。

执行：

```powershell
git diff --stat
git diff
```

重点：

检查：

- 是否偷渡 recommendation；
- 是否偷渡 AI DJ；
- 是否偷渡 autoplay；
- 是否偷渡 M10；
- 是否偷渡 Steam；
- 是否重新实现 M8；
- 是否 random node；
- 是否 random cluster；
- 是否 random position；
- 是否 fake similarity；
- 是否 fake data；
- 是否 all-pairs O(N²)；
- 是否把 100k nodes 放 React state；
- 是否 100k DOM nodes；
- 是否每帧 IPC；
- 是否 map renderer 与 Dreamscape renderer 同时高负载；
- 是否引入大型无必要依赖；
- 是否 GPL/AGPL/LGPL；
- 是否出现 network dependency；
- 是否存在 telemetry；
- 是否 Graph/Map 数据写入音乐目录；
- 是否 SQL string concatenation；
- 是否未验证 BLOB；
- 是否 position NaN；
- 是否 unbounded k；
- 是否 unbounded filter；
- 是否 giant JSON IPC；
- 是否 renderer leak；
- 是否 worker leak；
- 是否 stale build overwrites latest；
- 是否 partial map marked Ready；
- 是否有 TODO；
- 是否 ignored tests；
- 是否弱化 assertions；
- 是否提交真实音乐；
- 是否提交用户 artwork；
- 是否提交 local map store；
- 是否提交 local database；
- 是否提交 build artifacts。

---

# 541. Scope Audit

最终明确：

## M8 没有被重写

无：

- BPM engine；
- Key engine；
- LUFS engine；
- MFCC engine；
- Embedding engine。

## M10 没有被偷渡

无：

- SMTC；
- tray；
- Windows media control。

## M11+ 没有被偷渡

无：

- Steam；
- installer；
- updater；
- cloud；
- accounts；
- telemetry。

## M9 本身没有越界

无：

- recommendation；
- autoplay；
- AI DJ；
- social graph；
- online music graph。

---

# 542. Final M9 Audit

完成后输出：

# M9 Final Audit

## A. Baseline
## B. `aura-map`
## C. Analysis Dataset
## D. Embedding Compatibility
## E. Similarity Metric
## F. Nearest Neighbor Search
## G. ANN Index
## H. kNN Graph
## I. Graph Validation
## J. Clustering
## K. Cluster Statistics
## L. Spatial Layout
## M. Determinism
## N. Incremental Layout
## O. Position Persistence
## P. Map Persistence
## Q. Rebuild
## R. Incremental Update
## S. Cancellation
## T. M6 Map Integration
## U. Map Renderer
## V. LOD
## W. Viewport
## X. Search
## Y. Filters
## Z. Selection
## AA. Similar Tracks
## AB. Cluster Detail
## AC. Playback Integration
## AD. Accessibility
## AE. Reduced Motion
## AF. Performance
## AG. Large Dataset
## AH. Memory
## AI. Security
## AJ. Offline
## AK. Tests
## AL. Determinism
## AM. Soak
## AN. M1 Regression
## AO. M2 Regression
## AP. M3 Regression
## AQ. M4 Regression
## AR. M5 Regression
## AS. M6 Regression
## AT. M7 Regression
## AU. M8 Regression
## AV. Documentation
## AW. Scope Compliance

每一项只能：

`PASS / FAIL / BLOCKED`

禁止：

- 基本完成；
- 差不多；
- 大概可用；
- 理论支持；
- 应该没问题；
- 可以以后再修。

---

# 543. 必须提供的关键证据

## Analysis

必须证明：

```text
M8 embedding
↓
M9
```

真实：

连接。

---

## Similarity

必须证明：

```text same vector → 1
orthogonal → ~0
invalid → rejected
```

---

## Nearest Neighbor

必须：

提供：

```text top-K
stable ordering
K limits
```

---

## Graph

必须：

提供：

- node count；
- edge count；
- K；
- threshold；
- no self-loop；
- no duplicate；
- graph hash。

---

## Cluster

必须：

提供：

- algorithm；
- parameters；
- cluster count；
- deterministic result；
- cluster statistics。

---

## Layout

必须：

提供：

- algorithm；
- deterministic initialization；
- coordinate bounds；
- layout parameters；
- layout hash；
- incremental behavior。

---

## Persistence

必须：

提供：

```text fresh map
save
close
reopen
restore
```

---

## Incremental

必须：

提供：

```text add tracks
remove tracks
metadata-only update
analysis replacement
model version change
```

对应：

正确行为。

---

## Renderer

必须：

证明：

```text zoom
pan
hover
select
search focus
cluster focus
LOD
```

真实：

工作。

---

## Performance

必须：

提供：

```text 1k
10k
100k
```

数据：

实际：

backend / frontend：

性能：

其中：

无法：

运行：

必须：

明确：

`BLOCKED`

不能：

伪造。

---

## Offline

完全：

断网：

Map：

仍然：

能够：

使用：

已有：

analysis：

运行。

---

## No Fake Data

必须：

明确：

production：

没有：

- fake vector；
- fake cluster；
- fake positions；
- fake similarity。

---

# 544. M9-A 完成门槛

必须：

同时：

满足：

1. `aura-map` 正式可用。
2. M8 Analysis dataset 接入。
3. embedding compatibility 正确。
4. vector validation 正常。
5. canonical similarity metric 正常。
6. top-K neighbor query 正常。
7. scalable neighbor search 正常。
8. deterministic kNN graph 正常。
9. graph validation 正常。
10. deterministic clustering 正常。
11. cluster statistics 正常。
12. deterministic 2D layout 正常。
13. positions persistence 正常。
14. map version 正常。
15. dataset hash 正常。
16. stale detection 正常。
17. full rebuild 正常。
18. incremental update 正常。
19. cancellation 正常。
20. crash recovery 正常。
21. Map store 隔离正常。
22. M6 Map Shell 正式接入。
23. zoom 正常。
24. pan 正常。
25. hover 正常。
26. selection 正常。
27. search-to-focus 正常。
28. cluster navigation 正常。
29. real similarity detail 正常。
30. real playback integration 正常。
31. filter 正常。
32. LOD 正常。
33. no giant React state。
34. no high-frequency IPC。
35. WebGL fallback 正常。
36. reduced motion 正常。
37. accessibility baseline PASS。
38. performance evidence PASS。
39. offline PASS。
40. security PASS。
41. Rust quality gate PASS。
42. frontend quality gate PASS。
43. M1–M8 regression PASS。
44. documentation PASS。
45. scope audit PASS。

---

# 545. M9-B

如果 B 级完成：

必须：

单独：

列出：

```text
Advanced ANN = PASS
Incremental kNN = PASS
Advanced Clustering = PASS
Localized Layout = PASS
Cluster Labels = PASS
Density Layer = PASS
Album Art Nodes = PASS
Map Bookmarks = PASS
Saved Views = PASS
Large Library Optimization = PASS
```

未完成：

明确：

`NOT COMPLETED`

不得：

混入：

A。

---

# 546. README

加入：

真实：

Music Map：

说明。

例如：

> Music Map transforms locally analyzed tracks into an explorable spatial representation based on audio similarity.

中文：

可以：

> Music Map 根据本地音频分析结果，把音乐库组织成可探索的相似关系空间。

不要：

写：

> AI 自动理解你的音乐品味。

---

# 547. Privacy

明确：

Music Map：

不会：

上传：

- audio；
- embeddings；
- analysis；
- map data；
- interactions。

---

# 548. Performance Documentation

`docs/PERF.md`

必须：

包含：

至少：

```text
dataset size
embedding dimension
index build
graph build
cluster build
layout build
map load
viewport query
frontend FPS
memory
```

---

# 549. Security Documentation

`SECURITY.md`

加入：

```text
Vector Validation
Graph Validation
Dataset Versioning
Derived Store Isolation
IPC Bounds
Viewport Bounds
k Bounds
Filter Limits
NaN/Infinity Handling
Map Store Recovery
No Network Dependency
```

---

# 550. PROGRESS

最终：

```text
M9-A = PASS
M9-B = PASS / NOT COMPLETED
```

记录：

- actual similarity metric；
- actual ANN/index；
- actual K；
- graph policy；
- cluster algorithm；
- layout algorithm；
- dataset sizes tested；
- renderer FPS；
- memory；
- persistence；
- known limitations。

---

# 551. Git Commit

只有：

- M9-A 全部 PASS；
- 已声明 M9-B 全部 PASS；
- M1 regression PASS；
- M2 regression PASS；
- M3 regression PASS；
- M4 regression PASS；
- M5 regression PASS；
- M6 regression PASS；
- M7 regression PASS；
- M8 regression PASS；
- security PASS；
- determinism PASS；
- performance PASS；
- soak PASS；
- offline PASS；
- frontend gate PASS；
- Rust gate PASS；
- dependency audit PASS；
- final audit PASS；
- diff review 干净；

才允许：

```powershell
git add .
git commit -m "M9: implement music map"
```

**不要 push。**

---

# 552. Commit Hygiene

不得：

提交：

- user music；
- user artwork；
- local library DB；
- local map DB；
- map cache；
- generated ANN index；
- private embeddings；
- analysis exports；
- screenshots generated locally；
- credentials；
- secrets；
- target；
- node_modules；
- dist；
- build artifacts。

除非：

文件：

明确：

> 正式：

可分发：

test fixture / source asset。

---

# 553. Final Output

完成后严格输出：

```text
# M9 Final Audit

A. Baseline
B. aura-map
C. Dataset
D. Embedding Compatibility
E. Similarity
F. Nearest Neighbor
G. Graph
H. Clustering
I. Layout
J. Persistence
K. Rebuild
L. Incremental Update
M. Cancellation
N. M6 Integration
O. Renderer
P. LOD
Q. Viewport
R. Search
S. Filters
T. Selection
U. Similar Tracks
V. Cluster
W. Playback
X. Accessibility
Y. Performance
Z. Memory
AA. Security
AB. Offline
AC. Determinism
AD. Tests
AE. Soak
AF. M1 Regression
AG. M2 Regression
AH. M3 Regression
AI. M4 Regression
AJ. M5 Regression
AK. M6 Regression
AL. M7 Regression
AM. M8 Regression
AN. Documentation
AO. Scope

M9-A = PASS / BLOCKED
M9-B = PASS / NOT COMPLETED
COMMIT = <hash>
NEXT = M10
```

---

# 554. 最终停止条件

如果：

```text
M9-A = PASS
```

并且：

`M9-B` 已声明的项目全部通过：

立即：

> 停止。

不要开始：

- M10；
- SMTC；
- tray；
- Windows media integration；
- updater；
- installer；
- Steam。

下一阶段只能由我单独下达：

> **M10 提示词。**