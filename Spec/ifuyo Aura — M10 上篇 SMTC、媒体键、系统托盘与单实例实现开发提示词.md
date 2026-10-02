# ifuyo Aura
# M10 — Windows Integration / SMTC / Media Keys / System Tray / Single Instance
> 本文档为原 M10 开发提示词的**上篇（SMTC、媒体键、系统托盘与单实例实现，§0–242）**。
> 下篇《ifuyo Aura — M10 下篇 测试、原生 QA 与发布门禁开发提示词.md》承载 §243–411。
> 章节编号沿用原 M10 全局编号；跨篇引用（如“见 #132”）按编号在两篇中查找。


你现在进入 ifuyo Aura 项目的 **M10：Windows 系统集成阶段**。

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
M9 Music Map
```

基础上，正式实现：

> **Windows 系统级媒体集成与桌面后台能力。**

M10 的目标是让 ifuyo Aura 不再只是：

> “一个 Windows 窗口中的音乐播放器”

而是真正成为：

> **Windows 媒体生态中的本地音乐应用。**

本阶段负责：

```text
ifuyo Aura Playback Authority
        ↓
Windows Integration Layer
        ├─ SMTC
        ├─ Hardware Media Keys
        ├─ System Media Metadata
        ├─ Playback Position
        ├─ Seek Requests
        ├─ Tray
        ├─ Background App Lifecycle
        └─ Single Instance
```

---

# 1. 本阶段核心能力

## A 级必须完成

至少实现：

1. Windows integration layer。
2. SMTC lifecycle。
3. SMTC session ownership。
4. SMTC metadata。
5. SMTC artwork。
6. SMTC playback state。
7. SMTC play command。
8. SMTC pause command。
9. SMTC next command。
10. SMTC previous command。
11. SMTC stop command。
12. SMTC seek command。
13. SMTC position synchronization。
14. SMTC duration synchronization。
15. seekable / non-seekable state synchronization。
16. current track synchronization。
17. live stream synchronization。
18. ICY StreamTitle synchronization。
19. Windows media key integration。
20. external system playback command routing。
21. stale command protection。
22. SMTC event lifecycle cleanup。
23. Windows API error isolation。
24. Windows version fallback。
25. system tray icon。
26. tray menu。
27. tray play/pause。
28. tray next。
29. tray previous。
30. tray show/focus main window。
31. tray open Now Playing。
32. tray open Library。
33. tray open Music Map。
34. tray open Dreamscape。
35. tray Mini Player。
36. tray Exit。
37. tray state synchronization。
38. tray menu enable/disable state。
39. tray language synchronization。
40. tray lifecycle。
41. close/hide/background policy。
42. single-instance protection。
43. second-instance focus behavior。
44. process lifecycle integration。
45. shutdown cleanup。
46. system integration diagnostics。
47. deterministic mock tests。
48. Windows-specific integration tests。
49. M1–M9 full regression。
50. security review。
51. dependency/license review。
52. performance review。
53. documentation。
54. final audit。

---

# 2. B 级能力

A 级全部稳定之后，可以实现：

1. Global custom media shortcuts。
2. configurable media hotkeys。
3. SMTC seek forward / rewind。
4. SMTC playback rate if future Audio Core supports it。
5. taskbar / thumbnail integration。
6. taskbar playback progress integration。
7. richer tray dynamic submenu。
8. tray “Recently Played”。
9. startup behavior configuration。
10. start-minimized behavior。
11. richer Windows notification-area behavior。
12. Windows-specific diagnostics panel。
13. system integration self-test。
14. media-session stress testing。
15. additional Windows shell integrations。

B 级不得破坏 A 级。

---

# 3. 本阶段明确禁止

不要实现：

- Steam；
- Steam Overlay；
- Steam Achievements；
- Steam Cloud；
- installer redesign；
- updater；
- auto updater；
- code signing implementation；
- SignPath workflow；
- payment；
- license activation；
- account system；
- cloud sync；
- telemetry；
- analytics；
- crash reporting service；
- online metadata；
- online artwork；
- lyrics；
- Music Map redesign；
- Dreamscape redesign；
- new audio codec；
- new Source provider；
- new cache system；
- new analysis algorithm；
- new embedding model；
- recommendation engine；
- autoplay recommendation；
- AI DJ；
- social features。

特别注意：

> **M10 只做 Windows Integration，不借机实现 M11+ 发布体系。**

---

# 4. Windows Integration 的职责边界

必须建立：

```text
Windows Integration
```

或等价的：

```text platform/windows
```

负责：

```text Windows APIs
SMTC
Tray
Single Instance
Windows-specific lifecycle
```

不得让：

```text aura-core
```

依赖：

```text Windows SMTC
Tauri
Tray
WinRT
```

---

# 5. 推荐总体架构

推荐：

```text
M1/M2 Playback Authority
        ↓
Application Playback Service
        ↓
Windows Integration Coordinator
        ├───────────────┐
        ↓               ↓
     SMTC             Tray
        ↓               ↓
 Windows System      Notification Area
```

并保持：

```text React UI
   ↕
Typed Application IPC
   ↕
Application Layer
   ↕
M1–M9
```

---

# 6. 核心原则

必须明确：

> SMTC 和 Tray 都不是第二套播放器。

它们只是：

> **Windows 侧控制入口。**

因此：

```text Main Window
Mini Player
Command Palette
Tray
SMTC
Media Keys
```

最终：

全部调用：

> 同一个 Playback Authority。

---

# 7. Single Playback Authority

整个应用：

必须只有：

> 一个真正的 playback controller。

禁止：

```text React Player
+
Tray Player
+
SMTC Player
```

三个状态机。

正确：

```text Playback Authority
        ↑
 ┌──────┼───────┬────────┐
 UI    Mini    Tray     SMTC
