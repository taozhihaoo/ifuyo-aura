# ifuyo Aura
# M4 — Source System / Local / Direct HTTP / Internet Radio / ICY / Radio Browser

你现在进入 ifuyo Aura 项目的 **M4：来源系统与网络音频**。

项目正式名称：

`ifuyo Aura`

当前阶段目标：

> **在已经完成并通过审计的 M1 Audio Core、M2 Playback Quality、M3 Music Library 基础上，建立统一、合法、安全、可测试、可扩展的 Source 系统。**

本阶段负责：

- Source trait；
- 本地文件 Source；
- 合法 HTTP/HTTPS 直链 Source；
- Internet Radio Source；
- ICY metadata；
- Radio Browser 目录；
- Source resolution；
- 网络安全策略；
- Source metadata / license / attribution / cache policy；
- 网络 Source 与 M1/M2 Audio Core 的干净连接；
- B 级 Podcast RSS；
- B 级未加密 HLS；
- B 级 Windows Media Foundation AAC 路径。

**本阶段必须以 M1 + M2 + M3 的真实代码为唯一基线。**

不得假设前序阶段实现方式与提示词完全一致。

必须先审计，再增量实现。

---

# 0. 本阶段绝对边界

## A 级必须实现

1. `aura-sources` 正式建立或补全。
2. 统一 `Source` abstraction。
3. `search(query)`。
4. `resolve(input)`。
5. `Resolved`。
6. local file Source。
7. direct HTTP/HTTPS audio Source。
8. Internet Radio Source。
9. ICY protocol parsing。
10. ICY metadata stripping。
11. `StreamTitle` 提取。
12. Radio Browser source discovery。
13. Radio Browser mirror discovery。
14. Radio Browser mirror fallback。
15. Radio Browser `url_resolved` 使用。
16. codec filtering。
17. descriptive User-Agent。
18. redirect limit。
19. connect/read timeout。
20. response body size limits。
21. URL scheme validation。
22. local/private network policy。
23. malformed input safety。
24. source-level errors。
25. cache policy declaration。
26. license metadata。
27. attribution metadata。
28. Source → Audio Core 的适配层。
29. non-seekable streaming support。
30. offline-safe behavior。
31. 全部 Source parser deterministic tests。
32. online source tests 使用本地 fixtures，不访问真实网络。
33. M1/M2/M3 regression tests。

---

## B 级

如果 A 级全部稳定后：

1. Podcast RSS。
2. 未加密 HLS。
3. HLS live refresh。
4. HLS media sequence。
5. Windows Media Foundation AAC。
6. AAC/m4a streaming adapter。
7. 更完整的 source diagnostics。

---

# 1. 本阶段禁止

不要实现：

- network cache persistence；
- chunk cache；
- Range cache；
- LRU cache；
- pin；
- SQLite cache metadata；
- Jamendo；
- Internet Archive；
- Subsonic；
- online metadata enrichment；
- online artwork；
- online lyrics；
- Music Map；
- BPM；
- Key；
- LUFS analysis；
- MFCC；
- embedding；
- Dreamscape；
- Aurora；
- SMTC；
- tray；
- installer；
- updater；
- website；
- Steam；
- auto mix；
- WSOLA。

特别注意：

> **M5 才是 Network Cache。**

M4 允许：

- stream；
- buffering；
- 必要的内存缓冲；

但不得把 M5 的持久化缓存提前实现。

---

# 2. 来源策略基线

根据项目总章程：

```text
CachePolicy =
    Allowed
    SessionOnly
    Forbidden
```

Source 必须显式返回：

```text
Resolved {
    streams,
    meta,
    license,
    cache_policy,
    attribution
}
```

未知：

```text
SessionOnly
```

不得默认：

`Allowed`

---

# 3. 合法来源硬规则

禁止：

- 商业平台音频抓取；
- yt-dlp 类 extraction；
- 登录态音频访问；
- DRM；
- Widevine；
- FairPlay；
- SAMPLE-AES；
- protected HLS；
- 破解签名；
- 破解加密参数。

对于：

- YouTube；
- Spotify；
- 网易云；
- QQ Music；
- Apple Music；

等明确禁止平台：

必须根据：

`config/unsupported_platforms.json`

识别。

识别后：

> **不得发送任何请求。**

必须返回：

```text
该平台不受支持，请使用本地文件或开放来源
```

这一要求必须通过测试证明。

---

# 4. 联网触发原则

项目当前：

> 本地优先、零遥测。

M4 所有联网行为必须：

> 由用户明确触发。

例如：

- 用户输入 HTTP URL；
- 用户搜索 Radio Browser；
- 用户选择网络电台；
- 用户输入 RSS。

不得：

- 后台自动扫描互联网；
- 开机自动访问来源；
- 自动抓取元数据；
- 自动访问 artwork；
- 自动预加载未知网络来源。

---

# 5. 开始前 Baseline Audit

修改之前执行：

```powershell id="y6q4b2"
git status
git branch
git log --oneline -15
cargo metadata
cargo tree
cargo test --workspace
cargo deny check
```

检查：

```text id="z2s8cp"
crates/aura-core
crates/aura-testkit
crates/aura-library
crates/aura-sources
src-tauri
ui
config
docs
```

确认：

- M1；
- M2；
- M3；
- current HEAD；
- 当前依赖；
- 当前测试数量；
- 当前 warning；
- lockfile；
- `cargo-deny`；
- MSRV；
- `aura-sources` 是否已有代码。

输出：

# M4 Baseline Audit

A. Git  
B. M1 Status  
C. M2 Status  
D. M3 Status  
E. Workspace  
F. Existing Source Code  
G. Dependencies  
H. Security  
I. Risks

