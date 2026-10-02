# ifuyo Aura
# M5 — Persistent Network Cache / Range Reader / LRU / Pin / Recovery

你现在进入 ifuyo Aura 项目的 **M5：网络持久化缓存**。

项目正式名称：

`ifuyo Aura`

当前阶段目标：

> **在已经完成并通过审计的 M1 Audio Core、M2 Playback Quality、M3 Music Library、M4 Source System 基础上，实现商业产品级的网络持久化缓存系统。**

本阶段负责：

- `aura-cache` 正式建立或补全；
- 256KiB chunk cache；
- SQLite cache index；
- WAL；
- Range reader；
- 按需拉取；
- 顺序填充；
- prefetch；
- retry；
- exponential backoff；
- ETag；
- Last-Modified；
- offline cached playback；
- 2GB 默认容量；
- LRU eviction；
- pin；
- atomic write；
- crash recovery；
- FaultFS 故障注入；
- cache integrity check；
- Forbidden / SessionOnly / Allowed cache policy enforcement。

本阶段必须以：

> **M1 + M2 + M3 + M4 的真实代码**

作为唯一基线。

不得假设前序阶段的 API、线程模型、Source trait、Library API 与提示词完全一致。

必须先审计，再增量实现。

---

# 0. 本阶段绝对边界

## A 级必须实现

1. `aura-cache` crate。
2. CachePolicy enforcement。
3. content key。
4. chunk key。
5. 256KiB chunk storage。
6. SQLite cache index。
7. WAL。
8. atomic chunk write。
9. atomic metadata update。
10. cache open/reopen。
11. cache integrity checking。
12. cache index recovery。
13. HTTP Range reader。
14. missing chunk fetch。
15. sequential fill fallback。
16. seek-aware reads。
17. bounded prefetch。
18. ETag metadata。
19. Last-Modified metadata。
20. validation / revalidation。
21. finite retry。
22. exponential backoff。
23. partial download recovery。
24. offline playback of fully cached content。
25. cache size limit。
26. LRU eviction。
27. pin。
28. pin survives restart。
29. cache corruption recovery。
30. write failure recovery。
31. rename failure recovery。
32. index failure recovery。
33. crash simulation。
34. FaultFS integration。
35. Forbidden source zero-write proof。
36. SessionOnly source zero-persistence proof。
37. concurrent read safety。
38. concurrent fetch deduplication。
39. deterministic tests。
40. performance baseline。
41. M1/M2/M3/M4 regression。

---

## B 级

如果 A 级全部稳定后：

1. 更智能 prefetch。
2. range coalescing。
3. partial offline playback。
4. cache diagnostics。
5. cache usage breakdown。
6. manual cache clear primitives。
7. cache priority classes。

---

# 1. 本阶段明确禁止

不要实现：

- Music Map；
- BPM；
- Key；
- LUFS analysis；
- MFCC；
- embedding；
- Dreamscape；
- Aurora；
- Now Playing UI；
- Library UI；
- playlist UI；
- SMTC；
- tray；
- installer；
- updater；
- website；
- Steam；
- Jamendo；
- Internet Archive；
- Subsonic；
- 新的 Source provider；
- online lyrics；
- online artwork；
- 自动 metadata enrichment。

特别注意：

> **M5 是缓存层，不是新的来源层。**

不要借机扩展 M4 Source。

---

# 2. M5 的核心边界

M4：

> “获取网络内容。”

M5：

> “把允许缓存的网络内容安全地持久化。”

M1：

> “消费音频 PCM。”

因此：

必须形成：

```text id="s5w3jn"
Source
  ↓
Network Stream
  ↓
Cache Layer
  ↓
Decoder
  ↓
Resampler
  ↓
Audio Core
```

而不是：

```text id="n5dps8"
Source → Cache → Audio Core
```

Cache：

不能知道：

- WASAPI；
- CPAL；
- EQ；
- Aurora；
- UI。

---

# 3. CachePolicy 是硬门禁

必须严格支持：

```text
Allowed
SessionOnly
Forbidden
```

---

# 4. Allowed

允许：

- persistent chunks；
- SQLite cache metadata；
- LRU；
- ETag；
- Last-Modified；
- restart recovery；
- offline playback。

例如：

> 用户明确允许保存的合法直链内容。

---

# 5. SessionOnly

只允许：

> 当前进程生命周期内的内存缓存。

禁止：

- chunk 文件；
- persistent SQLite cache index；
- persistent metadata；
- restart recovery。

进程退出：

缓存必须消失。

---

# 6. Forbidden

例如：

> Internet Radio。

必须保证：

> **任何代码路径都不能向持久化 cache 写入音频数据。**

包括：

- direct chunk write；
- temporary file；
- SQLite cache index；
- revalidation metadata；
- retry temp data；
- fallback cache；
- crash recovery file。

---

# 7. 通用 Cache Policy Gate

建立：

```text id="s3w5rq"
CacheWriteGuard
```

或等价机制。

所有持久化缓存写入：

必须经过统一策略判断。

禁止：

> 某个新模块自己直接 `File::create()` 绕过 CachePolicy。

---

# 8. Cache Directory

默认位于：

> 应用数据目录中的 cache 目录。

例如：

```text id="j8s9v3"
%APPDATA%\ifuyo\ifuyo Aura\cache
```

最终路径：

必须由 application layer / configuration 提供。

