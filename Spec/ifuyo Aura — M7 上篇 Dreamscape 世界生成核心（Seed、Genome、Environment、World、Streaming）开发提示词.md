# ifuyo Aura
# M7 — Dreamscape / Procedural World / Seed / Environment / Mutation
> 本文档为原 M7 开发提示词的**上篇（世界生成核心，§0–127）**。
> 下篇《ifuyo Aura — M7 下篇 渲染、集成、测试与发布门禁开发提示词.md》承载 §128–330。
> 章节编号沿用原 M7 全局编号；跨篇引用（如“见 #132”）按编号在两篇中查找。


你现在进入 ifuyo Aura 项目的 **M7：Dreamscape 程序化梦境系统**。

项目正式名称：

`ifuyo Aura`

仓库 / 项目名称：

`ifuyo-aura`

品牌：

`ifuyo · 伊芙游`

---

# 0. 当前阶段目标

在已经完成：

```text
Phase 0
M1 Audio Core
M2 Playback Quality
M3 Music Library
M4 Source System
M5 Persistent Cache
M6 Desktop UI / Now Playing / Library / Aurora
```

的基础上，正式实现：

> **Dreamscape：一个由代码和 seed 驱动的、可确定性复现、可持续演化、具有环境状态与轻微随机事件的程序化梦境世界。**

Dreamscape 不是：

- 音频播放器；
- EQ；
- 音乐分析器；
- 音乐地图；
- 普通 WebGL 可视化背景；
- 一个静态 demo；
- 一组随机粒子。

它应该成为：

> **ifuyo Aura 的第二层体验：用户播放音乐时，可以进入一个由自己的音乐与 seed 共同孕育出的独特、安静、可长期存在的数字空间。**

---

# 1. 本阶段绝对边界

## A 级必须实现

1. `aura-synth` 正式建立或补全。
2. Dreamscape domain model。
3. deterministic seed system。
4. seed normalization。
5. seed hashing。
6. world generation。
7. biome / environment abstraction。
8. environment state。
9. deterministic procedural assets。
10. procedural scene manifest。
11. world topology。
12. object placement rules。
13. procedural variation。
14. mutation system基础。
15. rare-event framework。
16. environment transition。
17. time-of-day model。
18. weather / atmosphere model。
19. player/session persistence。
20. seed export/import。
21. stable world identity。
22. generation version。
23. deterministic replay。
24. Rust → UI typed Dreamscape DTO。
25. Three.js Dreamscape renderer。
26. renderer lifecycle。
27. camera/navigation。
28. LOD / density control。
29. FPS throttling。
30. resource disposal。
31. WebGL context recovery。
32. reduced-motion。
33. deterministic screenshot mode。
34. error fallback。
35. M6 integration。
36. basic playback telemetry integration。
37. M1–M6 regression。
38. complete deterministic tests。
39. performance benchmarks。
40. documentation / decision record。

---

## B 级

A 级稳定后，可以实现：

1. 多 biome。
2. 更复杂 environment transition。
3. 稀有视觉事件。
4. 世界中的 persistent landmarks。
5. seed bookmark。
6. world snapshot。
7. Dreamscape preset。
8. user-customized atmosphere。
9. richer camera modes。
10. procedural weather variation。
11. background simulation。
12. world history / evolution log。
13. 更丰富 mutation。
14. idle world evolution。
15. Dreamscape full-screen mode。

B 级不得破坏 A 级。

---

# 2. 本阶段明确禁止

不要实现：

- BPM analysis；
- Key detection；
- LUFS analysis；
- MFCC；
- embedding；
- recommendation；
- Music Map；
- graph relationship；
- graph layout；
- SMTC；
- system media transport；
- tray；
- installer；
- updater；
- Steam；
- online lyrics；
- online artwork；
- online metadata；
- cloud sync；
- account system；
- telemetry；
- social features；
- commercial-platform extraction；
- new Source provider；
- new cache mechanism；
- new codec。

