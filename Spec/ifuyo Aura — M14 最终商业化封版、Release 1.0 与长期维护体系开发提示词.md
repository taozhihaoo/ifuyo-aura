# ifuyo Aura — M14 最终商业化封版、Release 1.0 与长期维护体系

## 0. 任务身份

你现在负责的是：

> **ifuyo Aura M14 — Final Commercial Hardening, Release 1.0 & Long-Term Maintenance**

项目背景：

- 产品名称：**ifuyo Aura**
- 品牌：**ifuyo · 伊芙游**
- Repo：`ifuyo-aura`
- 技术栈：
  - Tauri 2
  - Rust
  - React
  - TypeScript
  - Vite
  - Zustand
  - Three.js
- 平台：
  - Windows 10 / Windows 11 x64 为正式目标平台
  - Steam 为商业发行渠道之一
- 产品定位：
  - Local-first desktop music player
  - 本地音乐、Direct HTTP(S)、Internet Radio 等合法来源的统一播放器
  - 音乐库、音频分析、Dreamscape、Music Map、Windows 媒体集成、Steam 集成
  - 无强制登录
  - 无遥测
  - 默认无 ifuyo 后端依赖
- 当前阶段：
  - M0–M13 已完成或已形成实施基线
  - M13 已完成 Steamworks 集成、Steam 分发结构与商业发行基础
  - 现在进入最终商业化封版阶段

---

# 1. M14 的核心目标

M14 **不是功能扩张阶段**。

M14 的唯一核心目标：

> **把现有 ifuyo Aura 从“工程上已经完成很多功能”提升到“可以真正交付给陌生用户、长期运行、升级、恢复、维护”的 Release 1.0 商业软件状态。**

最终应该形成：

```text
Development
    ↓
Internal Validation
    ↓
Release Candidate
    ↓
Steam Beta / Controlled Validation
    ↓
Production Release
    ↓
Post-release Monitoring
    ↓
Maintenance
```

M14 需要解决的不是“还能做什么”，而是：

```text
它是否稳定？
它是否可以升级？
升级失败能否恢复？
数据迁移是否安全？
音频播放是否长期稳定？
大型音乐库是否稳定？
Dreamscape 是否会泄漏资源？
Music Map 是否能处理真实数据规模？
Steam 不可用时是否仍然是一个完整播放器？
Windows 系统集成是否可靠？
安装、卸载、升级是否破坏用户数据？
Crash / failure 是否能定位？
发布包是否干净？
依赖、许可证、安全策略是否保持一致？
1.0 发布之后如何维护？
```

---

# 2. 最高优先级原则

严格遵守以下原则。

## 2.1 不得通过新增大型功能“解决”质量问题

M14 禁止借机增加：

- 新播放器核心能力
- 新音乐源
- 新大型分析算法
- 新视觉系统
- 新社交系统
- 登录系统
- 云端账户系统
- 推荐系统
- 在线音乐数据库
- AI 聊天
- 在线歌词平台
- YouTube / Spotify / 网易云 / QQ 音乐等平台提取
- Workshop 内容系统
- 大型插件市场
- 社交分享平台
- 游戏化签到
- 每日任务
- 强制互动
- 用户行为追踪
- 广告系统

除非修复某个已经存在的 P0/P1 release blocker，否则不要扩展功能。

---

# 3. 已有产品边界必须冻结

M14 以以下系统为最终基线：

```text
M0  Foundation
M1  Audio Core
M2  Playback Quality
M3  Music Library
M4  Sources
M5  Persistent Cache
M6  Desktop UI Foundation
M7  Dreamscape
M8  Audio Analysis
M9  Music Map
M10 Windows Integration
M11 Release Engineering
M12 Security / Privacy / Licensing
M13 Steamworks / Distribution
M14 Final Commercial Hardening / Release 1.0
```

M14 不得破坏已有架构边界。

---

# 4. 总体架构冻结

目标架构保持：

```text
Application
├── Audio
│   ├── aura-core
│   ├── decoder
│   ├── output
│   ├── playback authority
│   └── playback quality
│
├── Library
│   └── aura-library
│
├── Sources
│   └── source providers
│
├── Cache
│   └── persistent cache
│
├── Analysis
│   └── aura-analysis
│
├── Dreamscape
│   └── aura-synth
│
├── Music Map
│   └── aura-map
│
├── Windows Platform
│   └── SMTC / media keys / tray / single instance
│
├── Steam Platform
│   └── aura-steam
│
├── Release Platform
│   └── updater / installer / signing / CI
│
└── UI
    ├── React
    ├── Zustand
    ├── Tauri IPC
    └── Three.js
```

必须保持：

```text
UI
 ↓
Typed IPC
 ↓
Application Services
 ↓
Domain / Core
```

不得反过来。

---

# 5. 第一阶段：完整 M14 基线审计

## 5.1 审计要求

**先审计，再修改。**

第一步不得改代码。

必须读取真实项目当前状态：

```powershell
git status
git branch --show-current
git log -10 --oneline
git remote -v
```

然后识别：

- 当前 HEAD
- 当前 branch
- 是否存在未提交修改
- 是否存在未追踪文件
- 当前 version
- package version
- Cargo version
- Tauri version
- Steam App ID
- installer version
- updater version
- database schema version
- analysis version
- Dreamscape schema/version
- Music Map schema/version
- cache schema/version
- save/cloud schema/version

任何版本号冲突都必须记录。

---

# 6. M14 审计必须建立“最终系统矩阵”

创建内部审计文档：

```text
docs/M14_FINAL_AUDIT.md
```

建立至少以下矩阵：

| System | Current State | Test Coverage | Release Risk | Migration Risk | Recovery | M14 Action |
|---|---|---|---|---|---|---|
| Audio Core | | | | | | |
| Playback | | | | | | |
| Library | | | | | | |
| Sources | | | | | | |
| Cache | | | | | | |
| Analysis | | | | | | |
| Dreamscape | | | | | | |
| Music Map | | | | | | |
| Windows Integration | | | | | | |
| Steam | | | | | | |
| Installer | | | | | | |
| Updater | | | | | | |
| Security | | | | | | |
| Privacy | | | | | | |
| Licensing | | | | | | |

所有判断必须来自真实代码、测试、构建、日志和配置。

不得凭印象写：

```text
PASS
```

---

# 7. FACT / HYPOTHESIS / INFERENCE 审计规则

M14 延续项目已有审计规则。

所有重要结论必须分类：

```text
FACT
HYPOTHESIS
INFERENCE
```

例如：

```text
FACT:
cargo test --workspace = PASS

FACT:
真实 installer successfully produced

HYPOTHESIS:
某低端机器可能出现 WebView2 GPU fallback

INFERENCE:
需要增加 GPU fallback validation
```

禁止：

```text
应该没问题
大概稳定
感觉可以发布
应该兼容
理论上不会
```

---

# 8. P0 / P1 / P2 分类

只允许以下优先级：

```text
P0 = Release Blocker
P1 = Must Fix Before / During 1.0
P2 = Post-1.0 Improvement
```

## P0

包括但不限于：

- crash
- playback deadlock
- audio corruption
- user data loss
- migration corruption
- updater security failure
- installer failure
- Steam production packaging failure
- privilege escalation
- arbitrary code execution
- arbitrary file write
- SSRF
- secret leakage
- private key exposure
- broken single-instance authority
- broken Playback Authority
- unbounded memory growth
- unbounded cache growth
- severe GPU resource leak
- corrupted playlist/library
- release binary contains private development data

## P1

包括：

- serious UX bug
- recoverable playback failure
- major library scan problem
- moderate performance regression
- missing error explanation
- important accessibility regression
- important localization bug
- serious edge-case migration issue
- important Steam state mismatch