`aura-cache`：

不得硬编码 `%APPDATA%`。

---

# 9. Cache Layout

推荐：

```text id="2d2s0y"
cache/
  chunks/
    <content-key>/
      00000000.bin
      00000001.bin
      00000002.bin

  tmp/
  
  metadata/
```

但是：

如果更合理的实际设计：

可以调整。

必须记录到：

`DECISIONS.md`

---

# 10. Chunk Size

固定：

```text id="w1zj40"
256 KiB
```

必须定义：

```text
CHUNK_SIZE = 256 * 1024
```

不得：

在不同模块偷偷使用不同 chunk size。

---

# 11. Content Key

Cache 内容不能简单使用：

> URL 字符串

作为唯一身份。

至少考虑：

```text id="vib4xx"
normalized URL
source identity
stream identity
```

最终 content key：

必须：

- deterministic；
- stable；
- collision-resistant。

如果使用 hash：

必须明确：

> hash 输入是什么。

---

# 12. URL Normalization

必须明确定义：

- scheme；
- host；
- port；
- path；
- query。

不要：

> 随意删除 query。

因为某些合法直链依赖 query 参数。

但：

如果 query 中包含：

- token；
- password；
- secret；

不得进入：

- logs；
- diagnostics；
- UI。

Cache key 可以使用：

> 安全 hash。

---

# 13. Signed URL

对于合法来源：

如果 URL 中存在：

> 临时签名参数

不得直接把完整 URL 打进日志。

Cache key：

可以：

> 对完整有效 URL 计算 hash。

但是：

缓存 index 中：

不要存明文 secret-bearing URL。

如果必须保留原 URL：

需要：

> redacted / encrypted / policy-controlled design。

优先：

> URL hash + sanitized metadata。

---

# 14. Chunk Key

必须能够唯一定位：

```text id="r7m7li"
content_key
+
chunk_index
```

例如：

```text
<content-key>/00001234.bin
```

---

# 15. SQLite Cache Index

使用：

`rusqlite`

与 M3：

> 不要共享同一个 Library connection。

缓存数据库：

可以独立。

目标：

> cache 生命周期与 library 生命周期解耦。

---

# 16. Cache Schema

至少：

```text id="n8a7c2"
cache_entries
cache_chunks
cache_pins
```

或者等价设计。

---

# 17. Cache Entry

至少存：

```text id="3rqj3q"
content_key
source_kind
source_url_hash
content_length
mime
codec
seekable
created_at
last_accessed_at
etag
last_modified
cache_policy
```

具体字段：

按实际实现调整。

---

# 18. Cache Chunk

至少：

```text id="r2b0mk"
content_key
chunk_index
offset
size
stored_size
sha256
last_accessed_at
created_at
```

不要因为已有 filesystem 文件：

就完全不维护 DB integrity metadata。

---

# 19. Hash

每个 chunk：

建议保存：

> SHA-256

或等价强哈希。

目的是：

- corruption detection；
- crash recovery；
- deterministic test。

如果选择其它方案：

必须记录。

---

# 20. Cache Write Atomicity

禁止：

```text id="7hjxqa"
直接写 chunk file
```

然后：

```text
写 DB
```

如果中途崩溃：

可能留下：

> 半块数据。

必须：

```text id="emw7xn"
write temp
↓
flush/close
↓
atomic rename
↓
transactionally record metadata
```

具体顺序：

根据真实 filesystem semantics 选择。

---

# 21. Chunk File Validation

打开 cache：

必须检查：

- file exists；
- expected size；
- hash matches；
- DB metadata matches。

不一致：

标记为：

> corrupted / missing

然后可以重新获取。

---

# 22. Orphan Chunk

filesystem 中存在：

```text id="79cku2"
chunk file
```

但 DB 没有记录：

必须：

> 不影响应用运行。

cache integrity pass：

可以：

- 删除；
- quarantine；
- report。

默认选择：

记录到 `DECISIONS.md`。

---

# 23. Missing Chunk

DB 有：

> chunk metadata

但是：

chunk file 不存在。

必须：

> 标记 missing。

读取时：

重新获取。

不得：

- panic；
- 返回脏数据。

---

# 24. Corrupted Chunk

hash 不一致：

必须：

> 丢弃并重新获取。

禁止：

> “尽量播放坏数据”。

除非明确是无法校验的 legacy entry。

M5 新生成数据：

必须可校验。

---

# 25. Cache Integrity Check

建立：

```text id="dx5c5j"
verify_cache()
```

至少检查：

- DB rows；
- chunk files；
- hash；
- size；
- orphan；
- missing；
- duplicate；
- pin integrity。

输出：

```text
entries_checked
chunks_checked
corrupted
missing
orphaned
repaired
```

---

# 26. Cache Open

第一次打开：

如果 cache 不存在：

自动创建。

已有 cache：

加载。

如果 DB 打不开：

不得：

> 静默删除整个 cache。

先尝试：

- reopen；
- integrity check；
- recovery。

---

# 27. Recovery Policy

如果 DB 严重损坏：

必须：

> 保留原 cache DB backup。

然后重建 index 的可选策略：

依赖：

> filesystem chunk metadata。

如果 filesystem 中缺失足够信息：

至少：

> 安全降级为空 cache。

但：

不得删除用户的完整音乐库数据库。

只处理：

cache。

---

# 28. Cache vs Library Isolation

特别检查：

