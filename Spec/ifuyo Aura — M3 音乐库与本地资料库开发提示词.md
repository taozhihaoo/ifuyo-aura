# ifuyo Aura
# M3 — Music Library / Metadata / SQLite / Search / Artwork

你现在进入 ifuyo Aura 项目的 **M3：音乐库与本地资料库**。

项目正式名称：

`ifuyo Aura`

当前阶段目标：

> **在已经完成并通过审计的 M1 Audio Core 与 M2 Playback Quality 基础上，实现商业产品级的本地音乐资料库。**

本阶段负责：

- 本地音乐文件夹扫描；
- 音频标签读取；
- SQLite 持久化；
- FTS5 全文搜索；
- 中英文搜索；
- 封面提取与缩略图缓存；
- 文件系统增量监听；
- 文件移动 / 重命名识别；
- 播放统计保留；
- 基础音乐库 API；
- 手动歌单与智能歌单的核心数据模型；
- 与 M1/M2 播放核心之间的干净边界。

本阶段必须以：

> **M1 + M2 的真实代码**

作为唯一基线。

不得假设之前架构与实际代码完全一致。

必须先审计，再增量实现。

---

# 0. 本阶段绝对边界

## 本阶段必须实现

[A]

1. `aura-library` 正式建立。
2. 本地目录扫描器。
3. 支持的音频文件扩展名过滤。
4. 文件合法性检测。
5. metadata extraction。
6. artwork extraction。
7. SQLite schema。
8. SQLite migrations。
9. WAL。
10. FTS5。
11. 中英文基础搜索。
12. CJK 二元组策略。
13. 单字前缀搜索策略。
14. 过滤表达式的核心解析接口。
15. 增量扫描。
16. filesystem watcher。
17. 新增文件检测。
18. 修改文件检测。
19. 删除文件检测。
20. 重命名 / 移动识别。
21. 内容身份键。
22. 播放统计保留。
23. artwork thumbnail cache。
24. 基础 Library service API。
25. deterministic tests。
26. integration tests。
27. migration tests。
28. corruption / failure tests。
29. performance baseline。

---

## B 级能力

本阶段允许完成：

1. 手动歌单核心数据模型。
2. 智能歌单核心数据模型。
3. 播放队列持久化模型。
4. queue undo 所需 command model。

但：

> 不实现最终 UI。

---

# 1. 本阶段明确禁止

不要实现：

- Radio Browser；
- ICY；
- HLS；
- Subsonic；
- Jamendo；
- Internet Archive；
- HTTP source；
- network cache；
- Dreamscape；
- Music Map；
- BPM analysis；
- key detection；
- LUFS analysis；
- MFCC；
- embedding；
- auto mix；
- WSOLA；
- SMTC；
- tray；
- installer；
- updater；
- website；
- Steam；
- lyrics network access；
- online metadata lookup；
- online artwork lookup。

特别禁止：

> 不要因为“音乐库未来需要 BPM / Key / LUFS”而提前实现 M8 analysis pipeline。

M3 只负责：

> **保存未来分析结果的位置和数据模型接口。**

不负责计算这些分析结果。

---

# 2. M3 的核心原则

音乐库必须：

> **本地优先、可恢复、幂等、增量、确定性、不会因为单个坏文件破坏整个扫描。**

单个文件：

- 损坏；
- 标签乱码；
- artwork 异常；
- 权限错误；
- 路径过长；
- 文件被占用；

都不能让整个 library scan 崩溃。

---

# 3. 开始前：Baseline Audit

在任何修改之前：

执行：

```powershell
git status
git branch
git log --oneline -10
cargo metadata
cargo tree
cargo test --workspace
```

并检查：

```text
crates/aura-core
crates/aura-testkit
crates/aura-library
src-tauri
ui
```

确认：

