# ifuyo Aura
# M6 下篇 — Testing / Security / Performance Audit / Release Gates (§172–318)

> 本文档为原 M6 开发提示词的**下篇（验证篇，§172–318）**，由《ifuyo Aura — M6 上篇 桌面 UI、Tauri Shell、Now Playing 与 Aurora 实现开发提示词.md》拆分而来。
> 上篇承载实现内容（§0–171）。章节编号沿用原 M6 全局编号；跨篇引用（如“见 #132”）按编号在两篇中查找。
> 阅读本文前，上篇所定义的实现必须已完成或在收尾。

# 172. Test Architecture

M6 必须至少有：

## Unit

- selectors；
- command registry；
- i18n；
- UI state；
- formatting。

## Component

- playback controls；
- track row；
- command palette；
- queue；
- library search。

## Integration

- IPC adapter；
- library query flow；
- play track flow；
- scan progress。

## E2E / Smoke

- application start；
- navigation；
- search；
- playback control；
- command palette；
- mini player。

---

# 173. Frontend Unit Test

继续：

```powershell
npm test
```

不得：

因为新增 UI：

删除前序测试。

---

# 174. IPC Test

建立 fake IPC adapter：

例如：

```text
MockPlaybackService
MockLibraryService
```

让：

React tests：

不需要真实 Tauri。

---

# 175. Real Tauri Smoke Test

至少验证：

```text
Tauri app launches
main window appears
navigation works
Ctrl+K works
mini player can open
```

如果环境支持：

再验证：

```text
local audio
library
playback
```

CI 不具备 GUI 条件：

必须明确：

> not run

不能写：

PASS。

---

# 176. NullOutput E2E

允许建立：

测试模式：

```text
Tauri
 ↓
Mock / test application layer
 ↓
NullOutput
```

这样：

可以：

> 在无真实声卡环境测试播放 UI。

---

# 177. Library Test Data

不要：

把用户真实音乐：

提交进仓库。

测试使用：

- synthetic fixtures；
- tiny legal fixtures；
- mock database。

---

# 178. Screenshot Regression

建议至少：

保存：

```text
Now Playing
Library
Command Palette
Mini Player
Map Shell
```

截图。

用于：

> 视觉回归。

---

# 179. Screenshot Stability

测试截图：

关闭：

- 时间动态；
- random seed；
- volatile metadata；
- live network content。

Aurora：

必须允许：

> deterministic test seed。

否则视觉回归没有意义。

---

# 180. Aurora Test Mode

提供：

```text
VISUAL_TEST_MODE
```

或等价机制。

固定：

- seed；
- time;
- telemetry。

这样：

同一截图：

> 可重复。

---

# 181. Accessibility Test

至少：

检查：

- buttons accessible name；
- dialog roles；
- keyboard focus；
- tab order；
- disabled semantics；
- image alt behavior；
- progress semantics。

---

# 182. Automated Accessibility

如果新增：

axe / testing helper：

必须做 dependency audit。

可以使用：

> 轻量 accessibility test tooling。

但：

不要因为测试工具：

引入巨大 framework。

---

# 183. Keyboard Test Matrix

至少：

```text
Ctrl+K
Esc
Space
Enter
Arrow Up
Arrow Down
Tab
Shift+Tab
```

验证：

不会：

> focus trap 在死路。

---

# 184. Error Injection

测试：

人为让：

- audio backend error；
- library DB error；
- source error；
- cache error；

UI：

必须：

> graceful。

---

# 185. IPC Timeout

如果 command：

异常长时间：

UI：

不得：

无限 spinning。

建立：

合理 timeout / cancellable model。

但：

不要：

对正常 library scan：

强制短 timeout。

---

# 186. Cancellation UI

例如：

扫描：

应该支持：

```text
Cancel
```

并：

显示：

> Cancelled

而不是：

> error。

---

# 187. Network Error UI

Direct HTTP：

失败：

要显示：

> source unavailable / timeout / unsupported

而不是：

> “Unknown Error”。

---

# 188. Cache Error UI

如果：

cache 满：

```text
CacheFullPinned
```

UI：

显示：

> 无法继续缓存，因为当前缓存项目均被固定。

如果：

当前内容可以：

直接 stream：

可以继续播放。

最终策略：

以 M5 为准。

---

# 189. Backend Error Mapping

建立：

```text
Rust Error Code
↓
Application Error
↓
Localized UI Message
```

不要：

让组件自己：

