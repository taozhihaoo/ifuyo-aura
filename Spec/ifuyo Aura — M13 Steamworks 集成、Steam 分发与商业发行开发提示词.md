# ifuyo Aura — M13 Steamworks 集成、Steam 分发与商业发行开发提示词

你现在进入 **ifuyo Aura M13**。

本阶段不是继续堆普通播放器功能，而是把已经经过 M0–M12 建立起来的本地优先桌面音乐播放器，正式接入 **Steam / Steamworks / SteamPipe** 的发行体系，并建立一套可长期维护的 Steam 平台适配层。

---

# 0. 最高优先级：先读章程，不要立即编码

在任何修改前，必须完整检查当前仓库以及已有文档。

至少阅读：

```text
README.md
docs/ARCHITECTURE.md
docs/DECISIONS.md
docs/PROGRESS.md
docs/SECURITY.md
docs/PRIVACY.md
docs/SOURCES.md
docs/SOURCE_POLICY.md
docs/CONTENT_POLICY.md
docs/THIRD_PARTY_LICENSES.md
docs/CODE_SIGNING_POLICY.md
docs/RELEASE_CHECKLIST.md
docs/CHANGELOG.md
```

同时检查：

```text
package.json
package-lock.json / npm lockfile
src/
src-tauri/
Cargo.toml
Cargo.lock
tauri.conf.*
capabilities/
plugins/
.github/workflows/
```

检查 M10、M11、M12 的实际代码，而不是只看文档。

重点确认：

- M10 的 Windows / SMTC / Tray / Single Instance 生命周期；
- M11 的 installer / updater / signing / release pipeline；
- M12 的 security / privacy / licensing / source policy；
- M3 的 Library / Playlist / playback statistics；
- M5 的 Cache；
- M7 的 Dreamscape；
- M8 的 Analysis；
- M9 的 Music Map；
- M6 的 UI / Settings / Command Palette / Mini Player；
- 当前版本号；
- 当前 Application Identifier；
- 当前 Windows 安装包结构；
- 当前 CI；
- 当前第三方依赖和许可证清单。

如果已有实现与本提示词冲突，以**当前真实代码 + 已冻结章程 + 本阶段明确边界**共同决定。

不要为了“看起来完整”重构整个项目。

---

# 1. M13 的定位

M13 =

> **Steam Integration & Distribution**

本阶段解决的问题：

> ifuyo Aura 能否作为一个真正的商业桌面软件，被稳定地安装、启动、运行、更新，并通过 Steam 分发，同时又不破坏原有独立 Windows 发行版本？

M13 不应该把 ifuyo Aura 变成“必须登录 Steam 才能运行”的软件。

核心原则：

```text
Steam 是发行与平台能力层
不是播放器核心
不是数据拥有者
不是音乐来源
不是唯一启动方式
不是唯一更新方式
```

因此：

```text
aura-core
aura-library
aura-analysis
aura-synth
aura-map
cache
playback
SMTC
Tray
```

都不能反向依赖 Steam。

正确方向：

```text
                    ┌──────────────────┐
                    │   ifuyo Aura    │
                    │  Application Core │
                    └────────┬─────────┘
                             │
                 ┌───────────┴───────────┐
                 │                       │
        ┌────────▼────────┐     ┌────────▼────────┐
        │ Windows Adapter │     │ Steam Adapter   │
        │ SMTC / Tray     │     │ Steamworks      │
        └─────────────────┘     └─────────────────┘
```

Steam 只是 Adapter。

---

# 2. M13 的成功标准

必须达到：

## M13-A

真实 Steam 集成基础设施完成，并且：

- Steamworks Adapter 存在；
- Steam 生命周期正确；
- App ID 配置隔离；
- Steam API 初始化 / shutdown 生命周期正确；
- 非 Steam 环境可以正常运行；
- Steam 不可用时 ifuyo Aura 不崩溃；
- Steam API 不进入 audio callback；
- Steam API 不成为 Playback Authority；
- Steam 用户身份不成为本地播放器运行前提；
- Steam 状态不会污染普通 Windows Build；
- achievements / stats / cloud 等平台能力通过独立 adapter；
- Steam SDK / native binary 的来源、许可证、分发边界清楚；
- CI 可以在没有真实 Steam 账号和私密凭据的情况下完成绝大部分测试；
- Steam App ID / branch / depot 配置具备环境隔离；
- SteamPipe 构建结构完成；
- Steam 上传脚本完成；
- Steam-specific runtime files 正确进入发行产物；
- 不应公开的 Steamworks SDK 内容不进入 public repo；
- Steam Build 可以被验证；
- 现有独立 Windows 安装包继续正常工作。

## M13-B

在 A 基础上完成：

- Steam Achievements；
- Steam Stats；
- Steam Cloud；
- Rich Presence；
- Overlay / Steam UI integration；
- Steam Branch / Beta；
- Steam Deck / Proton 兼容性验证；
- Store Page 所需工程素材与发布元数据；
- SteamPipe 自动化；
- Steam Release candidate；
- Steam-specific diagnostics；
- Steam feature settings；
- 商业发布前最终检查。

---

# 3. 严格禁止的事情

M13 不得做以下事情：

```text
Steam 登录墙
Steam 账号绑定本地用户才能播放
Steam 登录失败导致播放器无法启动
上传音乐文件到 Steam Cloud
上传完整 Music Library 到云端
上传 Cache 到 Steam Cloud
上传 Artwork Cache 到 Steam Cloud
上传分析原始音频到云端
上传 MFCC / Embedding 到第三方服务
新增遥测
新增广告
新增账号系统
新增 ifuyo 云端账户
新增服务器端数据库
新增支付系统
新增社交平台
新增聊天
新增排行榜系统
新增在线音乐目录
新增商业音乐平台抓取
新增 Spotify / YouTube / 网易云 / QQ 音乐 / Apple Music 提取
新增远程 JS
新增自动下载第三方可执行程序
新增插件市场
新增用户脚本执行
```

尤其禁止：

> 为了 Steam Cloud 而把用户音乐、Cache、完整数据库或大型本地数据上传到 Steam。

Steam Cloud 只能考虑存储**少量、明确属于 ifuyo Aura 自身的可迁移应用状态**。

---

# 4. 第一阶段：Steam Integration Architecture

新增独立模块，例如：

```text
aura-steam/
```

或者：

```text
src-tauri/src/platform/steam/
```

具体位置根据现有架构决定，但必须保持边界清晰。

建议抽象：

```text
SteamPlatform
SteamLifecycle
SteamAvailability
SteamIdentity
SteamAchievements
SteamStats
SteamCloud
SteamRichPresence
SteamOverlay
SteamBranch
SteamDiagnostics
```

以及：

```text
SteamProvider
```

或者等价接口。

核心业务不得直接调用第三方 Steam API。

错误：

```rust
steamworks::achievement(...)
```

散落全项目。

正确：

```text
Application
   ↓
SteamService
   ↓
SteamProvider
   ↓
Steamworks Binding
```

---

# 5. Steam Lifecycle

实现完整 Steam lifecycle：

```text
Uninitialized
↓
Initializing
↓
Available
↓
Running
↓
ShutdownRequested
↓
Shutdown
```

异常：

```text
InitializationFailed
SteamUnavailable
SteamClientNotRunning
ApiUnavailable
VersionMismatch
ShutdownFailed
```

必须区分：

```text
Steam unavailable
```

和：

```text
ifuyo Aura unavailable
```

这是两个完全不同的问题。

Steam 不可用时：

```text
Local playback = continue
Library = continue
Cache = continue
Dreamscape = continue
Music Map = continue
SMTC = continue
Tray = continue
```

