# NETWORK BEHAVIOR（M12 §45/§87/§9）

> 本文档描述 ifuyo Aura **真实**的网络行为——何时联网、请求什么、
> 不发送什么。本地优先：断网时除网络来源（M4）外全部功能可用。

## 出网面完整清单（FACT，§45）

| 功能 | 触发 | 目标 | 发送内容 | 用户控制 |
| --- | --- | --- | --- | --- |
| Internet Radio | 用户添加电台 URL | 用户指定的电台 | 标准 HTTP 请求（无额外数据） | 移除电台即停止 |
| Radio Browser | 用户打开电台发现 | rbradio 实例（用户可选） | 无查询参数即匿名 | 不使用即零请求（默认关闭点击上报） |
| Direct HTTP 流 | 用户打开 http(s) 音频 URL | 该 URL | 标准 GET（Range） | 关闭播放即停止 |
| 更新检查 | 启动后 15s（可关）或手动 | GitHub Releases latest.json（M11） | 版本/平台/架构（官方协议字段） | 设置开关 §84/§85 |
| Release 发布 | 维护者 CI 动作 | GitHub | artifact | 非运行时行为 |

## 明确不发送（§45）

音乐库内容、播放列表、艺术家/曲目历史、个人路径、设备标识、遥测、
分析数据、Music Map/Dreamscape 数据。**零遥测**（§44）。

## 安全策略（§9/§65）

- scheme 白名单（http/https）；凭据 URL 拒绝；封锁表（config/
  unsupported_platforms.json）零请求拒绝；私网策略（allow_private =
  用户显式输入即确认，D-070）；重定向有界；响应/缓冲有界。
- 更新端点：production = HTTPS（§9）；dev = 本地 fake server
  （`dangerousInsecureTransportProtocol` 显式声明，配置隔离 §77）。

## Offline（§36/§127）

断网：本地播放/库/缓存/分析/Dreamscape/Map/SMTC/托盘/单实例全部正常；
更新检查失败仅影响更新功能（typed Network 错误，不阻塞启动）。
