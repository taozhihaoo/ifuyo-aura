# ifuyo Aura
# M7 下篇 — Renderer / Integration / Testing / Release Gates (§128–330)

> 本文档为原 M7 开发提示词的**下篇（渲染、集成、测试与发布门禁，§128–330）**，由《ifuyo Aura — M7 上篇 Dreamscape 世界生成核心（Seed、Genome、Environment、World、Streaming）开发提示词.md》拆分而来。
> 上篇承载世界生成核心（§0–127）。章节编号沿用原 M7 全局编号；跨篇引用按编号在两篇中查找。
> 阅读本文前，上篇所定义的世界生成核心必须已完成或在收尾。

# 128. Renderer Architecture

建议：

```text
ui/src/visual/dreamscape/
  DreamscapeRenderer
  DreamscapeScene
  DreamscapeCamera
  DreamscapeMaterials
  DreamscapeInstancePool
  DreamscapeEffects
  DreamscapeTelemetry
  DreamscapeLifecycle
```

实际：

可以调整。

---

# 129. Renderer Lifecycle

必须：

支持：

```text
create
loadWorld
updateEnvironment
updateTelemetry
render
pause
resume
resize
dispose
```

---

# 130. One Renderer Rule

M6 已经：

拥有 Aurora。

M7：

不要：

同时：

启动：

> Aurora renderer + Dreamscape renderer

两个高负载 WebGL context。

---

# 131. View Switching

进入 Dreamscape：

应该：

> 暂停或隐藏 Aurora render loop。

退出：

> 恢复 Aurora。

不要：

两套循环：

同时跑。

---

# 132. GPU Resource Ownership

必须明确：

谁拥有：

- geometries；
- materials；
- textures；
- render targets；
- instance buffers。

退出：

> 全部 dispose。

---

# 133. Geometry Pool

重复对象：

应：

> 共享 geometry。

不要：

每个实例：

> clone geometry。

---

# 134. Material Pool

相同视觉参数：

尽可能：

> reuse material。

---

# 135. Texture Policy

尽量：

> code / procedural texture。

如果需要 texture：

必须：

- local；
- small；
- licensed；
- bounded。

---

# 136. Shader Policy

Dreamscape：

shader：

应该：

模块化。

至少：

```text
atmosphere
terrain
flora
water
particles
postprocess
```

具体：

按实际实现。

---

# 137. Shader Uniforms

所有动态参数：

尽量：

通过：

> uniforms

更新。

不要：

频繁：

> 重建 material。

---

# 138. No Shader Recompile Per Mutation

Mutation：

不能：

每次：

重新编译：

> shader。

只更新：

> parameter。

---

# 139. Post Processing

允许：

- bloom；
- fog；
- color grading；
- subtle vignette。

但：

必须：

> 可关闭 / 降级。

---

# 140. Post Processing Budget

默认：

不要：

> 叠加五六层昂贵后处理。

---

# 141. Dreamscape Render Quality

建立：

```text
Low
Medium
High
Auto
```

或：

等价 performance mode。

---

# 142. Auto Quality

Auto：

可以：

根据：

> measured FPS / frame time

自动：

调整：

- pixel ratio；
- particle density；
- object density；
- post-processing；
- shadow quality。

---

# 143. Auto Quality Stability

不要：

```text
60
↓
30
↓
60
↓
30
```

不停震荡。

应该：

> hysteresis / cooldown。

---

# 144. FPS Targets

播放：

目标：

> 30 FPS 或更高。

高性能机器：

可以：

> 60 FPS。

低性能：

允许：

> 20–30 FPS。

不要：

牺牲：

> 音频稳定性

来追求：

> 60 FPS。

---

# 145. Idle Performance

没有播放：

Dreamscape：

应该：

降低：

> frame rate。

例如：

```text 5–15 FPS
```

或：

完全静止。

---

# 146. Minimized Window

窗口：

最小化：

必须：

> pause / throttle Dreamscape rendering。

---

# 147. Background Tab / Hidden Window

如果：

WebView：

不可见：

必须：

降低：

> render frequency。

---

# 148. Resize Performance

窗口 resize：

必须：

coalesce。

不得：

每次：

> pixel change

都重新生成整个 world。

---

# 149. DPI

测试：

```text
100%
125%
150%
175%
200%
```

Dreamscape：

必须：

> 不因高 DPI 失控。

---

# 150. Pixel Ratio Cap

必须：

设置：

合理：

```text renderer pixelRatio
```

上限。

不要：

> 4K × 200% = 巨型 render target。

---

# 151. WebGL Context Loss

必须：

处理：

```text
webglcontextlost
webglcontextrestored
```

context lost：

Dreamscape：

进入：

> fallback / paused state。

恢复：

> rebuild GPU resources。

---

# 152. Fallback

如果：

WebGL：

无法运行：

可以：

显示：

> atmospheric CSS fallback。

同时：

世界数据：

仍然有效。

---

# 153. WebGL Failure 不得损坏 World

renderer：

失败：

不能：

> 删除 seed / snapshot。

---

# 154. Telemetry

M7：

不得：

新增：

- analytics；
- usage upload；
- remote telemetry。

---

# 155. Local Diagnostics

可以：