## P2

可以进入未来维护：

- cosmetic issue
- minor animation issue
- optional feature
- richer presets
- advanced visualization
- minor UI improvements

M14 默认不实现 P2。

---

# 9. Final Release Contract

创建：

```text
docs/RELEASE_1_0_CONTRACT.md
```

明确 Release 1.0 的：

- Supported OS
- Supported architecture
- Required runtime
- Installer format
- Update mechanism
- Audio format support
- Source support
- Library guarantees
- Cache behavior
- Analysis behavior
- Dreamscape behavior
- Music Map behavior
- Windows integration
- Steam integration
- Privacy model
- Offline behavior
- Known limitations
- Unsupported sources
- Unsupported platforms
- Data migration policy
- Crash/recovery policy

必须避免 Marketing 语言冒充技术承诺。

---

# 10. Version Authority 终审

建立唯一版本源。

检查：

```text
package.json
Cargo.toml
tauri.conf.json
installer metadata
Steam build metadata
updater metadata
About dialog
Git tag
Release notes
```

不得出现：

```text
1.0.0
1.0
0.9.9
v1.0
```

互相冲突。

定义：

```text
PRODUCT_VERSION
BUILD_NUMBER
GIT_COMMIT
RELEASE_CHANNEL
```

至少能够在 diagnostics 中得到：

```text
ifuyo Aura
Version: x.y.z
Build: n
Commit: abcdef
Channel: stable/beta/dev
Platform: windows-x64
```

---

# 11. Release Channel

必须明确：

```text
dev
beta
stable
```

行为差异：

### dev

允许：

- verbose diagnostics
- local fixtures
- local Steam app id file
- dev tools
- unsafe test switches

### beta

允许：

- controlled experimental validation
- Steam beta branch
- enhanced diagnostics

不得：

- debug secrets
- developer filesystem
- private signing keys
- test accounts
- fake production metadata

### stable

必须：

- production configuration
- no dev-only files
- no debug panel
- no test secrets
- no test content
- no `steam_appid.txt` unless explicitly required by a documented test artifact
- production update configuration
- production signing

---

# 12. Data Preservation Audit

这是 M14 最重要部分之一。

建立：

```text
docs/DATA_PRESERVATION.md
```

明确区分：

```text
User-owned data
Application-owned data
Cache data
Derived data
Temporary data
Logs
Steam Cloud data
```

推荐结构：

```text
User Data
├── Library Database
├── Playlists
├── Settings
├── Saved Views
├── Dreamscape State
└── Other user-created metadata

Derived / Rebuildable
├── Analysis
├── Artwork Cache
├── HTTP Cache
├── Render Cache
└── Temporary Files
```

升级时：

```text
User data MUST survive
Rebuildable cache MAY be deleted
Derived data MAY be rebuilt
Temporary data MAY be deleted
```

明确：

```text
installer uninstall ≠ delete user data
update ≠ delete user data
repair ≠ delete user data
crash recovery ≠ delete user data
```

---

# 13. Migration Finalization

M14 必须模拟多个真实升级链：

```text
fresh install
M12-like state → M13-like state → M14
M11-like state → M14
previous release → M14
beta → stable
stable → newer stable
```

测试：

- empty database
- small library
- large library
- corrupted database
- partially migrated database
- old schema
- unexpected fields
- missing files
- moved files
- deleted files
- broken artwork
- interrupted migration
- killed process during migration

migration 必须尽量：

```text
transactional
idempotent
recoverable
versioned
observable
```

禁止：

```text
silent destructive migration
```

---

# 14. Migration Recovery

设计并测试：

```text
backup
migration
validation
commit
```

失败时：

```text
restore backup
mark migration failed
show recoverable error
preserve original data
```

必须验证：

```text
kill process during migration
power-loss-like simulation
disk-full simulation
permission denied
locked database
corrupted migration state
```

---

# 15. Playback Stability Soak Test

建立正式播放稳定性测试。

至少覆盖：

```text
play
pause
resume
seek
next
previous
queue
repeat
shuffle
crossfade
gapless
volume
EQ
limiter
device switch
device unplug
device reconnect
sleep/wake
display sleep
application minimize
application restore
```

长时间测试：

```text
30 min
2 hour
8 hour
```

目标：

- 无持续内存增长
- 无 thread explosion
- 无 file descriptor leak
- 无 decoder leak
- 无 GPU resource leak
- 无 audio callback violation
- 无 stale generation audio
- 无 queue corruption
- 无 playback authority divergence

---

# 16. Audio Callback 最终审计

必须重新检查 M1 规则。

Audio callback 严禁：

```text
allocation
blocking lock
filesystem IO
network IO
logging
sleep
decoder disk IO
panic
UI calls
Steam calls
Tauri calls
```

必须进行实际验证，而不是只看代码。

建立：

```text
Audio Callback Safety Test
```

尽可能使用：

- allocation counter
- lock instrumentation
- callback timing
- underrun instrumentation
- stress playback
- device switch

记录：

```text
callback duration
worst callback duration
underruns
allocation count
lock contention
```

---

# 17. Playback Authority 最终一致性测试

M10 已建立 Playback Authority。

M14 必须证明它仍然是唯一事实源：

```text
UI
SMTC
media keys
tray
Steam
keyboard
mini player
queue
external commands
```

全部只能：

```text
send command
```

不能直接修改播放真相。

建立一致性矩阵：

| Command Source | Command | Authority Path | Result |
|---|---|---|---|
| UI | Play | Playback Authority | |
| UI | Pause | Playback Authority | |
| Media Key | Pause | Playback Authority | |
| SMTC | Seek | Playback Authority | |
| Tray | Next | Playback Authority | |
| Steam | Launch | Playback Authority | |

禁止形成第二套 playback state machine。

---

# 18. Library Stress Validation

必须建立真实压力 fixture。

至少：

```text
100 tracks
1,000 tracks
10,000 tracks
50,000 tracks
```

验证：

- scan
- incremental scan
- watcher
- metadata normalization
- FTS search
- filtering
- sorting
- pagination
- artwork lookup
- playback statistics
- playlist operations
- rename
- move
- delete
- re-import

记录：

```text
startup time
initial scan
incremental scan
search latency
memory
database size
UI response
```

---

# 19. Search Stress

至少测试：

```text
Chinese
English
Japanese
mixed language
single character
two characters
punctuation
emoji
very long query
empty query
special SQL-like strings
filesystem-like strings
```

确认：

```text
FTS query cannot escape intended semantics
SQL injection impossible through user input
UI search is cancellable
stale search results cannot overwrite newer results
```

---

# 20. Cache Final Audit

再次确认 M5 规则：

```text
Forbidden
SessionOnly
Allowed
```

必须证明：

### Forbidden

```text
0 persistence writes
0 permanent cache entries
0 artwork persistence
0 metadata persistence
```

### SessionOnly

```text
memory/temp only
cleanup on shutdown
```

### Allowed

```text
persistent cache
eviction
revalidation
integrity verification
```

2GB 默认缓存上限必须真实执行。

测试：

```text
cache full
disk full
partial download
corrupted chunk
ETag change
304
206
200 fallback
range unsupported
network interruption
reconnect
```

---

# 21. Network Failure Matrix

对所有网络来源验证：

```text
DNS failure
timeout
connection reset
TLS failure
HTTP 404
HTTP 403
HTTP 429
HTTP 500
HTTP 503
redirect
redirect loop
oversized body
slow response
invalid content type
invalid response headers
cancellation
```

要求：

