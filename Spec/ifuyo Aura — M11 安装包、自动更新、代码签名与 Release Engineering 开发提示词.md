# ifuyo Aura — M11 Release Engineering

你现在进入 **M11：安装包、自动更新、代码签名、Release Pipeline 与发布工程化**。

这是 ifuyo Aura 从“功能完整的 Windows 桌面程序”进入“可真实分发的软件产品”的阶段。

项目基本信息：

- Product: `ifuyo Aura`
- Repository: `ifuyo-aura`
- Brand: `ifuyo · 伊芙游`
- Tech:
  - Tauri 2
  - Rust
  - React
  - TypeScript
  - Vite
  - Zustand
  - Three.js
- Primary platform: Windows 10/11 x64
- Local-first
- No telemetry
- No user account
- No cloud library
- No payment
- No subscription
- No Steam integration yet
- M0–M10 are the existing frozen baseline

---

# 0. 绝对执行规则

你不是在“随便补几个发布脚本”。

你要把当前 ifuyo Aura 建立成一个具备：

> 可构建 → 可打包 → 可验证 → 可签名 → 可发布 → 可检查更新 → 可安全安装更新 → 可回退/恢复 → 可审计

能力的正式桌面软件工程。

必须遵守以下规则：

1. **先审计，后修改。**
2. 不假设当前仓库结构、Tauri 配置、package manager、Rust workspace、CI 已经符合本阶段要求。
3. 以当前真实代码为准。
4. 不重构 M1–M10 已稳定的音频、Library、Source、Cache、UI、Dreamscape、Analysis、Map、SMTC、Tray 等业务架构。
5. 不把 Release/Updater 逻辑塞进 `aura-core`。
6. 不把安装/更新逻辑塞进音频 callback、Library、Analysis 或 Map。
7. 不引入云端账户体系。
8. 不添加 telemetry。
9. 不偷偷增加 analytics。
10. 不加入广告 SDK。
11. 不增加与产品目标无关的第三方服务。
12. 不下载运行时模型。
13. 不把任何真实私钥、证书、密码、token、GitHub secret、PFX、PEM secret、Tauri updater private key 写入仓库。
14. 不把敏感文件写进 Git history。
15. 不使用 fake signature。
16. 不为了让测试通过而绕过 signature validation。
17. 不伪造真实代码签名。
18. 不把 unsigned artifact 标记成 signed。
19. 不把本地测试发布称为 production release。
20. 如果缺少真实生产证书/生产私钥/生产发布凭据：
   - 可以完成架构、脚本、配置模板、mock、local signing、CI dry-run；
   - 但必须明确标记 `BLOCKED`；
   - 不允许编造 `PASS`。
21. 不开始 M12。
22. 本阶段结束必须停止。

---

# 1. M11 唯一目标

M11 的目标：

> **让 ifuyo Aura 具备正式 Windows 分发和安全自动更新的工程基础。**

本阶段完成以下完整链路：

```text
Git tag
   ↓
Version validation
   ↓
CI
   ↓
Frontend build
   ↓
Rust build
   ↓
Tauri bundle
   ↓
Windows installer
   ↓
Updater artifact
   ↓
Artifact hashing
   ↓
Signature
   ↓
Release metadata
   ↓
GitHub Release / configured distribution endpoint
   ↓
Existing ifuyo Aura
   ↓
Update check
   ↓
Signature verification
   ↓
Download
   ↓
Install
   ↓
Restart
   ↓
New version
```

并且这条链路必须具有：

- 可测试性
- 可审计性
- 可失败
- 可恢复
- 可撤销发布
- 明确版本
- 明确 channel
- 明确 artifact
- 明确签名状态

---

# 2. M11 范围

## A-grade 必须完成

### A1. Release Version Authority

建立统一版本来源。

必须审计并解决：

- `package.json`
- Tauri config
- Rust package version
- Git tag
- installer version
- updater version
- release metadata

之间可能出现的版本漂移。

最终必须建立单一明确的：

```text
ApplicationVersion
```

规则。

至少支持：

```text
MAJOR.MINOR.PATCH
```

例如：

```text
0.1.0
0.2.0
1.0.0
1.0.1
```

不得允许：

```text
package.json = 0.2.0
Tauri = 0.1.9
Git tag = app-v0.2.1
```

这种状态进入 release pipeline。

建立 version validation script：

```text
version:check
```

至少检查：

- semver 合法性
- frontend version
- Rust version
- Tauri version
- Git tag version
- generated release metadata version

必须一致。

---

# 3. Windows Installer

目标平台：

```text
Windows 10/11 x64
```

至少建立：

```text
NSIS installer
```

并根据当前 Tauri 实际配置评估：

```text
MSI
```

是否同时发布。

不要为了“看起来商业化”而无意义同时维护大量安装格式。

优先建立：

```text
ifuyo Aura_<version>_x64-setup.exe
```

正式命名规则。

---

## Installer 必须考虑

### 安装路径

默认采用适合普通桌面用户的 current-user 模式。

避免默认要求 Administrator，除非当前产品需求确实要求 per-machine。

---

### Shortcuts

根据当前实际产品需求决定：

- Start Menu shortcut
- Desktop shortcut

不要无条件制造多个快捷方式。

---

### Upgrade

必须验证：

```text
Old Version
   ↓
New Installer
   ↓
Existing User Data preserved
```

不得因为升级而删除：

- Library DB
- Artwork cache
- persistent cache
- playlists
- settings
- analysis database
- Dreamscape state
- Map state
- application preferences

除非 migration 明确要求。

---

### Uninstall

卸载程序与用户数据必须明确分离。

默认：

```text
Uninstall application
≠
Delete personal music library
```

不得删除用户的真实音乐文件。

不得执行：

```text
Delete entire Music folder
Delete arbitrary user data
```