保存：

```text
fps
frame_time
object_count
triangle_count
generation_time
region_count
```

仅：

> 本地诊断。

---

# 156. Memory Budget

建立：

> Dreamscape memory budget。

至少关注：

- CPU heap；
- GPU resource count；
- scene manifest；
- region cache；
- particles。

---

# 157. Memory Leak Audit

循环：

```text
enter
exit
enter
exit
```

至少几十次。

检查：

> memory / object count。

---

# 158. Renderer Leak Test

每次：

dispose：

必须：

- stop RAF;
- remove listeners;
- dispose geometries；
- dispose materials；
- dispose textures；
- release render targets。

---

# 159. Generation Leak Test

如果：

连续生成：

```text seed A
seed B
seed C
```

A/B：

完成后：

不得：

继续占用：

> 大量内存。

---

# 160. World Regeneration

用户点击：

`Regenerate`

必须：

> 正确 dispose old world

再加载：

> new world。

不能：

两个 world：

叠加。

---

# 161. Scene Transition

World A → World B：

不要：

硬切成：

> black frame。

可以：

使用：

> brief fade / dissolve。

---

# 162. Reduced Motion Transition

Reduced motion：

开启：

直接：

> 快速、无动画切换。

---

# 163. Environment Transition

Environment：

可以：

采用：

```text interpolate(a,b,t)
```

---

# 164. Mutation Transition

Mutation：

不是：

> instant replace。

应该：

> 渐变。

---

# 165. Mutation Safety

每个 mutation：

必须：

检查：

```text constraints
bounds
collision
performance budget
```

无效：

> discard。

---

# 166. Mutation Rollback

[B]

允许：

建立：

> rollback mutation。

如果：

新 world state：

不合法：

恢复：

> parent state。

---

# 167. World Validity

建立：

```text validate_world()
```

至少检查：

- finite coordinates；
- object count；
- bounds；
- valid enum；
- no NaN；
- no invalid IDs；
- no impossible environment values。

---

# 168. Numerical Safety

任何：

procedural 数值：

必须：

检查：

```text isFinite()
```

禁止：

- NaN；
- Infinity；
- extremely huge coordinates。

---

# 169. Coordinate Bounds

世界坐标：

必须：

有合理范围。

不要：

seed 导致：

```text x = 1e30
```

---

# 170. Deterministic Floating Point

尽量：

不要依赖：

> 不同 platform 的非确定性浮点细节。

世界 generation：

核心：

优先：

> 整数 / fixed parameter / stable algorithm。

---

# 171. Cross-Platform Determinism

目标：

Windows：

同一：

```text seed
version
environment
```

能够：

生成：

> 相同 logical world。

视觉 GPU：

允许：

> minor pixel-level rendering difference。

但：

logical manifest：

必须一致。

---

# 172. Manifest Determinism

必须：

> 比 renderer screenshot 更重要。

---

# 173. Screenshot Test Mode

建立：

```text VISUAL_TEST_MODE
```

或等价机制。

固定：

```text seed
time
environment
telemetry
quality
camera
```

---

# 174. Screenshot Determinism

相同 test mode：

生成：

> 稳定截图。

如果 GPU：

造成 minor variation：

必须：

记录：

> tolerance。

---

# 175. Unit Test — Seed

至少：

```text same string → same seed
same seed → same normalized seed
same seed → same world id
```

---

# 176. Unit Test — Genome

测试：

- bounds；
- default；
- serialization；
- deterministic generation；
- invalid values。

---

# 177. Unit Test — RNG

测试：

- named streams；
- derivation；
- reproducibility；
- independence semantics。

---

# 178. Unit Test — Biome

测试：

- deterministic selection；
- valid biome；
- all parameters bounded；
- palette valid。

---

# 179. Unit Test — Environment

测试：

- time interpolation；
- weather interpolation；
- bounds；
- serialization；
- invalid state rejection。

---

# 180. Unit Test — Mutation

至少：

```text
same world + same mutation seed → same result
mutation respects bounds
mutation never creates invalid world
mutation remains subtle
```

---

# 181. Unit Test — Rare Events

测试：

- deterministic trigger；
- event parameters；
- duration；
- replay；
- expiration。

---

# 182. Unit Test — World Validity

至少：

```text valid world
too many objects
NaN
Infinity
out of bounds
invalid biome
invalid environment
```

---

# 183. Integration Test — Generate

流程：

```text
seed
↓
generate
↓
serialize manifest
↓
deserialize
↓
hash
```

必须：

> hash 一致。

---

# 184. Integration Test — Snapshot

流程：

```text create world
↓
mutate
↓
snapshot
↓
destroy
↓
restore snapshot
```

最终：

> logical state 一致。

---

# 185. Integration Test — Replay

流程：

```text seed
↓
event 1
↓
event 2
↓
event 3
```

保存 event log。

重新：

> replay。

结果：

> 一致。

---

# 186. Integration Test — Region

至少：

```text region (0,0)
region (1,0)
region (0,1)
```

生成：

确认：

> 不越界、不重复严重冲突。

---

# 187. Renderer Test

至少：

```text initialize
load
render
resize
pause
resume
dispose
```

无：

> exception。

---

# 188. Renderer Reinitialization Test

