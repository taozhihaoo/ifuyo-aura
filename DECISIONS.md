# DECISIONS

工程决策记录。新增决策追加编号，不覆写既有条目；推翻旧决策时保留原文并追加新条目说明。

## D-001 Product Name

正式产品名：

`ifuyo Aura`

理由：

产品名称直接绑定 ifuyo 品牌与视觉核心 Aura，便于未来商业化。

> 2026-10-02 章程变更：产品名由 `IFUYO Music`（内部工程代号 `Aura`）正式更名为 `ifuyo Aura`。原内部代号并入正式产品名称，不再作为独立工程代号使用。

## D-002 Technology

Tauri 2 + Rust + React + TypeScript + Three.js。

理由：Charter §5 固定技术栈；Tauri 2 提供最小体积与原生集成，Rust 承载音频与领域内核。

## D-003 Platform

Windows-first。

理由：官方首发目标为 Windows（WASAPI / SMTC / Mica）；代码保持可移植性，但非 Windows 平台不承诺官方支持。

## D-004 Privacy

No telemetry。

理由：Charter §33 零遥测是产品身份的一部分，商业化也不能改变。

## D-005 Content

No commercial-platform extraction。

理由：合法性优先（Charter 产品原则第 1 条）。只接受本地与合法开放来源。

## D-006 Licensing

默认 MIT OR Apache-2.0 双许可。

理由：最大兼容未来商业发行与开源协作；依赖侧禁止 GPL / AGPL / LGPL（Charter §7）。

## D-007 Codec

官方发行版 codec 策略按总章程执行：优先 MP3 / FLAC / Vorbis / WAV-PCM / OGG / MKV-WebM / ALAC；AAC 默认经 Windows Media Foundation，官方构建不内置 AAC decoder，自行编译可启用 `aac-builtin` feature。

## D-008 Website

备案信息不属于 ifuyo Aura 发布页要求，不实现相关字段或展示。

## D-009 Scope

M0→M14，逐阶段实施；一次只做一个 Milestone，门禁全绿才 commit。

## D-010 src-tauri 接线时机（2026-10-02 建立，2026-10-03 已执行）

~~`src-tauri/` 在 Phase 0 仅建立脚手架（目录、`tauri.conf.json`、入口源码），**暂不加入 Cargo workspace members、不引入 tauri 依赖编译**。~~

**已落地（M0-3，2026-10-03）**：`src-tauri` 加入 workspace members，tauri 2.12 依赖树经 `cargo check/clippy/test` 全绿。占位图标集（`src-tauri/icons/`，品牌色占位图形）由 `@tauri-apps/cli icon` 生成，正式品牌 logo 按 BRAND.md 在 M6 前替换。`ui/dist/.gitkeep` 保证 fresh clone 上 `frontendDist` 路径存在。完整 `tauri build` 打包门禁仍待 UI（M6）激活。

理由（原始）：Phase 0 禁止实现 M1 之后业务功能，且本机 Rust 工具链未安装；提前把 tauri 拉进 workspace 会让 `cargo test` 拖入数百个依赖，违背"不得添加未必要的生产依赖"（§18）。M0 建立 IPC contracts 时再把 `src-tauri` 加入 members 并锁定 Tauri 2 版本。

## D-011 MSRV 与本机工具链（2026-10-02 建立，2026-10-03 更新）

- **工具链（2026-10-03 就绪）**：本机安装 VS Build Tools 2022（MSVC 14.44）+ rustup stable `x86_64-pc-windows-msvc`（rustc 1.99.0），`rust-toolchain.toml` 锁定 channel=stable，与 CI windows-latest 目标一致。首次 `cargo fmt --check` / `clippy -D warnings` / `test --workspace` 已全绿。
- **MSRV**：`1.87`（2026-10-03 以真实 1.87 工具链全 workspace check 验证通过；由 rubato 5.0.1 抬高，最初声明的 1.85 不成立）。

## D-013 M1 音频依赖锁定（2026-10-03）

`aura-core` 锁定 M1 四件套（crates.io 当前稳定版）：

| crate | 版本 | 用途 |
|---|---|---|
| `symphonia` | 0.6.1 | 解码；默认特性全开，覆盖 Charter §36 格式矩阵（M1 实装时可裁剪特性） |
| `cpal` | 0.18.2 | 音频输出（Windows WASAPI shared） |
| `rubato` | 5.0.1 | 重采样 |
| `rtrb` | 0.4.0 | lock-free SPSC ring buffer |

- `cargo deny check licenses bans sources` 全绿（127 包依赖树，无 GPL/AGPL/LGPL）。
- `cargo deny check advisories` 本机因无法连接 github.com（RustSec advisory-db clone 超时）NOT RUN，由 CI 执行。
- 只锁定到 minor 版本（如 `"0.6.1"`），`Cargo.lock` 精确固定；升级走显式决策。
- 备注：Windows-first 项目固定 MSVC 目标；不使用 windows-gnu。

## D-012 品牌色进入 brand.json（2026-10-02）

`brand.json` 除 Phase 0 §3 的四个必填字段外，附加 `version` / `copyright` / `colors`（bg、rose、violet、sky、mint），以满足总章程 §4.1 对 brand.json 的完整要求；`ui/src/design/tokens.ts` 与生成的 CSS variables 均以 `brand.json` 为单一事实来源，`scripts/generate-tokens.mjs` 负责生成与一致性校验。

## D-014 SPSC Ring Buffer 选型（2026-10-03，M1）

`rtrb` 0.4（M0 已锁定），外层包装帧语义 `AudioRingBuffer`（ProducerHandle/ConsumerHandle）。SPSC 由类型系统强制（句柄不可 Clone）。generation-aware 丢弃在 consumer 侧执行：producer bump 时以原子量记录缓冲中旧代帧数，consumer 检测到代切换后**精确丢弃**这些帧（不波及 bump 后写入的新代数据）。

## D-015 Decoder 选型（2026-10-03，M1）

直接使用 `symphonia_bundle_flac` 的具体类型 `FlacReader`/`FlacDecoder`（不经通用 probe/registry 层）。理由：M1 只支持 FLAC，具体类型 API 稳定且零漂移；symphonia 0.6 按 RFC 9639 实现的元数据/帧布局与旧版有差异，走通用层反而引入不确定。真实解码测试夹具由 libsndfile（soundfile 1.2.2）生成的 21KB 单声道 48k 1 秒 FLAC 文件提供（提交于 `crates/aura-core/tests/fixtures/`，自生成可公开分发，§41 合规）。

## D-016 内部 PCM 格式（2026-10-03，M1）

全 pipeline 统一**交错 f32**。源格式由 PcmBuffer 携带的 AudioFormat 显式标注。非有限样本值在输出渲染阶段确定性替换为 0（D-025）；有限极值允许通过（限幅属 M2）。单包解码失败不致命：跳包继续（容错策略）。

## D-017 重采样选型（2026-10-03，M1）