```text
switch(error.message)
```

---

# 190. Logging / Diagnostics

M6：

允许：

本地 debug panel。

例如：

```text
Ctrl+Shift+D
```

[B]

但：

默认关闭。

不得：

成为普通用户 UI。

---

# 191. Security Audit

检查：

- Tauri capabilities；
- CSP；
- filesystem permission；
- asset protocol；
- external links；
- command inputs；
- IPC validation；
- path handling；
- URL handling；
- shell permission；
- environment variable exposure；
- devtools production configuration。

---

# 192. Path Security

Library / artwork：

任何：

用户路径：

必须：

> 经过 Rust side validation。

不要：

相信 React 传来的：

```text
C:\Music\..\Secrets
```

---

# 193. URL Security

如果 M6 提供：

`Open URL`

输入：

必须：

继续走：

M4 Source Resolver。

不得：

UI：

自己：

`fetch(url)`。

---

# 194. Source UI Boundary

即便用户从：

Command Palette：

输入 URL：

流程必须：

```text
Command
 ↓
Application
 ↓
M4 Resolver
 ↓
M5 Cache if applicable
 ↓
Playback
```

不是：

```text
React
 ↓
fetch()
```

---

# 195. No Browser API Audio

禁止：

使用：

```text
HTMLAudioElement
Web Audio API
```

作为：

> 正式播放引擎。

UI：

必须：

连接：

Rust audio engine。

Three.js：

只负责：

视觉。

---

# 196. No Duplicate Player

整个应用：

必须：

只有：

> 一个权威 Playback Controller。

主窗口：

mini player：

command palette：

都调用：

同一 controller。

---

# 197. Queue Authority

同样：

只能存在：

> 一个权威 queue state。

多个 window：

都读取同一状态。

---

# 198. Library Authority

只能有：

> 一个 Library service。

React：

不缓存一整套第二数据库。

允许：

UI query cache，

但：

source of truth：

仍然是：

M3。

---

# 199. Cache Authority

M6：

不创建：

> localStorage cache of audio bytes。

不要：

浏览器缓存：

> 替代 M5。

---

# 200. LocalStorage Policy

UI：

可以使用：

很小的纯 UI preferences，

但：

不要保存：

- music metadata mirror；
- queue full data；
- audio cache；
- credentials；
- source secrets。

---

# 201. Credential Policy

M6：

禁止：

在：

- localStorage；
- React state；
- query string；
- logs；

保存：

- passwords；
- authorization；
- API keys。

M4/M5 已定义的：

credential policy：

继续生效。

---

# 202. Performance Profiling

至少使用：

浏览器/React profiling：

检查：

- Library scrolling；
- Search typing；
- Queue open；
- Now Playing；
- Aurora；
- resize。

输出：

`docs/PERF.md`

---

# 203. PERF.md

增加：

```text
M6 UI Startup
Library 10k rows
Search latency
Artwork scrolling
Command Palette open
Aurora FPS
Mini Player sync
Window resize
Memory baseline
```

必须注明：

- hardware；
- OS；
- build type；
- measurement method。

---

# 204. No Fake Performance

不能：

通过：

- 禁用测试；
- 降低截图质量；
- 不渲染 artwork；
- 关闭 Aurora；
- 假数据；

制造：

> “性能非常好”。

必须记录真实状态。

---

# 205. Memory Audit

检查：

长期运行：

- route switch；
- open/close queue；
- open/close command palette；
- mini player toggle；
- library search；

是否产生：

> listener leak。

尤其：

Tauri event subscriptions：

必须：

> unsubscribe。

---

# 206. Window Listener Cleanup

任何：

```text
resize
focus
blur
keyboard
Tauri events
WebGL listeners
```

必须：

在 unmount / dispose：

> 正确清理。

---

# 207. Aurora Dispose

切换 app lifecycle：

必须：

释放：

- renderer；
- geometry；
- material；
- texture；
- animation loop；
- listeners。

---

# 208. Event Subscription Hygiene

禁止：

每次 React render：

注册一次：

Tauri listener。

必须：

> 一次注册 / 一次清理。

---

# 209. React Strict Mode

如果：

开发环境启用：

React Strict Mode。

Aurora / event subscriptions：

必须：

能够正确应对：

> mount → unmount → mount。

不能：

> 双重 renderer / 双重 listener。

---

# 210. Production Build

必须实际验证：

```powershell
npm run build
```

以及：

