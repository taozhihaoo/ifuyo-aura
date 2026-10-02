# ifuyo Aura

> 以光与梦，编织每一次游玩。
> Weave light and dreams into every session.

**ifuyo Aura** 是一款面向 Windows 的本地优先、合法来源、无遥测的高品质桌面音乐播放器。它把你的本地音乐变成一个可以聆听、探索、观察、漫游的个人音乐宇宙：可探索的听感星图（Music Map）、音乐驱动的极光视觉（Aurora），以及内置的确定性生成音乐引擎（Dreamscape）。

**ifuyo Aura** is a local-first, legal-source, telemetry-free desktop music player for Windows. It turns your local music into a personal musical universe you can listen to, explore, observe, and wander: an explorable Music Map, music-driven Aurora visuals, and a built-in deterministic generative music engine (Dreamscape).

---

## English

### What it does

- Plays your **local music library** (FLAC / MP3 / OGG / WAV / M4A / ALAC) with a commercial-grade audio engine: gapless playback, crossfade, loudness normalization, parametric EQ.
- **Sources you own or that are legally open**: local files, direct HTTP(S) audio links, internet radio (ICY metadata), Radio Browser, and later Subsonic / Jamendo / Internet Archive.
- **Music Map**: your library laid out as a stable 2D star map of acoustic similarity, explorable with hover cards, lasso selection, and region playback.
- **Aurora**: a WebGL aurora as the Now Playing centerpiece, driven by key, energy, and BPM.
- **Dreamscape**: a deterministic procedural music engine, so the player is never empty even before you add music.
- Windows-native integration: SMTC, media keys, tray, single instance.

### What it deliberately does not do

- **No commercial-platform extraction.** No YouTube / Spotify / NetEase / QQ Music / Apple Music audio ripping, no yt-dlp-style behavior, no DRM circumvention, no login-session scraping, no online lyrics scraping. Unsupported platform URLs are rejected via `config/unsupported_platforms.json`.
- **No telemetry. Zero analytics. No ads. No accounts. No cloud sync.** Networking happens only when you trigger it (playing a direct URL, browsing radio stations), or when you explicitly opt in to online features.
- **No bundled music.** Demo content is fetched at runtime from legal open sources only.

### Privacy

See [PRIVACY.md](PRIVACY.md). Short version: your library, analysis data, and logs stay on your machine. Credentials for optional self-hosted services go into Windows Credential Manager, never to disk in plaintext.

### Status

Phase 0 (commercial foundation freeze) is complete. Milestones M0–M14 are tracked in [PROGRESS.md](PROGRESS.md). This is pre-alpha software under active development; features described above land milestone by milestone and are not all present yet.

### License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.

Third-party licenses: [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

### Brand assets

The name "ifuyo Aura", the vendor mark "ifuyo · 伊芙游", and the visual identity are brand assets of ifuyo · 伊芙游. See [BRAND.md](BRAND.md). You may reference the project by name and link to it; you may not present a modified build as official ifuyo Aura.

---

## 中文

### 它做什么

- 以商业产品级音频引擎播放你的**本地音乐库**（FLAC / MP3 / OGG / WAV / M4A / ALAC）：无缝播放、交叉淡入淡出、响度归一、参数 EQ。
- 只接受**你拥有的或法律上开放的来源**：本地文件、HTTP(S) 直链音频、网络电台（ICY 元数据）、Radio Browser，以及后续的 Subsonic / Jamendo / Internet Archive。
- **Music Map（音乐地图）**：把整个音乐库按听感相似度铺成一张稳定的二维星图，可悬停查看、套索选择、按区域播放。
- **Aurora（极光）**：以 WebGL 极光作为 Now Playing 主视觉，由调性、能量与 BPM 驱动。
- **Dreamscape（梦境）**：内置确定性程序化音乐引擎，没有本地音乐时播放器也不空转。
- Windows 原生集成：SMTC、媒体键、系统托盘、单实例。

### 它明确不做什么

- **不抓取商业平台。** 不做 YouTube / Spotify / 网易云 / QQ 音乐 / Apple Music 音频抽取，不做 yt-dlp 类行为，不破解 DRM，不模拟登录态，不联网抓歌词。不支持的平台 URL 通过 `config/unsupported_platforms.json` 识别并拒绝。
- **无遥测、零统计、无广告、无账号、无云同步。** 只有你主动触发（播放直链、浏览电台）或明确开启在线功能时才会联网。
- **不内置任何音乐文件。** 演示内容只通过合法开放来源在运行时获得。

### 隐私

见 [PRIVACY.md](PRIVACY.md)。简版：音乐库、分析数据与日志全部留在本机；可选自建服务的凭据存入 Windows Credential Manager，绝不明文落盘。

### 状态

Phase 0（商业化地基冻结）已完成，M0–M14 里程碑进度见 [PROGRESS.md](PROGRESS.md)。当前为活跃开发中的 pre-alpha 软件；上述能力按里程碑逐个落地，尚未全部存在。

### 许可证

采用 [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE) 双许可，任选其一。

第三方许可证：[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)。

### 品牌资产

"ifuyo Aura" 名称、"ifuyo · 伊芙游" 厂商标识与视觉身份是 ifuyo · 伊芙游 的品牌资产，边界见 [BRAND.md](BRAND.md)。可以按名称引用并链接本项目，但不得将修改版构建冒充为官方 ifuyo Aura。
