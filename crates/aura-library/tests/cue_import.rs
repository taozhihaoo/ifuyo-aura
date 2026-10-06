//! M15a：CUE 整轨导入集成测试——import_file 展开虚拟子轨。
use std::path::Path;

use aura_library::db::Database;

#[test]
fn cue_file_imports_as_virtual_subtracks() {
    let mut db = Database::open_in_memory().expect("db");
    {
        let conn = db.connection();
        // 根目录登记（import_file 需要 root 存在）
        conn.execute(
            "INSERT INTO library_roots(id, path, date_added_ms) VALUES (1, 'C:/music', 1000)",
            [],
        )
        .unwrap();
    }
    // 测试 CWD = crate 目录 → 仓库根的 fixtures 用绝对路径
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let src = root.join("tests").join("fixtures").join("fixture-cue.flac");
    let outcomes = {
        let tx = db.transaction().expect("tx");
        let outcomes =
            aura_library::db::repo::import_file(&tx, aura_library::types::RootId(1), &src, 1000)
                .expect("import");
        tx.commit().unwrap();
        outcomes
    };
    assert!(
        outcomes.len() >= 6,
        "应展开 ≥6 条导入（主轨 + 子轨），实际 {}",
        outcomes.len()
    );

    let conn = db.connection();
    let active: i64 = conn
        .query_row("SELECT count(*) FROM tracks WHERE is_active = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(active, 6, "激活行 = 6 条虚拟子轨（整轨主行隐藏）");
    let total: i64 = conn
        .query_row("SELECT count(*) FROM tracks", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 7, "总行 = 主轨（隐藏）+ 6 子轨");
    let cued: i64 = conn
        .query_row(
            "SELECT count(*) FROM tracks WHERE cue_start_seconds IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(cued, 6, "全部子轨带 cue 偏移");
    let titles: i64 = conn
        .query_row(
            "SELECT count(*) FROM tracks WHERE title = '轨道三'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(titles, 1, "轨道三入库（CUE 中文标题）");
}