- 不崩溃
- 不死锁
- 不无限重试
- 不产生无限缓存
- 不泄露内部路径
- 不泄露凭据
- 不打印敏感 URL 参数

---

# 22. SSRF / Private Network 最终回归

重新验证 M12。

必须覆盖：

```text
localhost
127.0.0.1
::1
RFC1918
link-local
metadata endpoints
IPv4 mapped IPv6
redirect to private IP
DNS rebinding
credential-bearing URL
file://
data:
javascript:
```

必须保持：

```text
rejected before dangerous network access
```

对于禁止平台提取源：

```text
request_count == 0
```

不要因为 M14 做 release refactor 就破坏 M4/M5 的 preflight rejection。

---

# 23. Security Regression

重新执行完整 M12 安全基线：

```text
filesystem traversal
UNC path
junction
symlink
reparse point
arbitrary file write
arbitrary process execution
DLL loading
IPC authorization
Tauri capability boundaries
CSP
unsafe frontend sinks
URL scheme validation
network access
secret handling
diagnostics redaction
updater validation
downgrade protection
dependency vulnerabilities
license violations
```

任何 M12 已通过的 P0 安全项被回归，立即视为 M14 P0。

---

# 24. Frontend Security Regression

搜索：

```text
dangerouslySetInnerHTML
innerHTML
eval
new Function
dynamic import
external script
iframe
window.open
location assignment
HTML injection
untrusted BBCode
unsafe markdown
unsafe URI
```

每一个命中都必须：

```text
JUSTIFIED
SANITIZED
OR REMOVED
```

音乐 metadata、网络标题、ICY title、RSS title、文件 tag 等全部视为：

```text
UNTRUSTED INPUT
```

---

# 25. Untrusted Media / Metadata Final Audit

验证：

```text
malformed audio
huge metadata
huge artwork
invalid UTF-8
invalid image
truncated image
malformed ID3
unusual sample rate
very long title
very long artist
very long album
duplicate metadata
path-like metadata
HTML-like metadata
```

不得导致：

- crash
- UI injection
- unbounded allocation
- SQL corruption
- path traversal

---

# 26. Dreamscape Final Stability

M7 系统必须进入长期运行测试。

测试：

```text
open
close
reopen
switch tracks
switch playlists
pause for long period
seek
volume changes
audio device changes
background / foreground
window resize
mini player
high item count
low GPU
GPU context loss
```

重点：

```text
memory stability
GPU resource stability
geometry disposal
texture disposal
WebGL context recovery
Three.js renderer cleanup
requestAnimationFrame lifecycle
event listener cleanup
subscription cleanup
worker cleanup
```

禁止：

```text
hidden window continues full-rate rendering
multiple render loops
orphaned RAF
orphaned timers
orphaned Web Workers
```

---

# 27. Dreamscape Visual Quality Freeze

不要再加大型视觉功能。

建立：

```text
M14 visual regression snapshots
```

至少包括：

```text
fresh seed
common seed
rare mutation seed
day
night
pause
playing
transition
low quality
medium quality
high quality
fallback mode
```

记录：

```text
rendered
fallback
broken
```

不要求像素级绝对一致，但必须识别：

- missing geometry
- broken lighting
- broken materials
- black scene
- blank scene
- clipping
- runaway density
- obvious shader regression

---

# 28. Music Map Final Stress

至少：

```text
0 tracks
1 track
10 tracks
100 tracks
1,000 tracks
5,000+ tracks
```

验证：

```text
build
rebuild
incremental update
missing embedding
invalid embedding
duplicate track
deleted track
reanalysis
cluster change
zoom
pan
selection
hover
search
filter
play selected
```

并验证：

```text
stable deterministic layout
```

不同机器运行：

```text
same input
same seed
same algorithm version
```

应该得到可接受的一致结果。

---

# 29. Analysis Final Pipeline Audit

M8 必须保持：

```text
local-first
deterministic where defined
versioned
incremental
cancellable
recoverable
```

验证：

```text
new track
reimport
metadata-only change
content change
analysis version change
model version change
cancel
retry
worker crash
invalid file
very long file
short file
silence
noise
```

没有 embedding 时：

```text
Map must degrade gracefully
```

不得：

```text
crash
fake embedding
silently substitute unrelated vectors
```

---

# 30. Performance Budget Freeze

创建：

```text
docs/PERFORMANCE_BASELINE.md
```

建立固定 baseline。

至少记录：

```text
cold startup
warm startup
idle CPU
playing CPU
Dreamscape CPU
Dreamscape GPU
Music Map CPU
Music Map GPU
memory at startup
memory after 1 hour
memory after 8 hours
library search
database load
first track playback
```

建议建立三个配置：

```text
Low
Balanced
High
```

所有“目标值”必须标明：

```text
MEASURED
TARGET
UNKNOWN
```

禁止把 TARGET 写成 MEASURED。

---

# 31. Idle Resource Policy

ifuyo Aura 是桌面常驻体验，因此：

当：

```text
window hidden
Dreamscape not visible
player idle
music paused
```

必须降低资源使用。

至少实现：

```text
render throttling
timers throttling
background worker throttling
analysis scheduler throttling
```

不得为了视觉存在感长期占满 CPU/GPU。

---

# 32. Low-End Windows Validation

创建：

```text
docs/WINDOWS_COMPATIBILITY.md
```

至少覆盖：

```text
Windows 10
Windows 11
x64
integrated GPU
discrete GPU
WebView2 environment
multiple audio devices
headphones
Bluetooth device
monitor sleep/wake
DPI scaling
100%
125%
150%
200%
```

重点测试：

```text
installer
startup
audio
SMTC
media keys
tray
window state
Dreamscape
Map
updater
uninstall
```

没有真实机器验证的项目必须明确写：

```text
NOT TESTED
```

不得写：

```text
PASS
```

---

# 33. DPI / Multi-monitor Final Validation

验证：

```text
single monitor
dual monitor
different DPI monitors
window moved across monitors
maximize
restore
fullscreen-like state
mini player
frameless titlebar
```

确保：

- 无错误缩放
- 无内容截断
- 无窗口位置丢失
- 无 off-screen window
- 无负坐标异常
- 无 restore crash

---

# 34. Installer Final Audit

最终 installer 必须测试：

```text
fresh install
repair
upgrade
downgrade attempt
reinstall
uninstall
user data preservation
silent failure
insufficient disk
permission error
locked files
running process
```

确认：

```text
installer does not package secrets
installer does not package developer data
installer does not package test music
installer does not package local databases
installer does not package SDK private content
```

---

# 35. Installer Signing

必须保持正式签名流程。

检查：

```text
Authenticode
installer signature
binary signature
timestamp
publisher identity
certificate validity
```

不得：

```text
self-signed certificate
production debug certificate
expired certificate
unsigned production executable
```

注意：

Windows SmartScreen 对新发布文件仍可能出现初始信誉警告，因此不得将“没有 SmartScreen 提示”伪装成 CI 自动 PASS。签名和长期发布者身份一致性应纳入 release policy。

---

# 36. Updater Final Audit

M11/M12 的 updater 是生产关键组件。

必须验证：

```text
new version available
same version
older version
malformed version
invalid signature
wrong signature
corrupted artifact
wrong platform
wrong architecture
missing manifest
timeout
network failure
download interruption
partial download
disk full
install failure
restart failure
```

必须保证：

```text
unsigned update rejected
invalid signature rejected
corrupt artifact rejected
wrong target rejected
downgrade rejected unless explicitly approved policy
```

Tauri updater 的更新签名验证属于强制安全机制；私钥绝不能进入仓库，且一旦遗失会影响后续给已有用户发布更新，因此 M14 必须把 updater signing key 当作最高级别 release secret 管理。

---

