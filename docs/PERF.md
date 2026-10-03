# PERF — 性能基线

> 原则（Charter §50）：所有性能必须实测，禁止伪造。每个里程碑在此追加/复核。

## 测量环境

| 项 | 值 |
|---|---|
| 日期 | 2026-10-03 |
| CPU | AMD Ryzen 7 7700 8-Core（release 构建单线程测量） |
| 内存 | 32 GB |
| OS | Windows 11 (10.0.26200) x64 |
| Rust | rustc 1.99.0 (b940084d7 2026-09-28)，stable-msvc |
| 代码版本 | M1（commit 0ee601f 之后的 M1-h 文档提交；测量时工作树与 0ee601f 一致） |
| 测量命令 | `cargo test -p aura-core --release --test perf_baseline -- --ignored --nocapture` |
| 测试源 | `crates/aura-core/tests/perf_baseline.rs`（`#[ignore]`，不进常规门禁） |

## M1 基线（release 构建，单线程）

| 项目 | 结果 | 说明 |
|---|---|---|
| SPSC ring buffer 吞吐 | ≈ 5.07 × 10⁹ 帧/秒（1024 帧块循环推取，204.8M 帧 / 40.4ms） | 远超实时需求（48kHz × 容忍 10ms 块 = 每秒 48k 帧） |
| FLAC 解码吞吐 | **3668× 实时**（20 段 1 秒 48k 单声道音频，5.45ms） | 目标 §50：≥15×/核 ✅ 大幅超出 |
| 重采样 44.1k→48k 吞吐 | **285.6× 实时**（60 秒等效输入，210ms，stereo） | rubato sinc 256 阶 |
| NullOutput 渲染吞吐 | 199×10⁶ 帧等效/秒（2.88M 帧合成 ramp，14.4ms）| 合成源（RampDecoder）路径；真实解码为主时以上行 decode 为准 |
| 回调平均耗时 | **5.1 µs / 1024 帧块**（48kHz 下块预算 21.3ms） | 预算占用 ≈ 0.02%，余量充足 |

## 解读与后续

- 回调路径 5.1µs/块：到 M2 加入全 DSP 链（loudness + EQ + crossfade）后需复测，仍应远低于预算。
- 重采样 285×：M6+ 设备格式协商接入后如需更高档位（sinc_len 512）需复测。
- FLAC 解码 3668×：单声道 48k 正弦夹具；多声道/高码率文件复测于 M3（真实库扫描）与 M6。
- 本文件所有数字可由上述命令在本机复现。

## M2 基线（release 构建，单线程；2026-10-03，commit M2-f）

| 项目 | 结果 | 说明 |
|---|---|---|
| ring buffer 吞吐 | ≈ 5.00 × 10⁹ 帧/秒 | 与 M1 持平 |
| FLAC 解码吞吐 | 3607× 实时 | 与 M1 持平 |
| 重采样 44.1k→48k | 275.9× 实时 | into-buffer API 后，与 M1 持平 |
| NullOutput 渲染（M1 基线路径） | 24×10⁶ 帧等效/秒（120ms/2.88M 帧） | 合成 ramp 源 |
| **全 DSP 回调（M2 §63）** | **avg 44.6µs / p95 49.3µs / p99 54.6µs / worst 4.33ms**（1024 帧块） | crossfade 0.5s + loudness(+4dB) + Bass Boost EQ + limiter + volume 全开；48kHz 块预算 21.3ms → **预算占用 ≈ 0.21%** |
| 1h 等效 soak | PASS（release，8.3s 真实耗时跑完 3600 轨 A/B 循环） | 无 panic/漂移/无效 PCM（§64–65） |
| 设备恢复成本 | O(格式重建)：重采样器重建 + 渲染缓冲重分配（µs 级） | 无真实声卡时序测量，M10 补充 |

### 复测命令

```text
cargo test -p aura-core --release --test perf_baseline -- --ignored --nocapture
cargo test -p aura-core --release --test m2_integration_v1 -- --ignored --nocapture
```

## M3 基线（release 构建；2026-10-03，M3-n）

机器/环境：Windows 11 x64（10.0.26200）、rustc 1.99.0、SQLite bundled（WAL，synchronous=NORMAL）、fixture 为 1.3KB 合成 FLAC（8kHz mono 0.25s，标签各异）。

