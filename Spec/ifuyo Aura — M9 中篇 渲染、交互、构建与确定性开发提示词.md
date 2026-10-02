# ifuyo Aura
# M9 中篇 — Rendering / Interaction / Build / Determinism (§109–269)

> 本文档为原 M9 开发提示词的**中篇（渲染、交互、构建与确定性，§109–269）**。
> 上篇《ifuyo Aura — M9 上篇 相似度内核、kNN 图、布局与聚类开发提示词.md》承载 §0–108；下篇《ifuyo Aura — M9 下篇 测试、性能与发布门禁开发提示词.md》承载 §270–554。
> 章节编号沿用原 M9 全局编号；跨篇引用按编号在三篇中查找。
> 阅读本文前，上篇所定义的地图计算内核必须已就绪。

# 109. Node Rendering

单个：

track node：

优先：

> GPU instancing。

禁止：

10k / 100k：

个 DOM element。

---

# 110. Node Shape

默认：

简单：

```text circle
```

或：

小型：

> rounded marker。

不要：

每个 node：

加载完整：

album cover。

---

# 111. Artwork-on-Node

[B]

可以：

当：

zoom 足够：

显示：

> selected / nearby nodes artwork。

不要：

100k：

同时：

加载：

100k artwork。

---

# 112. Node Color

颜色：

必须：

来自：

稳定：

color mapping。

可以：

支持：

```text Cluster
Energy
Key
Brightness
Source
```

---

# 113. Stable Cluster Colors

Cluster：

颜色：

必须：

基于：

> cluster identity

稳定生成。

同一 Map：

不能：

每次：

随机换颜色。

---

# 114. Color Accessibility

不能：

仅依赖：

颜色：

表达：

cluster。

至少：

hover / label / shape / selection：

提供：

第二视觉通道。

---

# 115. Node Size

默认：

> 由 UI 状态：

而不是：

随意的音乐“重要性分数”。

可以：

表示：

- selected；
- hovered；
- cluster aggregation。

不要：

把：

play count：

直接变成：

> “音乐更重要”。

如果未来使用：

必须：

用户明确开启。

---

# 116. No Vanity Metrics

Map：

不要：

自动：

使用：

```text play_count
```

把：

某首歌：

画得特别大。

---

# 117. Edges

地图：

默认：

不要：

把：

所有：

graph edges：

永远：

画出来。

---

# 118. Edge Visibility

推荐：

只显示：

- selected node neighbors；
- hover neighborhood；
- cluster-level relationships；

或者：

高 similarity edges。

---

# 119. Edge Budget

必须：

设置：

```text max_visible_edges
```

防止：

全图：

变成：

蜘蛛网。

---

# 120. Edge Width

edge：

可以：

根据：

similarity：

映射：

透明度 / width。

但：

必须：

有：

稳定：

mapping。

---

# 121. Similarity Color

如果：

使用：

similarity：

必须：

避免：

多彩彩虹。

推荐：

> single accent intensity

而不是：

10 种颜色。

---

# 122. Hover

Hover：

至少：

显示：

```text Title
Artist
Album
Similarity neighborhood summary
Cluster
BPM
Key
```

只显示：

M8/M3：

实际存在的字段。

---

# 123. Selection

Click：

node：

选中：

```text TrackId
```

右侧 / 下方：

显示：

> Track Detail Panel。

---

# 124. Track Detail Panel

至少：

```text Artwork
Title
Artist
Album

BPM
Key
Loudness
Energy
Brightness

Cluster
Neighbors
Similarity
```

如果：

某项：

没有：

> hidden / unavailable。

---

# 125. Detail Panel 不重新分析

选择：

Track：

不能：

触发：

> new BPM / Key / embedding analysis

除非：

用户明确：

> Analyze。

而且：

该功能：

由 M8。

---

# 126. Neighbor List

显示：

例如：

```text Most similar
1. Track B — 0.93
2. Track C — 0.91
3. Track D — 0.89
```

