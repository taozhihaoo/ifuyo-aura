# RELEASE.md — ifuyo Aura 发布工程

> M11 建立的发布链路：`Git tag → validate → test → build → bundle →
> checksum → (sign) → draft → 人工 verify → publish`。
> 状态：链路工程全部就绪；**生产签名 = BLOCKED**（见
> CODE_SIGNING_POLICY.md §当前状态）。

## 1. 版本权威（§A1）

单一 `ApplicationVersion`，五源一致（`npm run version:check` 强制）：

```text
ui/package.json = src-tauri/tauri.conf.json = src-tauri/Cargo.toml
                = brand.json = 最新 app-v* tag
```

- 版本变更由开发流程显式完成（§81：CI 不自动改版本/不自动提交）。
- tag 命名：`app-vMAJOR.MINOR.PATCH`（§20）。
- CHANGELOG 必须含对应版本段（§82；version:check 给 warning，release
  pipeline 强制 error）。

## 2. 构建与产物（§3/§22/§23）

```powershell
cd ui && npm ci && npm run build
npx tauri build            # 本地常规构建（无 updater artifacts）
```

产物（`target/release/bundle/`）：

| 产物 | 说明 |
| --- | --- |
| `nsis/ifuyo Aura_<v>_x64-setup.exe` | NSIS per-user 安装器（§3） |
| `*.nsis.zip` + `*.nsis.zip.sig` | updater artifact + minisign 签名（仅 release overlay） |
| `latest.json` | updater manifest（精确版本 artifact URL，§54） |

校验：`node scripts/release-verify.mjs <version>`（存在/非零/版本/架构/
签名/开发者路径残留 §22/§56/§92）。SHA-256 由 release pipeline 生成
`checksums.txt`（§23）。

## 3. Updater 配置（§5–§15 dev/prod 隔离 §77）

- **dev**（`tauri.conf.json` 默认）：updater 公钥=开发密钥对公钥、
  endpoint=`http://127.0.0.1:19988/...`（本地 fake server）；不生成
  updater artifacts（`createUpdaterArtifacts: false`）。
- **prod**（overlay，CI 生成 `target/tauri.release.conf.json`）：
  `createUpdaterArtifacts: true` + 生产公钥（GitHub Secret
  `TAURI_UPDATER_PUBKEY`）+ endpoint
  `https://github.com/<owner>/<repo>/releases/latest/download/latest.json`。
  私钥走 `TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]`（CI-only §26）。
- **Fail-closed（§78）**：`scripts/prepare-release-config.mjs` 在公钥
  缺失时非零退出；本地 dry-run 可用 dev 公钥 + 本地端点。

本地 dry-run（不发布、不用生产密钥 §79）：

```powershell
$env:TAURI_UPDATER_PUBKEY = (Get-Content target\dev-updater.key.pub -Raw).Trim()
$env:TAURI_UPDATER_ENDPOINT = "http://127.0.0.1:19988/updates/{{target}}/{{arch}}/{{current_version}}"
cd ui && npx tauri build --config ../target/tauri.release.conf.json
node ../scripts/release-verify.mjs
```

## 4. Release Pipeline（§18–§21）

`.github/workflows/release.yml`：push `app-v*` →
validate（version:check+changelog §82）→ test（Rust+前端全门禁 §19，
`--locked` §32）→ build（windows-latest §112；secrets 缺失 fail-closed
§78）→ verify（§56）→ **draft release + checksums.txt**（§55：人工
verify 后 publish）→ 发布后核验（§56）。

## 5. 签名体系（两类，§24）

| 体系 | 用途 | 密钥 | 状态 |
| --- | --- | --- | --- |
| Tauri updater signature（minisign ed25519） | updater artifact 完整性/来源 | dev=本地生成（target/，不入库）；prod=GitHub Secret | dev ✅；prod **BLOCKED**（密钥未创建） |
| Windows Authenticode | 安装器/exe 品牌+系统信任 | 证书（OV/EV）+ SignPath/硬件令牌 | **BLOCKED**（证书未申请；政策见 CODE_SIGNING_POLICY.md） |

两者独立，不可互相替代。

## 6. 更新运行时（§34–§48/§70–§76）

- 检查策略：启动延迟 15s、最小间隔 24h、设置开关（默认开）、手动
  检查常在（§8/§84/§85）。
- 安全边界（§109）：远程 manifest → 版本校验（降级拒绝 §47）→
  字段校验 §46 → 签名校验（插件 Rust 侧）→ 官方安装流程。无
  shell/任意 URL/任意执行（§6/§9）。
- 失败语义（§36/§65/§74/§100）：typed 错误分类；失败只影响更新
  功能，播放/库/缓存/Dreamscape/Map 完全不受影响；不阻塞启动。
- 隐私（§44/§45）：更新请求只含版本/平台/架构；零遥测。

## 7. 已知限制 / BLOCKED（§103 失败分类）

- **生产 updater 密钥对未创建** → signed release job 在 CI 上会按
  §78 fail-closed。完成动作：`tauri signer generate`（受控保管）→
  公钥入 Release overlay secret → 私钥入 `TAURI_SIGNING_PRIVATE_KEY`。
- **Windows Authenticode 证书未申请** → 安装器当前未签名（Windows
  SmartScreen 会有提示）。完成动作：购买 OV/EV 证书 → SignPath 或
  signtool 接入 → `WINDOWS_CERTIFICATE` secret。
- GitHub Release 发布未经真实远端验证（无 credentials/远端仓库）——
  pipeline 语法以 actions 官方 v4/v2 为准，发布 job 在真实仓库首跑时
  按输出修正。
- M11-B（channel UI/分阶段发布/更新历史等）全部 NOT COMPLETED。