---

# 6. 前序回归原则

如果 M1/M2/M3：

存在真正影响 M4 的缺陷：

先做：

> 最小必要修复。

不得：

- 重写 M1；
- 重写 M2；
- 重写 M3；
- 借机重新设计整个系统。

所有必要修复：

写入：

`DECISIONS.md`

与：

`PROGRESS.md`

---

# 7. `aura-sources` 职责

`aura-sources`：

只负责：

```text
Input
 ↓
Source Detection
 ↓
Source Resolution
 ↓
Resolved Stream
 ↓
Metadata / License / Attribution
```

不得负责：

- audio output；
- WASAPI；
- UI；
- persistent cache；
- Music Map；
- Dreamscape。

---

# 8. Source Trait

建立统一：

```rust id="e5nfjp"
trait Source
```

至少表达：

```text
search(query)
resolve(input)
```

具体 async/sync 设计：

根据当前 workspace 实际架构决定。

优先：

> 简单、可测试、清晰的异步边界。

---

# 9. Resolved Model

建立：

```text id="im3k0j"
Resolved
```

至少包含：

```text
streams
meta
license
cache_policy
attribution
```

建议：

```text id="kxd4o7"
ResolvedStream
  url
  mime
  codec
  bitrate
  sample_rate
  channels
  seekable
  live
```

具体字段：

根据实际需求增删。

不得创建巨大万能结构。

---

# 10. Source Identity

每个 Source 必须有明确：

```text id="lksmqg"
SourceKind
```

例如：

```text
Local
DirectHttp
InternetRadio
RadioBrowser
Podcast
Hls
```

未来来源可以继续扩展。

不得使用：

> free-form string everywhere

导致来源类型无法安全判断。

---

# 11. Source Errors

建立：

```text id="4hqtas"
SourceError
```

至少考虑：

```text
UnsupportedInput
UnsupportedPlatform
InvalidUrl
InvalidResponse
NetworkError
Timeout
RedirectLimit
BodyTooLarge
UnsupportedCodec
ProtectedStream
LicenseUnknown
SourceUnavailable
ParseError
```

错误必须：

- 可诊断；
- 可显示；
- 不 panic；
- 不泄漏凭据。

---

# 12. Source Resolver

建立：

```text id="y3wfpf"
resolve_input()
```

输入可能：

```text
local path
http URL
https URL
m3u8 URL
rss URL
radio station identifier
```

M4：

必须至少识别：

- local；
- http；
- https；
- unsupported commercial platform。

B 级：

- RSS；
- m3u8。

未知：

返回明确错误。

---

# 13. Resolver Order

推荐：

```text id="49z94c"
1. local filesystem path
2. supported URL scheme
3. known source domain
4. source-specific parser
5. unsupported platform check
6. unknown input
```

具体顺序如因安全原因调整：

必须记录。

关键：

> 禁止的平台必须在发送网络请求之前被拒绝。

---

# 14. URL Validation

URL 输入必须：

- parse；
- scheme validation；
- host validation；
- malformed authority rejection。

只允许：

```text
http
https
```

其余：

拒绝。

例如：

```text
file:
ftp:
data:
javascript:
blob:
ws:
wss:
```

不得直接交给网络 client。

---

# 15. Credential-in-URL

禁止：

```text id="l9j15q"
https://user:password@example.com/audio.mp3
```

或者：

```text
https://api:key@example.com
```

进入：

- logs；
- UI；
- Source metadata；
- persistent storage。

最安全策略：

> 默认拒绝带 credentials 的 URL。

如果以后确有合法使用场景：

单独设计。

M4 不放宽。

---

# 16. Redirect Policy

所有 HTTP Source：

最多：

```text
5 redirects
```

每次 redirect：

必须重新：

- validate URL；
- validate scheme；
- validate destination host；
- validate policy。

不能：

> 第一个 URL 合法，所以 redirect 到任何地址都自动允许。

---

# 17. Private Network / SSRF

根据总章程：

允许：

> 用户自建服务器 / 局域网地址。

但：

必须防止恶意跳转和隐式扫描。

建议：

- 明确识别 private/local address；
- 用户首次访问非公网地址时一次性确认；
- redirect 到不同网络范围时重新判断；
- 禁止自动探测内网；
- 禁止 localhost 扫描；
- 禁止通过 URL 列表枚举内网服务。

不得：

> 因为允许 LAN，就关闭全部 SSRF protection。

---

# 18. IP / Host Revalidation

如果 HTTP client resolve：

```text example.com → private IP
```

必须根据当前安全策略处理。

不能只检查：

> hostname 字符串。

真实行为必须写：

`SECURITY.md`

并测试。

---

# 19. HTTP Client

推荐：

`reqwest + rustls`

如果 M0 已经采用等价方案：

优先复用。

要求：

- connect timeout；
- read timeout；
- redirect limit；
- response size limit；
- descriptive User-Agent；
- cancellation。

---

# 20. User-Agent

所有外部请求：

使用：

```text id="gk7nkv"
ifuyo Aura/<version>
```

或：

项目正式名称对应的等价描述性格式。

不得：

- Chrome spoof；
- Firefox spoof；
- generic curl；
- 空 UA。

Radio Browser 必须使用描述性 UA。

---

# 21. HTTP Response Limits

所有不受信任网络响应：

必须有：

> 最大 body size。

尤其：

- JSON；
- RSS XML；
- m3u8；
- ICY metadata。

不能：

> 让服务器通过无限响应耗尽内存。

流式音频：

允许持续读取，但：

> metadata 控制块必须有单独大小限制。

---

# 22. Cancellation

网络请求：

必须可取消。

用户取消：

