# ifuyo Aura
# Phase 0 — Commercial Foundation Freeze

你现在进入 ifuyo Aura 项目的 Phase 0。

项目正式名称：

`ifuyo Aura`

仓库 / 项目名称：

`ifuyo-aura`

品牌：

`ifuyo · 伊芙游`

不要向我提问，自主完成本阶段。

本阶段目标：

> **审计当前目录 → 建立最终项目身份 → 固化商业化总章程 → 建立工程文档与架构边界 → 创建最小可运行仓库基线。**

本阶段禁止实现 M1 之后的业务功能。

---

# 1. 第一原则

这是一个：

- 个人现在使用；
- 商业产品标准开发；
- 未来可直接商业发行；
- 合法；
- 开源；
- 无遥测；
- Windows-first；
- 本地优先；

的音乐播放器。

不要因为当前用户只有一个人而降低工程标准。

不要因为未来可能商业化而现在提前实现账号、云同步、广告、付费、遥测。

---

# 2. 先完整审计

在修改任何文件之前：

检查当前目录。

确认：

- 当前是否为空；
- 是否已经存在代码；
- 是否已经存在 git；
- git branch；
- git status；
- git remote；
- 当前 Rust；
- 当前 Cargo；
- 当前 Node；
- 当前 npm；
- 当前 Tauri；
- 当前系统；
- 当前 PowerShell / shell；
- 是否存在已有项目配置；
- 是否存在密钥；
- 是否存在 `.env`；
- 是否存在音乐文件；
- 是否存在旧项目文档。

不得假设目录为空。

输出：

# Phase 0 Baseline Audit

至少包含：

A. Environment  
B. Git  
C. Existing Files  
D. Existing Dependencies  
E. Security  
F. Licensing  
G. Architecture  
H. Risks

没有问题就明确写 PASS。

有问题：

记录。

---

# 3. 项目身份冻结

创建或更新：

`brand.json`

必须成为品牌唯一事实来源。

使用：

```json
{
  "name": "ifuyo Aura",
  "vendor": "ifuyo · 伊芙游",
  "slogan": "以光与梦，编织每一次游玩。",
  "site": "https://ifuyo.com"
}
```

不得在其他地方创建第二套品牌事实。

---

# 4. 删除旧名称冲突

全项目搜索：

- Aura Player
- Aura Music
- IFY Music
- IFUYO Music
- old project names
- placeholder names
- old vendor names

如果发现与当前正式身份冲突：

统一修正为：

`ifuyo Aura`

内部模块：

允许使用：

`aura-*`

例如：

```text
aura-core
aura-analysis
aura-synth
```

这是工程代号，不是产品 UI 名称。

---

# 5. 建立文档

创建或更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
SOURCES.md
CONTENT_POLICY.md
PRIVACY.md
BRAND.md
SECURITY.md
CODE_SIGNING_POLICY.md
THIRD_PARTY_LICENSES.md
CHANGELOG.md
```

README：

先建立中英双语基础结构。

---

# 6. 记录关键决策

在 `DECISIONS.md` 记录至少：

## D-001 Product Name

正式产品名：

`ifuyo Aura`

理由：

产品名称直接绑定 ifuyo 品牌与视觉核心 Aura，便于未来商业化。

> 2026-10-02 章程变更：产品名由 `IFUYO Music`（内部工程代号 `Aura`）正式更名为 `ifuyo Aura`。原内部代号并入正式产品名称，不再作为独立工程代号使用。

## D-002 Technology

Tauri 2 + Rust + React + TypeScript + Three.js。

## D-003 Platform

Windows-first。

## D-004 Privacy

No telemetry。

## D-005 Content

No commercial-platform extraction。

## D-006 Licensing

默认 MIT OR Apache-2.0。

## D-007 Codec

官方发行版 codec 策略按总章程执行。

## D-008 Website

备案信息不属于 ifuyo Aura 发布页要求，不实现相关字段或展示。

## D-009 Scope

M0→M14，逐阶段实施。

---

# 7. CONTENT_POLICY

建立正式内容策略。

明确：

禁止：

- 商业平台音频抓取；
- yt-dlp 类 extraction；
- DRM；
- protected streams；
- login-session extraction；
- online lyrics scraping。

明确：

合法来源：

- local；
- direct URL；
- internet radio；
- Radio Browser；
- Dreamscape；
- 后续合法开放来源。

未知来源默认：

`SessionOnly`

---

# 8. PRIVACY

必须写成：

真实行为驱动，而不是营销文案。

明确记录：

本地保存什么。

什么时候联网。

访问哪些服务。

Radio Browser click 默认关闭。

更新检查默认关闭。

无遥测。

敏感凭据使用：

Windows Credential Manager。

---

# 9. ARCHITECTURE

建立正式架构文档。

至少：

```text
UI
 ↓
Tauri IPC
 ↓
Application Layer
 ↓
Domain / Services
 ↓