```

---

# 8. Command Routing

所有外部系统操作：

例如：

```text Play
Pause
Next
Previous
Stop
Seek
```

必须：

进入：

```text Application Playback Command
```

再：

调用：

M1/M2。

Windows Integration：

不得直接：

操作 decoder。

---

# 9. SMTC

SMTC：

必须作为：

> Windows-specific adapter。

不要把：

SMTC API：

散布：

整个：

application layer。

---

# 10. SMTC Ownership

建立：

```text SmTcController
```

或：

等价：

`WindowsMediaController`。

它负责：

- initialization；
- command subscription；
- metadata updates；
- playback state updates；
- timeline updates；
- teardown。

---

# 11. SMTC Lifecycle

至少：

```text Uninitialized
Initializing
Ready
Active
Paused
Inactive
Failed
Disposed
```

实际代码：

可以：

简化。

但：

状态必须：

明确。

---

# 12. SMTC Initialization

应用：

启动后：

应该：

在：

> Windows integration initialization

阶段：

创建：

SMTC 控制对象。

不得：

因为：

React 页面还没有打开：

就延迟到：

> Now Playing 页面 mount。

SMTC：

属于：

> application lifecycle。

---

# 13. SMTC Initialization Failure

如果 Windows API：

不可用：

必须：

> graceful disable。

结果：

主播放器：

仍然：

正常运行。

例如：

```text SMTC = Unavailable
Tray = Available
Player = Available
```

不得：

整个：

ifuyo Aura：

启动失败。

---

# 14. OS Version Policy

必须：

确认：

当前 M0：

定义的：

Windows 最低支持版本。

如果：

系统不支持：

SMTC：

应该：

```text integration unavailable
```

而不是：

调用：

不存在：

API。

不得：

将：

Windows 版本：

硬编码：

散布：

整个代码库。

---

# 15. Platform Isolation

Windows-specific：

代码：

使用：

例如：

```text cfg(windows)
```

或：

明确：

platform adapter。

不要：

让：

非 Windows：

编译：

依赖：

Windows-only runtime。

---

# 16. Windows Crate

如果需要：

`windows` crate：

先：

审计：

- version；
- feature flags；
- MSRV；
- binary impact；
- license；
- Windows compatibility；
- CI；
- build size。

只启用：

> 实际需要的 Windows API features。

禁止：

为了方便：

开启：

整个：

Windows API surface。

---

# 17. Tauri Integration

Tauri：

只作为：

> desktop host / lifecycle integration。

SMTC：

不要：

直接：

绑定：

React 组件。

Tray：

也不要：

让：

每个 React component：

自己创建。

---

# 18. SMTC Threading

Windows：

SMTC command event：

可能：

不运行：

在：

UI thread。

因此：

事件处理：

不能：

直接：

假设：

可以：

操作：

WebView / React UI。

正确：

```text SMTC event
↓
Windows adapter
↓
Application command
↓
Playback Authority
```

如果：

UI：

需要：

更新：

再：

通过：

application event bridge。

---

# 19. No UI Coupling

SMTC：

不得：

直接：

调用：

```text setState()
```

或：

操纵：

Zustand。

Windows Integration：

只：

产生：

> application-level commands / events。

---

# 20. Media Button Commands

至少支持：

```text Play
Pause
Next
Previous
Stop
```

根据：

当前 Windows API：

实际能力：

配置：

supported buttons。

---

# 21. Play

SMTC：

收到：

`Play`

必须：

调用：

真实：

```text playback.play()
```

不得：

自己：

修改：

UI：

`isPlaying = true`

最终状态：

来自：

Playback Authority。

---

# 22. Pause

同理：

```text playback.pause()
```

SMTC：

不能：

自己：

维护：

pause state。

---

# 23. Next

SMTC：

收到：

Next：

调用：

真实：

application：

`nextTrack()`。

不得：

直接：

操作：

Queue array。

---

# 24. Previous

调用：

真实：

`previousTrack()`。

行为：

遵循：

M6 / application layer：

现有：

previous semantics。

M10：

不要重新设计：

previous：

规则。

---

# 25. Stop

调用：

M1/M2：

真实：

`stop()`。

不得：

只：

设置：

SMTC：

`Stopped`

但：

播放器：

仍：

播放。

---

# 26. Seek

如果：

当前：

Track：

真实：

`seekable = true`

SMTC：

应该：

允许：

position request。

然后：

调用：

application：

```text seek(frame)
```

或：

等价：

frame-based：

API。

---

# 27. Non-seekable

例如：

Internet Radio：

SMTC：

不能：

显示：

可：

seek：

的：

假状态。

如果：

Windows API：

允许：

关闭：

position request：

则：

关闭。

至少：

收到：

seek：

request：

必须：

安全：

ignore / reject。

---

# 28. Live Stream

Live：

应该：

同步：

```text PlaybackStatus = Playing / Paused
```

但：

不伪造：

```text Duration
Position
```

不要：

把：

无限电台：

伪装：

成：

03:42 / 04:10。

---

# 29. Unknown Duration

Direct HTTP：

如果：

duration：

unknown：

SMTC：

应：

保持：

合理：

> unknown timeline。

不得：

填：

0：

冒充：

零秒歌曲。

---

# 30. Duration

Finite Track：

使用：

真实：

M1/M2：

duration。

不得：

从：

UI：

猜测。

---

# 31. Position

SMTC：

position：

必须：

来自：

Playback Authority。

推荐：

内部：

仍保持：

frame-based：

```text position_frames
sample_rate
```

Windows：

需要：

时：

再：

转换：

为：

所需：

time unit。

---

# 32. Position Conversion

必须：

使用：

checked / overflow-safe：

转换。

特别测试：

- 44.1 kHz；
- 48 kHz；
- 96 kHz；
- extremely long duration；
- unknown duration。

不得：

产生：

负值：

或：

整数：

overflow。

---

# 33. Timeline Synchronization

不能：

每个：

audio callback：

更新：

SMTC。

绝对禁止：

```text audio callback
→ Windows API
```

---

# 34. Timeline Update Frequency

SMTC：

position：

只需要：

> 低频同步。

建议：

约：

2–4 次/秒：

或：

经：

实际测量：

决定。

目标：

> 系统媒体 UI 能保持合理位置。

而不是：

60Hz：

疯狂更新。

---

# 35. No 60Hz Windows IPC

禁止：

```text Audio Callback
→ SMTC
→ UI
```

循环。

---

# 36. Track Change

当：

真实：

current track：

发生：

变化：

SMTC：

必须：

更新：

- title；
- artist；
- album；
- album artist；
- playback type；
- artwork；
- timeline；
- supported controls；
- playback status。

---

# 37. Track Update Ordering

必须：

避免：

出现：

```text Track A playing
↓
Track B starts
↓
SMTC metadata still A
↓
SMTC status B
```

应该：

保证：

> metadata + status + timeline：

在：

合理：

一致：

的：

update boundary：

内：

提交。

---

# 38. Generation / Revision

SMTC：

更新：

必须：

可：

关联：

```text playback_revision
track_generation
```

或者：

等价：

机制。

旧：

Track A：

异步更新：

不能：

覆盖：

Track B。

---

# 39. Example Race

```text Track A
↓
SMTC artwork A loading
↓
Track B selected
↓
B starts
↓
A artwork finishes
```

A：

不得：

覆盖：

SMTC：

当前：

B。

---

# 40. Metadata Snapshot

SMTC：

更新：

最好：

来自：

一个：

> immutable snapshot

例如：

```text
SmTcMediaSnapshot
```

包括：

```text track_id
title
artist
album
album_artist
artwork_ref
duration
seekable
live
playback_state
position
revision
```

---

# 41. Metadata Ownership

SMTC：

不得：

从：

SQLite：

自己：

查询：

音乐 metadata。

应该：

使用：

application：

提供：

的：

snapshot。

---

# 42. Track Metadata

优先：

使用：

M3：

实际：

数据：

```text title
artist
album
album_artist
track_number
```

缺失：

则：

使用：

明确：

fallback。

---

# 43. Metadata Fallback

例如：

title：

为空：

可以：

fallback：

为：

文件名：

但：

必须：

遵循：

M3：

实际：

metadata normalization：

不要：

在 M10：

再创造：

第二套：

metadata parser。

---

# 44. No Fake Metadata

不得：

显示：

随机：

- artist；
- album；
- genre。

没有：

就是：

> unavailable。

---

# 45. Album Artist

如果：

存在：

优先：

提供。

如果：

不存在：

可以：

使用：

artist：

或：

省略。

必须：

保持：

一致：

策略。

记录：

`DECISIONS.md`。

---

# 46. Track Number

如果：

SMTC：

实际：

API：

支持：

相关：

music metadata：

可以：

提供。

否则：

不要：

为了数量：

强塞。

---

# 47. Artwork

SMTC：

可以：

显示：

当前：

album artwork。

来源：

优先：

M3：

本地：

artwork cache。

---

# 48. Artwork Boundary

SMTC：

不得：

直接：

读取：

用户：

任意：

filesystem path。

必须：

读取：

> application-controlled artwork cache。

---

# 49. Artwork Path Validation

任何：

提供：

给：

Windows：

的：

artwork path：

必须：

验证：

- exists；
- within approved app data directory；
- expected image format；
- bounded size。

---

# 50. No Remote Artwork

如果：

网络：

source：

M4：

没有：

合法：

artwork：

SMTC：

不要：

联网：

补封面。

---

# 51. Artwork Failure

封面：

读取：

失败：

SMTC：

仍然：

应该：

显示：

metadata。

不要：

因为：

封面：

失败：

让：

系统媒体控制：

失效。

---

# 52. Artwork Refresh

Artwork：

只：

在：

- Track change；
- artwork change；

时：

更新。

不要：

每秒：

刷新：

thumbnail。

---

# 53. Radio Stream Metadata

M4：

已经：

提供：

`StreamMetadataEvent`

例如：

```text Artist
Title
```

M10：

应该：

能够：

更新：

SMTC。

---

# 54. ICY Update

当：

StreamTitle：

发生：

变化：

应该：

更新：

SMTC：

metadata。

---

# 55. ICY Update Coalescing

如果：

电台：

异常：

频繁：

发送：

metadata：

需要：

coalesce。

不要：

每个：

metadata：

变化：

都：

执行：

昂贵：

Windows：

update。

---

# 56. Radio Fallback Metadata

没有：

StreamTitle：

时：

可以：

保持：

station：

metadata。

不得：

猜：

当前：

歌曲。

---

# 57. Playback Status

必须：

根据：

真实：

PlaybackState：

同步：

SMTC：

例如：

```text Playing
Paused
Stopped
```

具体：

映射：

按：

当前：

Windows API：

实际：

enum。

---

# 58. Buffering

如果：

M4/M1/M2：

真实：

存在：

`Buffering`

SMTC：

应：

使用：

合理：

映射。

不得：

在：

普通：

loading：

时：

伪造：

playing。

---

# 59. Error State

播放：

进入：

Error：

SMTC：

不能：

继续：

显示：

Playing：

数分钟：

而：

播放器：

实际：

已经：

停止。

需要：

同步：

最终：

state。

---

# 60. Stopped Semantics

应用：

停止：

必须：

同步：

SMTC：

停止。

同时：

根据：

实际：

Windows：

语义：

决定：

是否：

保留：

最后：

metadata：

或：

清空。

必须：

明确：

记录。

---

# 61. Paused Semantics

暂停：

保持：

metadata：

与：

timeline。

position：

应该：

停住。

---

# 62. Resume

继续：

播放：

SMTC：

恢复：

`Playing`

并：

恢复：

position：

更新。

---

# 63. Next During Pause

如果：

用户：

在：

Paused：

状态：

从：

系统：

点击：

Next：

必须：

遵循：

application：

queue：

语义。

M10：

不要：

自行：

判定：

应该：

还是：

保持：

pause。

---

# 64. Previous Semantics

完全：

复用：

M6：

已有：

previous：

逻辑。

---

# 65. Queue Empty

没有：

Next：

Track：

时：

SMTC：

应该：

禁用：

Next。

不得：

让：

用户：

点击：

一个：

实际：

不会：

产生：

动作：

的：

按钮。

---

# 66. Queue Boundary

同理：

Previous：

没有：

有效：

前一曲：

则：

disabled / unavailable。

---

# 67. Dynamic Supported Buttons

随着：

queue：

和：

seekability：

变化：

SMTC：

supported buttons：

必须：

更新。

---

# 68. Unsupported Buttons

不要：

开启：

M10：

实际：

不支持：

的：

命令。

例如：

尚未实现：

fast forward：

就：

不要：

启用。

---

# 69. Playback Position Change Request

SMTC：

如果：

收到：

position：

变化：

必须：

验证：

- finite；
- non-negative；
- duration known；
- seekable；
- within bounds。

---

# 70. Invalid Seek

例如：

请求：

负位置：

或：

超过：

duration：

必须：

clamp / reject。

不能：

panic。

---

# 71. Seek Race

例如：

```text
SMTC seek 30s
↓
user UI seek 60s
↓
SMTC response arrives late
```

旧：

request：

不能：

把：

playback：

拉回：

30s。

必须：

有：

request/revision：

保护。

---

# 72. Command Ordering

对：

```text
Play
Pause
Play
Next
Pause
```

等：

快速：

连续：

命令：

必须：

按：

Application：

定义：

顺序：

执行。

不得：

SMTC：

建立：

自己：

的：

second state machine。

---

# 73. Duplicate Command

如果：

SMTC：

产生：

duplicate：

Play：

例如：

已经：

Playing：

再次：

Play：

必须：

安全：

幂等。

---

# 74. SMTC Callback Must Not Block

SMTC：

event callback：

不得：

- sleep；
- 长时间等待；
- 数据库 full scan；
- decoder；
- network；
- artwork decode；
- large allocation。

只：

> 投递：

application command。

---

# 75. Tray

建立：

Windows：

system tray。

Tauri 当前版本提供正式 tray API，应先按照项目实际 Tauri 版本审计当前实现方式，不机械复制旧版本示例。

---

# 76. Tray Ownership

推荐：

tray：

由：

> Rust / application layer

统一创建：

因为：

tray：

属于：

system integration lifecycle。

Frontend：

只：

消费：

必要：

状态。

---

# 77. Tray Lifecycle

至少：

```text
Created
Visible
Updating
Disposing
Disposed
```

不要：

每次：

React route：

变化：

重新：

create tray icon。

---

# 78. One Tray Icon

整个：

app：

原则上：

只有：

一个：

主：

ifuyo Aura：

tray icon。

不要：

主窗口：

一个：

Mini：

一个：

Dreamscape：

一个。

---

# 79. Tray Icon

必须：

使用：

正式：

ifuyo Aura：

icon。

不得：

使用：

debug placeholder。

---

# 80. Tray Icon Asset

图标：

必须：

考虑：

Windows：

系统托盘：

小尺寸：

显示：

例如：

16/20/24/32 px：

等：

合理：

DPI：

资源。

不要：

把：

巨型：

PNG：

作为：

唯一：

runtime：

资源。

---

# 81. Tray Tooltip

默认：

简洁：

例如：

```text
ifuyo Aura
```

不要：

因为：

当前：

歌曲：

不断：

修改：

tooltip：

造成：

系统：

刷新：

噪音。

---

# 82. Tray Menu

A：

至少：

提供：

```text Open ifuyo Aura
Now Playing
Library
Music Map
Dreamscape