必须检查卸载行为是否只针对应用安装目录和明确的应用数据。

如当前应用提供“删除用户数据”选项，必须显式、二次确认，并记录到文档。

---

# 4. User Data Migration

建立：

```text
data version
schema version
application version
```

的清晰概念。

升级时必须验证：

```text
Old DB
↓
Migration
↓
New DB
```

不存在：

```text
application update
→ silently wipe database
```

必须覆盖：

- Library SQLite
- cache index
- settings
- playlists
- analysis data
- Dreamscape state
- Music Map derived state

---

# 5. Automatic Updater

使用当前 Tauri 2 官方 updater architecture。

不要自己实现一个“下载 exe 然后执行”的简陋 updater。

必须建立独立的：

```text
UpdateService
```

或等价 abstraction。

建议结构：

```text
src/
  app/
    update/
      UpdateService.ts
      updateTypes.ts
      updateState.ts

src-tauri/
  update/
    updater.rs
    update_policy.rs
```

实际目录允许根据当前架构调整。

---

# 6. Updater Architecture

Updater 必须遵循：

```text
UI
 ↓
Application Update Service
 ↓
Tauri Updater Adapter
 ↓
Verified Update Artifact
```

不能：

```text
React
 ↓
直接执行 installer
```

也不能：

```text
React
 ↓
PowerShell
 ↓
download exe
 ↓
execute
```

禁止 shell-based arbitrary installer execution。

---

# 7. Update State Machine

建立明确状态机：

```text
Idle
Checking
UpdateAvailable
Downloading
Downloaded
Installing
RestartRequired
UpToDate
Failed
Cancelled
```

必要时增加：

```text
PolicyBlocked
SignatureInvalid
ManifestInvalid
NetworkUnavailable
UnsupportedPlatform
```

状态必须是 typed。

禁止：

```ts
status: string
```

这种弱类型核心状态。

使用明确 union：

```ts
type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "available"; version: string }
  ...
```

---

# 8. Update Check Policy

不要每次应用启动都疯狂请求更新服务器。

必须定义策略：

例如：

```text
manual check
startup delayed check
periodic check
```

但必须考虑：

- cold start
- network unavailable
- metered network
- offline mode
- privacy
- user preference

建议：

- 启动后延迟检查
- 增加最小检查间隔
- 本地记忆 last check
- 失败采用 bounded retry
- 不影响应用正常使用

禁止：

```text
Update server unavailable
→ application startup blocked
```

---

# 9. Update Network Policy

Updater 是产品需要的唯一特殊网络功能之一，但必须被严格约束。

要求：

- HTTPS only in production
- certificate validation enabled
- timeout
- bounded download
- cancellation
- no infinite retry
- no arbitrary redirect chains
- no arbitrary executable URLs
- no arbitrary command execution
- no user supplied update endpoint

Production environment 不允许：

```text
http://
```

除非当前官方 Tauri configuration 明确将其用于本地测试，并且测试路径与 production configuration 完全隔离。

---

# 10. Update Signature

这是 M11 的核心安全边界。

建立：

```text
Updater Public Key
Updater Private Key
```

明确生命周期。

必须遵守：

```text
PUBLIC KEY
→ may be stored in application configuration

PRIVATE KEY
→ NEVER repository
→ NEVER frontend
→ NEVER packaged into app
→ NEVER GitHub artifact
→ NEVER logs
```

---

# 11. Private Key Policy

必须建立正式文档：

```text
docs/CODE_SIGNING_POLICY.md
docs/RELEASE_SECURITY.md
```

至少记录：

- updater key ownership
- private key storage
- who can access
- CI secret usage
- rotation limitations
- backup requirements
- incident response
- lost-key consequences
- revoked-key procedure
- production vs development key

如果仓库已经存在：

```text
CODE_SIGNING_POLICY.md
```

则更新，而不是另建冲突文档。

---

# 12. Development / Production Key Separation

绝对不要：

```text
dev key == production key
```

建立：

```text
development
staging
production
```

至少 conceptual separation。

如果目前项目没有 staging environment：

```text
dev
production
```

也必须明确区分。

---

# 13. Tauri Updater Artifact

根据当前 Tauri 2 官方 updater 方式配置：

- updater artifact generation
- updater public key
- endpoint
- signature
- release metadata

不要自行猜测配置字段。

执行前：

> 阅读当前安装版本对应的官方 Tauri 2 updater 文档与实际 CLI/config schema。

不要照抄旧版 Tauri 1 配置。

---

# 14. Update Manifest

建立明确的 release metadata。

至少包含：

```text
version
platform
architecture
download URL
signature
release notes
pub date
minimum supported version
```

实际字段以当前 Tauri updater protocol 为准。

不得维护两套互相漂移的 metadata schema。

---

# 15. Update Channel

至少设计：

```text
stable
```

并为未来预留：

```text
beta
nightly
```

但 M11 默认只启用：

```text
stable
```

不要在 UI 中加入没有实际后端支持的 beta/nightly 开关。

---

# 16. Rollout Policy

设计 update rollout concept。

至少支持未来：

```text
100%
```

以及 conceptual：

```text
percentage rollout
```

但不要在 M11 做复杂云端 rollout service。

M11 可以：

```text
document the policy
```

而不是开发一个 SaaS。

---

# 17. Rollback

Windows updater 必须考虑：

```text
new version installation failed
```

或者：

```text
new version launches but migration fails
```

建立 recovery strategy。

至少保证：

- application data backup/migration safety
- installer failure does not corrupt Library
- updater failure does not delete user content
- old application remains usable where platform installer/updater semantics permit
- failed update is observable
- retry is bounded

不要虚构 Tauri 本身不存在的 automatic binary rollback。

必须明确：

```text
artifact rollback
```

与：

```text
database rollback
```

是两个问题。

---

# 18. Release Pipeline

