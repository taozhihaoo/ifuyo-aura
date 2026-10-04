# PRIVACY

> 本文描述真实行为，不是营销文案。软件行为变化时必须同步更新本文。政策上游依据：Charter §32–33、§35。

## 零遥测

ifuyo Aura 不含任何遥测、统计、崩溃上报或用户行为收集。不存在"匿名统计"。

## 本地保存什么（全部在本机）

| 数据 | 位置 | 说明 |
|---|---|---|
| 音乐库索引 / 播放统计 | `%APPDATA%\ifuyo\ifuyo Aura\`（SQLite） | M3 起 |
| 音频分析结果 | 同上数据库 | M8 起 |
| Music Map 布局与派生数据 | 同上数据库（独立 store） | M9 起 |
| 网络缓存 | `%APPDATA%\ifuyo\ifuyo Aura\cache`（仅 `Allowed` 来源） | M5 起 |
| 设置 | 本机配置文件 | M6 起 |
| 日志 | `%APPDATA%\ifuyo\ifuyo Aura\logs`（rolling） | 日志中绝不出现 password / token / API key / 私有凭据 |

不自动上传任何内容。用户可随时删除以上目录。

## 什么时候联网

默认状态下（首次安装、无任何在线操作）：**不联网**。

联网只发生在以下情形：

| 行为 | 访问 | 默认状态 |
|---|---|---|
| 播放 HTTP(S) 直链 / 电台流 | 该流所在服务器 | 用户主动触发才发生 |
| Radio Browser 电台搜索 | Radio Browser API（DNS discovery + 镜像） | 用户主动搜索才发生 |
| Radio Browser click reporting | Radio Browser | **默认关闭** |
| 更新检查 | GitHub Releases / 官方 site | **默认关闭**，首次使用时询问 |
| Subsonic / Jamendo / Internet Archive | 用户配置的自建/公开服务 | 用户主动添加来源才发生 |

局域网地址仅用于用户自建服务器；首次访问非公网地址需一次性确认。

## 敏感凭据

Subsonic 等服务的凭据只能存入 **Windows Credential Manager**，不进入源码、不明文写盘。API key 永不出现在仓库或日志中。

## 外链

应用内外链只允许通过系统浏览器打开，且仅限 ifuyo.com 与项目仓库（白名单）。

## 网络来源行为（M4 起，全部用户主动触发）

| 场景 | 发生什么 | 触发条件 |
|---|---|---|
| 用户粘贴直链 URL | 请求该 URL 所在服务器 | 用户明确输入并播放 |
| Radio Browser 搜索 | 请求 Radio Browser 镜像（UA：`ifuyo Aura/<版本>`） | 用户主动搜索电台 |
| 播放网络电台 | 请求电台流服务器 | 用户选择该电台 |
| Radio Browser click 上报 | `/json/url/<uuid>` 计数请求 | **默认关闭**；仅在用户显式开启设置后发送（§47） |
| Podcast RSS / HLS | 未实现（B 级） | — |

- 没有任何后台自动联网：不预加载、不自动扫描、不自动拉取元数据/artwork（§4）。
- 来源失败不上传诊断；不静默切换第三方服务（§49）。
- 网络来源的 metadata 不自动写入本地音乐库数据库（§92）。
- 本阶段没有遥测（与其他章节一致，零例外）。

## M6 确认（§242）

- M6（含下篇）未新增遥测/分析/云数据/远程日志：UI 无 `fetch`/XHR/beacon（grep 零命中），新增 IPC 命令（play_input/library_stats）均为本地操作。
- 用户主动联网行为不变：direct URL / Radio Browser 仍走 M4 策略（§278 未降级）；Open URL 输入即确认私网（D-070，仅影响用户显式输入的地址）。
- localStorage/sessionStorage：零使用（纯 UI 偏好亦由 Rust 侧 settings JSON 持久化，§200）。凭据类数据不落任何前端存储（§201）。
- `VisualTelemetry`（Aurora 渲染输入：isPlaying/progress/volume）为进程内数据结构，不离开进程，与"遥测"无关。

## M7 确认（§154/§236–§239）

- 零新增遥测/上传：渲染诊断（fps/frame_time/objects/triangles）仅本地 `canvas[data-fps]` 与内存（§155），无网络出站。
- Dreamscape 全程序化：无在线天气/日出 API（§237/§302）、无在线资产下载（§303）、seed 不出本机（§238）。
- 断网完整可用（§301）：世界生成/演化/快照全链路无网络。
- 快照存 app_data（§223），仅逻辑态，可随时删除不影响音乐库。

## M8 确认（§235–§239/§365）

- 分析 entirely local：raw audio/metadata/vectors/results/diagnostics 零上传（§235）。
- 零遥测：无 analysis/model usage/embedding/cloud error telemetry（§236/§379 grep 零命中）。
- 日志：不记录 raw PCM/完整向量/敏感路径（§237/§239——当前无任何运行时日志输出）。
- diagnostics 仅本地（§238）：summary/状态经 IPC 按需查询（§315 command/query 而非 push）。
## M9 中篇确认（Music Map）

- 地图 = derived data，全部可由本地 M3/M8 数据重建；map.json 存 app_data/map/（不进音乐文件夹 §184），损坏即删除重建，不触碰 M3 库（§185）。
- 节点元数据不落第二份持久化副本——渲染时从本地库现读；零遥测/零上传/零网络（构建、查询、渲染全链路）。
- 视图状态（zoom/pan）存 webview localStorage（本机 UI 偏好），按 dataset hash 键控，不含个人信息。
## M9 下篇确认（§358–§363/§547）

Music Map 明确零上传清单：**audio、embeddings、analysis 结果、map 数据（位置/簇/图）、地图交互（选择/搜索/过滤/视图）**——全部只存本机，不出网络。

- 无云向量库（§360）：无 Pinecone/hosted vector DB/cloud similarity service，相似度全部本地 exact 计算。
- 断网完整可用（§361）：已有 M8 分析存在时，构建/浏览/搜索/过滤/播放全链路离线工作。
- 无遥测（§358）：不记录 map usage/selected tracks/similarity queries/cluster interactions/screenshots/performance metrics（§491 grep 零命中）。
- map.json（位置/簇/边 + 版本校验和）存 app_data/map/，可随时删除，下次构建全量重建（derived data）。
## M10 上篇确认（§128–§132）

- Windows 只看到当前播放所需 metadata：title/artist/album/状态/位置/时长/可选封面（§129）——绝不暴露全库、播放历史、嵌入、分析向量、Music Map/Dreamscape 数据。
- 零新增遥测（§128/§131）：不统计不上传按键次数/播放行为；集成 diagnostics 仅本地内存（§225–§227）。
- SMTC/托盘/单实例完全离线可用（§126/§127）；不依赖任何云服务（§360 同精神）。
- 托盘菜单只显示当前曲目行（§132 合理系统媒体体验），无库列表。
