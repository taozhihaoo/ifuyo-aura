# Changelog

## M8 中篇（2026-10-05）

### Added
- 特征引擎：LUFS（BS.1770-4）/Tempo（分数周期谐波 comb+beat grid）/Key（chroma+KK+Camelot）/Spectral/MFCC——全部真实算法+合成 fixture 验证
- 规范预处理（22050/mono/线性重采样）+ 内部 radix-2 FFT
- 分析管线：单遍共享解码/取消/进度/clock 注入/特征隔离
- 调度器（优先级/取消/有界重试）+ 仓库 trait + InMemory + TrackAnalysisSummary + cosine 相似度原语
- 文档：D-098..D-104、ARCHITECTURE 算法节、PROGRESS/CHANGELOG

## M8 上篇（2026-10-05）

### Added
- aura-analysis 实装：domain model/status/availability/版本轴/analysis_key；EmbeddingProvider + 自有谱统计嵌入（32 维 L2，逐位确定）；M1 解码适配（mono downmix）；AnalysisStore（M3 迁移 v2 单表，单事务，替换/版本 Stale 检测）
- 文档：D-095..D-097、ARCHITECTURE 分析节、PROGRESS/CHANGELOG

### Notes
- 特征引擎（BPM/Key/LUFS/Spectral/MFCC）= M8 中篇；运行时/集成/门禁 = M8 下篇；零网络、零 fake analysis

## M7 下篇（2026-10-05）

### Added
- Three.js Dreamscape 渲染器（程序化地形/实例化/质量档/Auto 迟滞/节流/context 恢复/pixelRatio cap）
- M6 集成：Aurora 互斥、palette 四命令、Esc 分层退出、幂等 IPC + revision
- validate_world/快照 integrity/历史上限/Frozen；stress_fuzz（病态 seed+fuzz 500 轮+malformed 快照）
- 真实构建截图 ×8（含 Dreamscape 世界）；PERF M7 节；D-088..D-094

### Fixed
- body grid 居中导致全 app 布局收缩（Phase 0 遗留）；manifest serde 大小写致前端守卫拒绝；渲染器初始 setSize 缺失；WebGL 构造异常未捕获

### Notes
- M8（BPM/Key/LUFS/MFCC/embedding）零偷渡；VisualInfluence 扩展点 = VisualTelemetry 结构（§205/§206 null 时完整运行）

## M7 上篇（2026-10-05）

### Added
- `crates/aura-synth` 正式实装：确定性 seed 系统（DS 人类格式/world_id/版本轴）、RNG 命名流、Genome+受控 mutation、Environment（tick→相位/天气/光态）+约束 Palette、Biome×5、World 拓扑（地形/放置/预算）、稀有事件框架（日志/重放/过期）、SceneManifest+scene_hash、GeneratorV1、Region 流式生成（worker/取消/LRU）、Snapshot+Session
- shell：Dreamscape 会话装配 + `dreamscape.json` 逻辑态持久化 + 9 个 typed IPC 命令；ui 错误码接线
- 文档：D-079..D-087、ARCHITECTURE M7 节、PROGRESS/CHANGELOG

### Notes
- 渲染/集成/门禁 = M7 下篇（§128–330）；M2 禁止项（BPM/Key/…）零偷渡

## M6（2026-10-05）

### Added
- 桌面 shell（frameless + 自绘标题栏 + Mica/Win10 fallback）、Now Playing、Library（FTS5/虚拟化/扫描进度）、Command Palette、Queue、Mini Player、Map 壳、Settings（语言/缩放/reducedMotion/EQ/loudness/crossfade/Mica）
- `play_input`：Open File/URL 统一经 M4 Source Resolver（§275–277）；`library_stats`
- Aurora：WebGL fbm 视觉（30/8/2fps 节流、per-track seed、CSS fallback、VISUAL_TEST_MODE、data-fps 实测）
- 错误体系：AppError code→i18n 映射链 + 页面级 FeatureBoundary + NoticeBar
- 测试：vitest 42（a11y/键盘矩阵/挂载卫生/queue 引擎/format/errors）+ 桌面 smoke bin（真实 Wry + 计时）+ perf_ui 基线
- 截图管线 scripts/capture-screens.ps1（真实构建，docs/screenshots ×6）
- README 特性矩阵/截图/限制/构建说明；SECURITY/PRIVACY/ARCHITECTURE/PERF M6 章节；D-067..D-078

### Fixed
- mini 窗口 capabilities 缺权限导致创建被拒；裸 release 构建走 devUrl（custom-protocol feature）；整页幽灵滚动条；palette IME 输入路径（Ctrl+M 直达）

### Notes
- 无遥测；截图均来自 custom-protocol release 构建真实运行
