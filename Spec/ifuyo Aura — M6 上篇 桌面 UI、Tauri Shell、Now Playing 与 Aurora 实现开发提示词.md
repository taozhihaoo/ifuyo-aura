# ifuyo Aura
# M6 — Desktop UI Foundation / Tauri Shell / Now Playing / Library / Command Palette / Aurora
> 本文档为原 M6 开发提示词的**上篇（实现篇，§0–171）**。
> 下篇《ifuyo Aura — M6 下篇 测试、安全审计与发布门禁开发提示词.md》承载 §172–318。
> 章节编号沿用原 M6 全局编号；跨篇引用（如“见 #132”）按编号在两篇中查找。


你现在进入 ifuyo Aura 项目的 **M6：桌面 UI 基础、Tauri Shell、Now Playing、Library UI、Command Palette、Mini Player 与 Aurora 视觉系统**。

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

本阶段第一次把：

```text
M1 Audio Core
M2 Playback Quality
M3 Music Library
M4 Source System
M5 Persistent Cache
```

正式接入：

```text
Tauri 2
+
React
+
TypeScript
+
Vite
+
Zustand
+
Three.js
```

形成真正可长期使用的桌面产品。

本阶段核心目标：

> **不是做一个“能打开的前端壳”，而是完成一个真正可以每天使用的 ifuyo Aura 第一版桌面产品界面。**

必须至少形成：

```text
Tauri Desktop Shell
        ↓
Application / IPC Layer
        ↓
Rust Services
        ↓
M1 / M2 / M3 / M4 / M5
```

以及：

```text
Main Window
 ├─ Now Playing
 ├─ Library
 └─ Map Shell

Global
 ├─ Command Palette
 ├─ Mini Player
 └─ Playback / Queue / Settings
```

---

# 1. 本阶段绝对边界

## A 级必须实现

1. Tauri 2 desktop shell。
2. React + TypeScript application shell。
3. typed IPC contract。
4. Rust → UI state/event bridge。
5. UI global state architecture。
6. application state 与 UI state 分离。
7. Windows 11 Mica。
8. Windows 10 solid fallback。
9. frameless window。
10. custom title bar。
11. window controls。
12. main navigation。
13. Now Playing page。
14. Library page。
15. Map page shell。
16. global Command Palette。
17. Mini Player window。
18. playback controls。
19. queue UI。
20. real library search。
21. real library pagination / virtualization。
22. real artwork display。
23. real playback state display。
24. M2 playback settings connection。
25. UI scale 90%–140%。
26. zh-CN / en。
27. keyboard navigation。
28. visible focus state。
29. reduced-motion。
30. 120–240ms UI transition guideline。
31. Aurora visual system。
32. Aurora performance throttling。
33. structured loading / empty / error states。
34. basic local settings persistence。
35. command registration architecture。
36. frontend automated tests。
37. Tauri smoke tests。
38. accessibility baseline。
39. security / capability review。
40. M1–M5 full regression。

---

## B 级

如果 A 级稳定后：

1. 更完整的 queue interaction。
2. queue reorder。
3. drag & drop。
4. richer EQ UI。
5. richer source input UI。
6. richer cache diagnostics。
7. advanced Aurora presets。
8. mini-player polish。
9. window state restoration。
10. visual regression suite。

B 级不得破坏 A 级。

---

# 2. 本阶段明确禁止

不要实现：

- Dreamscape；
- Dreamscape seed system；
- Dreamscape synthesis；
- Dreamscape mutation；
- BPM；
- Key；
- LUFS analysis；
- MFCC；
- embedding；
- Music Map engine；
- graph generation；
- map layout algorithm；
- map relationship calculation；
- SMTC；
- system media transport integration；
- tray；
- installer；
- updater；
- Steam；
- online lyrics；
- online artwork；
- online metadata enrichment；
- new Source provider；
- new cache mechanism；
-新的 codec；
- cloud sync；
- account system；
- telemetry；
- analytics upload。

特别注意：

> M6 可以显示未来功能入口，但不能伪造未来数据。

例如：

BPM / Key / Energy：

如果 M8 尚未产生：

> UI 必须显示 unavailable / not analyzed / hidden。

不得显示随机值、默认值、占位值。

---

# 3. M6 的产品定义

M6 完成后：

用户应该能够：

```text
启动 ifuyo Aura
↓
看到正式桌面窗口
↓
浏览自己的音乐库
↓
搜索歌曲
↓
点击歌曲播放
↓
看到真实 Now Playing
↓
暂停 / 继续
↓
拖动进度
↓
调节音量
↓
打开队列
↓
切换 EQ / Loudness / Crossfade
↓
看到 Aurora
↓
打开 Ctrl+K
↓
执行常用命令
↓
切换 Mini Player
```

并且：

> 这些行为真正调用 M1–M5 的能力。

不能建立：

> “为了演示而做的一套 fake player”。

---

# 4. 开始前：Baseline Audit