循环：

```text initialize
dispose
initialize
dispose
```

确认：

没有：

- leaked RAF；
- duplicated listeners；
- duplicated renderer；
- GPU resource accumulation。

---

# 189. M6 Integration Test

必须验证：

```text Now Playing
↓
Open Dreamscape
↓
World loads
↓
Playback continues
↓
Aurora pauses
↓
Dreamscape runs
```

退出：

```text Dreamscape
↓
Aurora resumes
↓
Playback unchanged
```

---

# 190. Playback Continuity

打开 / 关闭 Dreamscape：

不能：

导致：

- playback pause；
- seek；
- generation change；
- queue mutation。

除非：

用户明确操作。

---

# 191. Audio Callback Isolation

Dreamscape：

不得：

进入：

> audio callback。

不允许：

- WebGL；
- Three.js；
- React；
- IPC；
- logging；

出现在：

audio callback。

---

# 192. UI Thread Safety

Procedural world generation：

如果很重：

不得：

> 阻塞 UI thread。

---

# 193. Generate Async

推荐：

```text
request world
↓
background generation
↓
manifest ready
↓
UI renderer consumes
```

---

# 194. Loading UX

Dreamscape：

加载：

应该：

> 显示一个极轻量的环境状态。

不要：

> 100% progress bar fake animation。

如果 generation 没有准确 total：

不要：

伪造百分比。

---

# 195. Generation Cancellation UI

用户：

离开 Dreamscape：

generation：

应该：

> cancel / deprioritize。

---

# 196. Queue Interaction

Dreamscape：

运行时：

用户仍然：

可以：

- pause；
- next；
- previous；
- volume。

---

# 197. Command Palette Integration

M7：

至少：

加入：

```text Open Dreamscape
Regenerate Dreamscape
Copy Dreamscape Seed
Save Dreamscape Snapshot
```

具体命令：

必须：

根据真实后端能力添加。

---

# 198. Keyboard

建议：

```text Esc
```

退出 Dreamscape。

但：

如果：

> 与 modal 冲突，

按当前 app keyboard architecture 调整。

---

# 199. Full-Screen

[B]

允许：

Dreamscape：

进入：

> full-screen immersive mode。

但：

不能：

强制全屏。

---

# 200. Full-Screen Escape

必须：

支持：

> Esc 退出。

---

# 201. Immersive UI

全屏：

可以：

隐藏：

大部分 UI chrome。

但：

必须：

有：

> 明确退出方式。

---

# 202. No Forced Fullscreen

第一次使用：

不得：

自动：

> fullscreen。

---

# 203. Dreamscape Entry Point

M6：

已经有：

Map shell。

M7：

可以：

增加：

> Dreamscape entry。

但：

不要：

让 Map 页面变成 Dreamscape。

---

# 204. Map Boundary

M7：

不得：

生成：

Music Map graph。

Dreamscape：

可以：

有：

> 自身空间。

但：

它不是：

> 音乐相似度地图。

---

# 205. M8 Compatibility

M7：

应该：

预留：

```text VisualInfluence
```

或：

其它：

未来分析扩展点。

未来：

M8 可以注入：

```text bpm
key
energy
lufs
embedding
```

但：

M7：

不得：

自己计算。

---

# 206. Analysis Injection

未来如果：

M8 提供：

```text energy
```

Dreamscape：

应该：

可以：

改变：

- atmosphere；
- density；
- lighting；
- weather tendency。

但：

M7 当前：

> 使用 null / absent analysis 时必须完整运行。

---

# 207. No Fake Analysis

当前没有：

BPM：

不要：

从：

> random / track duration

猜 BPM。

---

# 208. No Fake Key

同理：

不要：

> 随机 Key。

---

# 209. No Fake Embedding

更不能：

生成：

> random vectors

然后伪装：

> similarity / emotional space。

---

# 210. Music Identity Influence

M7：

即使：

没有 M8：

也可以：

使用：

> TrackId / world seed / playback state

影响：

Dreamscape。

---

# 211. Track-to-World Mapping

可以建立：

```text
world_seed
=
hash(
user_seed?
+
track_identity?
)
```

但：

必须：

记录：

具体：

> 是否一首歌一个 world。

不要：

擅自：

使得用户：

每换一首歌：

整个 Dreamscape：

彻底重生。

---

# 212. Recommended World Semantics

默认：

> World identity 独立于单首歌曲。

音乐：

作为：

> atmosphere input。

这样：

Dreamscape：

才能：

成为：

> 一个持续存在的空间。

---

# 213. User Seed

用户：

可以：

拥有：

> personal seed。

例如：

```text User Seed
+
World Slot
```

生成：

stable world。

---

# 214. World Slots

[B]

可以：

支持：

多个 world：

```text
World A
World B
World C
```

每个：

独立：

seed / snapshot。

---

# 215. Personal World

目标：

用户：

有：

> 一个默认世界。

而不是：

每天：

> 随机旅游。

---

# 216. Environment Persistence

Auto mode：

下次启动：

应该：

从：

> 上次 environment

继续。

---

# 217. Logical Tick

建立：

```text logical_tick
```

每个：

> mutation / event

推进：

tick。

不要：

用：

