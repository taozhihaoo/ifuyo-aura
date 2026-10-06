# ifuyo Aura — M14 音乐管理器升级计划（格式 · 用户资产 · 元数据 · 健康中心）

> 状态：计划（待用户按期下达执行）
> 来源：内部差距分析 + 两份外部结论（"结论一"功能清单型 / "结论二"能力簇型）合并裁决，
> 全部关键判断经本仓库代码事实校准（见 §2 已验证事实）。
> 编号说明：M13.x 已被界面优化三轮占用（见 PROGRESS）；M13 编号原保留给 Steamworks
> （待用户提示词）——本计划启用 **M14**，Steamworks 顺延 M15，避免混淆。

---

## 1. 定位与总判断

Aura 已完成"现代播放器"主体（确定性引擎/现代 UI/分析/Map/安全/发布工程），当前短板
**不在引擎、不在再加视觉效果**，而在四支柱中的两根：

```text
        IFUYO Aura
            │
  ┌─────────┼─────────┐
Playback  Library   Exploration
（强）    （弱）      （强/差异化）
```

对标 foobar2000 / MusicBee / AIMP（管理深度）与 Roon（输出与元数据呈现）：
缺的是 **格式兼容、用户资产（歌单/统计/收藏）、元数据编辑、输出控制、库健康** 五簇能力。
对标范围明确排除：云曲库/账号/推荐/社交/会员/广告（与 local-first + zero-telemetry 立场冲突，
**有意不追**，两份外部结论一致）。

## 2. 已验证的代码/依赖事实（裁决依据）

| # | 事实 | 验证方式 |
| --- | --- | --- |
| F1 | symphonia 0.6.1 具备 `aac`/`alac` codec 与 `isomp4` 容器 feature（默认未启用）；`all-codecs` 列表**无 opus** | registry Cargo.toml |
| F2 | cpal 公开 API **不暴露 WASAPI 独占模式**——独占=绕开 cpal 自建 windows-rs WASAPI 栈（季度级工程） | cpal 源码 |
| F3 | 引擎 consumer 为一次性 detach（所有权移交 CPAL 流，销毁不归还）——**设备切换需 consumer 换绑改造** | engine/player.rs |
| F4 | 扫描器/watcher 重扫以**文件标签刷新 DB**——"只改数据库不改文件"的标签编辑会被重扫覆盖 → 必须先做 identity **覆盖层** | scanner/identity 代码 |
| F5 | lofty 0.22 已在依赖树（M3 引入、M12 审计过），写入标签可行且与读取往返一致（examples/set_uslt 实测） | 代码+实测 |
| F6 | ffmpeg `lyrics=` 元数据**不落 USLT 帧**（只写文本元数据）——歌词标签 fixture 必须用 lofty 写 | fixture 实测 |
| F7 | 数据层部分现成：`playlists`/`playback_stats`/`album_artist_id`/`library_roots` 表已存在，缺的是字段与 UI | db/schema |
| F8 | AIFF 经 riff feature 支持；**Opus 为真缺口**（F1），后置 | 同 F1 |

## 3. 分期计划

### M14a 格式与兼容（P0，先行）

- [ ] **symphonia feature 开启**：`aac` + `isomp4` + `alac` + `aiff`（F1）——M4A/AAC/ALAC/AIFF
      解码进入支持矩阵；扫描器扩展名、探测、测试矩阵同步
- [ ] **CUE 整轨**：cue 解析器（TRACK/INDEX/REM）→ 虚拟子轨（一文件多曲、seek 映射文件偏移、
      库入车为子轨、封面/标签继承）
- [ ] **编码检测**：引入 `encoding_rs`（走 M12 供应链审批）——ID3v1/v2.3 Latin-1 声明下的
      GBK 乱码启发式修复（自动修复 + 设置开关双轨防误判）；**同时解决 GBK `.lrc` 歌词**
      （M13.3 后置项并入）
- [ ] 验收：真实格式矩阵（M4A/AAC/ALAC/AIFF/整轨+CUE/GBK 标签/GBK lrc）扫描→播放→seek→
      封面全链路 verify；symphonia 无 Opus（F8）如实记录后置

### M14b 用户资产（P0）

- [ ] **Track Memory**：`playback_stats` 扩展（rating/favorite/skip_count/first_played/
      resume_position）+ 记录埋点（播放完成/跳过判定）
- [ ] **播放列表 UI**：新建/重命名/删除/排序/批量加入移除（数据层 F7 现成）；侧栏歌单入口
- [ ] **M3U/M3U8 导入导出**（XSPF 顺带）；**拖放导入**（文件/文件夹进窗口即扫描）
- [ ] **队列升级**：拖拽排序、"下一首播放"（插入而非替换）
- [ ] **睡眠定时**（淡出停止）+ **播放时阻止系统睡眠**（SetThreadExecutionState）
- [ ] 验收：歌单全生命周期 verify + 统计字段入库断言 + 拖放/定时 CDP 实测

