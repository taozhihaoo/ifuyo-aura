# ifuyo Aura — M12 安全、隐私、许可证与商业发布合规加固开发提示词

## 0. 任务身份

你现在负责的是：

> **ifuyo Aura / M12 — Security, Privacy, Licensing & Compliance Hardening**

这是一个已经完成 M0–M11 的商业化桌面音乐播放器项目。

当前产品：

- 产品名：**ifuyo Aura**
- Git 仓库：`ifuyo-aura`
- 品牌：`ifuyo · 伊芙游`
- 技术栈：
  - Tauri 2
  - Rust
  - React
  - TypeScript
  - Vite
  - Zustand
  - Three.js
- 目标平台：Windows 10 / Windows 11 x64
- 产品定位：Local-first desktop music player
- 默认无遥测
- 无账号系统
- 无云端音乐库
- 无支付
- 无社交系统
- 不做商业音乐平台抓取
- 不依赖第三方商业音乐账号
- 本地优先，网络功能仅服务于明确允许的来源与功能

M0–M11 已建立：

- 商业基础与品牌规范
- Audio Core
- Playback Quality
- Music Library
- Source System
- Persistent Cache
- Desktop UI
- Dreamscape
- Audio Analysis
- Music Map
- Windows Integration / SMTC / Tray / Single Instance
- Installer / Updater / Signing / Release Engineering

### M12 的核心目标

> **不新增大功能，而是证明 ifuyo Aura 可以在公开仓库、真实 Windows 用户环境以及潜在商业发行场景下，以明确、可审计、可恢复、可解释的方式处理安全、隐私、第三方依赖、许可证、用户数据和网络行为。**

---

# 1. 最高优先级原则

## 1.1 先审计，后修改

严格遵循：

```text
AUDIT
→ FACT / HYPOTHESIS / INFERENCE
→ P0 / P1
→ 修复
→ 回归验证
→ 安全复审
→ 合规复审
→ 最终发布审计
→ commit
```

禁止：

- 先改代码再找理由
- 为了让 CI 变绿而降低检查标准
- 把工具无法验证的项目标记为 PASS
- 用假数据模拟安全能力
- 删除测试来绕过失败
- 降级依赖来绕过漏洞
- 修改安全边界来绕过权限错误
- 用“开发环境正常”代替生产级验证

必须区分：

- `FACT`
- `HYPOTHESIS`
- `INFERENCE`

必须区分：

- `P0`
- `P1`

本阶段只允许实际修复：

- P0 Security / Privacy / Compliance
- 必要的 P1 Security / Privacy / Compliance
- 必要的发布卫生问题

不要顺便重构无关代码。

---

# 2. M12 明确范围

M12 必须覆盖以下领域：

```text
A. Threat Model
B. Tauri IPC / Capability / Permission Security
C. WebView / CSP / Frontend Security
D. Filesystem / Path / Archive / Import Security
E. Network / SSRF / Redirect / URL Security
F. Audio / Metadata / Artwork Untrusted Content Security
G. Cache / Library / Analysis Data Security
H. Secrets / Credentials / Key Handling
I. Updater / Installer / Signing Security Review
J. Privacy & Data Lifecycle
K. Dependency / Supply Chain Security
L. License / Attribution / Commercial Compatibility
M. Content / Source Policy
N. Logging / Diagnostics Privacy
O. Security Documentation / Disclosure
P. CI Security Gates
Q. Final Public-Repository Audit
```

---

# 3. 明确禁止进入 M12 的范围

M12 不得实现：

- Steam
- Steamworks
- Steam achievements
- Steam Cloud
- Workshop
- Steam overlay
- 用户账号
- 登录系统
- 支付系统
- 云同步
- 社交功能
- 在线音乐商店
- 商业音乐平台登录
- DRM
- YouTube 音频抓取
- Spotify 抓取
- 网易云音乐抓取
- QQ 音乐抓取
- Apple Music 抓取
- 其他绕过平台官方 API / 播放授权的音乐提取
- AI 云端分析
- Telemetry
- Advertising
- Analytics tracking
- Remote code execution
- 插件市场
- 用户脚本系统
- 浏览器扩展
- 任意远程 JavaScript 加载
- 自动下载运行未知可执行文件
- 自动执行外部命令

尤其注意：

> M12 的目标是让现有能力更安全，不是增加“更强大的网络能力”。

---

# 4. M12-A / M12-B 分级标准

## M12-A：必须完成

必须完成：

- Threat Model
- Security Audit
- IPC / Capability / Permission 审计
- CSP 审计
- 文件系统边界审计
- URL / Network / SSRF 审计
- 不可信音频 / metadata / artwork 审计
- secrets 审计
- logging / privacy 审计
- updater / installer / signing 安全审计
- 第三方依赖安全审计
- license inventory
- SBOM 或等价依赖清单
- 第三方许可证文件
- Privacy 文档
- Security 文档
- Content Policy
- Source / Attribution 文档
- CI security gates
- Git history secret scan
- frontend + Rust 安全测试
- release artifact 安全审计
- public repository hygiene
- 完整最终审计

## M12-B：增强项

可以实现：

- 更严格的 OS sandbox / capability segmentation
- 更细的权限拆分
- 更完善的 security diagnostics
- 更完善的 data export / delete UI
- 自动 SPDX inventory
- 自动 SBOM
- advanced dependency policy
- CodeQL
- Dependabot / dependency update automation
- 更强的 fuzz tests
- mutation testing for security boundaries
- Windows Defender / SmartScreen 发布检查文档
- reproducibility enhancements
- signed provenance metadata
- crash-safe privacy redaction improvements
- security regression corpus

如果 B 项过多，不得影响 A。

---

# 5. A — Threat Model

先建立：

```text
docs/SECURITY_THREAT_MODEL.md
```

必须至少定义以下资产：

- 本地音乐文件
- 用户生成的播放列表
- Library SQLite 数据库
- Analysis 数据
- Artwork
- Cache
- Dreamscape state
- Music Map state
- Settings
- Updater configuration
- Signing public key
- Signing private key
- GitHub release credentials
- build environment
- installer artifact
- update artifact
- IPC commands
- filesystem access
- network access