- M1 commit；
- M2 commit；
- M1/M2 测试数量；
- 当前 warnings；
- SQLite 是否已有；
- `aura-library` 是否已有；
- 当前 lockfile；
- `cargo-deny` 状态；
- 当前 Rust version；
- 当前 MSRV；
- 当前 CI。

输出：

# M3 Baseline Audit

A. Git  
B. M1 Status  
C. M2 Status  
D. Workspace  
E. Existing Library Code  
F. Dependencies  
G. Database State  
H. Test Infrastructure  
I. Risks

---

# 4. 先检查真实依赖

优先使用 M0 已批准依赖。

重点：

```text
lofty
rusqlite
notify
```

如果 M0 已经锁定其它等价方案：

优先复用。

新增依赖必须检查：

1. license；
2. MSRV；
3. Windows compatibility；
4. CI compatibility；
5. maintenance；
6. binary impact；
7. 是否增加架构耦合；
8. cargo-deny 是否允许。

不要因为未来功能提前安装：

- embedding；
- ML；
- HTTP；
- image processing；
- search framework；

等不必要依赖。

---

# 5. `aura-library` 职责

`aura-library` 只负责：

```text
Filesystem
    ↓
Scanner
    ↓
Metadata Extraction
    ↓
Normalization
    ↓
Identity
    ↓
SQLite
    ↓
Search / Query
```

不得负责：

- audio decoding；
- playback；
- output；
- network；
- UI；
- Dreamscape；
- visualization。

---

# 6. 建议模块结构

可以采用：

```text
crates/aura-library/
  src/
    lib.rs
    error.rs
    types.rs

    scanner/
      mod.rs
      filters.rs
      walker.rs
      fingerprint.rs

    metadata/
      mod.rs
      reader.rs
      normalize.rs
      artwork.rs

    db/
      mod.rs
      connection.rs
      migrations.rs
      schema.rs
      repositories.rs

    search/
      mod.rs
      tokenizer.rs
      parser.rs
      query.rs

    watcher/
      mod.rs
      events.rs

    playlists/
      mod.rs

  tests/
```

不要机械照抄。

真实职责优先。

---

# 7. Library Domain Model

建立核心类型：

```text
Track
TrackId
TrackIdentity
TrackMetadata
Artwork
ArtworkRef
LibraryPath
ScanResult
ScanStatus
PlaybackStats
```

未来分析数据允许：

```text
AnalysisSummary
```

但本阶段：

只定义存储契约。

---

# 8. Track Identity

这是 M3 最重要的数据设计之一。

不能使用：

> file path

作为歌曲永久身份。

因为用户可能：

- 改名；
- 移动文件；
- reorganize folders；
- 更换盘符。

必须建立：

> content identity / stable identity

---

# 9. Content Identity

根据总章程：

推荐使用：

```text
file size
+
first 1MB
+
last 1MB
```

生成确定性 hash。

但是：

> 必须先判断真实实现的性能与碰撞风险。

如果选择等价方案：

必须记录到：

`DECISIONS.md`

目标：

同一文件改名：

`TrackId` 不变。

同一文件移动：

`TrackId` 不变。

---

# 10. Identity 行为

### Rename

路径变化：

TrackId 不变。

### Move

路径变化：

TrackId 不变。

### Metadata Only Change

TrackId：

不应因为标签改变而永久改变。

但：

metadata version / modified state：

必须更新。

### Actual Audio Content Change

应生成新的 content identity。

旧 track：

按照真实行为处理。

必须写入：

`DECISIONS.md`

---

# 11. File Scan

支持：

```text
.flac
.mp3
.ogg
.oga
.wav
.m4a
```

以及当前商业化章程明确支持的其他本地容器。

注意：

`.m4a`

不能简单视为：

> 必然 AAC。

应根据：

M2 的 codec policy

处理。

M3 只负责发现和 metadata extraction。

---

# 12. Extension Filter

扫描器：

先用扩展名快速过滤。

再：

> metadata / format detection