> milliseconds

作为：

唯一 world state。

---

# 218. Tick Persistence

退出：

保存：

```text logical_tick
```

重新：

恢复。

---

# 219. Evolution Bound

世界：

长期运行：

不能：

无限增长。

必须：

设置：

```text max_history
max_mutations
max_events
```

或者：

压缩历史。

---

# 220. State Compaction

[B]

可以：

把：

旧 mutation：

压缩到：

> current genome snapshot。

---

# 221. Save Format

建立：

版本化：

```json
{
  "format_version": 1,
  "seed": "...",
  "generation_version": 1,
  "world_id": "...",
  "genome": {},
  "environment": {},
  "logical_tick": 0,
  "mutation_state": {},
  "camera": {}
}
```

字段：

按真实 schema 调整。

---

# 222. Save Security

Snapshot：

不得：

包含：

- passwords；
- Authorization；
- cookies；
- source credentials；
- local absolute paths unless strictly necessary。

---

# 223. Snapshot Path

存储：

必须：

位于：

> application data directory。

不得：

写：

> music folder。

---

# 224. Corrupt Snapshot

如果：

snapshot：

损坏：

必须：

> fallback to last valid state / clean default。

不得：

> application startup crash。

---

# 225. Snapshot Atomic Write

使用：

```text
temp
↓
flush
↓
atomic replace
```

避免：

> 半写 snapshot。

---

# 226. Snapshot Backup

[B]

可以：

保留：

> 1 个 last-known-good backup。

---

# 227. Migration Test

测试：

```text version 1
↓
load
↓
upgrade
```

即使：

当前只有 v1，

也必须：

> migration architecture clear。

---

# 228. Serialization

优先：

> serde。

要求：

- deterministic；
- versioned；
- no unknown field panic。

---

# 229. Unknown Fields

未来 snapshot：

多出字段：

旧版本：

应该：

> 尽可能忽略。

---

# 230. Missing Fields

旧 snapshot：

缺少新字段：

使用：

> explicit default。

---

# 231. Documentation

更新：

`ARCHITECTURE.md`

加入：

```text
Dreamscape Architecture
Seed
Genome
RNG Streams
Biome
Environment
World Generation
Scene Manifest
Mutation
Rare Events
Persistence
Renderer
Region Generation
LOD
Performance
```

---

# 232. DECISIONS.md

至少记录：

- Dreamscape identity；
- seed format；
- seed hash；
- generation version；
- world version；
- RNG strategy；
- RNG stream isolation；
- genome；
- biome count；
- environment model；
- time model；
- weather model；
- mutation rate；
- mutation categories；
- event system；
- scene manifest；
- region system；
- generator threading；
- renderer architecture；
- instancing；
- LOD；
- object budgets；
- FPS target；
- auto quality；
- reduced motion；
- persistence；
- snapshot format；
- migration strategy。

---

# 233. PROGRESS.md

开始：

```text
M7-A = IN PROGRESS
M7-B = NOT STARTED
```

完成后：

真实记录：

- seed；
- generator；
- biome；
- environment；
- mutation；
- events；
- persistence；
- renderer；
- performance；
- tests；
- screenshots；
- known limitations。

---

# 234. `SOURCES.md`

M7：

不应增加：

> 外部网络来源。

如果完全 procedural：

记录：

> no external runtime assets。

如果使用第三方资产：

必须：

> 完整记录 license / source / attribution。

---

# 235. Content Policy

M7：

不改变：

M0：

> No commercial platform extraction。

Dreamscape：

属于：

> 本地程序生成体验。

---

# 236. Privacy

M7：

没有：

> 必要的网络行为。

不要为了：

- weather API；
- sunrise API；
- online seed service；

主动联网。

---

# 237. Weather Policy

Dreamscape：

默认：

> procedural weather。

如果未来真实天气：

另行设计。

M7：

不要联网抓现实天气。

---

# 238. No Cloud Seed

不要：

把：

Dreamscape seed：

同步到：

> online service。

---

# 239. No Telemetry

记录：

只能：

> 本地 diagnostics。

---

# 240. Dependency Audit

任何新增：

- noise crate；
- procedural generation crate；
- math crate；
- Three.js helper；
- post-processing library；

必须检查：

```text
license
maintenance
bundle size
security
Windows compatibility
CI
```

---

# 241. No Procedural Framework Bloat

如果：

简单：

Perlin / simplex / fBm

自实现：

可以：

> 使用项目已有 math / deterministic RNG。

不要：

> 为一个 50 行算法引入 5 个 dependency。

---

# 242. Dependency License Policy

继续遵守：

M0：

> Commercial-compatible license whitelist。

任何：

GPL / AGPL / LGPL：

不得：

> 偷渡进来。

---

# 243. Performance Benchmark

更新：

`docs/PERF.md`

至少记录：

```text
seed → genome
genome → world generation
manifest serialization
manifest deserialization
manifest hashing
initial renderer creation
first frame
steady-state FPS
region generation
mutation update
environment transition
snapshot save
snapshot load
```

---

# 244. Generation Benchmark

至少：

测试：

```text
small world
medium world
high quality world
```

记录：

- generation time；
- object count；
- manifest size；
- memory。

---

