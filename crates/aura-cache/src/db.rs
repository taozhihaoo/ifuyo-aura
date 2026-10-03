//! SQLite cache 索引（§15–§18/§63/§64）：独立于 M3 library DB（§28），
//! WAL，事务化写路径，时间戳全部注入（§105 确定性测试）。

use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection};

use crate::error::{CacheError, Result};

/// schema 版本（§83 equivalent：迁移式推进，只追加）。
pub const SCHEMA_VERSION: i64 = 1;

/// 单条 cache entry（§17）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntryRow {
    pub content_key: String,
    pub source_kind: String,
    pub url_hash: String,
    pub redacted_url: String,
    pub content_length: Option<u64>,
    pub mime: Option<String>,
    pub codec: Option<String>,
    pub seekable: bool,
    pub complete: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub policy: String,
    pub created_at_ms: i64,
    pub last_accessed_ms: i64,
    pub total_bytes: i64,
}

/// chunk 索引行（§18；stored_size 与 size 恒等 —— 原子写保证，D-057）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheChunkRow {
    pub content_key: String,
    pub chunk_index: u64,
    pub size: u64,
    pub sha256: String,
    pub created_at_ms: i64,
    pub last_accessed_ms: i64,
}

/// cache 索引 DB。
pub struct CacheDb {
    conn: Connection,
}

