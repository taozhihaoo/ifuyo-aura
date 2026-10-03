//! [B] 歌单与队列持久化模型（§58–§60）：手动歌单、智能歌单、
//! 播放队列、undo 命令日志。只建数据模型与事务性操作，不做 UI。
//!
//! - 手动歌单：position 主键稳定排序（§58）；重复 track 允许（不同位置）。
//! - 智能歌单：存 `FilterExpression` 序列化 + sort + limit（§59）；
//!   执行时求值，M3 无分析数据 → 分析字段不命中（§59 不伪造）。
//! - 队列：`queue_items`（position 唯一）+ `queue_log` 命令日志（§60 undo）。

use std::fmt;

use rusqlite::params;

use crate::error::Result;
use crate::search::filter::{parse_filter, FilterExpression};
use crate::service::LibraryService;
use crate::types::{PlaylistId, Track, TrackId};

/// 手动歌单元数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistInfo {
    pub id: PlaylistId,
    pub name: String,
    pub track_count: i64,
}

/// 智能歌单定义（§59）。
#[derive(Debug, Clone, PartialEq)]
pub struct SmartPlaylist {
    pub id: PlaylistId,
    pub name: String,
    pub query: FilterExpression,
    pub sort: Option<String>,
    pub limit: Option<i64>,
}

/// 队列项（§60）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub id: i64,
    pub position: i64,
    pub track_id: TrackId,
    /// JSON：来源上下文（playlist/album/search 等），M3 不解释内容。
    pub source_context: String,
}

/// 队列操作命令（§60 undo 模型）。
#[derive(Debug, Clone, PartialEq)]
pub enum QueueCommand {
    Append { track_id: TrackId },
    Insert { track_id: TrackId, position: i64 },
    Remove { position: i64 },
    Move { from: i64, to: i64 },
    Clear,
}

impl QueueCommand {
    fn op_name(&self) -> &'static str {
        match self {
            Self::Append { .. } => "append",
            Self::Insert { .. } => "insert",
            Self::Remove { .. } => "remove",
            Self::Move { .. } => "move",
            Self::Clear => "clear",
        }
    }

    fn payload(&self) -> String {
        match self {
            Self::Append { track_id } => format!(r#"{{"track_id":{}}}"#, track_id.0),
            Self::Insert { track_id, position } => {
                format!(r#"{{"track_id":{},"position":{}}}"#, track_id.0, position)
            }
            Self::Remove { position } => format!(r#"{{"position":{position}}}"#),
            Self::Move { from, to } => format!(r#"{{"position":{from},"to":{to}}}"#),
            Self::Clear => "{}".to_string(),
        }
    }
}

impl fmt::Display for QueueCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Move { from, to } => write!(f, "move {} {}", from, to),
            other => write!(f, "{} {}", other.op_name(), other.payload()),
        }
    }
}

impl LibraryService {
    // ===== 手动歌单（§58）=====

