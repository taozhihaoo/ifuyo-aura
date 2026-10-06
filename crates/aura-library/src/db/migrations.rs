//! 迁移系统（§51/§52/§83）：`PRAGMA user_version` 记录版本；
//! 每版一条确定性 DDL 脚本，单事务执行，失败整体回滚。
//! 升级路径只追加：新版本 push 到 `MIGRATIONS` 尾部，禁止改历史脚本。

use rusqlite::Connection;

use crate::error::{LibraryError, Result};

/// 迁移脚本表：`MIGRATIONS[n]` 把数据库从版本 `n` 升到 `n+1`。
pub const MIGRATIONS: &[&str] = &[
    super::schema::V1,
    super::schema::V2,
    // V3（M15a）：CUE 整轨虚拟子轨——tracks 增子轨偏移/时长
    "ALTER TABLE tracks ADD COLUMN cue_start_seconds REAL;
     ALTER TABLE tracks ADD COLUMN cue_duration_seconds REAL;",
];

/// 最新 schema 版本。
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// 当前版本（未迁移的新库为 0）。
///
/// # Errors
/// 查询失败时返回 [`LibraryError::Database`]。
pub fn current_version(conn: &Connection) -> Result<i64> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(Into::into)
}

/// 把数据库迁移到最新版本。空库从 0 开始；已是最新则 no-op。
/// 单事务：任何一步失败整体回滚（§52 Failure）。
///
/// # Errors
/// 脚本执行或版本写入失败时返回 [`LibraryError::Database`]。
pub fn migrate(conn: &mut Connection) -> Result<i64> {
    let from = current_version(conn)?;
    if from > SCHEMA_VERSION {
        return Err(LibraryError::CorruptDatabase {
            path: db_path(conn),
            reason: format!(
                "database schema version {from} is newer than supported {SCHEMA_VERSION}"
            ),
        });
    }
    if from == SCHEMA_VERSION {
        return Ok(from);
    }
    let tx = conn.transaction()?;
    for v in from..SCHEMA_VERSION {
        let script = MIGRATIONS[usize::try_from(v).expect("version fits usize")];
        tx.execute_batch(script)?;
    }
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(SCHEMA_VERSION)
}

/// 迁移失败的路径归属（用于 CorruptDatabase 报告）。
fn db_path(conn: &Connection) -> std::path::PathBuf {
    std::path::PathBuf::from(conn.path().unwrap_or_default())
}
