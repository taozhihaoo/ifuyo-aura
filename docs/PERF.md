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

## M6 UI（2026-10-05）

- 硬件：AMD Ryzen 7 7700（8C/16T）/ 31.2 GB RAM；GPU：虚拟显示适配器（GameViewer Virtual Display Adapter，远程会话）
- OS：Windows 10.0.26200（25H2 insider 内核）x64
- 构建：`cargo build --release -p aura-shell --features custom-protocol`（内嵌 ui/dist，优化构建）与 debug 构建对照
- 方法：cargo test 计时（`--ignored --nocapture`，Instant 环绕 50 次取均值/worst）；smoke bin 进程内计时；前端 250ms 快照轮询 + rAF 插值不经 Zustand（结构上排除全树重渲）

| 项 | 实测 | 说明 |
| --- | --- | --- |
| M6 UI Startup（进程起→setup 完成可查询） | 560.6ms（debug） | audio 设备打开 + SQLite 打开/迁移为主；release 更快（未单列） |
| player_snapshot 读取 | 188ns/call（100 次均值，debug） | 10Hz 驱动线程写 + UI 250ms 轮询读 |
| Library 10k 行 list_tracks（200/页×50 页） | avg 6.62ms / worst 10.15ms | UI 单页渲染量 200 行 + @tanstack/react-virtual 虚拟化 |
| Search latency（10k 行 FTS，UI debounce 250ms 后） | avg 1.98ms / worst 3.14ms | 4 组查询（中英/前缀/编号） |
| artwork/track_path 点查 | 4.99µs | play_track 管线的路径解析步 |
| Command Palette open | 未单列 | 纯前端（React 状态 + 预构建命令表）；vitest 渲染 <5ms |
| Aurora FPS | 目标 30（playing）/ 8（idle）/ 2（reduced-motion）；运行时实测经 `canvas[data-fps]` 暴露（2s 窗口均值） | 本会话为远程虚拟显示（无真实 GPU/合成器），绝对帧率不具代表性，故记录机制而非数字 |
| Mini Player sync | 与主窗同一 Rust 快照轮询（250ms），无独立第二引擎（§236/§237） | 结构保证 |
| Window resize | 150ms debounce（§136）；内部分辨率 cap dpr≤1.5（§134） | |
| Memory baseline | 未采样长期驻留（远程会话无常驻桌面，长时间 soak 不适合本环境，§296 如实声明） | soak 脚本见 PROGRESS 已知限制 |

复测命令：

```text
cargo test -p aura-shell --test perf_ui -- --ignored --nocapture
cargo run -p aura-shell --bin smoke   # startup-to-ready / snapshot 行
```

诚实声明（§204）：以上均为真实测量；无通过关 Aurora/不渲染 artwork/假数据制造的“好性能”。Aurora FPS 绝对值因远程虚拟显示环境不记录（机制已内建，实机运行时 data-fps 可直接读取）。

## M7 Dreamscape（2026-10-05）

- 硬件/OS：同 M6 节（Ryzen 7 7700 / 31.2GB / Win10.0.26200；远程虚拟显示）
- 构建：`cargo test -p aura-synth --test perf_generation -- --ignored --nocapture`（debug profile）；前端 vitest 计时
- 方法：Instant 环绕 ×100–1000 取均值；前端 engine 决策均为 O(1) 纯逻辑