```powershell
tauri build
```

具体 script：

根据现有 workspace。

---

# 211. Tauri Build

如果：

Windows 环境：

能够构建：

必须：

> 实际 build。

如果签名尚未配置：

不要强行伪造 signed installer。

M6：

只验证：

> unsigned build artifact。

---

# 212. Build Artifact Hygiene

禁止提交：

- target；
- dist；
- installer；
- local DB；
- screenshots；
- logs；

除非项目明确要求。

---

# 213. CI

继续：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check

npm run typecheck
npm run lint
npm test
npm run build
```

如果已有：

> Tauri build CI

也继续执行。

---

# 214. M1 Regression

确认：

- decoder；
- resampler；
- ring buffer；
- play；
- pause；
- stop；
- seek；
- generation；
- volume；
- zero allocation。

---

# 215. M2 Regression

确认：

- gapless；
- crossfade；
- loudness；
- limiter；
- hotplug；
- EQ。

---

# 216. M3 Regression

确认：

- scan；
- metadata；
- FTS5；
- CJK search；
- artwork；
- watcher；
- rename / move；
- identity；
- pagination；
- stats。

---

# 217. M4 Regression

确认：

- local source；
- direct HTTP；
- HTTPS；
- Internet Radio；
- ICY；
- Radio Browser；
- security；
- source errors；
- cache policy。

---

# 218. M5 Regression

确认：

- Allowed；
- SessionOnly；
- Forbidden；
- cache persistence；
- Range;
- revalidation；
- LRU；
- pin；
- recovery；
- offline cache。

---

# 219. UI Integration Regression

必须验证：

```text
Library
 ↓
Track
 ↓
Play
 ↓
Now Playing
 ↓
M1/M2
```

以及：

```text
M4 Source
 ↓
M5 Cache
 ↓
Playback
 ↓
Now Playing
```

以及：

```text
M3 Watcher
 ↓
UI library_changed
 ↓
Library refresh
```

---

# 220. Main User Flow Test

必须完整执行：

```text
Launch
↓
Library
↓
Add folder
↓
Scan
↓
Search
↓
Double click track
↓
Now Playing
↓
Pause
↓
Resume
↓
Seek
↓
Volume
↓
Queue
↓
Next
↓
Ctrl+K
↓
Mini Player
↓
Close Mini
↓
Exit
```

每一步：

必须真实可运行。

---

# 221. Second User Flow

网络：

如果本地测试 server 已提供：

```text
Open URL
↓
M4 resolve
↓
M5 cache policy
↓
playback
↓
Now Playing
```

CI：

只使用：

> local fake HTTP server。

禁止公网。

---

# 222. Internet Radio Flow

如果测试：

使用：

> local ICY fixture server。

验证：

```text
station
↓
ICY
↓
StreamTitle
↓
Now Playing metadata
```

不访问真实电台。

---

# 223. Unsupported Platform Flow

Command Palette：

如果用户输入：

```text
YouTube / Spotify / 网易云 / QQ Music / Apple Music
```

最终：

必须：

> 由 M4 拒绝。

UI：

不得绕过。

---

# 224. Library Search Flow

测试：

```text
中文
英文
单字
artist
album
prefix
special chars
```

确认：

> M3 真搜索结果能够正确到达 UI。

---

# 225. Scan Progress Flow

测试：

假装：

100 文件 fixture。

UI：

必须：

显示：

```text
0 → 100
```

或者真实 increment。

如果没有 total：

不要 fake percentage。

---

# 226. Error Flow

注入：

```text
database unavailable
source timeout
audio device error
cache full
unsupported codec
```

UI：

必须：

> 每种错误都有明确用户反馈。

---

# 227. No Crash Boundary

任何：

- malformed IPC;
- backend error;
- missing artwork;
- unavailable device;
- source error;
- empty library；

不得：

> React white screen。

---

# 228. White Screen Recovery

开发环境：

提供：

可诊断错误。

生产：

UI：

至少保持：

> Main Shell

即使：

某个 feature 挂掉。

---

# 229. Feature Isolation

推荐：

页面级错误边界：

```text
App
 ├─ NowPlayingBoundary
 ├─ LibraryBoundary
 └─ MapBoundary