特别注意：

> **M8 才负责真正的音频分析。**

因此本阶段不能为了让 Dreamscape “更智能”而提前实现：

```text
BPM
Key
LUFS
MFCC
Embedding
```

---

# 3. M7 与 M6 Aurora 的边界

M6 已经存在：

> Aurora。

Aurora 是：

> Now Playing 页面中的轻量、实时、音乐播放响应型背景视觉。

Dreamscape 是：

> 一个独立的、具有世界状态、seed、环境、时间、变异与持久性的程序化世界。

因此：

```text
Aurora
=
Reactive Visual Background
```

而：

```text
Dreamscape
=
Persistent Procedural World
```

不得把 Dreamscape：

> 简化成“更复杂一点的 Aurora”。

---

# 4. Dreamscape 产品体验目标

用户进入 Dreamscape：

应该感觉：

> “这是一个我可以偶尔回来看看、慢慢发现变化的地方。”

而不是：

> “这是一个一直在疯狂播放特效的 screensaver。”

核心体验：

```text
宁静
微妙
持续
独特
可发现
可复现
略带未知
```

---

# 5. 不要做成游戏

Dreamscape：

不是：

- RPG；
- idle game；
- survival；
- farming；
- achievement grind；
- score system；
- daily quest；
- clicker。

禁止：

- HP；
- XP；
- coins；
- stamina；
- loot；
- battle；
- quest；
- reward loop。

---

# 6. 不要做成随机屏保

最重要的区别：

```text
Random
```

不等于：

```text
Procedural
```

每一个世界：

必须有：

> stable identity。

同一个：

```text seed
+
generation version
+
environment state
```

必须：

> 可以重新构建出同一个基础世界。

---

# 7. `aura-synth`

建立：

```text
crates/aura-synth
```

职责：

```text
Seed
 ↓
Genome / Parameters
 ↓
Environment
 ↓
Procedural Generation
 ↓
Scene Manifest
```

不得负责：

- Three.js；
- WebGL；
- React；
- Tauri；
- audio callback；
- SQLite；
- HTTP。

---

# 8. Rust / Frontend 分工

推荐：

```text
Rust
  ↓
Dreamscape Domain
  ↓
World Definition / Scene Manifest
  ↓
Typed IPC
  ↓
Three.js
  ↓
GPU Rendering
```

Rust：

负责：

> 世界是什么。

Three.js：

负责：

> 世界怎么被看见。

---

# 9. 不要把生成算法全部放 TS

Dreamscape 的：

- seed；
- world generation；
- deterministic mutation；
- environment state；

应主要位于：

> Rust domain / synth layer。

这样：

> 同一个 seed 可以跨平台保持一致。

---

# 10. 但不要把 renderer 放 Rust

Three.js：

继续负责：

- camera；
- GPU；
- materials；
- meshes；
- lighting；
- shaders；
- render loop。

不要：

因为想“统一”：

把 WebGL renderer 塞进 Rust。

---

# 11. Dreamscape Domain Model

至少建立：

```text
DreamscapeWorld
DreamscapeSeed
DreamscapeGeneration
DreamscapeEnvironment
DreamscapeBiome
DreamscapeObject
DreamscapeEvent
DreamscapeSnapshot
DreamscapeMutation
```

具体命名：

可根据真实代码调整。

---

# 12. Seed

Seed：

必须：

- deterministic；
- serializable；
- human-copyable；
- stable；
- version-aware。

可以使用：

```text
u64
```

或：

更长的结构化 seed。

具体方案：

记录：

`DECISIONS.md`

---

# 13. Seed String

推荐：

用户最终看到的 seed：

不是：

> 一串超长十六进制 hash。

可以建立：

```text
DS-XXXX-XXXX-XXXX
```

等：

人类可复制的表示。

内部：

仍保存：

完整 deterministic seed。

---

# 14. Seed Normalization

用户输入：

```text
hello
```

