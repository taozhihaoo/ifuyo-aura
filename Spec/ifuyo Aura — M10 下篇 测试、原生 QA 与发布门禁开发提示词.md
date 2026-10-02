# ifuyo Aura
# M10 下篇 — Documentation / Testing / Native QA / Release Gates (§243–411)

> 本文档为原 M10 开发提示词的**下篇（文档同步、测试、原生 QA 与发布门禁，§243–411）**，由《ifuyo Aura — M10 上篇 SMTC、媒体键、系统托盘与单实例实现开发提示词.md》拆分而来。
> 上篇承载实现内容（§0–242）。章节编号沿用原 M10 全局编号；跨篇引用按编号在两篇中查找。
> 阅读本文前，上篇所定义的实现必须已完成或在收尾。

# 243. Documentation — Architecture

更新：

`ARCHITECTURE.md`

加入：

```text
M10 Windows Integration
Windows Integration Coordinator
SMTC Adapter
SMTC Metadata
SMTC Commands
Timeline Updates
Tray
Single Instance
Application Lifecycle
Background Mode
System Integration Event Flow
```

---

# 244. Documentation — Decisions

`DECISIONS.md`

至少：

记录：

- Windows integration ownership；
- SMTC implementation strategy；
- Windows API binding；
- SMTC lifecycle；
- supported buttons；
- playback status mapping；
- seek policy；
- position update frequency；
- timeline conversion；
- artwork source；
- artwork validation；
- ICY metadata updates；
- stale revision policy；
- tray ownership；
- tray icon；
- tray menu；
- tray click behavior；
- tray language；
- close behavior；
- background tray behavior；
- single-instance strategy；
- second-instance focus；
- global shortcut decision；
- Windows minimum version；
- failure fallback；
- performance policy。

---

# 245. Documentation — Security

更新：

`SECURITY.md`

加入：

```text
SMTC Input Validation
Tray Command Validation
Second-Instance Validation
Artwork Path Validation
Windows API Error Isolation
Timeline Conversion Safety
Metadata Length Limits
No Arbitrary Shell Execution
```

---

# 246. Documentation — Privacy

更新：

`PRIVACY.md`

明确：

M10：

不会：

上传：

- SMTC interactions；
- tray interactions；
- current track；
- playback state；
- listening history。

这些：

仅：

在：

Windows：

本地：

系统：

媒体：

集成：

需要：

的：

范围：

内：

使用。

---

# 247. Documentation — Sources

更新：

`SOURCES.md`

记录：

实际：

使用：

的：

- Microsoft SMTC API；
- Microsoft Windows APIs；
- Tauri tray API；
- Tauri single-instance plugin；
- global shortcut plugin（如果 B 完成）；
- 相关：

crate：

版本与：

license。

---

# 248. Documentation — Third Party Licenses

更新：

`THIRD_PARTY_LICENSES.md`

所有：

新增：

依赖：

必须：

真实：

记录：

- package；
- version；
- license；
- purpose；
- source；
- commercial redistribution status。

---

# 249. Documentation — Progress

开始：

```text
M10-A = IN PROGRESS
M10-B = NOT STARTED
```

完成后：

记录：

- SMTC；
- media keys；
- metadata；
- artwork；
- timeline；
- tray；
- single-instance；
- background mode；
- tests；
- Windows verification；
- soak；
- known limitations。

---

# 250. Test Architecture

M10：

测试：

必须：

分为：

```text Pure Unit
Application Integration
Windows Mock
Windows Native
Manual Windows QA
```

---

# 251. Pure Unit Tests

测试：

- metadata mapping；
- playback state mapping；
- supported button calculation；
- seek validation；
- timeline conversion；
- revision ordering；
- stale update rejection；
- tray menu command mapping；
- localization mapping。

---

# 252. Metadata Mapping Tests

例如：

```text title
artist
album
album artist
```

测试：

- empty；
- Unicode；
- long strings；
- special characters；
- emoji。

---

# 253. Playback State Tests

至少：

```text Playing
Paused
Stopped
Error
Live
Buffering
```

根据：

真实：

application：

state：

测试：

SMTC：

映射。

---

# 254. Seek Validation Tests

至少：

```text negative
zero
inside duration
exact duration
beyond duration
unknown duration
non-seekable
NaN-equivalent invalid
overflow
```

---

# 255. Timeline Conversion Tests

至少：

```text 44.1kHz
48kHz
96kHz
0 frames
1 frame
1 minute
1 hour
very long duration
```

确认：

不会：

overflow。

---

# 256. Button Enablement Tests

测试：

```text no previous
previous available
no next
next available
seekable
non-seekable
playing
paused
stopped
```

---

# 257. Revision Tests

测试：

```text revision 1
revision 2
revision 3
```

旧：

revision：

不能：

覆盖：

新：

状态。

---

# 258. Track Race Tests

模拟：

```text Track A
Track B
late A update
```

最终：

B：

必须：

继续：

为：

current：

state。

---

# 259. Artwork Race Tests

模拟：

```text A artwork pending
B artwork complete
A artwork complete
```

最终：

B。

---

# 260. Metadata Race Tests

模拟：

```text A metadata
B metadata
late A event
```

最终：

B。

---

# 261. Tray Mapping Tests

测试：

```text Play
Pause
Next
Previous
Open
Mini Player
Exit
```

每个：

都：

对应：

正确：

Application：

Command。

---