```

如果 Aurora 挂：

> Now Playing controls 仍可用。

---

# 230. Aurora Failure

如果：

WebGL：

失败：

Fallback：

```text
CSS atmospheric background
```

并：

继续：

- play；
- pause；
- seek；
- queue。

---

# 231. Library Failure

如果：

Library：

暂时失败：

Now Playing：

仍应：

> 能保持最后状态。

不要：

整个 app：

> “数据库坏了，所以播放器也打不开”。

---

# 232. Cache Failure

Cache：

坏：

如果 source：

仍可直连：

允许：

> degrade gracefully。

最终策略：

以 M5 为准。

---

# 233. Source Failure

Source：

失败：

Now Playing：

显示：

- source error；
- retry；
- return to library。

---

# 234. Accessibility + Visual

Aurora：

必须：

> 不影响文字可读性。

如果 background：

变亮：

foreground：

必须：

保持：

> 合理 contrast。

---

# 235. Aurora Occlusion

不要：

让 Aurora：

盖住：

- controls；
- queue；
- text；
- buttons。

Aurora：

应成为：

> 背景层。

---

# 236. GPU Safety

高 DPI / 多窗口：

必须：

避免：

同时创建：

> 两套高分辨率 Aurora renderer。

Mini Player：

默认：

> 不使用第二套 Aurora。

---

# 237. Mini Player Visual

Mini：

可以：

使用静态封面 + subtle gradient。

不要：

复制：

> 全 Aurora WebGL。

---

# 238. Map Shell Performance

M6 Map：

必须：

> 不初始化真正 Map engine。

否则：

会提前：

- graph allocate；
- WebGL scene；
- large data。

M9：

再处理。

---

# 239. Documentation

更新：

`ARCHITECTURE.md`

加入：

```text
M6 UI Architecture
Tauri Shell
Window Model
IPC Model
State Model
Command Model
Now Playing
Library UI
Queue
Mini Player
Aurora
Performance
Accessibility
```

---

# 240. DECISIONS.md

至少记录：

- UI structure；
- Zustand split；
- IPC DTO strategy；
- event strategy；
- event frequency；
- window architecture；
- Mica implementation；
- Win10 fallback；
- title bar;
- main window size;
- mini player size;
- UI scale；
- i18n;
- virtualization;
- command registry;
- Aurora architecture;
- Aurora telemetry；
- Aurora seed；
- render throttling；
- reduced motion；
- WebGL fallback；
- local asset policy。

---

# 241. SECURITY.md

加入：

```text
Tauri Capabilities
CSP
Asset Loading
IPC Validation
External Links
Window Permissions
Filesystem Access
Production DevTools Policy
```

---

# 242. PRIVACY.md

确认：

M6：

不新增：

- telemetry；
- analytics；
- cloud data；
- remote logs。

用户主动：

- direct URL；
- Radio Browser；

仍遵守 M4。

---

# 243. PROGRESS.md

开始：

```text
M6-A = IN PROGRESS
M6-B = NOT STARTED
```

完成后：

真实记录：

- shell；
- Now Playing；
- Library；
- Aurora；
- command palette；
- mini player；
- tests；
- accessibility；
- performance；
- known limitations。

---

# 244. Dependency Audit

任何新增 UI / WebGL dependency：

必须检查：

```text
license
MSRV
bundle size
security
maintenance
Windows compatibility
CI
```

重点：

- Tauri plugins；
- UI virtualization；
- i18n；
- accessibility tooling；
- Three.js helpers。

---

# 245. No Duplicate Frameworks

不要同时引入：

- 两套 i18n；
- 两套 modal；
- 两套 state management；
- 两套 tooltip；
- 两套 virtualization；
- 两套 icon library；

除非真实必要。

---

# 246. No UI Framework Bloat

不要：

为了快速做组件：

引入大型 UI framework

然后：

> 让所有视觉风格变成模板网站。

优先：

> 小型自有 design system。

---

# 247. CSS Architecture

推荐：

```text
tokens
globals
layout
components
features
windows
visual
```

禁止：

> 全项目一个 5000 行 `app.css`。

---

# 248. CSS Containment

对：

Aurora：

Library list：

大型区域：

合理使用：

```css
contain
content-visibility
```

等性能手段：

只有实际测量证明有价值时采用。

---

# 249. Animation Ownership

动画：

必须明确：

> CSS animation / WebGL animation / JS animation

由谁负责。

同一元素：

禁止：

> 三种 animation source 互相抢控制权。

---

# 250. State Selector Discipline

Zustand：

组件：

尽量：

```text
useStore(selector)
```

而不是：

```text
useStore()
```

把：

整个 store：

订阅进每一个组件。

---

# 251. Memoization Policy

不要：

机械：

到处加 `useMemo` / `useCallback`。

根据：

> profiler evidence

决定。

---

# 252. React Key Stability

所有列表：

使用稳定 domain ID：

例如：

```text TrackId
QueueItemId
PlaylistId
```

不能：

长期依赖 array index。

---

# 253. Date / Number Formatting

统一：

locale-aware formatter。

例如：

播放时长：

必须：

```text
mm:ss
hh:mm:ss
```

根据真实长度切换。

---

# 254. Duration Formatting

未知 duration：

必须：

明确：

```text Live
Unknown
```

不能：

`NaN:NaN`。

---

# 255. Number Safety

所有来自 Rust：

可能为：

- null；
- zero；
- negative；
- out-of-range；

UI：

必须：

防御。

特别：

volume；

progress；

duration。

---

# 256. Progress Safety

播放位置：

必须：

clamp：

```text 0 ≤ position ≤ duration
```

如果：

backend：

短暂超出：

UI：

不要：

> progress bar 溢出。

---

# 257. Command Safety

Command：

执行前：

检查：

```text enabled
```

disabled：

不得：

执行。

---

# 258. Shortcut Conflict

检查：

快捷键与：

- text input；
- browser shortcuts；
- OS shortcuts；

冲突。

Command Palette：

必须：

在文本输入状态：

适当让出：

Space / Ctrl+F。

---

# 259. Search Focus

Library：

必须：

有：

> 全局 focus search command。

例如：

```text Ctrl+F
```

执行：

自动：

focus。

---

# 260. Main Navigation Keyboard

可以：

支持：

```text Alt+1
Alt+2
Alt+3
```

或者：

其他明确方案。

不是必须。

如果实现：

记录。

---

# 261. Title Bar Accessibility

自定义 title bar：

按钮：

必须：

有：

- label；
- keyboard activation；
- tooltip。

不能：

> 视觉上只有几个 icon，没有任何辅助信息。

---

# 262. Drag Region

custom title bar：

drag region：

不能：

覆盖：

- close；
- maximize；
- minimize；
- interactive buttons。

否则：

按钮：

> 点不动。

必须测试。

---

# 263. Window Maximize

双击：

title bar：

必须：

符合 Windows 预期：

> maximize / restore。

如果实现：

必须：

尊重：

> app window bounds。

---

# 264. Multi-Monitor

至少考虑：

- monitor disconnected；
- DPI changed；
- window restored outside screen。

Mini Player：

同样。

---

# 265. Window State Corruption

如果保存的：

window config：

损坏：

必须：

> 使用默认值恢复。

不能：

> app launch crash。

---

# 266. Settings Corruption

同理：

settings JSON：

损坏：

必须：

- fallback defaults；
- record diagnostics；
- continue launch。

---

# 267. First Launch

第一次启动：

必须：

> 看起来是完整产品。

不要：

显示：

- internal debugging；
- blank window；
- placeholder TODO；

用户 UI：

必须：

只看到：

`ifuyo Aura`

---

# 268. Brand Audit

全 UI 搜索：

```text
Aura Player
Aura Music
IFY Music
IFUYO Music
placeholder vendor
old logo names
```

应该：

全部：

> PASS / 清理。

工程：

允许：

`aura-*`。

---

# 269. Product Copy

UI 文案：

应该：

体现：

> 克制、优雅、安静、有一点诗意。

但：

禁止：

过度营销。

例如：

不要大量：

> AI powered / revolutionary / next generation。

---

# 270. No AI Claims

本阶段：

不要在 UI：

加入：

- AI music analysis；
- AI DJ；
- AI recommendation；

除非真实功能已存在。

---

# 271. No False Privacy Claims

UI 可以写：

> Local-first

但必须：

与真实行为一致。

不要：

声称：

> “绝不联网”

因为 M4 支持明确的网络来源。

---

# 272. First Launch Privacy

如果第一次：

需要：

用户选音乐目录。

不要：

自动：

- 扫描 Documents；
- 扫描 Desktop；
- 扫描整个磁盘。

---

# 273. Library Root Permission

用户选择：

目录后：

UI：

显示：

> 已加入音乐库。

不要：

擅自扩大 root。

---

# 274. File Picker

文件选择：

支持：

```text
Audio files
Folders
```

具体 filter：

与 M3 实际支持格式同步。

不要：

UI 宣称：

支持一个 M3 实际不支持的 codec。

---

# 275. Open File

支持：

从 command palette：

打开单首本地音乐。

流程：

```text
file picker
 ↓