rubato 5 `Async::new_sinc`（256 阶 sinc、Blackman2 窗、1024 帧输入块、FixedAsync::Input、相对可调带宽 2.0——必须 ≥1 覆盖 44.1k↔48k 的 ratio 1.088）。采样率一致时旁路。吞吐基线 285× 实时（见 PERF.md）。EOF 尾块以 `partial_len` 喂入（rubato 以静音补齐缺帧）。

## D-018 时钟设计（2026-10-03，M1）

无独立 wall-clock 依赖：位置权威 = 输出侧消费帧数（SharedOutputState.position 原子量）。测试确定性由 aura-testkit 的 VirtualClock 与显式 `render_for_frames(n)` 推进保证；禁止 `SystemTime::now()/Instant::now()` 作为播放位置依据。

## D-019 Generation 设计（2026-10-03，M1）

load/seek 均 bump generation（u64，起 1）。ring buffer producer 在 bump 时记录缓冲内旧代帧数；consumer 检测代切换后精确丢弃。**并发残余竞态**：弹出过程中发生 bump 时，该块整体丢弃返回 0（输出静音）——最坏情况为单个渲染块的静音间隙，不会输出旧代样本；确定性单线程路径下零泄漏。逐块大小由 Engine 的 chunk_frames 控制（默认 buffer/8）。

## D-020 NullOutput 设计（2026-10-03，M1）

NullOutput = OutputStage（渲染）+ SharedOutputState（原子位置/underrun/目标增益）+ 捕获缓冲。`render_for_frames(n)` 以固定块推进：命令 → 填充（背压感知）→ 渲染 → 捕获。CI 无声卡全链路可跑。捕获缓冲容量预分配，零分配测量窗口内不增长。

## D-021 输出抽象与 CPAL 适配（2026-10-03，M1）

`AudioOutput` trait 最小化（format/start/pause/stop/consumed_frames/underruns）。真实设备走 `CpalOutput`（默认设备、设备默认格式，f32/i16 两路径；错误回调仅递增原子计数）。已知限制：设备格式与生产格式不一致时的重采样接线属 M6+；pause 语义由 Engine 驱动（渲染静音），设备流保持运行。

## D-022 Overrun 策略（2026-10-03，M1）

producer 遇缓冲满即**部分写入拒绝**（返回实际写入帧数），fill 循环背压暂停——不覆盖、不丢已缓冲数据。underrun（consumer 侧）：输出静音 + 计数 + 管线可恢复，不 panic/不阻塞/不分配。

## D-023 Stop 语义（2026-10-03，M1）

Stop：状态回 Stopped、**播放位置归零**、缓冲清空（generation bump）。暂停（Pause）位置保持。seek：位置设为目标帧（同块 generation bump）。

## D-024 EOF 尾块重采样策略（2026-10-03，M1）

流结束时剩余输入不足重采样器一个完整块：以 `partial_len` 喂入，rubato 以静音补齐缺失输入帧。尾部补齐不影响有效输出样本（调用方按帧数取用）。

## D-025 PCM 安全净化（2026-10-03，M1）

输出渲染阶段：非有限样本值（NaN/±Inf）确定性替换为 0，随后应用平滑增益。合成/解码路径产生的有限极值原样通过（限幅属 M2 loudness/limiter）。

## D-026 零分配承诺边界（2026-10-03，M1）

零分配承诺覆盖 **OutputStage::render 回调路径**（真实线程模型中即 audio callback：ring pop + 增益平滑 + 计数），由计数分配器测试证明。M1 单线程引擎中 decode/fill 属解码面，分配允许（真实线程模型中移入解码线程）；resampler 分块输出缓冲同理。真实线程接线（M6+）时保持此边界。
## D-027 下一曲预解码窗口（2026-10-03，M2）

窗口默认 3 秒（`DEFAULT_PRELOAD_WINDOW_FRAMES = 3 × 48_000` 帧口径），可经 `set_preload_window_frames` 配置。控制面在剩余帧数进入窗口时调用 `preload_next`；**同一时刻只允许一个 next**（重复调用覆盖前一个——测试与调用方契约）。

## D-028 Gapless 实现（2026-10-03，M2）

音轨切换在**同一管线内顺序解码**完成：当前轨 `decode_next` 返回 0 且有预装载 next 时，直接替换 decoder、重置每轨计数、**不 bump generation、不清 ring buffer、位置连续推进**。M1 的单管线天然无跨管线接缝，样本级无缝由连续解码保证。

## D-029 Gapless 元数据处理（2026-10-03，M2）

symphonia 0.6 的 FLAC gapless trim 在解码器构造时以 `AudioDecoderOptions { gapless: true }` 启用（M1 D-015 沿用）；有效帧数经 `TrackFrameRange` 记账（source_frames / valid_frames / rendered_frames）。**不自行猜测 trim 值**——synthetic 测试解码器天然无 padding。

## D-030 Crossfade 曲线（2026-10-03，M2）

等功率：`ga = cos(t·π/2)`、`gb = sin(t·π/2)`（t ∈ [0,1]），满足 ga²+gb²=1，单调无 overshoot。基于输出帧而非 wall-clock；`crossfade_frames = duration × sample_rate`（44.1/48/96kHz 均有测试）。

## D-031 Gapless/Crossfade 策略（2026-10-03，M2）

crossfade_frames == 0（引擎默认）→ 一律 gapless（情况 A/C）；cf > 0 且预装载了 next 且**双方格式一致**（resampler 旁路前提）→ crossfade（情况 B）；格式不一致 → 回退 gapless（D-033）；cf ≥ 曲目剩余长度 → 降级 overlap = remaining（不 panic，§16）。

## D-032 Loudness 增益公式与峰值安全（2026-10-03，M2）

`gain_db = −16 − integrated_lufs`；峰值安全上限 `max_gain = 10^(−1/20) / 10^(true_peak_db/20)`，实际增益取二者较小。元数据缺失或 enabled=false → gain = 1.0（不猜测，§25）。M2 只消费 `TrackLoudness`（synthetic fixture 测试），分析/持久化归 M3/M8。

## D-033 Limiter（2026-10-03，M2）

分段软限幅：|x| ≤ −1dBFS 直通；肩部 tanh 压缩，渐近 ≤ 1.0 dBFS；非有限输入 → 0。目标峰值 ≤ −1 dBFS 由 loudness ceiling 保证，limiter 为最后防线（对 EQ/preamp 组合过冲兜底，§45）。limiter 可开关：**生产恒开**，ramp provenance 测试关闭（D-034）。

## D-034 EQ 实现与同步（2026-10-03，M2）

RBJ cookbook biquad（Peak/LowShelf/HighShelf），10 频段固定中心频率，Direct Form I 级联按声道独立。预设为纯数据（6 个）。参数经 **EqBus 原子字发布**（50 × AtomicU32 + 偶奇序号撕裂检测）——回调 `load_into` 复制进本地 Copy 快照，无锁、无分配；序号奇数（写入中）时保留旧快照下块重试。EQ 系数计算在控制线程（`set_eq_bands`）。

