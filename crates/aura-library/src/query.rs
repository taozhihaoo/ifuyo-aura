//! 查询 API（§40–§42）与一致性诊断（§76/§77）。
//!
//! 返回类型为领域模型（`Track` 等），不泄漏 SQL 细节（§40）。
//! 分页：`limit/offset`（D-044）；排序恒定 + `id` 决胜（§37/§42）。

use rusqlite::params;

use crate::error::Result;
use crate::search;
use crate::service::LibraryService;
use crate::types::{ArtworkId, PlaybackStats, Track, TrackId};

/// 排序键（§42）。NULL/缺失排最后；同键 `id` 升序决胜（§37 稳定性）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Title,
    Artist,
    Album,
    DateAdded,
    Duration,
    Year,
}

impl SortKey {
    fn sql_order(self) -> &'static str {
        match self {
            Self::Title => "t.title ASC NULLS LAST",
            Self::Artist => "ar.name ASC NULLS LAST",
            Self::Album => "al.title ASC NULLS LAST",
            Self::DateAdded => "t.date_added_ms ASC",
            Self::Duration => "t.duration_frames ASC NULLS LAST",
            Self::Year => "t.year ASC NULLS LAST",
        }
    }
}

/// 查询分页参数（§41，D-044）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub limit: i64,
    pub offset: i64,
}

impl Page {
    #[must_use]
    pub const fn new(limit: i64, offset: i64) -> Self {
        Self { limit, offset }
    }
}

/// track 列投影（顺序对应 `map_track`）。
pub(crate) const TRACK_COLUMNS: &str = "t.id, t.identity_key, t.title, \
    ar.name, al.title, aar.name, g.name, \
    t.year, t.track_number, t.disc_number, t.duration_frames, t.sample_rate, \
    t.channels, t.codec, t.container, t.artwork_id, t.file_size, \
    t.date_added_ms, t.last_scanned_ms";

/// 列投影所需的 join（`tracks t` + 实体表别名）。
pub(crate) const TRACK_FROM: &str = "FROM tracks t \
    LEFT JOIN artists ar ON ar.id = t.artist_id \
    LEFT JOIN albums al ON al.id = t.album_id \
    LEFT JOIN artists aar ON aar.id = t.album_artist_id \
    LEFT JOIN genres g ON g.id = t.genre_id";

impl LibraryService {
    /// 列出活跃 track（§40/§41）。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn list_tracks(&self, sort: SortKey, page: Page) -> Result<Vec<Track>> {
        let sql = format!(
            "SELECT {TRACK_COLUMNS} {TRACK_FROM} WHERE t.is_active = 1 ORDER BY {}, t.id LIMIT ?1 OFFSET ?2",
            sort.sql_order()
        );
        let mut stmt = self.database().connection().prepare_cached(&sql)?;
        let rows = stmt.query_map(params![page.limit, page.offset], map_track)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 单个 track。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn get_track(&self, id: TrackId) -> Result<Option<Track>> {
        let sql = format!("SELECT {TRACK_COLUMNS} {TRACK_FROM} WHERE t.id = ?1");
        let row = self
            .database()
            .connection()
            .query_row(&sql, params![id.0], map_track);
        match row {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// FTS 全文搜索 → track 列表（相关性排序，§36/§37）。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn search_tracks(&self, query: &str, page: Page) -> Result<Vec<Track>> {
        let hits = search::search(self.database().connection(), query, page.limit, page.offset)?;
        let mut out = Vec::with_capacity(hits.len());
        for hit in hits {
            if let Some(track) = self.get_track(hit.track_id)? {
                out.push(track);
            }
        }
        Ok(out)
    }

    /// 过滤表达式查询（§38）。M3 在 Rust 侧求值（分析数据 M8 落库后改 SQL
    /// 下推，D-045）；`MissingData` 字段视为不命中（§38 不猜值）。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn filter_tracks(
        &self,
        expr: &crate::search::filter::FilterExpression,
        page: Page,
    ) -> Result<Vec<Track>> {
        use crate::search::filter::{evaluate, FilterContext, FilterOutcome};
        // 全量活跃集（稳定排序）顺序过滤再分页；M3 规模可接受（D-045）。
        let all = self.list_tracks(SortKey::Title, Page::new(i64::MAX, 0))?;
        let mut out = Vec::new();
        let mut skipped = 0i64;
        for track in all {
            let ctx = FilterContext {
                title: track.title.as_deref(),
                artist: track.artist.as_deref(),
                album: track.album.as_deref(),
                album_artist: track.album_artist.as_deref(),
                genre: track.genre.as_deref(),
                year: track.year,
                codec: track.codec.as_deref(),
                container: track.container.as_deref(),
                track_number: track.track_number,
                disc_number: track.disc_number,
            };
            if matches!(evaluate(expr, &ctx), FilterOutcome::Match) {
                if skipped < page.offset {
                    skipped += 1;
                } else if (out.len() as i64) < page.limit {
                    out.push(track);
                } else {
                    break;
                }
            }
        }
        Ok(out)
    }