M4 Local Source
 ↓
M1/M2
```

不要：

绕过 Source system。

---

# 276. Open URL

允许：

Command Palette：

输入：

合法：

`http://`
`https://`

最终：

进入：

M4。

不在 frontend：

直接 fetch。

---

# 277. Unsupported Input

输入：

```text
ftp://
file://
data:
javascript:
```

必须：

> M4 / application rejection。

UI：

显示清楚。

---

# 278. Source Security Consistency

M6：

不得因为：

> “UI 已经有 URL 输入”

而降低：

M4 的：

- SSRF；
- redirect；
- credential URL；
- unsupported platform；

保护。

---

# 279. Cache Policy Consistency

UI：

不能：

提供一个：

> “Cache everything”

按钮。

M5 policy：

依然：

> 权威。

---

# 280. SessionOnly Consistency

如果 source：

`SessionOnly`

UI：

不能：

显示：

> Downloaded forever / Offline Ready

除非：

M5 真正报告。

---

# 281. Forbidden Consistency

Internet Radio：

不能：

显示：

> Cached permanently

绝不允许。

---

# 282. Source Attribution

如果 M4：

提供：

`Attribution`

UI：

可以：

在 source details：

显示。

但：

不得：

擅自添加错误版权声明。

---

# 283. Library Artwork and Source Artwork