确认是否真的是可识别音频。

例如：

```text
fake.mp3
```

实际可能是：

PNG / ZIP / text。

不能进入正常音乐库。

---

# 13. Symlink / Reparse Policy

Windows 环境：

必须明确：

- symbolic links；
- junctions；
- reparse points；

如何处理。

默认建议：

> 不递归跟随未知 reparse point。

防止：

- infinite recursion；
- scanning outside selected root。

如果实现跟随：

必须：

- cycle detection；
- canonical identity；
- root boundary enforcement。

把最终决定写入：

`DECISIONS.md`。

---

# 14. Root Boundary

用户选择：

```text
D:\Music
```

扫描：

原则上只能位于允许 root 下。

必须防止：

- path traversal；
- unintended root escape；
- malformed paths。

如果 canonicalization 后路径超出 root：

拒绝。

---

# 15. Scanner

扫描器必须：

- iterative；
- cancellable；
- error tolerant；
- deterministic where possible。

不能因为：

```text
1 个文件 PermissionDenied
```

导致：

整个扫描失败。

每个文件记录：

```text
discovered
readable
recognized
imported
skipped
failed
```

---

# 16. Scan Result

最终输出至少包含：

```text
files_seen
files_imported
files_updated
files_deleted
files_skipped
files_failed
duration
```

错误：

必须包含：

- path；
- error category；

但不得泄漏敏感信息。

---

# 17. Incremental Scan

第二次扫描：

不要重新完整解析所有文件。

必须根据：

- path；
- metadata timestamp；
- file size；
- content identity；

判断：

是否需要重新处理。

如果文件没有变化：

> 不重复写数据库。

---

# 18. Idempotency

执行：

```text
scan()
scan()
scan()
```

连续三次。

最终：

> database state 必须一致。

不能产生：

- duplicate track；
- duplicate artwork；
- duplicate playlist member；
- duplicate statistics。

---

# 19. SQLite

使用：

`rusqlite`

默认：

`bundled`

如果 M0 已确定其它兼容方案：

记录。

数据库：

必须使用：

`WAL`

---

# 20. Database Location

用户库数据库应该位于：

用户应用数据目录。

不要：

- 写入源音乐目录；
- 写入仓库；
- 写入安装目录。

路径由 Tauri/application layer 决定。

`aura-library`：

只接受抽象后的 database path / connection。

不要硬编码：

`%APPDATA%`

到核心 crate。

---

# 21. SQLite Schema

必须至少支持：

```text
tracks
track_paths
artists
albums
genres
artworks
playback_stats
library_roots
```

如果 B 级 playlist 完成：

增加：

```text
playlists
playlist_tracks
smart_playlists
```

不要建立没有真实用途的大型 schema。

---

# 22. Track Schema

至少包含：

```text
id
identity_key
path
title
artist
album
album_artist
year
genre
track_number
disc_number
duration_frames
sample_rate
channels
codec
container
file_size
modified_time
date_added
last_scanned
```

以及：

未来分析结果的扩展位置。

但是：

> 不要在 M3 伪造 BPM / Key / LUFS。

---

# 23. Metadata Normalization

音乐标签经常出现：

- 空字符串；
- 空白；
- 不同大小写；
- Unicode normalization 差异；
- malformed text；
- 重复 artist；
- 不统一年份格式。

必须设计：

`normalize_metadata()`

要求：

- deterministic；
- 可测试；
- 不破坏原始信息。

---

# 24. Raw vs Normalized Metadata

推荐数据库区分：

```text
raw metadata
normalized metadata
```

或者：

保留 raw value 的等价设计。

重点：

> 搜索依赖 normalized metadata，但 UI 可以访问原始显示值。

---

# 25. Unicode

正确处理：

- UTF-8；
- emoji；
- 中文；
- 日文；
- 韩文；
- accented Latin；
- combining marks。

不得：

- lossy conversion；
- ASCII-only normalization。