必须识别攻击面：

```text
WebView
↓
Tauri IPC
↓
Rust Commands
↓
Filesystem
↓
SQLite
↓
Network
↓
Cache
↓
Audio decoder
↓
Metadata parser
↓
Artwork
↓
Updater
↓
Windows integration
```

至少考虑：

- malicious music file
- malicious metadata
- malformed artwork
- path traversal
- symbolic-link abuse
- junction abuse
- UNC path abuse
- oversized file
- decompression bomb
- malformed archive
- malicious URL
- SSRF
- localhost access
- private-network access
- redirect abuse
- credential leakage
- secret leakage
- IPC privilege escalation
- WebView XSS
- unsafe HTML / BBCode rendering
- script injection
- shell execution
- command injection
- SQL injection
- resource exhaustion
- disk exhaustion
- memory exhaustion
- CPU exhaustion
- poisoned cache
- stale update metadata
- downgrade attempt
- unsigned artifact
- wrong artifact
- tampered artifact
- replayed update
- malicious second instance
- log leakage
- telemetry leakage
- third-party dependency compromise

必须写出：

```text
Asset
Threat
Trust Boundary
Attack Vector
Existing Mitigation
Residual Risk
Test
Status
```

---

# 6. B — Tauri IPC / Capability / Permission Security

Tauri 的核心信任边界必须重新审查。

原则：

> Frontend = untrusted-ish client  
> Rust = privileged application boundary

前端不得获得不必要的系统能力。

逐一审查所有：

- Tauri commands
- invoke handlers
- events
- capabilities
- permissions
- scopes
- plugins

建立：

```text
docs/IPC_SECURITY.md
```

至少记录：

```text
Command
Frontend Reachable?
Capability
Permission
Input Validation
Output Validation
Sensitive?
Filesystem?
Network?
Process?
Risk
```

要求：

### 6.1 最小权限

每个 command：

- 只拥有必要能力
- 只访问必要目录
- 只接受明确类型
- 不接受任意路径
- 不接受任意 URL
- 不接受任意 shell command
- 不将内部 Rust 对象直接暴露给 frontend

### 6.2 输入永远不可信

对所有 IPC 输入：

- 长度限制
- 数值范围限制
- enum validation
- path canonicalization
- URL parsing
- scheme validation
- Unicode normalization where needed
- overflow checks
- resource limit
- cancellation handling

### 6.3 禁止危险 IPC

除非 M12 审计确认存在必要场景，否则：

- arbitrary shell execution = forbidden
- arbitrary PowerShell execution = forbidden
- arbitrary CMD execution = forbidden
- arbitrary process spawn from frontend = forbidden
- arbitrary filesystem root access = forbidden

---

# 7. C — WebView / CSP / Frontend Security

重新审查：

```text
src-tauri/tauri.conf.*
frontend HTML
frontend JS / TS
Three.js rendering
HTML injection
markdown / BBCode / tooltip
artwork display
external links
```

## 7.1 CSP

要求：

- 使用尽可能严格的 CSP
- 禁止不必要的 remote script
- 禁止不必要的 remote stylesheet
- 禁止任意 script source
- 禁止第三方 CDN runtime dependency
- 禁止动态执行代码
- 禁止危险 `eval` 类机制

检查：

```text
script-src
style-src
img-src
connect-src
font-src
media-src
object-src
frame-src
worker-src
```

逐项给出理由。

任何必须放开的来源都要写明：

```text
Why required
Who controls host
What content is loaded
Can it be removed?
Risk
```

## 7.2 XSS / HTML injection

重点审计：

- music title
- artist
- album
- genre
- comments
- lyrics-related text if present
- radio station metadata
- ICY StreamTitle
- playlist names
- tags
- filenames
- artwork URLs
- error messages

例如：

```text
<img>
<script>
<a href="javascript:...">
<style>
onerror=
onclick=
data:text/html
javascript:
```

必须确保来自音乐文件或网络源的 metadata 永远不能执行前端代码。

---

# 8. D — Filesystem Security

ifuyo Aura 是 Local-first 软件，因此 filesystem 是 M12 的核心。

必须审查所有文件访问入口：

```text
Library scanner
Watcher
Artwork
Cache
Playlist import/export
Analysis
Dreamscape state
Music Map state
Settings
Logs
Updater
Installer
```

---

## 8.1 Path Traversal

禁止：

```text
../
..\ 
encoded traversal
double encoded traversal
UNC abuse
absolute path escape
```

必须测试：

```text
..
%2e%2e
%252e%252e
..\..
/../../
C:\Windows
\\server\share
\\?\C:\
```

---

## 8.2 Symlink / Junction

必须考虑 Windows：

- symbolic links
- junctions
- reparse points

要求：

> Library root 内文件不应该借助 link 逃逸到未经授权目录。

如果当前设计无法安全处理：

- 明确拒绝
- 明确记录原因
- 测试 rejection

不要静默跟随到任意位置。

---

# 9. E — Network / SSRF Security

复审 M4 + M5 + updater。

任何网络请求必须经过统一的 URL / request policy boundary。

允许：

```text
http
https
```

具体允许范围必须以 M4/M5 当前实现为基准重新核对。

默认禁止：

```text
file:
ftp:
ws:
wss:
javascript:
data:
blob:
chrome:
devtools:
```

以及其他非必要 scheme。

---

## 9.1 SSRF

必须测试：

```text
127.0.0.1
localhost
0.0.0.0
::1
169.254.169.254
10.0.0.0/8
172.16.0.0/12
192.168.0.0/16
link-local
IPv6 private ranges
```

还要考虑：

- DNS rebinding
- redirect to private IP
- redirect from public → private
- decimal IP
- hexadecimal IP
- IPv4-mapped IPv6
- localhost aliases

如果产品功能确实需要本地/局域网 radio：

- 必须定义明确例外
- 必须经过显式 policy
- 不得让 frontend 获得 arbitrary request capability

---

# 10. HTTP 安全边界

审查：

