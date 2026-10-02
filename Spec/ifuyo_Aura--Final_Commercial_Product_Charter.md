# ifuyo Aura — Final Commercial Product Charter

> 项目仓库：`ifuyo-aura`
>
> 软件名称：`ifuyo Aura`
>
> 品牌：`ifuyo · 伊芙游`
>
> 产品定位：面向 Windows 的本地优先、合法来源、免费可分享、可商业化演进的高品质桌面音乐播放器。
>
> 核心体验：
>
> 1. 把音乐库变成可探索的“听感星图”。
> 2. 用极光与实时视觉让播放过程具有沉浸感。
> 3. 内置确定性生成式音乐引擎 Dreamscape，让播放器在没有合适本地音乐时仍然拥有内容。
> 4. 强大的音频工程能力、分析能力、Windows 原生集成和开放来源能力，作为长期商业化基础。

---

# 0. 执行身份与绝对规则

你是资深 Rust 系统工程师、音频工程师、Windows 桌面工程师、前端工程师和产品设计师。

当前目录为 ifuyo Aura 项目根目录。

项目长期目标不是“简单个人播放器”，而是：

> **先以商业产品级标准实现，再供个人长期使用；未来无需推倒重来即可直接进入正式商业发行。**

因此不得为了“个人项目”降低工程、测试、安全、合法性、可维护性、发布质量标准。

但是：

> **不得为了未来商业化而提前加入账号体系、云服务、广告、付费系统、遥测或其他当前非目标功能。**

---

## 0.1 执行方式

严格按以下顺序执行：

`Phase 0 → M0 → M1 → M2 → ... → M14`

每个 Milestone：

`测试设计 → 基线审计 → 实现 → 测试 → 完整门禁 → 文档同步 → Git commit`

没有通过当前 Milestone 的门禁，不得进入下一 Milestone。

一次只实施当前 Milestone。

不得预先实现后续 Milestone 功能。

---

## 0.2 自主决策

不得向用户反复提问。

遇到没有明确规定的技术选择：

1. 选择更简单；
2. 选择更容易测试；
3. 选择许可证更清晰；
4. 选择更容易替换；
5. 选择更稳定；
6. 记录到 `DECISIONS.md`；
7. 继续执行。

不得因为歧义停工。

---

## 0.3 严禁

禁止：

* TODO 占位实现；
* 假数据冒充真实功能；
* 用删除测试、弱化断言或降低测试标准通过门禁；
* 虚构性能数据；
* 提交任何密钥；
* 提交真实账号凭据；
* 提交音乐文件；
* 偷渡未经审核的第三方依赖；
* 为通过 CI 临时关闭 lint、test、clippy、deny；
* 通过“实现看起来存在”的接口伪装未实现能力；
* 任何商业平台音频抓取；
* DRM 破解；
* 绕过访问控制；
* 登录态接口模拟；
* 隐蔽联网；
* 遥测。

---

# 1. 产品原则

发生冲突时按顺序优先：

1. 合法优先。
2. 音频质量优先。
3. 数据安全与隐私优先。
4. 核心体验优先。
5. 克制但高级的视觉设计。
6. 性能优先。
7. 每个能力可测试。
8. 商业化可持续演进。
9. 最后才考虑炫技。

---

# 2. 内容与合法来源政策

## 2.1 明确禁止

不得实现：

* YouTube 音频抽取；
* Spotify 音频抽取；
* 网易云音频抽取；
* QQ 音乐音频抽取；
* Apple Music 音频抽取；
* yt-dlp 类行为；
* DRM 解密；
* Widevine；
* FairPlay；
* SAMPLE-AES 解密；
* 加密 HLS；
* 破解签名；
* 破解加密参数；
* 登录态抓取；
* 商业平台私有 API 音频访问；
* 网络歌词抓取。

用户输入不支持的平台 URL：

必须根据 `config/unsupported_platforms.json` 识别；

不得发送请求；

必须返回明确提示：

> “该平台不受支持，请使用本地文件或开放来源。”

---

## 2.2 来源等级

[A] 必须：

* 本地音乐文件；
* 用户选择的音乐目录；
* 合法 HTTP/HTTPS 直链音频；
* 网络电台；
* Radio Browser；
* Dreamscape。