使用 GitHub Actions 或当前仓库已经存在的 CI infrastructure。

Release pipeline 至少分成：

```text
validate
test
build
bundle
sign
publish
verify
```

不要把所有内容塞进一个 500 行 YAML job。

建议：

```text
ci.yml
release.yml
```

或等价结构。

---

# 19. Pull Request CI

PR 必须至少执行：

Frontend：

```text
npm run typecheck
npm run lint
npm test
npm run build
```

Rust：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

License/security：

```text
cargo deny check
```

实际命令以当前仓库为准。

不要为了 M11 临时删除失败的 quality gate。

---

# 20. Release Trigger

优先采用明确版本 tag：

```text
app-v*
```

例如：

```text
app-v0.1.0
app-v0.2.0
app-v1.0.0
```

release workflow 必须：

1. 读取 tag
2. 验证 semver
3. 验证 repository version
4. 运行完整测试
5. 构建 installer
6. 构建 updater artifact
7. 生成 checksums
8. 签名
9. 生成 release metadata
10. 发布 GitHub Release
11. 验证发布内容
12. 输出 release summary

---

# 21. Release Must Fail Closed

任何关键步骤失败：

```text
test failure
version mismatch
bundle failure
signature failure
checksum failure
missing artifact
invalid metadata
```

必须：

```text
FAIL
```

而不是：

```text
continue publishing
```

---

# 22. Artifact Manifest

每次 Release 都应该有明确 artifact inventory。

例如：

```text
ifuyo Aura_<version>_x64-setup.exe
ifuyo Aura_<version>_x64.msi
latest.json
checksums.txt
release-notes.md
```

实际文件名以 Tauri 当前生成结果为准。

必须自动验证：

- artifact exists
- non-zero size
- expected architecture
- expected version
- SHA-256 calculated
- updater signature present
- no accidental debug binary
- no source archive mistaken as application artifact

---

# 23. SHA-256

所有公开分发 artifact 建立 SHA-256。

例如：

```text
SHA256
Artifact
Size
Version
Platform
Arch
```

写入：

```text
checksums.txt
```

或者结构化 manifest。

不要只依赖浏览器下载页面。

---

# 24. Windows Code Signing

建立真正的：

```text
Windows Authenticode signing
```

release stage。

但必须区分：

```text
Updater signing
```

与：

```text
Windows code signing
```

这是两个不同的安全系统。

不能认为：

```text
Tauri updater signature
=
Windows Authenticode signature
```

---

# 25. Production Certificate

如果当前没有真实 Windows code-signing certificate：

不要：

```text
fake PASS
```

必须输出：

```text
M11-A BLOCKED
Reason:
Production Windows code-signing certificate is unavailable.
```

但仍然完成：

- signing abstraction
- CI secret wiring
- local unsigned build
- validation script
- signature verification test
- documentation
- release dry-run

---

# 26. Signing Secret Handling

GitHub Actions 中：

```text
PRIVATE KEY
CERTIFICATE
PASSWORD
```

必须只来自：

```text
GitHub Secrets
```

或者受控的 external signing service。

禁止：

```text
echo "$SECRET" > repository file
git add
```

禁止写入：

```text
logs
artifacts
cache
test snapshots
```

禁止：

```text
print(secret)
```

---

# 27. Signing Logs

CI 日志必须能够回答：

```text
Was artifact signed?
Which artifact?
Which certificate identity?
Which version?
```

但不能暴露：

```text
private key
certificate password
secret
token
```

---

# 28. Public Repository Security

由于该项目可以是 public repository：

必须扫描：

```text
git history
working tree
CI config
package.json
Cargo files
docs
examples
fixtures
```

寻找：

- passwords
- API keys
- private keys
- certificates
- .pfx
- .pem
- GitHub PAT
- updater private key
- signing secrets
- cloud credentials

尤其扫描 Git history。

不能只扫描 HEAD。

---

# 29. Secret Scanner

加入至少一种成熟 secret scanning strategy。

可以使用：

```text
gitleaks
```

或者当前项目已有等价工具。

要求：

- local audit
- CI audit
- baseline policy
- documented false positives

不要为了让 CI 通过而简单关闭 secret scanning。

---

# 30. Release Reproducibility

建立 release build metadata：

```text
commit SHA
version
build timestamp
Rust version
Node version
Tauri version
OS
architecture
build mode
```

注意：

不要在公开应用 UI 中显示不必要的内部 build information。

诊断页面可以显示：

```text
Version
Build
Commit
```

是否公开 commit SHA 由当前安全策略决定。

---

# 31. Build Environment Pinning

尽可能固定：

```text
Node version
package manager
Rust toolchain
Tauri CLI version
```

至少建立：

```text
rust-toolchain.toml
```

或者当前项目对应方案。

不要让：

```text
developer A
CI B
release C
```

使用完全不同的 Rust/Node toolchain。

---

# 32. Dependency Reproducibility

检查：

```text
package-lock.json / pnpm-lock.yaml / yarn.lock
Cargo.lock
```

release build 必须使用锁定版本。

禁止 release pipeline：

```text
npm install
cargo update
```

导致依赖无审计漂移。

优先：

```text
npm ci
cargo build --locked
```

或者与当前 package manager 对应的严格锁定方案。

---

# 33. Release Notes

建立自动 release notes generation。

至少来源：

```text
CHANGELOG.md
Git commits
version
```

release notes 必须区分：

```text
Added
Changed
Fixed
Security
Known Issues
```

不要把内部 AI coding notes 原样暴露给用户。

---

# 34. Update UX

M11 允许加入最小 Update UI。

位置建议：

```text
Settings → General / Updates
```

至少显示：

```text
Current Version
Update Status
Check for Updates
New Version Available
Release Notes
Download Progress
Install & Restart
Error
```

必须支持：

```text
Up to date
```

状态。

---

