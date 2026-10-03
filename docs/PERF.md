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