在任何修改前：

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
npm install
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
src-tauri
ui
config
docs
```

确认：

- Phase 0；
- M1；
- M2；
- M3；
- M4；
- M5；
- 当前 HEAD；
- 当前 workspace；
- 当前 Tauri 版本；
- 当前 React；
- 当前 TypeScript；
- 当前 Vite；
- 当前 Three.js；
- 当前 Zustand；
- 当前测试数量；
- 当前 warnings；
- 当前 CI；
- 当前 capabilities；
- 当前 CSP；
- 当前 UI scaffold。

输出：

# M6 Baseline Audit

A. Git  
B. M1 Status  
C. M2 Status  
D. M3 Status  
E. M4 Status  
F. M5 Status  
G. Tauri  
H. Frontend  
I. Existing UI  
J. Dependencies  
K. Security  
L. Risks

---

# 5. 前序阶段不合格怎么办

如果 M1–M5 某项存在：

> 真正阻塞 M6 的缺陷

只做：

> 最小必要修复。

不得借 M6：

- 重写 audio core；
- 重写 library；
- 重写 source；
- 重写 cache；
- 重构整个 Rust workspace。

所有必要修复：

记录：

`DECISIONS.md`

与：

`PROGRESS.md`

---

# 6. UI 总体架构

最终 UI 必须形成：

```text
ui/
  app/
  shell/
  design/
  features/
  components/
  state/
  ipc/
  commands/
  hooks/
  i18n/
  visual/
  windows/
```

实际目录可不同。

但职责必须清晰。

---

# 7. UI 分层

推荐：

```text
UI Components
      ↓
Feature State / Selectors
      ↓
Application Facade
      ↓
Typed IPC
      ↓
Tauri Commands / Events
      ↓
Rust Application Layer
      ↓
M1–M5 crates
```

禁止：

```text
React Component
   ↓
直接操作 aura-core internals
```

也禁止：

```text
React Component
   ↓
直接理解 SQLite
```

以及：

```text
React Component
   ↓