# 35. Update UX Restrictions

禁止：

```text
popup every startup
```

禁止：

```text
forced update
```

除非未来存在明确安全紧急策略。

M11 默认：

```text
user control
```

原则。

---

# 36. Offline

Offline 必须是正常状态，而不是异常程序状态。

当 updater endpoint 不可达：

```text
Music playback remains fully functional.
Library remains functional.
Dreamscape remains functional.
Map remains functional.
```

更新检查失败只能影响更新功能。

不能影响：

- audio engine
- local library
- cache
- playlist
- analysis
- Dreamscape
- Music Map

---

# 37. Updater 与 Playback 的边界

严禁：

```text
update check
→ touch audio callback
```

严禁：

```text
installer
→ modify active audio engine
```

安装前必须考虑：

```text
Is currently playing?
```

必要时：

```text
gracefully stop/prepare shutdown
```

但不能：

```text
kill process abruptly
```

---

# 38. Update While Playing

至少测试：

```text
Music playing
→ update available
→ user downloads
→ user installs
→ application shutdown
→ updater runs
→ app starts
```

验证：

- audio stops cleanly
- DB transaction complete
- cache integrity preserved
- application closes
- tray closes
- SMTC cleans up
- no orphan process
- no corrupt settings

---

# 39. Update While Dreamscape Open

测试：

```text
Dreamscape active
→ update
→ shutdown
→ restart
```

验证：

- renderer disposes
- WebGL resources released
- no zombie renderer
- window closes
- update can proceed
- new version launches normally

---

# 40. Update While Analysis Running

必须明确处理：

```text
M8 analysis worker active
→ application shutdown
→ update
```

要求：

- cancellation
- graceful worker termination
- no DB corruption
- incomplete analysis remains retryable
- no half-written BLOB
- no deadlock

---

# 41. Update During Cache Activity

测试：

```text
M5 cache write active
→ update initiated
```

验证：

- writes complete or cancel safely
- SQLite WAL remains valid
- temporary files cleaned
- partial chunks never become valid entries
- cache index consistent after restart

---

# 42. Database Migration Safety

至少测试两个 release versions：

```text
N
N+1
```

升级。

如果已有 migration infrastructure：

不要重新创建第二套。

必须验证：

```text
migration applied exactly once
```

重复启动不会重复迁移。

---

# 43. Crash-Safe Update State

建立最小 persistent update diagnostic state，例如：

```text
last_check
last_update_version
last_update_status
last_update_error
```

但不得记录敏感 URL query 参数、token 或 secret。

---

# 44. Telemetry Boundary

严格：

```text
NO TELEMETRY
```

因此不要实现：

```text
update success analytics
update adoption analytics
crash analytics
user behavior analytics
```

M11 的：

```text
last_update_status
```

只能存在本地。

---

# 45. Update Endpoint Privacy

Updater request 不应发送：

```text
music library
playlist
artist history
track names
personal path
device identifier
```

至少必须保证没有主动发送此类数据。

请求尽可能只包含：

```text
app version
platform
architecture
channel
```

实际 updater protocol 字段以官方机制为准。

---

# 46. Security Headers / Manifest Validation

如果使用自有 update endpoint 或 static JSON：

必须定义：

- HTTPS
- content type expectations
- size bounds
- schema validation
- signature validation
- version validation
- platform validation
- architecture validation

不要相信远程 JSON 的所有字段。

---

# 47. Update Downgrade Protection

默认：

```text
0.2.0
→ 0.1.0
```

不得被当作正常 update。

至少建立：

```text
version ordering
```

并拒绝低于当前版本的 update artifact。

除非未来显式建立 rollback channel。

---

# 48. Update Compatibility

定义：

```text
minimumSupportedVersion
```

但不要过早实现强制升级。

如果版本跨度过大：

```text
migration unsupported
```

应进入：

```text
UpdateBlocked
```

而不是强行安装。

---

# 49. Installer Identity

正式定义：

```text
productName
identifier
publisher
manufacturer metadata
version
```

确保：

- Windows uninstall entry 正确
- shortcut 正确
- installer identity 稳定
- update identity 稳定

不要因为换 installer 名字导致：

```text
Windows sees a different app
```

---

# 50. Stable Application Identifier

绝对不要随便修改：

```text
com.ifuyo.aura
```

或者当前实际 identifier。

必须审计当前实际 identifier。

一旦建立稳定生产 identity：

> 将它视为长期兼容性边界。

---

# 51. Windows Registry

如果当前 Tauri installer 不需要额外 Registry：

不要为了“商业化”而增加大量自定义 registry。

只有实际用途才允许写入：

- uninstall metadata
- required installer configuration
- future file association placeholders only if actually used

M11 不实现复杂：

```text
file association
URI scheme
shell extension
context menu integration
```

除非当前已有明确需求。

---

# 52. No File Association Scope Creep

M11 不实现：

```text
.mp3 double click association
.flac association
.m3u association
Windows context menu
```

这些属于未来版本能力。

本阶段先保证：

```text
installed app
→ stable identity
→ secure update
```

---

# 53. GitHub Release

优先使用：

```text
GitHub Releases
```

作为第一版 artifact distribution target，除非当前项目已有其他正式发布基础设施。

Release 必须包含：

- installer
- updater artifact
- update metadata
- checksum
- release notes

不要上传：

```text
node_modules
target/
debug/
source map dump
private symbols
private certificates
```

除非明确需要。

---

# 54. GitHub Release as Source of Updater

如果采用 GitHub Releases：

必须让 updater metadata 指向正确 release artifact。

不能写：

```text
latest/download/random-file.exe
```

然后依赖文件名猜测。

必须建立明确 artifact mapping。

---

# 55. Draft Release / Production Release

Release pipeline 推荐：

```text
build
→ draft release
→ verify
→ publish
```

而不是：