cache corruption：

不得：

- 删除 library DB；
- 修改 track metadata；
- 修改 playlists；
- 修改 playback stats。

Library corruption：

也不应该：

> 自动删除 cache。

---

# 29. Range Reader

建立：

```text id="j2n6yu"
CachedRangeReader
```

目标：

提供：

```text
Read
Seek
```

或等价接口。

---

# 30. Read Model

读取：

```text id="j77b9q"
requested offset
↓
determine chunk(s)
↓
use cache if present
↓
fetch missing chunks if allowed
↓
return bytes
```

不得：

> 每次读取重新从 0 下载。

---

# 31. Chunk Boundary

一个 read：

可以跨越：

```text id="e47m49"
chunk N
chunk N+1
chunk N+2
```

必须正确拼接。

测试：

- exactly boundary；
- ±1 byte；
- large read。

---

# 32. Partial Chunk

最后一个 chunk：

可以小于：

`256KiB`

必须：

> 合法。

不能强制 padding 成整块音频。

---

# 33. HTTP Range

如果服务器支持：

```text id="0mmxip"
Range: bytes=start-end
```

可以按 chunk 获取。

例如：

```text
Range: bytes=524288-786431
```

注意：

M4 已经负责：

> 探测 seek capability。

M5 负责：

> 根据真实 server capability 实施 cache reads。

---

# 34. Server Range Validation

服务器返回：

`206 Partial Content`

必须验证：

- Content-Range；
- requested offset；
- requested length；
- actual body size。

不得：

> 服务器返回错误内容也直接写入 chunk。

---

# 35. 200 For Range Request

如果：

请求 Range：

服务器返回：

`200 OK`

不能直接假定：

> 它支持 Range。

必须根据真实行为：

- sequential fill；
- 或重新请求；
- 标记 non-range-capable。

记录决定。

---

# 36. Sequential Fill

服务器不支持 Range：

允许：

> 从当前已知位置顺序读取。

但：

不得：

> 为了 seek 到 100MB 自动下载 100MB 并无限缓存。

必须：

有界任务与明确策略。

---

# 37. Non-Range Source

如果：

- no Range；
- seek requested；

必须标记：

```text
seekable = false
```

或者：

> seek only within already cached data

具体行为：

记录。

不得伪造远程 seek。

---

# 38. Offline Playback

如果：

某内容：

> 所需音频数据全部在 cache。

并且：

没有网络：

必须可以：

> 完整播放。

测试环境：

必须人为关闭网络。

但：

测试不能依赖真实公网资源。

---

# 39. Offline Partial Cache

[A 级不要求完整 partial offline playback]

如果只缓存了：

50%：

M5 可以：

- 返回 cache miss；
- 或等待网络。

不得：

> 误报告“离线可播放”。

B 级可以实现部分 offline。

---

# 40. ETag

保存：

```text id="qv4o9p"
etag
```

重新验证：

可以使用：

```text
If-None-Match
```

服务器：

`304`

则：

> cache entry 保持有效。

服务器：

`200`

如果内容改变：

> 必须谨慎处理 content identity。

---

# 41. Last-Modified

同时支持：

```text
If-Modified-Since
```

但：

如果：

ETag + Last-Modified

都存在：

优先：

> ETag。

具体规则：

记录。

---

# 42. Cache Revalidation

不能每次播放：

> 无条件重新下载。

也不能：

> 永久信任旧 cache。

需要：

> 明确 revalidation policy。

至少：

- metadata validation；
- 304；
- changed content；
- deleted source。

具体 TTL：

如需：

必须记录。

---

# 43. Content Changed

如果服务器内容已经变了：

旧 cache：

不能继续混入新内容。

必须：

> 创建新的 content identity / invalidate old entry。

禁止：

> 同一个 content key 中一半旧、一半新。

---

# 44. Chunk-Level Integrity

每个 chunk：

必须：

> 单独可验证。

这样：

一个坏 chunk：

无需重新下载整个文件。

---

# 45. Retry

允许有限 retry。

推荐：

```text id="m2ig5h"
1
2
3
```

或当前工程确定的等价有限次数。

retry：

只针对可恢复错误。

例如：

- timeout；
- connection reset；
- transient 5xx。

---

# 46. No Retry

不应重复：

- 404；
- 403；
- unsupported codec；
- malformed response；
- forbidden source。

---

# 47. Exponential Backoff

backoff：

必须：

> bounded。

例如：

```text
base
2×
4×
...
max
```

不得：

无限增长。

---

# 48. Jitter

如果生产网络任务可能并发：

可以加入：

> bounded jitter。

但是：

测试模式：

必须可确定性。

因此：

支持：

```text
Production random jitter
Test deterministic jitter
```

---

# 49. Prefetch

[A]

当前播放：

允许提前请求：

> 邻近 chunk。

默认：

例如：

> 前方 2~4 chunks。

具体值：

必须根据：

- audio bitrate；
- network behavior；
- memory；

测试后决定。

写入：

`DECISIONS.md`

---

# 50. Prefetch Bound

prefetch：

必须有：

> 最大未完成任务数。

禁止：

> 一个用户 seek 就创建几百个下载任务。

---

# 51. Seek and Prefetch

用户 seek：

必须：

- 取消或降级旧 prefetch；
- 优先当前目标 chunk；
- 避免旧位置继续消耗带宽。

---

# 52. Generation Interaction

