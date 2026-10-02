# ARCHITECTURE

> 状态：Phase 0 基线。crate 均为空骨架，本文件描述的是**目标边界**，M0 起逐阶段填充实现。
> Status: Phase 0 baseline. All crates are empty skeletons; this file describes target boundaries, filled in from M0 onward.

## 总体分层

```text
UI (React 18 + TypeScript + Vite + Zustand + Three.js)
 ↓
Tauri IPC（typed commands + events，allowlist + input validation）
 ↓
Application Layer（src-tauri：会话、命令路由、策略执行）
 ↓
Domain / Services
 ↓
Crates（纯 Rust 领域内核）
```

## Rust Crates

| Crate | 职责 | 关键约束 |
|---|---|---|
| `aura-core` | 音频引擎：decode → resample → ring buffer → output；时钟、seek generation、gain | **不得依赖 Tauri / UI / filesystem / network**；必须可通过 `NullOutput` 独立测试 |
| `aura-sources` | Source trait 与来源实现（local / direct URL / radio / ICY / Radio Browser / 后续开放来源） | 统一 `search / resolve`；每个 Source 返回 `Allowed / SessionOnly / Forbidden` 缓存策略 |
| `aura-cache` | 分块网络缓存（256 KiB chunks、Range、LRU、pin、故障注入） | 只允许持久化 `Allowed` 来源；`Forbidden` 来源零写盘 |
| `aura-library` | 音乐库：扫描、标签（lofty）、SQLite（WAL + FTS5）、watcher、smart search | 中文 CJK 二元组检索 |
| `aura-analysis` | 后台低优先级分析：BPM、key、LUFS、true peak、energy、MFCC、embedding | 可取消、可恢复、可缓存、content-key 绑定、确定性 |
| `aura-synth` | Dreamscape 确定性生成音乐引擎 | 同 seed 同版本逐位一致 |
| `aura-testkit` | 测试基础设施：NullOutput、FaultFS、合成音频、固定随机种子、确定性时钟 | 仅用于测试，不进入生产依赖 |

## 依赖方向（强制）

```text
src-tauri ──► aura-core / aura-sources / aura-cache / aura-library / aura-analysis / aura-synth
aura-sources ──► aura-core
aura-cache ──► aura-core
aura-library ──► aura-core
aura-analysis ──► aura-core
aura-synth ──► aura-core
aura-testkit ──► （被测试引用，不反向依赖任何 crate 的内部）
```

- **`aura-core` 不依赖 Tauri、UI、filesystem、network。** 音频内核可在无窗口、无网络环境下独立测试。
- crates 之间不得相互污染职责；不允许出现环。
- `windows` crate 及一切平台 API 只允许出现在 `src-tauri`（M10 起），不得进入任何 aura crate 内核。
- UI 不得绕过 IPC 直接触达领域逻辑；高频数据（如可视化帧）走 Tauri channel，频率受控。

## UI 结构

```text
ui/
  src/
    design/tokens.ts        # 设计 token 唯一事实来源（品牌色等，镜像 brand.json）
    generated/              # 由 scripts/generate-tokens.mjs 生成，禁止手改
    ...
```

- CSS variables 只能从 tokens 生成（`scripts/generate-tokens.mjs`，读取 `brand.json` 并与 `tokens.ts` 校验一致）。
- 业务代码禁止裸色值；lint 规则在本阶段仅预留结构，M6 落地。

## 构建与工程

- Cargo workspace：7 个 aura crates；`src-tauri/` 为脚手架，**暂未加入 workspace members**，M0 接线（见 DECISIONS.md D-010）。
- `config/unsupported_platforms.json`：不支持平台识别表，来源系统必须据此拒绝请求。
- CI（`.github/workflows/ci.yml`）预留全部门禁命令，见 PROGRESS.md 的当前可运行状态。