直接理解 reqwest
```

---

# 8. UI State 与 Domain State

必须严格区分：

## Domain state

例如：

```text
PlaybackState
Track
Queue
LibraryPage
SourceStatus
CacheStatus
```

来自：

Rust / application layer。

## UI state

例如：

```text
activePanel
commandPaletteOpen
queueOpen
eqPanelOpen
sidebarCollapsed
uiScale
language
windowMode
```

来自：

React / Zustand。

不要把两者混成一个巨型：

```text
AppStore
```

---

# 9. Zustand

使用：

`Zustand`

但：

禁止建立一个：

> 500+ fields 的 GlobalStore。

推荐拆成：

```text
playerStore
libraryStore
uiStore
settingsStore
queueStore
```

并使用：

> selectors

减少无关 render。

---

# 10. React Render Boundary

特别注意：

播放状态可能频繁变化。

Aurora 更可能需要：

> 高频视觉数据。

因此：

绝对不能：

> 每帧更新 Zustand → React 全树重渲染。

应该：

```text
Stable UI State
+
Imperative visual state
```

分开。

---

# 11. Typed IPC

建立统一：

```text
ui/ipc/
```

或者等价模块。

至少定义：

```text
PlayerSnapshot
TrackSummary
TrackDetails
LibraryPage
QueueSnapshot
PlaybackSettings
AudioSettings
SourceStatus
CacheStatus
ScanProgress
AppError
```

禁止：

```text
any
Record<string, any>
unknown
```

成为正常业务 DTO。

---

# 12. IPC DTO 原则

Rust 返回给 UI：

必须是：

> UI 所需的最小 DTO。

不要直接序列化：

- SQLite row；
- Rust internal struct；
- reqwest response；
- filesystem metadata internals。

---

# 13. IPC Event

建立统一事件层。

至少考虑：

```text
playback_state_changed
track_changed
queue_changed
scan_progress
library_changed
stream_metadata_changed
source_status_changed
cache_status_changed
```

具体事件按真实代码调整。

---

# 14. 高频事件限制

禁止：

```text
60Hz Tauri event
→ React state
→ whole UI render
```

例如：

> 播放进度。

应该：

- backend 提供低频稳定 snapshot；
- UI 本地 interpolation；
- requestAnimationFrame 驱动视觉；
- Aurora 使用独立 render state。

IPC：

不承担 60 FPS animation transport。

---

# 15. Structured App Error

建立：

```text
AppError
```

建议：

```text
code
message
recoverable
action
details?
```

例如：

```text
LIBRARY_DB_UNAVAILABLE
SOURCE_TIMEOUT
AUDIO_DEVICE_UNAVAILABLE
CACHE_FULL_PINNED
UNSUPPORTED_FORMAT
```

UI：

不得直接显示：

> Rust debug error / stack trace。

开发模式：

可以保留 diagnostics。

生产 UI：

使用友好消息。

---

# 16. Window Shell

实现正式桌面窗口。

必须：

- frameless；
- custom title bar；
- drag region；
- minimize；
- maximize / restore；
- close；
- focus；
- resize；
- minimize；
- restore。

Windows 体验：

必须：

> 看起来像一个真正的 Windows 桌面软件。

而不是：

> 把浏览器页面包起来。

---

# 17. Main Window

建议初始：

```text
1180 × 760
```

具体尺寸：

可以依据现有产品设计调整。

必须：

> 设置合理最小尺寸。

避免窗口缩小后：

- 控件重叠；
- title 被截断；
- queue 变形；
- Aurora 崩坏。

---

# 18. Window Persistence

本地保存：

- width；
- height；
- maximized；
- UI scale；
- language。

可选：

- position。

但：

位置恢复必须考虑：

> 多显示器 / 已断开的显示器。

如果窗口保存位置：

落在不可见区域：

自动修正。

---

# 19. Windows 11 Mica

Windows 11：

使用：

> Mica / 对应系统材质。

但：

必须保留：

> solid fallback。

---

# 20. Windows 10 Fallback

Windows 10：

不要：

- 透明化到无法阅读；
- fake Mica；
- 使用依赖 Win11 才存在的 API。

采用：

> 固态深色背景。

视觉仍保持：

- 深邃；
- 温暖；
- 高级；
- 清晰。

---

# 21. Platform Adapter

所有：

Windows-only UI capability：

放在：

```text
platform/windows
```

或者：

Rust platform adapter。

React：

不要：

到处写：

```text
if (isWindows)
```

---

# 22. Tauri Capabilities

M6 必须重新审查：

`src-tauri/capabilities/*`

原则：

> 最小权限。

只允许实际使用的：

- window；
- dialog；
- filesystem；
- path；
- event；
- app data；
- asset loading；

等能力。

如果没用到：

不要授权。

---

# 23. Shell Permission

默认：

> 不授权 shell。

禁止：

UI：

执行任意：

- PowerShell；
- cmd；
- external executable。

M6 不需要 shell。

---

# 24. CSP

保持严格：

不允许：

- remote script；
- remote stylesheet；
- remote font；
- remote iframe。

Three.js：

所有 shader / assets：

必须本地。

---

# 25. Remote Resource Policy

M6 UI：

默认：

> 零外部资源。

即使：

- 网络在线；
- M4 已经支持 network source；

UI 也不能自动：

加载远程 artwork / fonts / analytics。

---

# 26. Design System

必须继续使用 M0 建立的：

```text
design/tokens.ts
```

以及：

generated CSS variables。

如果当前实现不足：

本阶段完善。

---

# 27. Design Tokens

至少有：

```text
color
surface
text
muted
accent
danger
success

spacing
radius
shadow
blur
font
font-size
line-height

motion
duration
easing
```

但：

不要把设计 token 变成几百个无意义参数。

---

# 28. Color Policy

业务组件：

禁止：

```css
color: #xxxxxx;
background: #xxxxxx;
```

直接出现大量裸色。

统一：

```css
var(--color-...)
```

Aurora shader：

可以拥有独立 shader 参数。

但：

必须来源于统一视觉 token / palette。

---

# 29. Visual Language

整体风格：

> Dark / Atmospheric / Futuristic / Warm / Restrained / Elegant

避免：

- RGB gamer aesthetic；
- 大量霓虹；
- 过重玻璃拟态；
- 过多渐变卡片；
- 大面积纯黑死区；
- 过度圆角；
- 巨大的标题；
- 信息爆炸。

目标：

> 安静，但有生命。

---

# 30. Typography

必须：

- 清晰；
- 高可读；
- 中英文兼容；
- CJK 不发生 layout collapse；
- 不依赖远程字体。

中文：

不要强制使用：

> 英文优先字体。

必须真实测试：

- 中文标题；
- 日文；
- 英文；
- 长曲名；
- 长艺术家名。

---

# 31. Main Navigation

至少：

```text
Now Playing
Library
Map
```

可以增加：

```text
Settings
```

但建议：

> Settings 以 panel / modal / sheet 形式实现，而不是第四个一级主导航。

---

# 32. Navigation State

当前主页面：

必须由：

> typed route / enum

管理。

不要使用：

```text
string everywhere
```

例如：

```text
ViewId =
  "now-playing"
  "library"
  "map"
```

或 TypeScript union。

---

# 33. Now Playing

这是 M6 第一核心页面。

必须真正连接：

M1/M2。

---

# 34. Now Playing 布局

建议：

```text
┌───────────────────────────────────┐
│ Title Bar                         │
├───────────────┬───────────────────┤
│ Sidebar       │ Main                │
│               │                   │
│               │ Artwork / Aurora   │
│               │ Track Info         │
│               │ Progress           │
│               │ Controls            │
│               │                   │
└───────────────┴───────────────────┘
```

具体 layout：

可以更自由。

但：

必须保持：

> 主内容突出当前音乐。

---

# 35. Artwork

使用：

M3 的 artwork。

要求：

- 真实 artwork；
- 缩略图优先；
- fallback artwork；
- load failure graceful；
- no blocking large image decode in React render。

---

# 36. Artwork Loading

必须支持：

```text
loading
loaded
missing
corrupt
```

不要让：

> 一张坏封面

导致整个 Now Playing 页面失败。

---

# 37. Artwork Security

不要：

直接将：

> arbitrary local file path

作为 UI 可执行 URL。

通过：

> Tauri-safe asset / dedicated backend command / controlled resource protocol

加载。

不得打开：

任意 filesystem path。

---

# 38. Track Information

至少显示：

```text
Title
Artist
Album
```

可选：

```text
Album Artist
Track Number
Year
Genre
Source
```

但：

信息不足：

隐藏。

不要：

使用：

`Unknown Unknown Unknown`

铺满 UI。

---

# 39. Progress

必须显示：

- current position；
- duration；
- progress bar。

如果：

duration unknown：

例如 live stream：

不要伪造：

`03:21 / 04:10`

而应：

使用明确的：

> Live / Unknown duration

---

# 40. Seek UI

seek bar：

必须绑定：

M1/M4 的真实 seek capability。

如果：

```text
seekable = false
```

必须：

禁用 seek。

尤其：

Internet Radio：

不能伪造滑块可拖动。

---

# 41. Playback Controls

至少：

```text
Previous
Play / Pause
Next
Stop
```

播放按钮：

必须反映：

真实 state。

例如：

- Playing；
- Paused；
- Stopped；
- Buffering；
- Error。

---

# 42. Button Behavior

每个按钮：

必须：

- 有 keyboard focus；
- 有 accessible name；
- hover；
- active；
- disabled；
- tooltip / label。

禁止：

> UI 上有一个按钮，但没有任何真实 handler。

---

# 43. Volume

提供：

真实：

`SetVolume`

与：

M2 smoothing。

不要在 React 中再做一次“假音量”。

---

# 44. Loudness Control

允许：

Now Playing / Settings panel：

控制：

```text
Loudness Normalization
```

默认状态：

读取 M2/application settings。

UI 不计算 LUFS。

只调用：

> M2 backend。

---

# 45. Crossfade Control

允许：

设置：

```text
0–12 seconds
```

UI：

调用：

M2。

不要：

在 TS 端实现 crossfade。

---

# 46. EQ UI

如果 M2-B 已完成：

提供：

- 10 band EQ；
- preamp；
- preset；
- on/off。

所有参数：

通过 IPC：

> 发送给 Rust。

React：

不实现 DSP。

---

# 47. EQ Fallback

如果 M2-B 没有完成：

不要显示：

> fake EQ controls。

可以：

隐藏。

或者：

显示：

`Unavailable`

但不能：

> 给用户一个看似可以动、实际上无效的滑条。

---

# 48. Queue

Now Playing：

必须拥有：

> queue drawer / panel。

至少：

- 当前项；
- upcoming items；
- remove；
- play next；
- clear。

如果 M3-B queue persistence 已完成：

直接连接。

如果未完成：

可以使用：

> application-layer in-memory queue。

但：

不得重新建立第二套 Library persistence schema。

---

# 49. Queue Identity

Queue item：

必须引用：

```text TrackId
```

或者：

M4 Resolved Source 的稳定 source identity。

不能：

把整首曲子的数据库对象复制到 React queue state。

---

# 50. Queue Undo

[B]

如果 M3 已提供：

queue undo command：

可以连接。

否则：

M6 不为了 undo 重构整个 queue system。

---

# 51. Source Indicator

Now Playing 可以显示：

```text Local
HTTP
Internet Radio
Radio Browser
```

但：

必须：

使用真实 `SourceKind`。

不要根据：

URL 字符串猜测。

---

# 52. Network Metadata

M4 的：

`StreamMetadataEvent`

如果存在：

Now Playing：

可以显示：

当前：

`Artist / Title`

变化。

必须：

不依赖高频 React 全局刷新。

---

# 53. Live Stream UI

Internet Radio：

Now Playing：

应该体现：

```text LIVE
```

并：

- 禁用 seek；
- 显示 station / stream title；
- 显示合理连接状态。

---

# 54. Buffering UI

对于网络源：

如果播放器真的进入：

`Buffering`

UI：

显示：

> 正在缓冲

但：

不允许随便在所有加载状态都显示 buffering。

必须来自：

真实 playback/source state。

---

# 55. Error UI

播放错误：

至少显示：

- readable message；
- retry / reopen action；
- current source information。

禁止：

> 一次播放错误直接把整个应用锁死。

---

# 56. Aurora

Aurora 是：

> M6 的视觉核心。

但：

Aurora 不是 M8 Analysis。

---

# 57. Aurora 的定义

Aurora：

> 一个由 Three.js + GLSL 驱动的、本地生成、音乐播放状态响应的抽象视觉背景。

关键词：

```text
soft
organic
luminous
deep
slow
elegant
```

不要做：

- 音频频谱柱；
- DJ EQ 可视化；
- 五颜六色 RGB equalizer；
- 游戏化 HUD。

---

# 58. Aurora Canvas

必须：

> 整个应用尽可能只维护一个主 Aurora WebGL canvas。

不要：

- 每个组件创建 renderer；
- 每个页面重复创建 canvas；
- 每次切换页面重新初始化 WebGL。

---

# 59. Renderer Lifecycle

必须支持：

```text
create
mount
resize
pause
resume
dispose
```

页面切换：

不能：

> 每次重新创建 renderer。

---

# 60. Render Throttling

播放状态：

### Playing

目标：

> 24–60 FPS

根据设备性能动态决定。

### Paused / Stopped

降低到：

> 5–10 FPS

或者：

完全暂停。

### Window minimized

优先：

> pause rendering。

---

# 61. Reduced Motion

如果系统：

`prefers-reduced-motion`

或者应用设置：

`reducedMotion = true`

Aurora：

必须：

明显降低：

- rotation；
- drift；
- pulse；
- particle movement；

或：

完全静止。

不能：

> reduced motion 开启后依然高速动画。

---

# 62. Aurora Performance

必须：

避免：

- 每帧 React render；
- 动态创建 Three.js object；
- 每帧创建 Vector3；
- 每帧创建 arrays；
- 每帧 shader recompilation；
- texture recreation。

初始化：

> 预分配。

运行时：

> reuse objects。

---

# 63. Aurora Data Contract

建立：

```text
VisualTelemetry
```

最小可以包含：

```text
is_playing
playback_progress
volume
audio_level
source_kind
transition_progress
```

如果 M1/M2 当前已经真实提供：

可增加：

```text
rms
peak
```

但：

禁止伪造：

```text
bpm
key
energy
embedding
```

这些属于 M8。

---

# 64. Audio Level

如果 M1/M2 已存在：

> callback-safe level meter

直接复用。

如果不存在：

可以增加：

> 最小、无分析意义的 RMS / peak meter

但必须：

- callback-safe；
- atomic；
- no allocation；
- no lock；
- 不产生 FFT；
- 不产生 BPM；
- 不产生 Key。

---

# 65. Aurora 的音乐响应

Aurora 可以响应：

```text
play / pause
progress
audio level
volume
crossfade
source live state
```

而不是：

> 强行等待 M8 才有生命。

---

# 66. Aurora Color Mapping

颜色：

必须使用：

ifuyo Aura 的品牌视觉体系。

建议：

> warm ivory + amber + subtle cyan / violet accents

但：

不要使用高饱和 RGB。

最终颜色：

必须进入：

`tokens.ts`

或 shader palette config。

---

# 67. Aurora Composition

允许：

```text
gradient field
noise
soft particles
flow field
volumetric blobs
bloom-like glow
distortion
```

但：

必须：

> 代码生成。

禁止本阶段依赖：

- external 3D models；
- commercial texture packs；
- remote assets。

---

# 68. Shader Quality

必须：

避免：

- 巨型单 shader 文件；
- 所有视觉逻辑写成不可维护的一整坨 GLSL。

推荐：

```text
aurora/
  renderer
  scene
  materials
  shaders
  uniforms
  telemetry
  lifecycle
```

实际可调整。

---

# 69. Shader Error Handling

如果 WebGL：

- 初始化失败；
- shader compile 失败；
- context lost；

必须：

> graceful fallback。

例如：

使用：

CSS / Canvas 低级视觉背景。

不能：

> 整个 Now Playing 页面白屏。

---

# 70. WebGL Context Loss

必须考虑：

```text
webglcontextlost
webglcontextrestored
```

如果实际环境允许。

恢复后：

> 重新建立必要资源。

禁止整个应用 crash。

---

# 71. Aurora Idle State

没有播放：

视觉：

不要：

> 一片空白 + loading。

可以：

表现为：

> 非常缓慢、极轻微的呼吸状态。

---

# 72. Aurora Track Change

切换歌曲：

允许出现：

> 轻微 morph / color evolution / phase change。

但：

不要：

- 闪屏；
- 硬切；
- 每首歌都巨大的变化。

目标：

> “这盆东西还活着，但它今天和刚才不完全一样。”

---

# 73. Aurora Determinism

给定：

```text
track identity
visual seed
```

必须：

得到稳定初始状态。

播放中的：

> animation phase

可以继续推进。

但：

同一曲目再次打开：

应：

> 使用稳定 seed。

不要：

每次随机到完全不同。

---

# 74. Visual Seed

第一版可以：

```text
seed = hash(track identity)
```

或等价稳定算法。

注意：

> 这个 seed 不是 M7 Dreamscape seed。

它只用于：

> Now Playing visual variation。

---

# 75. Aurora 与 M8 的边界

M6：

使用：

```text
basic playback telemetry
```

M8：

未来可以提供：

```text
BPM
Key
LUFS
Energy
MFCC
Embedding
```

架构必须允许：

未来添加这些数据：

> 不需要重写 Aurora renderer。

---

# 76. Library Page

这是第二核心页面。

必须使用：

M3 的真实 Library API。

---

# 77. Library 主结构

建议：

```text
Search
Filters
Sort
View Controls
Track List
```

可以：

- list；
- compact list；
- grid；

默认：

> list。

不要为了视觉效果默认展示巨大卡片。

---

# 78. Library Search

必须真实调用：

M3 FTS5。

搜索：

支持：

- 中文；
- 英文；
- 单字；
- CJK bigram；
- prefix；
- special characters。

UI：

不能自己再实现一个“假搜索”。

---

# 79. Search Debounce

用户输入：

不要：

每一个字符都立即：

> 发送昂贵 SQLite query。

使用：

> 合理 debounce。

具体窗口：

根据实际体验测试。

---

# 80. Search State

必须支持：

```text
idle
searching
results
empty
error
```

加载期间：

不要闪烁整个页面。

---

# 81. Track List

至少：

```text artwork
title
artist
album
duration
```

可选：

```text year
genre
play count
```

---

# 82. Virtualization

如果数据库中：

> 数千 / 数万条歌曲

不能：

一次渲染全部 DOM。

优先：

使用现有或最小依赖：

`virtualized list`

或：

`@tanstack/react-virtual`

如果新增依赖：

必须审计 license / size / compatibility。

---

# 83. Stable Row Identity

每行：

使用：

```text TrackId
```

作为：

React key。

禁止：

使用：

```text array index
```

作为长期列表 key。

---

# 84. Sorting

UI 至少提供：

```text Title
Artist
Album
Date Added
Duration
Year
```

直接映射：

M3 query API。

不要：

把全部歌曲加载到前端：

> 然后 JS 排序。

---

# 85. Pagination

大量音乐：

必须：

使用：

M3 pagination。

例如：

```text page
limit
offset
```

或 cursor。

UI：

只保持：

当前可见数据 + 必要缓冲。

---

# 86. Library Filter

可以展示：

基础：

- artist；
- album；
- genre；
- year。

未来：

可以扩展：

- BPM；
- Key；
- energy；
- license。

但没有 M8 数据：

不要渲染虚假 filter options。

---

# 87. Folder Management

提供：

> Add Music Folder

通过：

Tauri native dialog。

用户选择目录后：

调用：

M3 library root API。

然后：

显示真实：

```text ScanProgress
```

---

# 88. Scan Progress

至少显示：

```text files discovered
files processed
imported
failed
```

不要：

只显示一个：

> “Loading…”

---

# 89. Scan Cancellation

UI 提供：

取消扫描。

必须调用：

M3 cancellation。

不要：

只在 UI 隐藏进度：

> backend 继续扫到底。

---

# 90. File Drop

[B]

可以支持：

拖入：

- folder；
- audio file。

通过 application layer：

进入 Library。

不得：

React 自己直接 filesystem scan。

---

# 91. Library Empty State

空音乐库：

不是：

> 空白页面。

必须有：

- 明确说明；
- Add Folder action；
- supported formats；
- no network requirement。

保持视觉克制。

---

# 92. Library Error State

数据库错误：

必须：

- 用户可理解；
- 提供 retry；
- 不显示 stack trace。

---

# 93. Double Click

双击 Track：

必须：

> 调用真实 playback command。

例如：

```text
play_track(track_id)
```

由 application layer：

解析：

Track → Source → Audio Core。

React 不负责整个播放 pipeline。

---

# 94. Context Menu

[B]

允许：

Track：

- Play;
- Play Next;
- Add to Queue;
- Open Folder;
- Remove from Library。

但：

如果某 action backend 尚未提供：

不要显示 fake action。

---

# 95. Map Page

M6：

只实现：

> 正式页面壳和导航入口。

不得实现：

> 真正 Music Map。

---

# 96. Map Page 内容

可以提供：

- page title；
- visual introduction；
- current library statistics；
- graceful empty state；
- future capabilities description。

但：

禁止：

伪造：

- clusters；
- nodes；
- similarity graph；
- BPM map；
- energy islands。

---

# 97. Map Placeholder

不要做一个丑陋的：

```text
COMING SOON
```

大白框。

应该：

> 使用正式产品视觉语言，形成一个“未来入口”。

但必须明确：

> 当前 Map engine 尚未启用。

---

# 98. Command Palette

快捷键：

```text
Ctrl + K
```

必须全局可用。

---

# 99. Command Architecture

不要建立：

```text
if command === ...
else if ...
else if ...
```

几百行 switch。

建立：

```text
Command
  id
  title
  keywords
  shortcut
  enabled
  execute()
```

或等价注册模型。

---

# 100. Command Palette 最小命令集合

至少：

```text
Go to Now Playing
Go to Library
Go to Map

Play / Pause
Next Track
Previous Track
Stop

Open Music Folder
Open File
Open URL

Toggle Queue
Toggle EQ
Toggle Loudness Normalization
Toggle Mini Player

Focus Library Search
```

具体命令：

必须根据 M1–M5 实际能力裁剪。

---

# 101. Command Search

支持：

- title；
- keyword；
- shortcut；
- fuzzy-ish matching。

不需要引入大型搜索框架。

---

# 102. Command Palette UX

必须支持：

- keyboard only；
- Arrow Up/Down；
- Enter；
- Esc；
- typing；
- focus trap。

打开：

自动 focus input。

关闭：

恢复之前 focus。

---

# 103. Global Keyboard Shortcuts

至少：

```text
Ctrl+K
Space
Ctrl+F
Esc
```

按产品实际冲突调整。

注意：

不要拦截：

> 用户正在输入文本时的 Space。

---

# 104. Accessibility

M6 必须达到：

> 基础可访问性，而不是只好看。

要求：

- semantic buttons；
- semantic headings；
- keyboard navigation；
- focus ring；
- tooltip / aria-label；
- dialog focus trap；
- logical tab order；
- no mouse-only action。

---

# 105. Focus Ring

禁止：

```css
outline: none;
```

然后：

什么都不补。

Focus：

必须清晰可见。

---

# 106. Contrast

正文：

必须具备良好对比度。

Muted text：

不能：

> 为了高级感而变成不可读灰色。

对实际 UI：

进行：

> contrast audit。

---

# 107. UI Scale

支持：

```text
90%
100%
110%
120%
130%
140%
```

或者等价粒度。

Scale：

必须：

真正影响：

- typography；
- spacing；
- control sizes；
- layout。

不能：

只是 browser zoom。

---

# 108. Responsive Layout

至少测试：

```text
900 × 600
1180 × 760
1600 × 1000
```

必须：

- queue 不遮住主控；
- sidebar 不压碎；
- artwork 不越界；
- Aurora 保持比例。

---

# 109. Mini Player

实现：

> 独立 320 × 96 左右的 compact window。

---

# 110. Mini Player 内容

至少：

```text
Artwork
Title
Artist
Play/Pause
Previous
Next
```

可选：

progress。

---

# 111. Mini Player Window

必须：

- frameless；
- always-on-top；
- draggable；
- close；
- resize policy明确；
- 与主播放器状态同步。

---

# 112. Mini Player Sync

主窗口：

改变：

播放状态。

Mini：

必须：

及时反映。

Mini：

点击：

Play/Pause。

主窗口：

必须同步。

不能：

> 两套 player state。

---

# 113. Mini Player Lifecycle

关闭 mini：

不得：

> 停止播放。

只是：

> 隐藏 mini window。

再次打开：

恢复：

> 当前 player snapshot。

---

# 114. Application Settings

M6 至少保存：

```text
language
ui_scale
reduced_motion
loudness_enabled
crossfade_seconds
eq_enabled
eq_preset
```

如果实际支持：

可增加：

- window state；
- mini player state。

---

# 115. Settings Storage

不要：

把纯 UI 设置塞进：

M3 Library DB。

设置：

应该使用：

> application configuration storage。

---

# 116. Settings Migration

配置格式必须：

有：

> version。

以后升级：

不能：

直接读旧 JSON 然后 panic。

至少：

支持：

> backward-compatible defaulting。

---

# 117. i18n

必须：

```text
zh-CN
en
```

所有用户可见字符串：

必须来自：

> locale resources。

禁止：

```tsx
<button>播放</button>
```

这种大量散落文本。

---

# 118. i18n Fallback

至少：

```text
selected language
↓
en
↓
zh-CN
```

或者与项目统一 fallback 策略保持一致。

---

# 119. Dynamic Language Switch

如果实现：

切换语言：

不需要重启。

至少：

> 当前窗口 UI 可重新渲染。

---

# 120. Long Text Testing

专门测试：

```text
超长中文曲名
超长英文标题
超长艺术家
超长专辑名
Japanese
emoji
mixed-script
```

不能：

因为一条标题：

> 把 Now Playing 布局撑爆。

---

# 121. Loading System

建立统一：

```text
LoadingState
EmptyState
ErrorState
SuccessState
```

不要：

每个页面自己发明一套。

---

# 122. Skeleton

适量使用：

skeleton。

但：

不要：

> 页面 80% 都是 skeleton。

---

# 123. No Fake Loading

禁止：

为了“看起来像高级软件”：

人为延迟：

2 秒 / 3 秒。

加载多少：

就显示多少。

---

# 124. Motion System

UI 普通过渡：

目标：

```text
120–240ms
```

用于：

- panel；
- modal；
- hover；
- route change；
- queue drawer。

禁止：

所有东西都无限 animation。

---

# 125. Hover / Focus

hover：

应该：

轻。

focus：

应该：

明显。

active：

应该：

有真实反馈。

不要：

每次 hover：

放大 10%。

---

# 126. Reduced Motion

系统：

```text
prefers-reduced-motion
```

以及应用：

```text
reducedMotion
```

都应该：

统一进入：

motion policy。

---

# 127. Performance Architecture

M6：

必须关注：

> UI 不应该拖累 Audio Core。

---

# 128. Audio Isolation

React：

不能：

直接触碰：

- audio callback；
- ring buffer；
- decoder。

IPC：

只是：

> control / snapshot bridge。

---

# 129. Main Thread

不得在 React/UI main thread：

执行：

- 大规模 metadata parsing；
- SQLite query；
- thumbnail generation；
- audio decoding；
- SHA-256；
- HTTP；
- cache integrity。

这些：

必须在 Rust / worker / backend。

---

# 130. Library Rendering Performance

大量 tracks：

采用：

- virtualization；
- pagination；
- memoization；
- stable selectors。

不要：

对每一个 row：

创建大量匿名 callback + expensive derived data。

先测：

再优化。

---

# 131. Artwork Cache Performance

大量 artwork：

不得：

一次全部加载到内存。

浏览列表：

应：

> lazy load / thumbnail load。

---

# 132. Aurora Performance Budget

至少测：

```text
FPS
frame time
GPU memory
renderer initialization
resize cost
context recovery
```

至少记录：

- idle；
- playing；
- resize；
- queue open；
- library navigation。

---

# 133. Performance Degradation

如果检测到：

Aurora：

明显掉帧。

可以：

自动降级：

```text
60 → 30
30 → 20
20 → 10
```

或者：

减少：

- particle count；
- resolution；
- post-processing。

不得：

> 因视觉效果拖垮普通音乐播放 UI。

---

# 134. Device Pixel Ratio

高 DPI：

必须：

测试：

```text
100%
125%
150%
200%
```

注意：

> WebGL canvas resolution 与 CSS size 分离。

不得：

> 4K DPI 导致 GPU 分配失控。

---

# 135. WebGL Resolution Cap

必须：

有合理：

> internal render resolution cap。

不要直接：

```text
canvasWidth × devicePixelRatio
```

无限放大。

---

# 136. Window Resize

Aurora resize：

必须：

debounce / coalesce。

不要：

每个 browser resize event：

都重新创建资源。

---

# 137. CPU Idle

暂停播放：

如果：

> Aurora 不需要持续动画

就：

降低 rendering。

UI idle：

不得：

产生：

高 CPU 消耗的无限 loops。

---

# 138. Error Logging

开发环境：

允许：

tracing / console。

生产：

不得：

大量：

`console.log()`。

尤其：

不能每一帧 log。

---

# 139. Diagnostics

M6 可以建立：

本地 diagnostics：

```text
window size
renderer FPS
audio state
library count
cache status
source status
```

但：

绝对不得：

> 上传。

---

# 140. Telemetry

M6：

不得新增：

- analytics；
- crash upload；
- usage tracking；
- telemetry；
- remote logs。

所有 metrics：

只能：

> 本地。

---

# 141. External Links

如果 UI 需要：

打开：

- ifuyo.com；
- source website；
- station website；

必须：

通过：

> 系统浏览器。

不要：

在 app webview 中随意打开远程网页。

---

# 142. Source / Cache UI Boundary

M6：

可以：

显示：

当前 Source：

```text Local / HTTP / Radio
```

也可以：

显示：

cache status。

但：

不要在 M6：

重新实现：

- HTTP client；
- cache lookup；
- cache storage；
- Radio Browser。

---

# 143. Cache Status

如果 M5 已提供：

可以显示：

```text Cached
Streaming
Offline Ready
```

但：

这些状态：

必须来自：

M5/application layer。

UI：

不能根据：

> 文件大小 / URL 猜。

---

# 144. Offline Mode

M6：

不得添加：

> “检测网络后自动 ping”。

离线行为：

由：

M4/M5 source/cache：

提供真实状态。

---

# 145. Audio Device UI

M2 已经有：

hotplug/recovery。

M6 可以显示：

```text Device
Connected
Unavailable
Recovering
```

但是：

SMTC / 系统媒体控制：

本阶段不实现。

---

# 146. Device Selector

如果当前 application layer：

已经有稳定：

> output device selection

可以提供 UI。

如果没有：

不要在 M6 自己创建：

> Windows device enumeration implementation。

把该功能保留在：

> 后续 Windows Integration。

---

# 147. Command / IPC Concurrency

快速连续操作：

例如：

```text
Play
Pause
Play
Seek
Next
```

不得：

产生：

> race 到 UI 显示错误状态。

前端：

不要仅根据“按钮点击”乐观修改最终状态。

最终以：

> backend snapshot

为准。

---

# 148. Optimistic UI

允许：

只有：

> 明确不会产生错误的轻量 UI 操作。

播放：

更推荐：

```text command sent
↓
backend state changed
↓
UI updated
```

不要：

> UI 自己猜 Playing，然后 backend 实际进入 Error。

---

# 149. Stale Event Protection

IPC event：

必须：

带：

必要的：

```text generation
sequence
timestamp / revision
```

或等价机制。

如果旧 event 到达：

不得：

覆盖新 state。

这与：

M1/M2 的 generation

形成统一思想。

---

# 150. State Revision

如果可行：

建立：

```text state_revision
```

用于：

playback / queue / library snapshots。

例如：

```text playback_revision = 42
```

UI：

只接受：

> 不低于当前 revision 的 snapshot。

---

# 151. Library Refresh

当 M3 watcher：

发现：

新文件。

UI：

可以：

收到：

`library_changed`

然后：

刷新当前 query。

不要：

每次 watcher event：

直接：

> 整页 reload。

可以：

debounce / coalesce。

---

# 152. Search + Library Changes

如果用户：

正在搜索：

```text night
```

同时：

数据库加入新 Track。

UI：

应该：

保持当前查询条件。

重新请求：

> 当前 query。

不要：

突然：

返回全部歌曲。

---

# 153. Scan Progress + UI

扫描：

必须：

显示：

真实 progress。

如果总文件数未知：

不要：

伪造：

`63%`

可以显示：

```text
Processed 126 / ?
```

---

# 154. Main App Loading

启动：

建议：

```text
Tauri shell
↓
UI shell visible
↓
backend initialization
↓
player snapshot
↓
library snapshot
```

不要：

等待整个 library scan：

> 才显示主界面。

---

# 155. Startup Priority

启动优先级：

```text
1. window
2. UI shell
3. player state
4. library metadata snapshot
5. background refresh
```

这样：

用户：

> 尽快看到软件。

---

# 156. Cold Start Benchmark

记录：

```text
process start
→ first meaningful UI
```

并记录：

> 实际机器实测。

目标：

> 尽量控制在商业桌面应用可接受范围。

如果测不到：

不要伪造。

---

# 157. Tauri Multiwindow

M6 的：

主窗口 + mini player：

必须：

共享：

application layer。

不要：

启动第二份：

audio engine。

---

# 158. Window IPC

窗口操作：

例如：

```text
showMiniPlayer
hideMiniPlayer
toggleMiniPlayer
focusMainWindow
```

由：

> application/window manager

统一管理。

不要：

React window component 自己到处调用 raw Tauri APIs。

---

# 159. Focus Restoration

Command Palette：

关闭后：

恢复之前：

> focused element。

Queue panel：

打开/关闭：

不要：

导致键盘 focus 丢失。

---

# 160. Modal Architecture

建立：

统一：

```text
ModalHost
DrawerHost
ToastHost
```

不要：

每个 feature：

自己建一整套 portal 生命周期。

---

# 161. Toast

可以有：

轻量：

- success；
- info；
- warning；
- error。

但：

不要：

每次：

play：

弹一个 toast。

---

# 162. Notification Policy

M6：

禁止：

- OS toast spam；
- background nagging；
- “你很久没听歌了”；
- forced check-in。

产品核心仍然是：

> 陪伴，不打扰。

---

# 163. UX Principle

任何 UI：

优先问：

> “用户现在真正需要什么？”

而不是：

> “还能塞什么按钮？”

---

# 164. No Feature Theater

禁止为了：

“看起来功能很多”：

添加：

- fake charts；
- fake statistics；
- fake AI labels；
- fake BPM；
- fake smart recommendations；
- fake cloud sync；
- fake social activity。

---

# 165. Empty Data Policy

数据不存在：

必须：

> 诚实显示 unavailable。

例如：

没有 analysis：

```text
Not analyzed
```

而不是：

```text
BPM 127
Key 8A
Energy 0.73
```

---

# 166. Real Data Contract

UI 可用的数据：

必须来自：

```text
M1
M2
M3
M4
M5
```

或者：

> M6 自身真实计算的 UI state。

不允许：

> random demo data

混进 production path。

---

# 167. Demo Fixtures

测试 / Storybook-like previews：

可以：

使用：

mock fixture。

但必须：

明确：

```text
TEST ONLY
```

生产构建：

不能加载这些 mock。

---

# 168. Component Architecture

建立可复用组件：

至少：

```text
Button
IconButton
Slider
Tooltip
Dialog
Drawer
SearchInput
TrackRow
Artwork
PlaybackControls
ProgressBar
```

组件：

应保持：

> domain-light。

---

# 169. Feature Components

Feature：

例如：

```text
NowPlaying
Library
Queue
CommandPalette
MiniPlayer
Aurora
```

使用：

可复用基础组件。

不要：

把整个 app：

写成一个：

`App.tsx 3000 lines`

---

# 170. Icon Policy

图标：

优先：

- SVG；
- local icon set；
- existing dependency。

不要：

每个按钮随意复制不同风格 SVG。

---

# 171. No Remote Fonts

任何字体：

必须：

本地系统字体或已批准本地资产。

---