```text
build
→ immediately public
```

至少为 release validation 留出一个明确阶段。

---

# 56. Release Verification

发布后必须自动验证：

```text
GitHub Release exists
Artifact exists
Checksum matches
Version matches
Updater manifest valid
Signature present
Platform matches
Architecture matches
```

如可以在 CI 中使用 public release URL 做验证，应在正式发布 pipeline 完成后执行。

---

# 57. Installer Smoke Test

Windows CI 必须至少有 installer smoke test。

目标：

```text
Install
→ launch
→ version query
→ basic window
→ exit
```

必要时：

```text
uninstall
```

也进行 smoke test。

测试不得依赖：

- 用户真实音乐文件
- 用户真实账户
- 公共音乐服务
- 公共 radio
- external website availability

使用：

```text
fixtures
fake local server
synthetic test library
```

---

# 58. Upgrade Smoke Test

构建：

```text
N
N+1
```

然后：

```text
install N
→ seed test data
→ install N+1
→ launch
→ validate data
```

至少验证：

```text
Library
Playlist
Settings
Analysis
Dreamscape state
Map state
Cache metadata
```

---

# 59. Updater Integration Test

建立一个 local fake update server。

例如：

```text
localhost
```

模拟：

### Case 1

```text
200
valid manifest
valid signature
```

结果：

```text
UpdateAvailable
```

### Case 2

```text
invalid JSON
```

结果：

```text
ManifestInvalid
```

### Case 3

```text
invalid signature
```

结果：

```text
SignatureInvalid
```

### Case 4

```text
downgrade
```

结果：

```text
UpdateRejected
```

### Case 5

```text
timeout
```

结果：

```text
Failed / NetworkUnavailable
```

### Case 6

```text
HTTP 500
```

结果：

```text
Failed
```

### Case 7

```text
corrupt artifact
```

结果：

```text
Rejected
```

---

# 60. No Public Network in CI

CI 不依赖：

```text
Google
GitHub pages
Radio Browser
Internet radio
Spotify
YouTube
QQ Music
NetEase
Apple Music
```

除：

```text
GitHub Release publishing itself
```

这类 release infrastructure 必需操作外。

功能测试使用：

```text
local fixture
fake server
mock endpoint
```

---

# 61. Current Tauri Documentation Check

在开始编码前：

必须阅读当前项目使用版本对应的 Tauri 2 官方文档：

- updater plugin
- Windows installer
- Windows code signing
- GitHub pipeline
- capabilities / permissions
- security configuration

不要根据记忆使用旧 API。

尤其注意：

```text
Tauri 1
```

与：

```text
Tauri 2
```

之间配置和 plugin architecture 的差异。

---

# 62. Tauri Capability / Permission Audit

由于 updater 属于高权限能力：

必须审计：

```text
src-tauri/capabilities/
```

确认 updater 权限只暴露真正需要的能力。

禁止：

```text
allow *
```

式粗暴授权。

原则：

> 最小权限。

Frontend 只能获得：

```text
check
download
install
```

中真正需要的能力。

不要因为调试方便开启全部插件权限。

---

# 63. Frontend IPC

定义 typed IPC contract：

```text
checkForUpdate()
getUpdateState()
downloadUpdate()
installUpdate()
cancelUpdate()
```

具体名称按现有架构统一。

禁止：

```ts
invoke("anything", rawUnknownObject)
```

核心 update command 参数必须：

```text
typed
validated
bounded
```

---

# 64. Rust Update Boundary

Rust 后端负责：

- updater lifecycle
- platform validation
- command validation
- error normalization
- security policy
- update state where appropriate

Frontend 负责：

- presentation
- user actions
- progress display
- localization

不要让 frontend 成为 security authority。

---

# 65. Error Model

禁止：

```text
catch(e) {
  return "Update failed"
}
```

必须定义 typed error categories。

例如：

```text
Network
Timeout
Manifest
Signature
Version
Platform
Architecture
Permission
Installer
Cancellation
Unknown
```

UI 可以将它们映射成用户可读文本。

日志保留技术细节，但不能包含 secret。

---

# 66. Localization

更新 UI 支持：

```text
zh
en
```

至少：

```text
Check for updates
Checking...
You are up to date
Update available
Downloading...
Installing...
Restart required
Update failed
Invalid update signature
```

禁止直接将英文硬编码在 React component 中。

---

# 67. Accessibility

Update UI 必须遵守 M6 已建立的：

- keyboard navigation
- focus visible
- reduced motion
- readable contrast
- screen-reader friendly labels

不要因为这是 Settings 功能就绕过现有设计系统。

---

# 68. Progress

下载进度必须：

```text
bounded
monotonic where possible
```

不要：

```text
120%
```

不要无限跳动。

如果服务器没有总大小：

显示：

```text
Downloading...
```

而不是伪造百分比。

---

# 69. Update Cancellation

必须明确：

```text
Can user cancel before installation?
```

建议：

```text
Checking → cancellable
Downloading → cancellable
Installing → not cancellable once committed
```

安装已经启动后不能提供虚假的 Cancel。

---

# 70. Restart Semantics

安装成功后：

```text
RestartRequired
```

或者由官方 Tauri updater 流程触发退出/重启。

必须保证：

- no double restart
- no restart loop
- no background zombie
- tray cleanup
- SMTC cleanup
- analysis worker shutdown
- renderer disposal

---

# 71. Single Instance Compatibility

必须与 M10 single-instance 集成。

更新重启期间：

不能出现：

```text
Old process + new process both active
```

必须验证：

```text
restart
→ one process
```

---

# 72. SMTC Compatibility

与 M10 SMTC 集成。

更新退出时：

```text
SMTC session cleaned
```

不得让 Windows 媒体控件长期残留 stale state。

---

# 73. Tray Compatibility

与 M10 Tray 集成。

更新期间：