而不是整个应用退出。

---

# 6. Steam App ID 管理

严格区分：

```text
Development
Steam Local Test
Beta
Production
```

绝不能把真实 App ID 硬编码到大量业务代码。

建议：

```text
SteamEnvironment
SteamAppId
SteamBranch
SteamBuildId
SteamChannel
```

集中管理。

开发环境可以支持：

```text
steam_appid.txt
```

但必须确保：

- 只用于开发；
- 不进入 Production artifact；
- 不进入正式 Steam Depot；
- 不进入最终独立 installer；
- CI release artifact 中不存在；
- `.gitignore` 和 release validation 都检查。

不要因为开发方便而把：

```text
steam_appid.txt
```

永久放进正式产物。

---

# 7. SteamAPI 初始化

按照当前 Steamworks 官方 API 文档实现生命周期。

要求：

```text
SteamAPI_Init
SteamAPI_RunCallbacks
SteamAPI_Shutdown
```

或者当前 SDK/binding 对应的等价机制。

必须：

- 明确初始化顺序；
- 明确 shutdown 顺序；
- 明确 callback polling 所在线程；
- 明确线程安全边界；
- 明确 Rust ownership；
- 明确 Tauri lifecycle；
- 明确 single-instance 下 Steam 初始化只能属于主实例。

禁止：

```text
React component → SteamAPI_Init
```

禁止：

```text
Zustand → Steam native API
```

禁止：

```text
audio callback → Steam API
```

---

# 8. Steam Restart / Launch Semantics

研究并实现当前 Steamworks 官方推荐的 Steam 启动流程。

需要处理：

```text
Direct Launch
Steam Launch
Debugger Launch
Shortcut Launch
Second Instance Launch
```

必须避免：

- 启动循环；
- Steam 重启死循环；
- second instance 重复初始化；
- Steam API 初始化失败后的异常状态；
- installer/uninstaller 意外进入 Steam restart；
- updater 过程被 Steam restart 打断。

如果使用：

```text
SteamAPI_RestartAppIfNecessary
```

必须严格按照当前 Steamworks 生命周期要求放置调用位置。

开发模式和 Production 模式必须分别处理。

---

# 9. Non-Steam Build

ifuyo Aura 必须继续支持：

```text
Standalone Windows Build
```

即：

```text
Steam 未安装
Steam 客户端未运行
Steam API 不存在
非 Steam 启动
开发环境
```

都不能导致播放器失效。

设计：

```text
SteamProvider
    ├── RealSteamProvider
    └── NullSteamProvider
```

Null Provider 必须是正式支持的状态，而不是测试临时 hack。

例如：

```text
Steam unavailable
```

应该得到：

```text
SteamIntegrationState::Unavailable
```

而不是：

```text
panic!
unwrap()
exit()
```

---

# 10. Steam Identity

Steam 用户身份只能作为：

```text
optional platform identity
```

存在。

禁止建立：

```text
Steam User = ifuyo Local User
```

这样的强绑定。

本地音乐库必须属于：

```text
local machine
```

而不是：

```text
Steam account
```

不要因此新增用户数据库。

如果需要读取：

```text
SteamID
PersonaName
Avatar
```

必须：

- 明确用途；
- 明确隐私边界；
- 不记录日志；
- 不持久化无必要的身份数据；
- 不发送到 ifuyo 服务端；
- 不用于广告；
- 不用于遥测；
- 不影响本地播放。

---

# 11. Steam Achievements

实现独立的：

```text
SteamAchievementService
```

不要把 achievement logic 散落到播放器核心。

建议使用少量、非强迫式成就。

例如从实际产品使用行为产生：

```text
第一次建立音乐库
第一次创建 Playlist
第一次完成长时间聆听
发现 Music Map
进入 Dreamscape
使用一次 EQ
第一次播放 Internet Radio
完成一次 Library Scan
```

但必须遵守：

> 不因为 achievements 改变产品本来的核心 UX。

不要新增：

```text
每日签到
连续登录
强制播放时长
强制完成任务
奖励广告
人为延长使用时间
```

Achievements 的核心目标是：

```text
记录自然产生的用户行为
```

而不是：

```text
制造留存压力
```

每个 Achievement 要有：

```text
API Name
Display Name
Description
Trigger
Conditions
Localization
Icon
Version
```

实现：

```text
AchievementDefinition
AchievementService
AchievementSynchronizer
```

必须具备：

- 幂等；
- 重复触发安全；
- offline safe；
- Steam unavailable safe；
- callback race safe；
- app restart safe；
- state reconciliation。

不要假设：

```text
SetAchievement()
```

调用成功后本地一定立即完成。

---

# 12. Steam Stats

实现可选 Stats Adapter。

Stats 必须只记录对产品本身有合理意义的数据。

例如：

```text
total_played_seconds
tracks_played
library_track_count_peak
playlists_created
radio_sessions
map_interactions
dreamscape_sessions
```

但在决定具体字段前，先检查是否真的有产品意义。

不要为了“Steam 有 Stats”而添加大量统计。

严禁：

```text
隐蔽统计
广告统计
用户画像
跨设备跟踪
商业遥测
第三方分析
```

Steam Stats 与 ifuyo 本地统计必须明确区分：

```text
Local Playback Statistics
        ≠
Steam Stats
```

不要让 Steam 成为本地统计数据库的唯一真相来源。

---

# 13. Steam Cloud

这是 M13 中必须特别谨慎的一块。

原则：

```text
Cloud = optional synchronization of app-owned state
```

绝不 Cloud：

```text
Music files
Audio cache
Artwork cache
Large analysis blobs
Waveform cache
Embedding database
Dreamscape binary cache
Temporary files
Logs
Crash dumps
Steam SDK files
```

默认优先考虑同步：

```text
Settings
Small user preferences
Selected playlists metadata
UI preferences
Dreamscape lightweight state
Music Map saved views
Bookmarks
```

但必须先定义：

```text
CloudManifest
```

明确每个字段：

```text
path
purpose
size limit
format
version
migration strategy
privacy class
conflict policy
```

---

# 14. Steam Cloud Conflict Handling

必须设计：

```text
LocalOnly
CloudOnly
BothSame
BothDifferent
CorruptLocal
CorruptCloud
MigrationRequired
Conflict
```

不能简单：

```text
Cloud always wins
```

也不能：

```text
Local always wins
```

至少需要：

```text
version
timestamp
schema_version
content_hash
```

或者等价可靠机制。

必须避免：

```text
旧设备覆盖新数据
新版本读取旧 schema 崩溃
Cloud corruption 导致 Library corruption
Cloud sync 导致播放状态错乱
Cloud sync 导致大量文件写入
```

Cloud merge 应该是：

```text
Metadata merge
```

而不是：

```text
binary database overwrite
```

尤其谨慎处理 SQLite：

不要为了简单而：

```text
cloud sync entire library.sqlite
```

如果已有 Library DB 不适合 Cloud，则明确：

```text
not cloud synchronized
```

不要强行同步。

---

# 15. Cloud Data Size Budget

为 Cloud 建立严格预算。

例如：

```text
Max total cloud payload
Max single file
Max frequency
Max write burst
Max retry
```

实际数字必须通过 Steamworks 当前限制和本项目数据结构确定。

所有超出限制的状态：

```text
fail closed
```

而不是偷偷上传。

---

# 16. Steam Rich Presence

实现：

```text
SteamRichPresenceService
```

但默认以**隐私最小化**为原则。

建议状态：

```text
Listening
Playing Radio
Browsing Library
Exploring Music Map
In Dreamscape
```

对于正在播放的音乐：