# 262. Tray Enablement Tests

测试：

queue：

边界：

变化：

后：

menu：

正确：

enable/disable。

---

# 263. Tray Localization Tests

至少：

```text zh-CN
en
```

切换：

并：

验证：

所有：

menu：

文本。

---

# 264. Single Instance Mock Tests

模拟：

```text instance A
instance B
```

确认：

B：

只：

向：

A：

发送：

最小：

activation：

signal。

---

# 265. SMTC Mock

建立：

Fake：

SMTC：

interface：

用于：

测试：

application：

与：

Windows：

integration：

之间：

的：

交互。

---

# 266. Tray Mock

同理：

Fake：

Tray：

测试：

可以：

不用：

真正：

Windows：

shell。

---

# 267. No Windows Required Unit Tests

大多数：

unit：

tests：

必须：

能够：

在：

非：

Windows：

环境：

运行：

如果：

Rust：

架构：

允许。

---

# 268. Windows Native Tests

至少：

在：

真实：

Windows：

环境：

执行：

```text SMTC initialization
SMTC button press
SMTC metadata
SMTC artwork
SMTC position
SMTC seek
Tray creation
Tray menu
Tray click
Single instance
```

---

# 269. Native SMTC Test

真实：

流程：

```text Launch
↓
Play local track
↓
Open Windows system media controls
↓
Verify title
↓
Verify artist
↓
Verify album
↓
Verify playback status
```

---

# 270. Native Media Button Test

真实：

使用：

Windows：

媒体：

键：

执行：

```text Play/Pause
Next
Previous
```

确认：

command：

真实：

生效。

---

# 271. Native Seek Test

如果：

系统：

媒体：

UI：

提供：

position：

改变：

必须：

验证：

实际：

seek：

发生。

---

# 272. Native Non-Seekable Test

使用：

local：

fake：

ICY：

server。

确认：

seek：

不可：

操作：

或：

被：

安全：

拒绝。

---

# 273. Native Metadata Test

测试：

中文：

```text
流光之外
```

英文：

```text
Beyond the Night
```

mixed：

```text
夜 / Night / 夜
```

确认：

Windows：

media：

UI：

显示：

正确：

Unicode。

---

# 274. Native Artwork Test

至少：

- artwork exists；
- artwork missing；
- artwork corrupt；
- artwork replaced；
- track changed。

---

# 275. Tray Native Test

验证：

```text tray icon visible
menu opens
Play works
Pause works
Next works
Previous works
Open works
Mini Player works
Exit works
```

---

# 276. Tray Close Test

默认：

close：

如果：

background mode：

关闭：

则：

应用：

真正：

退出。

如果：

B：

开启：

background mode：

则：

窗口：

隐藏：

但：

playback：

继续。

两种：

行为：

必须：

有：

测试。

---

# 277. Single Instance Native Test

执行：

```text ifuyo Aura
ifuyo Aura again
```

确认：

- only one window authority；
- one tray；
- one playback；
- one SMTC integration。

---

# 278. Restore From Tray

测试：

```text Main Window hidden
↓
tray Open
```

确认：

- window shown；
- focused；
- state preserved。

---

# 279. Restore From Second Instance

测试：

```text A running hidden
↓
start B
```

A：

必须：

恢复：

或：

focus。

---

# 280. Repeated Startup Test

连续：

启动：

应用：

数十：

次。

确认：

没有：

- duplicate tray；
- stale SMTC；
- zombie process；
- broken instance lock。

---

# 281. Crash Recovery

模拟：

应用：

异常：

退出。

重新：

启动：

确认：

SMTC：

与：

tray：

能够：

重新：

创建。

不会：

依赖：

旧：

process：

状态。

---

# 282. SMTC Disconnect / Error

如果：

Windows：

API：

出现：

错误：

应：

进入：

graceful：

`Unavailable / Failed`

然后：

主：

播放器：

仍：

继续。

---

# 283. Tray Reconnect

如果：

tray：

临时：

创建：

失败：

应用：

应该：

至少：

允许：

继续：

运行。

如果：

当前：

Tauri：

无法：

安全：

重新：

创建：

则：

明确：

记录：

limitation，

不能：

伪造：

PASS。

---

# 284. Window Minimize

测试：

```text play
minimize
SMTC
tray
restore
```

确认：

播放：

不：

中断。

---

# 285. Window Hidden

background：

模式：

下：

测试：

```text hide
SMTC Play/Pause
Tray Next
Tray Previous
restore
```

状态：

必须：

一致。

---

# 286. Audio Device Hotplug

测试：

```text Playing
Device removed
SMTC
Tray
Device restored
Resume
```

确认：

状态：

最终：

一致。

---

# 287. Radio Source Failure

测试：

local：

fake：

HTTP：

模拟：

stream：

failure。

确认：

SMTC：

最终：

停止：

或：

进入：

实际：

error：

状态。

---

# 288. Network Metadata Test

local：

fake：

ICY：

server：

发送：

```text StreamTitle A
StreamTitle B
StreamTitle C
```

确认：

SMTC：

最终：

C。

---

# 289. Long Metadata Test

发送：

极长：

StreamTitle。

确认：

- bounded；
- sanitized；
- no crash；
- no excessive memory。

---

# 290. Invalid Metadata Test

发送：

- malformed UTF-8；
- invalid delimiters；
- huge metadata；
- empty metadata。

确认：

SMTC：

不会：