```text
tray icon
→ disposed cleanly
```

不允许：

```text
ghost tray icon
```

---

# 74. Crash / Recovery

如果应用在更新前崩溃：

下次启动必须正常。

如果 updater 下载失败：

下次仍能正常使用旧版本。

如果 updater metadata invalid：

不能阻塞正常启动。

如果 update server unreachable：

不能阻塞正常启动。

---

# 75. Performance

更新检查不能明显阻塞：

```text
startup
audio
library loading
Dreamscape
Music Map
```

不要在：

```text
main UI thread
```

做大量：

- hashing
- file IO
- installer processing
- synchronous network
```

---

# 76. File System Safety

Updater 处理路径时：

必须避免：

```text
path traversal
arbitrary file overwrite
arbitrary executable launch
```

所有路径：

```text
validated
canonicalized where necessary
bounded
```

禁止用户输入路径直接进入 installer execution path。

---

# 77. Release Configuration Separation

建立：

```text
development
production
```

配置隔离。

例如：

```text
tauri.dev.conf
tauri.release.conf
```

或当前项目适合的方式。

Production updater endpoint：

```text
HTTPS
```

Development：

```text
local fake server
```

不能让 dev config accidentally enter release.

---

# 78. CI Secrets Guard

在 release workflow 开头增加安全检查。

如果生产 signing secret 不存在：

必须：

```text
fail closed
```

而不是：

```text
generate random certificate
pretend production
```

允许另设：

```text
unsigned-dry-run
```

workflow。

---

# 79. Dry Run

增加：

```text
release dry run
```

能力。

目标：

```text
version
→ build
→ bundle
→ checksum
→ manifest
```

但是：

```text
do not publish
do not sign with production secret
```

用于本地和 PR 验证。

---

# 80. Release Script

建立可重复执行的 release command。

例如：

```text
npm run release:check
npm run release:build
npm run release:verify
```

具体命令根据当前 package.json 统一。

脚本必须：

- exit non-zero on failure
- readable output
- no hidden side effects
- no force push
- no automatic git commit
- no automatic version bump without explicit command

---

# 81. Version Bumping

不要允许：

```text
CI automatically changes package.json
→ commit
→ push
```

Release pipeline 应尽量只消费：

```text
already committed version
```

版本变更由开发流程完成。

---

# 82. Changelog Discipline

必须保证：

```text
version tag
↔ CHANGELOG entry
```

匹配。

例如：

```text
## [0.2.0]
### Added
...
```

release pipeline 检查对应版本段是否存在。

---

# 83. Build Metadata

应用内部可以提供：

```text
Version
Build channel
```

例如：

```text
ifuyo Aura 0.2.0
Stable
```

不把：

```text
private CI information
```

暴露给最终用户。

---

# 84. Update Settings

至少考虑：

```text
Automatically check for updates
```

默认行为必须符合：

```text
No telemetry
No forced update
Non-blocking startup
```

设置持久化进入现有 Settings system。

不要创建第二套 Settings DB。

---

# 85. Manual Update

UI 必须支持：

```text
Check for Updates
```

这样用户可以在完全关闭 automatic update check 后主动检查。

---

# 86. Release Security Documentation

更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
SECURITY.md
PRIVACY.md
CODE_SIGNING_POLICY.md
THIRD_PARTY_LICENSES.md
PROGRESS.md
CHANGELOG.md
```

必要时新增：

```text
RELEASE.md
RELEASE_SECURITY.md
```

内容至少说明：

- release flow
- signing
- updater
- key management
- artifact validation
- public distribution
- rollback/recovery
- CI responsibilities

---

# 87. Privacy Documentation

更新隐私文档：

明确：

```text
ifuyo Aura does not use telemetry.
```

并明确 updater 网络请求：

- 为什么存在
- 请求什么
- 不发送什么
- 用户如何关闭自动检查
- 更新失败是否影响本地音乐播放

---

# 88. Third-Party License Audit

M11 新增 dependencies 必须继续遵守 M0 license policy。

Allowed baseline：

```text
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
```

继续禁止：

```text
GPL
AGPL
LGPL
```

以及未经审计的 proprietary redistribution requirements。

执行：

```text
cargo deny check
```

或当前等价许可证审计。

---

# 89. Installer Dependency Audit

检查 installer 是否引入：

- WebView2 requirement
- VC runtime
- helper executable
- updater binary
- signing helper

所有新增 runtime dependency：

必须记录：

```text
license
source
version
distribution rights
```

---

# 90. Binary Audit

对最终 installer / updater artifact 进行基本审计。

检查：

```text
unexpected DLL
unexpected EXE
debug runtime
test binaries
development certificate
private key
.env
source maps with secrets
```

尤其不能把：

```text
updater private key
```

打包进去。

---

# 91. Release Artifact Scan

Release artifact 必须扫描：

```text
strings
file tree
archive contents
```

确认：

```text
No secret
No private key
No test fixture
No local absolute path
No personal username
No developer home path
```

---

# 92. Windows Runtime Path Audit

检查构建结果中是否残留：

```text
C:\Users\<developer>\
D:\project\
OneDrive\
Desktop\
Documents\
```

之类开发者绝对路径。

如 debug symbol 中不可避免存在：

必须确认：

```text
not shipped
```

---

# 93. Installer Branding

安装程序使用：

```text
ifuyo Aura
```

而不是：

```text
Tauri
Vite
React
```

等内部工程名称。

应用图标、publisher、description 与 M0 brand identity 保持一致。

---

# 94. Internal Naming Boundary

允许：

```text
aura-*
```

继续存在于：

- Rust crate
- architecture
- internal docs
- source code

最终用户 installer UI、About、Start Menu、uninstaller 显示的产品名必须是：

```text
ifuyo Aura
```

不得把其他内部工程名称当作产品名显示。

---

# 95. Public Release Naming