必须：

来自：

真实：

M9 top-K。

---

# 127. Neighbor List 不得叫 Recommendation

UI：

使用：

```text Similar Tracks
Nearest Neighbors
Nearby in Map
```

不要：

使用：

```text Recommended
For You
You Should Listen
```

---

# 128. Similarity Semantics

UI：

如果：

显示：

`0.92`

必须：

知道：

这个：

0.92：

具体：

来自：

哪种：

metric。

可以：

显示：

> `Cosine similarity`

或：

在：

tooltip：

说明。

---

# 129. Search

Map：

必须：

能够：

搜索：

M3 Library。

搜索：

例如：

```text artist
title
album
```

---

# 130. Search-to-Focus

用户：

搜索：

歌曲：

Map：

应该：

能够：

> 平移 / 缩放到该 node。

---

# 131. Search Multiple

如果：

结果：

多个：

可以：

显示：

> result count

并：

允许：

逐个 focus。

---

# 132. Search Highlight

匹配：

node：

可以：

暂时：

增强：

- glow；
- outline；
- ring。

不得：

改变：

node 的真实位置。

---

# 133. Focus Animation

地图：

移动：

应：

平滑。

默认：

> 180–320ms。

Reduced Motion：

> instant / minimal transition。

---

# 134. Zoom

必须：

支持：

- wheel；
- trackpad；
- keyboard optional。

---

# 135. Zoom Bounds

必须：

有：

```text min_zoom
max_zoom
```

防止：

- Infinity；
- NaN；
- coordinate overflow。

---

# 136. Pan

必须：

支持：

鼠标 / trackpad：

> 拖动。

---

# 137. Zoom Center

鼠标：

wheel zoom：

最好：

> 以 cursor location 为中心。

但：

如果实现复杂：

至少：

保持：

稳定体验。

---

# 138. Reset View

提供：

```text Reset Map View
Fit Library
Focus Selected
```

这些：

是真实：

Map commands。

---

# 139. Fit Library

自动：

计算：

可见：

node bounds。

如果：

0 nodes：

不要：

产生：

divide-by-zero。

---

# 140. Fit Cluster

选择：

cluster：

可以：

> zoom to cluster bounds。

---

# 141. Viewport Query

不要：

一次：

从 Rust：

发送：

100k：

所有：

node：

如果：

当前：

viewport：

只需：

5k。

---

# 142. LOD

建立：

```text LOD0
LOD1
LOD2
LOD3
```

或者：

等价层级。

例如：

```text far → clusters
mid → node points
near → node details
selected → artwork/details
```

---

# 143. Viewport Culling

当前：

不可见：

node：

不要：

不断：

更新：

DOM。

GPU：

也尽量：

> cull / compact draw data。

---

# 144. Backend Map Dataset Loading

可以：

提供：

```text get_map_overview()
get_clusters()
get_nodes(viewport)
get_neighbors(track_id)
```

---

# 145. Frontend Map State

Zustand：

只能：

保存：

```text mapOpen
mapBuildStatus
selectedTrackId
selectedClusterId
zoom
pan
filters
colorMode
```

不要：

保存：

整个：

100k node dataset。

---

# 146. Node Dataset Storage

前端：

应：

使用：

> typed arrays / imperative renderer state

或：

其他：

非-reactive：

内存结构。

---

# 147. No Huge React State

禁止：

```text
mapStore.nodes = 100000 objects
```

然后：

每次：

render：

React：

全树更新。

---

# 148. Renderer Architecture

建议：

```text
MusicMapRenderer
MusicMapCamera
MusicMapNodeLayer
MusicMapEdgeLayer
MusicMapClusterLayer
MusicMapLabels
MusicMapInteraction
MusicMapLOD
MusicMapLifecycle
```

具体：

按真实架构调整。

---

# 149. One Map Renderer

Map：

只维护：

> 一个：