Crates
```

Rust：

```text
aura-core
aura-sources
aura-cache
aura-library
aura-analysis
aura-synth
aura-testkit
```

规定依赖方向。

尤其：

`aura-core`

不得依赖：

- Tauri；
- UI；
- filesystem；
- network。

核心应该可以通过：

`NullOutput`

独立测试。

---

# 10. Design Tokens

建立：

```text
ui/src/design/tokens.ts
```

建立：

```text
ui/src/generated/
```

CSS variables 从 tokens 生成。

禁止业务代码出现裸色。

建立未来 lint 规则所需结构。

---

# 11. Config

建立：

```text
config/
```

至少包含：

```text
unsupported_platforms.json
```

加入：

- YouTube；
- Spotify；
- 网易云；
- QQ Music；
- Apple Music；
- 其他明确禁止的平台。

识别后：

不得请求。

---

# 12. Gitignore

建立严格 `.gitignore`：

必须忽略：

- target；
- node_modules；
- dist；
- build；
- local databases；
- caches；
- logs；
- `.env`;
- secrets；
- credentials；
- generated local artifacts；
- user music；
- screenshots generated locally where appropriate。

但不得过度 ignore：

核心 source、docs、tests 必须被 Git 跟踪。

---

# 13. License

创建：

```text
LICENSE-MIT
LICENSE-APACHE
```

建立：

`THIRD_PARTY_LICENSES.md`

先建立生成机制说明。

不要虚构依赖许可证列表。

如果本阶段尚未有依赖：

明确写：

> Dependency license inventory will be generated after dependency resolution.

---

# 14. Security Baseline

创建：

`SECURITY.md`

至少规定：

- secret handling；
- dependency review；
- unsafe Rust review；
- IPC validation；
- path traversal protection；
- network URL validation；
- credential handling；
- logging redaction。

---

# 15. CODE_SIGNING_POLICY

建立：

未来准备：

- GitHub Actions；
- SignPath；
- CI-only signing；
- no private key in repository；
- 2FA requirement。

本阶段：

不请求真实签名。

不创建真实 signing secret。

只建立 policy。

---

# 16. PROGRESS

建立：

`PROGRESS.md`

记录：

```text
Phase 0 = IN PROGRESS
M0 = NOT STARTED
...
M14 = NOT STARTED
```

不要伪造任何完成状态。

---

# 17. 不要在 Phase 0 做这些事情

本阶段禁止：

- 音频播放；
- Audio callback；
- decoder；
- DSP；
- SQLite；
- FTS5；
- Dreamscape；
- Map；
- WebGL Aurora；
- Radio Browser；
- HLS；
- Subsonic；
- Jamendo；
- Installer；
- Updater；
- Steam integration。

这些由后续 Milestone 实施。

---

# 18. 最小基础工程

本阶段允许建立：

- Cargo workspace；
- crate directories；
- frontend scaffold；
- Tauri scaffold；
- testkit skeleton；
- CI skeleton；
- lint configuration；
- formatter configuration；
- deny configuration；
- TypeScript configuration；
- basic application entry；
- documentation structure。

但：

> 不得添加未必要的生产依赖。

---

# 19. CI Skeleton

建立 GitHub Actions。

至少预留：

```text
cargo fmt --check
cargo clippy
cargo test
cargo deny
frontend typecheck
frontend lint
frontend test
tauri build
```

本阶段如果某项因为尚未存在对应工程目标而无法运行：

不能伪造 PASS。

应：

- 明确原因；
- 保留正确命令；
- 在 PROGRESS.md 记录；
- 由对应 Milestone 修复。

---

# 20. Dependency Policy

添加依赖前：

先确认：

1. 是否必要；
2. license；
3. maintenance；
4. Windows compatibility；
5. CI buildability；
6. 是否增加架构耦合。

不要因为“以后可能用到”提前添加全部依赖。

---

# 21. Final Phase 0 Audit

完成后重新执行完整审计。

必须检查：

## Identity

- 产品名统一；
- Aura 属于正式产品名称，不再作为单独的内部代号；
- ifuyo 品牌统一。

## Git

- branch；
- status；
- ignored files；
- no secrets。

## Security

搜索：

```text
api_key
apikey
client_secret
password
token
private_key
BEGIN PRIVATE KEY
.env
```

不能出现真实凭据。

## Licensing

确认许可政策文档存在。

## Architecture

确认依赖边界文档存在。

## Documentation

确认全部必须文档存在。

## Build

至少运行现阶段可运行的：

```text
cargo fmt --check
cargo test
npm ...
```

不能把尚未实现的能力伪装成完成。

---

# 22. Git Commit

只有 Phase 0 全部完成并通过审计后：

commit：

```text
Phase 0: freeze commercial foundation
```

不得 push。

除非本阶段发现需要修复的实际 Git 问题，不要创建多个无意义 commits。

---

# 23. 最终输出

完成后输出：

# Phase 0 Final Audit

A. Product Identity  
B. Architecture  
C. Security  
D. Licensing  
E. Privacy  
F. CI  
G. Git  
H. Files Created  
I. Known Limitations  
J. Commit

最后明确：

```text
PHASE 0 = PASS / BLOCKED
NEXT = M0
```

如果 PASS，不要提前实现 M1。

停止在 M0 开始之前。