M1/M2 已有：

`Generation`

M5 必须支持：

> seek 后旧网络 fetch 的结果不能污染新播放 generation。

例如：

```text id="bjmz8e"
Old seek
 ↓
chunk fetch in flight
 ↓
New seek
 ↓
old fetch completes
```

旧 fetch：

可以进入 cache（如果内容 key 相同且合法）

但：

> 不得把旧 PCM / playback state 注入新 generation。

---

# 53. Fetch Deduplication

如果两个读取请求：

同时需要：

```text
chunk #42
```

不能：

> 下载两次。

应：

> shared in-flight request

或等价 deduplication。

---

# 54. Concurrent Readers

多个 reader：

可以同时读同一个 cache entry。

必须：

- 不破坏 chunk；
- 不互相覆盖；
- 不 corrupt metadata；
- access time 更新不会导致 DB race。

---

# 55. LRU

默认最大容量：

```text
2 GB
```

配置必须：

> 可调整。

---

# 56. Capacity Semantics

总容量：

到底统计：

- chunk bytes；
- temp files；
- metadata；
- pinned；

必须明确。

推荐：

> 只统计有效 chunk bytes。

metadata 可忽略。

记录。

---

# 57. LRU Key

淘汰优先依据：

```text
last_accessed_at
```

但：

不得让频繁读取单个 chunk：

导致其它内容永远无法淘汰。

如果采用更成熟：

> segmented LRU / weighted LRU

可以。

但：

第一版优先简单可靠。

---

# 58. Pinned Cache

支持：

```text id="mkwxhl"
pin(content_key)
unpin(content_key)
```

Pinned：

> 不得被普通 LRU 淘汰。

---

# 59. Cache Too Full

如果：

所有 entries 都 pinned：

无法满足：

新 chunk 写入。

必须返回：

```text id="c4yuw9"
CacheFullPinned
```

而不是：

> 删除 pinned 数据。

---

# 60. Atomic Eviction

淘汰：

必须：

- 更新 DB；
- 删除 chunk files；
- 保持一致。

不能：

DB 说不存在，但 file 还永远残留。

允许短暂 orphan：

但下一次 integrity check：

必须回收。

---

# 61. Eviction Concurrency

下载 chunk 的同时：

不能被 LRU 删除。

需要：

> in-flight protection / lease。

例如：

```text id="d3g3xj"
Fetching
Pinned
Reading
```

等价机制均可。

---

# 62. Access Time Updates

不要：

> 每读取一个 byte 就写 SQLite。

应该：

> 内存聚合 + 批量更新。

具体时间窗口：

根据性能决定。

测试必须证明：

不会因为播放：

> 产生高频 DB writes。

---

# 63. SQLite WAL

必须启用：

`WAL`

确保：

- read concurrency；
- safe transactions；
- recovery。

---

# 64. Cache DB Transactions

以下必须事务化：

```text id="q9abnf"
entry creation
chunk metadata insertion
pin change
eviction
revalidation state update
```

---

# 65. FaultFS

使用：

M0 testkit 的：

`FaultFS`

必须注入：

### Write failure

chunk 写入中断。

### Truncation

chunk 少写。

### Rename failure

atomic rename 失败。

### DB failure

metadata commit 失败。

### Restart

进程异常结束后重新打开。

---

# 66. Crash Recovery Scenario

测试：

```text id="p5g8ns"
download
↓
write temp
↓
crash
↓
restart
```

必须：

> 不把 temp 文件当作完整 chunk。

---

# 67. Crash During Rename

测试：

```text
write tmp
↓
rename
↓
crash boundary
```

重启：

必须：

- 检查 hash；
- 检查 DB；
- 清理 orphan；
- 恢复合法 chunk。

---

# 68. Crash During DB Commit

如果：

chunk file 已存在：

但：

DB transaction 未提交。

重启：

必须：

> 不产生 duplicate metadata。

---

# 69. Crash During Revalidation

服务器返回新 ETag：

但是进程崩溃。

重启：

必须：

> 不进入半新半旧状态。

---

# 70. Forbidden Source Test

必须建立：

```text id="m5q7m1"
test_forbidden_source_zero_persistent_writes
```

执行：

- stream data；
- metadata；
- retry；
- interruption；
- cancellation；
- shutdown。

最终：

cache filesystem：

> 零新增文件。

cache DB：

> 零新增 entry。

---

# 71. SessionOnly Test

必须建立：

```text id="rk4w6b"
test_session_only_not_persisted
```

启动：

> source。

产生：

> in-memory buffer。

关闭应用。

重新创建 cache：

必须：

> 找不到该内容。

---

# 72. Allowed Test

建立：

```text id="y4hec9"
test_allowed_persists
```

流程：

```text stream
↓
cache
↓
shutdown
↓
reopen
↓
offline read
```

必须：

> 成功读取。

---

# 73. Cache Policy Transition

如果同一 source：

从：

`SessionOnly`

变成：

`Allowed`

不能：

> 把旧 session buffer 静默写成 persistent cache。

必须由新的明确用户行为触发。

这条规则必须测试。

---

# 74. Content-Length

如果 HTTP response：

没有 Content-Length：

cache：

仍然可以处理流式内容。

但是：

必须：

- 不假设长度已知；
- 最终 EOF 后记录真实长度；
- 对未知长度资源限制内存 / disk growth。

---