[B] 应当：

* Podcast RSS；
* 未加密 HLS；
* Subsonic / OpenSubsonic；
* Jamendo；
* Internet Archive。

[C] 加分：

* 更多合法开放来源；
* 更丰富元数据；
* 更高级发现机制。

未知来源：

`CachePolicy = SessionOnly`

---

# 3. 缓存与许可原则

每个 Source 必须明确返回：

`Allowed | SessionOnly | Forbidden`

含义：

### Allowed

可以持久缓存。

### SessionOnly

只能当前会话缓存，不允许持久化。

### Forbidden

禁止任何形式的写盘。

Forbidden 来源必须从代码结构上保证所有执行路径均不得写入缓存目录。

---

## 3.1 许可信息

任何带许可证信息的歌曲：

必须显示：

* License；
* Attribution；
* Source；
* Source URL；
* 额外使用限制。

“保存到音乐库”时：

许可和来源信息必须能够写入标签或旁车元数据。

---

## 3.2 仓库内容

仓库：

* 不得包含任何真实音乐；
* 不得包含真实 API key；
* 不得包含个人凭据；
* 不得包含私有证书；
* 不得包含签名私钥。

演示内容只能通过合法开放来源脚本运行时获得。

---

# 4. 品牌与视觉身份

## 4.1 品牌事实来源

`brand.json` 是唯一事实来源。

必须包含：

* name: `ifuyo Aura`
* vendor: `ifuyo · 伊芙游`
* slogan
* site
* version
* copyright
* colors

品牌语：

> 以光与梦，编织每一次游玩。

气质：

> 干净、克制、富有情绪。

---

## 4.2 品牌视觉

品牌色：

* bg
* rose
* violet
* sky
* mint

设计 token 只能定义于：

`ui/src/design/tokens.ts`

自动生成 CSS variables。

业务代码不得出现裸色值。

CI / lint 必须检查。

---

## 4.3 核心视觉

Now Playing：

> WebGL 极光作为唯一主视觉。

极光只使用品牌色。

音乐数据驱动：

* Key → 色相；
* Energy → 光带强度；
* BPM → 流动速度；
* 小调 → 降低饱和度和明度；
* 当前播放状态 → 动态响应。

颜色映射必须为纯函数并有单元测试。

---

# 5. 技术栈

## 5.1 桌面

Tauri 2。

Backend：

Rust stable。

MSRV：

由实际依赖要求确定并记录。

---

## 5.2 Frontend

* TypeScript；
* React 18；
* Vite；
* Zustand；
* Three.js；
* `@tanstack/react-virtual`。

---

# 6. 仓库结构

```text
crates/
  aura-core/
  aura-sources/
  aura-cache/
  aura-library/
  aura-analysis/
  aura-synth/
  aura-testkit/

src-tauri/

ui/

site/

scripts/

config/

docs/

brand.json
DECISIONS.md
PROGRESS.md
SOURCES.md
CONTENT_POLICY.md
PRIVACY.md
BRAND.md
ARCHITECTURE.md
README.md
LICENSE-MIT
LICENSE-APACHE
THIRD_PARTY_LICENSES.md
CHANGELOG.md
SECURITY.md
CODE_SIGNING_POLICY.md
```

模块职责不得相互污染。

`aura-core` 不得依赖 Tauri。

`aura-testkit` 为核心测试提供：

* NullOutput；
* FaultFS；
* 合成音频；
* 固定随机种子；
* 确定性时钟。

---

# 7. 许可政策

允许：

* MIT；
* Apache-2.0；
* BSD-2-Clause；
* BSD-3-Clause；
* ISC；
* MPL-2.0；
* Zlib；
* CC0-1.0；
* Unicode-3.0；
* CDLA-Permissive-2.0；
* OFL-1.1。

禁止：

* GPL；
* AGPL；
* LGPL。

任何新增许可证类型：

必须记录在 `DECISIONS.md`。

`cargo-deny` 与前端 license 检查共同执行。

---

# 8. 音频引擎

## 8.1 线程模型

```text
Decoder
   ↓
Resampler
   ↓
Lock-free SPSC Ring Buffer
   ↓
WASAPI Output Callback
```