Play / Pause
Previous
Next
Mini Player

Exit
```

具体：

命令：

必须：

根据：

当前：

真实：

功能：

启用。

---

# 83. Tray Menu 不伪造未来功能

如果：

M9：

已经：

存在：

Music Map：

允许：

打开。

如果：

M7：

已经：

存在：

Dreamscape：

允许：

打开。

不得：

出现：

不存在：

的：

M11：

功能。

---

# 84. Tray Current Track

可以：

在：

tray：

菜单：

显示：

当前：

Track：

例如：

```text
Now Playing
Artist — Title
```

但：

必须：

是：

真实：

metadata。

如果：

无：

track：

显示：

`Nothing Playing`

或：

等价：

空状态。

---

# 85. Dynamic Play/Pause Label

Tray：

应：

根据：

真实：

playback state：

显示：

```text Play
```

或：

```text Pause
```

不能：

只：

根据：

上一次：

点击：

切换：

文案。

---

# 86. Tray Menu Enablement

例如：

没有：

Next：

则：

Next：

disabled。

没有：

Previous：

则：

Previous：

disabled。

没有：

正在：

播放：

Play：

可以：

根据：

application：

定义：

是否：

可用。

---

# 87. Tray Click Handling

Tray：

菜单：

点击：

应该：

进入：

统一：

Application Command。

例如：

```text tray.play
→ PlaybackCommand::Play
```

不要：

tray：

callback：

直接：

触碰：

audio engine。

---

# 88. Tray Double Click

推荐：

双击：

tray：

icon：

聚焦：

Main Window。

如果：

当前：

窗口：

隐藏：

则：

show + focus。

---

# 89. Tray Left Click

必须：

明确：

产品：

行为。

建议：

> 左键：

显示：

tray menu。

或者：

> 左键：

聚焦：

主窗口。

最终：

根据：

Windows：

实际：

用户：

体验：

选择。

必须：

记录：

`DECISIONS.md`。

---

# 90. Tray Right Click

至少：

确保：

可以：

正常：

打开：

context menu。

---

# 91. Tray Menu Rebuild

不要：

每个：

playback：

position：

都：

重建：

整个：

menu。

只：

在：

必要：

状态：

发生：

变化：

时：

更新：

例如：

- track changed；
- playback state changed；
- queue boundary changed；
- language changed。

---

# 92. Tray Menu Update Frequency

禁止：

```text audio callback
→ tray menu update
```

必须：

低频：

application：

state：

驱动。

---

# 93. Tray Language

如果：

M6：

支持：

zh-CN / en：

tray：

菜单：

也：

应该：

跟随：

应用：

当前：

语言。

---

# 94. Language Change

语言：

变化：

只需：

更新：

menu：

text。

不需要：

重新：

创建：

tray：

icon。

---

# 95. Tray Now Playing

如果：

current track：

发生：

变化：

可以：

更新：

tray：

菜单：

的：

当前：

歌曲：

文本。

不要：

每：

frame：

更新。

---

# 96. Tray Menu Artwork

不要：

默认：

在：

tray：

menu：

中：

加载：

large artwork。

Windows：

tray：

应该：

保持：

轻量。

---

# 97. Close Window Policy

M10：

必须：

明确：

```text
Close window
vs
Exit application
```

语义。

---

# 98. Default Policy

推荐：

默认：

```text Close
→ Exit application
```

这样：

用户：

不会：

因为：

点击：

右上角：

关闭：

却：

发现：

程序：

仍：

偷偷：

后台：

运行。

---

# 99. Optional Tray Background Mode

[B]

可以：

增加：

设置：

```text Keep running in system tray when window is closed
```

默认：

建议：

关闭。

启用：

后：

```text Close main window
↓
Hide main window
↓
Playback continues
↓
Tray remains active
↓
SMTC remains active
```

---

# 100. Background Mode Semantics

如果：

后台：

运行：

必须：

明确：

用户：

能够：

从：

Tray：

执行：

`Exit`。

不能：

让：

用户：

找不到：

如何：

真正：

退出：

程序。

---

# 101. Close vs Minimize

明确：

```text Minimize
→ still running
```

无论：

是否：

Tray：

background：

模式。

---

# 102. Tray Exit

Tray：

`Exit`

必须：

触发：

> 正式：

Application Shutdown。

不是：

简单：

关闭：

Main Window。

---

# 103. Exit Semantics

Exit：

必须：

正确：

执行：

```text stop UI listeners
stop windows integration
unsubscribe SMTC
destroy tray
cancel pending integration tasks
flush necessary state
shutdown application
```

具体：

顺序：

根据：

application：

实际：

lifecycle：

确定。

---

# 104. SMTC Dispose

应用：

真正：

退出：

必须：

注销：

SMTC：

events。

保存：

event token：

或：

等价：

subscription：

handle。

---

# 105. Event Unsubscription

禁止：

：

```text app restart
→ subscribe SMTC
→ subscribe again
```

导致：

同一个：

Play：

执行：

2 次。

---

# 106. Tray Reinitialization

同样：

禁止：

重复：

创建：

tray：

icon。

必须：

具有：

> singleton ownership。

---

# 107. Single Instance

建议：

M10-A：

正式：

引入：

Tauri：

single-instance：

能力。

其作用：

> 防止同时启动多个 ifuyo Aura 进程，从而出现多个 Playback Authority、多个 tray、多个 SMTC integration。

---

# 108. Single Instance Policy

第二次：

启动：

不能：

创建：

第二：

audio：

engine。

必须：

> 将：

已有：

实例：

focus。

---

# 109. Second Instance

流程：

```text
Instance A running
↓
User starts ifuyo Aura again
↓
Instance B detected
↓
B exits
↓
A receives activation
↓
A focuses Main Window
```

---

# 110. No Duplicate SMTC

测试：

启动：

两个：

进程：

最终：

只能：

有：

> 一个：

active：

SMTC：

integration：

owner。

---

# 111. No Duplicate Tray

同理：

必须：

只有：

一个：

tray：

icon。

---

# 112. No Duplicate Audio Engine

必须：

再次：

证明：

第二：

instance：

不会：

创建：

第二：

audio：

engine。

---

# 113. Focus Existing Window

已有：

实例：

收到：

second-instance：

activation：

应该：

尝试：

```text show
restore
focus
```

如果：

窗口：

最小化：

应该：

restore。

---

# 114. Background Tray Mode + Second Instance

即使：

主：

窗口：

当前：

隐藏：

第二：

实例：

启动：

也：

必须：

将：

已有：

实例：

恢复：

或：

focus。

---

# 115. No New Instance Routing

本阶段：

不要：

实现：

复杂：

file association：

argument：

routing。

M10：

只：

保证：

single-instance：

与：

focus：

正确。

文件：

双击：

打开：

属于：

未来：

Windows Shell / release integration。

---

# 116. SMTC + Single Instance

Single Instance：

必须：

初始化：

早于：

Windows：

media：

session：

可能：

造成：

多个：

instance：

开始：

注册：

SMTC：

的：

阶段。

避免：

短暂：

重复：

媒体：

session。

具体：

根据：

Tauri：

实际：

lifecycle：

设计。

---

# 117. Application Lifecycle

M10：

重新：

明确：

```text Starting
Initializing
Ready
Background
Closing
Closed
```

---

# 118. Integration Initialization Order

建议：

```text
Application Start
↓
Single Instance
↓
Application Services
↓
Playback Authority
↓
Windows Integration
↓
SMTC
↓
Tray
↓
UI Ready
```

实际：

如果：

Tauri：

要求：

其它：

顺序：

必须：

按：

真实：

生命周期：

调整。

---

# 119. Integration Failure Isolation

任何：

Windows：

integration：

失败：

都：

不能：

让：

audio：

playback：

失败。

例如：

```text SMTC unavailable
```

仍：

必须：

允许：

```text Local playback
Library
Now Playing
```

正常：

使用。

---

# 120. Tray Failure

如果：

tray：

创建：

失败：

应用：

仍：

可以：

运行。

状态：

可以：

显示：

```text Tray = unavailable
```

不要：

让：

整个：

播放器：

退出。

---

# 121. Windows API Failure

任何：

Windows：

API：

error：

必须：

转换：

为：

明确：

application：

error：

或：

diagnostic。

不能：

大量：

`unwrap()`。

---

# 122. No Panic Boundary

Windows：

integration：

任何：

外部：

系统：

状态：

都：

不能：

直接：

panic：

应用。

---

# 123. Permission / Capability

M10：

重新：

审计：

Tauri：

capabilities。

确认：

tray：

所需：

权限：

最小化。

系统：

Windows：

API：

也：

只：

初始化：

必要：

能力。

---

# 124. Shell Security

继续：

禁止：

SMTC / Tray：

执行：

任意：

- PowerShell；
- cmd；
- shell；
- external executable。

---

# 125. External URL

Tray：

如果：

未来：

打开：

ifuyo.com：

仍：

必须：

调用：

系统：

浏览器。

本阶段：

没有：

必要：

无需：

增加：

外链。

---

# 126. No Remote Dependency

SMTC：

和：

Tray：

不能：

依赖：

互联网：

正常：

运行。

---

# 127. Offline

完全：

断网：

情况下：

M10：

必须：

正常：

提供：

- Playback；
- SMTC；
- Tray；
- local metadata；
- local artwork；
- queue。

网络：

radio：

当然：

需要：

网络：

这是：

M4：

行为。

---

# 128. Privacy

M10：

不得：

新增：

- telemetry；
- analytics；
- system usage upload；
- Windows diagnostic upload。

---

# 129. What Windows Sees

必须：

明确：

产品：

实际：

向：

Windows：

系统：

提供：

的：

metadata：

仅：

包括：

用户：

当前：

播放：

所需：

信息：

例如：

- title；
- artist；
- album；
- playback state；
- position；
- duration；
- optional artwork。

不要：

向：

Windows：

暴露：

- full library；
- complete listening history；
- embedding；
- analysis vectors；
- Music Map；
- Dreamscape data。

---

# 130. No Listening History Upload

M10：

不得：

将：

playback history：

发送：

第三方。

---

# 131. No Telemetry From SMTC

不要：

记录：

用户：

按：

了：

多少：

次：

Play：

然后：

上传。

本地：

diagnostics：

可以：

统计：

但：

不得：

离开：

本机。

---

# 132. Tray Privacy

tray：

菜单：

不要：

显示：

不必要：

的：

用户：

信息。

当前：

Track：

属于：

合理：

系统：

媒体：

体验：

但：

不要：

显示：

完整：

Library：

列表。

---

# 133. SMTC Metadata Sanitization

所有：

发送：

给：

Windows：

的：

metadata：

必须：

经过：

长度：

与：

非法：

字符：

检查。

例如：

极端：

长：

title：

不能：

导致：

异常：

memory：

行为。

---

# 134. String Limits

定义：

合理：

上限：

例如：

```text title
artist
album
station
stream title
```

每个：

都有：

bounded：

长度。

具体：

通过：

实际：

Windows：

API：

与：

UI：

需求：

决定。

---

# 135. Unicode

SMTC：

与：

Tray：

必须：

支持：

- 中文；
- 日文；
- 韩文；
- accented Latin；
- emoji；
- mixed-script。

不得：

出现：

乱码：

或：

lossy ASCII：

转换。

---

# 136. Special Characters

测试：

```text
&
<
>
"
'
:
/
\
|
—
…
emoji
```

不能：

破坏：

tray：

menu：

或者：

SMTC：

metadata。

---

# 137. Empty Metadata

如果：

artist：

为空：

不要：

显示：

`Unknown Artist`

除非：

产品：

已有：

统一：

fallback：

策略。

应该：

与：

M3/M6：

保持：

一致。

---

# 138. Current Track Snapshot

建立：

统一：

application：

snapshot：

例如：

```text WindowsPlaybackSnapshot
```

包含：

```text revision
track_id
title
artist
album
artwork
playback_state
position_frames
duration_frames
sample_rate
seekable
live
has_previous
has_next
```

实际：

字段：

根据：

真实：

architecture：

调整。

---

# 139. Single Source of Truth

SMTC：

和：

Tray：

都：

消费：

同一个：

snapshot。

不要：

各自：

从：

不同：

地方：

查询：

state。

---

# 140. Snapshot Update Trigger

只：

在：

实际：

state：

变化：

时：

更新：

system integration。

例如：

- track changed；
- playback state changed；
- seek completed；
- queue boundary changed；
- stream metadata changed；
- artwork changed。

---

# 141. Position Updates Are Special

position：

属于：

较高：

频率：

状态。

必须：

使用：

专门：

低频：

timer / snapshot：

机制。

不要：

每个：

React：

render：

触发。

---

# 142. Position Timer Ownership

应该：

属于：

Windows：

integration：

或：

application：

system integration：

层。

不得：

React：

页面：

负责：

SMTC：

position：

timer。

---

# 143. Timer Cleanup

退出：

应用：

必须：

取消：

position：

timer。

否则：

可能：

后台：

继续：

更新：

SMTC。

---

# 144. Timer Frequency

通过：

benchmark：

决定：

例如：

250–500ms：

级别：

但：

不能：

硬性：

认为：

所有：

机器：

都：

应该：

相同：

频率。

最终：

真实：

记录：

`DECISIONS.md`。

---

# 145. No Background Busy Loop

不得：

```text loop {
  update SMTC
}
```

无限：

高频：

占：

CPU。

---

# 146. Tray State Update

Tray：

状态：

应该：

由：

低频：

application：

events：

更新。

---

# 147. State Coalescing

如果：

短时间：

内：

发生：

```text
pause
resume
pause
```

tray：

与：

SMTC：

可以：

只：

提交：

最终：

稳定：

state。

但：

不能：

破坏：

command：

语义。

---

# 148. Event Ordering

必须：

建立：

> monotonic revision。

例如：

```text revision 41
revision 42
revision 43
```

Windows integration：

只：

接受：

最新：

snapshot。

---

# 149. Stale Snapshot

如果：

旧：

snapshot：

异步：

完成：

不得：

覆盖：

新：

snapshot。

---

# 150. Artwork Race

特别：

测试：

```text Track A
artwork A pending