# 37. Updater Recovery

模拟：

```text
application crashes during update
installer fails
download interrupted
machine reboot
disk fills
permission denied
old binary remains
new binary corrupted
```

必须得到：

```text
recoverable state
```

不能进入：

```text
permanent broken install
```

---

# 38. Rollback Strategy

建立：

```text
docs/ROLLBACK_RUNBOOK.md
```

定义：

```text
software rollback
Steam branch rollback
updater rollback
database migration rollback
cloud state rollback
```

明确：

```text
what can be automatically rolled back
what requires manual intervention
what cannot safely be rolled back
```

特别注意：

```text
database schema migration
```

因为软件 binary 可以回退，但数据库已经迁移后未必能直接被旧版本使用。

因此需要：

```text
forward-compatible strategy
OR
explicit migration barrier
```

---

# 39. Steam Production Release Validation

M13 已完成 Steam integration。

M14 不重新设计 Steam integration，而是做 production readiness。

验证：

```text
Steam client running
Steam client not running
Steam offline
Steam API initialization failure
Steam unavailable
Steam overlay unavailable
Steam achievements unavailable
Steam cloud unavailable
Steam beta branch
Steam default branch
```

任何情况下：

```text
local playback continues
local library continues
local Dreamscape continues
local Map continues
local SMTC continues
```

Steam 不是播放器核心。

Steam 官方文档说明 Beta branch 用于将特定 build 提供给测试用户，而 Default branch 是正式客户获得的默认版本；Default branch 的实际切换仍是 App Admin 中的人工发布动作，因此不能把“CI 上传成功”写成“production 已发布”。

---

# 40. Steam Branch Release Matrix

至少形成：

```text
dev
beta
default
```

定义：

```text
Build ID
Depot
Branch
Manifest
Commit
Release Notes
```

确保：

```text
beta != accidental production
production != accidental debug build
```

---

# 41. Steam Offline Independence

必须进行真实验证：

```text
launch Steam
launch ifuyo Aura
disconnect network
disconnect Steam
continue playback
```

验证：

```text
music playback
library
cache
analysis
Dreamscape
Map
Windows media integration
```

都不能因为 Steam 不可用而崩溃。

---

# 42. Steam Cloud Final Validation

只允许同步已经定义的轻量应用状态。

禁止：

```text
music files
audio cache
artwork cache
full SQLite DB
analysis blobs
render cache
logs
temporary files
```

如果 Cloud 关闭：

```text
application remains fully usable
```

---

# 43. Steam Achievement Final Audit

Achievement 必须：

```text
idempotent
offline tolerant
non-critical
non-blocking
```

不得：

```text
require daily login
force user action
block features
send behavior telemetry
```

Steam achievement failure 绝对不能影响播放。

---

# 44. Steam Store / Marketing Consistency Audit

最终检查：

```text
Store description
Screenshot
Trailer if any
Feature list
System requirements
Privacy statement
Supported formats
Supported platform
Steam capsule
library assets
brand naming
```

必须与真实产品一致。

禁止：

```text
fake feature
fake screenshot
fake benchmark
fake award
fake compatibility
fake Steam verification status
```

品牌统一：

```text
ifuyo Aura
```

不要在面向用户的地方单独使用：

```text
Aura
```

指代产品；必须使用完整名称 `ifuyo Aura`。

---

# 45. Steam Deck / Proton

M14 只负责：

```text
compatibility assessment
```

如果实际验证，则记录：

```text
TESTED
```

如果没有真实设备：

```text
NOT TESTED
```

禁止写：

```text
Verified
Playable
Works perfectly
```

除非有实际平台证据。

---

# 46. Crash / Failure Diagnostics

M14 不加入远程 telemetry。

可以增强：

```text
local diagnostics
local logs
local export diagnostics
```

但必须：

```text
redacted
bounded
rotated
```

敏感数据不得进入诊断：

```text
absolute private paths
credentials
tokens
SteamID unless strictly necessary
full URLs with secrets
private library metadata unless explicitly exported by user
```

---

# 47. Crash Reproduction Protocol

创建：

```text
docs/CRASH_TRIAGE.md
```

定义：

```text
Symptom
Reproduction
Environment
Version
Build
Logs
Last action
Relevant subsystem
Severity
Workaround
Fix
Regression test
```

禁止“修了就算”。

任何 P0 crash fix：

```text
must have regression test
```

---

# 48. User Support / Recovery UX

不需要后端客服系统。

应用内只需提供：

```text
Help
About
Diagnostics
Open Data Folder
Open Logs Folder
Reset Visual Settings
Reset Application Settings
Repair Library Index
Rebuild Analysis
Clear Rebuildable Cache
```

这些操作必须明确区分：

```text
SAFE
DESTRUCTIVE
REBUILDABLE
```

不能把：

```text
Clear cache
```

实现为：

```text
Delete user library
```

---

# 49. Safe Mode

M14 可实现一个轻量的：

```text
Safe Mode
```

目标：

当用户启动异常时，可以绕过高风险 UI subsystem：

```text
Dreamscape
Music Map
optional GPU features
```

但仍保留：

```text
Playback
Library
Settings
Diagnostics
```

Safe Mode 不允许绕过：

```text
security checks
signature checks
filesystem restrictions
updater verification
```

---

# 50. Recovery Mode

可选增加：

```text
Repair / Recovery
```

仅处理：

```text
rebuild indexes
clear rebuildable cache
reset UI layout
disable problematic rendering feature
restore safe settings
```

严禁：

```text
silent destructive reset
```

---

# 51. Resource Leak Testing

M14 必须增加：

```text
file descriptor
thread
worker
timer
event listener
GPU resource
WebGL resource
decoder handle
HTTP client
database connection
Steam callback
```

生命周期测试：

```text
create
use
close
reopen
repeat x100
```

目标：

```text
resource count returns to baseline
```

---

# 52. Repeated Open/Close Stress

对以下模块：

```text
Dreamscape
Music Map
Settings
Mini Player
Queue
Library
Audio Device
Steam subsystem
```

执行：

```text
open/close 100 times
```

观察：

```text
memory
threads
GPU
handles
event listeners
```

任何单调增长必须调查。

---

# 53. App Lifecycle Stress

测试：

```text
startup
minimize
restore
sleep
wake
lock
unlock
shutdown
restart
crash
relaunch
update
```

重点确保：

```text
formal shutdown
single instance
playback shutdown
Steam shutdown
SMTC shutdown
render cleanup
workers stop
database flush
cache state safe
```

---

# 54. Single Instance Final Validation

同时启动多个 ifuyo Aura：

```text
2 instances
5 instances
10 launch attempts
```

必须：

```text
exactly one authority instance
```

第二实例应该：

```text
focus/open existing instance
pass intended command
exit
```

不得：

```text
two audio outputs
two Steam owners
two SMTC authorities
two tray owners
```

---

# 55. Database Integrity

M14 必须验证：

```text
PRAGMA integrity_check
foreign key integrity
migration consistency
index consistency
FTS consistency
transaction behavior
```

模拟：

```text
process crash
unexpected shutdown
disk full
database lock
```

确认：

```text
no silent corruption
```

---

# 56. Backup Strategy

定义本地 backup 范围。

至少需要：

```text
library metadata
playlists
settings
important saved state
```

不需要：

```text
huge caches
downloaded music
render cache
temporary logs
```

明确：

```text
backup frequency
backup retention
backup location
restore mechanism
```

不得把备份自身变成无限增长。

---

# 57. Determinism Audit

重新审计：

```text
seed
random stream
analysis version
layout version
migration version
database version
cache key
content identity
```

同一输入应该得到：

```text
same deterministic output
```