- redirect limit
- response size limit
- header size limit
- metadata size limit
- timeout
- connect timeout
- idle timeout
- cancellation
- retry
- backoff
- decompression
- content-type validation
- content-length handling
- chunk limits

绝对不能因为远程服务器：

```text
Content-Length: huge
```

就预分配无限内存或磁盘。

---

# 11. F — Audio / Metadata / Artwork Untrusted Content

音乐文件必须视为：

> **完全不可信输入。**

即使文件来自用户本地磁盘，也不能默认安全。

审计：

- FLAC
- MP3
- WAV
- OGG
- AAC
- radio stream data
- malformed containers
- oversized tags
- corrupted frame
- malicious metadata
- embedded artwork

重点：

### Metadata

对：

- title
- artist
- album
- genre
- comment
- composer
- grouping
- lyrics
- custom tags

设置合理长度上限。

不能因为一个 tag 写了数 MB / 数十 MB 就让 UI / SQLite / IPC 无限膨胀。

### Artwork

必须限制：

- file size
- decoded pixel count
- image dimensions
- format
- recursion / nested image behavior where applicable

防止：

```text
10 x 10 image metadata says 200000 x 200000
```

造成资源耗尽。

### Decoder

确认：

- panic safety
- error handling
- cancellation
- resource bounds
- malformed file isolation

单个坏文件不得：

- crash whole app
- hang scanner forever
- break Library transaction
- break analysis scheduler
- poison playback engine

---

# 12. G — SQLite / Library / Cache / Analysis Security

审计：

```text
Library DB
Cache DB
Analysis records
Map derived store
Dreamscape state
```

必须确认：

- parameterized SQL
- no string concatenated SQL
- transaction boundaries
- migration safety
- corruption recovery
- schema validation
- BLOB size limits
- text size limits
- no arbitrary SQL IPC
- database path is controlled
- no path traversal to database

---

# 13. Cache Security

M5 已有 Persistent Cache。

M12 必须验证缓存不能成为：

- arbitrary file write primitive
- path traversal primitive
- arbitrary executable store
- unbounded disk growth primitive
- cross-user shared cache
- poisoned content source

重点验证：

```text
cache key
content key
chunk key
destination path
HTTP URL
ETag
Last-Modified
Content-Range
```

任何 cache filename/path：

> 必须由受控 deterministic key 生成，而不是直接信任 URL/path。

---

# 14. H — Secret / Credential Security

必须重新扫描：

```text
repository
working tree
Git history
GitHub Actions
.env*
config
logs
tests
fixtures
docs
screenshots
release artifacts
```

禁止存在：

- updater private key
- Windows code-signing private key
- GitHub PAT
- API key
- cloud credential
- test credential
- password
- refresh token
- bearer token
- accidental local paths containing secrets

特别检查：

```text
src-tauri/
.github/
scripts/
docs/
dist/
target/
release/
```

---

# 15. Signing Key Policy

确认：

## Updater signing private key

永远不能：

- commit
- ship
- embed frontend
- embed installer
- print to logs
- write to diagnostics
- include in release artifacts

只能通过：

- protected environment
- local secure storage
- CI secret

使用。

Tauri updater 要求签名验证，并区分公钥与私钥；私钥必须保持机密。

## Windows Authenticode

必须与：

```text
Tauri updater signing
```

分开描述。

文档明确区分：

```text
Update artifact signing
≠
Windows executable code signing
```

---

# 16. I — Updater / Installer Security Review

M11 已经建立 updater / installer。

M12 不重新实现，只做安全复审。

检查：

- HTTPS
- endpoint validation
- signature validation
- artifact hash
- version comparison
- downgrade protection
- wrong-platform rejection
- wrong-architecture rejection
- malformed manifest handling
- malformed signature handling
- network timeout
- retry limits
- disk exhaustion
- interrupted download
- interrupted install
- stale metadata
- malicious metadata
- rollback semantics
- restart semantics

必须确认：

> App 不得因为远程返回错误数据就执行任意程序。

不得允许：

```text
manifest URL → arbitrary executable
```

这种路径存在。

---

# 17. J — Privacy Model

建立：

```text
docs/PRIVACY.md
```

必须明确：

## 默认收集

原则：

```text
No telemetry
No analytics
No tracking
No account
No advertising profile
No cloud library
```

如果当前实现确实如此，必须从代码验证，而不是只写文档。

---

# 18. Data Classification

将所有数据分级：

### A — User Content

- local music path
- filename
- metadata
- artwork
- playlist
- play history
- play count
- skip count
- last played
- user tags

### B — Derived Data

- BPM
- Key
- LUFS
- MFCC
- embedding
- Music Map positions
- Dreamscape seed
- generated state

### C — App Configuration

- settings
- UI preferences
- audio settings
- cache settings

### D — Diagnostics

- error logs
- crash reports
- debug information

必须明确：

```text
Where stored
Why stored
Retention
User deletion
Export
Whether network transmitted
```

---

# 19. No-telemetry verification

不要只检查代码里有没有：

```text
telemetry
analytics
```

还要检查：

- npm dependencies
- Rust dependencies
- startup requests
- background requests
- updater requests
- radio discovery
- artwork retrieval
- error reporting
- crash reporting

区分：

```text
product-required network activity
```

与：

```text
tracking/telemetry
```

最终必须产生：

```text
docs/NETWORK_BEHAVIOR.md
```

记录：

```text
Feature
Network Required?
Destination Type
User Triggered?
Background?
Data Sent
Data Received
Purpose
```

---

# 20. Data deletion

必须确认用户能够合理删除：

- library database
- cache
- analysis
- map derived data
- Dreamscape state
- playback history
- settings

不得出现：

> UI 表示已删除，但磁盘仍残留大量用户数据。

如果某些数据由于 Windows / SQLite / filesystem 机制不能立即物理擦除：

- 必须文档说明
- 必须保证 logical deletion
- 不得虚假宣称 secure erase

---

# 21. Data export

至少评估：

```text
Library metadata
Playlists
Settings
Dreamscape seed/state
Map saved views if implemented
```

可以采用：

```text
JSON
CSV
M3U / M3U8
```

等标准或稳定格式。

注意：

