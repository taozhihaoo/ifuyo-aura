# M11 Release Checklist（§102）

每次正式发布（`app-vX.Y.Z` tag）逐项核对。任何一项不过 = 不发布（§21 fail-closed）。

## 版本与变更
- [ ] `npm run version:check` PASS（ui=tauri=cargo=brand=tag §A1）
- [ ] CHANGELOG 含 `X.Y.Z` 版本段（§82）
- [ ] 版本号符合 MAJOR.MINOR.PATCH（§A1）

## 质量
- [ ] cargo fmt --check PASS
- [ ] cargo clippy --workspace --all-targets -- -D warnings PASS
- [ ] cargo test --workspace --locked PASS
- [ ] cargo deny check PASS（§88 无 GPL/AGPL/LGPL）
- [ ] ui: typecheck / lint / test / build PASS（§19）
- [ ] gitleaks secret scan PASS（§29）

## 构建与产物（§22/§23）
- [ ] NSIS installer 生成：`ifuyo Aura_X.Y.Z_x64-setup.exe`（§3 命名）
- [ ] updater artifact 生成：`.nsis.zip` + `.nsis.zip.sig`（§13）
- [ ] updater manifest（latest.json）有效（§14/§46）
- [ ] SHA-256 checksums.txt 生成（§23）
- [ ] release-verify PASS（§22/§56/§92 无开发者路径残留 §91）
- [ ] binary audit：无 debug 运行时/测试二进制/私钥/.env（§90）

## 签名（§24–§27）
- [ ] updater signature 验证 PASS（minisign 公钥匹配）
- [ ] Windows Authenticode 有效（证书可用时；否则如实 BLOCKED §25）

## 安装与升级（§57/§58）
- [ ] installer smoke PASS（静默装→启动→退出→卸载）
- [ ] 卸载后用户数据保留（§3/§4）
- [ ] upgrade smoke：N → N+1 数据保留（Library/Playlist/Settings/Analysis/Dreamscape/Map/Cache §58）

## 更新（§59/§97）
- [ ] updater integration test PASS（合法/坏签名/降级/超时/500/损坏 §59）
- [ ] 更新 soak 无泄漏/无重复监听（§97/§98）

## 发布（§53–§56）
- [ ] release notes 就绪（区分 Added/Changed/Fixed/Security/Known Issues §33）
- [ ] GitHub Draft Release 附件齐全（installer/updater/latest.json/checksums §53）
- [ ] 人工验证 draft 后 publish（§55）
- [ ] 发布后核验（public URL/checksum/version §56）