请求必须尽快结束。

不得：

> 一个失控网络请求永久阻塞 audio pipeline。

---

# 23. Local Source

[A]

本地 Source：

输入：

```text
Path
```

支持：

M3 已支持格式。

Source resolution：

需要提供：

- stream path；
- metadata；
- seekable；
- local cache policy。

本地：

```text
CachePolicy = Allowed
```

但：

M4 不实现 M5 cache。

---

# 24. Local Source 与 M3

M4 不复制 M3 的：

- metadata extraction；
- Track identity；
- DB insertion。

应该：

> 调用 `aura-library` 已有能力

或者：

由 application layer 组合。

不得：

`aura-sources` 再写一套 tag parser。

---

# 25. Direct HTTP Source

[A]

支持：

用户粘贴：

```text
http://...
https://...
```

解析。

目标：

> 合法的直接音频 URL。

---

# 26. Content-Type

如果服务器返回：

```text
audio/*
```

优先识别。

同时允许：

服务器 Content-Type 错误但 URL extension 明确为音频的情况：

必须通过：

> actual stream probing / decoder

判断。

不能：

> 只信 Content-Type。

也不能：

> 自动接受任意 binary response。

必须有安全上限和 codec detection。

---

# 27. Range / Seek

M4：

可以读取：

HTTP `Accept-Ranges` / `Range` 能力。

但是：

**不要实现 M5 persistent cache。**

Direct stream：

如果服务器支持 Range：

可以提供：

> seek-capable stream abstraction。

如果服务器不支持：

标记：

```text
seekable = false
```

不得伪造 seek。

---

# 28. Network Stream Abstraction

建立：

```text id="w1t2c4"
NetworkAudioStream
```

或者等价 abstraction。

至少表达：

- read；
- seek capability；
- cancel；
- metadata；
- source identity。

不得把 reqwest response 到处暴露。

---

# 29. Streaming Buffer

允许：

> bounded in-memory buffer。

容量：

必须有明确上限。

禁止：

> 无限 Vec 累积网络音频。

实时电台：

不能把整个 stream 放内存。

---

# 30. Internet Radio

[A]

Internet Radio Source：

特点：

- live；
- 通常 non-seekable；
- continuous；
- metadata embedded。

必须：

```text
seekable = false
live = true
```

除非真实 stream 明确支持其它能力。

---

# 31. ICY

实现：

```text id="xv3ykg"
Icy-MetaData: 1
```

请求头。

服务器返回：

```text
icy-metaint
```

之后：

```text
audio block
metadata block
audio block
metadata block
...
```

必须正确剥离 metadata。

---

# 32. ICY Parser

实现专门：

```text id="5gp6qm"
IcyReader
```

必须支持：

- partial reads；
- arbitrary chunk boundaries；
- metadata length byte；
- metadata padding；
- zero metadata length；
- malformed metadata。

不得假设：

> 一次 `read()` 就能读完整 block。

---

# 33. ICY Metadata Size

metadata length：

必须经过限制。

不能：

> 服务器声明巨大 metadata 导致内存分配。

设置安全最大值。

具体值：

根据协议实际情况与工程测试确定。

写入：

`DECISIONS.md`

---

# 34. StreamTitle

从 ICY metadata：

解析：

```text
StreamTitle
```

例如：

```text id="ey4p9e"
StreamTitle='Artist - Title';
```

必须：

- 提取；
- sanitize；
- UTF-8 安全转换；
- 发送给上层 metadata channel。

---

# 35. ICY Metadata Changes

如果：

StreamTitle 变化：

产生：

```text id="w8y0i6"
StreamMetadataEvent
```

例如：

```text
Artist
Title
```

本阶段：

只建立 backend event。

UI 后续处理。

---

# 36. ICY Malformed Data

如果：

- metadata 长度非法；
- 非 UTF-8；
- malformed key/value；
- metadata 过大；

不能：

让整个 audio source panic。

策略：

> 忽略 malformed metadata，但继续播放音频。

除非：

stream framing 已无法恢复。

---

# 37. Internet Radio Cache Policy

根据总章程：

```text id="jxur0j"
Forbidden
```

电台：

只允许：

> 内存滚动缓冲。

上限：

`<= 30 seconds`

不允许：

- chunk files；
- SQLite cache；
- disk cache；
- persistent buffering。

这一要求必须有测试。

---

# 38. Zero-Writes Test

使用 test sandbox：

启动电台 source。

播放若干：

synthetic chunks。

然后扫描指定 cache/app data 目录。

必须证明：

> 没有写入电台音频缓存。

---

# 39. Radio Browser

[A]

实现：

> Radio Browser directory source。

必须遵守：

不要写死单一服务器。

---

# 40. Radio Browser Discovery

使用：

```text id="jngdr2"
all.api.radio-browser.info
```

或章程中指定的：

`/json/servers`

机制。

目标：

获得：

> mirror/server list。

不得：

> 永久硬编码一个 API server。

---

# 41. Radio Browser Mirror Selection

获得 server list：

选择：

> 随机镜像。

要求：

- deterministic test mode 可以注入固定选择器；
- production 可以随机。

失败：

进入：

> fallback mirror。

---

# 42. Radio Browser Fallback

例如：

```text
Mirror A
↓ fail
Mirror B
↓ fail
Mirror C
```

最多尝试有限数量。

不得：

> 无限重试。

timeout：

必须：

短且可配置。

---

# 43. Radio Browser Request

所有请求：

使用：

```text
ifuyo Aura/<version>
```

User-Agent。

不得：

隐藏身份。

---

# 44. Station Listing

支持：

基本搜索：

```text
search(query)
```

至少能够按：

