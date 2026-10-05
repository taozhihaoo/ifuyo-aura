# SUPPLY_CHAIN（M12 §24/§40/§59/§88）

## 工具与最近一次结果（RUNTIME VERIFIED，2026-10-05）

| 工具 | 范围 | 结果 | 分类 |
| --- | --- | --- | --- |
| cargo audit 0.18.5 | Cargo.lock 608 crates | **0 vulnerabilities**；3 warnings（paste/proc-macro-error unmaintained=传递构建宏依赖；glib unsound=Linux GTK 栈，Windows 产物不含） | 无 P0；warnings 接受（无漏洞、无 Windows 影响） |
| cargo deny check | advisories/bans/licenses/sources | 全 ok | — |
| npm audit | ui 依赖树 | 5 项（1 critical/1 high/3 moderate）——**全部 dev 工具链**（vite/vitest/esbuild/@vitest/mocker），不进 Tauri 产物 | 见下分类 |

## npm audit 分类（§40 表）

| 依赖 | 漏洞 | Reachable? | Production? | 决策 |
| --- | --- | --- | --- | --- |
| vitest <3.2.6（critical，UI server 文件读取） | GHSA-5xrq-8626-4rwp | 否——仅 `vitest --ui` 模式；项目用 `vitest run` 无 UI | 否（dev-only） | **DEFER**：升 vitest 4 = 破坏性（测试 API 变更），列入独立任务 |
| vite ≤6.4.2（high/moderate，dev server） | GHSA-4w7w… | 否——仅 `npm run dev` 本地 localhost；生产 bundle 不含 vite | 否 | **DEFER**：vite 6 升级随 M11-B/独立任务 |
| esbuild ≤0.24.2（moderate，dev server） | GHSA-67mh | 否——同上 dev server | 否 | 同上 |
| @vitest/mocker / vite-node | 同 vitest | 否 | 否 | 同上 |

结论：**无生产可达漏洞**。DEFER 项为 dev-only + 需破坏性升级；已完成
非破坏性 `npm audit fix` 部分（本次 `npm audit fix` 无 --force，升级了
锁内可修复项）。跟踪：升级 vite6/vitest4 独立任务。

## SBOM（lite，§59/§47）

生成方式（可重复）：

```powershell
cargo tree --workspace --locked > sbom-cargo.txt   # Rust 全树
cd ui && npm ls --all > ../sbom-npm.txt            # 前端全树
```

正式 SBOM（CycloneDX/SPDX）= M11-B/未来任务（NOT COMPLETED，不伪造）。

## 策略（§59 License Policy）

- Allowed：MIT / Apache-2.0 / BSD / ISC / MPL-2.0 / Zlib /
  Unicode-3.0 / CDLA-Permissive-2.0 / OFL-1.1 / CC0
- Review：双许可组合变更、custom、NOASSERTION（当前依赖树无）
- Blocked：GPL / AGPL / LGPL（cargo deny bans 强制 §372）

新增依赖流程：§378 清单（license/MSRV/Windows/binary size/maintenance/
CI）→ THIRD_PARTY_LICENSES.md 登记 → deny 绿 → 提交。
