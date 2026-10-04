# CODE_SIGNING_POLICY

> 状态：政策基线（Phase 0）。本阶段不请求真实签名证书，不创建任何 signing secret。

## 目标流程（M11/M13 落地）

- 发行渠道：GitHub Actions（Windows runner）构建唯一 release artifact。
- 签名服务：准备接入 SignPath Foundation / SignPath OSS 流程。
- **CI-only signing**：只有 CI 产出的 artifact 允许被签名；本地构建一律不签名、不发布。
- **私钥管理**：签名证书与 updater 私钥只存在于 CI Secret（GitHub Secrets / SignPath），**绝不进入仓库**；缺少 signing configuration 时构建必须优雅跳过签名步骤，不得失败。
- **2FA**：SignPath 与 GitHub 账号必须启用两步验证。
- 责任角色：release 触发（tag push）与签名审批分离；artifact 哈希记录进 Release 说明。

## 当前阶段（Phase 0）

- 不申请证书。
- 不创建任何真实 signing secret。
- CI workflow 中预留签名 job 的位置但不启用。

---

## M11 落地状态（2026-10-05）

### 两类签名（不可互换 §24）

| 体系 | 用途 | 密钥/证书 | 当前状态 |
| --- | --- | --- | --- |
| Tauri updater signature（minisign ed25519） | updater artifact 完整性与来源 | dev 密钥对=本地生成（`target/dev-updater.key`，gitignored，TEST ONLY）；prod=**未创建**（GitHub Secret 计划：`TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]` + `TAURI_UPDATER_PUBKEY`） | dev ✅ / prod **BLOCKED** |
| Windows Authenticode | 安装器与 exe 的系统信任 | OV/EV 证书（SignPath 或硬件令牌） | **BLOCKED**（未申请；政策同 Phase 0 基线） |

### 私钥政策（§10/§11/§26）

- updater 私钥：**绝不**入仓库/前端/打包产物/CI artifact/日志；只存
  GitHub Secrets（CI-only signing §基线）或受控离线保管。
- dev/prod 密钥严格分离（§12）：dev 密钥仅用于本地 dry-run（端点=
  本地 fake server），**不得**给任何公开 artifact 签名。
- 密钥轮换：updater 公钥内置于已发布客户端——轮换=双公钥过渡期
  （未来 release overlay 支持 pubkeys 数组）；丢失私钥的后果=已发布
  客户端无法收到更新，需走"重新安装"恢复路径（incident response：
  撤销 endpoint manifest + 发布紧急安装器）。
- CI 日志（§27）：记录 artifact 名/版本/证书身份（是否签名）；不回显
  任何 secret 值。

### Fail-closed（§78）

release workflow 构建前检查 secrets；缺失 → signed job 显式 FAIL。
unsigned dry-run（本地/PR）用 dev 公钥或完全不生成 updater artifacts。

### 剩余 BLOCKED 项

- 生产 updater 密钥对未创建（无生产发布渠道）。
- Windows Authenticode 证书未申请。
- 上述任一就位后：接入 secret → 首个 signed tag release → 按
  docs/RELEASE_CHECKLIST.md 验证。