| 项 | 实测 | 说明 |
| --- | --- | --- |
| seed→genome | 475ns/call | world 流 8 字段派生 |
| world 生成（default） | 9.90ms/call（objects=679, landmarks=4） | placement 坡度/排除半径 + spatial hash |
| world 生成（dense genome） | 10.77ms/call（objects=748） | 预算帽内 |
| manifest serialize | 995.7µs/call（31,365 bytes JSON） | §115 有界 |
| manifest deserialize | 813.7µs/call | validation-first 守卫在 UI 侧另计 |
| manifest scene_hash | 60.3µs/call | 规范字节序 SipHash |
| snapshot save（capture+seal+JSON） | 124.6µs/call | §225 原子写路径 |
| snapshot load（parse+validate+restore） | 110.5µs/call | 含 world_id 一致性校验 |
| 渲染帧率目标 | 播放 30 / idle 10 / 隐藏 2 / reducedMotion 5（fps） | §144–147；远程虚拟显示无真实 GPU，绝对 FPS 不采样（data-fps 可实测） |
| Auto 质量 | 升档 >50fps×3s / 降档 <22fps×2s / 冷却 4s（§143） | engine 单测锁定 |

复测命令见 perf_generation.rs 头注释。诚实声明（§204）：真实测量；渲染 FPS 绝对值因远程虚拟显示不记录（机制内建 data-fps）。

## M8 分析（2026-10-05）

- 构建：debug profile（`cargo test -p aura-analysis --test perf_analysis -- --ignored --nocapture`）——**debug 数字偏慢**，release 会显著更快；记录真实状态 §204。
- 硬件/OS 同 M6 节。

| 项 | 实测（debug） | 说明 |
| --- | --- | --- |
| BPM time（30s 音） | 1.11 s | flux onset+分数周期 comb |
| Key time（30s 音） | 0.94 s | chroma 4096-FFT + 24 假设 Pearson |
| LUFS time（30s 音） | 2.67 s | K-weight 滤波+门控 |
| MFCC time（30s 音） | 1.13 s | mel26+DCT |
| Embedding time（30s 音） | 26.2 s | **O(N²) DFT 谱统计嵌入——已知热点**（中篇记录；下篇/后续换 FFT 可数量级加速，升 model_version） |
| 端到端 30s 音 | 32.4 s | 合成节奏 WAV 全链 |
| 端到端 180s 音 | 123.1 s | 线性于时长（§303）；内存 O(1)（窗口+hop 累加） |
| 入队 10k | 222.9 ms | 调度器去重+优先级 |
| dequeue 延迟 | 248.5 µs | |
| 存储每首 | ≈532 B（mfcc 104B + 嵌入 136B + 标量≈300B） | §218/§219/§220：10k ≈ 5.1 MB——远低于"数 MB/首"红线 |
| 批量查询 | 100/1000 行 ms 级（§221） | InMemory SQL |

复测命令：`cargo test -p aura-analysis --test perf_analysis -- --ignored --nocapture`

诚实声明（§204/§326）：全部真实测量；debug profile 数字不用于宣称 release 性能（§326 无营销指标）。Embedding 的 O(N²) DFT 是已识别热点，修复路径=换 FFT+升 model_version（D-095）。

## M9 Music Map（下篇 §297/§298/§474–§478，2026-10-05）

- 硬件（§475）：AMD Ryzen 7 7700（8C/16T）/ 31.2 GB RAM / AMD Radeon RX 7700 XT（+ iGPU）/ Windows 11（10.0.26200）/ **release profile** / rustc 1.99.0 / Node v24.19.0。
- 数据集（§476/§477）：确定性合成 4 簇、嵌入维度 D=32（M8 实际维度；测试 fixture 34 维槽位）——`tests/perf_map.rs` xorshift 微扰，无生产随机源。
- 复测：`cargo test -p aura-map --release --test perf_map -- --ignored --nocapture`

### 后端全链（§474 Backend，release 实测）

| 阶段 | 1k | 4k | 10k |
| --- | --- | --- | --- |
| dataset build（含向量校验/hash） | 0.5 ms | 2.3 ms | 5.8 ms |
| kNN graph（exact scan，K=12） | 93.9 ms | 1.45 s | 9.74 s |
| clustering（LPA 20 轮） | 2.2 ms | 8.5 ms | 23.6 ms |
| layout（PCA+grid，150 iter） | 89 ms | 1.41 s | 9.15 s |
| persistence（serde+checksum） | 1.4 ms | 4.6 ms | 14.7 ms |
| restore（免重算恢复 §174） | 2.3 ms | 9.0 ms | 34.1 ms |
| incremental relax（种子冻结 40 iter §277） | 26 ms | 0.40 s | 2.36 s |