## D-035 零分配承诺边界修订（2026-10-03，M2）

回调路径（`OutputStage::render`：pop → 净化 → 平滑 → loudness×preamp → EQ → limiter）零分配由计数分配器证明（m2_zero_alloc_v2.rs 独立二进制，避免并行测试计数干扰）。**解码面**（decode/fill/resampler 块处理/capture 诊断缓冲增长）允许分配，真实线程接线（M6+）后移入解码线程。为此 M2 将 resampler 改为 `process_chunk_into`（process_into_buffer + 复用输出缓冲）与 `map_channels_into`。

## D-036 热插拔策略（2026-10-03，M2）

设备状态机 `Connected / Removed`。断开：安全暂停（Playing→Paused）、保留曲目/位置/解码器；恢复：重建重采样与渲染缓冲（`set_output_format`，位置保留），**不自动播放**；默认设备变化走同一恢复路径；流错误仅计数（`device_errors`），不误触发曲目切换。检测本身归外部适配器（M10），回调内禁止设备枚举。

## D-037 内容身份算法（2026-10-04，M3）

`identity_key = "v1:<size:016x>:<sip128hex>"`：文件大小 + 首/尾 1MiB 采样的 SipHash-2-4 128 位指纹（`siphasher` crate，MIT/Apache-2.0，零传递依赖；charter §9 推荐方案的直接实现）。采样定义：size ≤ 2MiB 全量，否则头 1MiB + 尾 1MiB（中部不采样）。取舍：中部突变在 >2MiB 文件上不可见（可接受——真实音频替换几乎必然改变长度/头部元数据）；吞吐 ≈ 1.4 GB/s（scalar SipHash），增量扫描使重复采样不再发生（PERF M3）。键版本前缀 `v1` 保留指纹算法演进空间。

## D-038 身份解析：标签改写 vs 音频替换（2026-10-04，M3）

内容指纹在标签重写后必然变化（ID3/Vorbis 注释在头部/尾部采样窗口内），而 §10 要求"仅标签变更 TrackId 不变"。解析规则：同路径新内容时比对 `audio_sig = codec/sample_rate/channels/duration_frames` —— 未变 → 标签改写，保留 TrackId 刷新 metadata；变化 → 音频替换，新 track，旧 track 无活跃路径后停用（统计保留 §47）。边界：音频替换且时长/采样率/声道恰好全部不变（极端罕见）会被判为标签改写——接受并记录；不做逐帧解码比对（扫描性能不可接受）。

## D-039 移动+改标签同时发生的孤儿重关联（2026-10-04，M3）

新路径 + 新指纹（两个窗口都变）时：先查"已停用且 audio_sig 与搜索标题（search_text）匹配"的 track，命中则重关联（TrackId 与统计保留）；否则新建。ORDER BY id LIMIT 1 保证确定性。

## D-040 封面选择策略（2026-10-04，M3）

多封面时 `CoverFront` 优先，否则声明顺序第一个。MIME 以字节签名判定（JPEG FF D8 FF / PNG 魔数），不信任标签声明；非 JPEG/PNG 或 >8MB 跳过（§31/§66：artwork 失败 ≠ track 失败）。

## D-041 reparse point / symlink 策略（2026-10-04，M3）

不跟随任何 symlink/junction/reparse point（目录不递归，文件跳过并记 `ReparseSkipped`）。`DirEntry::file_type()` 用 readdir 数据不触发最终组件 follow。不做 cycle detection/canonical 白名单（跟随即复杂化，收益低）。root 边界由「canonicalize root + 不跟随 reparse」共同保证（§13/§14）。

## D-042 FTS 分词：CJK 单字+滑窗二元组（2026-10-04，M3）

rusqlite bundled 无法注册自定义 FTS5 tokenizer，采用预分词：索引侧每字段产出「小写英文词 + CJK 单字 + 相邻二元组」，空格连接，FTS5 用内建 unicode61 切分；查询侧同样分词后逐词 `"tok"*`（引号失效化全部 FTS 语法，§65）。单字 CJK 查询 = 前缀匹配 token 起始（§34）。陷阱修复：NFKD 会把浊音假名（ド→ト+浊点）分解丢浊点，CJK 段整体绕过 NFKD（normalize::search_text）。滑窗（非跳窗）二元组保证「光乐」等跨词内子串可命中。

## D-043 watcher debounce 窗口 500ms（2026-10-04，M3）

`EventCoalescer` 时间窗 500ms（窗口起点或最后事件后 500ms 即 flush，先到者）。合并优先级 Removed > Renamed > Changed（终态覆盖）。watcher 线程只写内存结构，DB 写只发生在应用层发起的扫描（§44/§45）；窗口实测以真实事件测试验证（§73 有界轮询）。

## D-044 分页：limit/offset（2026-10-04，M3）

offset 分页（非 cursor）：实现简单、排序键任意可换、10k 行 page 500 实测 122ms 满足交互级；M6 虚拟列表若实测不足再评估 cursor（sort key+id 复合游标）。

## D-045 filter 求值在 Rust 侧（2026-10-04，M3）

M3 无分析数据（BPM/Key/Energy 属 M8），`filter_tracks` 全量拉活跃集在 Rust 求值后分页。10k 行 <200ms 可接受；M8 落库后把可下推谓词翻译为 SQL（analysis 字段 JOIN track_analysis）。

## D-046 搜索排序：bm25 权重 + id 决胜（2026-10-04，M3）

`ORDER BY bm25(fts, 0, 8, 6, 4, 4, 2, 1), track_id`。§36 的 title>artist>album 优先级经列权重表达；实测发现微小语料（N≤个位数）IDF 为负会反转直觉排序——§36 明确允许 fallback「FTS rank + deterministic secondary ordering」，采用之；真实语料（百行以上）title 权重主导恢复。

## D-047 队列 Clear 的 undo = 显式 no-op（2026-10-04，M3）

命令日志粒度不足以恢复 Clear（需全队列快照）。M3：undo 弹出日志项并返回 `Clear`，不做恢复；M6 UI 如需完整 undo，加队列快照表再升级。Append/Insert/Remove/Move 全部可逆（Remove 在日志 payload 记录被删 track_id）。

## D-048 M3 依赖引入（2026-10-04，M3）

`lofty 0.22`（标签，MSRV 1.85）、`rusqlite 0.40 + bundled`（SQLite，bundled 自带 FTS5）、`notify 8.2`（watcher，CC0）、`image =0.25.9`（MSRV 1.85；**固定等号版本**——0.25.10 MSRV 1.88 超出工作区 1.87）、`siphasher 1.0`（SipHash-2-4-128）、`unicode-normalization 0.1`（NFC/NFKD）。全部许可在 deny 白名单；image/jpeg+png 最小 feature（无 webp/avif/gif）。

## D-049 M4 来源依赖与 async 边界（2026-10-04，M4）