M6：

默认：

只显示：

M3 本地 artwork。

网络 source：

如果：

M4 没有合法 online artwork：

不要：

自动从第三方抓。

---

# 284. Library Stats

可以显示：

```text
X tracks
Y albums
Z artists
```

这些：

来自：

M3。

不要：

为了视觉效果：

伪造数字。

---

# 285. Map Shell Stats

Map：

可以：

显示：

```text Tracks
Artists
Albums
```

如果：

真实 library API 有这些信息。

没有：

就不显示。

---

# 286. Empty Map

如果 library 为空：

Map：

应该：

提示：

> 先添加音乐。

但：

不是：

假 map。

---

# 287. Queue Empty

Queue empty：

应该：

显示：

> 当前队列为空。

可以：

提供：

`Browse Library`

action。

---

# 288. Now Playing Empty

没有正在播放：

Now Playing：

应该：

成为一个：

> 静默、漂亮的空状态。

例如：

- 品牌视觉；
- Aurora slow idle；
- Browse Library。

不要：

大大的：

`NO MUSIC`.

---

# 289. No Forced Engagement

禁止：

- startup tips popup spam；
- notification reminders；
- “continue listening” nagging；
- fake daily task；
- streak。

---

# 290. Shutdown

关闭主窗口：

应该：

正确：

- 停止 UI listeners；
- 关闭 mini；
- 停止 Aurora；
- flush settings；
- 交由 application layer 处理 audio lifecycle。

Audio shutdown：

不能：

由 React 直接暴力 kill。

---

# 291. Crash Safety

如果 UI：

崩溃：

不应该：

corrupt：

- library DB；
- cache DB；
- playback persistence。

---

# 292. Application Lifecycle

明确：

```text
starting
ready
closing
closed
```

避免：

> 某个异步任务：

应用已经关闭：

还在疯狂 setState。

---

# 293. Async Task Cleanup

所有：

- commands；
- subscriptions；
- fetch-like app calls；
- scan listeners；

必须：

支持：

cancellation / cleanup。

---

# 294. No Zombie Task

退出后：

不允许：

残留：

- interval；
- requestAnimationFrame；
- Tauri event listener；
- WebGL animation loop；
- timeout chain。

---

# 295. Testing Zombie Prevention

开发测试：

至少：

```text
open / close command palette × many times
open / close queue × many times
open / close mini player × many times
navigate views × many times
mount/unmount Aurora × many times
```

观察：

> memory / listener count。

---

# 296. Extended Soak

至少进行：

> 数小时级本地桌面使用 soak。

动作循环：

```text
play
pause
seek
next
search
open queue
toggle mini
switch library
return now playing
```

如果完整长测不适合 CI：

建立：

> extended local soak script。

不得把未运行写成 PASS。

---

# 297. Final Visual Audit

完成后：

实际运行：

至少审查：

```text
Now Playing
Library
Map
Command Palette
Mini Player
EQ
Queue
Error State
Empty State
```

检查：