对于明确允许 nondeterministic 的系统：

```text
document why
```

---

# 58. Content Identity Final Audit

确保：

```text
same file
=> same identity
```

文件内容变化：

```text
=> content identity changes
```

文件重命名：

```text
=> TrackId preserved
```

文件移动：

```text
=> TrackId preserved
```

文件替换：

```text
=> content identity changes
=> analysis invalidated appropriately
```

---

# 59. Release Artifact SBOM / Dependency Audit

最终 release 必须重新检查：

```text
Rust dependencies
Node dependencies
native binaries
DLL
runtime assets
Steam runtime redistributables
WebView2-related components
Three.js
Tauri
```

输出：

```text
dependency inventory
license inventory
security advisory status
```

不得出现：

```text
unknown binary
unknown DLL
untracked executable
unlicensed asset
```

---

# 60. Public Repository Final Audit

Repo 可以保持 Public。

但生产仓库不得包含：

```text
private keys
Steamworks private SDK content
signing certificates / PFX
updater private key
API secrets
CI secrets
real user music
real user library DB
personal absolute paths
private diagnostic dumps
developer machine dumps
test credentials
production credentials
private store assets under NDA
```

允许公开：

```text
Steam App ID
public configuration
source code
documentation
license files
public store assets
non-secret test fixtures
```

任何例外必须经过明确许可证和公开发布权审查。

---

# 61. Git Hygiene

检查：

```text
git status
git diff
git diff --cached
git ls-files
git check-ignore
```

再次扫描：

```text
*.db
*.sqlite
*.log
*.pem
*.key
*.pfx
*.crt
*.env
*.secret
steam_appid.txt
private SDK
local paths
user profile paths
downloads
test music
```

删除：

```text
temporary reports
local exports
debug screenshots
CI artifacts
crash dumps
```

除非这些文件明确属于 repository release asset。

---

# 62. License Regression

重新运行：

```text
cargo deny check
```

以及现有 license audit。

检查：

```text
new dependency
transitive dependency
native library
font
image
icon
shader
music sample
test fixture
Steam SDK content
```

许可证策略必须继续保持：

### Allowed

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

### Review

```text
dual license
custom license
unknown
NOASSERTION
```

### Block

```text
GPL
AGPL
LGPL
```

除非已经完成明确的法律/许可证审查并由项目 owner 明确批准。

---

# 63. Content Policy Regression

再次检查：

```text
YouTube
Spotify
music.163.com
y.qq.com
music.apple.com
```

必须继续：

```text
reject before network
request_count == 0
```

项目仍然不得变成：

```text
commercial streaming extractor
```

---

# 64. Privacy Final Audit

最终 Privacy Policy 必须能够回答：

```text
What data is stored?
Where?
Why?
How long?
Can user delete it?
Can user export it?
Does ifuyo receive it?
Does Steam receive it?
Does third-party source receive it?
Does analytics exist?
```

必须保持：

```text
No telemetry by default
No hidden analytics
No behavior profiling
No hidden cloud account
```

---

# 65. Offline Test

断网状态：

```text
Windows offline
Steam offline
Internet Radio unavailable
```

测试：

```text
local library
local music playback
playlist
queue
Dreamscape
Music Map
settings
analysis of local tracks
cache
```

本地播放器不得因为网络不存在而失去基本功能。

---

# 66. Fresh Machine Test

M14 必须尽量模拟陌生用户。

不要使用：

```text
developer machine state
existing database
existing caches
developer environment variables
developer certificates
local SDK assumptions
```

验证：

```text
fresh Windows profile
fresh installation
launch
scan music folder
play track
close
relaunch
update
```

所有缺失依赖都必须被明确发现。

---

# 67. Clean-Room Release Test

建立：

```text
release-candidate
```

使用最终 installer 安装。

不得直接：

```text
run from repo
```

而应该：

```text
install
launch installed binary
test installed product
```

---

# 68. Release Candidate Build

定义：

```text
RC-x
```

例如：

```text
1.0.0-rc.1
```

RC build 必须：

```text
reproducible
signed
audited
versioned
hash-recorded
```

---

# 69. Release Artifact Inventory

建立：

```text
docs/RELEASE_ARTIFACTS.md
```

记录：

```text
Artifact
Platform
Architecture
Version
SHA-256
Signature
Build ID
Commit
Branch
Timestamp
```

至少：

```text
installer
portable package if any
debug symbols if retained privately
update artifact
update manifest
Steam depot build
```

---

# 70. Reproducibility

尽量做到：

```text
same commit
same dependencies
same release config
same toolchain
```

产生：

```text
same logical artifact
```

如果某些 artifact 不可完全 reproducible：

```text
document why
```

但仍必须记录：

```text
hash
build environment
toolchain
commit
```

---

# 71. CI Final Pipeline

最终 CI 必须形成：

```text
PR
 ↓
Lint
 ↓
Typecheck
 ↓
Unit Tests
 ↓
Integration Tests
 ↓
Security
 ↓
License
 ↓
Build
 ↓
Artifact Audit
 ↓
Release Candidate
 ↓
Manual Approval
 ↓
Production Release
```

不得：

```text
PR automatically publish production
```

生产发布必须有明确 gate。

---

# 72. Final Required CI Checks

至少重新跑：

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

以及：

```text
real Tauri production build
installer build
installer installation test
updater validation
Steam artifact audit
Steam content preview
```

---

# 73. CI Truthfulness

禁止 fake pass。

例如：

```text
STEAM_PRODUCTION = PASS
```

除非真的完成 production validation。

真实人工动作必须写：

```text
MANUAL
NOT EXECUTED
BLOCKED
```

例如：

```text
Steam Store review = MANUAL
Production branch publish = MANUAL
Real hardware Bluetooth test = MANUAL
Code signing certificate issuance = MANUAL
```

---

# 74. Release Gate Matrix

建立：

```text
docs/RELEASE_GATE.md
```

至少：

| Gate | Result |
|---|---|
| Build | |
| Unit Tests | |
| Integration Tests | |
| Security | |
| License | |
| Installer | |
| Updater | |
| Migration | |
| Audio Soak | |
| Library Stress | |
| Dreamscape Stress | |
| Music Map Stress | |
| Windows Integration | |
| Steam Beta | |
| Privacy | |
| Public Repo Scan | |
| Artifact Hash | |
| Signing | |
| Documentation | |
| Manual Store Readiness | |

每项只允许：

```text
PASS
FAIL
BLOCKED
NOT TESTED
MANUAL
```

---

# 75. Final 1.0 Release Criteria

只有当：

```text
ALL P0 = 0
```

并且：

```text
P1 release blockers = 0
```

才允许：

```text
READY FOR 1.0
```

---

# 76. Known Issues Registry

创建：

```text
docs/KNOWN_ISSUES.md
```

每项：

```text
ID
Severity
Affected Version
Affected Platform
Description
Impact
Workaround
Status
```

不得隐藏已知问题。

---

# 77. Release Notes

创建：

```text
CHANGELOG.md
```

以及：

```text
docs/RELEASE_NOTES_1_0.md
```

Release notes 应区分：

```text
Added
Improved
Fixed
Compatibility
Known Issues
Privacy
System Requirements
```

不要夸大性能或兼容性。

---

# 78. User Migration Notice

如果存在数据迁移：

必须告诉用户：

```text
what changes
what stays
what can take time
what is backed up
what is rebuildable
```

不要出现：

```text
"Migration may take some time"
```

这种没有任何解释的空泛提示。

应尽可能告诉用户：

```text
what the software is doing
```

---

# 79. Operational Runbook

创建：

```text
docs/OPERATIONS_RUNBOOK.md
```