panic。

---

# 291. Performance Benchmark

更新：

`docs/PERF.md`

至少：

记录：

```text SMTC initialization
SMTC metadata update
SMTC timeline update
Tray creation
Tray state update
Single-instance startup overhead
```

---

# 292. Position Update Benchmark

实际：

记录：

不同：

update：

频率：

下：

CPU：

消耗。

不要：

默认：

写：

“几乎没有性能成本”

而：

没有：

测量。

---

# 293. Tray Update Benchmark

测量：

大量：

Track：

切换：

时：

tray：

update：

开销。

---

# 294. Startup Benchmark

分别：

记录：

```text normal startup
single-instance detection
SMTC initialization
tray initialization
first meaningful UI
```

---

# 295. Memory Audit

重点：

长时间：

运行：

确认：

没有：

持续：

增长：

来自：

- SMTC event subscriptions；
- tray menu objects；
- timer；
- artwork references；
- Windows adapter；
- single-instance handles。

---

# 296. Renderer Regression

M10：

不能：

因为：

tray：

或：

SMTC：

导致：

Aurora：

renderer：

重复。

---

# 297. WebGL Regression

确认：

```text open Now Playing
SMTC
Tray
hide
restore
```

不会：

创建：

第二：

Aurora：

renderer。

---

# 298. Audio Regression

必须：

确认：

M10：

操作：

不：

进入：

audio：

callback。

---

# 299. Full Playback Regression

重新：

验证：

M1/M2：

- decoder；
- resampler；
- ring buffer；
- play；
- pause；
- stop；
- seek；
- generation；
- gapless；
- crossfade；
- loudness；
- limiter；
- hotplug；
- EQ；
- zero allocation；
- deterministic rendering。

---

# 300. M3 Regression

确认：

- scanner；
- metadata；
- identity；
- rename；
- move；
- SQLite；
- FTS5；
- artwork；
- watcher；
- pagination；
- playlists if present；
- stats。

---

# 301. M4 Regression

确认：

- local；
- HTTP；
- HTTPS；
- ICY；
- Radio Browser；
- source security；
- credential URL rejection；
- unsupported platform zero-request；
- cache policy。

---

# 302. M5 Regression

确认：

- Allowed；
- SessionOnly；
- Forbidden；
- Range；
- persistence；
- revalidation；
- LRU；
- pin；
- recovery；
- offline cache。

---

# 303. M6 Regression

确认：

- Tauri shell；
- Now Playing；
- Library；
- Queue；
- Command Palette；
- Mini Player；
- Aurora；
- settings；
- i18n；
- accessibility；
- reduced motion。

---

# 304. M7 Regression

确认：

- Dreamscape；
- seed；
- generation；
- persistence；
- mutation；
- replay；
- renderer；
- reduced motion。

---

# 305. M8 Regression

确认：

- BPM；
- Key；
- LUFS；
- spectral；
- MFCC；
- embedding；
- persistence；
- scheduler；
- cancellation；
- model validation；
- offline。

---

# 306. M9 Regression

确认：

- Music Map；
- similarity；
- neighbors；
- clusters；
- layout；
- persistence；
- viewport；
- LOD；
- search；
- filters；
- selection；
- playback integration。

---

# 307. No Cross-Milestone Contamination

M10：

必须：

证明：

没有：

- M11 updater；
- installer；
- Steam；
- cloud；
- account；
- telemetry。

---

# 308. No Duplicate Playback State

diff：

重点：

检查：

有没有：

```text smtcPlaybackState
trayPlaybackState
```

成为：

第二：

套：

source-of-truth。

允许：

Windows adapter：

持有：

> last applied snapshot

但：

它必须：

是：

derived：

state。

---

# 309. No Duplicate Queue

Tray：

不得：

拥有：

第二：

queue。

SMTC：

也：

不得：

拥有：

第二：

queue。

---

# 310. No Duplicate Library

SMTC：

不：

查询：

SQLite。

Tray：

不：

维护：

Library：

缓存：

列表。

---

# 311. No Duplicate Artwork Cache

M10：

不：

新增：

第二：

套：

artwork：

缓存：

系统。

---

# 312. No Duplicate IPC

Windows：

integration：

不：

需要：

通过：

React：

再：

转：

给：

Rust：

执行：

command。

正确：

```text Windows
↓
Rust Application
```

UI：

另：

走：

```text React
↓
Tauri
↓
Rust
```

两者：

最终：

汇合：

到：

同一：

Application：

command。

---

# 313. Windows Integration vs UI

最终：

必须：

保持：

```text                    ┌─ React UI
Playback Authority ──────┼─ Mini Player
                    │    ├─ Tray
                    │    └─ SMTC
```

而：

不是：

```text React
 ↓
Tray
 ↓
SMTC
 ↓
Playback
```

---

# 314. Architecture Direction

依赖：

必须：

保持：

```text UI
   ↓
Application
   ↓
Windows Adapter
```

而：

不是：

```text aura-core
   ↓
Windows
```

---

# 315. No `windows` in aura-core

除非：

存在：

极其：

明确：

且：

已经：

架构：

批准：

的：

低层：

抽象：

原因，

否则：

`aura-core`

不得：

依赖：

Windows：

crate。

---

# 316. No Tauri in aura-core

同样：

严格：

禁止。

---

# 317. No WebView in SMTC

SMTC：

不得：

依赖：

WebView：

DOM。

---

