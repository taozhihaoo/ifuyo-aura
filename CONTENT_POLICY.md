# CONTENT_POLICY

> 本文是正式内容策略。任何来源系统、缓存系统、UI 的实现与它冲突时，以本文为准。政策上游依据：`Spec/ifuyo_Aura--Final_Commercial_Product_Charter.md` §2、§3。

## 一、明确禁止（不实现、不留接口、不留开关）

- 商业平台音频抓取：YouTube、Spotify、网易云音乐、QQ 音乐、Apple Music 等平台的音频抽取；
- yt-dlp 类 extraction 行为；
- DRM 破解：Widevine、FairPlay、SAMPLE-AES 解密、加密 HLS 解密；
- 破解签名或加密参数；
- 登录态接口模拟 / 登录态抓取；
- 商业平台私有 API 音频访问；
- 网络歌词抓取。

用户输入命中 `config/unsupported_platforms.json` 的平台时：**不发出任何请求**，直接返回明确提示：

> 该平台不受支持，请使用本地文件或开放来源。

## 二、合法来源

- 本地音乐文件、用户选择的音乐目录；
- 合法 HTTP/HTTPS 直链音频；
- 网络电台、Radio Browser；
- Dreamscape（内置生成，无网络）；
- 后续合法开放来源（Podcast RSS、未加密 HLS、Subsonic / OpenSubsonic、Jamendo、Internet Archive 等）。

分级与实现状态见 [SOURCES.md](SOURCES.md)。

## 三、缓存与许可原则

每个 Source 必须显式声明缓存策略：

| 策略 | 含义 |
|---|---|
| `Allowed` | 可以持久缓存 |
| `SessionOnly` | 只允许当前会话缓存，禁止持久化 |
| `Forbidden` | 禁止任何形式写盘；代码结构上保证所有执行路径均不写缓存目录 |

未知来源默认 `SessionOnly`。

任何带许可证信息的曲目必须展示：License、Attribution、Source、Source URL、额外使用限制。"保存到音乐库"时，许可与来源信息必须能够写入标签或旁车元数据。

## 四、识别机制

`config/unsupported_platforms.json` 是不支持平台的事实来源（域名模式列表）。来源系统 resolve 前必须先匹配该表；命中即拒绝。新增禁止平台只改配置表，不改业务代码。