    /// 创建歌单。
    ///
    /// # Errors
    /// 重名/SQL 失败。
    pub fn create_playlist(&mut self, name: &str, now_ms: i64) -> Result<PlaylistId> {
        let tx = self.db_tx()?;
        tx.execute(
            "INSERT INTO playlists(name, date_created_ms, date_modified_ms) VALUES (?1, ?2, ?2)",
            params![name, now_ms],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(PlaylistId(id))
    }

    /// 删除歌单（级联成员；不影响 tracks）。
    ///
    /// # Errors
    /// SQL 失败。
    pub fn delete_playlist(&mut self, id: PlaylistId) -> Result<bool> {
        let tx = self.db_tx()?;
        let n = tx.execute("DELETE FROM playlists WHERE id = ?1", params![id.0])?;
        tx.commit()?;
        Ok(n > 0)
    }

    /// 追加 track 到歌单尾部（stable ordering，§58）。
    ///
    /// # Errors
    /// 歌单/track 不存在或 SQL 失败。
    pub fn playlist_add_track(
        &mut self,
        id: PlaylistId,
        track_id: TrackId,
        now_ms: i64,
    ) -> Result<()> {
        let tx = self.db_tx()?;
        tx.execute(
            "INSERT INTO playlist_tracks(playlist_id, position, track_id)
             VALUES (?1, (SELECT coalesce(max(position), -1) + 1 FROM playlist_tracks WHERE playlist_id = ?1), ?2)",
            params![id.0, track_id.0],
        )?;
        tx.execute(
            "UPDATE playlists SET date_modified_ms = ?1 WHERE id = ?2",
            params![now_ms, id.0],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// 移除歌单成员并压实位置（保持稳定顺序，无空洞）。
    ///
    /// # Errors
    /// SQL 失败。
    pub fn playlist_remove_at(
        &mut self,
        id: PlaylistId,
        position: i64,
        now_ms: i64,
    ) -> Result<bool> {
        let tx = self.db_tx()?;
        let n = tx.execute(
            "DELETE FROM playlist_tracks WHERE playlist_id = ?1 AND position = ?2",
            params![id.0, position],
        )?;
        if n > 0 {
            // 压实：删除位之后的 position 前移。
            tx.execute(
                "UPDATE playlist_tracks SET position = position - 1
                 WHERE playlist_id = ?1 AND position > ?2",
                params![id.0, position],
            )?;
            tx.execute(
                "UPDATE playlists SET date_modified_ms = ?1 WHERE id = ?2",
                params![now_ms, id.0],
            )?;
        }
        tx.commit()?;
        Ok(n > 0)
    }

    /// 歌单内容（stable ordering）。
    ///
    /// # Errors
    /// SQL 失败。
    pub fn playlist_tracks(&self, id: PlaylistId) -> Result<Vec<Track>> {
        let sql = format!(
            "SELECT {} FROM playlist_tracks pt
             JOIN tracks t ON t.id = pt.track_id
             LEFT JOIN artists ar ON ar.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             LEFT JOIN artists aar ON aar.id = t.album_artist_id
             LEFT JOIN genres g ON g.id = t.genre_id
             WHERE pt.playlist_id = ?1 AND t.is_active = 1
             ORDER BY pt.position",
            crate::query::TRACK_COLUMNS
        );
        let mut stmt = self.database().connection().prepare_cached(&sql)?;
        let rows = stmt.query_map(params![id.0], crate::query::map_track)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 歌单列表。
    ///
    /// # Errors
    /// SQL 失败。
    pub fn playlists(&self) -> Result<Vec<PlaylistInfo>> {
        let mut stmt = self.database().connection().prepare_cached(
            "SELECT p.id, p.name, count(pt.position)
             FROM playlists p
             LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id
             GROUP BY p.id ORDER BY p.name, p.id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(PlaylistInfo {
                id: PlaylistId(r.get(0)?),
                name: r.get(1)?,
                track_count: r.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // ===== 智能歌单（§59）=====

    /// 创建智能歌单（query 为 `FilterExpression` 序列化）。
    ///
    /// # Errors
    /// 解析失败/重名/SQL 失败。
    pub fn create_smart_playlist(
        &mut self,
        name: &str,
        query: &str,
        sort: Option<&str>,
        limit: Option<i64>,
        now_ms: i64,
    ) -> Result<PlaylistId> {
        let parsed = parse_filter(query).map_err(|e| crate::error::LibraryError::Metadata {
            path: std::path::PathBuf::from("<smart-playlist>"),
            reason: e.to_string(),
        })?;
        let tx = self.db_tx()?;
        tx.execute(
            "INSERT INTO playlists(name, date_created_ms, date_modified_ms) VALUES (?1, ?2, ?2)",
            params![name, now_ms],
        )?;
        let id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO smart_playlists(id, query, sort, limit_count) VALUES (?1, ?2, ?3, ?4)",
            params![id, parsed.to_string(), sort, limit],
        )?;
        tx.commit()?;
        Ok(PlaylistId(id))
    }

    /// 读取智能歌单定义。
    ///
    /// # Errors
    /// SQL 失败 / 存储的 query 无法解析（CorruptDatabase 语义）。
    pub fn smart_playlist(&self, id: PlaylistId) -> Result<Option<SmartPlaylist>> {
        let row = self.database().connection().query_row(
            "SELECT p.name, s.query, s.sort, s.limit_count
             FROM smart_playlists s JOIN playlists p ON p.id = s.id
             WHERE s.id = ?1",
            params![id.0],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                ))
            },
        );
        match row {
            Ok((name, query, sort, limit)) => {
                let parsed = parse_filter(&query).map_err(|_| {
                    crate::error::LibraryError::CorruptDatabase {
                        path: self.database().path(),
                        reason: format!("smart playlist {id:?} has invalid query"),
                    }
                })?;
                Ok(Some(SmartPlaylist {
                    id,
                    name,
                    query: parsed,
                    sort,
                    limit,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// 执行智能歌单（对当前库求值；§59 不伪造分析结果）。
    ///
    /// # Errors
    /// 同 [`LibraryService::filter_tracks`]。
    pub fn run_smart_playlist(
        &self,
        id: PlaylistId,
        page: crate::query::Page,
    ) -> Result<Vec<Track>> {
        match self.smart_playlist(id)? {
            Some(sp) => self.filter_tracks(&sp.query, page),
            None => Ok(Vec::new()),
        }
    }

    // ===== 队列（§60）=====

    /// 队列内容（position 序）。
    ///
    /// # Errors
    /// SQL 失败。
    pub fn queue_items(&self) -> Result<Vec<QueueItem>> {
        let mut stmt = self.database().connection().prepare_cached(
            "SELECT id, position, track_id, source_context FROM queue_items ORDER BY position",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(QueueItem {
                id: r.get(0)?,
                position: r.get(1)?,
                track_id: TrackId(r.get(2)?),
                source_context: r.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 执行队列命令（事务 + 命令日志，§60 undo）。
    ///
    /// # Errors
    /// SQL 失败。
    pub fn queue_apply(&mut self, command: QueueCommand, now_ms: i64) -> Result<()> {
        let tx = self.db_tx()?;
        // Remove 在删除前查 track_id，写入扩展 payload 供 undo 重插。
        let removed_track_id: Option<i64> = match &command {
            QueueCommand::Remove { position } => tx
                .query_row(
                    "SELECT track_id FROM queue_items WHERE position = ?1",
                    params![position],
                    |r| r.get(0),
                )
                .ok(),
            _ => None,
        };
        let log = |tx: &rusqlite::Transaction<'_>, cmd: &QueueCommand| -> Result<()> {
            let mut payload = cmd.payload();
            if let (QueueCommand::Remove { .. }, Some(tid)) = (&command, removed_track_id) {
                payload = format!(
                    r#"{{"position":{},"track_id":{tid}}}"#,
                    match cmd {
                        QueueCommand::Remove { position } => *position,
                        _ => 0,
                    }
                );
            }
            tx.execute(
                "INSERT INTO queue_log(ts_ms, op, payload) VALUES (?1, ?2, ?3)",
                params![now_ms, cmd.op_name(), payload],
            )?;
            Ok(())
        };
        match &command {
            QueueCommand::Append { track_id } => {
                tx.execute(
                    "INSERT INTO queue_items(position, track_id, source_context)
                     VALUES ((SELECT coalesce(max(position), -1) + 1 FROM queue_items), ?1, '{}')",
                    params![track_id.0],
                )?;
            }
            QueueCommand::Insert { track_id, position } => {
                shift_tail(&tx, *position, 1)?;
                tx.execute(
                    "INSERT INTO queue_items(position, track_id, source_context) VALUES (?1, ?2, '{}')",
                    params![position, track_id.0],
                )?;
            }
            QueueCommand::Remove { position } => {
                tx.execute(
                    "DELETE FROM queue_items WHERE position = ?1",
                    params![position],
                )?;
                shift_head(&tx, *position, -1)?;
            }
            QueueCommand::Move { from, to } => {
                queue_move(&tx, *from, *to)?;
            }
            QueueCommand::Clear => {
                tx.execute("DELETE FROM queue_items", [])?;
            }
        }
        log(&tx, &command)?;
        tx.commit()?;
        Ok(())
    }

    /// undo：按命令日志逆放最近一条（§60）。返回被撤销的命令。
    ///
    /// # Errors
    /// 空日志 / SQL 失败。
    pub fn queue_undo(&mut self, now_ms: i64) -> Result<Option<QueueCommand>> {
        let Some((op, payload, log_id)) = self.last_queue_log()? else {
            return Ok(None);
        };
        let cmd = parse_queue_log(&op, &payload)?;
        let tx = self.db_tx()?;
        match &cmd {
            QueueCommand::Append { .. } => {
                // 逆：删尾。
                tx.execute(
                    "DELETE FROM queue_items WHERE position = (SELECT max(position) FROM queue_items)",
                    [],
                )?;
            }
            QueueCommand::Insert { position, .. } => {
                tx.execute(
                    "DELETE FROM queue_items WHERE position = ?1",
                    params![position],
                )?;
                shift_head(&tx, *position, -1)?;
            }
            QueueCommand::Remove { position } => {
                shift_tail(&tx, *position, 1)?;
                // apply 时已把被删 track_id 写进日志 payload（见下）。
                let track_id = num_field(&payload, "track_id").unwrap_or(0);
                tx.execute(
                    "INSERT INTO queue_items(position, track_id, source_context) VALUES (?1, ?2, '{}')",
                    params![position, track_id],
                )?;
            }
            QueueCommand::Move { from, to } => {
                queue_move(&tx, *to, *from)?;
            }
            QueueCommand::Clear => {
                // Clear 的 undo 需要清空前快照 —— 不在 M3 命令日志粒度内，
                // 记录为不可撤销并跳过（见 D-047）。
                tx.execute("DELETE FROM queue_log WHERE id = ?1", params![log_id])?;
                tx.commit()?;
                return Ok(Some(QueueCommand::Clear));
            }
        }
        tx.execute("DELETE FROM queue_log WHERE id = ?1", params![log_id])?;
        let _ = now_ms;
        tx.commit()?;
        Ok(Some(cmd))
    }

    fn last_queue_log(&self) -> Result<Option<(String, String, i64)>> {
        let row = self.database().connection().query_row(
            "SELECT id, op, payload FROM queue_log ORDER BY id DESC LIMIT 1",
            [],
            |r| {
                Ok((
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(0)?,
                ))
            },
        );
        match row {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// position 区间内移动（compact list 语义）。
fn queue_move(tx: &rusqlite::Transaction<'_>, from: i64, to: i64) -> Result<()> {
    if from == to {
        return Ok(());
    }
    // 取出元素，压中间区间，插入目标位。
    let track_id: Option<i64> = tx
        .query_row(
            "SELECT track_id FROM queue_items WHERE position = ?1",
            params![from],
            |r| r.get(0),
        )
        .ok();
    let Some(track_id) = track_id else {
        return Ok(()); // 无该位：no-op（不 panic）
    };
    tx.execute("DELETE FROM queue_items WHERE position = ?1", params![from])?;
    if from < to {
        shift_range(tx, from + 1, to, -1)?;
    } else {
        shift_range(tx, to, from - 1, 1)?;
    }
    tx.execute(
        "INSERT INTO queue_items(position, track_id, source_context) VALUES (?1, ?2, '{}')
         ON CONFLICT(position) DO UPDATE SET track_id = excluded.track_id",
        params![to, track_id],
    )?;
    Ok(())
}

/// 受限 JSON 数字字段窄解析（payload 由本 crate 写入；不引 serde）。
fn num_field(payload: &str, key: &str) -> Option<i64> {
    let marker = format!(r#""{key}":"#);
    let start = payload.find(&marker)? + marker.len();
    let rest = &payload[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// 解析命令日志行。
fn parse_queue_log(op: &str, payload: &str) -> Result<QueueCommand> {
    let cmd = match op {
        "append" => QueueCommand::Append {
            track_id: TrackId(num_field(payload, "track_id").unwrap_or(0)),
        },
        "insert" => QueueCommand::Insert {
            track_id: TrackId(num_field(payload, "track_id").unwrap_or(0)),
            position: num_field(payload, "position").unwrap_or(0),
        },
        "remove" => QueueCommand::Remove {
            position: num_field(payload, "position").unwrap_or(0),
        },
        "move" => QueueCommand::Move {
            from: num_field(payload, "position").unwrap_or(0),
            to: num_field(payload, "to").unwrap_or(0),
        },
        "clear" => QueueCommand::Clear,
        _ => {
            return Err(crate::error::LibraryError::CorruptDatabase {
                path: std::path::PathBuf::from("<queue_log>"),
                reason: format!("unknown queue op {op:?}"),
            })
        }
    };
    Ok(cmd)
}

/// 大偏移：把待移位行整体搬到负数区（与任何合法 position 不相交），
/// 再整体移回 —— 规避 UNIQUE(position) 的逐行更新中间态冲突。
const SHIFT_OFFSET: i64 = 1 << 40;

fn apply_shift(
    tx: &rusqlite::Transaction<'_>,
    where_clause: &str,
    start: i64,
    hi: Option<i64>,
    delta: i64,
) -> Result<()> {
    let _ = where_clause; // where 语义由 start/hi 表达
    let sql1 = if hi.is_some() {
        "UPDATE queue_items SET position = position - SHIFT_OFFSET WHERE position BETWEEN ?1 AND ?2"
            .replace("SHIFT_OFFSET", &SHIFT_OFFSET.to_string())
    } else {
        "UPDATE queue_items SET position = position - SHIFT_OFFSET WHERE position >= ?1"
            .replace("SHIFT_OFFSET", &SHIFT_OFFSET.to_string())
    };
    let sql2 = format!(
        "UPDATE queue_items SET position = position + {SHIFT_OFFSET} + {delta} WHERE position < 0"
    );
    match hi {
        Some(hi) => tx.execute(&sql1, params![start, hi])?,
        None => tx.execute(&sql1, params![start])?,
    };
    tx.execute(&sql2, [])?;
    Ok(())
}

/// position >= start 的连续段整体位移 `delta`。
fn shift_head(tx: &rusqlite::Transaction<'_>, start: i64, delta: i64) -> Result<()> {
    apply_shift(tx, "position >= ?", start, None, delta)
}

/// position >= start 整体 +delta。
fn shift_tail(tx: &rusqlite::Transaction<'_>, start: i64, delta: i64) -> Result<()> {
    shift_head(tx, start, delta)
}

/// 闭区间 [lo, hi] 内整体位移 `delta`。
fn shift_range(tx: &rusqlite::Transaction<'_>, lo: i64, hi: i64, delta: i64) -> Result<()> {
    apply_shift(tx, "position BETWEEN", lo, Some(hi), delta)
}