或：

```text
12345
```

或：

```text
DS-....
```

都应该：

先：

> normalize。

然后：

生成：

稳定内部 seed。

---

# 15. Seed Hash

如果 seed 来源：

字符串：

可以：

> hash → fixed-width seed。

禁止：

依赖：

Rust `DefaultHasher`

如果其跨版本稳定性没有得到保证。

应使用：

> 明确固定的 deterministic hash。

---

# 16. Seed Version

必须保存：

```text
seed_version
generation_version
```

例如：

```text
seed_version = 1
generation_version = 1
```

这是非常重要的。

---

# 17. 为什么要 generation version

以后：

算法发生变化：

同一个 seed：

可能：

生成不同世界。

为了：

保持旧世界可追溯：

必须：

```text
seed
+
generation version
```

共同定义：

> 世界版本。

---

# 18. Stable World Identity

建立：

```text
world_id
```

可以：

由：

```text
hash(
seed
+
generation_version
+
initial_environment
)
```

生成。

同一 world：

必须：

> 永久稳定。

---

# 19. Deterministic RNG

不要使用：

> global random state。

建议：

多个命名 RNG stream：

```text
world_rng
terrain_rng
vegetation_rng
weather_rng
event_rng
mutation_rng
```

或者等价机制。

---

# 20. RNG Stream Isolation

要求：

改变：

```text vegetation density
```

不应该导致：

> terrain 完全换样。

因此：

不同系统：

使用：

> 独立 deterministic RNG streams。

---

# 21. RNG Seed Derivation

例如：

```text
master_seed
 ↓
hash(master_seed + "terrain")
hash(master_seed + "vegetation")
hash(master_seed + "weather")
hash(master_seed + "events")
```

形成：

稳定子 seed。

不得：

简单：

```text master_seed + index
```

到处乱用。

---

# 22. RNG 测试

必须测试：

相同：

```text seed
generation
environment
```

输出：

> bit / value deterministic。

不同 seed：

不保证：

> 一定完全不同。

但：

应具备：

合理变化。

---

# 23. Genome

建立：

> Dreamscape Genome。

Genome：

不是 DNA simulation。

它是：

> 控制世界基础形态的参数集合。

例如：

```text
terrain_scale
flora_density
mist_density
light_softness
water_presence
color_temperature
object_scale_bias
horizon_height
```

具体字段：

根据实际设计决定。

---

# 24. Genome 与 Seed

要求：

```text seed
↓
genome
↓
world
```

Genome：

必须：

可序列化。

这样：

未来可以：

> 对 world snapshot 进行调试。

---

# 25. Genome Bounds

所有 genome parameter：

必须：

有：

- min；
- max；
- default；
- version。

禁止：

> 参数无限增长。

---

# 26. Genome Mutation

Mutation：

不能：

```text random everything
```

应该：

> 对小范围 parameter 做受控变化。

例如：

```text flora_density +3%
mist_density -1%
color_temperature +2%
```

---

# 27. Mutation Rate

必须：

明确：

> mutation 发生概率 / 触发条件。

不能：

> 每帧 mutation。

---

# 28. Mutation 时间尺度

Dreamscape：

强调：

> 慢变化。

默认：

mutation：

应该发生在：

- 新 session；
- 长时间 idle；
- 特定 world event；
- 明确触发；

而不是：

> 每 5 秒随机一次。

---

# 29. Mutation 可复现

如果：

world：

发生 mutation：

必须：

能够：

> 记录 mutation event。

例如：

```text
mutation_id
source_seed
parent_state
mutation_delta
timestamp / logical_tick
```

---

# 30. 不要用真实 wall-clock 作为 deterministic world logic

世界逻辑：

必须支持：

```text VirtualTime
LogicalTick
```

生产：

可以：

映射到：

real time。

测试：

使用：

virtual clock。

---

# 31. Environment

建立：

```text
EnvironmentState
```

至少考虑：