主：

Map renderer。

不要：

每个：

cluster：

建立：

WebGL canvas。

---

# 150. Don't Reuse Dreamscape Renderer

虽然：

M7：

用了：

Three.js。

M9：

可以：

共享：

基础 rendering utilities。

但：

不要：

让：

Music Map：

依赖：

Dreamscape world logic。

---

# 151. No Dreamscape Dependency

不得：

```text aura-map
 → aura-synth
```

如果：

不必要。

---

# 152. Map Renderer vs Dreamscape

两者：

完全不同：

```text Dreamscape
=
Procedural 3D world

Music Map
=
Data visualization / spatial graph
```

---

# 153. Renderer Coordinate System

必须：

定义：

稳定：

```text MapCoordinate
x
y
```

建议：

使用：

`f64`：

backend：

或者：

足够精度。

Frontend：

使用：

`float32`

时：

需要：

考虑：

large-world precision。

---

# 154. Coordinate Normalization

最终：

将：

布局：

归一化：

到：

有限：

例如：

```text [-1,1]
```

或：

类似：

稳定：

world coordinate。

不要：

产生：

百万级：

coordinate。

---

# 155. Layout Bounds

Map：

必须：

有：

finite：

bounds。

---

# 156. Spatial Hash

renderer：

如果：

节点：

很多：

可以：

建立：

前端：

spatial index：

用于：

hit testing。

不要：

对：

100k：

每个 click：

遍历：

全部 node。

---

# 157. Hit Testing

必须：

区分：

- hover；
- click；
- drag；
- pan。

---

# 158. Hover Frequency

不要：

每个：

mouse move：

向 Rust：

发 IPC。

Hit testing：

应该：

> 在前端本地。

---

# 159. Selection IPC

只在：

真正：

选中：

Track：

时：

调用：

backend：

获取：

detail / neighbors。

---

# 160. No 60Hz IPC

绝对禁止：

```text mousemove
→ IPC
→ Rust
```

---

# 161. Map Build Status

建立：

```text Pending
Building
Indexing
Graphing
Clustering
LayingOut
Persisting
Ready
Failed
Cancelled
Stale
```

可以：

合并：

但：

必须：

真实反映：

状态。

---

# 162. No Fake Progress

如果：

layout：

没有：

准确：

63%：

不要：

伪造。

可以：

显示：

```text Building similarity graph…
```

---

# 163. Build Progress

如果：

确实：

知道：

处理：

N/M：

可以：

显示：

真实：

progress。

---

# 164. Map Build Cancellation

必须：

支持：

```text cancel_map_build()
```

---

# 165. Build Deduplication

两个：

相同：

dataset：

build：

请求：

不应：

启动：

两份：

完整：

Map Build。

---

# 166. Map Build Generation

类似：

M1/M7/M8：

建立：

```text map_build_id
generation
```

旧：

build：

完成：

不得：

覆盖：

新：

build。

---

# 167. Example

```text map v1 building
↓
library changes
↓
map v2 starts
↓
v1 finishes late
```

必须：

> v1 不能覆盖 v2。

---

# 168. Incremental vs Full Build

建立：

```text FullRebuild
IncrementalUpdate
```

---

# 169. Full Rebuild Trigger

需要：

全量重建：

例如：

- model change；
- embedding dimension change；
- metric change；
- major map algorithm change。

---

# 170. Incremental Trigger

可以：

增量：

例如：

- add tracks；
- remove tracks；
- metadata changes；
- small analysis refresh。

---

# 171. Threshold

增量更新：

如果：

变化：

超过：

某个：

> percentage threshold

可以：

转：

full rebuild。

具体：

通过：

benchmark：

决定。

---

# 172. Rebuild Policy

例如：

```text <= 2% changed
→ incremental

> 2%
→ full rebuild
```

数字：

不要：

直接照搬。

以：

真实测试：

决定。

---

# 173. Persisted Layout

布局：