- alignment；
- spacing；
- clipping；
- overflow；
- text truncation；
- focus；
- hover；
- dark background；
- Mica；
- fallback。

---

# 298. Final Performance Audit

实际记录：

```text
Startup
Library scroll
Search
Now Playing render
Aurora FPS
Mini Player sync
Window resize
Memory
```

---

# 299. Final Accessibility Audit

实际验证：

- keyboard-only；
- focus；
- contrast；
- dialogs；
- labels；
- reduced motion；
- 125% / 150% DPI。

---

# 300. Final Security Audit

重新检查：

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

- Tauri permissions；
- shell permissions；
- CSP；
- remote resources；
- local paths；
- asset URLs；
- IPC payloads。

---

# 301. Diff Review

完成后：

先不要 commit。

执行：

```powershell
git diff --stat
git diff
```

重点检查：

- 是否偷渡 M7 Dreamscape；
- 是否偷渡 M8 Analysis；
- 是否偷渡 M9 Music Map engine；
- 是否偷渡 M10 SMTC / tray；
- 是否加入真实网络请求；
- 是否加入 telemetry；
- 是否加入 fake data；
- 是否加入第二套 audio player；
- 是否加入第二套 queue；
- 是否加入第二套 library store；
- 是否在 React 中直接操作 SQLite；
- 是否在 React 中直接 fetch source；
- 是否使用 HTMLAudioElement 作为正式播放；
- 是否有 remote asset；
- 是否有 remote font；
- 是否有裸色；
- 是否有 `any` 扩散；
- 是否有 listener leak；
- 是否有 WebGL resource leak；
- 是否有 giant component；
- 是否有 giant global store；
- 是否有 TODO；
- 是否有 debug logs；
- 是否弱化测试；
- 是否添加无必要依赖；
- 是否提交真实音乐；
- 是否提交本地 DB；
- 是否提交用户 artwork；
- 是否提交构建产物。

---

# 302. Frontend Quality Gate

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

# 303. Rust Quality Gate

必须：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

---

# 304. Tauri Build Gate

必须：

在当前支持环境执行：

```powershell
tauri build
```

或项目现有等价命令。

如果：

> 环境无法执行 GUI build

明确：

`BLOCKED`

不得写 PASS。

---

# 305. Full Regression

最终再次：

### M1

PASS

### M2

PASS

### M3

PASS

### M4

PASS

### M5

PASS

任何前序 regression：

> M6 FAIL。

---

# 306. Final M6 Audit

输出：

# M6 Final Audit

## A. Baseline

## B. Tauri Shell

## C. Window Architecture

## D. Mica / Fallback

## E. IPC

## F. State Architecture

## G. Now Playing

## H. Queue

## I. Library UI

## J. Search

## K. Scan / Folder Management

## L. Map Shell

## M. Command Palette

## N. Mini Player

## O. Settings

## P. i18n

## Q. Accessibility

## R. UI Scale

## S. Motion

## T. Aurora

## U. Aurora Performance

## V. Error Handling

## W. Security

## X. Tests

## Y. Performance

## Z. M1 Regression

## AA. M2 Regression

## AB. M3 Regression

## AC. M4 Regression

## AD. M5 Regression

## AE. Documentation

## AF. Scope Compliance

每项只能：

`PASS / FAIL / BLOCKED`

禁止使用：

- 基本完成；
- 大概可用；
- 理论支持；
- 应该没有问题；
- 差不多。

---

# 307. 必须提供的关键证据

最终必须明确提供：

## Desktop

- main window；
- custom title bar；
- Mica；
- Win10 fallback；
- mini player。

## Playback

- real play/pause；
- seek；
- volume；
- next/previous；
- crossfade；
- loudness；
- EQ。

## Library

- real scan；
- real search；
- Chinese search；
- artwork；
- pagination；
- virtualization。

## Queue

- current；
- upcoming；
- remove；
- next。

## Command Palette

- Ctrl+K；
- keyboard operation；
- real commands。

## Aurora

- WebGL initialization；
- deterministic seed；
- playback response；
- idle throttle；
- reduced motion；
- fallback；
- measured FPS。

## Accessibility

- keyboard;
- focus;
- labels;
- contrast;
- scale。

## Security

- capabilities；
- CSP；
- no shell；
- no remote UI assets；
- no secrets。

---

# 308. 必须提供的真实界面截图

至少准备：

