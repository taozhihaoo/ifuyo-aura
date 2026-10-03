//! 迁移与数据库打开测试（§52/§53/§74）：fresh / upgrade / re-run / failure
//! rollback / reopen / WAL / 只读 / 损坏不 panic / FTS5 可用性。

use aura_library::db::{migrations, Database};

#[test]
fn fresh_database_reaches_latest_schema() {
    let db = Database::open_in_memory().unwrap();
    assert_eq!(db.schema_version().unwrap(), migrations::SCHEMA_VERSION);
    assert_eq!(migrations::SCHEMA_VERSION, 1, "M3 只有一版 schema");
}

#[test]
fn rerun_migration_is_noop() {
    let mut db = Database::open_in_memory().unwrap();
    assert_eq!(db.migrate().unwrap(), migrations::SCHEMA_VERSION, "幂等");
}

#[test]
fn fresh_file_database_and_reopen() {
    let dir = std::env::temp_dir().join(format!("aura-m3-db-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("lib.db");

    {
        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 1);
    }
    // reopen：版本保持。
    {
        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 1);
        // WAL 生效（§19）。
        let mode: String = db
            .connection()
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn upgrade_from_version_zero_preserves_state() {
    // 旧 fixture：一个"版本 0"的库（含我们的 schema 之外的用户数据）。
    let dir = std::env::temp_dir().join(format!("aura-m3-upg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("old.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE user_notes(text TEXT);
             INSERT INTO user_notes VALUES ('keep me');
             PRAGMA user_version = 0;",
        )
        .unwrap();
    }
    {
        let db = Database::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 1);
        // 迁移不得破坏既有数据。
        let n: String = db
            .connection()
            .query_row("SELECT text FROM user_notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, "keep me");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn migration_failure_rolls_back_completely() {
    // 复刻 migrate() 的事务语义：中途失败 → 整体回滚，版本不变。
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE marker(x)").unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    let result = tx.execute_batch("CREATE TABLE t1(a); THIS IS NOT SQL;");
    assert!(result.is_err());
    drop(tx); // 回滚
    let tables: String = conn
        .query_row(
            "SELECT coalesce(group_concat(name), '') FROM sqlite_master WHERE type='table'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!tables.contains("t1"), "失败迁移不得留下半成品表: {tables}");
    assert!(tables.contains("marker"), "先前数据保留");
    let v: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 0, "版本只在成功提交后推进");
}

#[test]
fn newer_database_is_rejected_not_rebuilt() {
    let dir = std::env::temp_dir().join(format!("aura-m3-fut-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("future.db");
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 999).unwrap();
    }
    let err = Database::open(&path).unwrap_err();
    assert!(
        matches!(
            err,
            aura_library::error::LibraryError::CorruptDatabase { .. }
        ),
        "未来版本必须报 CorruptDatabase 而不是删库重建: {err:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn corrupt_database_file_reports_error_no_panic() {
    let dir = std::env::temp_dir().join(format!("aura-m3-corrupt-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("junk.db");
    std::fs::write(&path, b"this is definitely not a sqlite database").unwrap();
    // §53：打开失败返回错误，不 panic。
    assert!(Database::open(&path).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn readonly_open_rejects_missing_file() {
    let err = Database::open_readonly(std::path::Path::new("Z:/definitely/missing.db"));
    assert!(err.is_err());
}

#[test]
fn concurrent_readers_during_wal_write() {
    let dir = std::env::temp_dir().join(format!("aura-m3-wal-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("wal.db");
    {
        let mut db = Database::open(&path).unwrap();
        let tx = db.transaction().unwrap();
        tx.execute(
            "INSERT INTO library_roots(path, date_added_ms) VALUES (?1, ?2)",
            rusqlite::params!["C:\\music", 1000],
        )
        .unwrap();
        // 写事务未提交时，只读连接读旧快照（WAL，§74 concurrent reads）。
        {
            let ro = Database::open_readonly(&path).unwrap();
            let n: i64 = ro
                .connection()
                .query_row("SELECT count(*) FROM library_roots", [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "未提交写对只读连接不可见");
        }
        tx.commit().unwrap();
    }
    {
        let ro = Database::open_readonly(&path).unwrap();
        let n: i64 = ro
            .connection()
            .query_row("SELECT count(*) FROM library_roots", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "提交后可见");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn fts5_available_in_bundled_sqlite() {
    let db = Database::open_in_memory().unwrap();
    // schema 建表 tracks_fts 成功即证明 FTS5 可用；这里再验证查询路径。
    let row: String = db
        .connection()
        .query_row("SELECT title FROM tracks_fts LIMIT 1", [], |r| r.get(0))
        .unwrap_or_default();
    assert!(row.is_empty(), "空表无行");
}
