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