不要默认上传完整：

```text
local path
file name
full metadata
directory structure
```

如显示当前曲目，应使用：

```text
artist
title
```

且必须：

- 不上传本地路径；
- 不上传用户私人目录；
- 不上传播放列表完整内容；
- 不上传私密标签；
- 不上传音频文件；
- 不连接 ifuyo server。

最好提供：

```text
Enable Steam Rich Presence
```

独立设置。

默认策略必须与 M12 Privacy 文档一致。

---

# 17. Steam Overlay

Steam Overlay 不得成为产品核心依赖。

应采用：

```text
Optional Capability
```

而不是：

```text
Required Runtime
```

Steam Overlay 不可用时：

```text
ifuyo Aura continues normally
```

不要在业务层写：

```text
if overlay unavailable -> disable app
```

可以增加：

```text
Steam Overlay available
Steam Overlay unavailable
```

作为诊断信息。

如果当前 Steamworks API 对 Tauri/WebView 有特殊限制，必须查阅最新官方文档并记录结论。

不要为了 Overlay 引入复杂的第二套 UI Runtime。

---

# 18. Steam Command / Callback Dispatch

所有 Steam callbacks 必须进入一个受控 dispatcher。

建议：

```text
Steam Callback
      ↓
Steam Event
      ↓
Application Event Bus
      ↓
Relevant Service
```

不要：

```text
Steam callback
      ↓
直接修改 React state
```

也不要：

```text
Steam callback
      ↓
直接操作 Playback Core
```

核心事件例如：

```text
SteamAvailable
SteamUnavailable
SteamUserChanged
AchievementStored
StatsReceived
CloudChanged
OverlayActivated
RichPresenceUpdated
```

必须：

- 类型化；
- 可测试；
- 可记录 revision；
- 生命周期安全；
- shutdown 后不可回调已释放对象。

---

# 19. Steam + M10 Playback / SMTC

M10 已经建立：

```text
Playback Authority
```

M13 不得破坏。

正确结构：

```text
Media Key
SMTC
Tray
UI
Steam
Command Palette
Mini Player
        ↓
Playback Commands
        ↓
Playback Authority
```

而不是：

```text
Steam Play command
    ↓
direct audio manipulation
```

Steam 只发送：

```text
Play
Pause
Next
Previous
Stop
Seek
```

这些都必须走现有 Application Command。

不要创建第二套 queue / playback engine。

---

# 20. Steam + Tray

Tray 应显示：

```text
ifuyo Aura
```

而不是：

```text
Aura
```

必要时可以显示 Steam 状态：

```text
Steam: Connected
Steam: Unavailable
```

但不要污染默认用户 UI。

Tray Exit 必须：

```text
formal application shutdown
    ↓
Steam shutdown
    ↓
SMTC cleanup
    ↓
cache flush
    ↓
analysis worker stop
    ↓
renderer cleanup
    ↓
process exit
```

顺序根据现有生命周期决定。

禁止：

```text
process::exit()
```

绕过 cleanup。

---

# 21. Steam + Single Instance

M10 single-instance 是 M13 必须继承的核心。

必须保证：

```text
Only one active playback process
Only one Steam session owner
Only one SMTC owner
Only one tray owner
```

测试：

```text
Launch A
Launch B
Launch C
```

结果：

```text
A remains primary
B/C exit or forward activation
No duplicate playback
No duplicate Steam initialization
No duplicate tray
No duplicate SMTC
```

如果 second instance 从 Steam 启动，也必须正确处理。

---

# 22. SteamPipe

建立正式 SteamPipe 构建结构。

建议：

```text
steam/
  app/
    app_build.vdf
    depot_windows.vdf
  scripts/
  content/
```

具体目录根据项目现状调整。

必须明确：

```text
App
Depot
Build
Branch
```

之间的关系。

必须支持：

```text
Preview Build
Beta Build
Production Build
```

至少完成：

```text
Windows x64
```

正式配置。

SteamPipe 的上传过程不得依赖：

```text
developer local absolute path
```

不能出现：

```text
C:\Users\Name\Desktop\...
D:\Projects\...
```

之类的硬编码。

---

# 23. Steam Depot 内容策略

必须明确什么进入 Steam Depot。

应进入：

```text
ifuyo Aura executable
required runtime files
required application resources
required native Steam runtime binaries
licenses / notices
release metadata
```

不应进入：

```text
developer secrets
Steam private keys
code signing private keys
steam_appid.txt
dev certificates
local databases
user music
cache
logs
test fixtures
debug symbols unless deliberately configured
CI artifacts unrelated to runtime
local absolute path data
personal data
temporary files
```

建立：

```text
SteamArtifactManifest
```

用于构建后检查。

---

# 24. Steamworks SDK 管理

这是 Public GitHub Repo 的重要边界。

必须检查当前 Steamworks SDK 使用协议。

原则：

```text
Source SDK content ≠ public application source
Runtime redistributables ≠ full SDK
```

如果 Steamworks SDK 不允许公开提交：

```text
do not commit it
```

可以：

```text
local developer installation
private CI artifact
secure build dependency
```

但必须确保构建 reproducibility。

README 中说明：

```text
How to obtain Steamworks SDK
Required version
Where to place it locally
How CI provides it
Which runtime files are redistributed
```

不要上传：

```text
Steamworks SDK private/internal content
```

到 Public GitHub。

---

# 25. Steamworks Binding 选择

Rust 并没有要求你必须手写 Steamworks FFI。

先审查当前生态。

允许：

```text
maintained permissive Rust binding
official C ABI + thin Rust wrapper
well-audited third-party binding
```

选择必须审查：

```text
maintenance activity
Steamworks SDK compatibility
license
API coverage
unsafe usage
build complexity
Windows support
distribution constraints
public-repo compatibility
```

禁止因为“代码少”就选择一个：

```text
abandoned
unmaintained
unclear license
copyleft-incompatible
```

的 binding。

必须更新：

```text
docs/THIRD_PARTY_LICENSES.md
```

并记录选择理由。

---

# 26. Steam SDK License Boundary

必须特别检查：

```text
Steamworks SDK license
binding license
native redistributable license
application license
transitive dependency licenses
```

注意：

> “库是 MIT” 并不意味着 Steamworks SDK 本身就是 MIT。

不要自行猜测许可。

必须使用：

```text
actual upstream license
official SDK terms
SPDX where applicable
```

进行判断。

如果产生：

```text
Unknown
Review Required
Potentially incompatible
```

必须标记：

```text
BLOCKED
```

或：

```text
MANUAL ACTION REQUIRED
```

而不是伪造：

```text
PASS
```

---

# 27. Steam Branches

实现：

```text
default
beta
```

至少这两个概念。

可以预留：

```text
experimental
internal
```

但不能对用户开放。

要求：

```text
Production = stable
Beta = explicit opt-in
```

Beta 必须独立测试：

```text
installer
updater
Steam API
Cloud schema
Achievements
Native DLL
release config
```

不能让 beta build 意外污染 production channel。

---

# 28. Steam Local Content Server / Preview

为开发建立：

```text
SteamPipe Preview
Local Content Server
```

测试路径。

根据当前 Steamworks 文档实现。

尤其注意当前 Steam 对新建 local branch / private branch 的规则变化。

所有本地测试必须：

```text
developer-only
```

不得进入 production configuration。

---

# 29. Steam Store Integration

M13 不只是写代码，还需要建立 Store 发布工程。

新增文档：

```text
docs/STEAM_STORE.md
```

至少定义：