正式产品名称：

```text
ifuyo Aura
```

不要发布成：

```text
Aura Music
IFUYO Music
ifuyo Player
```

除非后续产品章程再次正式变更。

---

# 96. Testing Matrix

至少覆盖：

### Core

```text
version comparison
manifest parsing
signature validation
checksum validation
state transitions
error mapping
```

### Installer

```text
fresh install
upgrade
uninstall
user data preservation
```

### Updater

```text
no update
update available
download
cancel
signature mismatch
version mismatch
downgrade
network error
timeout
bad manifest
corrupt artifact
```

### Integration

```text
playing track
SMTC active
tray active
Dreamscape active
analysis active
cache active
Map open
```

### Windows

```text
Windows 10
Windows 11
x64
DPI scaling
dark/light
multi-monitor
```

---

# 97. Long-Running Update Soak

至少建立：

```text
install old
→ launch
→ check update repeatedly
→ cancel
→ retry
→ download
→ restart
→ relaunch
```

避免：

- memory leak
- duplicate timers
- duplicate update listeners
- zombie processes
- orphan windows
- duplicate tray icon

---

# 98. Duplicate Update Listener Test

尤其注意 React 生命周期：

```text
mount
unmount
mount
unmount
```

不能导致：

```text
2 update callbacks
4 callbacks
8 callbacks
```

每个 subscription 必须：

```text
registered once
disposed once
```

---

# 99. Update Event Backpressure

不要让 download progress：

```text
every chunk
→ 60/120Hz React state update
```

必要时 throttle/coalesce UI progress events。

Updater 下载速度不应直接决定 UI render frequency。

---

# 100. Error Recovery UX

错误必须给用户有意义的信息。

例如：

```text
无法检查更新
```

而不是：

```text
Error: reqwest::Error { ... }
```

技术诊断可以进入：

```text
Diagnostics
```

但默认 UI 保持人类可读。

---

# 101. No Fake Success

测试、日志、UI、CI 都禁止出现：

```text
Update successful
```

除非：

```text
installation actually succeeded
```

同样：

```text
Signed
```

只能来自真正的签名验证结果。

---

# 102. Production Release Checklist

建立：

```text
docs/RELEASE_CHECKLIST.md
```

至少：

```text
[ ] version matches
[ ] changelog matches
[ ] tests pass
[ ] lint pass
[ ] license audit pass
[ ] secret scan pass
[ ] installer generated
[ ] updater artifact generated
[ ] SHA-256 generated
[ ] updater signature valid
[ ] Windows Authenticode valid
[ ] release metadata valid
[ ] installer smoke test pass
[ ] upgrade smoke test pass
[ ] update integration test pass
[ ] release notes ready
[ ] GitHub Release verified
```

---

# 103. Failure Classification

最后审计结果不能只是：

```text
PASS
```

必须区分：

```text
PASS
BLOCKED
FAIL
```

其中：

### PASS

真实验证完成。

### BLOCKED

外部条件缺失，例如：

```text
production signing certificate unavailable
production updater key unavailable
GitHub release credentials unavailable
```

但代码和 dry-run architecture 已完成。

### FAIL

代码/工程本身有问题，例如：

```text
signature validation broken
version mismatch
installer corrupt
upgrade data loss
CI failed
```

---

# 104. M11-A

A-grade 为：

## M11-A Release Foundation

必须同时满足：

```text
Version Authority
+
Windows Installer
+
Secure Updater
+
Updater Signature
+
Release Metadata
+
Artifact Checksums
+
CI Release Pipeline
+
Secret Protection
+
Installer Smoke Test
+
Upgrade Test
+
Update Integration Test
+
Documentation
```

最终输出：

```text
M11-A = PASS
```

或者：

```text
M11-A = BLOCKED
```

或者：

```text
M11-A = FAIL
```

绝对不允许：

```text
M11-A = PASS
```

但真实 production signing / updater security 仍未配置的情况被掩盖。

---

# 105. M11-B

B-grade 可以做：

- richer update UI
- release channel UI
- staged rollout abstraction
- richer release dashboard
- update history
- installer customization
- taskbar update indicator
- advanced rollback documentation
- package size reporting
- artifact provenance
- SBOM
- enhanced reproducibility
- Windows ARM64 evaluation

但：

> B-grade 不允许破坏 A-grade 的稳定性。

---

# 106. 明确禁止进入 M11

不要实现：

```text
Steam
Steam Overlay
Steam Cloud
Steam Workshop
Steam Achievements
Steam API
```

不要实现：

```text
accounts
login
subscriptions
payments
licensing server
DRM
```

不要实现：

```text
telemetry
analytics
crash reporting SaaS
```

不要实现：

```text
cloud library
cloud playlist sync
cross-device sync
```

不要实现：

```text
YouTube
Spotify
QQ Music
NetEase Cloud Music
Apple Music
```

不要实现：

```text
AI music recommendation
LLM features
cloud embedding
cloud transcription
```

不要实现：

```text
social features
comments
sharing platform
```

不要开始 M12。

---

# 107. Architecture Constraint

最终架构应接近：

```text
                   ┌───────────────────────┐
                   │      ifuyo Aura      │
                   └───────────┬───────────┘
                               │
                    ┌──────────▼─────────┐
                    │ Application Layer  │
                    └──────┬───────┬─────┘
                           │       │
                ┌──────────▼─┐   ┌─▼───────────────┐
                │ Playback   │   │ Release /       │
                │ Authority  │   │ Update Service  │
                └────────────┘   └───────┬─────────┘
                                         │
                                  ┌──────▼──────────┐
                                  │ Tauri Updater   │
                                  │ Installer       │
                                  │ Release Adapter │
                                  └─────────────────┘
```

关键原则：

```text
Audio Core
    ↑
must NOT depend on
    ↓
Tauri / Updater / Installer
```

