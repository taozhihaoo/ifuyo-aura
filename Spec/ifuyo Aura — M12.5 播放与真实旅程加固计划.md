# ifuyo Aura — M12.5 播放与真实旅程加固计划

> 状态：执行中（2026-10-05/06 第一轮：Phase 0–3 完成 + Phase 5 修复×3；Phase 4 soak 已跑，见文末执行记录）
> 背景：M12 后用户首次真实使用暴露 3 个 P0 播放缺陷（5e5fcd6 / d2f4805 /
> 7639f8f），全部属于同一类风险——**逻辑层测试全绿，但"最终适配层 +
> 真实环境"接缝零覆盖**。本计划系统性堵住这一类，而非逐个打补丁。

---

## 1. 方法论

三个已修复 P0 的共性：单测/集成测试用 NullOutput 与模拟回调，验证了
控制面与 DSP 逻辑；真实设备的回调路径、资源生命周期、状态传播从未
被端到端执行。因此补三层，而不是堆单元测试：

| 层 | 现状 | 补法 | 抓住的 bug 类型 |
| --- | --- | --- | --- |
| L1 真实设备集成测试（aura-core `#[ignore]`） | 无 | 真开 CPAL 播合成音；用 `engine.capture()` 对 PCM 断言（静音/来源帧/连续性——不需要人耳） | 流生命周期、状态传播、真实回调时序 |
| L2 应用级旅程探针（CDP 黑盒） | 3 个临时脚本 | 固化为 `scripts/verify-*.ps1` 全旅程套件，输出 M-xx 证据行 | IPC 装配、UI 状态流、跨模块失同步 |
| L3 真实格式/真实数据 fixture | 合成 fixture | 程序生成含 ID3v2.3/2.4、内嵌封面、Unicode 文件名的 mp3/flac（TEST ONLY，无版权问题） | 扫描/元数据/封面/搜索的真实解析路径 |

原则："听得对"可以自动断言——capture 出的 PCM 对比 ramp 来源帧、
静音、gapless 连续性，等价于耳朵。真"听感"仍列手动 QA。

## 2. 嫌疑区全清单（同类风险排查）

### 播放链（已修 3 个，剩余嫌疑）
- [x] 真实 seek：拖进度条 → 音频从新位置出声、显示同步——矩阵 P03 + real_device seek 联动测试；顺带修复 seek 帧空间不换算（44.1k 源时间×48/44.1 错位）
- [x] 曲目播完（EOF）行为：M12.6 曲终检测 + 播放模式自动前进；real_device eof 测试 + 矩阵 P09
- [ ] crossfade/gapless 拼接连续性（capture 断言可验证）
- [x] 音量在真实回调路径生效（矩阵 P06）；EQ/响度 DSP 链已有确定性单测，真实回调逐项断言留待后续轮次
- [ ] Radio（M4）接真实输出播放（fake ICY 服务器已有，从未连真实输出）
- [ ] HTTP 流 + 缓存：播 HTTP → 写缓存 → 断网重放全旅程
- [ ] 播放中设备热插拔（M2 逻辑测过，真实切换未测）
- [ ] SMTC seek / 电台非 seekable 在真实系统面板的行为

### 库与数据链
- [x] 真实格式扫描：ID3v2.3/2.4/无标签/8KB 超长标签/Unicode 文件名/FLAC/OGG——verify-library L01-L08（fixture 由 gen-fixtures.py 生成）
- [x] 封面链路（提取→存库→artwork IPC）：L09 实测 mp3 APIC + FLAC PICTURE → data URL；UI <img> 渲染列手动 QA
- [x] Watcher：watcher_integration 测试在库（event_driven_rescan_preserves_identity 等，含已知 flake 重跑即绿）
- [ ] 播放列表创建/编辑/重启持久

### 设置与生命周期
- [x] 设置重启回环：verify-settings S01-S05（含抓出并修复的写后回读旧值 bug）
- [ ] 升级安装 N→N+1 用户数据保留（M11 §58，当前仅单版本）
- [ ] Mini Player 真实开/关/状态同步

### 长时运行
- [x] Soak：scripts/soak.mjs 30 分钟实跑（采样 JSONL + 泄漏/崩溃判定；结果见执行记录）
- [ ] Windows 睡眠/恢复

## 3. 执行计划（Phase 0–5）

### Phase 0 — 现状体检（先行）
- 三个临时 CDP 探针扩展为 `scripts/verify-playback.ps1`：扫描→播放→
  seek→暂停→恢复→停止→重播→下一曲→音量，输出 PASS/FAIL 矩阵
- 对当前构建实跑，产出"已知问题清单"（预期再抓出真 bug——这正是目的）