Track B
artwork B ready

A becomes ready late
```

最终：

SMTC：

只能：

显示：

B。

---

# 151. Metadata Race

同理：

StreamTitle：

A：

到达：

晚：

不能：

覆盖：

B：

当前：

歌曲：

metadata。

---

# 152. Tray Race

```text Track A
↓
Track B
↓
A tray update finishes
```

最终：

tray：

只能：

显示：

B。

---

# 153. Queue Race

```text queue changes
↓
Next enabled
↓
queue changes again
```

最终：

tray：

与：

SMTC：

必须：

反映：

最新：

queue：

边界。

---

# 154. SMTC Command vs UI Command Race

例如：

```text SMTC Next
+
UI Pause
```

必须：

统一：

进入：

application：

command：

sequencing。

不要：

由：

不同：

线程：

直接：

修改：

playback：

内部：

状态。

---

# 155. Command Idempotency

例如：

```text Tray Play
SMTC Play
```

同时：

发生：

如果：

已经：

Playing：

结果：

必须：

稳定：

而：

不会：

建立：

异常：

状态。

---

# 156. Tray Menu Command IDs

所有：

tray：

menu：

item：

使用：

稳定：

IDs：

例如：

```text tray.open
tray.nowPlaying
tray.library
tray.map
tray.dreamscape

