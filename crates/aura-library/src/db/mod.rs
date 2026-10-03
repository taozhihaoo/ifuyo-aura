//! SQLite 连接（§19/§20/§50）：WAL、参数化 SQL、事务安全。
//! 数据库路径由应用层决定（本 crate 不硬编码 %APPDATA%）。

pub mod migrations;
/// 低层仓库：事务内写路径。常规使用请走 service 层；
/// 集成测试与迁移工具可直用。
pub mod repo;
pub mod schema;

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::error::{LibraryError, Result};

/// 库数据库。`Connection` 本身 `!Sync`；并发由应用层组织
/// （推荐：单写连接 + 按需只读连接，WAL 下读不阻塞写）。
#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

impl Database {
    /// 打开（不存在则创建）并迁移到最新 schema。
    ///
    /// # Errors
    /// 打开失败（权限/损坏/锁定超时）或迁移失败；损坏语义见 §53。
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(LibraryError::from)?;
        Self::configure(&conn)?;
        let mut this = Self { conn };
        migrations::migrate(&mut this.conn)?;
        Ok(this)
    }

    /// 只读打开（校验存在性，防止误建空库）。
    ///
    /// # Errors
    /// 打开失败时返回 [`LibraryError::Io`]/[`LibraryError::Database`]。
    pub fn open_readonly(path: &Path) -> Result<Self> {
        if !path.is_file() {
            return Err(LibraryError::Io {
                path: Some(path.to_path_buf()),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "database not found"),
            });
        }
        let flags =
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags).map_err(LibraryError::from)?;
        Self::configure(&conn)?;
        Ok(Self { conn })
    }

    /// 内存库（测试用）。
    ///
    /// # Errors
    /// 同 [`Database::open`]。
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(LibraryError::from)?;
        Self::configure(&conn)?;
        let mut this = Self { conn };
        migrations::migrate(&mut this.conn)?;
        Ok(this)
    }

    fn configure(conn: &Connection) -> Result<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // WAL + NORMAL：崩溃安全（§50）与性能的常规平衡点。
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(Duration::from_secs(5))?;
        Ok(())
    }

    /// 当前 schema 版本。
    ///
    /// # Errors
    /// 查询失败时返回 [`LibraryError::Database`]。
    pub fn schema_version(&self) -> Result<i64> {
        migrations::current_version(&self.conn)
    }

    /// 再次执行迁移（已是最新则 no-op；§52 Re-run）。
    ///
    /// # Errors
    /// 同 [`Database::open`]。
    pub fn migrate(&mut self) -> Result<i64> {
        migrations::migrate(&mut self.conn)
    }

    /// 数据库文件路径（内存库为空）。
    #[must_use]
    pub fn path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(self.conn.path().unwrap_or_default())
    }

    /// 事务句柄。所有 track/artwork/playlist 写路径必须经此（§49）。
    ///
    /// # Errors
    /// 事务开启失败时返回 [`LibraryError::Database`]。
    pub fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }

    /// 底层连接。诊断/迁移工具用；常规查询请走 service 层
    /// （§40 的"不泄漏数据库内部"约束针对 service 查询 API）。
    #[must_use]
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
