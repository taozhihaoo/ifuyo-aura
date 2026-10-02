# ifuyo Aura
# M9 — Music Map / Similarity Graph / Clustering / Spatial Layout / Interactive Map
> 本文档为原 M9 开发提示词的**上篇（相似度内核、kNN 图、布局与聚类，§0–108）**。
> 中篇《ifuyo Aura — M9 中篇 渲染、交互、构建与确定性开发提示词.md》承载 §109–269；下篇《ifuyo Aura — M9 下篇 测试、性能与发布门禁开发提示词.md》承载 §270–554。
> 章节编号沿用原 M9 全局编号；跨篇引用（如“见 #132”）按编号在三篇中查找。


你现在进入 ifuyo Aura 项目的 **M9：Music Map 音乐地图系统**。

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
M8 Audio Analysis
```

基础上，正式实现：

> **Music Map：一个根据真实音乐分析结果构建的、可确定性生成、可高性能交互、可增量更新的本地音乐空间。**

M9 是：

> **ifuyo Aura 的 Music Intelligence Visualization Layer**

它第一次把：

```text
Track
+
M8 Analysis
+
Embedding
+
Similarity
+
BPM / Key / Energy / Loudness
```

组织成：

```text
Music Library
      ↓
Analysis Dataset
      ↓
Similarity Index
      ↓
Similarity Graph
      ↓
Clusters / Communities
      ↓
2D Spatial Layout
      ↓