tray.playPause
tray.previous
tray.next

tray.miniPlayer
tray.exit
```

不要：

用：

显示：

文本：

作为：

逻辑：

ID。

---

# 157. Tray Menu Localization

显示：

文本：

根据：

locale：

动态：

提供。

逻辑：

ID：

永不：

随：

语言：

变化。

---

# 158. Tray Menu Test

切换：

```text zh-CN
→ en
→ zh-CN
```

Tray：

必须：

始终：

有：

正确：

menu。

---

# 159. Tray Menu Keyboard

Windows：

tray：

menu：

属于：

系统：

menu。

确保：

menu：

item：

不包含：

危险：

或：

错误：

shortcut：

文本。

---

# 160. Tray Exit Confirmation

默认：

不要：

显示：

多余：

confirmation：

dialog。

如果：

用户：

明确：

选择：

Exit：

就：

正常：

退出。

---

# 161. Closing With Unsaved State

M10：

不：

需要：

创造：

新的：

复杂：

save：

dialog。

应用：

应该：

先：

按：

现有：

M6/M7：

application：

lifecycle：

保存：

必要：

state。

---

# 162. Mini Player Integration

Tray：

`Mini Player`

必须：

调用：

M6：

已有：

window manager。

不要：

M10：

重新：

创建：

mini：

window。

---

# 163. Dreamscape Integration

Tray：

`Dreamscape`

如果：

点击：

应该：

使用：

M7：

已有：

entry point。

不要：

tray：

自己：

生成：

world。

---

# 164. Music Map Integration

Tray：

`Music Map`

调用：

M9：

已有：

route：

entry。

不要：

tray：

自己：

创建：

map。

---

# 165. Library Integration

Tray：

`Library`

直接：

focus：

M6：

Library view。

---

# 166. Now Playing Integration

tray：

`Now Playing`

直接：

focus：

M6：

Now Playing。

---

# 167. Window Focus

任何：

Tray：

打开：

操作：

最终：

应该：

保证：

```text window.show()
window.unminimize()
window.set_focus()
```

或：

当前：

Tauri：

等价：

API。

---

# 168. Window Hidden State

如果：

窗口：

隐藏：

Tray：

`Open`

必须：

恢复。

---

# 169. Main Window Already Focused

如果：

已经：

focus：

则：

不要：

重新：

创建：

任何：

资源。

---

# 170. Main Window Destroyed

如果：

main：

window：

意外：

销毁：

system integration：

必须：

能够：

安全：

进入：

closing：

状态。

---

# 171. Tray Survives Window Visibility

如果：

产品：

选择：

background tray：

模式：

tray：

不能：

依赖：

React：

DOM：

继续：

存在：

才：

工作。

---

# 172. SMTC Survives Hidden Window

同样：

如果：

主：

窗口：

隐藏：

SMTC：

仍：

应该：

根据：

后台：

playback：

state：

工作。

---

# 173. Main Window Closed + Exit Policy

必须：

区分：

```text user clicks X
```

与：

```text user chooses Exit
```

这是：

M10：

最重要：

的：

desktop：

UX：

语义：

之一。

---

# 174. Background Playback

如果：

background：

模式：

启用：

则：

关闭：

UI：

不得：

停止：

playback。

如果：

默认：

模式：

关闭：

则：

X：

直接：

结束：

应用。

两者：

不能：

混乱。

---

# 175. Audio Continuity

所有：

Windows：

integration：

操作：

不得：

导致：

- pause；
- seek；
- generation reset；
- queue mutation；

除非：

用户：

明确：

执行：

对应：

命令。

---

# 176. SMTC Metadata Update Must Not Affect Audio

更新：

SMTC：

metadata：

不能：

阻塞：

audio：

pipeline。

---

# 177. Tray Action Must Not Affect Callback

Tray：

callback：

不能：

运行：

在：

audio：

callback。

---

# 178. Windows API Boundary

任何：

Windows：

API：

调用：

必须：

位于：

Windows：

integration：

线程 / service：

而：

不是：

audio：

callback。

---

# 179. No Audio Callback Windows Calls

绝对：

禁止：

```text
audio callback
→ SMTC
audio callback
→ tray
audio callback
→ WinRT
audio callback
→ HWND
```

---

# 180. Performance

M10：

应：

接近：

> negligible overhead。

系统：

集成：

不能：

让：

播放器：

持续：

消耗：

明显：

CPU。

---

# 181. Tray Idle Cost

没有：

用户：

交互：

时：

Tray：

不得：

高频：

更新。

---

# 182. SMTC Idle Cost

暂停：

之后：

position：

更新：

应该：

停止：

或：

显著：

降低。

---

# 183. Position Updates While Paused

Paused：

state：

只：

保留：

最后：

position。

无需：

持续：

timer：

update。

---

# 184. Stopped State

Stopped：

可以：

完全：

停止：

timeline：

timer。

---

# 185. Live Stream

Live：

如果：

正在：

播放：

SMTC：

只：

维持：

必要：

metadata/status：

同步。

不要：

建立：

假的：

100Hz：

position：

更新。

---

# 186. Windows Integration Worker

可以：

建立：

专门：

的：

background：

worker：

处理：

system：

updates。

但：

不要：

创建：

巨大：

framework。

---

# 187. Integration Queue

如果：

使用：

command queue：

必须：

有界。

不能：

SMTC：

event：

异常：

后：

无限：

积累：

commands。

---

# 188. Integration Backpressure

如果：

短时间：

收到：

大量：

metadata：

updates：

应该：

coalesce：

而：

不是：

无限：

排队。

---

# 189. Memory

Windows：

integration：

的：

长期：

memory：

应该：

非常：

小：

且：

稳定。

---

# 190. Long Soak

至少：

执行：

> 数小时：

Windows integration：

soak。

期间：

持续：

执行：

```text Play
Pause
Next
Previous
Seek
Open Tray
Hide Window
Restore Window
Mini Player
Library
Map
Dreamscape
```

---

# 191. SMTC Soak

持续：

播放：

并：

反复：

切换：

tracks。

观察：

- CPU；
- RAM；
- event subscriptions；
- metadata updates；
- Windows API errors。

---

# 192. Tray Soak

循环：

```text open tray
click menu
open window
hide
repeat
```

数十：

到：

数百：

次。

确认：

没有：

- duplicate menu；
- duplicate icon；
- duplicate handlers。

---

# 193. Single Instance Soak

反复：

启动：

第二：

instance。

确认：

- second process exits；
- first process remains；
- focus works；
- no duplicate tray；
- no duplicate SMTC；
- no duplicate playback。

---

# 194. Fast Command Soak

连续：

通过：

Tray / SMTC：

执行：

```text
Play
Pause
Play
Next
Next
Previous
Pause
Play
```

确认：

最终：

playback state：

与：

application：

一致。

---

# 195. Seek Soak

快速：

执行：

```text 10s
30s
5s
60s
20s
```

确认：

最终：

position：

等于：

最后：

合法：

request。

---

# 196. Metadata Race Soak

快速：

模拟：

多个：

StreamTitle：

event。

确认：

最终：

只：

保留：

最新：

有效：

metadata。

---

# 197. Artwork Race Soak

连续：

Track A/B/C：

切换：

确认：

最终：

SMTC：

Artwork：

属于：

C。

---

# 198. Queue Boundary Soak

连续：

变化：

queue：

长度：

确认：

SMTC：

Previous/Next：

正确：

enablement。

---

# 199. Close / Reopen Soak

循环：

```text
show
hide
show
minimize
restore
close
open
```

按照：

实际：

background：

policy：

执行。

不得：

造成：

SMTC：

重复：

subscription。

---

# 200. Shutdown Soak

多次：

```text
play
exit
launch
play
exit
```

确认：

每次：

shutdown：

都：

正确：

清理。

---

# 201. Windows Sleep / Resume

[B]

如果：

环境：

允许：

测试：

Windows：

sleep：

与：

resume：

情况下：

验证：

SMTC / Tray / Audio：

恢复。

本阶段：

A：

至少：

不要：

因为：

系统：

resume：

造成：

crash。

---

# 202. Device Hotplug Regression

M2：

已经：

负责：

设备：

hotplug。

M10：

不得：

破坏：

其：

逻辑。

测试：

```text
Playing
↓
Device Removed
↓
SMTC = Paused / appropriate state
↓
Device Restored
↓
Playback recovers
```

具体：

状态：

以：

M2：

实际：

定义：

为准。

---

# 203. Hotplug + SMTC

不能：

出现：

```text audio device removed
but SMTC says Playing forever
```

系统：

media：

状态：

应该：

最终：

与：

Playback Authority：

一致。

---

# 204. Hotplug + Tray

Tray：

当前：

playback：

状态：

也：

必须：

最终：

同步。

---

# 205. Source Failure + SMTC

M4：

网络：

source：

失败：

SMTC：

不得：

长期：

保持：

错误：

Playing：

状态。

---

# 206. Cache Failure + SMTC

M5：

缓存：

失败：

如果：

最终：

playback：

仍：

能够：

stream：

SMTC：

保持：

真实：

state。

如果：

playback：

失败：

SMTC：

也：

必须：

同步：

error/stopped：

语义。

---

# 207. Internet Radio + SMTC

必须：

真实：

测试：

local：

ICY：

fixture：

而：

不是：

公网：

电台。

---

# 208. Internet Radio + Tray

Tray：

应该：

可以：

显示：

station：

或：

stream：

metadata。

不要：

显示：

假的：

track：

duration。

---

# 209. Radio Metadata Update

如果：

M4：

发：

```text Artist A
Title A
```

再：

```text Artist B
Title B
```

SMTC：

最终：

应：

显示：

B。

---

# 210. Library Track + SMTC

Local：

歌曲：

必须：

显示：

完整：

metadata。

---

# 211. HTTP Track + SMTC

Direct HTTP：

finite：

track：

应：

同样：

能够：

显示：

metadata，

前提：

application：

真实：

拥有：

metadata。

没有：

则：

不猜。

---

# 212. Analysis Independence

M8：

分析：

数据：

不能：

自动：

进入：

SMTC。

不要：

在：

系统：

media：

面板：

显示：

```text BPM
Energy
Embedding
```

SMTC：

只：

承担：

Windows：

标准：

媒体：

信息。

---

# 213. Music Map Independence

M9：

map：

数据：

不：

进入：

tray：

默认：

菜单。

Tray：

只有：

`Open Music Map`。

---

# 214. Dreamscape Independence

Dreamscape：

运行：

时：

SMTC：

仍然：

服务：

真实：

playback。

---

# 215. Aurora Independence

Aurora：

运行：

与：

否：

不：

影响：

SMTC。

---

# 216. M7/M9 Visualizer Coordination

M10：

不得：

修改：

visual：

renderer：

生命周期。

只：

通过：

已有：

application：

commands：

打开：

对应：

feature。

---

# 217. Command Palette

M10：

可以：

注册：

少量：

system：

commands：

例如：

```text Open Tray Settings
Enable Background Mode
```

但：

不要：

创建：

大量：

system：

commands。

---

# 218. Existing Commands

继续：

复用：

M6：

Command Registry。

Tray：

也：

尽可能：

调用：

同一：

command：

executor。

---

# 219. Command Authority

例如：

```text Command Palette
→ Open Music Map