- 不应默认导出本地绝对路径以外的敏感信息
- 不应导出 secret
- 不应导出内部 debug state
- 导出数据必须经过 schema validation

如果完整 export 尚未合理设计，可以作为 M12-B，但必须在审计报告中说明。

---

# 22. K — Logging / Diagnostics Privacy

所有日志重新审计。

禁止默认记录：

- access token
- password
- signing private key
- authorization header
- cookies
- arbitrary request body
- full sensitive URL
- embedded credentials
- full local paths where unnecessary
- private metadata beyond diagnostic need

对于 URL：

```text
https://user:password@example.com/a
```

必须脱敏。

对于 filesystem：

如果只是 diagnostic：

```text
C:\Users\<name>\Music\foo.mp3
```

考虑：

```text
<USER_MUSIC>\foo.mp3
```

或 equivalent safe redaction。

---

# 23. L — Third-party Dependency Security

重新审计全部：

### npm

检查：

```text
package-lock
dependency tree
direct dependencies
transitive dependencies
optional dependencies
postinstall scripts
```

### Cargo

检查：

```text
Cargo.lock
workspace dependencies
build scripts
proc macros
unsafe crates
native dependencies
```

必须识别：

```text
dependency
version
purpose
license
source
risk
```

---

# 24. Supply Chain

建立：

```text
docs/SUPPLY_CHAIN.md
```

至少检查：

- lockfiles committed
- deterministic install
- dependency pinning strategy
- build toolchain version
- Rust toolchain
- Node version
- package manager version
- Tauri CLI version
- native Windows build dependencies
- GitHub Actions action versions
- action pinning policy

GitHub 侧建议检查：

- Secret Scanning
- Push Protection（若当前仓库/账户能力可用）
- Dependency graph
- Dependabot alerts
- Code scanning / CodeQL（若当前能力可用）

不要因为某个 GitHub Security 功能不可用就虚假标记 PASS；必须写：

```text
AVAILABLE
ENABLED
NOT AVAILABLE
MANUAL ACTION REQUIRED
```

---

# 25. M — License Compliance

这是 M12 的正式 A-grade 项目。

当前 ifuyo Aura 商业兼容许可证策略：

允许：

```text
MIT
Apache-2.0
BSD-2-Clause
BSD-3-Clause
ISC
MPL-2.0
Zlib
Unicode-3.0
CDLA-Permissive-2.0
OFL-1.1
CC0
```

默认禁止：

```text
GPL
AGPL
LGPL
以及未经人工确认的强 copyleft / source-disclosure obligations
```

注意：

> 不允许 AI “猜许可证”。

必须以：

- upstream LICENSE
- package metadata
- Cargo metadata
- npm metadata
- SPDX data
- official repository

等真实来源核实。

---

# 26. License Inventory

生成或更新：

```text
docs/THIRD_PARTY_LICENSES.md
```

至少包含：

```text
Package
Version
Direct / Transitive
License
SPDX Identifier
Purpose
Runtime / Build-only
Commercial Distribution Impact
Action
```

并建立 machine-readable inventory，例如：

```text
licenses.json
licenses.lock.json
sbom.spdx.json
```

任选合理方案，但必须：

- 可复现
- 可重新生成
- 不手工伪造结果

---

# 27. License 异常处理

遇到：

```text
Unknown
NOASSERTION
custom license
dual license
multiple license
runtime asset license
font license
model license
```

不得自动判定为“允许”。

应进入：

```text
REVIEW_REQUIRED
```

并形成清单。

---

# 28. Assets License

除了 npm / Cargo，还必须检查：

- fonts
- icons
- textures
- images
- music samples
- UI assets
- generated assets
- test assets
- model weights
- shader snippets
- copied code
- snippets from external repositories

虽然 ifuyo Aura 尽量使用纯代码与本地资源，但实际仓库必须逐项确认。

禁止：

> “这是网上常见素材，应该没问题。”

---

# 29. Content / Source Policy

建立或复审：

```text
docs/CONTENT_POLICY.md
docs/SOURCE_POLICY.md
```

必须明确：

### Local

用户自己的本地音乐：

```text
Allowed
```

### Direct HTTP/HTTPS

只允许合理来源和用户请求场景。

### Internet Radio / Radio Browser

允许：

- station discovery
- stream metadata
- station information

但：

- 不得声称拥有第三方音乐内容
- 不得默认永久缓存受限制内容
- 必须尊重 M4/M5 cache policy

### Commercial music platforms

必须明确：

```text
Not supported
```

包括：

- YouTube
- Spotify
- 网易云音乐
- QQ 音乐
- Apple Music
- 其他需要绕过官方访问控制才能取得音频数据的平台

尤其需要验证：

> unsupported platform detection 必须发生在网络请求之前。

测试要求继续保持：

```text
request_count == 0
```

---

# 30. M5 Cache + Content Policy 一致性

M12 不改变 M5 设计，但必须进行策略一致性审计。

必须确认：

```text
Forbidden
SessionOnly
Allowed
```

在：

- source
- playback
- cache
- UI
- diagnostics
- persistence

各层意义一致。

禁止出现：

```text
Source says Forbidden
↓
Cache still writes
```

这种违规。

必须有自动化测试覆盖。

---

# 31. M8 Analysis Privacy

M8 的：

- embeddings
- MFCC
- BPM
- Key
- LUFS
- derived metrics

均属于本地派生数据。

必须确认：

- 不发送到云端
- 不自动上传
- 不包含秘密
- 不因 analysis scheduler 产生额外网络请求
- embedding model 不偷偷下载

如果未来需要下载模型：

> 不属于 M12 默认行为，必须单独设计、明确告知用户并经过新的安全/许可证审计。

---

# 32. M7 / M9 Privacy Review

Dreamscape 和 Music Map：

必须确认：

- seed 不包含私人信息
- map positions 不上传
- artwork 不上传
- analysis 不上传
- world state 纯本地
- saved views 纯本地
- 无 hidden sync

如 seed 使用用户数据：

必须说明：

```text
Input
Normalization
Hash
Storage
Privacy implication
```

不得把完整本地路径直接作为公开 seed。