- name；
- country；
- language；
- codec；

进行基本过滤。

具体字段：

以实际 Radio Browser response 为准。

---

# 45. `url_resolved`

点播电台：

必须使用：

> `url_resolved`

而不是简单优先：

`url`

除非真实 response 缺失且明确允许 fallback。

最终必须记录：

> 实际使用哪个 URL 字段。

---

# 46. Codec Filtering

默认优先：

```text
MP3
OGG
FLAC
```

根据章程：

支持 codec filtering。

如果：

AAC：

走 M2/M4 的 WMF 路径；

如果实际不可用：

明确：

> 该电台编码暂不支持。

不能因为 codec 不支持：

panic。

---

# 47. Radio Browser Click Reporting

默认：

```text id="f3x0gl"
OFF
```

必须支持：

未来设置开关。

M4：

至少建立：

```text
click_reporting_enabled
```

以及：

发送逻辑。

但：

如果关闭：

> 不发送 click。

不能偷偷上报。

---

# 48. Privacy

更新：

`PRIVACY.md`

必须真实说明：

联网来源：

- direct URL；
- Radio Browser；
- podcast/RSS（如果 B 级完成）；
- HLS（如果 B 级完成）。

说明：

> Radio Browser click 默认关闭。

---

# 49. Radio Browser Offline Failure

如果无法：

- DNS；
- fetch servers；
- connect；
- parse response；

必须：

返回：

明确：

`SourceUnavailable`

不得：

- fallback 到未知域名；
- 自动搜索其他服务；
- 静默切换第三方服务。

---

# 50. Radio Browser Response Limits

Radio Browser：

同样必须：

- body size limit；
- timeout；
- parse limit；
- redirect validation。

不能因为它是公开 API：

就跳过安全控制。

---

# 51. License

Radio Browser directory metadata：

如果来源本身提供：

- station info；
- website；
- stream URL；

需要明确记录：

> source metadata

但不要擅自声明：

> 电台音频本身具有某种许可证。

电台站点的内容许可：

除非明确提供，否则：

> 不假定。

因此：

音频流的 cache policy：

保持总章程规定。

---

# 52. License Model

`ResolvedLicense`：

应该能表达：

```text
Unknown
Known(...)
```

不要：

> `license = CC0` 这种默认猜测。

只有明确证据：

才能声明具体 license。

---

# 53. Attribution

建立：

```text id="bf5pxx"
Attribution
```

至少能保存：

- provider；
- title/name；
- source URL；
- required attribution text。

没有：

> 明确署名要求

时：

不要编造。

---

# 54. Source Metadata

来源 metadata：

至少区分：

```text id="hp9l7v"
TrackMetadata
StreamMetadata
ProviderMetadata
```

不要所有东西都塞到一个：

`HashMap<String,String>`。

---

# 55. Source → Audio Core Adapter

M4 的关键边界：

```text id="9u2m7o"
Source
 ↓
ResolvedStream
 ↓
Stream Adapter
 ↓
aura-core Decoder Input
 ↓
Resampler
 ↓
Buffer
 ↓
Output
```

`aura-sources`：

不得：

- 操作 CPAL；
- 操作 WASAPI；
- 直接控制 output callback。

---

# 56. Seek Capability

每个 resolved stream：

必须明确：

```text
Seekable
NonSeekable
```

UI / application layer 后续：

才能决定是否显示 seek。

不能：

> 对 live stream 假装支持 seek。

---

# 57. Stream Type

至少区分：

```text
Finite
Live
UnknownDuration
```

M4：

必须确保：

Internet Radio：

`Live`

Direct finite audio：

通常：

`Finite / UnknownDuration`

---

# 58. Decoder Integration

M4 必须让：

Direct HTTP / Radio Browser：

最终能够：

> 进入 M1 的 decoder abstraction。

但：

对于：

- 不可 seek；
- streaming；

必须提供适配。

不要为网络来源复制另一套 decoder。

---

# 59. Error Propagation

网络错误：

必须能一路传递：

```text
SourceError
 ↓
Application Error
 ↓
Playback Error
```

但：

M4 不做最终 UI。

不得：

`unwrap()`。

---

# 60. Retry Policy

基本网络请求允许：

有限 retry。

例如：

- DNS transient；
- connect failure；
- 5xx。

不应该对：

- 4xx；
- unsupported codec；
- malformed response；
- forbidden platform；

反复重试。

具体策略：

记录 `DECISIONS.md`。

---

# 61. Retry + Audio Streaming

实时音频播放：

不要：

> 在 decoder callback 中 retry。

遇到网络中断：

由：

> Source / stream layer

管理。

Audio callback：

只处理已经进入 buffer 的 PCM。

---

# 62. Backpressure

网络读取速度：

可能大于：

音频消费速度。

必须：

> 有界缓冲。

不能：

无限增长。

网络读取速度小于消费速度：

应允许：

underrun。

交由 M2 既有机制处理。

---

# 63. Network Buffer 与 M1 Buffer

不要修改 M1 的核心 ring buffer 语义来适应网络。

推荐：

```text
Network byte stream
 ↓
Decoder
 ↓
PCM
 ↓
M1 audio ring buffer
```

network byte buffering：

属于 source layer。

audio PCM buffering：

属于 audio core。

---

# 64. Podcast RSS — [B]

只有 A 级全部稳定后再做。

支持：

RSS XML。

至少：

- channel title；
- episode title；
- enclosure URL；
- MIME type；
- optional duration。

使用：

`quick-xml`

或者当前 workspace 等价依赖。

---

# 65. RSS Security

必须：

- body limit；
- XML depth / entity safety；
- timeout；
- redirect policy；
- URL validation。