至少覆盖：

```text
Release
Hotfix
Rollback
Steam Beta
Steam Production
Updater Rollback
Broken Migration
Cache Corruption
Library Corruption
Analysis Failure
Dreamscape GPU failure
Music Map failure
Steam API failure
Windows integration failure
Signing failure
Security Incident
```

---

# 80. Hotfix Policy

定义：

```text
P0
P1
P2
```

对应：

```text
emergency hotfix
regular patch
future feature release
```

P0：

```text
minimum scope
fastest safe patch
no unrelated features
```

禁止：

```text
hotfix + feature bundle
```

---

# 81. Post-Release Version Policy

定义类似：

```text
1.0.0 = initial commercial release

1.0.x = bugfix / security / compatibility

1.x.0 = feature release
```

但不得在 M14 引入具体未来功能。

---

# 82. Backward Compatibility Policy

明确：

```text
new app version
old user data
old cache
old analysis
old saved views
old cloud state
```

处理规则。

必要时：

```text
version fences
```

例如：

```text
requires migration
requires rebuild
requires cache invalidation
requires user action
```

---

# 83. Feature Flags Final Audit

如果存在 feature flags：

必须检查：

```text
default value
release channel
persistence
migration
debug exposure
security exposure
```

禁止：

```text
hidden production feature
accidentally enabled debug feature
```

M14 结束后，删除已经没有意义的 temporary flags。

---

# 84. Settings Final Audit

检查：

```text
audio
output device
volume
crossfade
EQ
cache
language
theme
visual quality
Dreamscape
Map
startup
tray
Steam
privacy
updates
```

确保：

```text
setting schema versioned
invalid settings recover gracefully
missing fields use defaults
unknown fields ignored where safe
```

---

# 85. Accessibility Regression

验证：

```text
keyboard navigation
focus
focus visibility
tab order
tooltip
screen-readable labels
reduced motion
UI scale
high DPI
color-independent information
```

尤其检查：

```text
Now Playing
Queue
Library
Search
Settings
Mini Player
Map
Dreamscape controls
```

---

# 86. Localization Final Audit

确保：

```text
zh
en
```

至少不出现：

```text
missing key
raw translation key
broken interpolation
overflow
clipped text
format mismatch
```

对于 dynamic content：

```text
metadata
ICY title
source title
error message
diagnostics
```

必须避免错误把用户内容当翻译 key。

---

# 87. Error UX

最终错误信息必须做到：

```text
What happened
Why
Can retry?
Can recover?
What can user do?
```

例如不要只显示：

```text
Error 500
```

而应包含：

```text
Source unavailable
The server returned an error.
Try again later.
```

但不得泄露：

```text
internal stack
absolute path
secret
credential
private IP
```

---

# 88. Empty State / Degraded State Final Audit

必须检查：

```text
no library
no search result
no artwork
no embedding
no analysis
no Steam
no internet
no audio device
no GPU
Map empty
Dreamscape fallback
cache unavailable
database recovery
```

每一种状态都必须：

```text
intentional
understandable
recoverable
```

不能只是：

```text
blank screen
```

---

# 89. Broken GPU / WebGL Fallback

必须确保：

```text
Dreamscape unavailable
```

时：

```text
main player remains usable
```

以及：

```text
Music Map may fallback
```

不得：

```text
entire app unusable
```

---

# 90. Audio Device Failure

测试：

```text
device disappears
default device changes
Bluetooth disconnect
device reappears
device sample rate changes
```

要求：

```text
Playback Authority remains consistent
UI remains responsive
app can recover
```

---

# 91. Power Management

Windows 场景：

```text
sleep
hibernate
resume
screen lock
screen unlock
```

测试：

```text
audio
network stream
cache
Dreamscape
SMTC
Steam
database
```

不能：

```text
deadlock
infinite reconnect
corrupt state
```

---

# 92. Large Metadata / Large Artwork Protection

必须有 bounded limits：

```text
title length
artist length
album length
artwork dimensions
artwork compressed size
metadata field count
HTTP body size
```

超出限制必须：

```text
reject
truncate safely
or degrade gracefully
```

不能：

```text
unbounded allocation
```

---

# 93. Queue / Playlist Consistency

Stress：

```text
add 1,000 tracks
remove random tracks
reorder
shuffle
repeat
next
previous
duplicate
delete source file
re-scan
restart
```

检查：

```text
queue remains valid
TrackId remains valid
playback authority remains valid
```

---

# 94. State Recovery After Crash

模拟：

```text
kill process
terminate
crash
power-loss-like shutdown
```

恢复后验证：

```text
library
playlists
settings
queue
playback history
Dreamscape state
Music Map saved state
cache metadata
```

不要恢复明显无效的 transient state。

---

# 95. Privacy-Safe Diagnostics Export

如果存在：

```text
Export Diagnostics
```

则必须让用户知道：

```text
what is included
```

并尽量：

```text
redact
hash
remove
```

例如：

```text
C:\Users\Gray\Music
```

不能原样进入导出报告，除非用户明确选择完整路径诊断。

---

# 96. Final Public Documentation Set

M14 完成后，至少应有：

```text
README.md
ARCHITECTURE.md
SECURITY.md
PRIVACY.md
CONTENT_POLICY.md
SOURCE_POLICY.md
THIRD_PARTY_LICENSES.md
RELEASE_CHECKLIST.md
CHANGELOG.md

docs/
├── M14_FINAL_AUDIT.md
├── RELEASE_1_0_CONTRACT.md
├── RELEASE_GATE.md
├── RELEASE_ARTIFACTS.md
├── DATA_PRESERVATION.md
├── MIGRATION.md
├── ROLLBACK_RUNBOOK.md
├── CRASH_TRIAGE.md
├── PERFORMANCE_BASELINE.md
├── WINDOWS_COMPATIBILITY.md
├── OPERATIONS_RUNBOOK.md
├── STEAM.md
├── STEAM_RELEASE_CHECKLIST.md
├── STEAM_RELEASE_RUNBOOK.md
├── STEAM_DATA_BOUNDARY.md
└── STEAM_PRIVACY.md
```

如果项目已有同名文档：

```text
update
do not duplicate
```

---

# 97. README Final Rewrite

README 必须体现最终产品身份：

```text
ifuyo Aura
```

不是：

```text
prototype
demo
experimental
vibe-coded app
```

README 应至少说明：

```text
What it is
What it does
Supported platforms
Installation
Basic usage
Privacy
Source policy
License
Development
Build
Release
Known limitations
```

---

# 98. Architecture Final Review

创建：

```text
docs/ARCHITECTURE_FINAL_REVIEW.md
```

回答：

```text
What owns playback?
What owns library?
What owns analysis?
What owns cache?
What owns Dreamscape?
What owns Map?
What owns Steam?
What owns Windows integration?
What owns update?
What owns persistent state?
```

必须能明确指出：

```text
single authority
single responsibility
one-way dependency
```

---

# 99. Dependency Graph Audit

最终生成：

```text
docs/DEPENDENCY_BOUNDARY.md
```

至少确保：

```text
aura-core
```

不依赖：

```text
Tauri
React
Zustand
Steam
Windows UI
Three.js
network
filesystem UI layer
```

核心不应被平台污染。

---

# 100. No Hidden Backend

最终确认：

```text
ifuyo Aura
```

不隐式依赖：

```text
ifuyo backend
analytics server
auth server
recommendation server
telemetry endpoint
proxy server
```

如果某服务存在，必须明确：

```text
what
why
optional?
what data
fallback
```

---

# 101. Final Build Environment Documentation

记录：