---

# 33. Three.js / WebGL Security

检查：

- external texture URLs
- external model URLs
- remote shader content
- dynamic script loading
- unsafe HTML labels
- malformed image input
- huge texture dimensions
- WebGL context exhaustion

Dreamscape / Music Map：

> 默认只能加载受控的本地应用资源与用户允许的数据，不得把远程不可信内容直接当作 executable application content。

---

# 34. Windows-specific Security

重点审查：

- filesystem ACL assumptions
- AppData location
- LocalAppData
- Roaming data if used
- startup behavior
- tray
- single instance
- SMTC
- media keys
- URI handlers if any
- installer elevation
- uninstall
- updater elevation
- Windows path edge cases
- Unicode path
- long path
- reserved names
- UNC path
- reparse points

确认：

> 不需要管理员权限的操作不要强制管理员权限。

确认 installer / updater：

- 不会执行任意来自用户音乐目录的程序
- 不会把音乐目录加入 PATH
- 不会修改系统级配置以实现普通播放功能
- 不会注册无必要的协议处理器

---

# 35. Crash / Panic Safety

Rust：

- audit `unwrap`
- `expect`
- `panic!`
- `unsafe`
- FFI boundaries

重点不是机械删除所有 `unwrap`，而是判断：

```text
Can this input be attacker-controlled?
Can this fail in normal user environment?
Does panic kill whole app?
Is recovery possible?
```

Audio callback 尤其继续保持 M1/M2 的：

```text
no panic
no allocation
no blocking
no logging
no filesystem
no network
no decoder I/O
```

M12 不得破坏该约束。

---

# 36. Frontend Runtime Security

搜索：

```text
eval(
new Function(
innerHTML
dangerouslySetInnerHTML
document.write
window.open
location =
iframe
script injection
dynamic import from user content
```

每一项：

```text
Occurrence
Reason
Safe?
Mitigation
```

必须真实检查。

---

# 37. External Link Handling

如果 UI 支持：

- documentation link
- station website
- source URL
- project homepage

必须审查：

```text
https://
http://
```

与危险协议。

禁止将：

```text
javascript:
data:
file:
vbscript:
```

直接传给 shell / browser launch API。

外部链接必须：

- scheme validated
- normalized
- user initiated
- never auto-opened

---

# 38. Security Test Suite

新增独立 security test suite，例如：

```text
tests/security/
```

至少包含：

### IPC

- unauthorized command
- invalid enum
- invalid path
- invalid URL
- oversized input
- integer overflow
- malformed JSON

### Filesystem

- traversal
- encoded traversal
- absolute path escape
- UNC
- junction
- symlink/reparse
- invalid filename
- very long path

### Network

- localhost
- private IP
- metadata address
- redirect to private IP
- redirect chain
- invalid scheme
- malformed URL
- credential URL

### Metadata

- huge tag
- malformed tag
- Unicode
- HTML injection
- script payload
- control chars

### Artwork

- huge dimensions
- huge file
- malformed image
- unsupported format
- malicious filename

### Cache

- path escape
- oversized chunk
- malformed cache metadata
- collision
- corruption

### SQLite

- malformed migration
- invalid input
- oversized data
- transaction rollback

### Updater

- malformed manifest
- invalid signature
- wrong version
- downgrade
- wrong arch
- wrong platform
- corrupted artifact
- interrupted update

### Privacy

- logs contain no secret
- no credentials persisted
- no unexpected network
- no telemetry endpoint
- deletion removes expected data

---

# 39. Fuzzing

至少为高风险 parser 建立 fuzz strategy：

```text
URL parser
metadata parser
ICY parser
playlist parser
cache manifest
updater manifest
IPC payload
```

可以使用：

- cargo-fuzz
- property-based testing
- corpus based fuzzing

不要求 M12 一次完成大规模持续 fuzzing，但至少：

- 明确 fuzz targets
- 提供 seed corpus
- 建立最基本 smoke run
- 文档说明如何运行

---

# 40. Dependency Vulnerability Testing

执行真实工具。

根据实际项目工具链选择：

Rust：

```powershell
cargo audit
cargo deny check advisories
```

Frontend：

```powershell
npm audit
```

但注意：

> 不允许单纯为了“0 vulnerabilities”而自动升级所有依赖。

必须分类：

```text
Known vulnerability
Reachable?
Production?
Dev-only?
Exploitability
Fixed version
Breaking change?
Decision
```

---

# 41. CI Security Gate

M12 必须把安全检查纳入 GitHub Actions。

至少考虑：

```text
dependency audit
license check
secret scan
frontend test
Rust test
static analysis
```

对于 GitHub Actions 本身：

- 不使用未固定的危险脚本来源
- 不把 secrets 暴露给 fork PR
- 不打印 secrets
- 最小权限 `permissions`
- release job 与普通 CI 分离
- signing job 使用 protected secrets
- 非 release PR 不接触生产签名密钥

---

# 42. GitHub Repository Security

检查并记录：

```text
Secret scanning
Push protection
Dependency graph
Dependabot
CodeQL / Code scanning
Actions permissions
Environment protection
Release permissions
Branch protection if available
```

GitHub 文档目前明确提供 Secret Scanning、Push Protection、Code Scanning 等安全能力；公开仓库还会提供 dependency graph 等基础安全信息。

不要假定所有能力一定在当前账户配置中可用。

实际检查后记录：

```text
Enabled
Disabled
Unavailable
Manual configuration required
```

---

# 43. Repository Secret History Audit

必须检查：

```text
git log
git show
git rev-list --all
```

以及必要的专用 secret scanner。

重点：

- 私钥
- PAT
- API keys
- tokens
- passwords
- credentials

如果历史曾经泄露：

> 不得简单删除当前文件就宣布 PASS。

必须：

1. 识别 secret
2. 判断是否真实
3. 判断是否仍有效
4. revoke / rotate
5. 判断是否需要 history rewrite
6. 记录 remediation

如果没有能力安全 rewrite history：

必须明确：

```text
BLOCKED / MANUAL ACTION REQUIRED
```

---

# 44. Release Artifact Security Audit