# 75. Infinite Stream

例如：

Internet Radio：

即使 Source 被错误标记：

也不能：

> 无限写 cache。

Cache 层必须再次检查：

`Forbidden`

并拒绝持久化。

---

# 76. Live Stream Safety

对于：

```text
live = true
```

默认：

> 不持久 cache。

如果未来需要 timeshift：

另行设计。

M5 不实现 timeshift。

---

# 77. Cache API

建立：

```text id="u9s1vx"
CacheManager
CacheReader
CacheWriter
CacheEntry
CachePolicy
CacheStats
```

实际命名可调整。

API：

必须隐藏：

- SQLite internals；
- filesystem layout；
- SQL；
- HTTP client details。

---

# 78. Source Integration API

M4：

Source 提供：

```text
ResolvedStream
```

M5：

应该提供：

```text id="h2sx0h"
CachedStream
```

或等价抽象。

上层：

不要知道：

> 当前字节到底来自 cache 还是 HTTP。

---

# 79. Fallback Policy

Read：

先查 cache。

miss：

如果 policy Allowed：

> HTTP fetch。

SessionOnly：

> memory stream。

Forbidden：

> source stream。

不得：

> Forbidden → cache fallback。

---

# 80. Network Failure

如果：

cache 部分存在：

网络失败：

允许：

> 读取已缓存范围。

但：

如果播放器需要未缓存数据：

必须返回：

> network/source error。

不能：

> 假装 EOS。

---

# 81. Offline Detection

不要求提前检测“有没有网络”。

优先：

> 尝试 cache，再根据 fetch failure 判断。

不要：

> 自动 ping Google。

禁止无意义联网检测。

---

# 82. Retry + Cache

如果 chunk fetch 失败：

已有旧 chunk：

必须：

> 保留。

不要：

> 因为一次新 fetch 失败删除旧合法 cache。

---

# 83. Revalidation + Offline

离线：

如果 cache 足够播放：

应允许：

> 使用已有 cache。

不要：

> 因为无法 revalidate 就强制阻止播放。

但是：

必须区分：

- stale；
- validated；
- offline usable。

状态必须明确。

---

# 84. Cache Freshness

建立：

```text id="qwpwbm"
Fresh
Stale
Validated
Unknown
```

或者等价状态。

不要使用大量 bool。

---

# 85. URL / HTTP Header Safety

cache index：

不能保存：

- Authorization；
- Cookie；
- API keys；
- passwords。

如果 Source 使用敏感 headers：

必须：

> 与 cache key / metadata 隔离。

---

# 86. Logging

Cache 日志：

不得：

- 输出完整 signed URL；
- 输出 secrets；
- 输出 Authorization；
- 输出 Cookie。

可以记录：

```text
content_key
chunk_index
source host
operation
result
```

---

# 87. Telemetry

绝对不要新增 telemetry。

缓存命中率：

只能：

> 本地 diagnostics / metrics。

不得自动上传。

---

# 88. Metrics

允许建立：

```text id="xq9t84"
cache_hits
cache_misses
bytes_downloaded
bytes_served
evictions
corrupt_chunks
revalidations
```

但：

必须是：

本地内存统计。

未来 UI / logs 再读取。

---

# 89. Performance

更新：

`docs/PERF.md`

至少记录：

- chunk read；
- chunk write；
- SHA-256 verification；
- SQLite lookup；
- cache hit；
- cache miss；
- Range fetch；
- sequential fill；
- LRU scan；
- concurrent reads；
- fetch deduplication。

---

# 90. Performance Target

初步关注：

- cache hit 明显快于重新下载；
- DB lookup 不阻塞 audio callback；
- chunk verification 不进入 callback；
- cache eviction 不阻塞播放；
- large library/cache 不造成明显 UI/application stall。

不要伪造：

> “极低延迟”。

只记录实测。

---

# 91. Audio Callback Boundary

必须再次证明：

M5 绝不能让：

- filesystem；
- SQLite；
- HTTP；
- hashing；
- tracing；
- locks；

进入 audio callback。

缓存读取：

应该发生在：

> audio decode / producer side。

---

# 92. Thread Model

建议：

```text id="3riq1d"
Playback / Control
        ↓
Source / Cache Manager
        ↓
Network Fetch Workers
        ↓
Cache Storage
```

Audio callback：

> 只消费已经准备好的 PCM。

---

# 93. Shutdown

关闭应用：

必须：

- 停止新的 fetch；
- 取消未必要任务；
- 等待安全边界；
- flush 必要 metadata；
- 保证不会产生半写 chunk。

如果为了性能允许：

> background cleanup

必须确保：

不存在失控 task。

---

# 94. Cancellation

用户取消：

必须：

- cancel HTTP;
- stop prefetch;
- stop chunk assembly；
- release reader;
- 不产生 orphan tasks。

---

# 95. Fetch Ownership

同一 chunk：

如果：

task A 正在下载。

task B 请求：

> 必须等待 A 或共享结果。

不能：

> duplicate download。

A 取消：

B 仍需要：

可以接管或：

> 新建一次下载。

具体机制记录。

---

# 96. Cache Entry State

建议：

```text id="v5y4d3"
Absent
Fetching
Complete
Corrupt
Stale
Pinned
Evicting
```

不要让：

> 状态逻辑散布在几十个 if。

可以设计显式状态模型。

---

# 97. Tests — Basic Cache

