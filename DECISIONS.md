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