M11 已有 release pipeline。

M12 再检查最终 artifact：

```text
.exe
.msi
.nsis
updater artifact
.sig
manifest
source archive
SBOM
license files
```

必须确认：

- 没有 private key
- 没有 `.env`
- 没有 debug secrets
- 没有本地绝对路径
- 没有 test database
- 没有 user data
- 没有 development certificates
- 没有 temporary files
- 没有 unintended source dumps

---

# 45. Binary Hygiene

检查最终 binary：

- strings / metadata
- debug symbols
- absolute build paths
- URLs
- internal hostnames
- test credentials
- hidden feature flags
- internal project names if they should not ship

`Aura` 已随章程变更并入正式产品名称 `ifuyo Aura`，不再是工程 codename。

其他 internal project names 仍不得在用户-facing branding 中取代正式产品名称。

但不需要为了隐藏它们而做不必要的 binary obfuscation。

---

# 46. Privacy-preserving Diagnostics

建立：

```text
docs/DIAGNOSTICS.md
```

定义：

```text
What is logged
What is not logged
How to enable verbose logs
How to redact logs
How to export diagnostics
```

默认：

```text
No telemetry
Local only
User initiated
```

---

# 47. Security Documentation

至少形成：

```text
SECURITY.md
PRIVACY.md
CONTENT_POLICY.md
SOURCE_POLICY.md
SECURITY_THREAT_MODEL.md
IPC_SECURITY.md
NETWORK_BEHAVIOR.md
SUPPLY_CHAIN.md
DIAGNOSTICS.md
THIRD_PARTY_LICENSES.md
```

必要时：

```text
SBOM.md
SECURITY_ADVISORY_PROCESS.md
```

---

# 48. SECURITY.md

必须包含：

```text
Supported versions
How to report vulnerability
What information to provide
What not to disclose publicly first
Expected acknowledgement process
Security scope
Out of scope
```

不要写无法履行的时间承诺。

---

# 49. Privacy 文案要求

所有 Privacy 文档必须：

- 简明
- 准确
- 基于真实实现
- 不夸张
- 不宣称不存在的安全保护
- 不宣称“绝对安全”
- 不宣称“完全匿名”
- 不宣称“绝不联网”

应该根据实际代码明确描述：

```text
Local music scanning
Radio
Radio Browser
HTTP streams
Artwork
Updater
Release infrastructure
```

哪些行为会联网。

---

# 50. Security Boundaries 必须形成架构图

更新：

```text
docs/ARCHITECTURE.md
```

至少包含：

```text
React / WebView
       │
       │ Typed IPC
       ▼
Tauri Commands
       │
       ├── Library
       ├── Playback
       ├── Sources
       ├── Cache
       ├── Analysis
       ├── Dreamscape
       ├── Music Map
       ├── Windows Integration
       └── Updater
```

标记：

```text
UNTRUSTED
TRUSTED
PRIVILEGED
NETWORK
USER DATA
```

明确：

> Frontend 不拥有 application authority。

---

# 51. Playback Authority 不得被破坏

M10 已明确：

> Playback Authority 是播放状态唯一真实来源。

M12 安全修改不得改变：

```text
Playback Authority
```

SMTC、Tray、Frontend、Media Keys、Updater、Dreamscape 等仍然：

```text
command / adapter / observer
```

不得变成第二套 playback authority。

---

# 52. Security 与 Audio callback 的隔离

安全代码不得侵入实时 callback。

不得在 callback 中加入：

- security logging
- filesystem check
- mutex
- async call
- network request
- dynamic allocation
- database query

安全审计必须证明：

> security hardening 没有降低 M1/M2 realtime guarantees。

---

# 53. Performance Regression

M12 需要重新跑关键性能测试：

```text
startup
library scan
search
playback
cache
analysis
Dreamscape
Music Map
SMTC
tray
updater check
```

尤其验证：

- security validation 不造成明显 playback jitter
- URL policy 不造成 callback blocking
- logging redaction 不造成实时路径锁竞争
- filesystem validation 不造成 UI freeze
- analysis security limits 不导致 scheduler starvation

---

# 54. Security Resource Limits

统一检查：

```text
MAX_FILE_SIZE
MAX_METADATA_SIZE
MAX_ARTWORK_SIZE
MAX_ARTWORK_PIXELS
MAX_URL_LENGTH
MAX_HEADER_SIZE
MAX_REDIRECTS
MAX_RESPONSE_SIZE
MAX_CACHE_CHUNK
MAX_IPC_PAYLOAD
MAX_PLAYLIST_SIZE
MAX_ANALYSIS_MEMORY
```

不要随处写 magic numbers。

建议建立：

```text
SecurityLimits
```

或同等集中配置。

---

# 55. Error Handling

安全失败必须：

```text
Fail closed
```

例如：

- invalid signature → reject
- invalid URL → reject
- unauthorized IPC → reject
- path escape → reject
- malformed metadata → reject item
- corrupt cache → invalidate
- corrupted DB → recover / fail safely
- unknown license → review
- missing model → explicit unavailable

禁止：

```text
invalid → silently fallback to unsafe
```

---

# 56. Security Event Logging

可以记录安全事件，但：

> 不能记录秘密本身。

例如：

```text
Blocked invalid URL scheme
Rejected path traversal
Updater signature mismatch
IPC permission denied
Invalid artifact version
```

应该记录：

- event type
- reason
- subsystem
- timestamp if already supported

而不是：

- token
- password
- private key
- complete credential URL

---

# 57. Security Test Determinism

所有 security tests：

- deterministic
- no real public network
- no third-party dependency
- no external services
- no real credentials

网络类测试使用：

```text
local fake server
```

禁止 CI 依赖：

```text
example.com
github.com
Radio Browser real API
random public hosts
```

Updater release integration 可以访问 Release infrastructure 的受控环境，但普通 CI test 不应依赖公网。

---

# 58. License Testing in CI

如果能够自动化：

```text
cargo deny
license checker
npm license inventory
SPDX validation
```

则加入 CI。

当出现 disallowed license：

```text
CI FAIL
```

除非已记录人工豁免。

---