`reqwest 0.12 blocking + rustls-tls + json`（无 tokio 直依；blocking 内部 runtime 由 reqwest 托管）。选 blocking 的理由：workspace 全同步栈、M1 Decoder 是 pull 模型、取消可用"所有权交接 + drop response 关 socket"满足（D-052）；M6 Tauri 层如需 async，由应用层 `spawn_blocking` 桥接。serde/serde_json 用于 Radio Browser 解析。

## D-050 ICY metadata 上限（2026-10-04，M4）

协议形状上限：长度字节 ≤255 → 块 ≤4080 字节（255×16）。`MAX_ICY_METADATA_BYTES = 4080`，超出（不可能的声明）视为流损坏 → 该块丢弃、流终止（不消费音频）。非 UTF-8/畸形键值 → 仅丢弃该块，音频继续（§36）。

## D-051 流缓冲容量（2026-10-04，M4）

finite：1 MiB 固定。live：`min(bitrate×30s, 2 MiB)`，未知码率 256 KiB（≈64kbps×30s），下限 256 KiB（§37 滚动 ≤30s）。容量构造时固定，运行期不增长（§29/§100）。

## D-052 网络流取消协议（2026-10-04，M4）

reqwest blocking 无 per-read timeout。协议：`Response` 唯一宿主是共享 slot；feeder 分段借用（take → read 一块 → 放回）。取消方持锁取走并 drop → socket 关闭 → feeder 下一次 read 错误退出（正常服务器毫秒级）。stalled-read 期间消费侧立即返回 Timeout（不等 socket；keepalive 15s 兜底关闭）。已知限制：服务器彻底静默时 feeder 线程最终退出时间取决于 TCP keepalive，消费侧永远不被阻塞。

## D-053 DNS 层 IP 复检（2026-10-04，M4）

自定义 `reqwest::dns::Resolve`：解析结果逐 IP 过私网检查，任一私网 IP 且策略 deny → 整个解析失败（不给 hyper 任何可用地址），防 rebinding（§18）。host 字符串层检查（localhost/.local/IP 字面量）仍在 validate_url 先行。

## D-054 url_resolved 使用与记录（2026-10-04，M4）

播放一律优先 `url_resolved`，缺失回退 `url`；实际使用的字段由 `Station::stream_url()` 返回（调用方可记录），不静默切换。

## D-055 symphonia mp3 特性（2026-10-04，M4）

symphonia 元 crate 默认特性**不含 mp3**（只有 flac/ogg/vorbis/wav/adpcm/mkv/pcm）。Charter §14 支持矩阵含 MP3 且网络电台以 MP3 为主 → aura-core 显式 `features = ["mp3"]`（symphonia-bundle-mp3，MPL-2.0 纯 Rust；§74 只禁止 AAC/FDK 类）。probe 解码器（StreamingDecoder）覆盖网络场景未知容器；FLAC 仍走 M1 具体类型路径（D-015 不变）。

## D-056 content key 与 URL 规范化（2026-10-04，M5）

`content_key = hex(SipHash-2-4-128(normalized_url))`（专用密钥，与 M4 内容身份密钥分离防跨域碰撞解释）。规范化：scheme/host 小写、默认端口剔除、fragment 剔除；**path+query 原样保留**（§12：合法直链依赖 query；不同 token = 不同内容）。index 只存 url_hash + redacted URL（host/path，无 query）——signed URL 的 token 不入库不进日志（§13/§85）。chunk 路径 = `chunks/<ck[0..2]>/<ck>/<idx:08>.bin` 固定深度映射，content key 必须 32 位小写 hex（§116/§118：用户字符串永不入路径）。

## D-057 chunk 记账与原子写顺序（2026-10-04，M5）

`CHUNK_SIZE = 256 KiB` 全模块唯一。写入顺序：tmp 写（cache 根内 `tmp/`，确定性文件名）→ 写后大小校验（FaultFS Truncation 在此被抓住）→ 原子 rename → 写后 SHA-256 校验 → DB 事务记录。崩溃窗口分析：tmp 残留（integrity/清理 tmp 回收，§133）；rename 后 DB 前 → orphan 文件（integrity 报告后删除 D-060）。`stored_size` 恒等于 `size`（原子写保证），不设冗余列。

## D-058 DB 损坏恢复（2026-10-04，M5）

cache.db 打不开 → `cache.db.corrupt-<ts>` 备份（连同 -wal/-shm）→ 重建空索引（§27 安全降级）。不尝试从文件系统重建索引（文件无 url/validator 元数据，重建会引入不可校验数据）；chunks 保留为 orphan，integrity pass 按策略清理。结构隔离保证（§28）：aura-cache 不知道 library DB 路径，无法误删。

## D-059 读路径不付 hash 成本（2026-10-04，M5）

chunk 读只校验存在 + 大小；SHA-256 校验发生在写入后、integrity pass、崩溃恢复扫描三处。理由（§90/§91）：读路径在 decode 之前的 producer 侧，逐读 hash 会把 2.4GB/s 的额外 CPU 推进音频供给链；损坏数据靠写入时校验 + 周期 integrity 拦截，符合 §44「单 chunk 单独可验证」。

## D-060 orphan 策略（2026-10-04，M5）

orphan chunk（文件在 DB 无行）与非法目录名：integrity pass 报告后**删除**。理由：全部 M5 缓存内容可由源重新获取（Allowed 直链），保留幽灵文件只占磁盘；不 quarantine（无诊断 UI 消费）。

## D-061 revalidation 策略（2026-10-04，M5）

open 时一次条件验证（If-None-Match 优先，无 ETag 才用 If-Modified-Since §41）。304 → Validated（缓存继续）；200 → 内容变更 → 作废该 key 全部 chunks（§43 绝无半新半旧）+ 更新 validator；404/410/网络失败 → Stale（§83：离线可用，状态显式）。不做 TTL 周期验证（M5 播放场景一次验证足够；后续如需 TTL 记录新决策）。非 Range 服务器（200 on Range）→ 标记 non-range + sequential fill（§35/§36 有界），seek 只允许已填充区间（§37）。

## D-062 retry/backoff/prefetch 参数（2026-10-04，M5）

retry 3 次，仅 timeout/连接类/5xx；指数退避 base 200ms、cap 1.6s、jitter 注入（测试恒 0，§48/§106）。prefetch：读取触发式（obtain 后单线程拉取前方 2 chunk）+ 显式 `prefetch_ahead(n)`；epoch 计数器 seek 即递增使旧预取终止（§50 任务上限 = ahead 常数；§51 seek 取消）。

## D-063 磁盘满/只读 cache（2026-10-04，M5）

写失败（DiskFull/PermissionDenied）→ 错误上抛，不损坏已有 chunks、不动 pinned（§124）；已缓存内容可继续读（§126 只读 cache 允许读）。Allowed 目录不可写的 graceful 降级（§125：转 SessionOnly）由应用层捕获 PermissionDenied 后降级实现 —— cache 层提供错误分类，不静默重试。