```text
Product Name
Short Description
Long Description
Features
System Requirements
Languages
Categories
Tags
Screenshots
Capsule Assets
Library Assets
Icon
Logo
Trailer
Privacy Statement
Support URL
Website URL
Version information
Release channel
```

品牌统一：

```text
ifuyo Aura
```

禁止在用户可见 Steam 商店素材里出现：

```text
internal codename
developer-only terminology
```

除非它是技术文档而不是产品 branding。

---

# 30. Steam Store Visual Assets

建立：

```text
steam/store/
```

或者等价目录。

至少准备：

```text
header capsule
small capsule
main capsule
vertical capsule
screenshots
library capsule
library hero
library logo
icon
```

具体尺寸必须以当前 Steamworks 官方模板为准，不允许根据旧经验硬编码。

设计原则：

```text
ifuyo Aura
Music
Dream
Atmosphere
Local Library
Music Map
Dreamscape
```

整体视觉必须与：

```text
ifuyo · 伊芙游
```

保持一致。

禁止：

```text
review score
fake award
fake press quote
夸张性能数字
不存在的功能
不存在的 AI 能力
不真实的云服务
```

---

# 31. 商店截图不能骗人

截图必须来自：

```text
actual ifuyo Aura
```

不得：

```text
Photoshop fake UI
虚构不存在的 Music Map
虚构不存在的 Dreamscape
虚构 Steam Cloud
虚构 AI 推荐
虚构在线音乐目录
```

Store copy 所描述的功能必须和当前真实 Build 一致。

---

# 32. Steam Store Content Audit

建立一个 Store Feature Matrix：

```text
Feature
Implemented?
In Steam Build?
Visible in store?
Tested?
Documented?
Localization complete?
```

避免出现：

```text
Store says X
Build actually does Y
```

这是 M13-A 的发布阻断项。

---

# 33. Steam Languages

至少确保：

```text
English
Simplified Chinese
```

能够正确运行。

如果 Store 声称支持：

```text
Japanese
Traditional Chinese
Korean
German
...
```

则必须确认实际 UI coverage。

不要因为翻译文件存在就声明语言完整。

---

# 34. Steam User Data Boundary

明确：

```text
Steam-owned data
ifuyo-local data
User-owned music
```

三种不同类别。

建议文档：

```text
docs/STEAM_DATA_BOUNDARY.md
```

定义：

| 数据 | 本地 | Steam | Cloud | Telemetry |
|---|---:|---:|---:|---:|
| Music files | YES | NO | NO | NO |
| Library DB | YES | NO | NO / limited | NO |
| Playlists | YES | optional metadata | optional | NO |
| Settings | YES | NO | optional | NO |
| Artwork cache | YES | NO | NO | NO |
| Audio cache | YES | NO | NO | NO |
| Analysis | YES | NO | normally NO | NO |
| Dreamscape state | YES | optional | optional | NO |
| Music Map saved views | YES | optional | optional | NO |
| Steam achievements | Steam | YES | Steam-managed | NO |
| SteamID | transient | Steam | N/A | NO |

实际内容必须以最终实现为准。

---

# 35. Steam Cloud 与 M5 Cache 必须完全隔离

明确：

```text
M5 Cache
     ≠
Steam Cloud
```

Cache 是：

```text
performance/storage layer
```

Steam Cloud 是：

```text
optional user-state synchronization layer
```

Cache 永远不得因为：

```text
Steam login
Steam cloud enabled
```

突然变为可上传数据。

---

# 36. Steam + M8 Analysis

M8：

```text
BPM
Key
LUFS
MFCC
Embedding
```

都属于本地分析。

Steam 不应该读取原始分析数据库上传云端。

如果未来需要 Cloud synchronization：

只允许研究：

```text
small derived preferences
saved map views
analysis-independent state
```

不能直接：

```text
sync analysis.sqlite
```

除非经过严格重新设计和安全审计。

M13 不要求这么做。

---

# 37. Steam + M9 Music Map

Steam 只需要考虑：

```text
saved view
bookmark
layout preference
```

不需要上传：

```text
whole graph
all embeddings
all vectors
audio metadata database
```

Music Map 的核心计算继续本地进行。

---

# 38. Steam + M7 Dreamscape

Steam Cloud 可以考虑：

```text
seed
world preference
selected scene
favorite environment
small persistent state
```

但：

```text
render cache
Three.js GPU resources
large generated assets
temporary simulation
```

绝不进入 Cloud。

---

# 39. Steam Deck / Proton

M13-A：

```text
评估
```

M13-B：

```text
验证
```

不要直接承诺：

```text
Steam Deck Verified
```

除非真实完成对应验证流程。

因为当前 ifuyo Aura 是：

```text
Windows-first Tauri 2 application
```

需要实际评估：

```text
Windows build under Proton
Native Linux build feasibility
Tauri WebView behavior
audio output
filesystem permissions
Steam input
window mode
text input
controller navigation
Mica/window APIs fallback
SMTC behavior
Tray differences
Steamworks initialization
```

尤其注意：

```text
Windows-only native integrations
```

可能与 Proton / SteamOS 行为不同。

如果不能完整验证：

```text
M13-B = NOT COMPLETED
```

不要写：

```text
Steam Deck supported
```

---

# 40. Steam Deck UI

如果实际验证 Deck：

至少评估：

```text
1280×800
small display
touch
controller
keyboard absence
on-screen keyboard
focus navigation
font scaling
library navigation
search
playlist interaction
Now Playing
Music Map
Dreamscape
settings
```

但不要为了兼容 Deck 而在 M13 重写整个 UI。

只有实际阻塞问题才进行针对性修改。

---

# 41. Steam Input

因为 ifuyo Aura 是工具型桌面应用：

Steam Input 不是默认核心功能。

先做：

```text
compatibility assessment
```

必要时提供：

```text
Play/Pause
Previous
Next
Volume
Open Search
Toggle Mini Player
```

但不要因此构建完整游戏手柄输入框架。

所有输入仍然应该最终进入：

```text
Application Commands
```

---

# 42. Steam Auto-Cloud vs Cloud API

先根据实际需求决定：

```text
Steam Auto-Cloud
```

还是：

```text
ISteamRemoteStorage
```

不要为了“技术炫技”使用 Cloud API。

判断标准：

```text
data size
conflict complexity
migration
atomicity
versioning
user control
privacy
cross-machine behavior
```

如果简单的 Auto-Cloud 已经足够，则不要引入不必要的复杂 API。

---

# 43. Steam Runtime Artifacts

Windows Steam Build 必须检查：

```text
steam_api64.dll
```

或当前 SDK 对应的合法 runtime artifact。

必须验证：

```text
present
correct architecture
correct location
correct release configuration
not developer-only
not accidentally omitted
```

不能把：

```text
wrong architecture DLL
debug DLL
old SDK DLL
```

带进正式 Build。

---

# 44. Dependency Architecture

最终推荐：

```text
Application
├── Playback
├── Library
├── Analysis
├── Dreamscape
├── Music Map
├── Windows Platform
├── Steam Platform
└── Release Platform
```

依赖方向：

```text
Domain/Core
    ↓
Application
    ↓
Platform Adapters
```

不要：

```text
Core → Steam
Core → Windows
Core → Tauri
```

尤其：

```text
aura-core
```

不得新增 Steamworks import。

---

# 45. Rust / Tauri Boundary

Steam privileged operations 必须在 Rust 侧执行。

前端只能调用：

```text
steam_get_state
steam_get_identity
steam_get_capabilities
steam_enable_feature
steam_get_diagnostics
```

之类的 typed command。

禁止：

```text
frontend arbitrary Steam command
```

所有参数：

```text
validated
bounded
typed
```

---

# 46. Security Requirements

继承 M12。