---

# 108. Dependency Direction

推荐：

```text
aura-core
aura-library
aura-source
aura-cache
aura-analysis
aura-map
aura-synth
    ↓
application layer
    ↓
Tauri adapters
    ↓
Windows / updater / installer
```

不要反过来。

---

# 109. Security Boundary

M11 的安全边界：

```text
Remote Update Metadata
        ↓
Validate
        ↓
Version Check
        ↓
Platform Check
        ↓
Signature Validation
        ↓
Artifact Verification
        ↓
Install
```

绝对不能：

```text
Remote Metadata
        ↓
Download EXE
        ↓
Execute
```

---

# 110. Deterministic Test Fixtures

建立 fixtures：

```text
valid update
invalid signature
expired version
downgrade
malformed manifest
wrong architecture
wrong platform
corrupted artifact
```

fixture 不允许包含真实私钥。

可以使用：

```text
test-only signing key
```

但必须放在：

```text
tests/fixtures/
```

并明确：

```text
TEST ONLY
```

不要复用 production key。

---

# 111. Actual Windows Verification

由于产品第一目标是 Windows：

必须优先在：

```text
Windows
```

执行真实：

```text
tauri build
installer smoke test
updater test
```

不能只在 Linux/macOS 上完成一次：

```text
cargo test
```

然后声称 M11 完成。

---

# 112. CI Platform

至少：

```text
windows-latest
```

作为正式 release builder。

如果当前项目已经有多平台 matrix：

保留现有结构，但：

> M11 的生产 release acceptance 以 Windows x64 为主。

---

# 113. Artifact Size

记录：

```text
Installer compressed size
Updater artifact size
```

建立 baseline。

如果 M11 导致：

```text
+50%
+100%
```

之类异常增长：

必须调查原因。

不要无条件接受：

```text
unused dependency
debug symbols
duplicate runtime
```

---

# 114. Startup Regression

安装/更新能力不能显著增加 startup overhead。

验证：

```text
cold startup
warm startup
offline startup
```

更新检查必须异步、非阻塞。

---

# 115. Final Repository Audit

实施完成后：

检查：

```text
git status
git diff
git diff --cached
```

检查：

```text
private key
secret
certificate
.env
```

检查：

```text
build artifacts
target/
dist/
```

是否正确 ignore。

检查：

```text
no accidental generated release files committed
```

---

# 116. Git History Audit

至少检查：

```text
git log
git diff HEAD~N
```

确认不存在：

```text
private key
password
certificate
token
personal path
```

如发现曾经提交过 secret：

必须停止并报告。

不能简单：

```text
delete file
```

后声称安全。

---

# 117. Final Commands

根据当前项目真实脚本执行：

```text
npm run typecheck
npm run lint
npm test
npm run build

cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

再执行：

```text
actual Windows Tauri build
```

以及：

```text
installer smoke test
updater integration test
upgrade test
```

不要为了兼容当前项目而伪造命令。

先读取：

```text
package.json
Cargo.toml
tauri.conf.json
```

再决定最终命令。

---

# 118. Final Diff Review

完成代码后：

必须：

1. 查看完整 diff。
2. 检查是否有 M1–M10 无关修改。
3. 检查 dependency diff。
4. 检查 Tauri config diff。
5. 检查 capability diff。
6. 检查 CI diff。
7. 检查 release scripts。
8. 检查 docs。
9. 检查 secrets。
10. 检查 generated artifacts。

发现与 M11 无关的重构：

原则上撤销。

---

# 119. Commit

只有所有检查完成后：

```bash
git add .
git commit -m "M11: implement release engineering"
```

不要自动 push。

---

# 120. 最终报告格式

最终必须严格输出：

```text
# M11 Final Audit

## A. Baseline
...

## B. Version Authority
...

## C. Installer
...

## D. Updater
...

## E. Signature
...

## F. Release Pipeline
...

## G. Windows Verification
...

## H. Security
...

## I. Data Migration
...

## J. Performance
...

## K. Documentation
...

## L. Git / Secret Audit
...

## M. Tests
...

M11-A = PASS / BLOCKED / FAIL
M11-B = PASS / NOT COMPLETED
COMMIT = <hash or NOT COMMITTED>
NEXT = M12
```

重要：

### `PASS`

只允许表示真实验证完成。

### `BLOCKED`

必须写明：

```text
具体阻塞项
为什么无法验证
已经完成了什么
剩余什么
```

### `FAIL`

必须指出：

```text
实际失败项
复现方式
影响
建议修复位置
```

---

# 121. Stop Condition

当：

```text
M11-A
M11-B
commit
```

报告完成后：

**立即停止。**

不要顺手开始：

```text
M12
```

不要开始：

```text
Steam
```

不要开始：

```text
commercial payment
```

不要开始：

```text
Workshop
```

不要开始：

```text
cloud sync
```

---

# 122. 最终产品状态目标

M11 完成后，ifuyo Aura 应该从：

```text
功能完整的软件
```

提升为：

```text
可以被用户安装
        ↓
可以被用户升级
        ↓
升级不会破坏用户音乐数据
        ↓
更新包可以验证来源
        ↓
Release 可以自动构建
        ↓
artifact 可以审计
        ↓
公开仓库仍保持安全
```

然后才进入：

```text
M12
```

---

# EXECUTION MODE

现在开始执行。

严格遵守：

```text
AUDIT FIRST
→ PLAN
→ IMPLEMENT
→ TEST
→ SECURITY AUDIT
→ RELEASE DRY RUN
→ FINAL AUDIT
→ COMMIT
→ STOP
```

不要询问我是否继续。

不要把未完成工作描述成已完成。

不要伪造 Windows / signing / updater / release 验证结果。

所有事实必须来自当前仓库、当前构建结果、当前测试结果和当前官方 Tauri 2 文档。