# 59. Dependency Allowlist Policy

不要只列“禁止 GPL”。

同时定义：

```text
Allowed
Review required
Blocked
```

例如：

```text
Allowed:
MIT
Apache-2.0
BSD
ISC
MPL-2.0
Zlib
Unicode-3.0
CDLA-Permissive-2.0
OFL-1.1
CC0

Review:
Dual-license
Custom
Unknown
NOASSERTION
model-specific license

Blocked:
GPL
AGPL
LGPL
```

具体结果必须以真实依赖树为准。

---

# 60. Security Regression Matrix

最终形成一张 matrix：

| Area | M4 | M5 | M6 | M7 | M8 | M9 | M10 | M11 | M12 |
|---|---|---|---|---|---|---|---|---|---|
| Network | ✓ | ✓ | | | | | | ✓ | ✓ |
| Filesystem | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| IPC | | | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Privacy | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Signing | | | | | | | | ✓ | ✓ |
| License | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Secrets | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

只允许填入真实验证结果。

---

# 61. Manual Security Checklist

必须提供：

```text
docs/M12_SECURITY_CHECKLIST.md
```

至少：

### Before Release

- [ ] no secrets
- [ ] no private signing key
- [ ] updater public key matches production key
- [ ] installer identity correct
- [ ] licenses verified
- [ ] SBOM generated
- [ ] CSP reviewed
- [ ] capabilities reviewed
- [ ] IPC reviewed
- [ ] SSRF reviewed
- [ ] path traversal reviewed
- [ ] metadata injection reviewed
- [ ] artwork limits reviewed
- [ ] cache boundaries reviewed
- [ ] updater verified
- [ ] privacy verified
- [ ] public repo checked
- [ ] release artifacts scanned

---

# 62. Security Review of M11 Secrets

M11 中所有：

- GitHub Actions
- Tauri updater private key
- code signing credentials
- release tokens

必须重新检查权限模型。

要求：

```text
CI test job
≠
release job
≠
signing job
```

不要让普通 PR job 访问：

```text
production signing secrets
```

---

# 63. No Secret in Frontend

检查：

```text
import.meta.env
VITE_*
window.*
localStorage
sessionStorage
IndexedDB
frontend bundle
```

绝对不能存在：

- updater private key
- GitHub token
- signing credential
- server secret

Public configuration 可以存在。

Secret 不行。

---

# 64. Security Boundary for Updater

Frontend：

```text
request update
```

Rust：

```text
validate policy
→ check endpoint/config
→ invoke updater
→ validate update
→ execute signed update flow
```

Frontend 不应决定：

```text
arbitrary update binary path
arbitrary executable path
arbitrary signature bypass
```

---

# 65. Security Boundary for External Sources

Frontend：

```text
request source
```

Application layer：

```text
validate source policy
→ validate URL
→ validate capabilities
→ apply network restrictions
→ create source
```

Frontend 不应：

```text
raw fetch arbitrary URL
```

尤其不能绕过 M4/M5 网络 policy。

---

# 66. M12 不得引入 Architecture Drift

禁止产生：

```text
security_utils_everywhere
```

或：

```text
random validation copied in 30 places
```

应尽量形成：

```text
security policy boundary
filesystem policy boundary
network policy boundary
content validation boundary
```

但不要为了“架构漂亮”而大型重构。

原则：

> Centralize security policy, localize domain behavior.

---

# 67. Documentation Synchronization

所有代码行为变化必须同步：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
SECURITY.md
PRIVACY.md
CONTENT_POLICY.md
SOURCE_POLICY.md
THIRD_PARTY_LICENSES.md
PROGRESS.md
CHANGELOG.md
```

必要时：

```text
PERF.md
RELEASE_CHECKLIST.md
```

不能只改代码不更新文档。

---

# 68. Decision Records

对于重要安全决策新增 ADR，例如：

```text
ADR-0XX: Tauri capability boundary
ADR-0XX: Filesystem sandbox policy
ADR-0XX: Network SSRF policy
ADR-0XX: License compatibility policy
ADR-0XX: Privacy and telemetry policy
ADR-0XX: Security logging policy
```

编号按项目现有规则递增。

---

# 69. M12 Implementation Order

严格按照：

```text
Step 1
Baseline Audit

Step 2
Threat Model

Step 3
Tauri IPC / Capability / CSP

Step 4
Filesystem / Network

Step 5
Audio / Metadata / Artwork

Step 6
Database / Cache / Analysis

Step 7
Secrets / Updater / Installer

Step 8
Privacy

Step 9
License / SBOM / Supply Chain

Step 10
CI Security Gates

Step 11
Security Tests / Fuzz Smoke

Step 12
Documentation

Step 13
Full Regression

Step 14
Final Security Audit

Step 15
Final Diff Review

Step 16
Commit
```

---

# 70. Baseline Audit Output

首先输出：

```text
# M12 Baseline Audit

## A. Repository
## B. Frontend
## C. Rust
## D. Tauri
## E. IPC
## F. Capabilities
## G. CSP
## H. Filesystem
## I. Network
## J. Cache
## K. Database
## L. Analysis
## M. Dreamscape
## N. Music Map
## O. SMTC/Tray
## P. Installer
## Q. Updater
## R. Signing
## S. GitHub Actions
## T. Dependencies
## U. Licenses
## V. Secrets
## W. Privacy
## X. Documentation
```

所有项目写：

```text
FACT
STATUS
RISK
```

---

# 71. P0 修复纪律

P0 例如：

- secret committed
- private signing key exposed
- arbitrary command execution
- arbitrary file write
- arbitrary file read
- IPC privilege escalation
- updater signature bypass
- remote code execution path
- SSRF to local/private network where unintended
- path traversal escaping user root
- XSS leading to privileged IPC abuse
- GPL/AGPL/LGPL dependency accidentally shipped against explicit policy

发现 P0：

> 必须优先修复，不能继续堆 B 功能。

---

# 72. P1 示例

P1 可以包括：

- missing rate limit
- weak diagnostics redaction
- incomplete fuzz coverage
- weak path error handling
- insufficient metadata limits
- missing supply-chain automation
- incomplete privacy UX
- incomplete documentation
- non-blocking security quality issues

---

# 73. Required Commands

根据真实 package scripts 执行，不要虚构命令。

至少尝试：

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
npm audit
```