## D-064 Engine 消费端移交（2026-10-05，M6 必要最小修复 §5）

M1 Engine 的 render_for_frames 同时承担「解码填充 ring」与「排空 ring → OutputStage」。M6 真实输出需要 CPAL 回调内的 OutputStage 直接消费 ring（CpalOutput::open_default(consumer, …) 设计意图）。新增 `Engine::detach_consumer_for_external_output()`：一次性移出 ConsumerHandle 并置 `consumer_detached` 标志；此后驱动线程只用 `fill_and_process()`（fill + process_commands）。占位空 ring consumer 保证误用安全。M1 全部 88 测试回归通过（增量 API，零语义变更）。

## D-065 实时命令路径（2026-10-05，M6）

UI 实时命令（Play/Pause/Stop/Seek/SetVolume）经 AudioCommandQueue（rtrb，单消费者）投递；队列由驱动线程独占，故 AudioHandle 将队列移入驱动线程，Handle 侧经控制通道 `Control::Post(AudioCommand)` 转发 —— 单一写入点，避免跨线程 &mut。快照读走 Arc<Mutex<PlayerSnapshot>>（10Hz 刷新 + revision 递增，§147–150 后端权威）。

## D-066 Tauri 插件与权限（2026-10-05，M6）

capabilities 最小集：core:default + core:window:{minimize,toggle-maximize,close,start-dragging} + dialog:default（Add Music Folder）。tauri-plugin-dialog（前端 @tauri-apps/plugin-dialog + Rust tauri-plugin-dialog）。无 shell/updater/fs 通配权限（§22/§23）。CSP 维持全自托管。

## D-067 artwork UI 通道（2026-10-05，M6 补强）

Now Playing 封面经 `track_artwork(track_id)` 命令读取 M3 artworks 表字节 → base64 data URL 注入 `<img>`（§37：任意本地路径不进 webview URL）。四态：loading（请求中）/ loaded（`<img>` onload）/ missing（无 artwork_id 或悬空引用）/ corrupt（解码失败）。前端 `Artwork` 组件 + `useArtwork(trackId)` hook；TrackId 经 `track_id_by_path(path)` 解析。

## D-068 Win11 Mica 接入（2026-10-05，M6 补强）

`window-vibrancy 0.5` `apply_mica/clear_mica` 于启动（settings.mica_enabled，默认 true）与 `set_mica` 命令时应用；apply 失败（非 Win11/合成器关闭）静默保持 solid 背景（§20 fallback）。settings.mica_enabled 持久化。

## D-069 冒烟测试的 runtime 混链限制（2026-10-05，M6）

同一测试二进制同时链接 `tauri::test`（MockRuntime）与 Wry 时在 Windows 触发 STATUS_ENTRYPOINT_NOT_FOUND（DLL 入口点冲突）。处置：① 无窗口 mock IPC 冒烟仅保留**无 State 依赖**的 `app_info` 分发测试；② 真实窗口冒烟独立为 `--bin smoke`（仅链 Wry）：构建与 lib.rs 相同装配的应用 → run_iteration×3 触发 setup → 注入 State 调用 `player_snapshot`/`app_info` 命令函数 → 断言快照合法 → cleanup。`any_thread()` 允许事件循环/构建跑在非主线程。泛型命令（`fn cmd<R: Runtime>(State<'_, AppState<R>>)`) 在 `generate_handler!` 宏路径产生 E0283 —— 命令固定绑定 Wry（生产唯一 runtime）。

## D-070 Open URL 私网策略（2026-10-05，M6-B）

壳层唯一 HttpClient 以 `allow_private` 构建：用户在 Command Palette 显式输入 URL 即为 M4 §17 意义的“一次性确认”（自建/局域网服务是合法场景）。scheme 白名单、凭据拒绝、平台封锁表、重定向上限全部照常生效（§278 不因 UI 入口降级）。默认拒绝路径仍由 `deny_private()` 覆盖非用户输入场景。

## D-071 decoder 在命令线程打开（2026-10-05，M6-B）

`Control::Load` → `Control::LoadDecoder`：文件/网络流的 decoder 打开（含探测）移至 IPC 命令线程执行，失败以结构化 AppError（UNSUPPORTED_SOURCE 等）回传 UI；驱动线程只装载已就绪 decoder。旧路径驱动线程吞错（`continue`）违反 §184/§187，已消除。

## D-072 queue 单一权威 + 游标（2026-10-05，M6-B）

应用层 in-memory queue（§48 允许）扩充为游标模型：`playWithQueueContext(track, context)` 以双击曲目为当前项、可见列表为上下文整体替换；`playNext/playPrevious` 沿游标行走并在边界返回 false（§257 执行前检查）；`removeAt` 维护游标一致性。palette/main/mini 均引用同一模块单例（§197）。

## D-073 错误映射链（2026-10-05，M6-B）

Rust `AppError.code` → `ui/src/ipc/errors.ts` 静态表 → i18n key（zh/en）。组件禁止 `switch(error.message)`（§189）；未知 code 回退 `error.internal`。新增错误码必须同步映射表（errors.test.ts 以「全部已知码双语可译」断言锁定）。

## D-074 VISUAL_TEST_MODE（2026-10-05，M6-B）

URL 带 `?visualTest` 时 Aurora 进入确定性模式：seed 固定 `visualSeed('visual-test')`、time 固定 42s、telemetry 固定 playing/50%，渲染单帧后停止 rAF（§179/§180）。正常路径 drawFrame 与循环共享同一函数。

## D-075 custom-protocol feature（2026-10-05，M6-B）

裸 `cargo build --release` 的 Tauri 应用仍加载 devUrl（generate_context! 仅在 custom-protocol feature 下内嵌 frontendDist）。src-tauri 增加标准模板 feature 透传 `custom-protocol = ["tauri/custom-protocol"]`；发布/截图构建命令为 `cargo build --release -p aura-shell --features custom-protocol`（与 `tauri build` 等价的前半段）。另：ui/dist 变化不触发 cargo 重编译——发布前必须 touch 源文件或 clean（已两次踩坑）。

## D-076 mini 窗口权限与标题（2026-10-05，M6-B）

mini 窗口（label `mini`）经 JS `new WebviewWindow()` 创建，需 capabilities：`core:webview:allow-create-webview-window` + `core:webview:allow-get-all-webviews`（getByLabel）+ `core:window:allow-show`，并将 `mini` 加入 capability windows 列表。窗口标题实际为 tauri.conf 产品名（"ifuyo Aura"），自动化按尺寸（≈340×110）识别。§191：create-webview-window 仅允许应用代码以固定 URL `index.html?view=mini` 创建，无任意导航。

## D-077 IME 与快捷键（2026-10-05，M6-B）