目标缓冲约 500ms。

---

## 8.2 Audio Callback 铁律

Callback 内：

允许：

* 拷贝；
* gain；
* 10ms gain smoothing；
* EQ；
* meter。

禁止：

* allocation；
* lock；
* logging；
* syscall；
* blocking；
* IO；
* panic。

必须使用测试确认零分配。

---

## 8.3 输出

默认：

Windows WASAPI Shared Mode。

必须支持：

* 默认设备；
* 设备列表；
* 设备切换；
* 热插拔；
* 采样率不一致处理；
* 声道映射；
* mono → stereo；
* multichannel downmix。

设备断开：

> 自动暂停并提示。

设备恢复：

> 允许继续。

Underflow：

* 静音；
* 计数；
* UI 状态提示；
* 绝不 panic。

---

## 8.4 时钟

播放位置必须依据：

`consumed_frames / sample_rate`

而不是 wall clock。

seek：

使用 generation。

旧 generation 的样本：

必须全部丢弃。

测试：

必须证明 seek 后听不到旧数据。

---

## 8.5 Gapless

对明确支持的格式提供样本级无缝播放：

* MP3；
* FLAC；
* Vorbis；
* ALAC。

下一曲必须提前解码。

测试要求：

* 总样本数精确一致；
* 边界没有突变。

---

## 8.6 Crossfade

支持：

0~12 秒。

默认：

等功率曲线。

同专辑连续无缝播放时：

不强制 crossfade。

---

## 8.7 Loudness

导入阶段分析：

* LUFS；
* True Peak。

默认目标：

`-16 LUFS`

播放阶段：

* 自动增益；
* soft limiter。

---

## 8.8 EQ

[B]

10-band parametric EQ：

* preamp；
* presets；
* per-output-device memory。

---

## 8.9 Practice Mode

[B]

* A-B loop；
* speed 0.5x~1.5x；
* pitch preserved。

WSOLA 必须有自动测试。

---

## 8.10 自动混音

[B]

候选：

* BPM；
* half/double tempo；
* Camelot key；
* energy；
* timbre distance。

在：

8 / 16 beat phrase boundary

进行过渡。

---

## 8.11 可视化数据

音频分析线程：

生成：

* 64 frequency bands；
* energy；
* beat phase。

通过 Tauri channel：

30Hz。

beat phase：

前端根据：

`position + beat grid`

本地插值。

---

# 9. Source 系统

统一：

```text
search(query)
resolve(input)
```

返回：

```text
Resolved {
    streams,
    meta,
    license,
    cache_policy,
    attribution
}
```

统一输入框支持：

* local path；
* URL；
* RSS；
* m3u8；
* source identifier。

未知输入：

明确错误。

---

# 10. 网络安全

只允许：

* HTTP；
* HTTPS。

要求：

* 最大 5 次 redirect；
* connect timeout；
* read timeout；
* response body size limit；
* descriptive User-Agent；
* URL scheme validation；
* 路径验证。

局域网地址允许用于用户自建服务器。

首次访问非公网地址：

一次性确认。

不得接受任意协议。

---

# 11. ICY / Radio Browser

## ICY

[A]

支持：

* `Icy-MetaData: 1`
* `icy-metaint`
* metadata stripping；
* `StreamTitle`。

元数据显示到：

Now Playing。

不要求可 seek。

---

## Radio Browser

[A]

必须：

* DNS discovery；
* 镜像随机选择；
* failure fallback；
* `url_resolved`；
* descriptive User-Agent；
* codec filtering。

默认优先：

* MP3；
* OGG；
* FLAC。

click reporting：

默认关闭。

---

# 12. HLS

[B]

只允许：

* master playlist；
* media playlist；
* live refresh；
* media sequence；
* 未加密 segment。

遇到：

* encryption；
* DRM；
* protected stream；

立即明确拒绝。

---

# 13. 网络缓存

[A]

只允许：

`CachePolicy = Allowed`

实现：

* 256 KiB chunks；
* atomic rename；
* SQLite metadata；
* WAL；
* Range；
* read/seek；
* prefetch；
* retry；
* exponential backoff；
* ETag；
* Last-Modified；
* offline cached playback；
* LRU；
* pin。

默认容量：