不得支持：

> 外部实体解析导致本地文件读取。

禁止：

XXE 类行为。

---

# 66. RSS Cache Policy

根据总章程：

只有：

> 用户点击下载

才：

`Allowed`

仅仅浏览 RSS：

不得持久化 episode audio。

M4：

如果实现下载：

必须经过明确 user action。

否则：

`SessionOnly`

---

# 67. HLS — [B]

只支持：

> 未加密 HLS。

支持：

- master playlist；
- media playlist；
- live refresh；
- media sequence。

---

# 68. HLS 禁止

遇到：

```text
EXT-X-KEY
```

或其它保护 / encryption：

立即拒绝：

```text id="9cew4y"
不支持受保护的流
```

不得：

- 解密；
- 请求 key；
- 破解参数；
- 下载受保护 segment。

---

# 69. HLS Redirect / Segment Security

playlist：

每个：

- master URL；
- media URL；
- segment URL；

都必须重新进行：

> URL / redirect / network policy validation。

不能因为 master playlist 合法：

就允许 segment 去任意 host。

---

# 70. HLS Segment Limits

必须限制：

- playlist size；
- segment size；
- number of queued segments；
- live refresh interval。

不能：

> 恶意 playlist 让客户端无限创建任务。

---

# 71. HLS Live Refresh

media playlist：

支持：

> media sequence

以及：

> 定期 refresh。

必须有：

> bounded task lifecycle。

停止播放：

refresh 必须停止。

---

# 72. HLS Cache Policy

M4：

默认：

`SessionOnly`

M5：

再实现：

> persistent cache。

---

# 73. Media Foundation AAC — [B]

官方发行版：

根据总章程：

不内置 AAC decoder。

Windows：

使用：

> Media Foundation

路径。

---

# 74. AAC Policy

不要启用：

```text
symphonia aac
```

官方 build。

不得添加：

```text
symphonia-adapter-fdk-aac
```

也不得添加：

GPL/LGPL codec library。

---

# 75. WMF Boundary

建立：

独立：

```text id="kdnv7d"
MediaFoundationDecoder
```

或者等价 adapter。

必须：

- Windows-specific implementation；
- isolation；
- clean error;
- no Tauri dependency in core abstraction。

---

# 76. WMF Non-Windows

虽然官方目标为 Windows：

crate：

应尽量：

> compile-time isolation。

非 Windows：

可以：

`UnsupportedPlatform`

而不是：

编译整个工程失败。

---

# 77. M4 Audio Format Support

M4 的 Source 层必须明确：

```text id="lslr83"
Local FLAC
Local MP3
...
Direct HTTP supported codec
Internet Radio supported codec
```

不要把：

> URL extension

当作：

> codec guarantee。

最终由 decoder probing 决定。

---

# 78. Unsupported Codec

必须返回：

```text id="p1jvfh"
UnsupportedCodec
```

例如：

服务器返回 AAC：

但 WMF 不可用：

显示：

> 该来源的音频编码暂不支持。

不要：

- 自动换来源；
- 自动下载；
- 自动使用未经批准 codec。

---

# 79. Parser Architecture

所有协议 parser：

应该尽可能：

> pure / deterministic。

例如：

```text
parse_icy_metadata()
parse_radio_browser_response()
parse_rss()
parse_m3u8()
```

输入：

fixture bytes

输出：

typed structure。

不直接：

- 网络；
- filesystem；
- UI。

---

# 80. Parser Robustness

任意：

- truncated；
- invalid UTF-8；
- extra whitespace；
- unexpected field；
- unknown field；
- empty response；
- malformed response；

都不能：

panic。

未知字段：

> 忽略。

已知字段类型错误：

> 返回 parse error / fallback。

---

# 81. Fuzz / Property Testing

至少对：

- ICY；
- Radio Browser response；
- RSS；
- m3u8；

建立：

fuzz/property test structure。

如果本阶段只实现 A：

至少：

ICY + Radio Browser

必须拥有属性测试。

---

# 82. No Real-Network Tests

CI：

绝对禁止真实网络 source tests。

测试：

使用：

- recorded JSON fixtures；
- recorded ICY byte streams；
- recorded HTTP response fixtures；
- local XML；
- local m3u8。

---

# 83. Fixture Policy

所有 fixture：

必须：

- 小；
- 合法；
- 可公开；
- 来源明确；
- 不包含私人凭据。

不得：

提交：

- user credentials；
- real account cookies；
- Authorization headers；
- commercial copyrighted audio。

---

# 84. Fake HTTP Server

测试 Direct HTTP：

使用：

> local test HTTP server。

用于模拟：

- 200；
- 206；
- 301/302；
- 404；
- 500；
- timeout；
- malformed headers；
- wrong content type；
- oversized response；
- redirect chain。

不能访问公网。

---

# 85. Fake Radio Browser

创建：

local fixture server

模拟：

```text
/json/servers
/json/stations/search
/json/url/<stationuuid>
```

验证：

- discovery；
- random selection；
- fallback；
- url_resolved；
- codec filtering；
- click reporting。

---

# 86. Fake ICY Server

建立：

deterministic byte stream：

```text
audio
metadata
audio
metadata
```

metadata：

故意切成：

- 1 byte；
- 2 bytes；
- 7 bytes；
- arbitrary chunks。

验证 parser：

> 与网络 read boundary 无关。

---

# 87. Unsupported Platform Test

至少：

```text id="t4qnvl"
https://youtube.com/...
https://spotify.com/...
https://music.163.com/...
https://y.qq.com/...
https://music.apple.com/...
```

必须证明：

> 没有任何 network request。

最理想：

test server / request spy：

assert:

`request_count == 0`