特别检查：

```text
Steam callback input
Steam user metadata
Steam cloud paths
branch names
App IDs
artifact URLs
Steam-generated paths
native library loading
DLL search path
environment variables
command line
```

禁止：

```text
arbitrary DLL path
arbitrary executable launch
Steam-provided path directly passed to shell
```

Steam integration 不得扩大 Tauri capability。

---

# 47. Steam Cloud Path Safety

如果实现 Cloud：

必须限制：

```text
relative paths only
known directory
allowed file extensions
size limit
depth limit
no absolute paths
no UNC
no traversal
no symlink escape
no junction escape
```

严禁：

```text
..\..\something
C:\...
\\server\share
file://
```

形式进入 Cloud writer。

---

# 48. Store / Build / Runtime 三层分离

必须明确：

```text
Steam Store Metadata
Steam Build Metadata
Runtime Steam Integration
```

三者不能混为一谈。

例如：

```text
store_assets/
steam_build/
src-tauri/src/platform/steam/
```

职责清晰。

---

# 49. CI/CD

扩展 M11 pipeline。

至少增加：

```text
steam-validate
steam-build
steam-artifact-audit
steam-content-preview
```

生产上传必须满足：

```text
release tag
signed application
validated installer
validated runtime
license audit PASS
security audit PASS
artifact audit PASS
Steam config valid
```

---

# 50. Steam Credentials

绝不把：

```text
Steam password
Steam Guard code
Steam Web API key
Steamworks publishing credentials
Steam partner secrets
Steam Cloud credentials
```

放进：

```text
repo
source code
frontend
logs
artifact
README
docs
```

CI 使用：

```text
GitHub Secrets
```

或等价 secret manager。

PR 不得获得生产 Steam publishing credentials。

---

# 51. Steam Web API

除非确有必要，否则不要使用：

```text
Steam Web API
```

尤其不要为了：

```text
ownership check
analytics
user profile
```

额外建立远程后端。

本项目默认：

```text
No ifuyo backend
```

本阶段继续维持。

如某 API 需要 server-side secret：

```text
M13 = BLOCKED / DEFERRED
```

不要为了实现它而偷偷增加 backend。

---

# 52. Steam Ownership

ifuyo Aura 本地运行不应因为：

```text
Steam ownership API unavailable
```

而停止本地能力。

Steam 启动上下文只用于：

```text
platform integration
```

不是：

```text
license authority for core runtime
```

实际 DRM / license enforcement 必须遵守当前 Steamworks 规则，并保持与独立分发版本的架构隔离。

不要自行实现脆弱 DRM。

---

# 53. Steam Release Configuration

增加：

```text
Steam Release Configuration
```

例如：

```text
steam/app_build.vdf
steam/depot_windows.vdf
steam/README.md
steam/config/
steam/scripts/
```

其中：

```text
AppID
DepotID
Branch
Description
ContentRoot
Executable
LaunchOptions
```

都必须经过环境变量或模板机制安全注入。

禁止：

```text
production secret baked into source
```

---

# 54. Steam Launch Options

最终 Steam launch configuration 必须只调用正式入口。

不要加入：

```text
-debug
-dev
--unsafe
--allow-local-content
--skip-signature
--no-security
```

之类开发参数。

Release Build 必须确认：

```text
debug flags absent
test flags absent
mock provider absent
fake Steam mode absent
```

除非它们被严格编译时隔离。

---

# 55. Steam Beta / Internal Testing

至少规划：

```text
Default
Beta
```

推荐流程：

```text
main
 ↓
CI
 ↓
Release Candidate
 ↓
Steam Beta
 ↓
Smoke Test
 ↓
Production
```

Beta 必须测试：

```text
installation
upgrade
launch
audio
SMTC
tray
single-instance
library
cache
analysis
dreamscape
map
cloud
achievements
shutdown
```

---

# 56. Upgrade Testing

必须验证：

```text
Standalone vNext → current Steam
Steam vN → Steam vN+1
Steam Beta → Steam Stable
```

重点确认：

```text
Library survives
Playlists survive
Settings survive
Analysis survives
Cache survives safely
Dreamscape state survives
Music Map state survives
Steam state doesn't corrupt local state
```

不应该重新扫描所有音乐库，除非确有 migration requirement。

---

# 57. Uninstall Testing

Steam uninstall 必须确认：

```text
Application binaries removed
User music untouched
User library data policy correct
Cache policy correct
Steam Cloud state follows Steam rules
local settings policy documented
```

不要误删：

```text
C:\Music
D:\Music
network folders
```

---

# 58. Steam Install Location

不要假设：

```text
C:\Program Files (x86)\Steam\
```

一定存在。

Steam library 可能位于：

```text
other drive
custom path
```

因此：

```text
Steam install path ≠ application data path
```

所有路径必须动态处理。

---

# 59. Diagnostics

增加：

```text
Steam Diagnostics
```

至少可以查看：

```text
Steam detected
Steam API initialized
Steam App ID
Steam build/channel
Steam branch
Steam user available
Achievements available
Cloud available
Rich Presence available
Overlay available
Steam DLL version
```

但：

```text
SteamID
credentials
tokens
private data
```

必须默认脱敏。

---

# 60. Diagnostics UI

可以在：

```text
Settings → Advanced → Diagnostics
```

增加 Steam section。

例如：

```text
Steam Integration
Status: Connected

App ID: ****
Branch: beta
Build: xxxxx

Achievements: Ready
Cloud: Enabled
Rich Presence: Disabled
Overlay: Available
```

但正式用户界面不要暴露内部调试术语。

---

# 61. Error UX

错误必须区分：

```text
Steam unavailable
Steam client not running
Steam SDK unavailable
Steam initialization failed
Cloud unavailable
Achievement request failed
Steam build mismatch
Steam branch unavailable
```

UI 不应该出现：

```text
FATAL ERROR: Steam API returned false
```

而应该转成用户可理解文本。

日志中保留详细技术原因。

---

# 62. Offline Behavior

拔网线 / Steam Offline 模式测试必须覆盖：

```text
local music playback
cached playback
library
playlist
analysis
dreamscape
music map
settings
SMTC
tray
```

Steam-dependent features：

```text
Cloud
Achievements
Rich Presence
```

可以暂时 unavailable。

不得导致：

```text
local player unavailable
```

---

# 63. No Telemetry

M13 不增加任何：

```text
Google Analytics
Sentry
PostHog
Mixpanel
Amplitude
自建 telemetry
```

Steam features 本身也不能被用作隐藏遥测系统。

必须更新：

```text
docs/PRIVACY.md
```

明确：

```text
Steam platform interaction
```

与：

```text
ifuyo analytics
```

的区别。

---

# 64. Steam Privacy Model

新增：

```text
docs/STEAM_PRIVACY.md
```

说明：

```text
What Steam receives
What ifuyo receives
What remains local
What Cloud synchronizes
What Rich Presence exposes
What Achievements expose
What is never uploaded
```

不得含糊写：

```text
We respect privacy
```

必须描述实际数据流。

---

# 65. Network Model

Steam API 属于：

```text
Steam platform network activity
```

但 ifuyo 自身的：

```text
music source network
radio
Radio Browser
HTTP
cache
```

仍然独立。

不要把：

```text
Steam network
```

和：

```text
source network
```

混成一个 networking subsystem。

M4/M5 逻辑继续保持原有边界。

---

# 66. Steam + Radio

不允许因为 Steam 集成而新增：

```text
Steam music catalog
```

或：

```text
Steam-delivered music
```

Radio 仍然来自 M4：

```text
Internet Radio
ICY
Radio Browser
direct HTTP
```