# 318. No WebView in Tray

Tray：

也：

不得：

通过：

WebView：

模拟：

系统：

菜单。

---

# 319. Native Integration Priority

Windows：

系统：

能力：

优先：

使用：

native：

API / Tauri：

正式：

能力：

而：

不是：

DOM：

hack。

---

# 320. Documentation Quality

README：

增加：

真实：

Windows：

integration：

说明：

例如：

```text
Windows media controls
System tray
Media keys
Single-instance application
```

如果：

某：

B：

功能：

未完成：

必须：

明确：

未完成。

---

# 321. README 不要夸大

不要：

写：

```text
Full Windows integration
Perfect media key support
Zero resource usage
```

除非：

有：

真实：

证据。

---

# 322. User-facing Copy

如果：

需要：

settings：

文案：

保持：

ifuyo：

风格：

```text
Run in system tray when the main window is closed
```

中文：

例如：

> 关闭主窗口后继续驻留系统托盘

但：

不要：

过度：

营销。

---

# 323. Settings

A：

建议：

增加：

```text Keep running in system tray when window is closed
```

默认：

`false`

如果：

当前：

M6：

设置：

架构：

允许。

---

# 324. Settings Persistence

使用：

M6：

已有：

application：

settings：

storage。

不要：

创建：

第二：

Windows：

settings：

数据库。

---

# 325. Setting Version

如果：

新增：

字段：

必须：

遵守：

M6：

已有：

settings：

migration：

机制。

---

# 326. Tray Background Mode

如果：

用户：

关闭：

该：

设置：

主：

窗口：

X：

应该：

恢复：

默认：

exit：

语义。

---

# 327. Startup Behavior

M10：

默认：

不要：

自动：

startup：

到：

tray。

如果：

未来：

需要：

自启动：

放：

B/M11：

单独：

设计。

---

# 328. No Silent Startup

不要：

注册：

Windows：

startup：

而：

不在：

UI：

告知：

用户。

---

# 329. Single Instance UX

用户：

双击：

两次：

应用：

不应该：

看到：

两个：

窗口：

或者：

两个：

播放器。

---

# 330. Second Instance Focus UX

如果：

第二：

instance：

被：

启动：

已有：

实例：

应该：

快速：

focus。

不要：

弹：

错误：

对话框：

说：

“程序已在运行”

然后：

让：

用户：

自己：

找：

旧：

窗口。

---

# 331. Focus Safety

如果：

第一：

instance：

处于：

Mini Player：

或：

Dreamscape：

状态：

second instance：

默认：

应该：

focus：

Main Window。

不要：

强制：

切换：

到：

Dreamscape：

或：

Mini Player。

---

# 332. Focus Restore

如果：

Main Window：

在：

另：

一个：

monitor：

仍：

存在，

应该：

正常：

restore。

如果：

窗口：

状态：

无效：

遵循：

M6：

已有：

window：

restore：

policy。

---

# 333. Single Instance + Portable Dev

开发：

模式：

同样：

应：

能：

测试：

single-instance。

测试：

不要：

要求：

发行：

installer。

---

# 334. Tauri Build

必须：

在：

真实：

Windows：

环境：

运行：

项目：

现有：

Tauri：

build。

---

# 335. Tauri Build Gate

执行：

```powershell
npm run build
tauri build
```

或者：

当前：

项目：

等价：

命令。

如果：

当前：

环境：

无法：

完成：

真实：

Windows：

build：

必须：

明确：

`BLOCKED`。

不能：

伪造：

PASS。

---

# 336. Native Windows QA

必须：

在：

真实：

Windows：

机器：

至少：

手动：

验证：

- SMTC；
- Tray；
- media keys；
- single instance；
- close behavior；
- hidden background mode（若 B）。

---

# 337. Windows Version Matrix

至少：

根据：

项目：

支持：

矩阵：

测试：

```text Windows 10
Windows 11
```

如果：

实际：

CI：

没有：

所有：

Windows：

版本，

必须：

明确：

哪些：

真实：

测试：

完成。

---

# 338. DPI Matrix

至少：

测试：

```text 100%
125%
150%
200%
```

确认：

Tray：

icon：

正常。

Main：

window：

正常。

Mini：

window：

正常。

---

# 339. Multi-Monitor

至少：

测试：

- Main window monitor A；
- Tray interaction；
- restore；
- move；
- second instance focus。

不要：

让：

Windows：

integration：

破坏：

M6：

window：

state。

---

# 340. System Theme

至少：

测试：

```text Light Windows
Dark Windows
```

Tray：

icon：

必须：

可：

识别。

如果：

icon：

需要：

不同：

variant：

必须：

合理：

选择。

---

# 341. Windows Explorer Relaunch

[B]

如果：

explorer.exe：

重启：

可以：

检查：

Tray：

是否：

可：

恢复：

或：

已知：

限制：

是否：

明确：

记录。

不要：

伪造：

完美支持。

---

# 342. Media Center Verification

真实：

验证：

至少：

检查：

Windows：

系统：

媒体：

控件：

是否：

显示：

正确：

metadata。

---

# 343. No Unsupported Windows Claims

如果：

某个：

SMTC：

特性：

只：

在：

某些：

Windows：

版本：

存在：

必须：

文档：

真实：

说明。

---

# 344. Manual Evidence

最终：

必须：

收集：

实际：

证据：

例如：

