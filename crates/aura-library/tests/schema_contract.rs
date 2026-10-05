//! Schema 契约（M12.5 加固）：shell 层统计 SQL 依赖 tracks 列形态——
//! 规范化迁移后 tracks 持 artist_id/album_id 外键，**无** artist/album 文本列。
//! `library_stats` 曾查 `COUNT(DISTINCT artist)` 被 unwrap_or(0) 吞成恒 0
//! （M12.5 L2 矩阵实测抓出）。本测试锁死列形态，未来漂移在此报警。

use aura_library::db::Database;

#[test]
fn tracks_columns_match_shell_stats_sql_contract() {
    let db = Database::open_in_memory().expect("in-memory db");
    let conn = db.connection();
    let cols: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT name FROM pragma_table_info('tracks')")
            .expect("pragma");
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).expect("rows");
        rows.filter_map(Result::ok).collect()
    };
    for required in ["artist_id", "album_id", "is_active"] {
        assert!(
            cols.iter().any(|c| c == required),
            "tracks 缺少 shell 层依赖的列 {required}（library_stats/join SQL 会静默变 0/错）"
        );
    }
    for banned in ["artist", "album"] {
        assert!(
            !cols.iter().any(|c| *c == banned),
            "tracks 不应再有文本列 {banned}（规范化迁移后由 artists/albums 表外键承担）"
        );
    }
}

#[test]
fn stats_distinct_foreign_keys_count_non_null_only() {
    let db = Database::open_in_memory().expect("in-memory db");
    let conn = db.connection();
    // COUNT(DISTINCT artist_id) 对 NULL 不计数：无标签曲目的 artist_id 为 NULL。
    let artists: i64 = conn
        .query_row("SELECT COUNT(DISTINCT artist_id) FROM tracks", [], |r| {
            r.get(0)
        })
        .expect("stats sql runs");
    let albums: i64 = conn
        .query_row("SELECT COUNT(DISTINCT album_id) FROM tracks", [], |r| {
            r.get(0)
        })
        .expect("stats sql runs");
    assert_eq!((artists, albums), (0, 0), "空库统计为 0 且 SQL 可执行");
}