```text
01-now-playing.png
02-library.png
03-command-palette.png
04-mini-player.png
05-map-shell.png
```

另外建议：

```text
06-eq.png
07-queue.png
08-empty-library.png
09-error-state.png
```

截图：

必须来自：

> 当前真实构建。

不得使用 mock marketing render。

---

# 309. Documentation Evidence

最终更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
SECURITY.md
PRIVACY.md
CHANGELOG.md
```

README：

加入：

- actual feature list；
- screenshots；
- current limitations；
- build instructions；
- development instructions。

不要宣称：

> 尚未实现的 M7/M8/M9/M10 功能已经存在。

---

# 310. README Feature Matrix

建议加入：

```text
Feature                     Status

Local playback              ✅
Gapless                     ✅
Crossfade                   ✅
Loudness                    ✅
EQ                          ✅ / B
Library                     ✅
HTTP source                 ✅
Internet Radio              ✅
Persistent cache            ✅
Now Playing                 ✅
Aurora                      ✅
Music Map                   M9
Dreamscape                  M7
Analysis                    M8
SMTC                        M10
```

实际状态：

必须：

> 根据真实仓库更新。

---

# 311. PROGRESS.md

最终记录：

```text
M6-A = PASS
M6-B = PASS / NOT COMPLETED
```

并记录：

- implemented；
- tests；
- screenshots；
- performance；
- accessibility；
- security；
- known limitations。

---

# 312. M6-A 完成门槛

必须同时满足：

1. Desktop shell 可运行。
2. Now Playing 可用。
3. Library 可用。
4. Command Palette 可用。
5. Mini Player 可用。
6. Aurora 可运行。
7. IPC 真实连接 M1–M5。
8. accessibility baseline PASS。
9. security audit PASS。
10. frontend quality gate PASS。
11. Rust quality gate PASS。
12. full regression PASS。
13. screenshots 完成。
14. documentation 完成。

---

# 313. M6-B

如果 B 级实现：

必须：

> 单独列出完成项。

例如：

```text
Queue Reorder = PASS
Drag Drop = PASS
Advanced EQ UI = PASS
Visual Presets = PASS
```

未完成：

明确：

`NOT COMPLETED`

---

# 314. Scope Compliance

最终必须确认：

### M7 没有被偷渡

无：

- Dreamscape；
- seed world；
- mutation；
- environment simulation。

### M8 没有被偷渡

无：

- BPM；
- Key；
- LUFS analysis；
- MFCC；
- embedding。

### M9 没有被偷渡

无：

- music map engine；
- graph generation；
- spatial layout。

### M10 没有被偷渡

无：

- SMTC；
- tray；
- Windows media integration。

---

# 315. Git Commit

只有：

- M6-A 全部 PASS；
- M6-B 若声明完成则 PASS；
- M1 regression PASS；
- M2 regression PASS；
- M3 regression PASS；
- M4 regression PASS；
- M5 regression PASS；
- frontend gate PASS；
- Rust gate PASS；
- security PASS；
- accessibility PASS；
- performance audit PASS；
- final audit PASS；
- diff review 干净；

才允许：

```powershell
git add .
git commit -m "M6: implement desktop UI foundation"
```

**不要 push。**

---

# 316. Commit Hygiene

commit：

不得包含：

- local DB；
- target；
- node_modules；
- dist；
- cache；
- user music；
- credentials；
- generated screenshots；
- temporary diagnostics；

除非：

该文件：

> 明确属于项目源码资产。

---

# 317. Final Output

完成后严格输出：

```text
# M6 Final Audit

A. Baseline
B. Tauri Shell
C. Window
D. Mica
E. IPC
F. State
G. Now Playing
H. Queue
I. Library
J. Search
K. Command Palette
L. Mini Player
M. i18n
N. Accessibility
O. Aurora
P. Performance
Q. Security
R. Tests
S. M1 Regression
T. M2 Regression
U. M3 Regression
V. M4 Regression
W. M5 Regression
X. Documentation
Y. Scope

M6-A = PASS / BLOCKED
M6-B = PASS / NOT COMPLETED
COMMIT = <hash>
NEXT = M7
```

---

# 318. 最终停止条件

如果：

```text
M6-A = PASS
```

并且：

`M6-B` 已声明的项目全部通过：

立即：

> 停止。

不要开始：

- M7；
- Dreamscape；
- M8；
- Analysis；
- M9；
- Music Map。

下一阶段只能由我单独下达：

> **M7 提示词。**