保存：

必须：

包含：

```text map_version
dataset_hash
track_id
x
y
```

---

# 174. Layout Restore

重新打开：

如果：

dataset：

没变：

直接：

> restore positions。

不要：

每次启动：

重新计算：

地图。

---

# 175. Layout Invalidation

以下：

必须：

使旧 positions：

失效：

- incompatible model；
- new layout algorithm；
- major graph changes；
- dataset replacement。

---

# 176. Position Hint

旧 position：

即使：

Map：

需要重建：

可以：

作为：

> initial position hint。

这样：

地图：

变化：

尽量：

平滑。

---

# 177. Mental Map Stability

用户：

应该：

能够：

建立：

> 对音乐地图的空间记忆。

因此：

> “同一首歌每次启动都在完全不同的位置”

是：

不可接受行为。

---

# 178. Cluster Stability

类似：

cluster：

算法：

升级：

可以：

变化。

但：

版本：

必须：

明确。

---

# 179. Map Snapshot

[B]

可以：

保存：

> 用户当前：

- viewport；
- filters；
- color mode；
- selected cluster；
- selected track。

不必：

保存：

graph。

---

# 180. Map View Persistence

[A]

至少：

保存：

> 上次：

zoom / pan。

如果：

map dataset：

没变：

恢复。

---

# 181. Map View Corruption

如果：

保存的：

viewport：

无效：

回退：

> default fit view。

不能：

crash。

---

# 182. Map Filters Persistence

可以：

保存：

用户：

最后：

filters。

但：

如果：

字段：

已经：

不存在：

必须：

graceful default。

---

# 183. Map Database

如果：

需要：

derived data：

可以：

使用：

单独：

Map store。

必须：

明确：

其：

> “rebuildable derived data” 身份。

---

# 184. Map Store Location

必须：

位于：

应用数据：

而不是：

音乐文件夹。

---

# 185. Map Store Isolation

Map store：

损坏：

不得：

删除：

M3 Library DB。

---

# 186. Atomic Map Write

Map dataset：

保存：

必须：

使用：

> transaction / temp + atomic replace

等：

可靠：

策略。

---

# 187. Map Recovery

启动：

如果：

map store：

损坏：

应该：

```text
Map = Stale / Rebuild Required
```

而不是：

> whole app unavailable。

---

# 188. Partial Build

如果：

build：

中途：

失败：

不能：

把：

半成品：

标记：

`Ready`。

---

# 189. Ready Semantics

只有：

```text
dataset validated
graph validated
clusters validated
positions validated
persistence committed
```

后：

才：

`Ready`。

---

# 190. Corrupted Map Node

某个：

node：

position：

NaN：

必须：

> invalidate / rebuild node。

不能：

整个：

renderer：

NaN。

---

# 191. Renderer Numeric Safety

每次：

收到：

positions：

检查：

```text Number.isFinite(x)
Number.isFinite(y)
```

---

# 192. Bounds Safety

Frontend：

不要：

接受：

巨大：

camera transform：

例如：

`1e30`

导致：

WebGL：

precision failure。

---

# 193. Cluster Bounds

cluster：

bounds：

必须：

finite。

---

# 194. WebGL Safety

Map renderer：

如果：

WebGL：

失败：

应该：

显示：

> 轻量：

Canvas / DOM fallback

或：

> simplified map list。

至少：

Library / Now Playing：

仍然：

正常。

---

# 195. WebGL Context Loss

必须：

处理：

```text webglcontextlost
webglcontextrestored
```

---

# 196. Renderer Lifecycle

必须：

支持：

```text create
loadDataset
resize
pause
resume
dispose
```

---

# 197. Hidden Map

Map：

不在当前页面：

必须：

> 暂停：

animation loop。

地图：

不应该：

后台：

持续：

60 FPS。

---

# 198. Map Idle Performance

静止：

地图：

不需要：

持续：

高频：

CPU animation。

默认：