```text
time_of_day
weather
humidity
temperature
wind
fog
light
season
```

但：

第一版不要参数爆炸。

---

# 32. Environment 参数范围

所有环境参数：

必须：

明确：

```text min
max
default
```

并：

> deterministic。

---

# 33. Time of Day

至少支持：

```text Dawn
Day
Dusk
Night
```

不需要：

一开始：

> 每分钟创建不同 state。

可以：

使用：

连续：

```text time_phase ∈ [0,1)
```

由：

renderer：

做平滑插值。

---

# 34. Time Transition

时间变化：

必须：

平滑。

禁止：

```text Day → Night
```

突然：

> 整个场景变黑。

---

# 35. Weather

第一版至少：

```text Clear
Misty
Drizzle
Rain
Overcast
```

具体数量：

可以调整。

---

# 36. Weather 不要变成灾害系统

禁止：

- storms；
- lightning spam；
- tornado；
- flooding；
- destructive simulation。

目标：

> 氛围，而不是灾难。

---

# 37. Weather Transition

天气状态：

需要：

> transition。

例如：

```text Clear → Misty
```

平滑：

```text 0% → 100%
```

---

# 38. Wind

Wind：

应影响：

- foliage movement；
- mist；
- particle drift；

但：

不要：

> 大规模物理模拟。

---

# 39. Fog

Fog：

主要用于：

> depth / atmosphere。

不要：

把所有场景：

> 变成看不清的灰雾。

---

# 40. Light

建立：

> abstract light state。

例如：

```text sun_intensity
ambient_intensity
horizon_glow
```

最终：

renderer：

把它们：

> 映射到 Three.js。

---

# 41. Palette

World palette：

必须：

由：

```text seed
+
biome
+
environment
```

决定。

但：

不能：

随机到：

> 极端难看的颜色组合。

---

# 42. Palette Constraints

必须：

限制：

- saturation；
- luminance range；
- contrast；
- accent frequency。

目标：

> 长时间观看不会疲劳。

---

# 43. Biome

第一版：

至少：

> 3–5 种 biome archetypes。

例如：

```text
Quiet Meadow
Misty Forest
Dream Shore
Moon Garden
Amber Valley
```

名字可自行设计。

---

# 44. Biome Archetype

Biome：

不要：

只是一套颜色。

必须至少定义：

```text
terrain profile
object family
density profile
atmosphere
palette
lighting tendency
```

---

# 45. Biome Selection

Biome：

必须：

由：

> seed → genome

决定。

不能：

每次打开：

> 随机换 biome。

---

# 46. Biome Variation

同一 biome：

不同 seed：

应：

产生：

> 不同结构。

例如：

同样是：

`Misty Forest`

A：

树更密。

B：

雾更重。

C：

地形更开阔。

---

# 47. World Topology

不要：

直接放：

> 一万个随机 object。

需要：

先建立：

> 空间结构。

例如：

```text
world
 ├─ terrain
 ├─ horizon
 ├─ landmarks
 ├─ object zones
 └─ atmospheric zones
```

---

# 48. Terrain

地形：

可以：

由：

- low-frequency noise；
- spline；
- height field；
- analytic functions；

等生成。

第一版：

优先：

> 计算成本可控。

---

# 49. Terrain Determinism

相同：

```text seed
terrain parameters
```

必须：

生成：

> 相同 terrain data。

---

# 50. Terrain Bounds

世界：

必须：

有：

> finite logical bounds。

禁止：

第一版做无限开放世界。

---

# 51. Horizon

Horizon：

可以：

作为：

> illusion of depth。

不要求：

真实无限世界。

---

# 52. Object Placement

对象：

必须：

经过：

> placement rules。

例如：

```text distance from path
slope tolerance
density
exclusion radius
biome affinity
```

---

# 53. No Object Overlap

至少主要 landmark：

必须：

避免：

明显：

- 穿模；
- 重叠；
- 悬空；
- 互相嵌入。