    /// 封面字节（§40）。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn get_artwork(&self, id: ArtworkId) -> Result<Option<(String, Vec<u8>)>> {
        Ok(self
            .database()
            .connection()
            .query_row(
                "SELECT mime, data FROM artworks WHERE id = ?1",
                params![id.0],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?)),
            )
            .ok())
    }

    /// 艺术家列表（活跃 track 计数；名称序，§40）。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn get_artists(&self) -> Result<Vec<(i64, String, i64)>> {
        let mut stmt = self.database().connection().prepare_cached(
            "SELECT a.id, a.name, count(t.id)
             FROM artists a
             LEFT JOIN tracks t ON t.artist_id = a.id AND t.is_active = 1
             GROUP BY a.id ORDER BY a.name ASC, a.id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 专辑列表（活跃 track 计数；标题序，§40）。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn get_albums(&self) -> Result<Vec<(i64, String, i64)>> {
        let mut stmt = self.database().connection().prepare_cached(
            "SELECT al.id, al.title, count(t.id)
             FROM albums al
             LEFT JOIN tracks t ON t.album_id = al.id AND t.is_active = 1
             GROUP BY al.id ORDER BY al.title ASC, al.id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// 播放统计（§54；identity 绑定，不与路径绑定）。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn playback_stats(&self, id: TrackId) -> Result<PlaybackStats> {
        let row = self.database().connection().query_row(
            "SELECT play_count, skip_count, total_played_frames, last_played_at_ms
             FROM playback_stats WHERE track_id = ?1",
            params![id.0],
            |r| {
                Ok(PlaybackStats {
                    play_count: u64::try_from(r.get::<_, i64>(0)?).unwrap_or(0),
                    skip_count: u64::try_from(r.get::<_, i64>(1)?).unwrap_or(0),
                    total_played_frames: u64::try_from(r.get::<_, i64>(2)?).unwrap_or(0),
                    last_played_at_ms: r.get(3)?,
                })
            },
        );
        Ok(row.unwrap_or_default())
    }

    /// 记录一次播放（§55：存储模型；play/skip 判定语义由应用层集成）。
    ///
    /// # Errors
    /// 写入失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn record_play(&mut self, id: TrackId, played_frames: u64, now_ms: i64) -> Result<()> {
        let tx = self.db_tx()?;
        tx.execute(
            "INSERT INTO playback_stats(track_id, play_count, total_played_frames, last_played_at_ms)
             VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(track_id) DO UPDATE SET
                play_count = play_count + 1,
                total_played_frames = total_played_frames + excluded.total_played_frames,
                last_played_at_ms = excluded.last_played_at_ms",
            params![id.0, i64::try_from(played_frames).unwrap_or(i64::MAX), now_ms],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// 记录一次跳过（§55）。
    ///
    /// # Errors
    /// 写入失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn record_skip(&mut self, id: TrackId, now_ms: i64) -> Result<()> {
        let tx = self.db_tx()?;
        tx.execute(
            "INSERT INTO playback_stats(track_id, skip_count, last_played_at_ms)
             VALUES (?1, 1, ?2)
             ON CONFLICT(track_id) DO UPDATE SET
                skip_count = skip_count + 1, last_played_at_ms = excluded.last_played_at_ms",
            params![id.0, now_ms],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// 一致性诊断（§76/§77）：只诊断不修复（M3 不建大修复系统）。
    ///
    /// # Errors
    /// 查询失败时返回 [`crate::error::LibraryError::Database`]。
    pub fn integrity_check(&self) -> Result<IntegrityReport> {
        let conn = self.database().connection();
        let mut report = IntegrityReport::default();

        {
            let mut stmt = conn.prepare("PRAGMA foreign_key_check")?;
            report.broken_foreign_keys = stmt.query_map([], |_r| Ok(()))?.count() as i64;
        }

        report.orphan_artworks = conn.query_row(
            "SELECT count(*) FROM artworks a
             WHERE NOT EXISTS (SELECT 1 FROM tracks t WHERE t.artwork_id = a.id)",
            [],
            |r| r.get(0),
        )?;

        report.dangling_artwork_refs = conn.query_row(
            "SELECT count(*) FROM tracks t
             WHERE t.artwork_id IS NOT NULL
               AND NOT EXISTS (SELECT 1 FROM artworks a WHERE a.id = t.artwork_id)",
            [],
            |r| r.get(0),
        )?;

        report.duplicate_identities = conn.query_row(
            "SELECT count(*) FROM (
                SELECT identity_key FROM tracks GROUP BY identity_key HAVING count(*) > 1
             )",
            [],
            |r| r.get(0),
        )?;

        report.duplicate_active_paths = conn.query_row(
            "SELECT count(*) FROM (
                SELECT path FROM track_paths WHERE is_active = 1 GROUP BY path HAVING count(*) > 1
             )",
            [],
            |r| r.get(0),
        )?;

        {
            let mut stmt = conn.prepare("SELECT path FROM track_paths WHERE is_active = 1")?;
            let paths: Vec<String> = stmt
                .query_map([], |r| r.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            report.missing_files = paths
                .iter()
                .filter(|p| !std::path::Path::new(p.as_str()).is_file())
                .count() as i64;
        }

        report.fts_rows = conn.query_row("SELECT count(*) FROM tracks_fts", [], |r| r.get(0))?;
        report.fts_orphans = conn.query_row(
            "SELECT count(*) FROM tracks_fts f
             WHERE NOT EXISTS (SELECT 1 FROM tracks t WHERE t.id = f.track_id)",
            [],
            |r| r.get(0),
        )?;

        Ok(report)
    }
}

/// 一致性报告（§76）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IntegrityReport {
    pub broken_foreign_keys: i64,
    pub orphan_artworks: i64,
    pub dangling_artwork_refs: i64,
    pub duplicate_identities: i64,
    pub duplicate_active_paths: i64,
    pub missing_files: i64,
    /// FTS 行总数（对照活跃 track 数）。
    pub fts_rows: i64,
    /// 指向已不存在 track 的 FTS 行。
    pub fts_orphans: i64,
}

impl IntegrityReport {
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.broken_foreign_keys == 0
            && self.dangling_artwork_refs == 0
            && self.duplicate_identities == 0
            && self.duplicate_active_paths == 0
            && self.fts_orphans == 0
    }
}