中文 IME 组合态会拦截 palette 文本输入的字母键与 Enter（截图管线实测）。自动化/键盘路径不依赖 palette 输入：mini 用 Ctrl+M、queue 用 Ctrl+Q 直达（§260 记录）。产品结论：palette 对 IME 用户可正常输入中文（组合态提交后过滤），但自动化测试使用快捷键。

## D-078 键盘导航图（2026-10-05，M6-B）

Ctrl+K palette；Esc 逐层退出（palette→queue/settings）；Space 播放/暂停（输入态让出 §258）；Ctrl+F 聚焦库搜索；Alt+1/2/3 主导航；Ctrl+Q queue；Ctrl+M mini。全部为应用级 keydown（window 捕获），不与 OS 全局热键冲突（仅在窗口聚焦时生效）。

## D-079 Dreamscape seed 设计（2026-10-05，M7 上篇）

内部 seed = 60 bit 规范值 + `seed_version`（SipHash-13，域分离固定密钥；§15 禁 DefaultHasher）。人类格式 `DS-XXXXX-XXXX-XXXX`：Crockford Base32，60 bit 载荷 + 4 bit 折叠校验（§13，非裸 hex）；解码大小写不敏感、I/L→1、O→0，非法返回 None 不 crash（§105）。`world_id = hash(seed + generation_version + 初始环境)`（§18），永久稳定。60 bit 选择：13 字符 Crockford 恰好承载 64 bit（60+4），无损双向转换。

## D-080 RNG 命名流（2026-10-05，M7 上篇）

SplitMix64（全周期、零依赖）；流 = `hash(master_seed, seed_version, 流名)`（§21 结构化派生，非 index 偏移）。标准流名集合固定（world/terrain/vegetation/weather/event/mutation/placement/palette），新增只追加。逐日/逐桶决策用 `from_stream_bucket(master, stream, bucket)`（O(1)、同桶同结果、可重放）。序列锚点测试锁定算法变更（打破即有意升级并记 DECISIONS）。

## D-081 Genome 与 mutation 步长（2026-10-05，M7 上篇）

8 个有界参数（min/max/default 集中在 `GenomeField::bounds`，§25）+ `GENOME_VERSION`。seed→genome 以 default 为锚、world 流决定 ±半幅内偏移。mutation：每条 1–3 字段、步长 ≤ 区间 3%（§26 小步、§83 不毁世界）；触发仅 NewSession/LongIdle(每 8 逻辑小时窗口至多一次)/WorldEvent/Explicit（§27/§28，无每帧随机）；记录含 parent_state_hash 与 mutation_id（§29 可重放，日志重放一致性测试锁定）。

## D-082 环境时间模型（2026-10-05，M7 上篇）

绝对逻辑 tick（1 tick = 1 逻辑分钟，§30）为唯一时间源；`time_phase = tick/1440 mod 1`（§33 连续相位，渲染层插值）。天气五档氛围级（§35/§36），`neighbors` 过渡图禁 Clear↔Rain 直跳，blend 全程 ≈240 tick（§37）；决策按 240-tick 日桶派生。fog 硬上限 0.7（§39）；夜间环境光 ≥0.25（§99）。palette：biome 锚 ±seed 漂移 ±色温联动，点缀相隔 90–150°，sat/lum/accent 约束区间（§41/§42），300 组合扫描测试锁定。

## D-083 世界预算与放置（2026-10-05，M7 上篇）

有限边界 ±256 m（§50）；地形 = 两倍频程 value-noise（晶格值经固定 hash，查询顺序无关，§48/§49）。预算常量（§63）：objects ≤512 / landmarks ≤7 / particles ≤256，密度受 1.3 硬帽——极端 genome 实测不超。放置规则（§52）：坡度容差、0.9 m 排除半径（spatial hash 3×3 邻域，§54）、landmark 排除（§53）、族亲和由 biome 决定；landmark 稳定 id = hash(seed, index)（§56）。

## D-084 稀有事件稀有度（2026-10-05，M7 上篇）

事件 = `hash(seed, 240-tick 日桶) + event 流`（§91 可重放）；桶触发概率 1/30（期望 ≈ 每逻辑月一次，§90 真稀有）；时长 2–6 逻辑小时，各 1/4 淡入淡出（§94），任意时刻同时可见 ≤3。事件日志只记 event_id/kind/tick/params（§93）；simulate→to_log→replay 与模拟可见集合一致（§92 测试锁定）。无奖励字段（§96）。

## D-085 SceneManifest 与 scene_hash（2026-10-05，M7 上篇）

manifest 为 renderer-friendly 但 domain-neutral 的纯数值 DTO（terrain patch/palette/light/atmosphere/landmarks/SoA 实例组，§110）；无 Three.js 概念（§111）。`scene_hash` = 域分离 SipHash 按规范字段序（§113）；同 seed+version+genome+env 相等、任一输入变化即变（§114 测试锁定）。体积受世界预算传导，实测 <256 KiB（§115）。

## D-086 Region 流式与取消（2026-10-05，M7 上篇）

64 m 区域网格（§117）；region seed = hash(world seed, coord)（§118）；区域内容 = 世界对象的确定性子集（§119，切分完整性测试锁定）。`RegionStreamer`：worker 线程（§121）+ 非阻塞 request/poll（§120 不阻塞调用帧）+ 代际（epoch）取消——cancel_all 前移代数，陈旧代结果不送达（§124）；drop 即停机。LRU 内存缓存 ≤16 区域（§125 B 级；纯内存，与 M5 无关 §126）。

## D-087 逻辑态持久化（2026-10-05，M7 上篇）

`dreamscape.json`（app_data）仅存 DreamscapeSnapshot 逻辑态（§79/§80/§127）：world_id/seed/代/genome/env/mutation_log/event_log/camera/tick；原子写（tmp+rename）。损坏/版本不符 → 回退缺省世界并继续启动（§265/§266 精神）；restore 命令区分 NO_SNAPSHOT/CORRUPT/DREAMSCAPE_VERSION_UNAVAILABLE（§106 不静默换世界）。缺省 seed 固定 `ifuyo-aura-default`（§77 明天回来仍是同一世界；Regenerate 仅显式 §103）。shell 侧 9 个 typed IPC 命令为下篇渲染层的数据通道。

## D-088 Dreamscape 渲染器架构（2026-10-05，M7 下篇）

`ui/src/visual/dreamscape/`：engine.ts（纯逻辑：质量档/AutoQuality 迟滞/节流/GenerationGate/Lifecycle 状态机，无 WebGL 可单测）+ scene.ts（manifest→GL 装配，Disposables 登记释放）+ DreamscapeRenderer（生命周期编排）。§128 建议的 8 文件结构收敛为 3 文件——职责等价（Telemetry=stats 回调；InstancePool=每族共享 InstancedMesh §61/§133；Effects=场景内 fog/光，无后处理栈 §140）。

## D-089 质量档与 Auto（2026-10-05，M7 下篇）