---

# 88. Redirect Security Tests

必须测试：

```text id="2nxqec"
https → https
https → http
https → private IP
public → localhost
public → credential URL
>5 redirects
```

最终行为：

必须符合：

`SECURITY.md`

实际决定。

---

# 89. Source Determinism

同一：

```text input
fixture
config
```

多次：

```text resolve()
```

必须得到：

> 相同结构化结果。

production 随机 mirror：

测试模式：

必须支持：

> injected deterministic selector。

---

# 90. Source Selection Policy

当一个 URL 有多个 stream：

例如：

- MP3；
- OGG；
- FLAC；

必须按照明确策略排序。

默认：

> 优先高质量且 Audio Core 当前真正支持的 codec。

但不得：

> 为了“高质量”选择实际无法播放的 codec。

具体 scoring：

记录：

`DECISIONS.md`

---

# 91. Network Metadata Events

建立最小：

```text
StreamMetadataEvent
```

支持：

- StreamTitle；
- station title；
- artist；
- track title。

未来 UI：

M6 接入。

M4：

不做 UI。

---

# 92. No Persistent Metadata Enrichment

M4：

不允许：

> 自动把网络来源 metadata 写进 M3 SQLite。

除非用户明确将来源加入音乐库，并由上层 application layer 执行。

M4 只提供：

> resolved metadata。

---

# 93. Library Interaction

如果 M4 需要与 M3：

`Track`

对接：

只通过：

> application/domain contract

不要：

`aura-sources → SQLite`

直接写数据库。

---

# 94. Cache Policy Tests

每个 Source 必须有测试：

### Local

`Allowed`

### Direct HTTP

按 Source resolution：

明确返回。

### Internet Radio

`Forbidden`

### HLS

`SessionOnly`

### RSS

浏览：

`SessionOnly`

用户主动保存音频：

未来：

`Allowed`

Radio Browser：

目录 metadata：

不得误认为音频许可证。

---

# 95. Forbidden Source Zero-Write

不仅要测试 Internet Radio。

如果任何 Source：

```text CachePolicy::Forbidden
```

都必须：

> 没有持久写盘行为。

建立通用测试 helper：

```text assert_forbidden_source_no_disk_write()
```

为 M5 做准备。

---

# 96. Logging

source layer：

可以：

`tracing`

但日志中禁止：

- credentials；
- Authorization；
- API key；
- password；
- signed URL query；
- full secret-bearing URL。

URL logging：

默认只记录：

- scheme；
- host；
- sanitized path；

必要时：

> redacted URL。

---

# 97. Security Audit

必须检查：

- SSRF；
- redirect SSRF；
- localhost；
- private IP；
- credential URLs；
- oversize body；
- malformed metadata；
- XML external entity；
- m3u8 injection；
- header injection；
- path traversal；
- unbounded buffering。

---

# 98. Dependency Review

运行：

```powershell id="3wq06c"
cargo tree
cargo deny check
```

重点审查：

```text
reqwest
rustls
quick-xml
windows
tauri related dependencies
```

如需添加 m3u8/parser dependency：

必须单独审查：

- license；
- maintenance；
- MSRV；
- Windows；
- build complexity。

如果自行实现协议 parser 更简单且可测试：

优先自行实现。

---

# 99. Performance

建立：

`docs/PERF.md`

增加：

- local resolve；
- direct HTTP headers/resolve；
- ICY parser throughput；
- Radio Browser parse；
- RSS parse（若 B）；
- HLS parse（若 B）；
- source resolution memory；
- live stream buffer memory。

不要伪造目标。

只记录实际结果。

---

# 100. Network Streaming Performance

不要为了：

> 更快

无限增加内存 buffer。

优先：

- bounded；
- backpressure；
- predictable memory。

---

# 101. Test Matrix — Source

至少：

```text id="ff5a9y"
local file
direct http
direct https
unsupported platform
invalid scheme
credential URL
redirect
redirect limit
timeout
oversized response
malformed content
unsupported codec
```

---

# 102. Test Matrix — ICY

至少：

```text id="q0t1ib"
metadata interval
zero metadata
multiple metadata blocks
partial reads
UTF-8
invalid UTF-8
large metadata
missing StreamTitle
multiple StreamTitle updates
malformed block
audio continuity
```

---

# 103. Test Matrix — Radio Browser

至少：

```text id="3v4xqg"
DNS discovery fixture
server list parse
mirror selection
fallback
station search
codec filtering
url_resolved
click disabled
click enabled
malformed JSON
empty response
large response
timeout
```

---

# 104. Test Matrix — HTTP

至少：

```text id="2a7jkf"
200
206
301
302
404
403
500
timeout
bad content-type
invalid URL
redirect >5
private redirect
credential URL
```

---

# 105. Test Matrix — HLS [B]

如果 B 级完成：

```text
master playlist
media playlist
live refresh
media sequence
segment ordering
segment timeout
invalid playlist
oversized playlist
EXT-X-KEY rejection
redirect validation
stop cancellation
```

---

# 106. Test Matrix — RSS [B]

如果 B 完成：

```text
valid RSS
missing enclosure
multiple enclosures
invalid XML
oversized XML
XXE attempt
redirect
HTTPS
HTTP
episode duration
MIME filtering
```

---

# 107. Test Matrix — WMF AAC [B]

如果 B 完成：

至少：

```text
supported Windows environment
unsupported environment
decoder initialization failure
invalid AAC/M4A
short file
stream error
seek if supported
non-seekable stream
clean shutdown
```

真实设备 / OS test：

可以使用：

`#[ignore]`

但 CI 不得假装已通过真实硬件测试。

---

# 108. Source Lifecycle