Interactive Music Map
```

用户最终看到的应该是：

> **“我的音乐不是一张列表，而是一片有结构的空间。”**

---

# 1. 本阶段核心定义

Music Map：

不是：

- 推荐系统；
- AI DJ；
- 自动歌单；
- “你应该听什么”的机器；
- 在线音乐地图；
- 社交音乐地图；
- GPS 地图；
- Dreamscape；
- Aurora；
- 3D 游戏世界。

Music Map：

负责：

> **把用户自己的音乐库按照真实音频相似关系组织成一个可探索空间。**

---

# 2. 用户体验目标

用户打开 Music Map：

应该可以：

```text
看到自己的音乐分布
↓
看到相似音乐自然聚在一起
↓
看到不同音乐区域
↓
缩放 / 平移
↓
搜索歌曲
↓
定位歌曲
↓
查看附近歌曲
↓
查看相似度
↓
按 BPM / Key / Energy / Genre / Artist 等筛选
↓
点击歌曲
↓
打开详情 / 播放
```

目标不是：

> “地图告诉我该听什么。”

而是：

> **“地图让我发现自己已经拥有的音乐之间的关系。”**

---

# 3. M9 的产品边界

M9：

可以：

> 展示事实性的 similarity / clustering 关系。

M9：

不能：

> 替用户做播放决策。

允许：

```text
Find similar tracks
```

含义：

> 查看分析得到的相似邻居。

不允许变成：

```text
Recommended for you
You should listen to this
Best next song
AI DJ suggestion
```

这些属于未来独立产品能力。

---

# 4. 本阶段 A 级必须实现

至少：

1. `aura-map` 正式建立或补全。
2. M8 Analysis dataset adapter。
3. analysis compatibility validation。
4. embedding dataset extraction。
5. embedding model/version compatibility。
6. similarity metric contract。
7. cosine similarity。
8. vector validation。
9. exact similarity primitive。
10. scalable nearest-neighbor strategy。
11. top-K neighbor search。
12. deterministic tie-break。
13. k-nearest-neighbor graph。
14. graph edge weighting。
15. isolated node handling。
16. graph validation。
17. deterministic clustering/community detection。
18. cluster identity。
19. cluster statistics。
20. deterministic 2D layout。
21. initial layout generation。
22. incremental layout update。
23. position persistence。
24. dataset/version identity。
25. stale/incompatible analysis handling。
26. missing-analysis handling。
27. map generation lifecycle。
28. cancellation。
29. rebuild。
30. incremental update。
31. derived map index persistence。
32. crash-safe map persistence。
33. corruption recovery。
34. Map backend query API。
35. batch node retrieval。
36. selected-node neighbor query。
37. cluster query。
38. map viewport query / LOD data query。
39. typed IPC contract。
40. M6 Map Shell 正式接入。
41. WebGL/Canvas interactive renderer。
42. zoom。
43. pan。
44. hover。
45. selection。
46. search-to-focus。
47. cluster navigation。
48. map filtering。
49. node detail panel。
50. similarity inspection。
51. real playback integration。
52. performance LOD。
53. large-library support。
54. deterministic screenshot mode。
55. security validation。
56. complete automated tests。
57. performance benchmark。
58. M1–M8 regression。

---

# 5. M9 B 级能力

A 级全部稳定之后，可以实现：

1. 更高级 ANN index。
2. incremental kNN update。
3. incremental clustering。
4. localized layout refinement。
5. cluster semantic labeling。
6. density / heatmap layer。
7. richer filter combinations。
8. map trails / paths。
9. album-art node mode。
10. richer visual encoding。
11. map bookmarks。
12. saved map views。
13. map snapshots。
14. multiple map profiles。
15. animated focus transitions。
16. map minimap。
17. advanced similarity inspection。
18. large-library progressive loading。
19. 100k+ node optimization。
20. background map rebuild scheduling。

B 级不得破坏 A 级。

---

# 6. 本阶段明确禁止

不要实现：

- recommendation engine；
- smart recommendation；
- autoplay recommendation；
- AI DJ；
- automatic queue recommendation；
- “best next track”；
- social graph；
- online music graph；
- user sharing；
- cloud map；
- account system；
- telemetry；
- remote embedding；
- cloud vector database；
- online music metadata；
- lyrics；
- Dreamscape；
- Dreamscape world generation；
- new Aurora system；
- SMTC；
- tray；
- installer；
- updater；
- Steam；
- commercial-platform extraction；
- new Source provider；
- new network cache；
- new codec。

尤其：

> **M9 只使用 M8 已经真实生成的数据。**

禁止：

为了让地图“看起来完整”：

> 创建 fake embedding / fake similarity / fake cluster / fake position。

---

# 7. No Fake Map

禁止：

```text random node positions
random clusters
random edge weights
random embedding
```

也禁止：

```text TrackId hash
↓
position
```

然后：

> 宣称这是音乐相似度地图。

任何：

> 空间关系

必须：

来自：

真实：

```text M8 embedding
+
similarity metric
+
layout algorithm
```

---

# 8. 无分析时怎么办

如果：

某歌曲：

没有：

> compatible M8 embedding

该歌曲：

必须明确：

```text Unanalyzed
No compatible embedding
Stale analysis
Unsupported
```

不得：

给它随机位置。

UI：

可以：

显示：

> `142 tracks are not mapped`

或：

类似真实统计。

---

# 9. M8 Analysis 是唯一数据源

M9：

必须：

从 M8：

读取：

```text embedding
BPM
Key
Key confidence
LUFS
Energy
Brightness
Spectral features
Analysis version
Model version
```

具体：

以真实 M8 DTO 为准。

不得：

自己再次：

- decode；
- FFT；
- BPM；
- Key；
- LUFS；
- MFCC；
- embedding。

---

# 10. 不复制 M8

M9：

不得：

实现：

第二套：

```text analyzer
embedding provider
model runtime
audio decoder
```

M8：

是：

> analysis authority。

M9：

是：

> analysis consumer。

---

# 11. 开始前 Baseline Audit

任何代码修改前：

必须：

先审计真实仓库。

执行：

```powershell
git status
git branch
git log --oneline -25

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
npm run build
```

检查：

```text
crates/aura-core
crates/aura-library
crates/aura-sources
crates/aura-cache
crates/aura-analysis
crates/aura-synth
crates/aura-testkit
src-tauri
ui
docs
config
```

重点确认：

- M8 analysis schema；
- TrackAnalysis；
- embedding metadata；
- vector dimension；
- dtype；
- normalization；
- similarity metric；
- analysis version；
- model version；
- TrackId；
- content identity；
- M6 Map Shell；
- existing Three.js infrastructure；
- M7 renderer lifecycle；
- current Zustand architecture；
- current IPC architecture；
- current DB migration system；
- current performance framework；
- current license policy。

输出：

# M9 Baseline Audit

A. Git  
B. M1 Status  
C. M2 Status  
D. M3 Status  
E. M4 Status  
F. M5 Status  
G. M6 Status  
H. M7 Status  
I. M8 Status  
J. Analysis Schema  
K. Embedding Contract  
L. Existing Map Code  
M. Dependencies  
N. Performance Infrastructure  
O. Security / Licensing  
P. Risks

---

# 12. 前序阶段修复原则

如果：

M1–M8：

存在真正：

> 阻塞 M9 的缺陷。

只做：

> 最小必要修复。

不得借 M9：

- 重写 M8；
- 重写 M7；
- 重写 M6；
- 重写 M3；
- 重写 M5；
- 重写 Audio Core。

所有必要修复：

必须写入：

`DECISIONS.md`

与：

`PROGRESS.md`

---

# 13. `aura-map`

建立：

```text
crates/aura-map
```

职责：

```text
M8 Analysis Dataset
        ↓