# 245. Rendering Benchmark

至少：

测试：

```text
Low
Medium
High
Auto
```

记录：

- FPS；
- frame time；
- draw calls；
- triangles；
- particles；
- memory。

---

# 246. Long Soak

至少：

进行：

> 数小时 Dreamscape soak。

期间：

循环：

```text
pause
resume
environment change
track change
regenerate
exit
re-enter
snapshot
restore
```

检查：

- memory growth；
- GPU resource growth；
- listener leaks；
- RAF leaks；
- logical state corruption。

---

# 247. Deterministic Soak

另外：

使用：

固定：

```text seed
virtual clock
event sequence
```

运行：

> deterministic replay。

最终：

state hash：

必须一致。

---

# 248. Renderer Soak

进入 / 退出：

循环：

> 数十到数百次。

观察：

> 是否逐渐变慢。

---

# 249. Stress World

建立：

> pathological seed fixtures。

例如：

产生：

- high density；
- large terrain scale；
- extreme weather；
- many landmarks。

确保：

> performance budgets 仍然有效。

---

# 250. Invalid Seed Stress

使用：

- empty；
- huge string；
- Unicode；
- emoji；
- malformed seed；
- whitespace；
- repeated separators。

不得：

panic。

---

# 251. Fuzz Seed Parser

至少：

对：

`parse_seed()`

进行：

> fuzz / property testing。

目标：

> arbitrary input never causes panic。

---

# 252. Fuzz Snapshot

对：

snapshot parser：

进行：

> malformed JSON / bytes tests。

不得：

panic。

---

# 253. Fuzz Manifest

manifest：

解析：

必须：

> bounded / validation-first。

---

# 254. Security Audit

必须检查：

- path traversal；
- snapshot path；
- corrupted save；
- malformed seed；
- integer overflow；
- float overflow；
- NaN；
- Infinity；
- object-count abuse；
- manifest-size abuse；
- shader uniform abuse；
- arbitrary asset path；
- local resource access；
- IPC parameter validation。

---

# 255. IPC Contract

建立：

```text
DreamscapeSnapshot
DreamscapeViewState
DreamscapeCommand
DreamscapeStatus
DreamscapeGenerationProgress
```

或等价结构。

---

# 256. Dreamscape Commands

至少：

```text
open
close
regenerate
set_environment
set_quality
save_snapshot
restore_snapshot
copy_seed
```

具体：

根据真实实现调整。

---

# 257. Command Idempotency

例如：

```text close
close
close
```

不得：

> error。

同样：

```text open
open
```

不得：

> 创建第二个 renderer。

---

# 258. Regenerate Concurrency

如果：

用户连续：

```text regenerate
regenerate
regenerate
```

必须：

> 旧 generation 可取消。

最终：

只剩：

> 最后请求的 world。

---

# 259. Stale Generation Protection

每个 generation：

应有：

```text generation_id
```

旧 generation：

完成后：

不能：

> 覆盖新 world。

---

# 260. Render Request Ordering

如果：

```text world A
↓
world B
↓
A generation completes late
```

A：

不得：

> replace B。

---

# 261. Environment Update Ordering

同样：

旧：

environment transition：

不能：

> 覆盖新 environment。

---

# 262. IPC Revision

可以：

使用：

```text revision
```

确保：

UI：

只接受：

> 最新 world state。

---

# 263. No UI Polling Loop

不要：

```text setInterval(() => invoke(...), 100ms)
```

疯狂轮询：

world。

优先：

> event-based updates / snapshot requests。

---

# 264. Low Frequency Event

Dreamscape status：

可以：

> low frequency event。

视觉：

不要：

通过 Tauri：

60FPS：

不断传输。

---

# 265. Renderer Local Loop

所有：

camera / animation / shader interpolation：

应该：

> 在 renderer 本地。

---

# 266. Environment Interpolation

backend：

提供：

```text environment target
```

renderer：

本地：

> interpolate。

---

# 267. Audio Level

同样：

backend：

提供：

> low-frequency atomic/snapshot。

renderer：

本地：

> smooth。

---

# 268. M6 Event Integration

不要：

为 M7：

重新创建：

> 第二套 event bus。

复用 M6：

> typed IPC/event bridge。

---

# 269. Store Integration

Zustand：

只存：

```text dreamscapeOpen
dreamscapeStatus
dreamscapeViewState
```

不要：

把整个：

> SceneManifest

放进：

Zustand reactive state。

---

# 270. Scene Manifest Outside React State

Scene manifest：

应该：

> imperative / ref-based。

否则：

> 大量 procedural data 会触发 React render。

---

# 271. Camera State

camera：

应该：

> renderer local。

需要持久化：

才：

提取：

> compact CameraSnapshot。

---

# 272. World State vs Render State

必须：

明确区分：

```text World State
Render State
```

World：

> deterministic logical data。

Render：

> GPU-specific state。

---

# 273. Render State Never Persisted

不得：

保存：

> raw render state。

只保存：

> logical state。

---

# 274. Error Boundary

Dreamscape：

必须：

有：

> feature-level error boundary。

如果 Dreamscape：

失败：

主：

- playback；
- Library；
- Queue

仍可用。

---

# 275. Fallback Scene