---

# 26. 艺术家与专辑

重复实体：

不要无意义复制。

例如：

100 首同一 Artist 的歌：

Artist 可以共享一个实体。

但如果实体去重规则复杂：

优先保持：

> 实现简单、查询稳定。

不要为了极致数据库范式过度设计。

---

# 27. Artwork

从音乐文件：

读取 embedded artwork。

支持：

- JPEG；
- PNG；
- 项目实际 codec 支持的常见图片格式。

如果存在多个 artwork：

明确选择策略。

例如：

> 优先 front cover。

记录：

`DECISIONS.md`

---

# 28. Artwork Storage

不要把完整原始 artwork 反复嵌入每个 Track row。

建立：

```text
artworks
```

通过：

`artwork_id`

引用。

可以按：

artwork content hash

去重。

---

# 29. Thumbnail Cache

为 UI 后续使用：

生成多个尺寸，例如：

```text
64
256
```

具体尺寸可根据实际 UI 需要调整。

要求：

- deterministic；
- atomic write；
- corrupt cache 可重新生成；
- 不影响 library DB 正常使用。

---

# 30. Artwork Cache Location

只能写：

应用数据 / artwork cache。

不得修改：

> 用户原始音乐文件。

---

# 31. Artwork Failure

如果封面：

- malformed；
- too large；
- decode failure；

歌曲仍然必须能够成功进入库。

Artwork failure：

只是：

`artwork_unavailable`

不是：

`track_import_failed`

---

# 32. FTS5

数据库必须支持：

FTS5。

至少搜索：

- title；
- artist；
- album；
- album_artist；
- genre；
- path。

---

# 33. 中文搜索

实现：

> CJK 二元组 tokenization。

示例：

`流光音乐`

至少能拆成：

```text
流光
光音
音乐
```

具体实现：

必须有单元测试。

---

# 34. 单字前缀

对于：

```text
音
```

应可以匹配：

包含该单字起始 token 的结果。

不得破坏普通英文前缀搜索。

---

# 35. 英文搜索

必须支持：

- case-insensitive；
- prefix；
- normal word matching。

例如：

```text
pink
```

能够匹配：

`Pink Floyd`

---

# 36. 搜索 Ranking

第一版保持简单。

优先考虑：

1. title exact / prefix；
2. artist exact / prefix；
3. album；
4. remaining fields。

如果 SQLite 查询策略无法可靠表达：

采用：

FTS rank + deterministic secondary ordering。

不得使用随机排序。

---

# 37. 搜索结果稳定性

相同数据库：

相同查询：

必须：

> 得到相同顺序。

除非用户明确指定：

`ORDER BY`

---

# 38. Filter Query Language

为未来 UI / Library 使用：

支持：

```text
bpm:120-130
key:8A
energy:>0.6
license:cc
```

但 M3：

> 不要求真正拥有 BPM / Key / Energy 数据。

因此：

必须设计：

```text
FilterExpression
```

以及：

future-capable AST。

缺失字段：

返回：

> no match / unsupported filter

而不是猜值。

---

# 39. Filter Parser

至少支持：

```text
field:value
field:>value
field:<value
field:min-max
```

Parser：

必须：

- deterministic；
- recoverable；
- clear errors；
- no panic。

---

# 40. Library Query API

建立统一接口：

```text
list_tracks(...)
get_track(...)
search_tracks(...)
filter_tracks(...)
get_artwork(...)
get_artists(...)
get_albums(...)
```

这些 API：

不得返回 database-specific internals。

---

# 41. Pagination

大量歌曲不能一次全部加载到内存。

支持：

```text
limit
offset
```

或：

> cursor based pagination

具体选择：

记录到 `DECISIONS.md`。

优先：

简单、稳定、容易供 M6 virtualization 使用。

---

# 42. Sorting

至少支持：

```text
title
artist
album
date_added
duration
year
```

排序：