静止：

应该：

> 0 或很低的 render workload。

---

# 199. Animation

允许：

- node focus；
- zoom transition；
- cluster expand。

但：

所有：

默认：

> brief + restrained。

---

# 200. No Constant Particle Effects

Music Map：

不要：

变成：

> Dreamscape / screensaver。

---

# 201. Visual Language

沿用：

ifuyo Aura：

```text Dark
Atmospheric
Warm
Restrained
Elegant
```

---

# 202. Map Background

可以：

使用：

很弱：

gradient / grain / atmospheric field。

但：

不要：

复制：

Aurora：

完整效果。

---

# 203. Node Style

推荐：

> small luminous points。

选中：

> ring / halo。

Hover：

> subtle glow。

---

# 204. Cluster Style

cluster：

可以：

使用：

> softly bounded region / halo。

不要：

巨大：

有边框：

的：

UI 气泡。

---

# 205. Edge Style

边：

应：

非常：

轻。

默认：

低 alpha。

---

# 206. Labels

不要：

100k labels。

默认：

只显示：

- selected；
- hovered；
- nearby；
- cluster。

---

# 207. Label Collision

如果：

label：

相互：

重叠：

优先：

隐藏低优先级：

label。

---

# 208. Label Priority

建议：

```text selected
>
hovered
>
search match
>
cluster
>
nearby
```

---

# 209. Search Highlight Priority

Search：

结果：

应该：

高于：

普通：

node：

显示。

---

# 210. Map Legend

至少：

当：

用户：

选择：

`Color By`

显示：

> legend。

---

# 211. Color By Cluster

显示：

cluster：

颜色。

---

# 212. Color By Energy

Energy：

必须：

来自：

M8。

如果：

没有：

energy：

不要：

提供：

该 color mode。

---

# 213. Color By Key

Key：

可以：

映射：

到：

稳定：

pitch-class：

palette。

不得：

使用：

随机颜色。

---

# 214. Color By Brightness

Brightness：

使用：

M8：

真实：

brightness。

---

# 215. Color By Source

可以：

显示：

```text Local
HTTP
Radio
```

但：

必须：

来自：

真实：

SourceKind。

---

# 216. Missing Feature in Color Mode

例如：

用户选择：

`Color by Energy`

部分歌曲：

没有：

energy。

必须：

显示：

neutral / unavailable。

不得：

使用：

0。

---

# 217. Map Filter UI

必须：

让：

用户：

能够：

快速：

启用 / 清除：

filter。

---

# 218. Filter Chips

建议：

显示：

当前：

filters：

例如：

```text BPM 120–130
Energy > 0.6
Genre = Electronic
```

---

# 219. Clear Filters

必须：

一键：

`Clear All Filters`

---

# 220. No Filter Surprise

改变：

filter：

不要：

自动：

播放：

歌曲。

---

# 221. Play From Map

点击：

Map node：

可以：

执行：

> Play Track

这应该：

调用：

M6：

既有：

Playback command。

---

# 222. Play Similar Track

Neighbor：

点击：

可以：

播放。

但：

不能：

自动：

连续：

播放：

一整个：

similarity list。

---

# 223. Queue From Map

[B]

可以：

用户明确：

将：

选中：

Track：

加入：

queue。

---

# 224. No Automatic Queue Rewrite

打开：

Music Map：

不得：

自动：

修改：

当前：

queue。

---

# 225. Current Track Highlight

正在播放：

Track：

应该：

在：

Map：

显示：

> current playing indicator。

---

# 226. Playback Position

Map：

不需要：

60 FPS：

接收：

playback position。

只需要：

> 当前 Track identity。

---

# 227. No Audio Analysis in Renderer

frontend：

不得：

计算：

- RMS；
- FFT；
- BPM；
- embedding。

---

# 228. Map Query API

至少：

提供：