### Phase 1 — L1 真实设备测试套件
- `crates/aura-core/tests/real_device.rs`（`#[ignore]`，本机手动跑）
- 播放矩阵：play/seek/pause/stop/eof 自动下一曲/crossfade 连续性/
  音量与 EQ 生效——全部 capture 断言
- 运行：`cargo test -p aura-core --test real_device -- --ignored`

### Phase 2 — L2 旅程探针固化
- `verify-playback.ps1`（含 seek/next/eof）、`verify-library.ps1`
  （真实 fixture 扫描/搜索/封面）、`verify-settings.ps1`（改→重启→
  回读）、扩展 `m10-native-verify.ps1`（tray/SMTC 项）
- 证据行入 docs/PERF.md 或独立证据文档

### Phase 3 — L3 真实格式 fixture
- `scripts/gen-fixtures.py` 生成 mp3/flac 到 `tests/fixtures/`
  （TEST ONLY），供 L2 与 M3 回归使用

### Phase 4 — Soak
- 30 分钟起步（循环：播放→seek→暂停→下一曲→停止→重启），采样内存/
  进程/句柄；按结果决定是否拉长到数小时

### Phase 5 — 修复与回归
- 新发现 bug：P0 优先修，每个修复带回归测试（沿用 M12 模式）
- 完成标准：全旅程矩阵连续两轮全绿、无新增 P0、文档同步

## 4. 明确不做
- 不大规模重构；不拆除 NullOutput（确定性单测保留——缺的是上面的层）
- 不引入需要人耳的验收（capture 断言替代）；真"听感"仍列手动 QA
- 不碰 M13/Steam

## 5. 已完成的同类修复（本计划的前置）
- 5e5fcd6：CPAL 流被 start() 局部变量 drop → 所有权移入 AudioHandle
- d2f4805：device 回调硬编码 Playing → SharedOutputState 状态原子
- 7639f8f：apply_stop 未 seek 解码器 → seek_to_frame(0) + provenance 回归
- 探针工具：examples/cpal_probe.rs、scripts/cdp-{playback,pause}-probe.mjs

---

## 6. 执行记录（2026-10-05/06 第一轮加固）

### 产出
| 层 | 产物 | 结果 |
| --- | --- | --- |
| L1 真实设备 | `crates/aura-core/tests/real_device.rs`（#[ignore]×4） | 4/4 PASS（首跑即抓 harness 状态发布缺失=真实接缝价值自证） |
| L2 播放矩阵 | `scripts/verify-playback.mjs` C01+P01-P09 | 10/10 PASS |
| L2 库矩阵 | `scripts/verify-library.mjs` L01-L13 + `scripts/gen-fixtures.py`（7 fixtures 入库 tests/fixtures/） | 13/13 PASS |
| L2 设置回环 | `scripts/verify-settings.mjs` S01-S05 | 5/5 PASS（修复后） |
| Phase 4 soak | `scripts/soak.mjs` | **30min/446 ops：RSS 40→42MB（噪声级）、句柄 437→437、零操作错误、进程全程存活=PASS** |
| QA 隔离 | lib.rs `AURA_DATA_DIR` 钩子 + `scripts/lib/qa-app.mjs` | 扫描/设置测试零用户数据污染 |

### Phase 5 修复（矩阵/套件实跑抓出，均带回归）
1. **seek 帧空间不换算**（engine）：44.1k 源收 48k 帧号=时间×48/44.1——近端 seek 落超界失败、远端内容与进度错位。回归 `seek_converts_output_frames_to_source_rate`。
2. **library_stats 列名错误**：规范化 schema 后 `COUNT(DISTINCT artist)` 无此列、被 `unwrap_or(0)` 吞成**自 M3 迁移以来恒 0**。修正 + `schema_contract.rs` 契约测试。
3. **设置写后回读旧值**：save_settings 只写盘不更新内存缓存。拆 write_settings（持锁方用，防重入死锁）/save_settings（写盘+同步缓存）。verify-settings 锁行为。

### 教训（流程）
- 一次 release 增量构建因瞬时文件锁失败未被察觉 → QA 脚本跑在**旧 exe**上，把 fixture 扫进了用户真实曲库（已按 path 清理还原）。此后构建后必须验证产物再启动。
- plain `cargo build` 产物引用 devUrl:5173（已知坑 D-075 复发）：QA/探针用 exe 必须 `tauri build` 产物，否则 invoke 全部 "Origin header is not a valid URL"。

### 明确未做（后续轮次）
- crossfade/gapless 拼接的 capture 断言（真实输出上）、Radio/HTTP 流接真实输出、设备热插拔实测、SMTC 真实面板行为、播放列表重启持久、升级 N→N+1、Mini Player、Windows 睡眠/恢复——嫌疑区清单保留未勾
- EQ/响度在真实回调路径的逐项 PCM 断言（DSP 单测已有）