---

# 54. Spatial Hash

如果 object 数量较大：

可以：

使用：

> spatial hash / grid。

避免：

O(N²)

碰撞检查。

---

# 55. Landmark

世界：

应该：

存在：

> 少量可识别 landmark。

例如：

- old tree；
- stone;
- water pool；
- arch;
- glowing flower cluster。

---

# 56. Landmark Identity

每个 landmark：

可以拥有：

```text
landmark_id
```

由：

world seed + placement index

确定。

这样：

以后：

> snapshot / mutation

可以稳定引用。

---

# 57. Procedural Object Families

至少定义：

```text
flora
stone
water
light
particle
landmark
```

具体 family：

由 biome 决定。

---

# 58. Procedural Geometry

M7：

优先：

> pure code geometry。

例如：

- primitive composition；
- instancing；
- spline;
- generated vertices；
- shader deformation。

---

# 59. External Assets

默认：

禁止：

- 外部 3D model；
- commercial texture；
- remote asset；
- random downloaded package。

Dreamscape：

应：

> 尽量实现“无外部模型依赖”。

---

# 60. Asset License

如果确实需要任何：

第三方视觉资产：

必须：

在开发前：

检查：

- license；
- redistribution；
- commercial usage；
- repository compatibility。

记录：

`THIRD_PARTY_LICENSES.md`

---

# 61. Instancing

大量重复对象：

优先：

使用：

> InstancedMesh / GPU instancing

或等价机制。

禁止：

> 每棵树一个独立 renderer object

导致数千 draw calls。

---

# 62. LOD

至少：

根据距离：

改变：

- geometry complexity；
- particle density；
- shader cost。

---

# 63. Object Budget

必须：

定义：

```text max_objects
max_landmarks
max_particles
max_instances
```

不能：

> seed 越极端，object 数量无限上升。

---

# 64. Density Budget

所有：

```text density
```

必须：

受到：

> global performance budget

约束。

---

# 65. Camera

Dreamscape：

至少支持：

```text passive camera
guided camera
```

或者等价两种状态。

---

# 66. Passive Camera

默认：

用户：

不需要操作。

camera：

缓慢：

- orbit；
- pan；
- breathing motion。

---

# 67. Guided Camera

可让：

用户：

轻微：

> 调整视角。

不要：

第一版做：

自由飞行器。

---

# 68. Camera Bounds

camera：

必须：

限制：

> 不穿越世界底部 / 地形 / clipping plane。

---

# 69. Camera Motion

目标：

> 极慢。

禁止：

- rapid zoom；
- shake；
- spin；
- aggressive transitions。

---

# 70. Reduced Motion

开启：

`reduced motion`

Dreamscape：

必须：

降低：

- camera motion；
- wind；
- particles；
- environment pulses；
- object sway。

---

# 71. Audio Reactivity

M7：

允许：

响应：

M6 basic telemetry：

```text
is_playing
audio_level
volume
transition_progress
```

---

# 72. Audio Reactivity Boundary

M7：

不得：

自行实现：

- FFT；
- BPM；
- beat detection；
- key detection。

---

# 73. Audio Level Mapping

audio level：

只应该：

轻微影响：

- light pulse；
- mist movement；
- particle intensity；
- camera breathing。

不要：

> 音量一大，整个世界剧烈爆炸。

---

# 74. Paused World

音乐暂停：

Dreamscape：

不应：

彻底冻结。

可以：

进入：

> slow ambient idle state。

但：

motion：

显著降低。

---

# 75. Stopped World

停止播放：

可以：

回到：

> idle atmosphere。

不要：

立即销毁世界。

---

# 76. Track Change

切换歌曲：

不应：

重新生成：

> 整个 world。

默认：

world：

保持。

允许：

轻微：

- palette drift；
- light change；
- event；
- atmosphere change。

---

# 77. World Persistence

用户今天看到：

某个 world。

明天回来：

默认：