如果：

生成失败：

显示：

> procedural fallback scene。

不是：

> white screen。

---

# 276. Fallback Seed

fallback：

可以：

使用：

> fixed internal seed。

但：

必须：

不污染：

用户 world state。

---

# 277. Fallback Screenshot

测试：

必须：

有：

> renderer failure fallback test。

---

# 278. No Fake “Generating”

如果：

generation 真在：

就：

显示：

> real loading state。

如果：

早已完成：

不要：

假装：

> generating...

---

# 279. No Artificial Delay

禁止：

为了：

“梦境进入感”

加入：

> 2 秒 / 3 秒 fake delay。

---

# 280. Dreamscape Entry Animation

可以：

有：

很短：

> 120–500ms transition。

具体：

依照 reduced motion。

---

# 281. Exit Animation

同理：

> brief fade。

不能：

阻塞：

playback。

---

# 282. Visual Hierarchy

Dreamscape：

应该：

突出：

> environment。

不要：

充满：

- buttons；
- cards；
- debug labels。

---

# 283. Debug UI

开发环境：

可以：

有：

```text Seed
Biome
FPS
Objects
Version
```

但：

必须：

> production 隐藏。

---

# 284. Debug Panel

[B]

可以：

通过：

`Ctrl+Shift+D`

查看：

Dreamscape diagnostics。

不能：

默认开启。

---

# 285. Accessibility

Dreamscape：

必须：

提供：

> “Exit Dreamscape” keyboard action。

以及：

必要：

UI control labels。

---

# 286. Non-Visual Accessibility

不能：

所有信息：

都依赖：

> 颜色。

例如：

environment：

不能：

> 只有颜色变了，没有 semantic state。

---

# 287. Reduced Motion Test

必须：

自动化检查：

开启：

`reducedMotion`

后：

- camera；
- particles；
- transitions；

显著降低。

---

# 288. Quality Mode Test

至少：

Low / Medium / High：

都能：

> 正常打开世界。

---

# 289. Resize Test

至少：

```text
900 × 600
1180 × 760
1600 × 1000
```

---

# 290. High DPI Test

至少：

```text 125%
150%
200%
```

---

# 291. Multiwindow Safety

Main：

+ Mini Player：

不应该：

创建：

> duplicate Dreamscape。

Mini：

默认：

> 不显示 Dreamscape。

---

# 292. Mini Player Interaction

Dreamscape：

运行时：

Mini Player：

仍然：

可以：

- pause；
- next；
- previous。

---

# 293. Shutdown

关闭：

主窗口：

Dreamscape：

必须：

- cancel generation；
- stop render loop；
- dispose GPU；
- save required state；
- unsubscribe events。

---

# 294. Fast Shutdown

不能：

因为：

Dreamscape：

生成一个：

> 100MB scene

导致：

退出长时间卡死。

---

# 295. Background Persistence

snapshot：

应：

在：

> safe application lifecycle boundary

执行。

不要：

每 mutation：

立即写盘。

---

# 296. Write Coalescing

建议：

将：

多个：

```text environment updates
mutation events
camera changes
```

合并：

再保存。

---

# 297. Camera Persistence Frequency

不要：

每帧：

写：

camera state。

---

# 298. Autosave

[B]

可以：

做：

> low-frequency autosave。

但：

不能：

> 高频 SQLite/file write。

---

# 299. Snapshot Integrity Hash

可以：

增加：

```text checksum
```

用于：

检测：

> corrupted snapshot。

---

# 300. World Integrity

保存：

必须：

> validate before write。

读取：

必须：

> validate after parse。

---

# 301. No External Network Requirement

Dreamscape：

核心功能：

即使：

> 完全断网

也必须：

> 正常工作。

---

# 302. No Online Weather

默认：

不调用：

- Weather API；
- sunrise API；
- cloud service。

---

# 303. No Online Asset

不下载：

- textures；
- models；
- HDRI；
- fonts。

---

# 304. Build Reproducibility

生产构建：

Dreamscape：

不应该：

依赖：

> 外部网络下载运行时资产。

---

# 305. Test Offline

测试环境：

应该：

能够：

> 完全禁网运行 Dreamscape。

---

# 306. M1–M6 Regression

M7 完成后必须：

重新运行：

### M1

- decoder；
- resampler；
- ring buffer；
- seek；
- generation；
- volume；
- zero allocation。

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
- SQLite；
- FTS5；
- artwork；
- watcher；
- identity。

### M4

- local source；
- HTTP；
- HTTPS；
- ICY；
- Radio Browser；
- security；
- cache policy。

### M5

- cache persistence；
- Range；
- revalidation；
- LRU；
- pin；
- recovery；
- SessionOnly；
- Forbidden。

### M6

- Tauri shell；
- Now Playing；
- Library；
- Command Palette；
- Mini Player；
- Aurora；
- accessibility；
- UI scale；
- settings。

任何 regression：

> M7 FAIL。

---

# 307. Frontend Quality Gate

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

也必须执行。

---

# 308. Rust Quality Gate

必须：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

---

# 309. Tauri Build

当前 Windows 环境：

如果能够：

必须：

```powershell
tauri build
```

如果环境无法：

明确：

`BLOCKED`