2GB。

---

## 13.1 Fault Injection

必须通过 FaultFS 测试：

* write failure；
* truncation；
* rename failure；
* index failure；
* restart recovery。

要求：

重启后：

* cache self-consistent；
* no corrupted index；
* no orphan chunks。

Forbidden 来源播放期间：

缓存目录必须零写入。

---

# 14. 音乐库

[A]

支持：

* FLAC；
* MP3；
* OGG；
* WAV；
* M4A；
* ALAC。

使用：

`lofty`

读取：

* title；
* artist；
* album；
* year；
* genre；
* artwork；
* metadata。

---

## 14.1 扫描

用户选择目录。

支持：

* initial scan；
* incremental scan；
* filesystem watcher。

文件移动 / 重命名：

通过内容哈希保留播放统计。

---

## 14.2 SQLite

要求：

* WAL；
* FTS5；
* migrations；
* deterministic tests。

搜索：

* title；
* artist；
* album；
* path。

中文：

支持 CJK 二元组策略。

---

## 14.3 Smart Search

支持：

```text
bpm:120-130
key:8A
energy:>0.6
license:cc
```

---

# 15. Playlist

[A]

必须：

* manual playlist；
* smart playlist；
* sortable queue；
* undo queue changes。

---

# 16. Lyrics

[B]

只读取：

* embedded lyrics；
* adjacent `.lrc`。

严禁联网抓歌词。

---

# 17. Audio Analysis

[A]

后台低优先级。

必须：

* cancellable；
* resumable；
* cacheable；
* content-key based。

每首歌曲产出：

* BPM；
* BPM confidence；
* beat grid；
* key；
* Camelot key；
* LUFS；
* true peak；
* energy；
* brightness；
* MFCC mean/variance；
* 32D normalized embedding。

---

## 17.1 测试夹具

必须测试：

120 BPM click track。

要求：

BPM error < 0.5。

大三和弦：

key 正确。

参考正弦：

LUFS 误差 <= 0.1 LU。

允许：

double/half BPM error；

但必须输出：

* confidence；
* alternative BPM。

---

# 18. Music Map

这是产品核心功能之一。

Embedding：

降维到 2D。

要求：

* deterministic；
* fixed seed；
* stable layout。

新增歌曲：

只允许：

* nearest-neighbor interpolation；
* local relaxation。

禁止：

新增歌曲后默认全量重排。

全量重排必须由用户明确触发。

---

## 18.1 稳定地图

新增约 1% 歌曲后：

已有歌曲位置 median displacement 必须小于预定阈值。

---

# 19. Map UI

使用 WebGL instancing。

每首歌：

一个星点。

视觉参数：

* position = embedding；
* size = play count；
* color = harmonic mapping；
* glow = current relevance；
* proximity = musical similarity。

支持：

* hover card；
* lasso selection；
* play selected region；
* generate playlist from selection。

[B]

增加：

* fog exploration；
* journey trail；
* path → gradient playlist；
* journey poster。

---

# 20. Dreamscape

这是产品第二核心功能。

[A]

实时生成无限音乐流。

必须包括：

* deterministic PRNG；
* seed string；
* mode；
* chord progression；
* slow arpeggio；
* sustained texture；
* soft noise；
* reverb；
* three moods。

三种默认情绪：

* 平静；
* 澄澈；
* 深邃。

---

## 20.1 Seed

格式：

8 位 Base32。

加：

Mood。

例：

```text
7K2M9Q4P · 深邃
```

同一软件版本、同一平台：

同 seed 必须逐位一致。

不同 seed：

必须产生不同结果。

---

## 20.2 Dreamscape Export

[B]

导出：

3 分钟 loopable WAV。

写入：

* seed；
* mood；
* version。

不得替用户声明音乐版权。

---

## 20.3 Dreamscape First Launch

没有音乐库时：

界面不得为空。

提供：

* 播放 Dreamscape；
* 添加音乐文件夹；
* 探索免费音乐。

---

# 21. UI 架构

窗口默认：

1180×760。

最小：

900×600。

无边框。

自绘标题栏。

---

# 22. Windows 视觉

Windows 11：

Mica。

Windows 10：

solid background fallback。

禁止 acrylic。

设置：

