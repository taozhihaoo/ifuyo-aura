# SOURCES

ifuyo Aura 的音频来源分级与实现状态。政策依据见 [CONTENT_POLICY.md](CONTENT_POLICY.md)。

## 来源等级

### [A] 必须（Charter 强制）

| 来源 | 缓存策略 | 状态 |
|---|---|---|
| 本地音乐文件 / 用户选择目录 | Allowed | M3 |
| 合法 HTTP/HTTPS 直链音频 | 按来源声明 | M4 |
| 网络电台（ICY） | SessionOnly（直播流不持久化） | M4 |
| Radio Browser | SessionOnly（发现服务） | M4 |
| Dreamscape（内置生成） | 无网络 | M7 |

### [B] 应当

| 来源 | 缓存策略 | 状态 |
|---|---|---|
| Podcast RSS | 按来源声明 | M4+ |
| 未加密 HLS | SessionOnly | M4+ |
| Subsonic / OpenSubsonic | 按来源声明 | M11 |
| Jamendo | Allowed（CC 许可来源） | M11 |
| Internet Archive | Allowed | M11 |

### [C] 加分

更多合法开放来源、更丰富元数据、更高级发现机制。当前无计划清单。

### 未知来源

`CachePolicy = SessionOnly`（只允许会话内缓存，禁止持久化）。

## 明确禁止（不实现，见 CONTENT_POLICY.md）

商业平台音频抽取（YouTube / Spotify / 网易云 / QQ 音乐 / Apple Music 等）、yt-dlp 类行为、DRM 破解、登录态抓取、网络歌词抓取。不支持平台 URL 通过 `config/unsupported_platforms.json` 识别并直接拒绝，不发出任何请求。

## 许可与署名

任何带许可证信息的曲目必须展示 License / Attribution / Source / Source URL / 额外限制；"保存到音乐库"时这些信息必须写入标签或旁车元数据。

## 演示内容

仓库不包含任何真实音乐。演示内容只通过合法开放来源脚本在运行时获得（脚本随对应 Milestone 提供，当前不存在）。