### M14c 音乐管理器（P1，覆盖层设计先行）

- [ ] **覆盖层设计先行**（F4，本计划新增的关键前置）：identity 键控的"用户覆盖表"，
      重扫合并策略=编辑字段优先；先出设计文档（DECISIONS）再动工
- [ ] **Tag Editor**：单曲/批量（title/artist/album/albumArtist/genre/year/track/disc/
      composer/comment/compilation/lyrics/artwork）；流程=DB Edit → Preview → Validate →
      **显式**写回文件（lofty 写入，F5 已验证）；不写回则覆盖层保证重扫不丢
- [ ] **Library Health 中心**：缺失文件/重复曲目/不可读/缺封面/缺元数据/不一致专辑艺术家
      一览（identity/watcher/diagnostics 基础现成）
- [ ] **Signal Path 面板**（快赢，可提前任意轮）：源格式→重采样/EQ/响度/crossfade→输出
      的当前链路可视化（引擎状态已知，纯呈现）
- [ ] 验收：批量编辑预览/应用/写回/重扫不丢失 E2E；Health 各类异常注入实测

### M14d 体验进阶（P1/P2，依赖前几期）

- [ ] **智能歌单**（规则式：流派/年份/**BPM/调性/响度**/rating/play_count——分析特征是
      独有优势；依赖 M14b 字段）
- [ ] **文件整理**（Preview→Plan→Apply→Verify 模板移动，绝不直接 Rename；依赖 M14c 覆盖层）
- [ ] **桌面歌词**（独立置顶+鼠标穿透窗，复用 Mini Player 多窗口管线；歌词面板 M13.3 为数据源）
- [ ] **Smart Queue / Auto-DJ**（"接下来 20 首不连续同艺术家、优先高评分、BPM/调性连贯"——
      分析+Map 特征闭环，差异化主打）
- [ ] 验收：各功能独立 verify 脚本扩展

## 4. 明确后置

- **Opus**（symphonia 0.6 无 codec，F8；等 0.7 或独立解码器）
- **WASAPI 独占 / bit-perfect / ASIO**（F2，季度级自建栈；发烧受众后置）
- **DSD（DSF/DFF/DoP）**、**TTML 逐字歌词**、**APE/WMA/WV**（长尾格式）
- **古典作品/乐章结构**、**多房间/DLNA/Subsonic**、**CD 抓轨/刻录/转换器**
- **Last.fm/ListenBrainz 上报**（如做必须显式 opt-in）

## 5. 明确不做（立场）

账号体系、云同步、在线曲库、推荐算法、社交评论、会员广告、DRM、插件/皮肤生态
（最多保留 Provider API 形态的歌词/元数据源扩展点）、Roon 式知识图谱
（Music Map 已是差异化路线，不照抄）。

## 6. 外部结论裁决记录

| 议题 | 结论一 | 结论二 | 裁决 |
| --- | --- | --- | --- |
| M4A 解码 | Windows 自带解码 | （未提策略） | **拒绝 MF 双路径**：symphonia feature 即可（F1），保持单一确定性解码栈 |
| Opus 进 P0 | 未提 | P0 | **后置**（F8） |
| CUE 排期 | P0 首批 | P1 | **P0 首批**（国内整轨存量，实现独立） |
| 编码检测 | 提前 P0 | 未单列 | **采纳提前**（含 GBK 歌词合并解决） |
| 标签编辑 DB-first | 未展开 | 提出 | **采纳，但前置覆盖层设计**（F4 为结论二盲区） |
| WASAPI 独占 | P2 | P0 ★★★★★ | **后置**（F2 工程量 vs 受众） |
| 输出设备选择 | P0 | 未提（只讲独占） | **拆两步**：跟随默认设备（先）/手动指定（consumer 换绑后，F3） |
| Track Memory/Health/Signal Path/文件整理 | 未展开 | 提出 | **采纳** |
| 智能歌单 | P1 | P0 | **M14d**（依赖 M14b 字段） |
| 桌面歌词首批 | 进 6 项 | 提及未排期 | **M14d**（独立窗口成本，非纯 UI） |
| 队列拖拽/下一首播放/阻止睡眠 | 部分提及 | 均未提 | **纳入 M14b** |
| 插件/皮肤/Roon 图谱/云生态 | 不追 | 不追 | **一致：不做** |

## 7. 门禁口径（各期统一）

fmt/clippy 干净、workspace --locked 全绿、vitest 全绿、verify-library/verify-playback 扩展断言、
tauri build + CDP 实测、格式期加真实文件矩阵（M4A/AAC/ALAC/AIFF/CUE/GBK 各一）；
如实记录：M11-A 生产签名 BLOCKED 不在本计划范围，发布前必须解决。