必须定义：

```text id="ox65jj"
Created
Resolving
Ready
Streaming
Ended
Failed
Cancelled
```

不要为了简单：

把 lifecycle 散落在几十个 bool 中。

---

# 109. Cancellation Semantics

Source 被取消：

必须：

- 结束 HTTP request；
- 结束 background task；
- 释放 response；
- 释放 temporary buffers；
- 不产生 orphan task。

测试：

取消后：

> bounded time 内 resource count 回收。

---

# 110. Stream EOF

对于 finite direct audio：

EOF：

正常。

对于 live radio：

EOF：

必须定义为：

> stream ended / source failure

而不是：

> 播放器自动结束成 next track

最终 next-track 行为由 application layer 决定。

---

# 111. Source Reconnection

M4 不强制实现自动 reconnect。

如果实现：

必须：

- bounded retry；
- no duplicate audio；
- no stale metadata；
- no generation corruption；
- no infinite reconnect。

否则：

明确留到后续版本。

---

# 112. M1/M2 Integration Regression

M4 完成：

必须再次确认：

- direct stream → decoder；
- non-seekable stream → decoder；
- output underrun；
- pause；
- stop；
- seek on seekable source；
- seek unavailable on live source；
- generation after source failure；
- device recovery。

任何 regression：

M4 FAIL。

---

# 113. M3 Regression

必须确认：

- Local Source 能读取 M3 支持的格式；
- metadata extraction 不重复实现；
- Track identity 不被破坏；
- library database 不被 Source layer 直接写入；
- local root policy 不回退。

---

# 114. No UI

M4 不实现：

- Now Playing；
- Library page；
- Source browser UI；
- radio browser UI；
- URL input UI；
- RSS UI；
- HLS UI；
- network settings UI。

允许：

建立未来 M6 使用的：

> typed IPC / DTO contracts。

但不能实现最终页面。

---

# 115. IPC Boundary

如果需要：

建立：

```text
resolve_source
search_source
cancel_source
```

所需的最小 contract。

不要：

> 从 React 直接拿 reqwest response。

---

# 116. `SOURCES.md`

M4 完成时更新：

`SOURCES.md`

必须记录：

- Source；
- endpoint；
- purpose；
- request behavior；
- cache policy；
- license treatment；
- attribution treatment；
- verification date；
- implementation status；
- known limitations。

如果联网：

按照 M0 规则：

> 重新核对之前记录的外部事实。

变化：

必须使用最新事实。

---

# 117. `CONTENT_POLICY.md`

补充：

- direct URL；
- internet radio；
- Radio Browser；
- RSS；
- HLS；
- AAC；
- unsupported platforms；
- protected stream handling。

---

# 118. `PRIVACY.md`

如实记录：

### 用户主动输入 URL

发送到目标服务器。

### Radio Browser 搜索

请求 Radio Browser。

### Radio Browser click

默认关闭。

### 本阶段

没有遥测。

### Source failure

不自动上传诊断。

---

# 119. `SECURITY.md`

加入：

```text id="a6lh5n"
URL Validation
Redirect Validation
SSRF Policy
Credential URL Policy
Response Size Limits
Cancellation
XML Safety
HLS Security
ICY Limits
```

必须与真实实现一致。

---

# 120. `ARCHITECTURE.md`

加入：

```text id="m72j5a"
Source Architecture
Source Trait
Source Resolver
Resolved Model
Network Stream Model
ICY Pipeline
Radio Browser Flow
Source → Audio Core Boundary
Cancellation
Error Propagation
Security Boundary
```

---

# 121. `DECISIONS.md`

至少记录：

- Source trait 设计；
- async strategy；
- HTTP client；
- timeout；
- redirect；
- private network policy；
- credential URL policy；
- Source selection；
- Direct HTTP seek capability；
- ICY parser；
- ICY metadata limit；
- Radio Browser discovery；
- mirror selection；
- retry；
- click reporting；
- network buffer；
- RSS strategy；
- HLS strategy；
- WMF strategy；
- cache policy；
- parser implementation choices。

---

# 122. `PROGRESS.md`

记录：

```text id="7s2gnc"
M4-A = IN PROGRESS
M4-B = NOT STARTED
```

完成后真实记录：

- A 功能；
- B 功能；
- tests；
- fixtures；
- performance；
- known limitations。

---

# 123. Parser Determinism

所有 parser：

相同输入：

必须得到：

> 相同输出。

Radio Browser production mirror 的“随机选择”不能污染：

> parser determinism。

---

# 124. Production Randomness vs Test Determinism

明确：

```text id="m2h56v"
Production:
random mirror selection

Test:
injected deterministic selector
```

不得：

为了测试：

修改 production behavior。

---

# 125. Forbidden Network Request Test

必须建立：

> request spy / deny-all transport

至少验证：

unsupported platform：

不发送：

- DNS；
- HTTP；
- redirect；
- metadata request。

如果客户端设计导致无法观察 DNS：

至少通过：

> resolver / transport mock

证明：

没有发起网络请求。

---

# 126. Malformed Input Rule

任何 Source 输入：

原则：

> “不信任，先解析，再使用。”

不能：

```text
unwrap()
expect()
panic!()
```

处理外部输入。

---

# 127. Memory Safety

特别检查：

- stream metadata；
- RSS body；
- m3u8；
- JSON；
- ICY metadata；
- audio buffer；

所有长度：

必须有界。

---

# 128. Network Audio 与缓存边界

再次强调：

M4：

> 负责“获取与解析”。

M5：

> 负责“持久化缓存”。

因此：

不要在 M4 实现：

```text
chunks/
SQLite cache
ETag persistence
LRU
pin
resume cache
```