```text
M10-SMTC-01
M10-SMTC-02
M10-TRAY-01
M10-TRAY-02
M10-SINGLE-01
...
```

具体：

编号：

可以：

调整。

---

# 345. Screenshots

至少：

准备：

```text
m10-smtc-now-playing.png
m10-smtc-metadata.png
m10-smtc-controls.png
m10-tray-menu.png
m10-tray-playing.png
m10-single-instance.png
```

截图：

必须：

来自：

真实：

当前：

Windows：

build。

---

# 346. No Fake System UI Screenshot

禁止：

用：

Photoshop：

或者：

UI：

mock：

伪装：

Windows：

SMTC：

截图。

必须：

来自：

真实：

系统：

UI。

---

# 347. Test Data Policy

真实：

用户：

音乐：

不得：

提交：

Git。

可以：

使用：

合法：

tiny：

fixture。

---

# 348. No Private Metadata Submission

截图：

如果：

包含：

用户：

个人：

文件名：

不得：

提交：

仓库。

可以：

使用：

synthetic：

fixture。

---

# 349. Local Fake Radio

M10：

Radio：

测试：

必须：

继续：

使用：

M4：

local：

fake：

ICY：

server。

禁止：

公网：

电台：

作为：

CI：

测试。

---

# 350. No Network Test

SMTC / Tray：

核心：

测试：

不得：

依赖：

互联网。

---

# 351. Offline Native Test

完全：

断网：

情况下：

应该：

能够：

测试：

```text Local track
→ Playback
→ SMTC
→ Tray
```

---

# 352. Test Fixture

建议：

准备：

```text
M10LocalTrack
M10RadioFixture
M10ArtworkFixture
M10LongMetadataFixture
```

全部：

合法：

且：

明确：

`TEST ONLY`。

---

# 353. Extended Soak

至少：

进行：

> 数小时：

本地：

Windows：

desktop：

soak。

循环：

```text Play
Pause
Seek
Next
Previous
Open Tray
Hide Window
Restore
Switch Library
Open Map
Open Dreamscape
Open Mini Player
Exit
Restart
```

---

# 354. Soak Metrics

记录：

- RAM；
- CPU；
- process count；
- thread count；
- SMTC subscription count；
- tray state；
- timer count；
- WebView count；
- GPU resource count。

如果：

某个：

指标：

无法：

精确：

测：

必须：

明确：

`NOT MEASURED`，

不能：

伪造：

数字。

---

# 355. Zombie Process Audit

关闭：

应用：

后：

检查：

没有：

残留：

ifuyo：

process。

如果：

background：

模式：

开启：

则：

必须：

按：

定义：

正常：

保留。

---

# 356. Zombie Timer Audit

退出：

后：

不能：

继续：

SMTC：

position：

timer。

---

# 357. Zombie Event Audit

退出：

后：

不能：

继续：

处理：

SMTC：

events。

---

# 358. Zombie Tray Audit

退出：

后：

tray：

icon：

必须：

消失。

---

# 359. Zombie Window Audit

退出：

后：

不得：

残留：

Mini Player：

window。

---

# 360. Crash Boundary

Windows：

integration：

crash：

不得：

导致：

Library：

DB：

corruption。

不得：

导致：

Cache：

DB：

corruption。

不得：

导致：

Analysis：

DB：

corruption。

---

# 361. Application Shutdown

最终：

必须：

建立：

清晰：

shutdown：

顺序：

```text
Stop accepting external system commands
↓
Stop SMTC position timer
↓
Unsubscribe SMTC events
↓
Destroy tray
↓
Stop integration workers
↓
Application shutdown
```

实际：

如果：

Playback：

应该：

先：

安全：

停止，

则：

按：

真实：

application：

lifecycle：

调整。

---

# 362. Don't Kill Playback From Tray Callback

Tray：

Exit：

不能：

直接：

`process::exit()`：

绕过：

application：

shutdown。

应该：

走：

正式：

shutdown：

路径。

---

# 363. Don't Kill Process From SMTC

SMTC：

事件：

绝不：

直接：

结束：

process。

---

# 364. Error Recovery

如果：

SMTC：

初始化：

失败：

可以：

重试：

一次：

或：

有限：

次数：

但：

不得：

无限：

retry。

---

# 365. Tray Recovery

同理：

tray：

初始化：

失败：

如果：

当前：

Tauri：

能力：

允许：

可以：

有限：

retry。

不能：

后台：

永远：

重试：

每：

1s。

---

# 366. Backoff

系统：

integration：

若：

需要：

retry：

必须：

bounded：

backoff。

---

# 367. No Busy Repair Loop

禁止：

```text every 10ms:
try SMTC
try Tray
```

这种：

repair：

loop。

---

# 368. Integration Health

允许：

本地：

health：

状态：

```text
SMTC = Ready
Tray = Ready
SingleInstance = Ready
```

---

# 369. Health Semantics

状态：

必须：

来自：

真实：

implementation。

不能：

初始化：

成功：

就：

永远：

显示：

`Ready`。

---

# 370. Final Security Scan

重新：

搜索：

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

并：

检查：

- Windows API；
- tray；
- second instance；
- artwork path；
- metadata；
- command IDs；
- position conversion。

---

# 371. Dependency Audit

执行：

```powershell
cargo tree
cargo deny check
npm ls
```

重点：

检查：

- `windows`；
- Tauri plugins；
- tray；
- single-instance；
- global-shortcut（如果 B）；
- any Windows helper crate。

