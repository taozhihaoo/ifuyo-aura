//! FTS5 索引维护与检索（§32/§36/§37/§65）。
//!
//! 索引行在 track 写事务内同步（§49）；查询全部参数化，用户输入经
//! tokenizer 加引号失效化（§65）。

pub mod tokenizer;

use rusqlite::{params, Connection, Transaction};

use crate::error::Result;
use crate::types::TrackId;

/// FTS 行内容（display 值经 tokenizer 规范化后入库）。
#[derive(Debug, Clone, Default)]
pub struct FtsDoc<'a> {
    pub title: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub album_artist: Option<&'a str>,
    pub genre: Option<&'a str>,
    pub path: &'a str,
}

/// 删除 track 的 FTS 行（若记录了 rowid）。
pub fn delete_track(conn: &Connection, fts_rowid: Option<i64>) -> Result<()> {
    if let Some(rowid) = fts_rowid {
        conn.execute("DELETE FROM tracks_fts WHERE rowid = ?1", params![rowid])?;
    }
    Ok(())
}

/// 插入/替换 track 的 FTS 行，返回新 rowid（调用方写回 `tracks.fts_rowid`）。
pub fn insert_track(tx: &Transaction, track_id: TrackId, doc: &FtsDoc<'_>) -> Result<i64> {
    let cell = |s: Option<&str>| tokenizer::index_text(s.unwrap_or_default());
    tx.execute(
        "INSERT INTO tracks_fts(track_id, title, artist, album, album_artist, genre, path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            track_id.0,
            cell(doc.title),
            cell(doc.artist),
            cell(doc.album),
            cell(doc.album_artist),
            cell(doc.genre),
            tokenizer::index_text(doc.path),
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

/// 搜索结果项。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchHit {
    pub track_id: TrackId,
    /// FTS5 rank（越负越相关；仅用于排序，不对外承诺数值语义）。
    pub rank: f64,
}

/// 全文搜索（§32：title/artist/album/album_artist/genre/path）。
/// 排序：bm25 加权（title > artist > album = album_artist > genre > path）
/// + track_id 决胜（§36/§37 确定性）。只返回活跃 track。
///
/// 空查询/无有效 token → 空结果（不退化成全表）。
pub fn search(conn: &Connection, query: &str, limit: i64, offset: i64) -> Result<Vec<SearchHit>> {
    let Some(match_expr) = tokenizer::build_match_query(query) else {
        return Ok(Vec::new());
    };
    let mut stmt = conn.prepare_cached(
        "SELECT tracks_fts.track_id, bm25(tracks_fts, 0.0, 8.0, 6.0, 4.0, 4.0, 2.0, 1.0)
         FROM tracks_fts
         JOIN tracks ON tracks.id = tracks_fts.track_id AND tracks.is_active = 1
         WHERE tracks_fts MATCH ?1
         ORDER BY bm25(tracks_fts, 0.0, 8.0, 6.0, 4.0, 4.0, 2.0, 1.0), tracks_fts.track_id
         LIMIT ?2 OFFSET ?3",
    )?;
    let hits = stmt
        .query_map(params![match_expr, limit, offset], |r| {
            Ok(SearchHit {
                track_id: TrackId(r.get(0)?),
                rank: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(hits)
}