必须稳定。

`NULL` / missing：

要定义排序位置。

---

# 43. 文件系统监听

使用：

`notify`

监视：

用户选择的 library roots。

监听：

- create；
- modify；
- remove；
- rename。

---

# 44. Watcher 不直接改数据库

推荐：

```text
Filesystem Event
       ↓
Debounce
       ↓
Rescan Candidate
       ↓
Metadata Extraction
       ↓
Library Service
       ↓
SQLite
```

Watcher：

只负责：

> 提供事件。

不要把数据库写操作散落在 watcher callback 中。

---

# 45. Debounce

某些文件复制/写入过程会触发大量事件。

必须进行：

> debounce / coalescing。

不能：

每次 `Modify`

都立即解析文件。

具体时间窗口：

根据实际测试决定。

记录：

`DECISIONS.md`

---

# 46. Partial File Handling

文件正在复制时：

可能：

- size 不稳定；
- metadata 不完整；
- parser 失败。

扫描器应该：

- 识别失败；
- 延迟重试；
- 不污染数据库。

不得将：

> 半写入状态

当作稳定音乐 metadata。

---

# 47. Delete Handling

文件删除：

必须从：

> active library

中移除 / 标记 deleted。

但：

必须根据产品数据模型决定：

是否保留播放历史。

默认：

> 播放统计可以保留在逻辑 identity/history 层。

不要因为删除文件：

把所有历史统计静默销毁。

---

# 48. Rename / Move Handling

Watcher 收到 rename：

必须尽量识别：

> old identity == new identity

最终：

- TrackId 不变；
- stats 不丢；
- artwork 引用不丢。

如果无法识别：

按：

新文件 + 旧文件删除

处理。

但必须避免重复实体长期存在。

---

# 49. Database Transactions

所有：

> track import/update/delete

都必须使用事务。

例如：

metadata update：

必须保证：

```text
track
+
path
+
artwork reference
```

不会处于半更新状态。

---

# 50. Crash Safety

模拟：

在以下步骤中失败：

- track insert；
- metadata update；
- artwork reference；
- stats update。

重新打开数据库：

必须：

> SQLite 保持一致。

测试：

必须使用 transaction rollback。

---

# 51. Migration System

建立：

```text
schema_version
```

或者等价机制。

必须支持：

```text
fresh database
old database
new database
```

migration：

必须：

- deterministic；
- idempotent where applicable；
- transaction safe。

---

# 52. Migration Tests

至少：

### Fresh

空数据库 → 最新 schema。

### Upgrade

旧 fixture → 最新 schema。

### Re-run

migration 再跑一次：

不能破坏数据库。

### Failure

migration 中途故意失败：

必须 rollback。

---

# 53. Database Corruption Handling

如果 SQLite 打开失败：

应用层未来应能够：

- 显示数据库异常；
- 不 panic。

M3 至少：

提供明确：

`DatabaseError`

并保证错误路径没有：

> `unwrap()` panic。

---

# 54. Playback Statistics

至少支持：

```text
play_count
skip_count
last_played_at
total_played_frames
```

字段可按实际设计调整。

播放统计：

不得与物理路径绑定。

核心依据：

> Track identity。

---

# 55. Statistics Semantics

必须定义：

什么算：

- play；
- completion；
- skip；
- partial play。

M3 只建立：

> storage model

M1/M2 不需要回头接入完整统计逻辑。

未来由 application layer 集成。

---

# 56. Analysis Placeholder

建立一个未来兼容结构，例如：

```text
track_analysis
```

允许未来存：

```text
bpm
bpm_confidence
key
camelot_key
lufs
true_peak
energy
brightness
embedding
analysis_version
```

但是：

**M3 不生成这些值。**

---

# 57. Analysis Version

以后分析算法升级：

必须允许：

```text
analysis_version
```

不同版本重新分析。

不要把分析值直接作为永久真相。

---