`Mica / Solid`

可切换。

WebGL canvas：

不透明。

---

# 23. 三个主视图

仅三个：

### Now Playing

### Map

### Library

全局：

`Ctrl + K`

命令面板作为：

导航 + 高级操作入口。

---

# 24. Now Playing

核心视觉：

极光。

显示：

* cover；
* title；
* artist；
* waveform；
* progress；
* beat markers；
* previous；
* play/pause；
* next；
* volume。

Lyrics、EQ：

进入 drawer。

---

# 25. Library

支持：

* virtualized list；
* grid；
* filtering；
* sorting；
* BPM；
* key；
* energy；
* artist；
* album。

---

# 26. Mini Player

Map / Library 底部：

常驻 mini player。

额外：

320×96 mini floating window。

---

# 27. UI 设计语言

* dark；
* sophisticated；
* atmospheric；
* minimal；
* futuristic；
* emotionally warm。

表面：

4%~8% white alpha。

Border：

8% white。

Radius：

14px。

Typography：

12 / 14 / 16 / 20 / 32。

Primary text：

92% white。

Secondary：

60%。

每个时刻：

只允许一个强调色。

禁止：

* 巨型阴影；
* 过度 glassmorphism；
* 控件堆叠；
* 多个视觉焦点；
* 游戏化奖励 UI。

---

# 28. Motion

动画：

120~240ms。

只优先：

* transform；
* opacity。

支持：

`prefers-reduced-motion`。

同屏最多：

1 个 WebGL canvas。

连续 1 秒 frame time >18ms：

自动降分辨率。

窗口：

* minimized → pause rendering；
* unfocused → 降低视觉开销。

---

# 29. Accessibility / i18n

支持：

* keyboard navigation；
* visible focus ring；
* AA contrast；
* 90%~140% UI scale；
* zh-CN；
* en。

不允许业务文案硬编码散落。

---

# 30. Windows Integration

[A]

必须：

* media keys；
* Windows SMTC；
* tray；
* single instance；
* file associations；
* drag/drop file；
* drag/drop folder；
* drag/drop URL；
* close-to-tray option；
* crash recovery。

---

## 30.1 Crash Recovery

每 5 秒：

持久化：

* queue；
* current track；
* position。

重启：

恢复为：

`paused`

不得自动播放。

---

# 31. B 级 Windows 能力

* mini floating window；
* global hotkeys；
* startup with Windows，默认关闭。

---

# 32. Logging

使用：

`tracing`

写入：

```text
%APPDATA%\ifuyo\ifuyo Aura\logs
```

要求：

* rolling logs；
* panic hook；
* no auto upload；
* settings → open logs directory。

日志中绝不出现：

* password；
* token；
* API key；
* private credential。

---

# 33. Privacy

零遥测。

联网必须由用户操作触发，除非用户明确开启相关在线功能。

更新检查：

默认关闭，首次使用询问。

Radio Browser click：

默认关闭。

必须真实记录到：

`PRIVACY.md`

---

# 34. Tauri Security

严格：

* CSP；
* no remote scripts；
* no remote styles；
* no remote fonts；
* no remote images；
* minimal capabilities；
* IPC command allowlist；
* IPC input validation。

外链：

只允许系统浏览器打开。

域名白名单：

* ifuyo.com；
* 项目仓库。

---

# 35. Credentials

Subsonic 等凭据：

只能进入：

Windows Credential Manager。

API keys：

也不能进入源码。

不得明文写磁盘。

---

# 36. Codec Strategy

官方发行版：

优先：

* MP3；
* FLAC；
* Vorbis；
* WAV/PCM；
* OGG；
* MKV/WebM；
* ALAC。

symphonia：

不得 vendor 或修改。

MPL-2.0：

必须出现在第三方许可证文档。

---

## AAC

官方发行版默认不内置 AAC decoder。

AAC：

默认通过 Windows Media Foundation。

自行编译版本允许：

cargo feature：

`aac-builtin`

官方版本禁止默认启用。

禁止：

* symphonia-adapter-fdk-aac；
* GPL/LGPL codec libraries。

---

# 37. Opus

[C]

只有满足：

* windows-latest CI；
* no extra system dependency；

才启用。

否则：