不得：

伪造 PASS。

---

# 310. Diff Review

完成后：

先不要 commit。

执行：

```powershell
git diff --stat
git diff
```

重点检查：

- 是否偷渡 M8 analysis；
- 是否偷渡 M9 Music Map；
- 是否偷渡 M10 SMTC / tray；
- 是否偷渡 Steam；
- 是否新增 telemetry；
- 是否调用外部 weather API；
- 是否加入 remote assets；
- 是否加入商业视觉资源；
- 是否加入真实音频；
- 是否引入大型 procedural dependency；
- 是否出现 random world generation without seed；
- 是否 seed 不可复现；
- 是否每次启动重生世界；
- 是否 mutation 失控；
- 是否 object count 无上限；
- 是否 world generation 阻塞 UI；
- 是否在 React state 放巨大 manifest；
- 是否创建第二个 audio engine；
- 是否创建第二个 event bus；
- 是否 renderer leak；
- 是否 WebGL context leak；
- 是否 snapshot 无版本；
- 是否 snapshot 非原子写入；
- 是否存在 fake loading；
- 是否存在 fake analysis；
- 是否存在 TODO；
- 是否弱化测试；
- 是否 ignored tests；
- 是否提交本地 world snapshot；
- 是否提交 build artifacts。

---

# 311. Dependency Final Audit

再次：

```powershell
cargo tree
cargo deny check
```

前端：

检查：

```powershell
npm ls
```

确认：

- 没有重复 framework；
- 没有 GPL/AGPL/LGPL；
- 没有无必要大型 dependency；
- 没有重复 Three.js renderer dependency；
- 没有重复 state framework。

---

# 312. Final Visual Audit

实际运行：

至少观察：

```text
Default World
Biome A
Biome B
Biome C
Dawn
Day
Dusk
Night
Clear
Misty
Rain
Mutation
Rare Event
Regenerate
Snapshot Restore
Reduced Motion
Low Quality
High Quality
```

---

# 313. Final Interaction Audit

实际验证：

```text
Open Dreamscape
↓
World loads
↓
Play music
↓
Pause
↓
Resume
↓
Next
↓
Change environment
↓
Return
↓
Open again
↓
World persists
```

---

# 314. Final Determinism Audit

至少执行：

```text
seed A
→ generate
→ hash

seed A
→ regenerate
→ hash
```

必须：

> 一致。

再：

```text
seed A
→ snapshot
→ restore
→ hash
```

必须：

> 一致。

---

# 315. Final Mutation Audit

同一：

```text world
seed
logical tick
event sequence
```

重复：

> mutation sequence。

结果：

> 一致。

---

# 316. Final Performance Audit

记录：

```text
generation_time
first_frame_time
steady_fps
p95_frame_time
object_count
instance_count
draw_calls
triangle_count
memory
```

分别记录：

```text Low
Medium
High
Auto
```

---

# 317. Final Memory Audit

重点：

```text enter/exit × 50+
regenerate × 20+
snapshot restore × 20+
environment change × 100+
```

不得：

出现：

> 持续增长且无法解释的资源。

---

# 318. Final Security Audit

重新搜索：

```text
api_key
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

- snapshot path；
- arbitrary asset loading；
- IPC validation；
- seed parser；
- manifest validation；
- world bounds；
- object budget；
- no external network requirement。

---

# 319. Documentation Evidence

最终必须更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
SECURITY.md
PRIVACY.md
CHANGELOG.md
docs/PERF.md
```

README：

加入：

> Dreamscape 的真实能力。

不要宣称：

- unlimited worlds；
- AI generated worlds；
- real weather；
- intelligent music analysis；

除非实际存在。

---

# 320. Screenshot Evidence

至少准备：

```text
10-dreamscape-default.png
11-dreamscape-night.png
12-dreamscape-environment.png
13-dreamscape-mutation.png
14-dreamscape-seed.png
15-dreamscape-settings.png
```

如果实现：

建议增加：

```text
16-dreamscape-fullscreen.png
17-dreamscape-reduced-motion.png
18-dreamscape-map-shell-comparison.png
```

截图必须：

> 来自当前真实构建。

不得使用：

> marketing render。

---

# 321. PROGRESS.md

最终：

```text
M7-A = PASS
M7-B = PASS / NOT COMPLETED
```

记录：

- worlds；
- seeds；
- generator version；
- biome count；
- object budget；
- mutations；
- events；
- snapshot tests；
- replay tests；
- performance；
- soak；
- known limitations。

---

# 322. Final M7 Audit

输出：

# M7 Final Audit

## A. Baseline

## B. `aura-synth`

## C. Seed

## D. Genome

## E. RNG

## F. World Identity

## G. Biome

## H. Environment

## I. Terrain

## J. Object Placement

## K. Landmarks

## L. Procedural Geometry

## M. Scene Manifest

## N. Region Generation

## O. Mutation

## P. Rare Events

## Q. Replay

## R. Persistence

## S. Snapshot

## T. Migration

## U. Three.js Renderer

## V. Camera

## W. LOD

## X. Instancing

## Y. Performance

## Z. Reduced Motion

## AA. Audio Reactivity Boundary

## AB. M6 Integration

## AC. IPC