```text
Node version
npm version
Rust version
Cargo version
Tauri CLI
Windows SDK if applicable
build tools
Steamworks SDK requirements
code signing tool
```

不得依赖：

```text
"works on my machine"
```

---

# 102. Release Candidate Test Script

创建：

```text
scripts/release-smoke-test.*
```

或等价方案。

至少自动化：

```text
install
launch
health check
version check
basic IPC
library access
playback initialization
shutdown
```

如平台允许，再加入：

```text
installer detection
update metadata validation
signature validation
```

---

# 103. Manual Release Smoke Test

最终人工测试必须按以下路径：

```text
Install
→ First Launch
→ Add Music Folder
→ Scan
→ Search
→ Play
→ Pause
→ Seek
→ Next
→ Queue
→ EQ
→ Crossfade
→ Artwork
→ Dreamscape
→ Music Map
→ Mini Player
→ Media Keys
→ Tray
→ Restart
→ Update
→ Restart
```

整个路径不能出现：

```text
crash
data loss
state corruption
```

---

# 104. 1.0 Golden Path

建立：

```text
docs/GOLDEN_PATH.md
```

定义普通用户第一天体验：

```text
Install
↓
Launch
↓
Choose music folder
↓
Library populated
↓
Play music
↓
Explore Dreamscape
↓
Open Music Map
↓
Close
↓
Return next day
```

验证这条路径：

```text
clear
stable
beautiful
non-intrusive
recoverable
```

不要添加：

```text
forced onboarding
account creation
daily check-in
notification spam
```

---

# 105. Long-Term Maintenance Model

M14 必须确定：

```text
Release
→ Maintenance
```

之后主要工作：

```text
bugfix
security
compatibility
performance
OS changes
Steam SDK maintenance
dependency upgrades
```

而不是持续堆大型系统。

---

# 106. SDK / Dependency Update Policy

未来升级：

```text
Tauri
Rust
React
Three.js
Steamworks SDK
Node
dependencies
```

都必须：

```text
isolated branch
baseline tests
security checks
license checks
performance checks
release candidate
```

不要直接升级 production dependency。

---

# 107. Steam SDK Update Policy

Steamworks SDK 后续升级必须：

```text
audit changelog
license
runtime changes
API changes
callback changes
build changes
```

然后：

```text
beta validation
```

禁止：

```text
blind SDK replacement
```

---

# 108. Security Incident Runbook

创建：

```text
docs/SECURITY_INCIDENT_RUNBOOK.md
```

流程：

```text
Detect
Contain
Assess
Patch
Rotate
Release
Notify
Document
```

尤其考虑：

```text
updater signing key compromise
Steam secret compromise
CI secret compromise
malicious dependency
published vulnerable binary
```

---

# 109. Signing Key Policy

必须明确：

```text
Where private keys live
Who can access
Backup
Rotation
Recovery
CI injection
Local release procedure
Emergency revocation plan
```

不得进入 Git。

---

# 110. Production Secret Audit

执行：

```text
repository scan
Git history scan
CI config scan
artifact scan
installer scan
Steam upload script scan
release notes scan
```

确保没有：

```text
private keys
tokens
passwords
CI credentials
store credentials
Steam credentials
certificate passwords
```

---

# 111. Historical Git Scan

不要只扫描当前 HEAD。

至少扫描：

```text
recent history
release tags
large blobs
deleted files
```

避免：

```text
secret exists only in old commit
```

---

# 112. Binary Inspection

对最终 installer / executable：

检查：

```text
embedded URLs
debug paths
developer usernames
test strings
feature flags
secret-like strings
private endpoints
```

如果发现：

```text
developer-only endpoint
```

必须移除或确认其无敏感性。

---

# 113. Network Endpoint Inventory

创建：

```text
docs/NETWORK_ENDPOINTS.md
```

列出：

```text
Steam
Radio Browser if used
user-selected URLs
updater endpoint
any other endpoint
```

任何未知 endpoint：

```text
BLOCK
```

---

# 114. Final Privacy Network Verification

运行 release candidate。

记录 outbound network behavior。

特别确认：

```text
startup
idle
library scan
play local file
Dreamscape
Music Map
exit
```

不会偷偷发起未知请求。

---

# 115. Telemetry Verification

确认不存在：

```text
analytics SDK
crash reporting SaaS
usage telemetry
hidden ping
tracking pixel
remote metrics
```

除非项目边界后来明确修改，但 M14 默认：

```text
NO TELEMETRY
```

---

# 116. Memory Regression

至少：

```text
startup
idle
play
Dreamscape
Map
open/close
2h soak
8h soak
```

记录：

```text
working set
private bytes
GPU memory if measurable
```

重点不是追求“绝对小”，而是：

```text
stable
bounded
predictable
```

---

# 117. CPU Regression

分别测：

```text
idle
local playback
radio playback
Dreamscape
Map
analysis worker
library scan
```

确保：

```text
non-active subsystem does not remain at full activity
```

---

# 118. Startup Regression

建立：

```text
cold start
warm start
startup with 0 tracks
startup with 10k tracks
startup after crash recovery
startup after migration
startup with Steam unavailable
startup with GPU fallback
```

观察：

```text
time to interactive
time to playback
UI responsiveness
```

---

# 119. Background Task Scheduler

最终检查：

```text
analysis worker
library watcher
cache maintenance
artwork cleanup
Steam callbacks
```

不能：

```text
busy loop
run indefinitely
continue after shutdown
```

必须有：

```text
cancellation
shutdown
backpressure
bounds
```

---

# 120. Final Concurrency Audit

重点模块：

```text
Audio
Library
Cache
Analysis
Dreamscape
Map
Steam
Updater
```

检查：

```text
race
deadlock
starvation
duplicate work
stale state
cancellation race
shutdown race
```

---

# 121. Test Flakiness Audit

运行：

```text
full test suite x3
```

必要时：

```text
stress tests x10
```

将：

```text
flaky
timing-sensitive
network-dependent
machine-dependent
```

测试单独标记。

禁止：

```text
retry until green
```

来掩盖真实问题。

---

# 122. Test Isolation

所有测试：

```text
temporary directory
temporary database
test cache
test configuration
mock external services
```

不得污染：

```text
developer library
real user data
real credentials
real Steam production state
```

---

# 123. Release Security Tests

在最终 RC 上重新运行：

```text
security tests
fuzz tests where available
dependency audit
artifact scan
secret scan
license scan
```

结果必须保存为 release evidence。

---

# 124. Final Release Evidence

建立：

```text
release-evidence/
```

或等价 CI artifact。

包含：

```text
test report
security report
license report
artifact hashes
build metadata
installer validation
migration validation
Steam beta validation
performance baseline
compatibility matrix
```

这些内容可以保存为 CI artifacts，不一定进入 Git。

---

# 125. Release Decision

最终不要写：

```text
Looks good
```

必须输出：

```text
RELEASE_STATUS =
READY
NOT_READY
BLOCKED
```

并给出：

```text
P0_COUNT
P1_COUNT
P2_COUNT
UNTESTED_COUNT
MANUAL_ACTION_COUNT
```

---

# 126. Final Audit Output Format

必须严格输出：