pub(crate) fn map_track(r: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
    Ok(Track {
        id: TrackId(r.get(0)?),
        identity_key: r.get(1)?,
        title: r.get(2)?,
        artist: r.get(3)?,
        album: r.get(4)?,
        album_artist: r.get(5)?,
        genre: r.get(6)?,
        year: r.get(7)?,
        track_number: r
            .get::<_, Option<i64>>(8)?
            .map(|v| u32::try_from(v).unwrap_or(0)),
        disc_number: r
            .get::<_, Option<i64>>(9)?
            .map(|v| u32::try_from(v).unwrap_or(0)),
        duration_frames: r
            .get::<_, Option<i64>>(10)?
            .map(|v| u64::try_from(v).unwrap_or(0)),
        sample_rate: r
            .get::<_, Option<i64>>(11)?
            .map(|v| u32::try_from(v).unwrap_or(0)),
        channels: r
            .get::<_, Option<i64>>(12)?
            .map(|v| u16::try_from(v).unwrap_or(0)),
        codec: r.get(13)?,
        container: r.get(14)?,
        artwork_id: r.get::<_, Option<i64>>(15)?.map(ArtworkId),
        file_size: u64::try_from(r.get::<_, i64>(16)?).unwrap_or(0),
        date_added_ms: r.get(17)?,
        last_scanned_ms: r.get(18)?,
    })
}