```text
get_map_status()
get_map_overview()
get_map_nodes(...)
get_map_clusters()
get_map_neighbors(track_id, k)
get_map_track(track_id)
request_map_build()
cancel_map_build()
```

命名：

以实际：

application architecture：

为准。

---

# 229. Batch API

M9：

必须：

支持：

> batch node retrieval。

不要：

前端：

逐个：

请求：

1000 nodes。

---

# 230. Viewport Query

推荐：

API：

支持：

```text viewport
zoom_level
lod
filters
```

返回：

> 当前：

渲染需要的：

最小数据。

---

# 231. Server-side Culling

如果：

viewport query：

在：

backend：

执行：

必须：

避免：

每次：

pan：

都：

进行：

昂贵：

full graph computation。

---

# 232. Client-side Rendering Cache

frontend：

可以：

缓存：

最近：

map data。

但：

必须：

有：

> bounded memory。

---

# 233. Map Data Invalidated

如果：

dataset_hash：

改变：

前端：

旧：

dataset：

必须：

失效。

---

# 234. Map Version Handshake

启动：

Map：

先获取：

```text dataset_hash
map_version
status
```

再：

加载：

dataset。

---

# 235. Stale Map UI

如果：

Map：

需要：

rebuild：

UI：

显示：

例如：

```text Music Map needs to be rebuilt
1,482 new or changed tracks
```

前提：

数字来自：

真实：

backend。

---

# 236. User-Triggered Rebuild

可以：

提供：

```text Rebuild Map
```

---

# 237. Automatic Background Rebuild

[A]

允许：

library 变化后：

后台：

低优先级：

更新：

Map。

但：

不得：

阻塞：

- playback；
- Library；
- UI。

---

# 238. Background Build Priority

建议：

```text Playback
>
User interaction
>
Library scan
>
Map build
```

---

# 239. Analysis Priority Interaction

如果：

M8：

正在：

分析：

歌曲。

M9：

不得：

使用：

> 尚未完成：

的 embedding：

构建：

假的：

partial map。

可以：

等：

analysis 完成。

---

# 240. Incremental Analysis Arrival

M8：

每完成：

一批：

embedding：

M9：

可以：

标记：

dataset：

dirty。

不要：

每个 track：

完成：

就：

重建：

一次。

---

# 241. Debounced Map Rebuild

建议：

批量：

coalesce。

例如：

> 一批 analysis 完成后：

统一：

更新。

具体：

根据：

真实：

体验：

决定。

---

# 242. No Rebuild Storm

100 首：

analysis：

完成：

不得：

触发：

100 次：

Map rebuild。

---

# 243. Dataset Lock

Map：

build：

期间：

必须：

使用：

稳定：

dataset snapshot。

不能：

build：

中途：

数据库：

变化：

导致：

半旧半新：

graph。

---

# 244. Dataset Snapshot

推荐：

```text
query compatible tracks
↓
freeze dataset hash
↓
build
```

---

# 245. Snapshot Consistency

如果：

build：

结束：

dataset hash：

已经：

变化：

不得：

直接：

标记：

`Ready`。

应该：

标：

`Stale`

并：

等待：

下一次：

build。

---

# 246. Atomic Dataset Commit

只有：

dataset：

仍然：

匹配：

build：

开始时：

的：

hash：

才：

commit：

Map。

---

# 247. M8 Version Compatibility

M9：

必须：

能够：

明确：

显示：

```text
compatible analysis
stale analysis
unsupported model
missing embedding
```

---

# 248. M8 Analysis Coverage

Map Overview：

应该：

至少：

提供：

```text total tracks
mapped tracks
unmapped tracks
stale tracks
unsupported tracks
```

这些：

必须：

真实。

---

# 249. Coverage Ratio

可以：

显示：

```text 8,412 / 9,003 tracks mapped
```

不能：

把：

unmapped：

偷偷：

排除：

然后：

显示：

“全库”。

---

# 250. Map Overview

至少：

显示：

