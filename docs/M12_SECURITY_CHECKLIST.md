# M12 Security Checklist（§61）

> 每次发布前逐项核对。证据列注明验证方式：
> RUNTIME VERIFIED / STATICALLY VERIFIED / MANUAL VERIFICATION REQUIRED。

## Before Release

- [ ] no secrets — gitleaks CI 绿 + §43 history scan（RUNTIME VERIFIED 2026-10-05）
- [ ] no private signing key — dev 密钥在 gitignored target/；生产密钥不存在（STATICALLY VERIFIED）
- [ ] updater public key matches production key — **MANUAL VERIFICATION REQUIRED**（生产密钥未创建）
- [ ] installer identity correct — 注册表 DisplayName/Version 0.1.0（RUNTIME VERIFIED）
- [ ] licenses verified — cargo deny licenses ok（RUNTIME VERIFIED）
- [ ] SBOM generated — cargo tree/npm ls 命令就绪（docs/SUPPLY_CHAIN.md）；正式 SBOM 文件 NOT COMPLETED
- [ ] CSP reviewed — script-src 'self' 无 unsafe-*；connect-src 仅 ipc:（STATICALLY VERIFIED）
- [ ] capabilities reviewed — 最小集（core:default+显式 window/webview/dialog/updater/process §62）（STATICALLY VERIFIED）
- [ ] IPC reviewed — 50 命令全 typed；security_m12 回归（STATICALLY VERIFIED + RUNTIME 回归）
- [ ] SSRF reviewed — M4 策略+封锁表+凭据拒绝（RUNTIME VERIFIED M4 测试套+security_m12）
- [ ] path traversal reviewed — resolver 存在性校验+artwork 无用户路径（security_m12）
- [ ] metadata injection reviewed — sanitize 限长/控制字符（security_m12 §252）
- [ ] artwork limits reviewed — M3 artwork store 字节通道+悬空引用安全（M3 测试）
- [ ] cache boundaries reviewed — M5 策略/完整性测试（M5 回归）
- [ ] updater verified — 签名验证/降级拒绝/同版本 null（RUNTIME VERIFIED via fake server + 单测）
- [ ] privacy verified — 零遥测 grep+文档（§44/§45）
- [ ] public repo checked — git log 全 history 扫描无真实密钥（§43，RUNTIME VERIFIED）
- [ ] release artifacts scanned — release-verify.mjs（§22/§92：签名/版本/路径残留）

## Windows 手动 QA（§336/§74——本环境不可自动化项）

- [ ] 媒体键按下 → SMTC 面板状态正确
- [ ] 托盘菜单逐项点击
- [ ] DPI 100/125/150/200% 下托盘图标与主窗
- [ ] 多显示器拖动与恢复
- [ ] 深/浅主题下托盘图标可辨
- [ ] Windows 10 实机（当前仅 Win11 实测）