---

# 372. No License Contamination

继续：

M0：

硬规则：

不得：

加入：

```text GPL
AGPL
LGPL
```

---

# 373. Source Verification

对于：

Windows：

API：

使用：

当前：

官方：

Microsoft：

文档：

作为：

reference。

SMTC：

官方：

文档：

明确：

支持：

系统：

媒体：

控制：

与：

媒体：

metadata：

集成；SMTC 也提供：

按钮：

事件：

用于：

接收：

Play/Pause/Next 等：

系统：

操作。

---

# 374. Tray Documentation Verification

当前：

Tauri：

tray：

API：

支持：

创建：

tray：

icon：

以及：

menu：

和：

click：

事件；

实际：

实现：

必须：

以：

仓库：

所使用：

的：

Tauri：

版本：

为：

准。

---

# 375. Architecture Decision

如果：

当前：

Tauri：

已有：

tray：

能力：

优先：

复用。

如果：

SMTC：

需要：

Windows：

native：

binding：

才：

新增：

最小：

Windows：

crate。

---

# 376. No Framework Bloat

M10：

不要：

引入：

大型：

Windows：

desktop：

framework。

Tauri：

已经：

承担：

窗口：

host。

SMTC：

和：

Tray：

只：

需要：

最小：

native：

integration。

---

# 377. No Duplicate Window Framework

不要：

因为：

tray：

重新：

引入：

第二：

window：

manager。

复用：

M6。

---

# 378. No Duplicate Command Framework

复用：

M6：

Command Registry。

---

# 379. No Duplicate Settings

复用：

M6：

settings：

system。

---

# 380. Final Diff Review

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

- 是否偷渡 M11 updater；
- 是否偷渡 installer；
- 是否偷渡 Steam；
- 是否新增 telemetry；
- 是否新增 cloud；
- 是否新增 account；
- 是否新增网络；
- 是否重新实现 audio engine；
- 是否创建第二套 playback controller；
- 是否创建第二套 queue；
- 是否创建第二套 library；
- 是否创建第二套 artwork cache；
- 是否 SMTC callback 操作 UI；
- 是否 SMTC callback 调用 decoder；
- 是否 SMTC callback 进入 audio callback；
- 是否 tray callback 直接调用 audio internals；
- 是否使用 `process::exit()` 绕过 shutdown；
- 是否重复注册 SMTC；
- 是否重复创建 tray；
- 是否没有 single-instance protection；
- 是否 second instance 能执行 arbitrary commands；
- 是否 timeline overflow；
- 是否 metadata 无上限；
- 是否 artwork 任意路径读取；
- 是否动态创建无限 timer；
- 是否无限 retry；
- 是否有 global shortcut conflict；
- 是否媒体键 double-trigger；
- 是否有 TODO；
- 是否 ignored tests；
- 是否弱化 assertions；
- 是否提交用户音乐；
- 是否提交本地 artwork；
- 是否提交 local screenshots；
- 是否提交 local DB；
- 是否提交 secrets；
- 是否提交 build artifacts。

---

# 381. Scope Audit

必须明确：

## M10 完成

```text
SMTC
Media Keys
System Metadata
Artwork
Timeline
Seek
Tray
Single Instance
Windows Lifecycle
```

## M10-B

单独：

```text
Global Custom Shortcuts
Advanced SMTC Controls
Taskbar Integration
Advanced Tray
Startup Policy
```

## M11+ 未实现

无：

```text Installer
Updater
Code Signing
Steam
Cloud
Accounts
Telemetry
Payment
```

---

# 382. Final Quality Gate

必须：

执行：

```powershell
npm run typecheck
npm run lint
npm test
npm run build
```

Rust：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

Windows：

```powershell
tauri build
```

如果：

项目：

当前：

实际：

command：

不同：

使用：

真实：

项目：

命令。

---

# 383. Full Regression

必须：

重新：

执行：

```text M1 PASS
M2 PASS
M3 PASS
M4 PASS
M5 PASS
M6 PASS
M7 PASS
M8 PASS
M9 PASS
```

任何：

regression：

```text M10 FAIL
```

---

# 384. Final Native Test

真实：

Windows：

执行：

```text
Launch
↓
Play local track
↓
Open Windows media controls
↓
Play/Pause
↓
Next
↓
Previous
↓
Seek
↓
Pause
↓
Resume
↓
Open Tray
↓
Play/Pause
↓
Next
↓
Open Now Playing
↓
Open Library
↓
Open Music Map
↓
Open Dreamscape
↓
Mini Player
↓
Exit
```

每一步：

必须：

是真实：

command：

而：

不是：

mock：

显示。

---

# 385. Final Background Test

如果：

B：

background：

mode：

实现：

真实：

执行：

```text
Enable tray background mode
↓
Play
↓
Close main window
↓
Playback continues
↓
SMTC works
↓
Tray works
↓
Reopen main
↓
Exit
```

---

# 386. Final Single Instance Test

执行：

```text
Launch A
↓
Play
↓
Launch B
↓
B exits
↓
A focuses
↓
Playback unchanged
↓
Tray remains singular
↓
SMTC remains singular
```

---

# 387. Final SMTC Metadata Test

覆盖：

```text Local FLAC
Local MP3
Direct HTTP finite stream
Internet Radio
ICY StreamTitle changes
Missing artwork
Changing artwork
Chinese metadata
Unicode metadata
Long metadata
```