# 58. Manual Playlist — [B]

如果本阶段实现：

支持：

```text
playlists
playlist_tracks
```

要求：

- stable ordering；
- duplicate policy；
- delete semantics；
- transaction。

不要实现 UI。

---

# 59. Smart Playlist — [B]

至少存：

```text
name
query
sort
limit
```

例如：

```text
energy:>0.6
```

但由于 M3 没有真实分析数据：

只建立：

> executable query representation

不要伪造分析结果。

---

# 60. Queue Model — [B]

如果实现：

建立：

```text
QueueItem
queue_order
track_id
source_context
```

要求：

未来可以支持：

- drag reorder；
- remove；
- undo。

但：

M3 不做 UI。

---

# 61. Library API 与 Audio Core

`aura-library`：

不得直接控制：

- WASAPI；
- CPAL；
- audio callback。

上层 application layer：

负责：

```text
Library
 ↓
Track reference
 ↓
Audio Core
```

保持：

> Library 与 Audio Core 解耦。

---

# 62. File Access

M3 的 library scanner：

可以读取用户主动选择的音乐目录。

不得：

- 扫描整个 C:\；
- 扫描 Windows 系统目录；
- 自动搜集用户其他文件；
- 绕过用户指定 root。

---

# 63. Privacy

M3 处理的数据：

默认只保存在：

> 本地。

不要上传：

- 文件名；
- 音乐 metadata；
- 文件路径；
- artwork；
- playback statistics。

不得联网做 metadata enrichment。

---

# 64. Security

防止：

- path traversal；
- symlink escape；
- malformed metadata；
- image decompression abuse；
- gigantic artwork；
- database injection；
- FTS query injection；
- runaway recursive scan。

SQLite 查询：

必须使用参数化 SQL。

禁止：

字符串拼接 SQL。

---

# 65. FTS Query Safety

用户输入：

不能直接拼成：

```sql
MATCH '...'
```

必须：

- sanitize；
- parameterize；
- escape FTS syntax；

或者使用安全的 query builder。

测试：

- quotes；
- colon；
- parentheses；
- hyphen；
- wildcard；
- Chinese；
- emoji。

不得 panic。

---

# 66. Artwork Safety

对 artwork：

设置合理限制：

- 最大尺寸；
- 最大解码内存；
- 最大文件大小。

超限：

跳过 artwork。

不能因为一张恶意图片：

让扫描整个音乐库失败。

---

# 67. Scan Cancellation

必须支持：

```text
CancellationToken
```

或等价机制。

用户取消：

扫描应在合理边界停止。

不能：

继续扫描大量剩余目录。

取消：

不得留下：

> half-written transaction。

---

# 68. Progress Reporting

建立结构化 progress：

```text
ScanProgress
  discovered
  processed
  imported
  failed
  current_path
```

未来 M6 可直接接 UI。

但是：

M3 不做 UI。

---

# 69. Deterministic Test Fixtures

建立测试音乐 fixture：

至少包含：

```text
FLAC
MP3
OGG
WAV
```

如果合法且可安全分发。

同时使用 synthetic / tiny fixtures。

每个 fixture：

记录：

- format；
- source；
- license；
- expected metadata。

不得提交真实商业音乐。

---

# 70. Metadata Test Matrix

测试：

### 完整标签

title / artist / album 全部存在。

### 缺失标签

只存在 title。

### Unicode

中文 / 日文 / accented text。

### Empty

空字符串。

### malformed

异常 metadata。

### Multiple artwork

验证选择策略。

### invalid file

格式错误。

任何单文件错误：

不得 panic。

---

# 71. Scanner Test Matrix

至少：

```text
empty directory
single file
nested directory
100 files
duplicate content
renamed file
moved file
deleted file
modified metadata
modified audio
permission failure
malformed audio
unsupported extension
fake audio extension
partial file
```

---

# 72. Incremental Test

流程：