至少：

```text id="g0hy2o"
test_cache_create
test_cache_reopen
test_chunk_write
test_chunk_read
test_chunk_hash
test_chunk_missing
test_chunk_corrupt
test_orphan_cleanup
```

---

# 98. Tests — Range

至少：

```text id="t7h9fc"
test_range_first_chunk
test_range_middle_chunk
test_range_last_chunk
test_range_crosses_boundary
test_range_exact_boundary
test_range_one_byte_before_boundary
test_range_one_byte_after_boundary
```

---

# 99. Tests — HTTP

使用：

> local fake HTTP server。

测试：

```text id="0g11cn"
206
200
416
404
500
timeout
etag
last_modified
redirect
oversized response
```

禁止公网测试。

---

# 100. Tests — Persistence

至少：

```text id="y2o2mr"
allowed_persists
session_only_not_persisted
forbidden_zero_write
reopen_after_restart
cache_capacity
lru_eviction
pin
pinned_survives_restart
all_pinned_cache_full
```

---

# 101. Tests — Recovery

至少：

```text id="m0zj4n"
write_failure
truncate_failure
rename_failure
db_failure
orphan_chunk
missing_chunk
corrupt_chunk
crash_before_commit
crash_after_file_write
recovery_after_restart
```

---

# 102. Tests — Concurrency

至少：

```text id="qkxevk"
concurrent_same_chunk
concurrent_different_chunks
reader_during_fetch
reader_during_eviction
pin_during_eviction
revalidate_during_read
cancel_during_fetch
```

禁止使用脆弱的固定：

`sleep 100ms`

同步线程。

---

# 103. Tests — Policy

至少：

```text
forbidden_no_file
forbidden_no_db_entry
forbidden_retry_no_write
forbidden_cancel_no_write
session_only_no_persistence
allowed_persists
unknown_policy_defaults_session_only
```

---

# 104. Determinism

相同：

- fixture；
- cache state；
- HTTP fixture；
- commands；

必须得到：

> 相同 cache state。

LRU 如果涉及同一时间戳：

测试必须使用：

> deterministic clock。

不要依赖 wall-clock。

---

# 105. Virtual Clock

为了测试：

支持：

```text id="8xw6w6"
VirtualClock
```

用于：

- last_accessed；
- TTL；
- retry backoff；
- revalidation。

生产环境：

可使用真实时钟。

---

# 106. Retry Determinism

测试：

如果 source 连续：

```text
timeout
timeout
success
```

必须：

> 在固定 retry schedule 下得到一致行为。

---

# 107. Cache Capacity Test

设置：

小容量，例如：

`1MiB`

写入：

多个 chunks。

验证：

> LRU 正确淘汰。

Pinned：

永不淘汰。

---

# 108. LRU Determinism

使用虚拟时间：

```text
A access
B access
C access
```

再超出容量。

必须：

> 淘汰最旧未 pin entry。

如果时间相同：

必须有：

> deterministic tie-breaker。

例如：

`content_key + chunk_index`

---

# 109. Cache Metrics Test

确保：

```text
hit
miss
write
evict
corrupt
```

统计：

不会出现：

- underflow；
- overflow；
- race；
- negative counts。

---

# 110. M4 Regression

M5 完成后：

必须重新验证：

### Direct HTTP

- resolve；
- stream；
- seek capability。

### Internet Radio

- ICY；
- non-seekable；
- Forbidden。

### Radio Browser

- station resolve；
- stream playback。

### Source security

- redirect；
- private network；
- credential URL。

任何 regression：

M5 FAIL。

---

# 111. M3 Regression

确认：

- local files 不被 cache 系统接管；
- Library 不受到 cache DB 影响；
- Local Source 的 CachePolicy 不改变 M3 行为。

---

# 112. M2 Regression

确认：

- cached stream → decoder；
- decoder → gapless；
- decoder → crossfade；
- network underrun；
- seek + generation。

尤其：

> cache refill 不破坏 generation。

---

# 113. M1 Regression

必须：

```powershell id="p98mfa"
cargo test --workspace
```

确认：

- decoder；
- resampler；
- ring buffer；
- NullOutput；
- seek；
- generation；
- volume；
- zero allocation。

---

# 114. No UI

M5 不实现：

- cache settings page；
- cache browser；
- progress UI；
- source UI；
- Library UI；
- Now Playing。

如果需要：

建立最小 typed diagnostics contract。

---

# 115. Security Audit

必须特别检查：

- cache path traversal；
- malformed content key；
- malicious filenames；
- symlink/reparse；
- temp file escape；
- atomic rename escape；
- signed URL leakage；
- SQL injection；
- unbounded cache growth；
- oversized chunk；
- corrupt metadata；
- malicious ETag / Last-Modified；
- redirect handling；
- HTTP response limits。

---

# 116. Filesystem Security

Cache 的：

`content_key`

绝不能：

直接成为：

> arbitrary filesystem path。

必须：

> sanitize / hash / fixed-depth mapping。

---

# 117. Symlink / Reparse

Cache directory：

原则上：

> 不跟随用户可控 symlink / junction。

如果发现：

> unexpected reparse point

应该：

安全拒绝 / quarantine。

记录实际策略。

---

# 118. Temp Files

临时文件：

只能：

位于：

> cache-controlled temp directory。

禁止：

将用户提供的 filename：

直接拼到 temp path。

---

