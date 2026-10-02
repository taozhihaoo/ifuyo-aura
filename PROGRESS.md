# PROGRESS

## Milestone Status

```text
Phase 0 = COMPLETE (2026-10-02)
M0  = NOT STARTED   (Project Foundation: workspace wiring, IPC contracts, testkit, brand pipeline)
M1  = NOT STARTED   (Audio Engine)
M2  = NOT STARTED   (Playback Quality)
M3  = NOT STARTED   (Music Library)
M4  = NOT STARTED   (Sources)
M5  = NOT STARTED   (Cache)
M6  = NOT STARTED   (UI Foundation)
M7  = NOT STARTED   (Dreamscape)
M8  = NOT STARTED   (Analysis)
M9  = NOT STARTED   (Music Map)
M10 = NOT STARTED   (Windows Integration)
M11 = NOT STARTED   (Online Sources)
M12 = NOT STARTED   (Advanced Experience)
M13 = NOT STARTED   (Distribution)
M14 = NOT STARTED   (Final Release Audit)
```

无任何伪造完成状态：除 Phase 0 外全部 NOT STARTED；Phase 0 只包含地基冻结（身份、文档、骨架、CI 配置），不包含任何 M1+ 业务功能。

## Phase 0 Baseline Audit（2026-10-02）

### A. Environment

| 项 | 结果 |
|---|---|
| OS | Windows 11 (10.0.26200), x64 |
| Shell | Git Bash (MINGW64), PowerShell 5.1 备用 |
| Rust / Cargo | ✅ 已安装（2026-10-03）：rustup stable `x86_64-pc-windows-msvc`，rustc/cargo 1.99.0，VS Build Tools 2022 (MSVC 14.44)；见 DECISIONS D-011 |
| Node / npm | node v24.19.0 / npm 11.17.0 |
| Tauri CLI | 未安装（按需经 cargo tauri / npm 引入） |
| cargo-deny | 未安装（M0-2 安装并首跑） |

### B. Git

- 仓库：`E:\WorkSpace\ifuyo-aura`，分支 `main`，Phase 0 之前**零提交**。
- remote：`origin = https://github.com/taozhihaoo/ifuyo-aura.git`（已配置，本阶段不 push）。
- 用户：taozhihaoo / zihaotao@qq.com。
- Phase 0 前未跟踪文件：`.zcodeignore`（ZCode 工作区本地配置，不入库）、`Spec/`（项目规格文档，入库）。

### C. Existing Files

仅 `.zcodeignore` 与 `Spec/`（Charter + Phase 0 + M1–M14 规格）。无代码、无旧项目文档、无音乐文件、无 `.env`、无密钥/证书文件。

### D. Existing Dependencies

无（无 package.json、无 Cargo.toml）。Phase 0 新增依赖均为前端脚手架的必要 devDependencies 与 react/react-dom，见 `ui/package.json`。

### E. Security

对仓库执行 `api_key|apikey|client_secret|password|token|private_key|BEGIN PRIVATE KEY` 扫描：命中的仅为 Spec 中的**审计清单条目**（要求确保这些内容不存在），无真实凭据。PASS。

### F. Licensing

Phase 0 前无 LICENSE 文件。已创建 `LICENSE-MIT`、`LICENSE-APACHE`、`THIRD_PARTY_LICENSES.md`（机制说明；依赖清单待依赖解析后生成）。PASS。

### G. Architecture

Phase 0 前无架构文档。已创建 `ARCHITECTURE.md`（分层、crate 职责、依赖方向、aura-core 隔离约束）。PASS。

### H. Risks

1. 本机无 Rust 工具链 → cargo 类门禁本地不可运行（见下节），由 CI 与 M0 承接。
2. Tauri CLI 未安装 → `tauri build` 门禁延后至 M0 接线后。
3. `Spec/` 中保留少量旧名称（`IFUYO Music` / `Aura Music` 等）——均为刻意的旧名称审计清单与章程变更记录，非冲突。

## Phase 0 Gate Status（§19 规则：不伪造 PASS）

| 门禁命令 | 状态 | 说明 |
|---|---|---|
| `cargo fmt --check` | **PASS**（2026-10-03 本地 msvc） | 首次真实执行，零 diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS**（2026-10-03 本地 msvc） | 零告警 |
| `cargo test --workspace` | **PASS**（2026-10-03 本地 msvc） | 7 crates 骨架测试全过 |
| `cargo deny check` | **部分 PASS**（2026-10-03 本地）：licenses / bans / sources 全绿（127 包）；advisories 因本机无法连接 github.com **NOT RUN**，由 CI 执行 |
| `npm run typecheck` | **PASS**（2026-10-02 本地） | tsc --noEmit 零错误 |
| `npm run lint` | **PASS**（2026-10-02 本地） | eslint 零告警 |
| `npm test` | **PASS**（2026-10-02 本地） | vitest 2/2（design tokens 契约） |
| `npm run build`（vite） | **PASS**（2026-10-02 本地） | 31 modules，dist 产出正常 |
| `tauri build` | **DEFERRED** | src-tauri 未接线（D-010），M0 激活；CI 中该 job 保留命令（`if: false` 占位） |

由对应 Milestone 修复项：M0-2 安装 cargo-deny 并首跑；M0-3 接线 src-tauri 后激活 tauri build 门禁。

## Phase 0 Final Audit（2026-10-02，§21）

- **Identity**：全仓库扫描旧名称（Aura Player / Aura Music / IFY Music / IFUYO Music / IFUYO Player），仓库代码与文档零命中；Spec 中的命中均为刻意的旧名称审计清单与章程变更记录。`brand.json` 为唯一品牌事实。PASS。
- **Git**：分支 `main`；本阶段前零提交；remote 已配置但**未 push**（§22）。`.zcodeignore` 已忽略；node_modules/dist/target 均被忽略。PASS。
- **Security**：对全部新增文件执行 `api_key|apikey|client_secret|password=|private_key|BEGIN PRIVATE KEY` 扫描，零命中；无 `.env`；无音乐文件入库。PASS。
- **Licensing**：LICENSE-MIT / LICENSE-APACHE / THIRD_PARTY_LICENSES.md（机制 + 待生成声明）存在。PASS。
- **Architecture**：ARCHITECTURE.md 含分层、crate 职责与依赖方向、aura-core 隔离约束。PASS。
- **Documentation**：§5 全部 12 份文档存在。PASS。
- **Build**：npm 侧全绿（见上表）；cargo 侧 NOT RUN（工具链缺失，已如实记录，命令保留，CI/M0 承接）。PASS（带已记录限制）。