Steam 只是发行平台。

---

# 67. Steam Workshop

M13-B 可以评估：

```text
Steam Workshop
```

但默认：

```text
NOT IMPLEMENTED
```

除非找到真正合理的产品内容。

可以研究未来：

```text
visual presets
Dreamscape scenes
theme packs
map styles
```

但不要在本阶段实现完整 Workshop。

尤其禁止允许 Workshop 内容直接成为：

```text
arbitrary executable
arbitrary JS
arbitrary native library
```

Workshop 内容未来如实现，必须继续经过 M12 security boundary。

---

# 68. Steam Screenshots

可以评估 Steam Screenshots integration。

但不应让：

```text
Steam Screenshot API
```

成为截图系统唯一依赖。

浏览器/WebView/Three.js screenshot 必须继续有自己的本地实现。

Steam integration 只是：

```text
export/share adapter
```

---

# 69. Steam Timeline / Other APIs

不要因为 Steamworks API 很多就全部实现。

M13 只选择对：

```text
ifuyo Aura
```

真正有意义的功能。

明确：

```text
Implemented
Deferred
Rejected
Not Applicable
```

例如：

```text
Matchmaking = Not Applicable
Leaderboards = Not Applicable
Inventory = Not Applicable
Microtransactions = Not Applicable
Steam Networking = Not Applicable
Workshop = Deferred
Achievements = Implemented
Stats = Implemented / optional
Cloud = Implemented / optional
Rich Presence = Optional
Overlay = Optional
```

最终结果以实际判断为准。

---

# 70. Steam Store Category

研究当前 Steamworks 支持的 Application Type。

不要机械选择：

```text
Game
```

因为 ifuyo Aura 是：

```text
desktop music software
```

应根据 Steam 当前分类与产品定位评估：

```text
Software
```

以及具体适合的 Steam 商店分类 / tags。

不要在代码中硬编码 Steam 商店分类。

把结论记录到：

```text
docs/STEAM_STORE.md
```

---

# 71. Commercial Release Readiness

M13 需要形成：

```text
Steam Release Readiness Matrix
```

字段：

```text
Area
Status
Evidence
Blocking?
Owner
Manual Action
```

至少包括：

```text
Steamworks onboarding
App creation
App ID
Store page
Pricing
Build
Depot
Branch
Install
Launch
Update
SteamAPI
Achievements
Stats
Cloud
Rich Presence
Overlay
Standalone
Windows
Steam Deck assessment
Licenses
Privacy
Security
Artwork
Localization
Support
Release checklist
```

---

# 72. Steamworks Account Manual Actions

有些事情无法由代码完成。

必须明确列出：

```text
MANUAL ACTION REQUIRED
```

例如：

```text
Steamworks partner onboarding
App creation
Steam Direct fee
Bank/tax information
Identity verification
Store page submission
Pricing
Store assets upload
Steam build upload
Review submission
Release button
```

不要伪造：

```text
PASS
```

如果没有真实 Steamworks partner environment。

可以：

```text
CODE READY
CONFIG READY
UPLOAD READY
MANUAL ACTION REQUIRED
```

---

# 73. Steam Direct

不要在代码里写死 Steam Direct 商业条件。

在：

```text
docs/STEAM_RELEASE_CHECKLIST.md
```

中要求开发者按照当前 Steamworks 官方文档确认：

```text
current fee
onboarding
waiting periods
store review
build review
release process
```

因为这些属于平台政策，可能发生变化。

---

# 74. Steam Store Release Flow

建立：

```text
Coming Soon
↓
Store Review
↓
Build Review
↓
Beta
↓
Production
```

实际是否启用 Coming Soon / Beta，由真实 Steamworks 账号和商业计划决定。

不要把：

```text
CI success
```

等同：

```text
Steam released
```

---

# 75. Release Notes

建立 Steam release notes 模板：

```text
Version
Highlights
New
Improved
Fixed
Known Issues
Compatibility
Privacy
Licensing
```

不得写不存在的功能。

---

# 76. Steam Support Information

Store / README / website 至少保持：

```text
support URL
privacy URL
terms if required
source repository
license
third-party notices
```

保持：

```text
ifuyo Aura
ifuyo · 伊芙游
```

品牌一致。

---

# 77. Public GitHub Repository

M13 完成后仓库仍可以 Public，但必须再次审计：

```text
Steam App ID
Steam config
Steam SDK
Steam credentials
Steamworks secrets
store drafts
private signing config
CI secrets
user data
music files
```

其中：

```text
public App ID
```

如果按 Steam 当前开发流程本身可以公开，则可以保留在必要位置；

但：

```text
credentials
private keys
publishing secrets
```

绝不能公开。

不要把“App ID”与“secret”混为一谈。

---

# 78. Git History Audit

必须检查：

```text
git log -p
git grep
git rev-list --all
```

搜索：

```text
steam_password
SteamGuard
steam_api_key
steam_web_api_key
token
secret
private_key
certificate
```

以及：

```text
.env
*.key
*.pem
*.pfx
*.p12
```

必须同时扫描：

```text
working tree
git history
CI configuration
release artifacts
```

发现秘密：

```text
rotate/revoke
```

而不是只删除当前文件。

---

# 79. Artifact Security Audit

最终 Steam artifact 检查：

```text
.exe
.dll
installer
.zip
.vdf
manifest
symbols
release notes
license files
```

确认：

```text
No secrets
No local paths
No test credentials
No debug configs
No developer files
No dev Steam AppID file
No temporary files
No unrelated DLL
```

---

# 80. Reproducible Steam Build

Steam build 必须尽可能做到：

```text
same source
same lockfiles
same toolchain
same build config
same dependency versions
```

产生：

```text
same logical artifact
```

至少要求：

```text
deterministic metadata
build provenance
hash inventory
version manifest
```

如果完全 bit-for-bit deterministic 暂时做不到，不要假装做到了。

记录原因。

---

# 81. Steam Build Metadata

每个 Steam Build 生成：

```text
build-manifest.json
```

至少：

```json
{
  "product": "ifuyo Aura",
  "channel": "stable",
  "version": "...",
  "platform": "windows",
  "arch": "x64",
  "gitCommit": "...",
  "buildTime": "...",
  "steamAppId": "...",
  "depot": "...",
  "artifactSha256": "..."
}
```

敏感字段必须根据安全政策处理。

---

# 82. Version Authority

M11 已经建立 version authority。

M13 不得重新建立第二套版本号。

确保：

```text
package.json
Cargo.toml
tauri.conf
Windows installer
Steam Build
Steam Store version
Git tag
release manifest
```

彼此一致。

---

# 83. Steam Build Launch Smoke Test

正式 Build 至少真实测试：

```text
Install
Launch from Steam
Steam unavailable
Steam client offline
Direct launch
Play local track
Pause
Next
Previous
Seek
Volume
SMTC
Tray
Close
Reopen
Single instance
Library scan
Playlist
Dreamscape
Music Map
Settings
Shutdown
```

---

# 84. Steam Cloud Smoke Test

如果 B 完成 Cloud：

至少：

```text
Machine A
    create settings
    create playlist metadata
    exit

Machine B
    sync
    launch
    verify
```

同时：

```text
conflict test
corrupted data test
older version test
newer version test
offline test
large file rejection test
```

---

# 85. Achievement Smoke Test

至少测试：

```text
trigger
duplicate trigger
Steam unavailable
Steam reconnect
restart
offline state
delayed callback
multiple sessions
```

必须确保：

```text
no duplicate local event
no crash
no stuck state
```

---

# 86. Rich Presence Smoke Test

测试：