---

# 388. Final Tray Test

覆盖：

```text
No track
Playing
Paused
Next unavailable
Next available
Previous unavailable
Previous available
Chinese
English
Window visible
Window minimized
Window hidden
```

---

# 389. Final Regression — Playback Safety

证明：

```text SMTC command
Tray command
UI command
Mini Player command
```

全部：

最终：

进入：

同一个：

Playback Authority。

---

# 390. Final Regression — Audio Callback

证明：

SMTC：

和：

Tray：

没有：

任何：

路径：

进入：

audio：

callback。

---

# 391. Final Regression — No Double Command

证明：

媒体：

键：

不会：

因为：

SMTC：

与：

global shortcut：

重复：

执行：

两次。

如果：

B：

global shortcut：

未：

实现：

则：

明确：

不存在：

该：

重复：

风险。

---

# 392. Final Regression — No Duplicate Session

证明：

整个：

应用：

只有：

一个：

SMTC：

integration：

owner。

---

# 393. Final Regression — No Duplicate Tray

证明：

只有：

一个：

tray：

icon：

owner。

---

# 394. Final Regression — No Duplicate Player

证明：

整个：

应用：

只有：

一个：

Playback Authority。

---

# 395. Final Offline Audit

完全：

断网：

验证：

```text Local playback
SMTC
Media keys
Tray
Library
Now Playing
Map
Dreamscape
Mini Player
```

都：

仍：

能够：

运行：

其中：

网络：

来源：

按照：

M4：

真实：

行为：

不可用：

属于：

正常：

结果。

---

# 396. Documentation Evidence

最终：

必须：

更新：

```text README.md
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

# 397. README Feature Matrix

根据：

实际：

仓库：

更新：

例如：

```text
Feature                     Status

