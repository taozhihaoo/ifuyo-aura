//! ifuyo Aura 网络缓存（M5）：256 KiB 分块、原子改名、SQLite 元数据（WAL）、
//! Range 读取、按需拉取、prefetch、LRU、pin、故障自愈。
//!
//! 策略硬门禁（§3–§7）：只有 `Allowed` 来源允许持久缓存；`Forbidden`
//! 来源从代码结构上保证所有执行路径零写盘；`SessionOnly` 仅进程内内存。
//!
//! 边界（§2/§91）：cache 不知道 WASAPI/CPAL/EQ/UI；持久化写入全部经过
//! 统一策略门（`CacheWriteGuard` 语义）；数据库与 M3 library 完全隔离（§15/§28）。

pub mod db;
pub mod error;
pub mod keys;
pub mod manager;
pub mod store;