以及项目已有的：

```text
security scan
license scan
SBOM generation
Tauri build
installer smoke test
updater integration test
```

如果某命令不存在：

```text
NOT AVAILABLE
```

不要伪造成功。

---

# 74. Windows Validation

必须尽量在真实 Windows 环境执行。

至少验证：

- install
- launch
- update
- uninstall
- data preservation
- library access
- playback
- radio
- cache
- analysis
- Dreamscape
- Music Map
- SMTC
- tray
- single instance

以及安全边界：

- malicious filename
- malicious metadata
- traversal
- UNC
- junction / symlink where feasible
- malformed artwork
- malformed source
- invalid updater manifest
- invalid update signature

---

# 75. Public Repository Final Audit

在 commit 前再次扫描：

```text
git status
git diff
git diff --cached
git ls-files
git log
```

检查：

- secrets
- local paths
- personal information
- test credentials
- temporary reports
- binary garbage
- private certificates
- signing keys
- generated user data
- `.env`
- databases
- caches
- screenshots with sensitive information

确认 `.gitignore` 继续正确。

---

# 76. Release Artifact Audit

实际 build 后，对 artifact 做：

```text
file list
hash
signature
metadata
strings
secret scan
license verification
```

输出：

```text
Artifact
SHA-256
Size
Platform
Architecture
Signature
Contains Secret?
Contains User Data?
PASS/FAIL
```

---

# 77. Security Acceptance Criteria

以下全部必须：

### MUST PASS

- IPC boundary
- capability least privilege
- CSP
- filesystem safety
- network safety
- SSRF defense
- metadata handling
- artwork resource limits
- cache path safety
- SQLite parameterization
- updater signature validation
- signing secret protection
- no telemetry
- privacy documentation
- license inventory
- dependency audit
- public repository secret audit
- release artifact audit
- security regression tests

---

# 78. BLOCKED Criteria

以下任何情况：

```text
Cannot validate signing secret handling
Cannot test Windows updater
Cannot verify actual license
Cannot verify security boundary
Cannot perform native Windows integration
Cannot verify artifact contents
Cannot determine actual dependency license
```

都必须：

```text
BLOCKED
```

而不是：

```text
PASS
```

---

# 79. No Fake Compliance

绝对禁止：

```text
placeholder security scan
fake SBOM
fake license inventory
fake secret scan
hardcoded PASS
dummy signature
fake Windows validation
fake installer validation
fake update verification
fake privacy claim
```

如果只能做静态审计：

明确写：

```text
STATICALLY VERIFIED
```

如果真实运行：

```text
RUNTIME VERIFIED
```

如果人工：

```text
MANUAL VERIFICATION REQUIRED
```

---

# 80. Final Audit Format

最终输出必须严格：

```text
# M12 Final Security / Privacy / License Audit

## A. Threat Model
PASS / BLOCKED / FAIL

## B. IPC / Capability Security
PASS / BLOCKED / FAIL

## C. CSP / Frontend Security
PASS / BLOCKED / FAIL

## D. Filesystem Security
PASS / BLOCKED / FAIL

## E. Network / SSRF
PASS / BLOCKED / FAIL

## F. Audio / Metadata / Artwork
PASS / BLOCKED / FAIL

## G. SQLite / Cache / Analysis
PASS / BLOCKED / FAIL

## H. Secrets
PASS / BLOCKED / FAIL

## I. Updater / Installer / Signing
PASS / BLOCKED / FAIL

## J. Privacy
PASS / BLOCKED / FAIL

## K. Dependency / Supply Chain
PASS / BLOCKED / FAIL

## L. License Compliance
PASS / BLOCKED / FAIL

## M. CI Security
PASS / BLOCKED / FAIL

## N. Documentation
PASS / BLOCKED / FAIL

## O. Runtime Security Regression
PASS / BLOCKED / FAIL

## P. Release Artifact Audit
PASS / BLOCKED / FAIL
```

然后：

```text
# P0 Findings
...

# P1 Findings
...

# Manual Actions Required
...

# Remaining Known Risks
...

# Files Changed
...

# Tests
...

# Security Evidence
...

M12-A = PASS / BLOCKED / FAIL
M12-B = PASS / NOT COMPLETED
COMMIT = <hash>
NEXT = M13
```

---

# 81. Git Commit

只有在：

- M12-A PASS
- 或明确 BLOCKED 且所有阻塞项已文档化
- working tree review 完成
- tests 完成
- diff review 完成

之后才能提交。

提交：

```text
M12: harden security privacy licensing
```

不要 push。

---

# 82. 最终执行纪律

再次强调：

### 这不是一个“加几个安全函数”的任务。

必须从：

```text
Frontend
↓
IPC
↓
Rust
↓
Filesystem
↓
Network
↓
Cache
↓
Decoder
↓
Database
↓
Analysis
↓
Dreamscape
↓
Music Map
↓
SMTC
↓
Updater
↓
Installer
↓
CI
↓
Repository
```

进行完整的 trust-boundary 审计。

### 目标不是“看起来安全”。

目标是：

> **让 ifuyo Aura 在公开 GitHub、真实 Windows 用户环境以及未来商业发行前，形成一套可解释、可测试、可持续维护的安全与合规基线。**

---

# 83. M12 完成条件

最终必须满足：

```text
M0 Commercial Foundation        PASS
M1 Audio Core                   PASS
M2 Playback Quality             PASS
M3 Music Library                PASS
M4 Sources                      PASS
M5 Persistent Cache             PASS
M6 Desktop UI                   PASS
M7 Dreamscape                   PASS
M8 Audio Analysis               PASS
M9 Music Map                    PASS
M10 Windows Integration         PASS
M11 Release Engineering         PASS
M12 Security/Privacy/License    PASS
```

此时项目才能进入下一阶段：

```text
NEXT = M13
```

M12 完成后不要擅自开始 M13。

停止并输出最终审计报告。