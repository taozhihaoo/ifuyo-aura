# SOURCES

ifuyo Aura 的音频来源分级与实现状态。政策依据见 [CONTENT_POLICY.md](CONTENT_POLICY.md)。

## 来源等级

### [A] 必须（Charter 强制）

| 来源 | 缓存策略 | 状态 |
|---|---|---|
| 本地音乐文件 / 用户选择目录 | Allowed | ✅ M3（M4 Source 化） |
| 合法 HTTP/HTTPS 直链音频 | SessionOnly 默认；Allowed 时持久缓存（M5） | ✅ M4+M5 |
| 网络电台（ICY） | Forbidden（≤30s 内存滚动） | ✅ M4 |
| Radio Browser | 目录 metadata ≠ 音频许可 | ✅ M4 |
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

## M4 实现明细（2026-10-04）

### 本地文件（aura-sources::local）
- 输入：用户选择的文件路径；metadata 复用 aura-library（不重复实现，§24）。
- 能力：Seekable / Finite / `CachePolicy::Allowed`。
- 验证日期：2026-10-04（fixture 测试）。

### 合法 HTTP/HTTPS 直链（aura-sources::resolver）
- 行为：GET + 头探测；`Accept-Ranges: bytes` + `Content-Length` → Seekable+Finite，否则 NonSeekable（不伪造 seek，§27）；`icy-*` 头出现 → 按电台处理。
- 限制：重定向 ≤5 且逐跳重校验；响应体（文本类）上限 8 MiB；Content-Type 仅作提示，最终由 decoder probing 判定（§26/§77）。
- 缓存政策：`SessionOnly`（M5 才有持久缓存，§94）。
- 验证日期：2026-10-04（fake server 矩阵测试，零公网请求）。

### 网络电台（aura-sources::radio）
- 行为：请求头 `Icy-MetaData: 1`；`icy-metaint` 剥离；`StreamTitle` → StreamMetadataEvent（§35/§91）。
- 能力：Live / NonSeekable；EOF = 流结束（§110）。
- 缓存政策：`Forbidden`——只允许 ≤30s 内存滚动缓冲（D-051），零写盘有测试（§38/§95）。
- 验证日期：2026-10-04。

### Radio Browser（aura-sources::radio_browser）
- 端点：发现 `https://all.api.radio-browser.info/json/servers`（不硬编码单一服务器，§40）；搜索 `/json/stations/search`（name/country/language/codec 过滤）；click 上报 `/json/url/<uuid>`。
- 请求行为：UA `ifuyo Aura/<version>`（§20/§43）；镜像随机选择（生产）/注入选择器（测试，§41/§124）；失败 fallback ≤3 镜像（§42）；body ≤64KiB/8MiB、超时（§50）。
- `url_resolved` 优先并记录实际使用字段（§45/D-054）。
- click 上报：**默认关闭**（§47）；关闭 = 零请求（有测试证明）。
- 缓存政策/许可：目录 metadata ≠ 音频许可（§51）；`ResolvedLicense::Unknown`，不假定（§52）。
- 离线：全失败 → `SourceUnavailable`，不换未知服务（§49）。
- 验证日期：2026-10-04（本地 fake server fixture；未做真实网络验证——按 §82 测试零真实网络）。

### 不支持平台
- `config/unsupported_platforms.json` 为唯一注册表（14 平台：YouTube/Spotify/网易/QQ/Apple Music 等）。
- 命中 → resolve 阶段拒绝，**不得发出任何请求**；DNS spy 测试证明零请求（§3/§87/§125）。

## 已知限制（M4）
- Podcast RSS / HLS / WMF AAC 为 B 级，本阶段未实现（resolver 对 .m3u8 给出明确 tier-B 提示）。
- AAC 电台流 → `UnsupportedCodec`（等 B 级 WMF 路径）。
- 自动重连未实现（§111：后续版本）。

## M8 分析算法来源（§256）

| 名称 | 类型 | 来源/依据 | 版本 | 许可 | 用途 | 核验日期 | 商业状态 | 已知限制 |
|---|---|---|---|---|---|---|---|---|
| ITU-R BS.1770-4 | 标准参考 | ITU-R 公开标准（建议书文本公开可读） | BS.1770-4 | 标准文本（实现为自有代码，无第三方代码） | LUFS/LRA/true peak 算法依据 | 2026-10-05 | 实现自有、无许可负担 | true peak 为 4× 过采样近似（非认证测量仪） |
| Krumhansl–Kessler 剖面 | 学术参考 | Krumhansl, *Cognitive Foundations of Musical Pitch*（公开文献数值表） | 经典版 | 剖面数值为公开学术事实（自有代码实现） | key 检测 24 假设相关 | 2026-10-05 | 同上 | 孤立三和弦/相对调区分度有限（D-101） |
| HTK mel 滤波器公式 | 公式参考 | HTK Book 公开的 hz↔mel 公式 | — | 公式为公开学术事实（自有代码实现） | MFCC mel 频带 | 2026-10-05 | 同上 | — |
| aura-spectral-stats | 自有嵌入模型 | 本仓库 aura-analysis/embedding.rs（谱统计+DCT） | model_version=1 | 本项目双许可 | 本地 embedding | 2026-10-05 | 完全自有 | 统计式而非学习式（§4 无伪造语义——真实信号统计） |

- **零第三方代码/模型/权重**（D-095/§254/§255：无隐藏模型许可问题）。
- 全部离线可运行（§328 offline audit PASS——分析链路无网络路径）。