```text
initial scan
↓
rename
↓
scan
↓
assert same TrackId

move
↓
scan
↓
assert same TrackId

metadata edit
↓
scan
↓
assert same TrackId + metadata updated

audio replacement
↓
scan
↓
assert new identity
```

---

# 73. Watcher Integration Test

使用：

temporary directory

执行：

```text
create
modify
rename
delete
```

验证：

database 最终状态正确。

注意：

测试不得依赖固定 sleep。

使用：

- deterministic polling；
- event synchronization；
- retry with bounded condition wait；

而不是：

> “sleep 500ms 应该就好了”。

---

# 74. Database Tests

必须测试：

- insert；
- update；
- delete；
- transaction；
- rollback；
- migration；
- reopen；
- WAL；
- concurrent reads；
- duplicate identity。

---

# 75. Search Tests

至少：

```text
search_title
search_artist
search_album
search_path
search_chinese
search_single_character
search_prefix
search_case_insensitive
search_special_characters
search_empty
search_no_result
stable_ordering
```

---

# 76. Library Consistency Tests

最终 database：

必须满足：

- active path 唯一；
- identity 唯一；
- no duplicate tracks；
- no dangling artwork reference；
- no dangling playlist reference；
- no impossible null states。

建立：

`library_integrity_check()`

或等价诊断 API。

---

# 77. Repair / Recovery

实现一个最小：

```text
library integrity check
```

用于发现：

- orphan artwork；
- missing files；
- duplicate identity；
- invalid path；
- broken foreign key。

如果修复逻辑复杂：

本阶段至少提供：

> diagnose

不要为了 M3 强行建立“大修复系统”。

---

# 78. Performance Baseline

至少建立 benchmark：

### Scan

1,000 files。

### Search

10,000 records。

### Metadata Extraction

single file / batch。

### Artwork

thumbnail generation。

### Database

bulk insert / query。

记录：

`docs/PERF.md`

注明：

- machine；
- OS；
- Rust version；
- database mode；
- fixture size；
- command；
- result。

不要伪造目标达成。

---

# 79. 目标

初始参考目标：

- 10,000 首库可正常查询；
- 10,000 个文件扫描性能可接受；
- 搜索结果交互级延迟；
- 不产生明显 UI-blocking database operation。

如果实际未达到：

记录事实和原因。

不要为了满足数字削弱功能。

---

# 80. Logging

普通扫描线程允许：

`tracing`

日志：

不得：

- 输出全部音乐文件路径到 debug 日志；
- 输出敏感用户数据；
- 上传日志。

错误应尽可能：

> category + concise path context

而不是整个 metadata blob。

---

# 81. M3 不修改最终 UI

允许：

为未来 M6 提供：

- DTO；
- query contracts；
- library state model；

但是：

不得实现：

- Library 页面；
- 搜索框 UI；
- Artwork grid；
- virtual list；
- playlist UI。

M3 只实现 backend / domain / persistence。

---

# 82. M3 不实现网络

本阶段：

测试与实现均：

> 零真实网络请求。

任何：

- HTTP；
- DNS；
- Radio Browser；
- Internet Archive；
- Jamendo；

都属于后续 Milestone。

---

# 83. Migration / DB Versioning

在数据库中保留：

```text
schema_version
```

或正式 migration mechanism。

每次 schema change：

必须：

- migration；
- test；
- documentation。

禁止：

> 直接删除数据库重建来“完成 migration”。

---

# 84. Dependency Audit

运行：

```powershell
cargo tree
cargo deny check
```

确认：

新增依赖：

全部符合许可白名单。

尤其关注：

- sqlite bundled dependencies；
- image codecs；
- metadata parser dependencies。

真实许可证信息：

以 dependency tooling 为准。

不要手工猜测。

---

# 85. M1 / M2 Regression

M3 完成后：

必须重新运行：

```powershell
cargo test --workspace
```

并确认：

M1：