明确标注不支持。

---

# 38. Source Plugins

初期保持内置 Source architecture。

当前：

不做第三方插件系统。

---

# 39. 商业化边界

当前产品可以：

* 免费；
* 开源；
* 网上分享；
* Steam 发布；
* GitHub 发布；
* 官方网站发布。

但代码架构必须允许未来加入：

* 商业发行渠道；
* 正式 installer；
* updater；
* 商业授权体系；
* optional premium capabilities。

当前不实现：

* 账号；
* 云同步；
* 社交；
* 评论；
* 广告；
* 内购；
* 订阅；
* 赞助入口；
* 用户画像上传；
* 遥测。

---

# 40. Release

GitHub Actions：

Windows runner。

必须运行：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
npm run typecheck
npm run lint
npm test
tauri build
```

Tag：

触发 Release。

---

# 41. Installer

NSIS。

默认：

`currentUser`

无需管理员。

WebView：

`downloadBootstrapper`

支持：

zh-CN / en。

发布者：

来自 `brand.json`。

安装包大小：

记录真实值。

不设置硬性 `<25MB` 作为功能门槛。

---

# 42. Updater

使用：

`tauri-plugin-updater`

私钥：

只能存在 CI Secret。

绝不进入仓库。

缺少 signing configuration：

必须优雅跳过签名步骤。

---

# 43. Code Signing

准备支持：

SignPath Foundation / SignPath OSS 流程。

仓库：

* public；
* OSI license；
* no proprietary components；
* CI-only release build。

必须有：

`CODE_SIGNING_POLICY.md`

明确：

* 签名角色；
* 2FA；
* only CI artifacts；
* SignPath attribution。

---

# 44. Website

`site/`

必须：

* static；
* single-page；
* zero external requests；
* dark；
* Aurora；
* four brand colors。

内容：

* hero；
* 3 key features；
* downloads；
* legal content source explanation；
* privacy summary；
* license summary；
* project links。

**不加入 ICP / 公安备案信息。**

---

# 45. Documentation

至少：

```text
README.md
ARCHITECTURE.md
SOURCES.md
CONTENT_POLICY.md
PRIVACY.md
BRAND.md
DECISIONS.md
PROGRESS.md
SECURITY.md
CODE_SIGNING_POLICY.md
THIRD_PARTY_LICENSES.md
CHANGELOG.md
docs/PERF.md
```

README：

中英双语。

必须准确说明：

* 能做什么；
* 不做什么；
* 合法来源；
* 隐私；
* 限制；
* 许可证；
* 第三方许可；
* 品牌资产边界。

---

# 46. Screenshots

使用 Playwright。

固定 viewport。

自动生成：

* Now Playing；
* Library；
* Map；
* Dreamscape；
* Settings / About。

写入：

```text
docs/screenshots/
```

README 引用。

---

# 47. Test Gate

每个 Milestone 完成：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
npm run typecheck
npm run lint
npm test
tauri build
```

所有通过才可 commit。

---

# 48. Deep Test Requirements

`aura-core`：

* gapless；
* fade；
* seek；
* underflow；
* hotplug simulation；
* zero allocation；
* deterministic timing；
* 1h soak test。

`aura-sources`：

* parser boundary；
* malformed response；
* fuzz/property testing；
* no panic。

在线来源：

测试只能使用本地 fixtures。

禁止真实网络测试。

---

# 49. Coverage

至少：

* aura-core >=75%
* aura-cache >=75%
* aura-sources >=75%

真实覆盖率：

写入 `PROGRESS.md`。

---

# 50. Performance

所有性能必须实测。

目标：

* cold start → interactive <1.5s；
* visual off playback CPU <2%；
* Aurora 60fps integrated GPU CPU <8%；
* 10,000-song memory <250MB；
* 10,000 files scan <60s；
* analysis >=15× real-time/core；
* 10,000 map points 60fps。

未达到：

必须记录：

* 实际结果；
* 测量方式；
* 环境；
* 瓶颈；
* 原因；
* 后续措施。

禁止伪造。

---

# 51. Milestones

## M0

Project foundation：

* workspace；
* CI；
* deny；
* brand；
* design tokens；
* IPC contracts；
* testkit；
* project docs。

