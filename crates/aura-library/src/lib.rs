//! ifuyo Aura 音乐库（M3）：目录扫描（初始/增量/watcher）、标签读取（lofty）、
//! SQLite（WAL + FTS5，中文 CJK 二元组）、内容哈希身份、搜索与过滤。
//!
//! 职责边界（Charter §5/§61）：
//! `Filesystem → Scanner → Metadata → Normalization → Identity → SQLite → Search`
//! 不负责 audio decoding / playback / output / network / UI / Dreamscape。

pub mod artwork;
pub mod db;
pub mod error;
pub mod identity;
pub mod metadata;
pub mod normalize;
pub mod scanner;
pub mod search;
pub mod types;