### 查询（§298，10k 节点）

| 查询 | 实测 |
| --- | --- |
| top-k neighbor（K=8，exact） | 1.13 ms/call |
| viewport query（BTreeMap 扫描） | 86.8 µs |
| restore 后 detail/cluster 数据 | 内存查表（µs 级） |

### 内存核算（§291/§295/§478，账面值）

| N | 向量 (N×D×4) | 边 (≈12N×20B) | 位置 | 簇 |
| --- | --- | --- | --- | --- |
| 1k | 0.13 MB | 0.22 MB | ≈8 KB | ≈4 KB |
| 10k | 1.3 MB | 2.2 MB | ≈80 KB | ≈40 KB |

- §292 无四份复制：SQLite BLOB → 采集期一次性入 dataset（构建线程局部）→ 快照只存 positions/clusters（f32 对）；前端只持 positions typed arrays + 按需元数据，不重复持有向量。
- 大库（§290/§393）：>25k 节点 build_knn_graph 显式 `DatasetTooLarge` 拒绝（测试锁定）——B 级 ANN/progressive 未引入，不伪造可扩展性。

### 前端（§474 Frontend/§299）

- 渲染 = Canvas2D 按需重绘（§197/§198：静止 0 rAF）；无逐帧渲染循环 → FPS 指标不适用（无持续动画；focus 动画 240ms 即停）。
- 单帧重绘 10k 视口节点 = 空间 hash culling 后仅绘制可见子集（§143/§300 可视预算）；可视边 ≤30（§301）；far 缩放优先簇晕（§302）。
- 帧时间/GPU 内存（§299 的浏览器级指标）：本环境无 WebGL 剖析管线——**如实标注未测**；dpr cap ≤2（§326）控制填充率。
- progressive render（§304/§305）：首次打开 = status→overview→loadAll（真实加载驱动），无定时器假 loading。

诚实声明：全部真实 release 测量；kNN graph 与 layout 为 O(N²·K) exact——10k≈9s debug 级别可用、25k 上限明确拒绝。后续 B 级路径（ANN/近似布局）引入前须按 §378 审计依赖并重测。
## M10 Windows Integration（§291–§294/§398，2026-10-05）

- 硬件同 M9 节（Ryzen 7 7700 / 31.2 GB / Windows 11 / tauri build release）。

| 项 | 实测 | 说明 |
| --- | --- | --- |
| Single-instance 第二启动退出延迟 | 102–120 ms | 含进程启动+互斥检测+退出（M10-SINGLE-02 脚本实测多次） |
| 应用启动到可用（smoke startup-to-ready） | 421–606 ms | 含 audio/库/dreamscape/map/集成 worker/托盘全部初始化 |
| integration worker 空闲成本 | 2 Hz 快照轮询 + 事件驱动 | §180/§181：无媒体活动时仅 500ms recv_timeout，无忙循环 §145 |
| SMTC position tick | 500 ms 间隔（D-124），仅 Playing | §182/§183：暂停即停 |
| SMTC 初始化/元数据/timeline 单次调用耗时 | NOT MEASURED | WinRT 调用级计时未插桩（下篇如需可加诊断时钟）；失败均入 diagnostics §225 |
| Tray creation / menu update | NOT MEASURED（单次创建+低频 set_text） | §91/§92：仅状态变化时更新 |
| 长时运行内存（§295/§354） | NOT MEASURED（数小时 soak 未执行——如实声明） | 结构上：worker 持单快照+有界通道（§187），无累积容器 |

复测：`scripts/m10-native-verify.ps1`（M10-* 证据行）。