Local Playback              ✅
Gapless                     ✅
Crossfade                   ✅
Loudness                    ✅
EQ                          ✅ / B
Music Library               ✅
HTTP Source                 ✅
Internet Radio              ✅
Persistent Cache            ✅
Now Playing                 ✅
Aurora                      ✅
Dreamscape                  ✅
Audio Analysis              ✅
Music Map                   ✅
Windows Media Controls     ✅
Media Keys                  ✅
System Tray                 ✅
Single Instance             ✅
Installer                   Future
Updater                    Future
Steam                      Future
```

实际：

状态：

必须：

以：

代码：

为：

准。

---

# 398. PERF.md

增加：

```text Windows Integration Initialization
SMTC Metadata Update
SMTC Timeline Update
Tray Creation
Tray Menu Update
Single Instance Overhead
Background Mode CPU
Long-running Memory
```

---

# 399. Known Limitations

必须：

如实：

列出：

例如：

```text SMTC unavailable on unsupported Windows versions
某些 Windows shell UI 行为由系统版本决定
tray icon behavior may vary by Windows shell state
```

实际：

限制：

必须：

基于：

真实：

测试。

---

# 400. No “Perfect Windows Support”

不要：

README：

声称：

> 完美：

支持：

所有：

Windows：

系统：

行为。

---

# 401. M10-A Completion Gate

必须：

同时：

满足：

1. Windows integration layer 正式建立。
2. SMTC 初始化正常。
3. SMTC failure graceful。
4. Play command 正常。
5. Pause command 正常。
6. Next command 正常。
7. Previous command 正常。
8. Stop command 正常。
9. Seek command 正常。
10. Non-seekable 正确。
11. Live 正确。
12. Position synchronization 正常。
13. Duration synchronization 正常。
14. Metadata synchronization 正常。
15. Artwork synchronization 正常。
16. ICY metadata synchronization 正常。
17. Media key integration 正常。
18. Tray icon 正常。
19. Tray menu 正常。
20. Tray commands 正常。
21. Tray state synchronization 正常。
22. Tray localization 正常。
23. Close policy 正常。
24. Exit policy 正常。
25. Single Instance 正常。
26. Second instance focus 正常。
27. No duplicate playback authority。
28. No duplicate tray。
29. No duplicate SMTC。
30. Windows API errors graceful。
31. No audio callback violation。
32. No high-frequency system API loop。
33. Security PASS。
34. Offline PASS。
35. Windows native tests PASS。
36. Frontend quality gate PASS。
37. Rust quality gate PASS。
38. Tauri build PASS。
39. M1 regression PASS。
40. M2 regression PASS。
41. M3 regression PASS。
42. M4 regression PASS。
43. M5 regression PASS。
44. M6 regression PASS。
45. M7 regression PASS。
46. M8 regression PASS。
47. M9 regression PASS。
48. Performance evidence PASS。
49. Soak PASS。
50. Documentation PASS。

---

# 402. M10-B

如果 B：

完成：

必须：

单独：

输出：

```text
Global Custom Shortcuts = PASS
Advanced SMTC Controls = PASS
Taskbar Integration = PASS
Advanced Tray = PASS
Startup Policy = PASS
```

未完成：

必须：

明确：

```text NOT COMPLETED
```

不能：

把：

B：

混入：

A：

完成：

判断。

---

# 403. Final M10 Audit

完成后：

严格：

输出：

# M10 Final Audit

## A. Baseline
## B. Windows Integration
## C. SMTC Lifecycle
## D. SMTC Metadata
## E. SMTC Artwork
## F. SMTC Playback State
## G. SMTC Commands
## H. SMTC Seek
## I. SMTC Timeline
## J. Media Keys
## K. ICY Metadata
## L. Tray
## M. Tray Commands
## N. Tray State
## O. Background Mode
## P. Single Instance
## Q. Second Instance
## R. Window Lifecycle
## S. Error Isolation
## T. Security
## U. Privacy
## V. Performance
## W. Memory
## X. Native Windows Tests
## Y. Offline
## Z. M1 Regression
## AA. M2 Regression
## AB. M3 Regression
## AC. M4 Regression
## AD. M5 Regression
## AE. M6 Regression
## AF. M7 Regression
## AG. M8 Regression
## AH. M9 Regression
## AI. Accessibility
## AJ. Documentation
## AK. Dependency Audit
## AL. Scope Compliance

每一项只能：

`PASS / FAIL / BLOCKED`

禁止：

- 基本完成；
- 大概可用；
- 理论支持；
- 应该没问题；
- 手工看起来正常；
- 以后再补。

---

# 404. 必须提供的关键证据

最终：

必须：

提供：

### SMTC

- initialization；
- metadata；
- artwork；
- Play；
- Pause；
- Next；
- Previous；
- Stop；
- Seek；
- position；
- duration；
- supported buttons；
- live/non-seekable。

### Media Keys

- Play/Pause；
- Next；
- Previous。

### Tray

- icon；
- menu；
- Play/Pause；
- Next；
- Previous；
- Open；
- Mini Player；
- Exit。

### Single Instance

- duplicate launch；
- focus existing；
- no duplicate player；
- no duplicate tray；
- no duplicate SMTC。

### Race Safety

- stale metadata；
- stale artwork；
- stale position；
- stale command；
- queue boundary。

### Lifecycle

- minimize；
- hide；
- restore；
- close；
- exit；
- restart。

### Performance

- startup；
- SMTC updates；
- Tray updates；
- RAM；
- CPU；
- soak。

### Security

- metadata bounds；
- path validation；
- timeline bounds；
- second-instance validation；
- no shell。

---

# 405. Visual / Native Evidence

至少准备：

```text
m10-smtc-now-playing.png
m10-smtc-controls.png
m10-smtc-artwork.png
m10-tray-menu.png
m10-tray-playing.png
m10-single-instance.png
```

必须：

来自：

> 当前真实 Windows build。

---

# 406. Final Diff Review

最终：

再次：

```powershell
git diff --stat
git diff
```

必须：

确认：

```text
M10 only
```

没有：

- M11 updater；
- installer；
- Steam；
- cloud；
- telemetry；
- accounts；
- payment；
- new codec；
- new source；
- new cache；
- new analysis；
- recommendation。

---

# 407. Git Commit

只有：

- M10-A 全部 PASS；
- 已声明的 M10-B 全部 PASS；
- M1 regression PASS；
- M2 regression PASS；
- M3 regression PASS；
- M4 regression PASS；
- M5 regression PASS；
- M6 regression PASS；
- M7 regression PASS；
- M8 regression PASS；
- M9 regression PASS；
- security PASS；
- privacy PASS；
- performance PASS；
- soak PASS；
- Windows native verification PASS；
- Tauri build PASS；
- dependency audit PASS；
- final audit PASS；
- diff review 干净；

才允许：

```powershell
git add .
git commit -m "M10: implement Windows integration"
```

**不要 push。**

---

# 408. Commit Hygiene

禁止提交：

- user music；
- user artwork；
- local database；
- local cache；
- local analysis DB；
- private metadata exports；
- diagnostics dumps；
- local SMTC captures；
- local tray captures；
- target；
- node_modules；
- dist；
- installers；
- private signing keys；
- secrets；
- `.env`。

如果截图：

属于：

正式：

项目：

文档资产：

且：

不包含：

用户：

私人：

内容：

才：

允许：

提交。

---

# 409. PROGRESS

最终：

更新：

```text
M10-A = PASS
M10-B = PASS / NOT COMPLETED
```

记录：

- SMTC；
- media keys；
- tray；
- single instance；
- background mode；
- native tests；
- performance；
- soak；
- security；
- screenshots；
- known limitations。

---

# 410. Final Output

完成后严格：

输出：

```text
# M10 Final Audit

A. Baseline
B. Windows Integration
C. SMTC
D. Media Keys
E. Metadata
F. Artwork
G. Timeline
H. Seek
I. Tray
J. Background Mode
K. Single Instance
L. Lifecycle
M. Security
N. Privacy
O. Performance
P. Native Windows Tests
Q. Offline
R. M1 Regression
S. M2 Regression
T. M3 Regression
U. M4 Regression
V. M5 Regression
W. M6 Regression
X. M7 Regression
Y. M8 Regression
Z. M9 Regression
AA. Documentation
AB. Dependencies
AC. Scope

M10-A = PASS / BLOCKED
M10-B = PASS / NOT COMPLETED
COMMIT = <hash>
NEXT = M11
```

---

# 411. 最终停止条件

如果：

```text
M10-A = PASS
```

并且：

已声明：

`M10-B`

全部：

通过：

立即：

> 停止。

不要开始：

- M11；
- Installer；
- Updater；
- Code Signing；
- Steam；
- Steam Overlay；
- Steam Cloud；
- Cloud Sync；
- Accounts；
- Payment；
- Telemetry。

下一阶段：

只能：

由我：

单独：

下达：

> **M11 提示词。**