impl CacheDb {
    /// 打开/创建 + WAL + busy timeout + 迁移（§26/§63/§127）。
    ///
    /// # Errors
    /// 打不开/迁移失败 → [`CacheError::Database`]（调用方走恢复策略，§27）。
    pub fn open(path: &Path) -> Result<Self> {
        // cache root 必须先存在（DB + chunks/tmp 同根；§26 自动创建）。
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(CacheError::from)?;
            }
        }
        let conn = Connection::open(path).map_err(CacheError::from)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(Duration::from_secs(5))?;
        let this = Self { conn };
        this.migrate()?;
        Ok(this)
    }

    /// 内存库（测试）。
    ///
    /// # Errors
    /// 同 [`CacheDb::open`]。
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(CacheError::from)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let this = Self { conn };
        this.migrate()?;
        Ok(this)
    }

    fn migrate(&self) -> Result<()> {
        let v: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(CacheError::from)?;
        if v > SCHEMA_VERSION {
            return Err(CacheError::Corrupt {
                what: format!("cache db version {v} newer than supported"),
            });
        }
        if v == SCHEMA_VERSION {
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction()?;
        if v < 1 {
            tx.execute_batch(
                "CREATE TABLE cache_entries (
                    content_key    TEXT PRIMARY KEY,
                    source_kind    TEXT NOT NULL,
                    url_hash       TEXT NOT NULL,
                    redacted_url   TEXT NOT NULL,
                    content_length INTEGER,
                    mime           TEXT,
                    codec          TEXT,
                    seekable       INTEGER NOT NULL DEFAULT 0,
                    complete       INTEGER NOT NULL DEFAULT 0,
                    etag           TEXT,
                    last_modified  TEXT,
                    policy         TEXT NOT NULL,
                    created_at_ms     INTEGER NOT NULL,
                    last_accessed_ms  INTEGER NOT NULL,
                    total_bytes       INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE cache_chunks (
                    content_key   TEXT NOT NULL REFERENCES cache_entries(content_key) ON DELETE CASCADE,
                    chunk_index   INTEGER NOT NULL,
                    size          INTEGER NOT NULL,
                    sha256        TEXT NOT NULL,
                    created_at_ms    INTEGER NOT NULL,
                    last_accessed_ms INTEGER NOT NULL,
                    PRIMARY KEY (content_key, chunk_index)
                );
                CREATE INDEX idx_chunks_access ON cache_chunks(last_accessed_ms);
                CREATE INDEX idx_entries_access ON cache_entries(last_accessed_ms);
                CREATE TABLE cache_pins (
                    content_key  TEXT PRIMARY KEY,
                    pinned_at_ms INTEGER NOT NULL
                );",
            )?;
        }
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        tx.commit()?;
        Ok(())
    }

    /// 插入/更新 entry（§64 事务化由调用方包多操作时使用 tx 版本）。
    pub fn upsert_entry(&self, e: &CacheEntryRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO cache_entries(
                content_key, source_kind, url_hash, redacted_url, content_length,
                mime, codec, seekable, complete, etag, last_modified, policy,
                created_at_ms, last_accessed_ms, total_bytes
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
             ON CONFLICT(content_key) DO UPDATE SET
                source_kind=excluded.source_kind, url_hash=excluded.url_hash,
                redacted_url=excluded.redacted_url, content_length=excluded.content_length,
                mime=excluded.mime, codec=excluded.codec, seekable=excluded.seekable,
                complete=excluded.complete, etag=excluded.etag,
                last_modified=excluded.last_modified, policy=excluded.policy,
                last_accessed_ms=excluded.last_accessed_ms, total_bytes=excluded.total_bytes",
            params![
                e.content_key,
                e.source_kind,
                e.url_hash,
                e.redacted_url,
                e.content_length.map(|v| v as i64),
                e.mime,
                e.codec,
                i64::from(e.seekable),
                i64::from(e.complete),
                e.etag,
                e.last_modified,
                e.policy,
                e.created_at_ms,
                e.last_accessed_ms,
                e.total_bytes,
            ],
        )?;
        Ok(())
    }

    pub fn get_entry(&self, content_key: &str) -> Result<Option<CacheEntryRow>> {
        let row = self.conn.query_row(
            "SELECT content_key, source_kind, url_hash, redacted_url, content_length,
                    mime, codec, seekable, complete, etag, last_modified, policy,
                    created_at_ms, last_accessed_ms, total_bytes
             FROM cache_entries WHERE content_key = ?1",
            params![content_key],
            map_entry,
        );
        match row {
            Ok(e) => Ok(Some(e)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 记录 chunk 元数据（文件已成功写入之后，§20 顺序）。
    pub fn insert_chunk(&self, c: &CacheChunkRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO cache_chunks(content_key, chunk_index, size, sha256, created_at_ms, last_accessed_ms)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(content_key, chunk_index) DO UPDATE SET
                size=excluded.size, sha256=excluded.sha256, last_accessed_ms=excluded.last_accessed_ms",
            params![
                c.content_key,
                c.chunk_index as i64,
                c.size as i64,
                c.sha256,
                c.created_at_ms,
                c.last_accessed_ms,
            ],
        )?;
        // entry 聚合字段落库内更新（与 chunk 同连接，非跨进程事务场景足够；
        // 多步事务由 `transaction()` 包裹）。
        self.conn.execute(
            "UPDATE cache_entries SET total_bytes = (
                SELECT coalesce(sum(size), 0) FROM cache_chunks WHERE content_key = ?1
             ) WHERE content_key = ?1",
            params![c.content_key],
        )?;
        Ok(())
    }

    pub fn get_chunk(&self, content_key: &str, chunk_index: u64) -> Result<Option<CacheChunkRow>> {
        let row = self.conn.query_row(
            "SELECT content_key, chunk_index, size, sha256, created_at_ms, last_accessed_ms
             FROM cache_chunks WHERE content_key = ?1 AND chunk_index = ?2",
            params![content_key, chunk_index as i64],
            map_chunk,
        );
        match row {
            Ok(c) => Ok(Some(c)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_chunks(&self, content_key: &str) -> Result<Vec<CacheChunkRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_key, chunk_index, size, sha256, created_at_ms, last_accessed_ms
             FROM cache_chunks WHERE content_key = ?1 ORDER BY chunk_index",
        )?;
        let rows = stmt.query_map(params![content_key], map_chunk)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// 删除 chunk 元数据（淘汰/修复路径）。
    pub fn delete_chunk(&self, content_key: &str, chunk_index: u64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM cache_chunks WHERE content_key = ?1 AND chunk_index = ?2",
            params![content_key, chunk_index as i64],
        )?;
        self.conn.execute(
            "UPDATE cache_entries SET total_bytes = (
                SELECT coalesce(sum(size), 0) FROM cache_chunks WHERE content_key = ?1
             ) WHERE content_key = ?1",
            params![content_key],
        )?;
        Ok(())
    }

    /// 删除 entry（CASCADE chunks；pins 保留记录由调用方决定）。
    pub fn delete_entry(&self, content_key: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM cache_entries WHERE content_key = ?1",
            params![content_key],
        )?;
        Ok(())
    }

    /// LRU 候选：按 last_accessed 升序（§57；平局 content_key+chunk_index 决胜 §108）。
    pub fn lru_chunks(&self, limit: i64) -> Result<Vec<CacheChunkRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.content_key, c.chunk_index, c.size, c.sha256, c.created_at_ms, c.last_accessed_ms
             FROM cache_chunks c
             LEFT JOIN cache_pins p ON p.content_key = c.content_key
             WHERE p.content_key IS NULL
             ORDER BY c.last_accessed_ms ASC, c.content_key ASC, c.chunk_index ASC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], map_chunk)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn pin(&self, content_key: &str, now_ms: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO cache_pins(content_key, pinned_at_ms) VALUES (?1, ?2)
             ON CONFLICT(content_key) DO UPDATE SET pinned_at_ms = excluded.pinned_at_ms",
            params![content_key, now_ms],
        )?;
        Ok(())
    }

    pub fn unpin(&self, content_key: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM cache_pins WHERE content_key = ?1",
            params![content_key],
        )?;
        Ok(())
    }

    pub fn is_pinned(&self, content_key: &str) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT count(*) FROM cache_pins WHERE content_key = ?1",
            params![content_key],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// §56：容量只统计有效 chunk bytes。
    pub fn total_chunk_bytes(&self) -> Result<u64> {
        let n: i64 =
            self.conn
                .query_row("SELECT coalesce(sum(size), 0) FROM cache_chunks", [], |r| {
                    r.get(0)
                })?;
        Ok(n.max(0) as u64)
    }

    pub fn entry_count(&self) -> Result<u64> {
        let n: i64 = self
            .conn
            .query_row("SELECT count(*) FROM cache_entries", [], |r| r.get(0))?;
        Ok(n.max(0) as u64)
    }

    pub fn chunk_count(&self) -> Result<u64> {
        let n: i64 = self
            .conn
            .query_row("SELECT count(*) FROM cache_chunks", [], |r| r.get(0))?;
        Ok(n.max(0) as u64)
    }

    /// §62：批量 access-time 刷新（内存聚合后的落库入口）。
    pub fn touch_chunks(&self, touches: &[(String, u64, i64)]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for (ck, idx, ts) in touches {
            tx.execute(
                "UPDATE cache_chunks SET last_accessed_ms = ?3 WHERE content_key = ?1 AND chunk_index = ?2",
                params![ck, *idx as i64, *ts],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// 显式事务入口（§64：entry 创建 + chunk 记录组合场景）。
    pub fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }

    /// DB 完整性快照（诊断）。
    pub fn quick_check(&self) -> Result<String> {
        self
            .conn
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(CacheError::from)
    }
}

fn map_entry(r: &rusqlite::Row<'_>) -> rusqlite::Result<CacheEntryRow> {
    Ok(CacheEntryRow {
        content_key: r.get(0)?,
        source_kind: r.get(1)?,
        url_hash: r.get(2)?,
        redacted_url: r.get(3)?,
        content_length: r.get::<_, Option<i64>>(4)?.map(|v| v.max(0) as u64),
        mime: r.get(5)?,
        codec: r.get(6)?,
        seekable: r.get::<_, i64>(7)? != 0,
        complete: r.get::<_, i64>(8)? != 0,
        etag: r.get(9)?,
        last_modified: r.get(10)?,
        policy: r.get(11)?,
        created_at_ms: r.get(12)?,
        last_accessed_ms: r.get(13)?,
        total_bytes: r.get(14)?,
    })
}

fn map_chunk(r: &rusqlite::Row<'_>) -> rusqlite::Result<CacheChunkRow> {
    Ok(CacheChunkRow {
        content_key: r.get(0)?,
        chunk_index: r.get::<_, i64>(1)?.max(0) as u64,
        size: r.get::<_, i64>(2)?.max(0) as u64,
        sha256: r.get(3)?,
        created_at_ms: r.get(4)?,
        last_accessed_ms: r.get(5)?,
    })
}