> 仍然是同一个 world。

不能：

每次启动：

> 完全重生。

---

# 78. Session State

至少保存：

```text
world_id
seed
generation_version
environment
mutation state
camera state
```

具体：

根据产品需求调整。

---

# 79. Snapshot

建立：

```text
DreamscapeSnapshot
```

能够：

保存：

> 当前世界的关键 state。

---

# 80. Snapshot 不保存 GPU 对象

不要：

序列化：

- WebGL textures；
- Mesh；
- renderer；
- Three.js scene。

只保存：

> logical state。

---

# 81. Restore

snapshot：

恢复后：

必须：

重新：

```text logical state
 ↓
procedural scene
 ↓
renderer
```

---

# 82. World Evolution

Dreamscape：

可以：

长期：

发生：

> subtle evolution。

例如：

- 一个区域更茂盛；
- 雾更轻；
- 某 landmark 出现；
- 某种颜色比例变化。

---

# 83. Evolution 不要毁掉原世界

Mutation：

必须：

> 小步。

默认：

不应该：

> 今天森林，明天沙漠。

---

# 84. Mutation Categories

至少可以：

```text
Growth
Atmosphere
Color
Placement
Rare Phenomenon
```

---

# 85. Growth

例如：

```text
flora density + small delta
```

---

# 86. Atmosphere

例如：

```text
mist density
wind intensity
ambient light
```

---

# 87. Color

例如：

```text
warmth
accent hue
horizon glow
```

变化：

必须：

> subtle。

---

# 88. Placement Mutation

可以：

增加：

> 少量新对象 / landmark。

但：

不得：

每次 mutation：

大规模重建整个世界。

---

# 89. Rare Phenomenon

建立：

> rare event framework。

例如：

- luminous bloom；
- drifting lights；
- unusual mist；
- meteor-like slow streak；
- firefly field。

具体事件：

可以自己设计。

---

# 90. Rare Event Rate

稀有事件：

必须：

> 真正稀有。

禁止：

每分钟触发一次“彩蛋”。

---

# 91. Rare Event Determinism

事件：

如果：

由：

```text world_id
logical_tick
event_stream
```

决定：

必须：

可重放。

---

# 92. Event Replay

测试：

记录：

```text event log
```

再次：

replay：

必须：

得到：

> 相同 logical state。

---

# 93. Event Log

至少：

```text event_id
event_type
logical_tick
parameters
```

不需要：

记录：

每帧视觉位置。

---

# 94. Event Expiration

稀有事件：

必须：

有：

```text duration
fade_in
active
fade_out
```

防止：

事件：

> 永久积累。

---

# 95. World History

[B]

可以：

记录：

```text
created
mutated
event_seen
snapshot
```

作为：

> 本地 world history。

---

# 96. No Reward

Event：

不奖励：

- XP；
- points；
- coins。

它的价值：

就是：

> 被发现。

---

# 97. Discovery

用户发现：

稀有变化：

可以：

通过：

> 视觉本身。

不需要：

> “恭喜你获得史诗彩蛋！”

---

# 98. No Notification Spam

Dreamscape：

禁止：

发送：

- OS notifications；
- toast spam；
- reminders。

---

# 99. World Time

逻辑时间：

应该：

与：

> 用户现实时间

存在：

合理映射。

但：

不能：

让用户凌晨 3 点：

看到极端刺眼效果。

---

# 100. User Override

[B]

允许：

用户：

手动选择：

- time of day；
- weather；
- biome preset。

但：

默认：

仍然自然变化。

---

# 101. Environment Mode

可以：

建立：

```text
Auto
Manual
Frozen
```

其中：

`Frozen`

表示：

> 环境状态不自动演化。

---

# 102. World Seed UI

M6：

已经有：

Command Palette。

M7：

可以增加：

```text
Open Dreamscape
Copy Seed
Regenerate
Save Snapshot
Restore Snapshot
```

---

# 103. Regenerate

用户：