# 119. Atomic Rename Safety

rename 前后：

必须验证：

> source + destination 均在 cache root。

防止：

path escape。

---

# 120. Dependency Audit

运行：

```powershell id="4qh4yx"
cargo tree
cargo deny check
```

重点：

- rusqlite；
- HTTP related dependencies；
- hashing；
- filesystem helpers；
- async task libraries。

不要为了 cache 添加重复 runtime。

如果 M4 已经使用 Tokio：

优先复用。

---

# 121. No Duplicate HTTP Clients

M5：

不得再引入第二套：

> HTTP client。

优先复用 M4 的网络抽象。

---

# 122. Cache API Contract

如果 M4 当前没有：

干净的：

```text
ByteStream
```

需要：

先建立最小 abstraction。

不得：

> 让 aura-cache 直接依赖 reqwest Response。

---

# 123. Error Model

建立：

```text id="8yt5a1"
CacheError
```

至少：

```text
PermissionDenied
DiskFull
Corrupt
Database
Network
RangeUnsupported
SourceChanged
CacheFullPinned
Cancelled
InvalidKey
```

具体可调整。

---

# 124. Disk Full

如果磁盘空间不足：

必须：

- 返回明确 error；
- 不 panic；
- 不损坏已有 chunks；
- 不删除 pinned；
- 尽可能保留已有合法 cache。

---

# 125. Permission Denied

cache 目录无法写：

必须：

> graceful degrade

例如：

- Allowed → SessionOnly；
- 或明确不可缓存。

具体行为：

记录 `DECISIONS.md`。

不要：

> 每次播放无限重试写盘。

---

# 126. Read-Only Cache

如果 cache filesystem：

只读。

允许：

> 读取已有 cache。

禁止：

> 强制要求写入才能播放。

如果内容已完整缓存：

应该能够离线播放。

---

# 127. Database Lock

SQLite lock：

必须：

> bounded retry / busy timeout

但：

不能：

> 无限等待。

---

# 128. Cache Clear Primitive

[B]

如果实现：

支持：

```text
clear_entry
clear_all
```

必须：

- transaction safe；
- no deletion of Library DB；
- no deletion of user music。

---

# 129. Cache Diagnostics

[B]

如果实现：

提供：

```text
entry count
total bytes
pinned bytes
hit rate
miss rate
corrupt count
```

全部：

> 本地诊断。

---

# 130. M5 Performance Review

更新：

`docs/PERF.md`

至少实际测量：

```text
cache hit read throughput
cache miss first-chunk latency
256KiB write throughput
SHA-256 throughput
SQLite lookup latency
concurrent chunk request behavior
LRU eviction duration
cache reopen time
integrity scan duration
```

不要为了性能：

- 关闭 integrity；
- 降低 hash；
- 删除 WAL；
- 删除 atomic write。

---

# 131. Soak Test

至少执行：

> 长时间 cache read/write soak。

建议：

循环：

```text
track A
track B
track C
seek
read
evict
re-fetch
```

持续足够长的时间。

如果 CI 不适合：

建立 extended local soak test。

不得把未运行写成 PASS。

---

# 132. Memory Bounds

验证：

缓存系统：

不会：

- 无限增长 RAM；
- 因单个巨大 response 产生无限 Vec；
- 因 prefetch 无限排队。

必须有明确：

> memory bound。

---

# 133. Disk Bounds

验证：

cache：

不会超出：

```text
configured_capacity + bounded temporary overhead
```

不要让：

temp files

无限增长。

---

# 134. Backpressure

下载速度大于：

消费速度：

cache fetch：

应该：

> 受限。

不能：

> 无限写入 / 无限排队。

---

# 135. Source Cancellation

关闭 playback：

如果：

cache fetch 不再需要：

应：

> cancel / deprioritize

但：

如果该 fetch 被另一个 reader 共享：

不能：

> 粗暴取消整个 shared fetch。

---

# 136. Shared Fetch Semantics

定义：

```text id="l1b4tm"
FetchKey = content_key + chunk_index
```

同一个 FetchKey：

共享任务。

不同 content：

完全隔离。

---

# 137. Chunk Lifecycle

建议：

```text
Missing
 ↓
Fetching
 ↓
Writing
 ↓
Committed
 ↓
Cached
 ↓
Evicted
```

错误：

```text
Fetching → Failed
Writing → Corrupt
```

状态必须：

可恢复。

---

# 138. No UI Coupling

Cache progress：

不得直接操作 React。

只产生：

typed state / events。

后续 M6 自行消费。

---

# 139. Documentation

更新：

`ARCHITECTURE.md`

加入：

```text
Cache Architecture
Cache Policy
Chunk Storage
Cache Index
Range Reader
Prefetch
Fetch Deduplication
Revalidation
LRU
Pin
Crash Recovery
FaultFS
```

更新：

`DECISIONS.md`

至少记录：

- content key；
- URL normalization；
- chunk size；
- cache path；
- SQLite separation；
- hash；
- atomic write；
- cache recovery；
- Range behavior；
- 200-vs-206 behavior；
- sequential fill；
- revalidation；
- retry；
- backoff；
- prefetch；
- deduplication；
- LRU；
- pin；
- virtual clock；
- SessionOnly；
- Forbidden；
- disk full；
- read-only cache。

---

# 140. `PROGRESS.md`

开始：

```text
M5-A = IN PROGRESS
M5-B = NOT STARTED
```