Low/Medium/High/Auto（§141）：pixelRatioCap 1.0/1.25/1.5（§150）、地形细分 32/64/96、粒子 0.35/0.7/1.0、后处理仅 High（§139/§140 默认不叠层）。Auto：帧时长 >45ms 持续 2s 降档、<20ms 持续 3s 升档、变更后 4s 冷却（§143 不震荡）；中带 20–45ms 不动作。

## D-090 帧率与可见性（2026-10-05，M7 下篇）

targetFps：播放 30 / idle 10（§145 5–15 取中）/ 最小化或隐藏 2（§146/§147）/ reducedMotion 5（§70 相机近静止+粒子停）。音频稳定性优先（§144 不追 60fps）。§263/§264：IPC 只做低频推进（2s 实时 = 30 逻辑分钟）+ 按需拉取；相机/插值/粒子全部渲染器本地（§265–§267）。

## D-091 Aurora 互斥（2026-10-05，M7 下篇）

§130/§131：dreamscape 视图时不挂载 AuroraCanvas（React 条件卸载），退出即恢复——单高负载 WebGL 规则由结构保证（测试锁定）。Mini Player 永不显示 Dreamscape（§291/§292）。

## D-092 base.css 全屏居中移除（2026-10-05，M7 下篇）

Phase 0 的 `body{display:grid;place-items:center}` 使 app-root 收缩居中——真实构建截图暴露主布局/Dreamscape 画布被压缩（§282/§204 不伪造视觉）。移除后 app-root 满幅；M6 各页面布局随之正常化（此前恰好被绝对定位的 Aurora 遮罩掩盖）。

## D-093 幂等命令与 revision（2026-10-05，M7 下篇）

§256/§257：dreamscape_open/close 重复调用不报错（渲染器单例在 UI 侧，close 不销毁世界状态 §75）；set_quality 合法值校验后幂等；set_environment 冻结/解冻（§100/§101）。§262：状态 revision 单调递增随 DTO 下发——UI 陈旧回调丢弃的基础（§259/§260/§261 由 GenerationGate 在渲染器侧同语义执行）。

## D-094 快照完整性校验与历史上限（2026-10-05，M7 下篇）

§299：snapshot integrity = 载荷规范 hash（integrity 字段归零后计算）；validate 顺序 = 版本门（§106）→ 校验和 → world_id 一致性。§229/§230：`#[serde(default)]` 旧快照缺字段显式默认（integrity=0 视为无校验和）、未知字段忽略。§219/§220：MAX_MUTATION_LOG=200 / MAX_EVENT_LOG=500，超出即裁最旧（当前 genome 即压缩态）。§225：persist 原子写（tmp+rename）且写前 seal（§300）。

## D-095 Embedding 选型：自有 DSP 统计嵌入（2026-10-05，M8 上篇）

§39–§44 要求"真实、可运行、合法、可本地、可商业发行"的 embedding。评估结论：**不引入任何第三方模型/推理运行时**（ONNX/Candle/Burn 均因 binary size、打包、license 审计成本与"为一个功能引入大框架"原则（§44/§241）被否）。采用自有 SpectralStatsProvider：Hann 帧内 DFT（O(N²)，中篇换 FFT 时升 SPECTRAL/MFCC_ALGORITHM_VERSION）→ 14 个对数频带 dB 统计 ×2 + 帧能量 mean/std + ZCR/质心/滚降/时长代理 = 32 维，L2 归一化，f32 LE。满足 §42"真实可运行"（真实信号统计，非随机/hash，区分度有测试）；§46 纯定序 f32 运算逐位确定（无平台容差需求）；§40 永不联网下载；zero license 负担。若未来引入学习型模型：新 provider + 新 model_id/version，旧结果经 §18 版本机制自然 Stale。

## D-096 分析持久化落点与 schema v2（2026-10-05，M8 上篇）

§24/§25：不建第二数据库——analysis 表由 **M3 迁移 V2** 定义（M3 拥有 schema 真相 §6），aura-analysis 仅持 `&Connection` 门面（AnalysisStore），依赖方向 analysis → rusqlite（不反向依赖 aura-library，避免环）。表设计：单表 + 标量列 + 两个带元数据 BLOB（mfcc: 13+13 f32 LE；embedding: dim×4 f32 LE + dim/model_id/model_version 列，§27/§28）。analysis_key UNIQUE（§23）。读回时：未知枚举串拒绝（不静默映射）、BLOB 尺寸×dim 校验（§29）。

## D-097 analysis_key 派生（2026-10-05，M8 上篇）

§23：`analysis_key = SipHash13(固定域密钥, content_identity ‖ 0xff ‖ analysis_version ‖ embedding_model_id ‖ 0xff ‖ embedding_model_version)` 的 64-bit hex。判定结果是否仍对应现状的唯一权威；content 变化（§20）或版本升级（§18）都会使旧键失效→Stale。

## D-098 规范分析采样率与重采样（2026-10-05，M8 中篇）

22050 Hz：Nyquist 11kHz 覆盖节奏/调性/倒谱所需频段，内存与 DFT 成本减半；重采样用线性插值（分析用途对混叠不敏感，记录为已知限制；下篇可升 polyphase）。声道策略 §88：mono fold-down (L+R)/2；反相位 → 静音语义（测试锁定 §174）。窗口参数集中 params.rs（Hann 1024/512 频谱帧；chroma 专用 4096/2048——5.4Hz/bin 可分低频半音）。

## D-099 LUFS 实现（2026-10-05，M8 中篇）

ITU-R BS.1770-4：K-weighting 两节 biquad（shelf 1.68k +4dB、HP 38Hz，RBJ 双线性随 fs 重算）；门控 400ms/100ms hop，绝对门 -70 + 相对门 -10 LU；LRA 3s/1s P10–P95 相对门 -20。True peak：4× 过采样 zero-stuff + 33-tap sinc×Hann（**相位子滤波器各自归一**——首版按总和归一实测衰减 4×，测试捕获修正）。静音（全块低于绝对门）→ None（below measurable，§72 不产生 -0 LUFS）。fixture 公式自洽：amplitude_for_target_lufs(L)=√(2·10^((L+0.691)/10))（mono），-12/-16/-20/-23 ±0.5 LU。

## D-100 Tempo 选择规则（2026-10-05，M8 中篇）

onset = spectral flux 半波整流 → ±1 帧盒式模糊（抗分数周期取整抖动）→ **分数周期 comb**（1/8 帧步长、线性插值 shift 相关，谐波 Σw_k·score(2^k·p) w=1/k）——整数 lag 自相关对「每秒整数倍点击」（如 180 BPM @43fps = 43 帧真周期）不可见，分数周期直接求解。§52 折叠：真基频族由 comb 胜出（谐波和多一正项），标签折叠进偏好窗 [70,180)（行业常速段）——60→120、30→120、240→120、180→120 为规则内正确结果（§52 允许自定规则，测试显式锁定）。confidence = comb 峰/包络能量 [0,1]。§54：静音→Unavailable、短于 2s→TooShort、噪声→低置信。

## D-101 Key 检测与置信语义（2026-10-05，M8 中篇）