| 项目 | 结果 | 说明 |
|---|---|---|
| 初始扫描（1000 文件） | **287ms（3484 文件/s）**，imported=1000 | walk + lofty 标签 + head/tail 1MiB 身份 + FTS 索引 + 事务提交，全链路 |
| 增量重扫（1000 文件） | **25.4ms**（skipped=1000） | size+mtime 未变 → 零解析零写库（§17），≈40× 加速 |
| 10k 行批量导入（含 FTS） | 11,778 rows/s（849ms） | 单事务 upsert_track 全路径 |
| 10k 库搜索延迟 | **avg 1.11ms / worst 2.13ms** | FTS5 bm25 加权 + 前缀查询（§79 交互级 ✓） |
| 10k 库 list_tracks 分页 | 122ms / 10k 行（500/页） | 实体 join + 排序 |
| metadata 提取 | 39.8 µs/文件 | parse_file（lofty + 规范化） |
| 内容身份（5MB 文件） | 1.49 ms/文件 | size + head/tail 1MiB SipHash-2-4-128 采样 |
| 缩略图 512→256 | 2.46 ms/张 | 解码（受限）+ box-filter + PNG 编码 |

### 解读

- §79 目标核对：10,000 首库可正常查询 ✓（搜索 avg 1.1ms）；10k 文件扫描可接受 ✓（微小夹具 2.9s；真实库瓶颈是身份采样 1.49ms/5MB，增量扫描后不再发生）。
- 身份采样吞吐 ≈ 1.4 GB/s（SipHash scalar）；若未来真实库首扫成为痛点，可评估并行化（扫描在后台线程，不阻塞 UI）。
- 全部数字可复现：

```text
cargo test -p aura-library --release --test perf_baseline -- --ignored --nocapture
```

## M4 基线（release 构建；2026-10-04，M4-i）

环境：同 M3（Windows 11 x64、rustc 1.99.0）；HTTP 数字为 127.0.0.1 fake server 回环。

| 项目 | 结果 | 说明 |
|---|---|---|
| 本地 resolve | **65.9 µs/次** | 存在性 + 扩展名 + aura-library 标签解析 |
| URL 校验 | **0.96 µs/次** | parse + scheme/凭据/封锁表/私网全套 |
| 直链 HTTP resolve（回环） | 5.48 ms/次 | GET + 头探测（网络 RTT 主导；公网来源取决于远端） |
| ICY 剥离吞吐 | **1818 MB/s**（1MB 流 579µs） | IcyReader 流式剥离 + 元数据解析 |
| Radio Browser 解析 | 3.5 µs/条（2000 条电台） | serde 逐条容错解析 |
| servers 解析 | 0.2 µs/条（500 条） | 同上 |
| live 缓冲上限 | 128kbps→480KB；320kbps→1.2MB；未知→256KB；finite→1MiB | min(码率×30s, 2MiB)（§37/D-051） |

### 复测命令

```text
cargo test -p aura-sources --release --test perf_baseline -- --ignored --nocapture
```

## M5 基线（release 构建；2026-10-04，M5-i）

环境：同 M4（Windows 11 x64、rustc 1.99.0、SQLite bundled WAL）。

| 项目 | 结果 | 说明 |
|---|---|---|
| 256KiB chunk 写 + DB 记录 | **244 MB/s**（100 块 107.6ms） | tmp 写 + rename + hash + 事务记录全链路 |
| 256KiB chunk 读 | **1423 MB/s**（100 块 18.4ms） | 文件读 + 大小校验 |
| SHA-256 校验 | **2429 MB/s** | integrity/写入路径 |
| SQLite chunk lookup | **6.68 µs/次**（10k 次 66.8ms） | 主键查询 |
| LRU 扫描 | 13.4ms/100 次（600 行表） | 排序查询 |
| cache reopen | 9.2ms | WAL 打开 + 迁移检查 |
| integrity 扫描（150 chunk + 450 missing） | 120ms | hash 全量校验 + 修复 |

### §90 目标核对

- cache hit 明显快于重新下载 ✓（读 1423 MB/s vs 回环 HTTP fetch ~5.5ms/请求起）
- DB lookup 6.7µs、verification/eviction 均不在 audio callback 路径（结构上：cache 层在 decode 之前的 producer 侧，§91）
- 实测记录，无伪造（§130 不以关 integrity/降 hash 换性能）

### 复测命令

```text
cargo test -p aura-cache --release --test perf_baseline -- --ignored --nocapture
```