Tray
→ Open Music Map
```

两者：

必须：

最终：

调用：

相同：

application：

command。

---

# 220. Accessibility

Tray：

属于：

Windows：

native：

菜单：

本身：

不需要：

React：

Accessibility。

但：

主：

UI：

仍：

必须：

遵循：

M6：

accessible：

架构。

---

# 221. SMTC Accessibility

SMTC：

metadata：

必须：

清晰：

正确。

不要：

使用：

内部：

TrackId：

作为：

用户：

看到：

的：

title。

---

# 222. System UI Language

SMTC：

系统：

UI：

本身：

由：

Windows：

控制。

应用：

只：

提供：

正确：

metadata。

不要：

试图：

改变：

Windows：

SMTC：

UI：

语言。

---

# 223. No Custom SMTC UI

不要：

重现：

Windows：

系统：

媒体：

面板：

作为：

ifuyo：

自己的：

UI。

---

# 224. No SMTC Duplicate

M10：

不要：

创建：

另一套：

“Media Center”：

页面。

系统：

已有：

SMTC。

---

# 225. Windows Media Session Diagnostics

允许：

本地：

diagnostics：

显示：

```text SMTC available
SMTC state
current track
supported buttons
seekable
last revision
last update
last error
```

仅：

本地。

---

# 226. Tray Diagnostics

也：

可以：

显示：

```text Tray initialized
menu version
visible
background mode
```

仅：

本地。

---

# 227. Diagnostics Must Not Affect State

diagnostics：

不能：

成为：

system：

integration：

source of truth。

---

# 228. Security — SMTC

检查：

- malformed metadata；
- huge strings；
- invalid position；
- unsupported command；
- stale revision；
- repeated commands。

---

# 229. Security — Tray

检查：

- menu command IDs；
- lifecycle；
- untrusted metadata；
- localization strings；
- malformed state。

不要：

让：

Track metadata：

被：

解析：

为：

tray：

命令。

---

# 230. Security — Single Instance

检查：

second-instance：

activation：

payload：

不要：

接受：

未验证：

任意：

外部：

命令。

---

# 231. No Arbitrary Command From Second Instance

本阶段：

第二：

instance：

只：

执行：

最小：

允许：

动作：

例如：

focus：

main：

window。

不要：

允许：

外部：

process：

直接：

传：

任意：

JSON：

command：

让：

程序：

执行。

---

# 232. Security — Windows API

Windows：

外部：

events：

必须：

视为：

untrusted：

input。

任何：

position：

time：

count：

都：

需要：

validation。

---

# 233. Integer Safety

特别：

检查：

Windows：

time：

unit：

与：

Audio：

frame：

之间：

的：

转换。

不得：

overflow。

---

# 234. Float Safety

如果：

SMTC：

位置：

转换：

涉及：

seconds：

float：

则：

必须：

验证：

finite。

---

# 235. Path Safety

Artwork：

路径：

必须：

受：

M3/M6：

现有：

路径：

policy：

保护。

M10：

不得：

创造：

任意：

file：

access。

---

# 236. No User Path Injection

Tray：

或者：

SMTC：

metadata：

不得：

将：

用户：

track：

path：

当作：

Windows：

resource：

直接：

打开。

---

# 237. Dependency Audit

新增：

依赖：

必须：

审计：

- license；
- MSRV；
- Windows；
- CI；
- binary size；
- maintenance；
- security。

---

# 238. Tauri Plugins

如果：

使用：

- tray；
- single-instance；
- global-shortcut；

必须：

确认：

当前：

Tauri：

版本：

已有：

实现。

不要：

重复：

引入：

自定义：

Windows：

库。

---

# 239. Global Shortcut

如果：

B：

实现：

custom：

global：

media：

shortcuts：

可以：

使用：

当前：

Tauri：

global-shortcut：

能力；

官方当前：

插件：

支持：

Windows，

且：

注册：

shortcut：

后：

通过：

handler：

收到：

事件。

但：

不要：

为了：

媒体键：

而：

无条件：

注册：

大量：

全局：

快捷键。

SMTC：

优先。

---

# 240. Global Shortcut Opt-in

B：

custom：

global：

shortcut：

应该：

为：

> 用户：

明确：

开启：

的：

功能。

不要：

默认：

抢占：

系统：

常用：

快捷键。

---

# 241. Media Key Priority

媒体：

Play/Pause：

Next：

Previous：

优先：

SMTC。

不要：

通过：

全局：

shortcut：

与：

SMTC：

同时：

响应：

同一：

按键：

造成：

double command。

---

# 242. No Shortcut Conflict

如果：

某：

global：

shortcut：

已经：

被：

其它：

应用：

占用，

必须：

graceful：

失败。

不能：

因此：

影响：

playback。

---