- audio core；
- seek；
- generation；
- NullOutput；
- zero allocation；

全部仍通过。

M2：

- gapless；
- crossfade；
- loudness；
- hotplug；
- EQ；

全部仍通过。

任何 regression：

M3 FAIL。

---

# 86. 最终质量门禁

必须：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

如果前端已有：

```powershell
npm run typecheck
npm run lint
npm test
```

也必须重新通过。

---

# 87. Diff Review

完成实现后：

先不要 commit。

执行：

```powershell
git diff --stat
git diff
```

检查：

- 是否偷渡 M4；
- 是否偷渡 M8；
- 是否引入真实网络；
- 是否引入在线 metadata；
- 是否伪造 BPM / Key / LUFS；
- 是否写入音乐源目录；
- 是否把 path 当永久 identity；
- 是否有 SQL string concatenation；
- 是否有 FTS injection；
- 是否有 watcher callback 直接写 DB；
- 是否有 blocking scan；
- 是否有大量无必要依赖；
- 是否有 TODO；
- 是否有 ignored tests；
- 是否弱化 assertion；
- 是否提交真实音乐；
- 是否提交本地 DB；
- 是否提交真实 artwork。

---

# 88. 最终 M3 Audit

输出：

# M3 Final Audit

## A. Baseline

## B. Scanner

## C. Metadata

## D. Identity

## E. SQLite

## F. Migrations

## G. FTS5

## H. CJK Search

## I. Artwork

## J. Incremental Scan

## K. Watcher

## L. Rename / Move

## M. Playback Statistics Model

## N. Playlist Model

## O. Security

## P. Performance

## Q. Tests

## R. M1 Regression

## S. M2 Regression

## T. Dependencies

## U. Documentation

## V. Scope Compliance

每项只能：

`PASS / FAIL / BLOCKED`

不能使用：

- 基本完成；
- 大致可用；
- 应该没问题；
- 理论上支持。

---

# 89. 必须提供的关键证据

最终必须明确：

### Library

- 支持的文件格式；
- 实际扫描测试；
- 增量扫描测试；
- 幂等测试。

### Identity

- rename 保持 TrackId；
- move 保持 TrackId；
- content replacement 创建新 identity。

### Database

- fresh migration；
- upgrade migration；
- rollback；
- reopen；
- WAL。

### Search

- English；
- Chinese；
- CJK bigram；
- single-character；
- special characters。

### Artwork

- extraction；
- thumbnail；
- corrupt image handling。

### Watcher

- create；
- modify；
- rename；
- delete。

### Security

- root boundary；
- symlink/reparse policy；
- parameterized SQL；
- FTS query safety。

---

# 90. Git Commit

只有：

- A 级全部 PASS；
- B 级如果声明完成则全部 PASS；
- M1 regression PASS；
- M2 regression PASS；
- quality gate 全绿；
- final audit 全 PASS；
- diff review 干净；

才允许：

```powershell
git add .
git commit -m "M3: implement music library"
```

不要 push。

---

# 91. PROGRESS

更新：

```text
M3 = PASS
```

记录：

- files scanned；
- tracks imported；
- tests；
- migrations；
- search coverage；
- watcher tests；
- performance；
- B 级功能状态；
- known limitations。

如果 B 级没有完成：

明确写：

```text
M3-A = PASS
M3-B = NOT COMPLETED
```

不得伪装。

---

# 92. 最终输出

最终输出：

```text
# M3 Final Audit

A. Baseline
B. Scanner
C. Metadata
D. Identity
E. Database
F. Search
G. Artwork
H. Incremental / Watcher
I. Security
J. Performance
K. Tests
L. M1 Regression
M. M2 Regression
N. Dependencies
O. Documentation
P. Scope

M3 = PASS / BLOCKED
COMMIT = <hash>
NEXT = M4
```

如果：

`M3 = PASS`

立即停止。

**不要开始 M4。**