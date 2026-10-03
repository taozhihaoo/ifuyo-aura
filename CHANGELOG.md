# Changelog

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