```text
Playing
Paused
Stopped
Radio
Map
Dreamscape
Private/local metadata
Steam unavailable
shutdown
```

确认：

```text
No local path leak
No directory leak
No credential leak
No stale artist/title
No update storm
```

---

# 87. Overlay Smoke Test

只验证：

```text
available
unavailable
toggle
focus return
```

不要因 Overlay 而引入：

```text
fullscreen hacks
window ownership hacks
renderer replacement
```

---

# 88. Performance

Steam integration 不得显著影响：

```text
audio callback
idle CPU
memory
startup
shutdown
renderer FPS
library scanning
analysis
map
dreamscape
```

尤其：

```text
Steam callbacks
```

不得高频唤醒 React UI。

UI 状态应该：

```text
event-driven
coalesced
throttled
```

---

# 89. Memory

Steam native layer 必须检查：

```text
startup memory
shutdown cleanup
callback lifetime
DLL lifetime
reconnect
branch switching
Cloud data
Achievement state
Rich Presence string allocations
```

不得：

```text
leak every track change
leak every callback
leak every playlist event
```

---

# 90. Audio Callback Rule

继续继承 M1/M2 最高约束：

Steam API 不得执行于 audio callback。

禁止：

```text
allocation
lock
filesystem
network
Steam API
Tauri IPC
logging
```

进入实时 callback。

---

# 91. Steam Callback Thread

建立：

```text
Steam Thread / Application Thread
```

或当前架构等价机制。

需要明确：

```text
which thread initializes
which thread pumps callbacks
which thread shuts down
```

不要依赖：

```text
UI thread
```

或者：

```text
audio thread
```

作为 Steam runtime。

---

# 92. Testing Architecture

新增：

```text
tests/steam/
```

至少：

```text
steam_mock
steam_lifecycle
steam_identity
achievement_mapping
stats_mapping
cloud_manifest
cloud_conflict
rich_presence
overlay_state
branch_config
appid_validation
artifact_validation
launch_mode
single_instance
```

使用：

```text
MockSteamProvider
```

测试绝大多数逻辑。

---

# 93. Native Steam Integration Tests

如果环境具备真实 Steam：

建立：

```text
Steam Native Integration Tests
```

否则：

```text
BLOCKED / MANUAL ACTION REQUIRED
```

不得将：

```text
mock tests PASS
```

描述成：

```text
real Steamworks PASS
```

二者必须分开。

---

# 94. Public CI

Public PR CI 不得要求：

```text
Steam account
Steam password
Steam Guard
Steamworks publishing secret
```

PR CI：

```text
lint
typecheck
unit test
integration test
Steam mock test
build
artifact audit
license audit
```

Release CI 才可以：

```text
optional Steam upload
```

并通过 protected environment。

---

# 95. Release Permissions

GitHub Actions production workflow 必须使用：

```text
least privilege
environment protection
manual approval when appropriate
```

尤其：

```text
Steam publishing
code signing
GitHub release
```

都不能成为普通 PR 的写权限。

---

# 96. Steam Upload Workflow

最终可以形成：

```text
Tag
 ↓
Validate
 ↓
Test
 ↓
Build
 ↓
Sign
 ↓
Artifact Audit
 ↓
Prepare Steam Content
 ↓
SteamPipe Preview
 ↓
Steam Beta Upload
 ↓
Smoke Test
 ↓
Production Upload
```

生产发行仍然需要人工确认。

不要设计成：

```text
every push → publish to Steam
```

---

# 97. Store / Build Mismatch Gate

必须在 release pipeline 里检查：

```text
Store advertised feature
        ↓
Production build
```

不存在则：

```text
BLOCK RELEASE
```

例如：

```text
Store says Music Map
Build doesn't contain Music Map
```

必须阻断。

---

# 98. Steam Branch Artifact Policy

不同 branch：

```text
beta
default
```

必须生成：

```text
known channel
known version
known git commit
```

防止：

```text
unknown build
wrong artifact
wrong branch
old installer
```

被误上传。

---

# 99. Steam Build Rollback

不要声称：

```text
automatic binary rollback
```

除非实际实现。

最低要求：

```text
known-good build
known-bad build
branch switching
manual rollback procedure
```

写入：

```text
docs/STEAM_RELEASE_RUNBOOK.md
```

---

# 100. Release Runbook

建立：

```text
docs/STEAM_RELEASE_RUNBOOK.md
```

至少包括：

```text
Pre-release audit
Version bump
Build
Sign
Artifact validate
SteamPipe preview
Beta publish
Smoke test
Production publish
Release verification
Rollback procedure
Incident procedure
Post-release verification
```

---

# 101. Incident Runbook

必须定义：

```text
Bad Steam build
Bad DLL
Broken Steam initialization
Cloud corruption
Achievement bug
Store/build mismatch
Wrong depot
Wrong branch
Expired certificate
Signing failure
CI credential compromise
```

每类问题：

```text
detect
stop
contain
rollback
communicate
repair
verify
```

---

# 102. Documentation Updates

M13 完成后更新：

```text
README.md
docs/ARCHITECTURE.md
docs/DECISIONS.md
docs/PROGRESS.md
docs/SECURITY.md
docs/PRIVACY.md
docs/THIRD_PARTY_LICENSES.md
docs/RELEASE_CHECKLIST.md
docs/CHANGELOG.md
docs/CODE_SIGNING_POLICY.md
docs/PERF.md
```

新增：

```text
docs/STEAM.md
docs/STEAM_STORE.md
docs/STEAM_RELEASE_CHECKLIST.md
docs/STEAM_RELEASE_RUNBOOK.md
docs/STEAM_DATA_BOUNDARY.md
docs/STEAM_PRIVACY.md
```

如果某文档实际不需要，可以合并，但不能让信息散落无处。

---

# 103. Architecture Decision Record

新增至少一个 ADR：

```text
ADR: Steamworks Integration Boundary
```

回答：

```text
Why Steam?
Why adapter architecture?
Why Steam is optional?
Why no Steam dependency in core?
Why achievements?
Why/no Cloud?
Why/no Workshop?
Why/no Web API backend?
How SDK/binding selected?
What happens without Steam?
```

---

# 104. Dependency Audit

M13 新增依赖必须逐项审核：

```text
Package
Version
Source
License
Maintainer
Activity
Security advisory
Runtime / build only
Steam SDK compatibility
Commercial distribution suitability
```

继续执行 M12 license policy：

```text
MIT
Apache-2.0
BSD-2-Clause
BSD-3-Clause
ISC
MPL-2.0
Unicode-3.0
CDLA-Permissive-2.0
OFL-1.1
CC0
```

对：

```text
GPL
AGPL
LGPL
unknown
NOASSERTION
custom
dual-license
```

进行实际审查。

不要机械认为：

```text
Steamworks compatible
```

就等于：

```text
license automatically safe
```

---

# 105. Security Scan

运行：

```text
cargo audit
cargo deny check advisories
npm audit
```

以及：

```text
secret scan
git history scan
artifact scan
Steam config scan
```

所有问题必须分类：

```text
production
development
transitive
reachable
not reachable
false positive
upgrade available
breaking upgrade
manual review
```

---

# 106. Build Commands

至少运行：

```powershell
npm run typecheck
npm run lint
npm test
npm run build

cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
cargo audit
```

然后：

```text
actual Tauri production build
actual Windows installer build
actual Steam content preparation
actual SteamPipe preview
```

如果环境允许：

```text
actual Steam Beta upload
```

没有真实 Steam 环境时：

```text
do not fake it
```

---

# 107. Native Windows Validation

真实 Windows 环境检查：