M4 可以读取：

> HTTP Range capability

但不要建立持久化 cache。

---

# 129. Final Quality Gate

必须：

```powershell id="cswz7r"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

如前端已有门禁：

```powershell id="o0z3m7"
npm run typecheck
npm run lint
npm test
```

也必须运行。

---

# 130. Full Regression

必须重新运行整个 workspace tests。

确认：

M1：

- decoder；
- resampler；
- ring buffer；
- seek；
- generation；
- volume；
- zero allocation。

M2：

- gapless；
- crossfade；
- loudness；
- limiter；
- hotplug；
- EQ。

M3：

- scanner；
- metadata；
- SQLite；
- FTS5；
- artwork；
- watcher；
- identity。

全部 PASS。

---

# 131. Diff Review

完成后：

不要立即 commit。

执行：

```powershell id="j1c2qx"
git diff --stat
git diff
```

重点检查：

- 是否偷渡 M5 cache；
- 是否偷渡 M7 Dreamscape；
- 是否偷渡 M8 analysis；
- 是否偷渡 M6 UI；
- 是否调用商业平台；
- 是否存在 extraction code；
- 是否存在 DRM；
- 是否存在歌词抓取；
- 是否存在在线 artwork；
- 是否存在 metadata enrichment；
- 是否自动联网；
- 是否关闭 SSRF 防护；
- 是否允许 credential URL；
- 是否存在 unlimited response buffer；
- 是否存在 unbounded ICY metadata；
- 是否存在 XML XXE；
- 是否存在 HLS encryption handling；
- 是否存在 unsupported platform network request；
- 是否在 Source 层直接写 SQLite；
- 是否在 Source 层直接调用 audio output；
- 是否存在 TODO；
- 是否存在 ignored tests；
- 是否弱化 assertions；
- 是否提交凭据；
- 是否提交商业音频；
- 是否提交大型 fixtures。

---

# 132. 最终 M4 Audit

输出：

# M4 Final Audit

## A. Baseline

## B. Source Architecture

## C. Resolver

## D. Local Source

## E. Direct HTTP/HTTPS

## F. Network Stream

## G. ICY

## H. Internet Radio

## I. Radio Browser

## J. License / Attribution / Cache Policy

## K. Network Security

## L. Cancellation

## M. Parser Robustness

## N. Tests

## O. Performance

## P. M1 Regression

## Q. M2 Regression

## R. M3 Regression

## S. Documentation

## T. M4-A Scope

## U. M4-B Scope

每项只能：

`PASS / FAIL / BLOCKED`

不得写：

- 基本完成；
- 大概可用；
- 应该没问题；
- 理论支持。

---

# 133. A 级关键证据

必须明确提供：

### Source

- Source trait；
- resolver；
- resolved model。

### Local

- local resolve；
- metadata handoff；
- seek capability。

### HTTP

- HTTP；
- HTTPS；
- redirect limit；
- timeout；
- body limit；
- unsupported URL handling。

### ICY

- metadata interval；
- partial read；
- StreamTitle；
- malformed metadata；
- audio continuity。

### Radio Browser

- discovery；
- mirror selection；
- fallback；
- search；
- url_resolved；
- codec filter；
- click reporting default off。

### Security

- unsupported platform zero-request test；
- SSRF policy；
- credential URL handling；
- bounded metadata；
- cancellation。

---

# 134. B 级关键证据

如果 B 完成：

### RSS

- parser；
- XXE protection；
- enclosure；
- limits。

### HLS

- master；
- media；
- live refresh；
- media sequence；
- encryption rejection；
- segment limits。

### WMF

- AAC decode；
- Windows isolation；
- graceful unsupported path。

如果 B 没完成：

必须明确：

```text id="t9q6y1"
M4-A = PASS
M4-B = NOT COMPLETED
```

不得影响 A 级发布判断。

---

# 135. Dependency Final Audit

再次：

```powershell id="j46a4o"
cargo tree
cargo deny check
```

确认：

- 没有 GPL；
- 没有 AGPL；
- 没有 LGPL；
- 没有未经批准 codec；
- 没有无必要的大型网络框架；
- 没有重复 HTTP client；
- 没有重复 parser。

---

# 136. Git Commit

只有：

- M4-A 全部 PASS；
- B 级如果声明完成则全部 PASS；
- M1 regression PASS；
- M2 regression PASS；
- M3 regression PASS；
- security tests PASS；
- quality gate 全绿；
- final audit PASS；
- diff review 干净；

才允许：

```powershell id="0j9rzu"
git add .
git commit -m "M4: implement source system"
```

**不要 push。**

---

# 137. PROGRESS

更新：

```text id="k4n2k4"
M4-A = PASS
```

如果 B 完成：

```text
M4-B = PASS
```

否则：

```text
M4-B = NOT COMPLETED
```

记录：

- tests；
- fixtures；
- sources；
- security checks；
- network behavior；
- performance；
- known limitations。

---

# 138. 最终输出

完成后输出：

```text id="3q5z7x"
# M4 Final Audit

A. Baseline
B. Source Architecture
C. Resolver
D. Local
E. Direct HTTP/HTTPS
F. Network Stream
G. ICY
H. Internet Radio
I. Radio Browser
J. Security
K. Cache Policy
L. Cancellation
M. Tests
N. Performance
O. M1 Regression
P. M2 Regression
Q. M3 Regression
R. Documentation
S. M4-A
T. M4-B

M4-A = PASS / BLOCKED
M4-B = PASS / NOT COMPLETED
COMMIT = <hash>
NEXT = M5
```

如果 M4-A / 已声明的 M4-B 全部通过：

**立即停止。**

**不要开始 M5。**