Compatibility Check
        ↓
Vector Index
        ↓
Similarity
        ↓
kNN Graph
        ↓
Clusters
        ↓
Spatial Layout
        ↓
Map Dataset
```

不得负责：

- audio playback；
- WebGL implementation；
- Tauri window；
- HTTP；
- source discovery；
- Dreamscape。

---

# 14. 推荐 Map 分层

推荐：

```text
aura-analysis
      ↓
aura-map
      ↓
application layer
      ↓
typed IPC
      ↓
MusicMapRenderer
```

---

# 15. Map Domain Model

至少建立：

```text
MusicMap
MapDataset
MapVersion
MapBuild
MapNode
MapEdge
MapCluster
MapPosition
SimilarityScore
MapViewport
MapFilter
MapSelection
```

具体命名：

根据实际代码调整。

---

# 16. Map Node

一个：

`MapNode`

至少关联：

```text
TrackId
analysis_key
position
cluster_id
```

可包含：

```text
similarity_degree
```

但：

不要复制完整：

TrackAnalysis。

---

# 17. Map Node Identity

Node：

不能：

仅使用：

数组 index。

必须：

使用：

```text TrackId
+
compatible analysis identity
```

例如：

```text TrackId
content_identity
analysis_version
embedding_model_id
embedding_model_version
```

组成：

> stable node identity。

---

# 18. Content Replacement

M8：

如果：

Track：

内容被替换。

M9：

必须：

识别：

旧：

```text node
```

已失效。

不得：

继续使用：

旧 position / edges：

当作新音乐。

---

# 19. Metadata-only Change

如果：

Track：

只有：

- title；
- artist；
- album；

变化。

embedding：

不变。

则：

Map Node：

应该：

继续使用：

> 原位置。

---

# 20. Track Rename

文件：

rename：

TrackId：

保持。

Map：

保持：

> node identity / position。

---

# 21. Track Move

文件：

move：

Map：

不得：

因为 path 变化：

重新布局。

---

# 22. Analysis Version

M9：

必须：

记录：

```text analysis_version
```

---

# 23. Map Version

另外建立：

```text map_algorithm_version
```

它与：

M8：

`analysis_version`

完全不同。

---

# 24. 为什么需要 Map Version

如果：

layout algorithm：

变化。

同一个：

analysis dataset：

可能：

产生：

完全不同的位置。

因此：

必须：

能够：

识别：

```text old map
new map
```

---

# 25. Dataset Identity

必须建立：

```text dataset_id
```

或：

等价：

stable dataset identity。

至少由：

```text library / track set
+
compatible analysis key
+
embedding model
+
analysis version
```

决定。

---

# 26. Dataset Hash

推荐：

为：

当前 Map dataset：

计算：

> deterministic dataset hash。

输入：

必须：

canonical order。

例如：

按：

`TrackId`

排序。

不要：

依赖数据库自然返回顺序。

---

# 27. Dataset Order

所有：

map build：

读取：

Track：

必须：

按：

> stable deterministic order

进入 pipeline。

不能：

依赖：

filesystem enumeration order。

---

# 28. Embedding Compatibility

M9：

只能：

比较：

兼容向量。

必须：

确认：

```text dimension
dtype
normalization
model_id
model_version
analysis_version
distance_metric
```

相容。

---

# 29. 不兼容 Embedding

例如：

```text model-A 768D
model-B 384D
```

不得：

混入：

同一 Map。

---

# 30. Mixed Model Policy

默认：

> 一个 Music Map 只使用一个兼容 embedding model/version。

如果：

Library：

存在多模型：

可以：

显示：

```text 1,231 tracks unavailable for current map model
```

而不是：

偷偷：

转换。

---

# 31. No Silent Projection Across Models

禁止：

```text 768D
↓
truncate
↓
384D
```

也禁止：

```text model A
↓
random projection
↓
model B
```

除非：

未来有：

明确：

> trained compatibility transform。

本阶段：

不做。

---

# 32. Vector Validation

所有：

进入 Map：

的向量：

必须：

检查：

- dimension；
- dtype；
- finite；
- normalization；
- expected length；
- zero-vector semantics。

---

# 33. Zero Vector

如果：

向量：

是：

zero vector。

Cosine：

必须：

返回：

```text Invalid
```

或：

明确：

`No similarity`

不得：

返回：

> 0.0 并悄悄混入。

---

# 34. Invalid Vector

包含：

```text NaN
Infinity
wrong length
corrupt bytes
```

必须：

> exclude / invalid。

不得：

污染：

Map graph。

---

# 35. Similarity Metric

M8：

已经定义：

> similarity contract。

M9：

必须：

严格使用：

该 contract。

推荐：

如果 M8 已指定：

```text cosine similarity
```

则：

M9：

统一使用：

> cosine similarity。

不得：

M9：

自行改成：

Euclidean：

然后：

导致：

不同结果。

---

# 36. Cosine Similarity

定义：

```text
cosine(a,b)
```

至少：

支持：

```text same → 1
orthogonal → ~0
opposite → -1
```

具体：

以 M8 contract 为准。

---

# 37. Similarity Range

必须：

明确：

```text [-1, 1]
```

或：

项目 M8 实际定义范围。

所有 UI：

必须：

遵循：

真实范围。

---

# 38. Numerical Stability

Cosine：

必须：

避免：

- divide by zero；
- NaN；
- Infinity；
- overflow。

---

# 39. SIMD

可以：

使用：

SIMD / vectorized math。

但：

必须：

保持：

logical result：

在：

documented tolerance。

---

# 40. Exact Similarity Primitive

建立：

纯函数：

```text similarity(a,b)
```

便于：

单元测试。

不得：

依赖：

数据库。

---

# 41. Top-K Similarity

核心：

建立：

```text top_k_neighbors(track_id, k)
```

必须：

返回：

```text TrackId
similarity
rank
```

---

# 42. Top-K 稳定性

相似度相同时：

必须：

使用：

稳定 tie-break。

例如：

```text similarity DESC
TrackId ASC
```

保证：

每次：

结果：

一致。

---

# 43. K Bound

不要：

允许：

用户：

发送：

```text k = 10,000,000
```

必须：

bounded。

例如：

应用层：

规定：

合理：

`k <= 1000`

具体：

根据实际需求。

---

# 44. Exact vs Approximate Search

M9：

必须：

根据：

实际：

library size：

选择：

> exact / approximate neighbor search strategy。

不能：

盲目：

对：

100k tracks：

做：

O(N²)。

---

# 45. Search Strategy

建议：

## 小库

```text exact vector scan
```

## 中大型库

```text ANN / indexed nearest-neighbor
```

具体：

阈值：

通过：

benchmark 决定。

必须：

记录：

`DECISIONS.md`。

---

# 46. ANN Dependency

如果：

使用：

ANN dependency：

必须先审计：

- license；
- MSRV；
- Windows；
- memory；
- build；
- maintenance；
- commercial redistribution。

如果：

不符合：

M0 license policy：

不能使用。

---

# 47. ANN Determinism

如果：

ANN：

内部：

存在：

randomized build：

必须：

固定：

seed。

如果：

无法保证：

稳定结果：

需要：

定义：

> deterministic tie / tolerance semantics。

不能：

每次启动：

地图：

随机重排。

---

# 48. ANN Index Version

Index：

必须：

版本化。

至少：

```text index_algorithm_version
index_parameters_version
```

---

# 49. ANN Index Persistence

如果：

使用：

ANN：

允许：

持久化。

但：

必须：

包含：

```text model_id
model_version
dimension
metric
normalization
index_version
dataset_hash
```

否则：

> discard and rebuild。

---

# 50. ANN Corruption

Index：

损坏：

不得：

导致：

app crash。

应该：

> delete/rebuild derived index。

不得：

影响：

M3 Library DB。

---

# 51. kNN Graph

M9：

必须：

建立：

> k-nearest-neighbor graph。

---

# 52. Graph Direction

原始 kNN：

可以：

是：

> directed graph。

但：

地图视觉：

推荐转换：

> mutual / symmetrized graph。

具体：

通过：

benchmark：

决定。

---

# 53. Edge Weight

每条：

Edge：

至少包含：

```text source
target
similarity
```

---

# 54. Edge Validity

Edge：

必须：

满足：

```text source != target
finite similarity
compatible nodes
```

不得：

self-loop。

---

# 55. Negative Similarity

如果：

metric：

允许：

负 similarity：

必须：

决定：

是否：

进入：

graph。

例如：

默认：

> 负相似度不形成“相似边”。

具体规则：

记录。

---

# 56. Similarity Threshold

图构建：

可以：

使用：

```text top-k
+
minimum similarity threshold
```

避免：

大量：

非常弱的边。

---

# 57. K

默认：

建议：

```text k = 8–16
```

最终：

必须：

以实际：

图质量 / 性能：

决定。

---

# 58. Graph Density

不得：

让：

100k nodes：

生成：

数十亿 edges。

必须：

有：

```text max_edges
```

预算。

---

# 59. Graph Edge Bound

推荐：

```text max_edges ≈ N × K
```

再：

根据：

symmetrization：

控制。

---

# 60. Isolated Nodes

没有：

足够：

相似邻居的 Track：

允许：

成为：

> isolated node。

不得：

强行：

连接：

随机歌曲。

---

# 61. Graph Validation

建立：

```text validate_graph()
```

检查：

- node count；
- edge count；
- self-loop；
- invalid node；
- invalid weight；
- duplicate edge；
- graph symmetry policy；
- max degree。

---

# 62. Duplicate Edge

如果：

`A → B`

和：

`A → B`

出现：

重复：

必须：

去重。

---

# 63. Symmetric Edge

如果：

采用：

无向图：

确保：

```text A—B
```

不会：

产生：

不一致：

```text A→B = 0.91
B→A = 0.74
```

---

# 64. Edge Storage

Map：

是：

derived data。

可以：

采用：

```text SQLite
```

或：

> compact binary adjacency representation。

选择：

必须：

基于：

benchmark：

而不是：

为了理论优雅。

---

# 65. Map Derived Data

Map：

不得：

成为：

M3 Library 的：

> source of truth。

如果：

Map data：

损坏：

可以：

完全重建。

---

# 66. Map Persistence

推荐：

至少：

保存：

```text map metadata
dataset hash
algorithm versions
node positions
cluster ids
```

---

# 67. Graph Persistence

如果：

保存：

graph：

必须：

能：

验证：

```text dataset hash
map version
analysis compatibility
```

否则：

重建。

---

# 68. Layout

M9：

必须：

将：

graph：

转化：

为：

> 2D spatial coordinates。

---

# 69. 2D Map

第一版：

优先：

> 2D Music Map。

不是：

Dreamscape 3D。

不是：

> 一个新的 3D 世界。

---

# 70. Renderer

可使用：

已有：

Three.js。

但：

优先：

使用：

> orthographic 2D / instanced rendering。

不要：

建立：

又一个：

高负载：

3D world engine。

---

# 71. Layout Input

Layout：

至少：

使用：

```text graph
```

可：

使用：

> embedding-derived initialization。

---

# 72. 推荐 Layout Pipeline

推荐：

```text Embedding
 ↓