完成后真实记录：

- A 状态；
- B 状态；
- cache tests；
- recovery tests；
- concurrency；
- performance；
- policy tests；
- known limitations。

---

# 141. Cache Test Fixtures

所有 HTTP fixture：

必须：

- 本地；
- 可重复；
- 不访问公网；
- 不包含秘密。

必须包含：

- valid range；
- invalid range；
- changed ETag；
- 304；
- server no-range；
- timeout；
- failure。

---

# 142. No Real Network Tests

M5 CI：

绝对禁止：

- internet；
- Radio Browser；
- Jamendo；
- Internet Archive；
- user URLs。

全部使用：

> local fake server / fixtures。

---

# 143. Final Quality Gate

必须：

```powershell id="w6boez"
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

前端已有门禁：

```powershell id="mi7k9j"
npm run typecheck
npm run lint
npm test
```

也必须重新通过。

---

# 144. Full Regression

完整重新运行：

M1：

- Audio Core；
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

M4：

- Source；
- HTTP；
- ICY；
- Radio Browser；
- security；
- parser。

全部 PASS。

---

# 145. Diff Review

完成后：

先不要 commit。

执行：

```powershell id="m9y87n"
git diff --stat
git diff
```

重点检查：

- 是否偷渡 M6 UI；
- 是否偷渡 M7 Dreamscape；
- 是否偷渡 M8 analysis；
- 是否新增 Source provider；
- 是否允许 Forbidden write；
- 是否允许 SessionOnly persistence；
- 是否使用 URL 明文作为文件路径；
- 是否存在签名 URL 泄露；
- 是否存在 unbounded memory；
- 是否存在 unbounded disk；
- 是否存在 duplicate fetch；
- 是否存在 race；
- 是否存在 DB/file inconsistency；
- 是否取消了 WAL；
- 是否移除了 atomic write；
- 是否有 TODO；
- 是否有 ignored test；
- 是否弱化 assertions；
- 是否真实公网测试；
- 是否提交 cache；
- 是否提交本地数据库。

---

# 146. Final M5 Audit

输出：

# M5 Final Audit

## A. Baseline

## B. Cache Architecture

## C. CachePolicy

## D. Chunk Storage

## E. SQLite Index

## F. Atomic Write

## G. Integrity / Recovery

## H. Range Reader

## I. Sequential Fill

## J. Revalidation

## K. Retry / Backoff

## L. Prefetch

## M. Fetch Deduplication

## N. LRU

## O. Pin

## P. SessionOnly

## Q. Forbidden

## R. Concurrency

## S. Memory Bounds

## T. Disk Bounds

## U. Security

## V. Tests

## W. Performance

## X. M1 Regression

## Y. M2 Regression

## Z. M3 Regression

## AA. M4 Regression

## AB. Documentation

## AC. M5-A

## AD. M5-B

每项只能：

`PASS / FAIL / BLOCKED`

禁止使用：

- 基本完成；
- 大概可用；
- 理论支持；
- 应该没问题。

---

# 147. 必须提供的关键证据

最终必须明确提供：

### Cache

- chunk size；
- content key；
- cache capacity；
- actual stored bytes。

### Range

- 206；
- 200 no-range；
- boundary reads；
- seek behavior。

### Persistence

- shutdown；
- reopen；
- offline playback。

### Recovery

- write failure；
- truncation；
- rename failure；
- DB failure；
- restart。

### Cache Policy

- Allowed persists；
- SessionOnly does not persist；
- Forbidden zero-write。

### LRU

- eviction；
- pin；
- all-pinned behavior。

### Concurrency

- duplicate fetch prevention；
- concurrent reads；
- fetch cancellation。

### Security

- path isolation；
- URL secret redaction；
- bounded response；
- bounded disk；
- bounded memory。

---

# 148. Git Commit

只有：

- M5-A 全部 PASS；
- 已声明的 M5-B 全部 PASS；
- M1 regression PASS；
- M2 regression PASS；
- M3 regression PASS；
- M4 regression PASS；
- security PASS；
- quality gate 全绿；
- final audit PASS；
- diff review 干净；

才允许：

```powershell id="8e2p0j"
git add .
git commit -m "M5: implement persistent cache"
```

**不要 push。**

---

# 149. PROGRESS

更新：

```text
M5-A = PASS
```

如果 B 完成：

```text
M5-B = PASS
```

否则：

```text
M5-B = NOT COMPLETED
```

记录：

- cache entries；
- total bytes；
- tests；
- recovery tests；
- hit/miss benchmark；
- performance；
- known limitations。

---

# 150. 最终输出

最终输出：

```text id="glf8cn"
# M5 Final Audit

A. Baseline
B. Cache Architecture
C. Policy
D. Chunk Storage
E. Database
F. Range Reader
G. Revalidation
H. Retry
I. Prefetch
J. Deduplication
K. LRU
L. Pin
M. Recovery
N. Concurrency
O. Security
P. Tests
Q. Performance
R. M1 Regression
S. M2 Regression
T. M3 Regression
U. M4 Regression
V. Documentation
W. M5-A
X. M5-B

M5-A = PASS / BLOCKED
M5-B = PASS / NOT COMPLETED
COMMIT = <hash>
NEXT = M6
```

如果 M5-A / 已声明的 M5-B 全部通过：

**立即停止。**

**不要开始 M6。**