主动：

`Regenerate`

才：

允许：

> 用新 seed / 新 world。

不能：

每次启动：

自动 regenerate。

---

# 104. Seed Copy

必须：

> 一键复制 seed。

复制：

必须：

> 只复制安全 seed。

不要：

包含：

filesystem paths；

credentials。

---

# 105. Seed Import

允许：

用户：

粘贴 seed。

解析失败：

显示：

> Invalid Dreamscape seed。

不能：

crash。

---

# 106. Seed Compatibility

旧 seed：

如果：

generation version：

仍支持：

应该：

> 继续打开。

如果不再支持：

必须：

显示：

> World version unavailable

而不是：

> 静默改变世界。

---

# 107. World Migration

如果未来：

generation version：

升级。

M7：

至少：

建立：

```text migration interface
```

但：

不要：

现在写十套未来 migration。

---

# 108. Versioned Generator

建议：

```text GeneratorV1
```

未来：

```text GeneratorV2
```

这样：

旧世界：

仍然能够：

> 使用旧生成规则。

---

# 109. Generator Contract

Generator：

输入：

```text Seed
GenerationVersion
Genome
Environment
```

输出：

```text SceneManifest
```

---

# 110. Scene Manifest

SceneManifest：

应该：

尽量是：

> renderer-friendly but domain-neutral。

例如：

```text
nodes
transforms
materials
instances
lights
effects
landmarks
```

具体字段：

根据 renderer 需要设计。

---

# 111. 不要把 Three.js 类型泄漏进 Rust

Rust：

禁止：

出现：

```text THREE.Vector3
THREE.Mesh
```

等概念。

---

# 112. Manifest Serialization

必须：

能够：

```text generate
serialize
deserialize
hash
```

用于：

deterministic testing。

---

# 113. Manifest Hash

可以：

建立：

```text scene_hash
```

用于：

测试：

同 seed：

> 输出一致。

---

# 114. Scene Hash Test

相同：

```text seed
version
genome
environment
```

必须：

```text scene_hash(A) == scene_hash(B)
```

---

# 115. Manifest Size

必须：

防止：

> seed 导致数 MB / 数十 MB 的巨大 manifest。

设置：

> bounded object count。

---

# 116. Streaming Generation

如果：

世界较大：

不要：

一次：

生成无限多内容。

可以：

> chunk / region based generation。

---

# 117. Region

定义：

```text RegionCoord
```

例如：

```text x
z
```

或：

其它空间坐标。

---

# 118. Region Seed

每个 region：

可以：

由：

```text world_seed
+
region_coord
```

派生：

稳定 seed。

这样：

不同区域：

可以：

独立生成。

---

# 119. Region Determinism

相同：

```text world
+
region
```

必须：

> 相同。

---

# 120. Region Streaming

camera：

接近新区域：

可以：

> 异步生成。

但：

生成：

不得：

阻塞 UI frame。

---

# 121. Generation Thread

Procedural generation：

允许：

在：

> worker / Rust background task

中执行。

---

# 122. Renderer Thread

Three.js：

继续：

> 在 UI / browser rendering environment

中执行。

不要：

后台线程：

直接：

> 操作 WebGL objects。

---

# 123. Async Generation

生成：

必须：

支持：

```text pending
ready
failed
cancelled
```

---

# 124. Generation Cancellation

用户：

关闭 Dreamscape：

正在生成：

任务必须：

> 可以取消。

---

# 125. Region Cache

[B]

可以：

对已经生成：

region：

进行：

> memory cache。

但：

必须：

> bounded。

---

# 126. Do Not Confuse With M5 Cache

Dreamscape region cache：

是：

> procedural generation cache。

不是：

> network cache。

不要：

写入：

M5 cache database。

---

# 127. Persistence Boundary

如果要持久化：

Dreamscape：

只保存：

> world snapshot / logical state。

不要：

把：

整个 procedural mesh cache：

写进用户数据。

---

