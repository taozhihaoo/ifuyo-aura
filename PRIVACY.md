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