```text
Windows 10
Windows 11
x64
high DPI
light mode
dark mode
multiple monitors
custom Steam library path
non-default install path
Unicode user path
Unicode music path
long path
Steam installed
Steam not installed
Steam running
Steam offline
Steam not running
```

---

# 108. Manual Test Matrix

必须输出：

| Test | Expected | Actual | Status |
|---|---|---|---|
| Steam launch | Success |  |  |
| Direct launch | Success |  |  |
| No Steam | Local app works |  |  |
| Offline Steam | Local app works |  |  |
| Play | Works |  |  |
| Pause | Works |  |  |
| Previous | Works |  |  |
| Next | Works |  |  |
| Seek | Works |  |  |
| SMTC | Works |  |  |
| Tray | Works |  |  |
| Single instance | One owner |  |  |
| Achievement | Sync |  |  |
| Stats | Sync |  |  |
| Cloud | Sync / disabled |  |  |
| Rich Presence | Correct / disabled |  |  |
| Overlay | Works / unavailable |  |  |
| Upgrade | Data survives |  |  |
| Uninstall | Policy correct |  |  |
| Steam Beta | Works |  |  |
| Production artifact | Clean |  |  |

---

# 109. M13 的代码质量要求

禁止：

```text
god object
Steam helper dumping everything
global mutable Steam singleton without lifecycle
unsafe FFI scattered everywhere
Steam calls scattered in UI
Steam checks scattered everywhere
stringly-typed events everywhere
panic on unavailable Steam
```

尽可能建立：

```text
typed state
typed commands
typed events
central lifecycle
central policy
bounded input
isolated FFI
```

---

# 110. M13 不需要大规模重构

除非：

```text
existing architecture blocks Steam integration
```

否则：

```text
do not rewrite
do not rename everything
do not migrate framework
do not replace Tauri
do not replace React
do not replace audio engine
do not replace SQLite
do not replace renderer
```

优先：

```text
adapter
boundary
integration
tests
docs
release engineering
```

---

# 111. Definition of Done

M13-A 必须同时满足：

```text
[ ] Steam adapter exists
[ ] Steam lifecycle exists
[ ] Null Steam provider exists
[ ] Steam-independent startup works
[ ] AppID separation exists
[ ] Production does not ship steam_appid.txt
[ ] No Steam dependency in aura-core
[ ] Steam does not touch audio callback
[ ] Steam does not own Playback Authority
[ ] Single instance works
[ ] SMTC works
[ ] Tray works
[ ] Steam runtime DLL policy validated
[ ] Steam SDK/binding license audited
[ ] SteamPipe structure exists
[ ] Windows Steam artifact builds
[ ] Artifact audit passes
[ ] Docs updated
[ ] Security scan passes
[ ] Tests pass
[ ] Release configuration validated
```

---

# 112. M13-B Definition of Done

B 功能完成情况分别记录，不得混在 A：

```text
[ ] Achievements
[ ] Stats
[ ] Steam Cloud
[ ] Rich Presence
[ ] Overlay
[ ] Steam Beta branch
[ ] Steam Deck assessment
[ ] Proton assessment
[ ] Store assets
[ ] Store copy
[ ] Steam release runbook
[ ] Steam Beta build
[ ] Steam production candidate
```

每一项写：

```text
PASS
NOT COMPLETED
BLOCKED
NOT APPLICABLE
```

---

# 113. Final Audit

编码结束后必须先停止修改，执行完整发布审计。

输出：

# M13 Final Audit

## A. Architecture

## B. Steam Lifecycle

## C. AppID / Build / Branch

## D. Steamworks SDK / Binding

## E. Achievements / Stats

## F. Cloud

## G. Rich Presence

## H. Overlay

## I. SteamPipe

## J. Store Readiness

## K. Windows Build

## L. Steam Runtime

## M. Security

## N. Privacy

## O. Licensing

## P. CI/CD

## Q. Artifact Audit

## R. Upgrade / Uninstall

## S. Native Steam Tests

## T. Steam Deck / Proton

## U. Documentation

---

# 114. 必须区分三种状态

最终报告严禁模糊：

```text
CODE READY
```

```text
TEST PASS
```

```text
PLATFORM MANUAL ACTION REQUIRED
```

例如：

```text
Steam store page = MANUAL ACTION REQUIRED
Steam production upload = MANUAL ACTION REQUIRED
Steam partner onboarding = MANUAL ACTION REQUIRED
Real Steam Beta test = PASS
Steam mock tests = PASS
```

不要把没有真实 Steamworks 账号执行过的流程写成：

```text
PASS
```

---

# 115. 最终结果格式

最后必须严格输出：

```text
M13-A = PASS / BLOCKED / FAIL

M13-B = PASS / NOT COMPLETED

STEAM_NATIVE = PASS / BLOCKED

STEAM_BETA = PASS / BLOCKED / NOT COMPLETED

STEAM_PRODUCTION = READY / BLOCKED / NOT READY

MANUAL_ACTION_REQUIRED =
- ...

COMMIT = <hash>

NEXT = M14
```

如果因为缺少真实：

```text
Steamworks partner account
App ID
Steam credentials
production signing environment
real Steam client
real Steam test environment
```

而无法完成真实验证：

```text
BLOCKED
```

不要伪造结果。

---

# 116. Git 要求

M13 完成后：

```text
git diff
git status
```

必须检查。

逐项审查：

```text
security-sensitive files
Steam configs
SDK artifacts
licenses
CI
release scripts
installer
native binaries
docs
tests
```

确认：

```text
No secrets
No personal files
No local paths
No user music
No cache
No private Steam SDK material
No debug artifact
```

然后创建单独 commit：

```text
M13: integrate Steam and distribution
```

要求：

```text
working tree clean
commit contains only M13 work
```

**不要 push。**

---

# 117. 严格停止线

完成 M13 后立即停止。

不要开始：

```text
M14
Steam Workshop implementation
complex Steam social
backend
cloud service
accounts
payments
telemetry
subscription
mobile client
Mac client
Linux native rewrite
```

即使发现这些内容“很有意思”，也只能记录到：

```text
docs/FUTURE.md
```

不能顺手实现。

最终：

```text
M13 complete
→ stop
→ wait for next milestone
```

---

# 118. 最终工程目标

M13 完成后，ifuyo Aura 的架构应该清晰地变成：

```text
                    ┌────────────────────────────┐
                    │       ifuyo Aura          │
                    │     Application Layer      │
                    └─────────────┬──────────────┘
                                  │
            ┌─────────────────────┼─────────────────────┐
            │                     │                     │
      ┌─────▼─────┐        ┌──────▼──────┐       ┌────▼─────┐
      │   Core    │        │   Windows   │       │  Steam   │
      │ Playback  │        │ SMTC / Tray │       │ Adapter  │
      └─────┬─────┘        └─────────────┘       └────┬─────┘
            │                                           │
      ┌─────▼───────────────────────────────────────────▼─────┐
      │                    Local-first                         │
      │ Library / Cache / Analysis / Dreamscape / Music Map   │
      └───────────────────────────────────────────────────────┘
```

并且始终满足：

```text
Steam is optional.
Local playback is authoritative.
User music remains local.
Core remains platform-independent.
Security boundary remains intact.
Release pipeline is auditable.
Public repository remains safe.
```

---

# 119. M13 的一句话判断标准

最终不要问：

> “有没有把 Steam API 接上？”

真正应该问：

> **“ifuyo Aura 是否已经具备一个干净、可维护、可测试、可审计、不会绑架核心播放器架构的 Steam 商业发行层？”**

只有答案是：

```text
YES
```

M13-A 才能：

```text
PASS
```

最终停止，不进入 M14。