chroma：4096-FFT（5.4Hz/bin）+ 输入去均值（Hann DC 放大泄漏）+ 跳过 <55Hz + m¹ 累积（m² 指数方差使噪声 chroma 假尖锐，实测 peak/mean 7.3）。24 假设（Krumhansl–Kessler maj/min 旋转）Pearson 最大。confidence = 最优相关强度 × 与次优分离度（×4 经验缩放），[0,1]。§64 门：chroma Shannon 熵 > 2.30（max 2.485）→ Unavailable（音阶 run 实测 2.0–2.2，白噪声 2.37）。Camelot（§65）：号=关系大调五度圈位（A minor→C major→8A）；**首版按调名字母直查错位（A minor 得 11A）**，修正为小调 +3 半音取关系大调。§66：canonical 升号（枚举仅含 Cs）。fixture 教训：孤立三和弦无法区分相对调（KK 剖面对 7 音平坦 chroma 区分度弱）——相对调用例用三和弦（tonic/third/fifth+八度），熵门用音阶 run。

## D-102 Energy 定义（2026-10-05，M8 中篇）

energy = RMS dBFS 线性映射 [-60, 0] → [0,1]（感知响度代理：-60dBFS 以下 0、0dBFS 1）。**不是**谱能量也不是振幅绝对值（§77 语义显式）。Brightness = Centroid / Nyquist [0,1]（§78 finite/bounded/deterministic）。

## D-103 MFCC 配置（2026-10-05，M8 中篇）

pre-emphasis 0.97；帧 1024/hop 512（规范参数 §79）；FFT 1024；26 mel 三角滤波器 20Hz–Nyquist（HTK 式）；ln floor 1e-10（§85 log(0) 防护）；DCT-II 归一 → 13 系数（§82）；存储 mean+variance 摘要（§83）。确定性：纯定序运算，×10 逐位一致（§187）。

## D-104 调度器与仓库（2026-10-05，M8 中篇）

AnalysisRepository trait（save/load/mark_stale/list_pending）+ InMemory 实现（§140/§141）；SQLite 桥接由 shell/应用层（AnalysisStore 已满足合同，§139 不让特征提取知 SQL）。AnalysisScheduler：单 worker 顺序（§97 有界）；优先级两级 User>Background（§96）；同 track 去重保高优先；取消幂等（§102）；重试有界 MAX_RETRIES=1（§109/§110）。M8 下篇接 shell 任务队列/并发 worker（§155 退出协议）。

## D-108..D-115 Music Map 内核（2026-10-05，M9 上篇）

- **D-108 dataset**：兼容合同 = 首个有效嵌入定义（dim/model_id/model_version/analysis_version 全等）；不兼容 → 计数排除（§29–§31 无投影）。TrackId 升序 canonical 进入管线（§27）；dataset hash = SipHash(排序后 track_id‖analysis_key‖model‖dim‖versions)（§26）。
- **D-109 相似度**：严格复用 M8 cosine 合同（§35/§36，f64 累计→f32）；零向量 → Err（§33 不悄悄 0.0）；NaN/Inf → Err（§34）。
- **D-110 top-K**：similarity DESC + TrackId ASC tie-break（§42）；k 钳制 ≤1000（§43）。
- **D-111 搜索策略**：exact scan ≤25k 节点；超出 → DatasetTooLarge（ANN 为 B 级 §46，引入前记录审计）。
- **D-112 kNN 图**：K=12 默认、阈值 sim>0（§55 cosine 负值不成边）；canonical 无向单条存储（a<b §63）；去重（§62）；边预算 ≤2NK（§58）；孤立节点允许（§60）；validate_graph 七检查（§61）。
- **D-113 聚类**：确定性 label propagation（节点 id 升序异步更新、计数最大+标签最小 tie-break、收敛即停）；canonical 重编号 = size DESC → min track_id ASC（§101/§102）；孤立节点自成单簇。Cluster 不伪造语义标签（§104）。
- **D-114 布局**：确定性 PCA 初始化（幂迭代 24 轮、无随机起点、符号规范=最大绝对载荷为正 §74/§75）→ grid 斥力（半径 0.12、32×32 bins §77）+ 边引力（similarity 加权）→ [-1,1]² clamp；预算 150 迭代/25k 节点（§78）。增量 = neighbor_centroid 初始化 + 局部松弛（§82–§85）。
- **D-115 持久化**：JSON + BTreeMap 位置（HashMap 序列化顺序不稳定导致校验和抖动——测试捕获）；payload 校验和（checksum 归零 FNV）；validate = format/map/analysis 版本 + dataset hash + checksum 全过（§67/§335–§340），任一不符 → 丢弃重建（derived data §65，不碰 M3）。
## D-116..D-119 Music Map 构建/渲染/交互（2026-10-05，M9 中篇）

- **D-116 构建服务**（build.rs）：请求时冻结 dataset（hash 锁定 §243–§244）；结束时 hash 已变 → Stale 而非 Ready（§245/§246）；generation（AtomicU64）保证旧 build 迟到不覆盖新 build（§166/§167）；相同 dataset hash 的活跃 build 去重（§165）；取消复用 M8 CancelToken；BuildKind 阈值 30%（§171/§172 显式常量起点，benchmark 复核留待 PERF）。
- **D-117 免重算恢复**：PersistedMap 增加 kNN 边 + graph/cluster 算法版本字段；dataset hash 未变 → restore_map 直接重建快照（图校验 + 非有限位置/簇统计检查，§174/§189/§190），布局/图/聚类零重算；失效/损坏 → 删除重建（derived data §65/§187，不碰 M3 §185）。
- **D-118 shell map IPC**（map_ipc.rs MapHost）：节点元数据（title/artist/bpm/key/energy/brightness/loudness）不持久化——每次快照从 M3/M8 数据库现读（M3 是唯一真相，避免第二份元数据副本）；只取 Completed 行的 embedding（§239 绝不用未完成分析造假图）；持久化 temp+rename 原子写（§186）；惰性 restore 每运行只评估一次；查询命令 = status/overview/nodes（视口批量 §229/§230）/clusters/neighbors（真选中才 IPC §159）/track/build/cancel。
- **D-119 前端架构**：mapStore 只存 8 个白名单字段（§145，测试锁定形状）；MapDataModel = 非响应式 typed arrays + 空间 hash（§146/§147/§156，hit testing 本地零 IPC §158）；单 Canvas2D 渲染器（§149；§194 fallback 语义由 Canvas2D 天然满足）；按需渲染无 idle rAF（§197/§198）；边仅选中邻域 ≤30 条（§118/§119）；配色全部稳定映射——簇=黄金角散列色板、key=12 pitch class 固定色相、energy/brightness=冷→暖标量、缺失=中性灰（§113/§213/§216）；视图持久化 localStorage 按 dataset hash 键控（§180/§181——UI 域状态不入 Rust settings；损坏/非有限 → 缺省 fit）。