Deterministic dimensionality reduction / initialization
 ↓
Graph-aware spatial relaxation
 ↓
Collision / boundary adjustment
 ↓
Normalized 2D positions
```

具体：

算法：

根据：

实际 benchmark：

决定。

必须：

记录：

`DECISIONS.md`。

---

# 73. Dimensionality Reduction

如果：

需要：

3D/高维 → 2D：

可以：

使用：

- PCA；
- deterministic random projection；
- other deterministic dimensionality reduction。

不得：

使用：

不受控制的：

random initialization。

---

# 74. PCA

如果：

使用：

PCA：

必须：

固定：

- component order；
- sign convention；
- eigenvector tie-break。

否则：

同一个 dataset：

可能：

得到：

镜像/翻转：

不同结果。

---

# 75. Sign Ambiguity

这是：

必须：

处理的问题。

PCA：

如果：

component：

符号：

相反：

视觉：

可能：

整体镜像。

必须：

canonicalize。

例如：

> 最大绝对载荷项为正。

具体：

记录。

---

# 76. Force Layout

图：

如果：

需要：

force relaxation：

必须：

选择：

> 可扩展的布局算法。

不允许：

对：

100k：

直接：

O(N²)：

全部节点两两计算。

---

# 77. Barnes-Hut / Spatial Acceleration

如果：

使用：

force-based：

推荐：

采用：

- Barnes-Hut；
- grid；
- spatial partition；

或等价加速。

---

# 78. Layout Node Budget

布局：

应：

有：

```text max_iterations
max_layout_time
max_nodes
```

防止：

一个 pathological dataset：

无限计算。

---

# 79. Layout Cancellation

地图重建：

必须：

可取消。

用户：

打开 Library：

不得：

因为：

Map build：

卡死：

UI。

---

# 80. Layout Worker

推荐：

Map generation：

在：

> Rust worker / background task

执行。

---

# 81. Frontend 不计算完整布局

React：

不得：

对：

100k node：

运行：

layout algorithm。

前端：

只：

> 渲染后端生成的布局。

---

# 82. Incremental Layout

这是：

M9 重要能力。

如果：

只增加：

10 首歌曲：

不要：

默认：

把：

10,000 首全部重排。

---

# 83. New Node Position

新增歌曲：

可以：

初始化：

在：

> nearest neighbors centroid

附近。

---

# 84. Local Relaxation

然后：

只对：

邻域：

执行：

> localized relaxation。

已有：

nodes：

尽量：

保持。

---

# 85. Existing Position Stability

Map：

更新：

不应该：

因为一首新歌：

让：

整个地图：

大规模跳动。

---

# 86. Track Removal

歌曲：

从库删除：

Map：

应：

移除：

node / edges。

其它：

节点：

尽可能：

保持位置。

---

# 87. Content Replacement

旧：

Track：

替换：

必须：

视为：

new analysis identity。

Map：

不得：

继承：

旧节点的：

相似边：

作为事实。

可以：

继承：

> spatial hint

但：

必须：

重新计算：

similarity。

---

# 88. Analysis Refresh

M8：

算法升级：

Map：

必须：

检测：

dataset version change。

---

# 89. Rebuild Trigger

以下变化：

必须：

能够：

触发：

Map rebuild / reindex：

```text embedding model changed
analysis version changed
embedding preprocessing changed
map algorithm version changed
```

---

# 90. Metadata-only Update

以下：

不需要：

重建：

```text title
artist
album
genre
```

除非：

用户当前启用了：

> metadata-based color/filter

这只需要：

> UI refresh。

---

# 91. Filter Model

M9：

至少支持：

```text Artist
Album
Genre
Year
Source
BPM
Key
Energy
Loudness
```

具体：

根据：

M3/M8：

真实字段决定。

---

# 92. Filter 不改变真实 Graph

过滤：

默认：

> 不重建 similarity graph。

它只是：

> hide / dim / isolate nodes。

---

# 93. Filter Modes

至少：

```text Show Only Matching
Dim Non-Matching
```

---

# 94. Combined Filters

支持：

```text AND
```

例如：

```text BPM 120–130
AND
Energy > 0.6
```

---

# 95. Reuse M3 Filter AST

如果 M3 已经：

有：

`FilterExpression`

M9：

必须：

复用。

不要：

创造：

第二套：

filter language。

---

# 96. Map-specific Filter

如果：

M9：

增加：

```text similarity > 0.8
cluster:3
```

可以：

作为：

Map-specific query：

但：

不能：

污染：

M3 core filter AST。

---

# 97. Filter Safety

任何：

filter：

必须：

bounded。

禁止：

- 无限 OR；
- 巨型 regex；
- massive ID list；

造成：

UI / backend：

DoS。

---

# 98. Cluster

Map：

必须：

支持：

> cluster/community representation。

---

# 99. Cluster Algorithm

算法：

必须：

真实：

使用：

graph / vector data。

禁止：

```text cluster_id = hash(track_id) % 8
```

---

# 100. Deterministic Clustering

需要：

保证：

```text same dataset
same algorithm version
same parameters
```

→

> same cluster assignment。

---

# 101. Cluster ID

Cluster：

不要：

直接用：

算法内部：

临时 index。

建立：

稳定：

```text cluster_identity
```

或：

canonical cluster key。

---

# 102. Cluster Renumbering

如果：

算法：

重新运行：

cluster：

ID：

可能：

变化。

如果没有：

真正稳定 ID：

允许：

重新编号。

但：

Map UI：

不得：

将：

数字：

误认为：

永久用户数据。

---

# 103. Cluster Statistics

每个：

cluster：

至少：

计算：

```text count
centroid
radius / spread
density
dominant genre
dominant artist
bpm range
energy range
```

如果：

某 metadata：

缺失：

不要：

猜。

---

# 104. No Fake Cluster Label

禁止：

直接：

命名：

```text Chill
Workout
Late Night
Happy
```

除非：

有：

真实：

可验证：

语义模型。

默认：

cluster：

只使用：

例如：

```text Cluster 01
Cluster 02
```

并：

允许：

用：

metadata：

统计解释。

---

# 105. Cluster Summary

可以：

显示：

```text 423 tracks
38 artists
12 albums
118–132 BPM
```

这些：

必须：

来自：

真实数据。

---

# 106. Cluster Position

Cluster：

可以：

拥有：

> centroid position。

用于：

地图缩小时：

显示：

cluster marker。

---

# 107. Cluster LOD

当：

地图：

缩小到：

无法：

看到单个 node：

显示：

> cluster bubbles / aggregated markers。

---

# 108. Cluster Marker

Cluster：

marker：

必须：

显示：

真实：

```text track_count
```

不能：

random size。

---