```text
M14-A = PASS / BLOCKED / FAIL

RELEASE_STATUS = READY / NOT_READY / BLOCKED

P0_COUNT = <n>
P1_COUNT = <n>
P2_COUNT = <n>

UNTESTED_COUNT = <n>
MANUAL_ACTION_COUNT = <n>

BUILD = PASS / FAIL
TESTS = PASS / FAIL
SECURITY = PASS / FAIL / BLOCKED
LICENSE = PASS / FAIL / BLOCKED
INSTALLER = PASS / FAIL / BLOCKED
UPDATER = PASS / FAIL / BLOCKED
MIGRATION = PASS / FAIL / BLOCKED
AUDIO_SOAK = PASS / FAIL / NOT_TESTED
WINDOWS_COMPATIBILITY = PASS / PARTIAL / NOT_TESTED
STEAM_BETA = PASS / BLOCKED / NOT_TESTED
PRODUCTION_ARTIFACT = READY / BLOCKED
PUBLIC_REPO_AUDIT = PASS / FAIL

MANUAL_ACTION_REQUIRED =
- ...
- ...
- ...

RELEASE_BLOCKERS =
- ...
- ...

KNOWN_LIMITATIONS =
- ...
- ...

COMMIT = <hash>
NEXT = MAINTENANCE
```

---

# 127. Git Commit Policy

实施过程中保持：

```text
small logical commits
```

至少建议：

```text
M14: final hardening
M14: release validation
M14: documentation
```

但如果修改量很小，可以合并为一个最终提交。

禁止：

```text
unrelated refactor
style-only churn
massive formatting change
dependency upgrade without necessity
```

---

# 128. Git Push Policy

M14 默认：

```text
COMMIT = YES
PUSH = NO
```

除非项目 owner 另行要求。

最终只提交。

不要自动：

```text
git push
```

---

# 129. M14 的“停止条件”

这是非常重要的一条。

M14 完成后：

> **不得主动设计 M15。**

除非后续发现：

```text
critical production defect
security issue
OS compatibility issue
Steam platform break
updater issue
data migration issue
```

否则项目进入：

```text
MAINTENANCE MODE
```

而不是：

```text
FEATURE FACTORY MODE
```

---

# 130. M14 不做的事情

明确禁止：

```text
M15
M16
M17
```

式连续堆叠系统。

尤其不要继续增加：

```text
online social
accounts
cloud music
lyrics ecosystem
AI recommendation
streaming extraction
workshop marketplace
plugin marketplace
chat
messaging
live service
notification system
daily engagement system
```

未来如果新增功能，必须作为独立产品决策重新立项，而不是顺手塞进 M14。

---

# 131. 最终产品哲学冻结

ifuyo Aura 的核心体验必须仍然是：

> **一个安静、漂亮、尊重用户数据、真正属于用户自己的桌面音乐播放器。**

因此：

```text
No forced login
No forced cloud
No forced telemetry
No forced interaction
No engagement tricks
No daily chores
No noisy notifications
```

Dreamscape：

```text
陪伴
```

Music Map：

```text
探索
```

Audio Engine：

```text
稳定
```

Library：

```text
可靠
```

Steam：

```text
分发与平台能力
```

而不是：

```text
改变产品核心
```

---

# 132. 最终工程原则

M14 完成后，项目必须满足：

```text
Core is boring
Platform is isolated
UI is expressive
Data is safe
Audio is deterministic
Network is bounded
Graphics are recoverable
Steam is optional
Updates are signed
Releases are reproducible
Diagnostics are private
Documentation is explicit
Failures are recoverable
```

---

# 133. 重要：不得假装完成人工动作

以下事情如果没有真实外部操作，不得标记 PASS：

```text
Steam Direct onboarding
Steam Store approval
Steam production publish
real production beta user validation
real code-signing account verification
real certificate issuance
real Steam Deck hardware validation
real Bluetooth hardware validation
real Windows hardware matrix
real SmartScreen reputation accumulation
```

必须标：

```text
MANUAL
NOT EXECUTED
BLOCKED
```

---

# 134. 重要：不得把“代码存在”当成“功能成立”

例如：

```text
Updater code exists
```

不等于：

```text
Updater PASS
```

只有实际测试：

```text
signed manifest
valid signature
corrupt artifact
wrong artifact
successful update
failed update
recovery
```

才能声明对应结论。

同理：

```text
Steam integration exists
```

不等于：

```text
Steam production PASS
```

---

# 135. 最终发布判断

只有以下条件同时成立：

```text
P0 = 0

P1 release blockers = 0

Build = PASS

Tests = PASS

Security = PASS

License = PASS

Installer = PASS

Updater = PASS

Migration = PASS

Core playback = PASS

Data preservation = PASS

Public repo audit = PASS
```

并且所有无法执行的外部动作明确列入：

```text
MANUAL_ACTION_REQUIRED
```

才允许：

```text
RELEASE_STATUS = READY
```

---

# 136. M14 最终交付物

最终必须至少留下：

```text
1. 完整测试结果
2. 完整安全审计结果
3. 完整许可证审计结果
4. Release Candidate
5. Installer
6. Updater metadata/artifact
7. SHA-256 hashes
8. Version manifest
9. Migration validation
10. Performance baseline
11. Windows compatibility matrix
12. Steam beta/build validation
13. Release runbook
14. Rollback runbook
15. Crash triage runbook
16. Final documentation
17. Known issues
18. CHANGELOG
19. Clean git history
20. Final release commit
```

---

# 137. 最终工程状态

当 M14 完成后，项目应从：

```text
Active Development
```

切换到：

```text
Commercial Maintenance
```

即：

```text
ifuyo Aura 1.0
```

成为稳定产品基线。

后续所有修改默认遵循：

```text
issue
→ reproduce
→ test
→ minimal fix
→ regression
→ release candidate
→ release
```

而不是：

```text
idea
→ code
→ feature
→ feature
→ feature
```

---

# 138. 执行顺序

严格按照：

```text
Phase A
Complete Audit
    ↓
Phase B
Release Blocker Fix
    ↓
Phase C
Data / Migration / Recovery
    ↓
Phase D
Audio / Library / Cache Stress
    ↓
Phase E
Dreamscape / Map Stability
    ↓
Phase F
Windows / Installer / Updater
    ↓
Phase G
Steam Production Readiness
    ↓
Phase H
Security / Privacy / License Regression
    ↓
Phase I
RC Build
    ↓
Phase J
Final Documentation
    ↓
Phase K
Final Audit
    ↓
Commit
    ↓
STOP
```

不得跳过 Phase A。

---

# 139. M14 的最终要求

这是一个：

> **Release Engineering + Reliability + Security + Recovery + Commercial Readiness**

任务，而不是：

> **Feature Development**

任何想新增功能的修改，都必须先问：

```text
Is this required to make 1.0 safe, stable, installable, upgradeable, recoverable, supportable, or compliant?
```

如果不是：

```text
DO NOT IMPLEMENT IN M14.
```

---

# 140. 开始执行

现在开始。

第一步：

**只做完整基线审计，不修改代码。**

审计完成后，输出：

```text
M14 BASELINE AUDIT

A. Repository
B. Architecture
C. Audio
D. Library
E. Sources
F. Cache
G. Analysis
H. Dreamscape
I. Music Map
J. Windows Integration
K. Steam
L. Installer
M. Updater
N. Security
O. Privacy
P. Licensing
Q. Performance
R. Tests
S. Documentation
T. Release Readiness

P0 =
P1 =
P2 =

M14-A = PASS / BLOCKED / FAIL
```

然后才允许开始实施。

---

# 141. 最终停止协议

完成 M14 后：

```text
DO NOT PUSH
DO NOT START M15
DO NOT ADD FUTURE FEATURES
DO NOT REFACTOR UNRELATED CODE
```

只允许输出：

```text
FINAL AUDIT
FINAL STATUS
KNOWN LIMITATIONS
MANUAL ACTION REQUIRED
COMMIT HASH
NEXT = MAINTENANCE
```

## 最终状态目标

```text
ifuyo Aura 1.0
Commercially Hardened
Release Ready
Maintainable
Recoverable
Privacy Respecting
Steam Distributed
Windows First
Local First
```