## AD. Security

## AE. Tests

## AF. Determinism

## AG. Soak

## AH. M1 Regression

## AI. M2 Regression

## AJ. M3 Regression

## AK. M4 Regression

## AL. M5 Regression

## AM. M6 Regression

## AN. Documentation

## AO. Scope Compliance

每项只能：

`PASS / FAIL / BLOCKED`

禁止：

- 基本完成；
- 差不多；
- 应该没问题；
- 理论支持；
- 后续再修；
- 大致可用。

---

# 323. 必须提供的关键证据

## Seed

必须提供：

- normalization；
- stable hash；
- deterministic output；
- versioning。

## Generation

必须提供：

- same seed → same manifest；
- manifest hash；
- bounded object count；
- pathological seed handling。

## Environment

必须提供：

- time-of-day；
- weather；
- transitions；
- bounds。

## Mutation

必须提供：

- bounded mutation；
- deterministic mutation；
- replay；
- invalid mutation rejection。

## Rare Event

必须提供：

- deterministic trigger；
- lifecycle；
- replay。

## Persistence

必须提供：

- snapshot；
- restore；
- corruption handling；
- atomic write。

## Renderer

必须提供：

- lifecycle；
- resize；
- pause；
- resume；
- dispose；
- context recovery。

## Performance

必须提供：

- generation；
- first frame；
- FPS；
- memory；
- object count；
- low/high quality。

## Integration

必须证明：

```text
M6 Player
+
M7 Dreamscape
```

可以：

> 同时工作且互不破坏。

---

# 324. Scope Compliance

最终必须明确：

### M8 没有被偷渡

无：

- BPM；
- Key；
- LUFS；
- MFCC；
- embedding。

### M9 没有被偷渡

无：

- Music Map；
- similarity graph；
- graph layout；
- spatial music mapping。

### M10 没有被偷渡

无：

- SMTC；
- tray；
- system media integration。

### M11+ 没有被偷渡

无：

- Steam；
- installer；
- updater；
- cloud；
- accounts；
- telemetry。

---

# 325. M7-A 完成门槛

必须同时满足：

1. `aura-synth` 正式可用。
2. Seed deterministic。
3. World deterministic。
4. Generation version 正常工作。
5. 至少 3 个稳定 biome。
6. Environment state 正常。
7. Procedural scene manifest 正常。
8. Mutation 正常。
9. Rare-event framework 正常。
10. Snapshot 正常。
11. Restore 正常。
12. Replay 正常。
13. Three.js renderer 正常。
14. Dreamscape 可从 M6 打开。
15. Playback 不受影响。
16. Aurora 与 Dreamscape 生命周期正确切换。
17. Performance 有真实数据。
18. reduced motion 正常。
19. WebGL fallback 正常。
20. frontend gate PASS。
21. Rust gate PASS。
22. M1–M6 regression PASS。
23. security PASS。
24. documentation PASS。

---

# 326. M7-B

如果 B 级完成：

必须：

> 单独列出。

例如：

```text
Multiple Worlds = PASS
Fullscreen = PASS
Rare Events = PASS
World History = PASS
Advanced Mutation = PASS
```

未完成：

明确：

`NOT COMPLETED`

---

# 327. Git Commit

只有：

- M7-A 全部 PASS；
- M7-B 若声明完成则 PASS；
- M1 regression PASS；
- M2 regression PASS；
- M3 regression PASS；
- M4 regression PASS；
- M5 regression PASS；
- M6 regression PASS；
- frontend gate PASS；
- Rust gate PASS；
- security PASS；
- deterministic tests PASS；
- performance PASS；
- soak PASS；
- final audit PASS；
- diff review 干净；

才允许：

```powershell
git add .
git commit -m "M7: implement Dreamscape"
```

**不要 push。**

---

# 328. Commit Hygiene

commit：

不得包含：

- local world snapshots；
- local settings；
- cache；
- user music；
- private seed files if treated as personal data；
- target；
- node_modules；
- dist；
- build artifacts；
- generated local screenshots；
- diagnostics dumps；

除非：

该文件：

> 明确属于正式项目源码 / fixture。

---

# 329. Final Output

完成后严格输出：

```text
# M7 Final Audit

A. Baseline
B. aura-synth
C. Seed
D. Genome
E. RNG
F. World
G. Biome
H. Environment
I. Terrain
J. Procedural Objects
K. Scene Manifest
L. Region Generation
M. Mutation
N. Rare Events
O. Replay
P. Snapshot
Q. Renderer
R. Performance
S. Persistence
T. IPC
U. M6 Integration
V. Security
W. Tests
X. Determinism
Y. Soak
Z. Regression
AA. Documentation
AB. Scope

M7-A = PASS / BLOCKED
M7-B = PASS / NOT COMPLETED
COMMIT = <hash>
NEXT = M8
```

---

# 330. 最终停止条件

如果：

```text
M7-A = PASS
```

并且：

`M7-B` 已声明的项目全部通过：

立即：

> 停止。

不要开始：

- M8；
- Audio Analysis；
- BPM；
- Key；
- LUFS；
- MFCC；
- Embedding；
- Music Map。

下一阶段只能由我单独下达：

> **M8 提示词。**