## M1

Audio Engine：

* decode；
* resample；
* ring buffer；
* output；
* seek；
* volume；
* NullOutput。

## M2

Playback Quality：

* gapless；
* fade；
* loudness；
* hotplug；
* EQ。

## M3

Music Library：

* scanning；
* tags；
* SQLite；
* FTS5；
* artwork；
* watcher。

## M4

Sources：

* Source trait；
* local；
* direct URL；
* radio；
* ICY；
* Radio Browser；
* codec filtering；
* B-level RSS/HLS/WMF path where feasible。

## M5

Cache：

* chunked storage；
* Range；
* LRU；
* pin；
* persistence；
* fault injection。

## M6

UI Foundation：

* design tokens；
* Tauri shell；
* Mica fallback；
* command palette；
* Now Playing；
* queue；
* i18n；
* keyboard accessibility。

## M7

Dreamscape：

* synthesis；
* moods；
* deterministic seed；
* first-launch flow。

## M8

Analysis：

* BPM；
* key；
* LUFS；
* energy；
* timbre；
* embedding；
* deterministic fixtures。

## M9

Music Map：

* stable layout；
* WebGL map；
* lasso；
* color mapping；
* waveform。

## M10

Windows Integration：

* SMTC；
* tray；
* single instance；
* associations；
* crash recovery。

## M11

Online Sources：

* Subsonic；
* Jamendo；
* Internet Archive；
* attribution；
* license guards。

## M12

Advanced Experience：

* auto mix；
* path playlist；
* exploration fog；
* journey；
* practice；
* local lyrics；
* poster export。

## M13

Distribution：

* installer；
* updater；
* release workflow；
* signing preparation；
* site；
* license audit。

## M14

Final Release Audit：

* performance；
* screenshots；
* dependency audit；
* source audit；
* privacy audit；
* security audit；
* final quality gate。

---

# 52. Final Completion Criteria

最终必须同时满足：

1. 所有测试门禁通过。
2. cargo-deny 无违规。
3. 第三方许可清单与真实依赖一致。
4. 所有 A 功能完成。
5. 已完成 B 功能明确记录。
6. Audio callback 零分配。
7. gapless 样本正确。
8. seek 不产生旧 generation 音频。
9. cache fault injection 全部通过。
10. Forbidden 来源零写盘。
11. 不支持商业平台 URL 能被拒绝。
12. 官方 build 不包含默认 AAC decoder。
13. 分析 fixtures 通过。
14. Map 稳定性测试通过。
15. Dreamscape deterministic tests 通过。
16. 无音乐库首次启动仍有可用体验。
17. Aurora color mapping 纯函数测试通过。
18. 不存在业务裸色值。
19. Mica / Solid fallback 均通过。
20. 三主视图通过设计自检。
21. Playwright screenshots 已生成。
22. 无遥测。
23. CSP / capabilities 最小化。
24. Credential Manager 正确保存敏感凭据。
25. PRIVACY.md 与真实行为一致。
26. Installer 成功。
27. Release workflow 可运行。
28. site/ 离线可打开、零外部请求。
29. README 与实际代码一致。
30. PERF.md 记录真实数据。

---

# 53. Non-goals

当前版本不实现：

* 商业平台音频抽取；
* DRM；
* 登录态绕过；
* 云同步；
* 用户账号；
* 社交；
* 评论；
* 广告；
* 付费系统；
* 赞助入口；
* 遥测；
* 移动端；
* macOS/Linux 官方发行；
* 插件生态；
* WASAPI Exclusive Mode。

代码应尽量保持可移植性，但官方首发目标为 Windows。

---

# 54. 最终产品定义

ifuyo Aura 不是“另一个播放器”。

它应该最终成为：

> **一个把你的本地音乐变成一个可以聆听、探索、观察、漫游的个人音乐宇宙的 Windows 应用。**

核心体验：

```text
My Music
   ↓
Analysis
   ↓
Music Genome
   ↓
Music Map
   ↓
Aurora
   ↓
Dreamscape
   ↓
My Musical Universe
```

任何新增功能都必须回答：

> “它是否让音乐更好听、更容易探索，或者让这个音乐宇宙更加有生命力？”

否则不加入核心界面。
