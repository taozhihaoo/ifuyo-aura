# Fuzz Smoke（M12 §39）

高风险 parser 的 fuzz 策略。当前阶段：**corpus-based smoke**（确定性
固定输入回归，`cargo test` 即跑）；cargo-fuzz（nightly libFuzzer）为
后续任务，targets 已在此登记。

## Fuzz targets（登记）

| target | 解析器 | 入口 | corpus |
| --- | --- | --- | --- |
| url_resolver | M4 resolve_input（scheme/凭据/封锁/私网） | tests/security_m12.rs network_* | 本文件表内字面量 + M4 fixture |
| icy_parser | aura_sources radio StreamTitle | M4 radio 测试 | M4 fixture |
| playlist_parser | M3 播放列表导入 | M3 测试 | M3 fixture |
| updater_manifest | tauri-plugin-updater（上游）+ UpdateService.validateManifest（ui/src/features/update） | update.test.ts + 上游测试 | tests fixtures |
| ipc_payload | tauri 宏类型化反序列化（50 命令） | tauri 框架保证 | — |
| cache_manifest | M5 缓存索引 | M5 测试 | M5 fixture |
| metadata_tags | M3 metadata 归一化 | M3 测试 | M3 fixture |

## 如何运行 smoke

```powershell
cargo test --workspace          # 含全部 corpus 回归
cd ui && npx vitest run         # 含 validateManifest fixtures
```

## 升级路径（§39）

引入 cargo-fuzz：`cargo fuzz init` + 为 url_resolver/icy_parser 建立
`fuzz/fuzz_targets/`，CI 每跑 5 分钟 smoke（nightly job）。