```text clusters
mapped tracks
artists
albums
```

来源：

M3 / M9。

---

# 251. Cluster Statistics

Map：

Cluster detail：

可以：

显示：

```text tracks
artists
albums
BPM range
energy range
dominant keys
```

缺失：

> 不显示。

---

# 252. Cluster Metadata

不要：

自动：

叫：

```text Happy
Sad
Workout
Night
```

除非：

未来：

有：

真实：

语义模型。

默认：

使用：

> Cluster 01 / Cluster 02

---

# 253. Cluster Dominant Metadata

可以：

计算：

```text dominant_artist
dominant_genre
dominant_key
```

但：

必须：

以：

真实：

frequency / distribution：

为依据。

---

# 254. No Semantic Hallucination

禁止：

从：

BPM / Key / Embedding：

直接：

声称：

```text this is relaxing
this is happy
this is emotional
```

除非：

未来：

有：

明确：

训练 / validation。

---

# 255. Similarity Explanation

Selected Track：

可以：

显示：

```text High similarity:
embedding similarity 0.91
```

也可以：

同时：

显示：

```text BPM 128
Key 8A
```

但：

不要：

伪造：

> “因为 BPM 和 Key 一样所以相似度是 0.91”。

相似度：

来自：

实际：

embedding metric。

---

# 256. Secondary Feature Comparison

Detail panel：

可以：

显示：

selected vs neighbor：

```text BPM
Key
Energy
Brightness
LUFS
```

但：

这些：

只是：

辅助：

信息。

不要：

把：

它们：

加权：

进：

embedding similarity：

除非：

明确：

设计：

另行版本化。

---

# 257. Similarity Formula

M9：

默认：

```text similarity = M8 embedding metric
```

如果：

未来：

引入：

hybrid similarity：

必须：

新增：

```text map_similarity_version
```

并：

记录：

精确：

公式。

---

# 258. Hybrid Similarity [B]

如果：

B：

实现：

例如：

```text embedding similarity
+
tempo compatibility
+
key compatibility
```

必须：

明确：

各项：

weight。

不得：

偷偷：

加入：

“感觉更合理”的权重。

---

# 259. No Hidden Weighting

A：

禁止：

：

```text score =
0.7 * embedding
+ 0.2 * bpm
+ 0.1 * key
```

这种：

隐藏：

公式。

M9-A：

严格：

使用：

M8：

canonical similarity。

---

# 260. Similarity Precision

数据库：

可以：

存：

`f32`

如果：

实际：

足够。

如果：

排序：

需要：

更多：

precision：

可以：

`f64`。

具体：

通过：

benchmark：

决定。

---

# 261. Ranking Precision

当：

两个：

similarity：

相同：

必须：

有：

stable tie-break。

---

# 262. Graph Build Reproducibility

同一：

```text dataset
analysis version
model
map version
parameters
```

重复：

build：

必须：

得到：

> 相同 graph hash。

---

# 263. Cluster Reproducibility

同理：

必须：

得到：

> 相同 cluster assignment/hash。

---

# 264. Layout Reproducibility

同理：

必须：

得到：

> 相同 logical positions。

允许：

documented：

浮点：

tolerance。

---

# 265. Layout Hash

可以：

建立：

```text map_layout_hash
```

用于：

regression。

---

# 266. Full Map Hash

推荐：

建立：

```text map_dataset_hash
```

由：

```text nodes
edges
clusters
positions
```

canonical serialization：

生成。

---

# 267. Canonical Serialization

必须：

稳定：

- node order；
- edge order；
- cluster order；
- field order。

---

# 268. Graph Ordering

所有：

edge：

必须：

canonicalize：

例如：

```text min(source,target)
max(source,target)
```

如果：

无向图。

---

# 269. Layout Numeric Tolerance

测试：

不要：

因为：

不同 CPU：

最后：

`1e-12`：

差异：

就：

失败。

使用：

合理：

epsilon。

---

