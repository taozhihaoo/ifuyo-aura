//! 仓库层（§18/§21/§26/§28/§47/§49/§72）：所有写路径在单事务内完成，
//! 身份解析规则见 D-038/D-039。幂等：同一输入重复导入不产生重复行。

use std::path::Path;

use rusqlite::{params, Connection, Transaction};

use crate::identity;
use crate::metadata::ParsedFile;
use crate::normalize::search_text;
use crate::types::{Artwork, ArtworkId, RootId, TrackId, TrackIdentity};

/// 导入结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportOutcome {
    pub track_id: TrackId,
    pub created: bool,
    pub updated: bool,
}

/// 单文件完整导入管线：解析标签 → 内容身份 → 封面去重 → track upsert。
/// 必须在事务内调用（调用方负责提交/回滚，§49/§67：取消不半写）。
pub fn import_file(
    tx: &Transaction,
    root_id: RootId,
    path: &Path,
    now_ms: i64,
) -> crate::error::Result<Vec<ImportOutcome>> {
    let parsed = crate::metadata::parse_file(path)?;
    let file_size = std::fs::metadata(path)
        .map_err(|e| crate::error::LibraryError::Io {
            path: Some(path.to_path_buf()),
            source: e,
        })?
        .len();
    let modified_ms = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        })
        .unwrap_or(0);
    let ident = crate::identity::content_identity(path, parsed.metadata.audio_sig())?;
    let artwork_id = parsed
        .artwork
        .as_ref()
        .map(|a| upsert_artwork(tx, a))
        .transpose()?;
    let main = upsert_track(
        tx,
        root_id,
        path,
        &parsed,
        &ident,
        artwork_id,
        file_size,
        modified_ms,
        now_ms,
    )?;
    let mut outcomes = vec![main];

    // M15a CUE 整轨：同名 .cue 且 ≥2 AUDIO 子轨 → 展开为虚拟子轨
    // （identity = 父身份#轨道号、合成路径 path#NN、偏移/时长入新列）。
    if let Some(sheet) = crate::cue::load_for_audio(path) {
        let src_rate = f64::from(parsed.metadata.sample_rate.unwrap_or(44_100).max(1));
        let file_dur = parsed.metadata.duration_frames.map(|f| f as f64 / src_rate);
        let parent_artwork: Option<ArtworkId> = tx
            .query_row(
                "SELECT artwork_id FROM tracks WHERE id = ?1",
                params![main.track_id.0],
                |r| r.get::<_, Option<i64>>(0).map(|a| a.map(ArtworkId)),
            )
            .ok()
            .flatten();
        for (i, sub) in sheet.subtracks.iter().enumerate() {
            let start = sub.start_seconds;
            let duration = match sheet.subtracks.get(i + 1) {
                Some(next) => (next.start_seconds - start).max(0.0),
                None => file_dur.map(|d| (d - start).max(0.0)).unwrap_or(0.0),
            };
            if duration <= 0.0 {
                continue;
            }
            let ident_sub = TrackIdentity {
                identity_key: format!("{}#{}", ident.identity_key, sub.number),
                audio_sig: ident.audio_sig.clone(),
            };
            let mut meta = crate::types::TrackMetadata {
                title: Some(
                    sub.title
                        .clone()
                        .unwrap_or_else(|| format!("Track {:02}", sub.number)),
                ),
                artist: sub
                    .performer
                    .clone()
                    .or_else(|| sheet.album_performer.clone()),
                album: sheet.album.clone(),
                album_artist: sheet.album_performer.clone(),
                cue_start_seconds: Some(start),
                cue_duration_seconds: Some(duration),
                duration_frames: Some((duration * src_rate) as u64),
                sample_rate: parsed.metadata.sample_rate,
                channels: parsed.metadata.channels,
                codec: parsed.metadata.codec.clone(),
                container: parsed.metadata.container.clone(),
                ..crate::types::TrackMetadata::default()
            };
            meta.album_artist = meta.album_artist.or_else(|| meta.artist.clone());
            let synth = format!("{}#{:02}", path.display(), sub.number);
            let o = upsert_track(
                tx,
                root_id,
                Path::new(&synth),
                &ParsedFile {
                    metadata: meta,
                    artwork: None,
                    artwork_unavailable: None,
                },
                &ident_sub,
                parent_artwork,
                file_size,
                modified_ms,
                now_ms,
            )?;
            outcomes.push(o);
        }
        // 子轨展开成功 → 隐藏整轨主行（对标：用户不应看到超长整轨；
        // identity 保留，删掉 .cue 后重扫即恢复整轨行）。
        if outcomes.len() > 1 {
            tx.execute(
                "UPDATE tracks SET is_active = 0 WHERE id = ?1",
                params![main.track_id.0],
            )?;
        }
    }
    Ok(outcomes)
}

pub fn insert_root(conn: &Connection, path: &str, now_ms: i64) -> crate::error::Result<RootId> {
    conn.execute(
        "INSERT INTO library_roots(path, date_added_ms) VALUES (?1, ?2)
         ON CONFLICT(path) DO UPDATE SET path = excluded.path",
        params![path, now_ms],
    )?;
    let id = conn.query_row(
        "SELECT id FROM library_roots WHERE path = ?1",
        params![path],
        |r| r.get(0),
    )?;
    Ok(RootId(id))
}

pub fn get_roots(conn: &Connection) -> crate::error::Result<Vec<(RootId, String)>> {
    let mut stmt = conn.prepare("SELECT id, path FROM library_roots ORDER BY id")?;
    let rows = stmt.query_map([], |r| Ok((RootId(r.get(0)?), r.get(1)?)))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// 内容哈希去重的 artwork 引用（§28）。
pub(crate) fn upsert_artwork(tx: &Transaction, art: &Artwork) -> crate::error::Result<ArtworkId> {
    let content_hash = format!("art:{}", identity::fingerprint_hex(&art.bytes));
    tx.execute(
        "INSERT INTO artworks(content_hash, mime, byte_size, data) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(content_hash) DO NOTHING",
        params![content_hash, art.mime, art.bytes.len() as i64, art.bytes],
    )?;
    let id: i64 = tx.query_row(
        "SELECT id FROM artworks WHERE content_hash = ?1",
        params![content_hash],
        |r| r.get(0),
    )?;
    Ok(ArtworkId(id))
}

/// 实体 upsert：按 norm 去重（§26），display 名跟随最新导入。
fn upsert_entity(
    tx: &Transaction,
    table: &str,
    name: &str,
    norm: &str,
) -> crate::error::Result<i64> {
    // 表名来自本文件内的固定常量，不拼接用户输入（§64）。
    tx.execute(
        &format!(
            "INSERT INTO {table}(name, norm) VALUES (?1, ?2)
             ON CONFLICT(norm) DO UPDATE SET name = excluded.name"
        ),
        params![name, norm],
    )?;
    tx.query_row(
        &format!("SELECT id FROM {table} WHERE norm = ?1"),
        params![norm],
        |r| r.get(0),
    )
    .map_err(Into::into)
}

/// 导入/更新一个文件（单事务，§49）。
///
/// 身份解析（D-038/D-039）：
/// 1. `identity_key` 已存在 → 同一 track（改名/移动/重复内容合并）；
/// 2. 同路径新内容且 `audio_sig` 未变 → 标签改写：保留 `TrackId`，刷新 metadata；
/// 3. 同路径新内容且 `audio_sig` 变化 → 音频替换：新 track；旧 track 无活跃
///    路径时停用（播放统计保留，§47）；
/// 4. 全新文件：若存在已停用且音频签名+搜索标题匹配的孤儿（移动+改标签
///    同时发生）→ 重新关联；否则新建。
///
/// # Errors
/// SQL 失败返回 [`crate::error::LibraryError::Database`]；事务由调用方提交。
#[allow(clippy::too_many_arguments)]
pub fn upsert_track(
    tx: &Transaction,
    root_id: RootId,
    path: &Path,
    parsed: &ParsedFile,
    identity: &TrackIdentity,
    artwork_id: Option<ArtworkId>,
    file_size: u64,
    modified_ms: i64,
    now_ms: i64,
) -> crate::error::Result<ImportOutcome> {
    // 路径规范化存储：canonicalize 消除 `\\?\` 前缀差异与大小写/分隔符歧义；
    // 失败（文件被并发删除）时退回原路径字符串。
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let path_str = canonical.to_string_lossy().into_owned();
    let m = &parsed.metadata;

    let title = m.title.clone();
    let artist = m.artist.clone();
    let album = m.album.clone();
    let album_artist = m.album_artist.clone().or_else(|| artist.clone());
    let genre = m.genre.clone();

    let norm_title = title.as_deref().map(search_text);
    let artist_id = artist
        .as_deref()
        .map(|n| upsert_entity(tx, "artists", n, &search_text(n)))
        .transpose()?;
    let album_artist_id = album_artist
        .as_deref()
        .map(|n| upsert_entity(tx, "artists", n, &search_text(n)))
        .transpose()?;
    let album_id = album
        .as_deref()
        .map(|n| {
            let norm_album = search_text(n);
            let artist_for_album = album_artist_id.or(artist_id);
            tx.execute(
                "INSERT INTO albums(title, norm, artist_id) VALUES (?1, ?2, ?3)
                 ON CONFLICT(norm, IFNULL(artist_id, 0)) DO UPDATE SET title = excluded.title",
                params![n, norm_album, artist_for_album],
            )?;
            tx.query_row(
                "SELECT id FROM albums WHERE norm = ?1 AND IFNULL(artist_id, 0) = IFNULL(?2, 0)",
                params![norm_album, artist_for_album],
                |r| r.get::<_, i64>(0),
            )
            .map_err(crate::error::LibraryError::from)
        })
        .transpose()?;
    let genre_id = genre
        .as_deref()
        .map(|n| upsert_entity(tx, "genres", n, &search_text(n)))
        .transpose()?;

    // ① 身份已存在：改名/移动/重复内容 → 合并到既有 track。
    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM tracks WHERE identity_key = ?1",
            params![identity.identity_key],
            |r| r.get(0),
        )
        .ok();

    if let Some(track_id) = existing {
        upsert_path_row(tx, track_id, root_id, &path_str, now_ms)?;
        update_track_fields(
            tx,
            track_id,
            parsed,
            identity,
            norm_title.as_deref(),
            artist_id,
            album_id,
            album_artist_id,
            genre_id,
            artwork_id,
            file_size,
            modified_ms,
            now_ms,
        )?;
        tx.execute(
            "UPDATE tracks SET is_active = 1 WHERE id = ?1",
            params![track_id],
        )?;
        sync_fts(
            tx,
            track_id,
            title.as_deref(),
            artist.as_deref(),
            album.as_deref(),
            album_artist.as_deref(),
            genre.as_deref(),
            &path_str,
        )?;
        ensure_stats_row(tx, track_id)?;
        return Ok(ImportOutcome {
            track_id: TrackId(track_id),
            created: false,
            updated: true,
        });
    }

    // 已知路径上的既有 track（用于规则 ②/③）。
    let prev_track: Option<i64> = tx
        .query_row(
            "SELECT track_id FROM track_paths WHERE path = ?1",
            params![path_str],
            |r| r.get(0),
        )
        .ok();

    if let Some(prev) = prev_track {
        let prev_sig: String = tx.query_row(
            "SELECT audio_sig FROM tracks WHERE id = ?1",
            params![prev],
            |r| r.get(0),
        )?;
        if prev_sig == identity.audio_sig {
            // ② 标签改写：TrackId 不变（§10），身份键与字段刷新。
            update_track_fields(
                tx,
                prev,
                parsed,
                identity,
                norm_title.as_deref(),
                artist_id,
                album_id,
                album_artist_id,
                genre_id,
                artwork_id,
                file_size,
                modified_ms,
                now_ms,
            )?;
            touch_path(tx, prev, &path_str, now_ms)?;
            tx.execute(
                "UPDATE tracks SET is_active = 1 WHERE id = ?1",
                params![prev],
            )?;
            sync_fts(
                tx,
                prev,
                title.as_deref(),
                artist.as_deref(),
                album.as_deref(),
                album_artist.as_deref(),
                genre.as_deref(),
                &path_str,
            )?;
            ensure_stats_row(tx, prev)?;
            return Ok(ImportOutcome {
                track_id: TrackId(prev),
                created: false,
                updated: true,
            });
        }
        // ③ 音频替换：新建 track，路径转挂新 track。
        let new_id = insert_track(
            tx,
            parsed,
            identity,
            norm_title.as_deref(),
            artist_id,
            album_id,
            album_artist_id,
            genre_id,
            artwork_id,
            file_size,
            modified_ms,
            now_ms,
        )?;
        tx.execute(
            "UPDATE track_paths SET track_id = ?1, is_active = 1, last_seen_ms = ?2 WHERE path = ?3",
            params![new_id, now_ms, path_str],
        )?;
        sync_fts(
            tx,
            new_id,
            title.as_deref(),
            artist.as_deref(),
            album.as_deref(),
            album_artist.as_deref(),
            genre.as_deref(),
            &path_str,
        )?;
        ensure_stats_row(tx, new_id)?;
        deactivate_if_orphan(tx, prev)?;
        return Ok(ImportOutcome {
            track_id: TrackId(new_id),
            created: true,
            updated: false,
        });
    }

    // ④ 全新路径：先尝试移动+改标签的孤儿重关联。
    let orphan: Option<i64> = match (&norm_title, identity.audio_sig.as_str()) {
        (Some(nt), sig) => tx
            .query_row(
                "SELECT id FROM tracks
                 WHERE is_active = 0 AND audio_sig = ?1 AND norm_title = ?2
                 ORDER BY id LIMIT 1",
                params![sig, nt],
                |r| r.get(0),
            )
            .ok(),
        _ => None,
    };

    if let Some(orphan_id) = orphan {
        update_track_fields(
            tx,
            orphan_id,
            parsed,
            identity,
            norm_title.as_deref(),
            artist_id,
            album_id,
            album_artist_id,
            genre_id,
            artwork_id,
            file_size,
            modified_ms,
            now_ms,
        )?;
        tx.execute(
            "INSERT INTO track_paths(track_id, root_id, path, is_active, first_seen_ms, last_seen_ms)
             VALUES (?1, ?2, ?3, 1, ?4, ?4)
             ON CONFLICT(path) DO UPDATE SET track_id = excluded.track_id,
                 is_active = 1, last_seen_ms = excluded.last_seen_ms",
            params![orphan_id, root_id.0, path_str, now_ms],
        )?;
        tx.execute(
            "UPDATE tracks SET is_active = 1 WHERE id = ?1",
            params![orphan_id],
        )?;
        sync_fts(
            tx,
            orphan_id,
            title.as_deref(),
            artist.as_deref(),
            album.as_deref(),
            album_artist.as_deref(),
            genre.as_deref(),
            &path_str,
        )?;
        ensure_stats_row(tx, orphan_id)?;
        return Ok(ImportOutcome {
            track_id: TrackId(orphan_id),
            created: false,
            updated: true,
        });
    }

    // ⑤ 全新 track。
    let new_id = insert_track(
        tx,
        parsed,
        identity,
        norm_title.as_deref(),
        artist_id,
        album_id,
        album_artist_id,
        genre_id,
        artwork_id,
        file_size,
        modified_ms,
        now_ms,
    )?;
    tx.execute(
        "INSERT INTO track_paths(track_id, root_id, path, is_active, first_seen_ms, last_seen_ms)
         VALUES (?1, ?2, ?3, 1, ?4, ?4)",
        params![new_id, root_id.0, path_str, now_ms],
    )?;
    sync_fts(
        tx,
        new_id,
        title.as_deref(),
        artist.as_deref(),
        album.as_deref(),
        album_artist.as_deref(),
        genre.as_deref(),
        &path_str,
    )?;
    ensure_stats_row(tx, new_id)?;
    Ok(ImportOutcome {
        track_id: TrackId(new_id),
        created: true,
        updated: false,
    })
}

/// 扫描收尾：root 内未在本轮出现的路径停用（§47），
/// 无活跃路径的 track 停用；返回停用路径数。
pub fn mark_missing(
    tx: &Transaction,
    root_id: RootId,
    seen_paths: &[String],
    now_ms: i64,
) -> crate::error::Result<u64> {
    let mut deactivated = 0u64;
    let mut stmt =
        tx.prepare("SELECT id, path FROM track_paths WHERE root_id = ?1 AND is_active = 1")?;
    let rows: Vec<(i64, String)> = stmt
        .query_map(params![root_id.0], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);
    for (id, path) in rows {
        // M15a：CUE 子轨的合成路径（`整轨路径#NN`）由整轨路径覆盖——
        // 未变化跳过时子轨不在本轮导入产物里，但不应被当作消失停用。
        let covered = seen_paths
            .iter()
            .any(|sp| path.as_str() == sp.as_str() || path.starts_with(&format!("{sp}#")));
        if !covered {
            tx.execute(
                "UPDATE track_paths SET is_active = 0 WHERE id = ?1",
                params![id],
            )?;
            deactivated += 1;
            let track_id: i64 = tx.query_row(
                "SELECT track_id FROM track_paths WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )?;
            deactivate_if_orphan(tx, track_id)?;
        }
    }
    let _ = now_ms;
    Ok(deactivated)
}

fn deactivate_if_orphan(tx: &Transaction, track_id: i64) -> crate::error::Result<()> {
    let active: i64 = tx.query_row(
        "SELECT count(*) FROM track_paths WHERE track_id = ?1 AND is_active = 1",
        params![track_id],
        |r| r.get(0),
    )?;
    if active == 0 {
        tx.execute(
            "UPDATE tracks SET is_active = 0 WHERE id = ?1",
            params![track_id],
        )?;
    }
    Ok(())
}

fn upsert_path_row(
    tx: &Transaction,
    track_id: i64,
    root_id: RootId,
    path: &str,
    now_ms: i64,
) -> crate::error::Result<()> {
    tx.execute(
        "INSERT INTO track_paths(track_id, root_id, path, is_active, first_seen_ms, last_seen_ms)
         VALUES (?1, ?2, ?3, 1, ?4, ?4)
         ON CONFLICT(path) DO UPDATE SET track_id = excluded.track_id,
             is_active = 1, last_seen_ms = excluded.last_seen_ms",
        params![track_id, root_id.0, path, now_ms],
    )?;
    Ok(())
}

fn touch_path(
    tx: &Transaction,
    track_id: i64,
    path: &str,
    now_ms: i64,
) -> crate::error::Result<()> {
    tx.execute(
        "UPDATE track_paths SET is_active = 1, last_seen_ms = ?1, track_id = ?2 WHERE path = ?3",
        params![now_ms, track_id, path],
    )?;
    Ok(())
}

fn ensure_stats_row(tx: &Transaction, track_id: i64) -> crate::error::Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO playback_stats(track_id) VALUES (?1)",
        params![track_id],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_track(
    tx: &Transaction,
    parsed: &ParsedFile,
    identity: &TrackIdentity,
    norm_title: Option<&str>,
    artist_id: Option<i64>,
    album_id: Option<i64>,
    album_artist_id: Option<i64>,
    genre_id: Option<i64>,
    artwork_id: Option<ArtworkId>,
    file_size: u64,
    modified_ms: i64,
    now_ms: i64,
) -> crate::error::Result<i64> {
    let m = &parsed.metadata;
    tx.execute(
        "INSERT INTO tracks(
            identity_key, audio_sig, title, norm_title,
            artist_id, album_id, genre_id, album_artist_id,
            year, track_number, track_total, disc_number, disc_total,
            duration_frames, sample_rate, channels, codec, container,
            cue_start_seconds, cue_duration_seconds,
            artwork_id, file_size, modified_ms, date_added_ms, last_scanned_ms, is_active
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,1)",
        params![
            identity.identity_key,
            identity.audio_sig,
            m.title,
            norm_title,
            artist_id,
            album_id,
            genre_id,
            album_artist_id,
            m.year,
            m.track_number.map(i64::from),
            m.track_total.map(i64::from),
            m.disc_number.map(i64::from),
            m.disc_total.map(i64::from),
            i64::try_from(m.duration_frames.unwrap_or(0)).unwrap_or(i64::MAX),
            m.sample_rate.map(i64::from),
            m.channels.map(i64::from),
            m.codec,
            m.container,
            m.cue_start_seconds,
            m.cue_duration_seconds,
            artwork_id.map(|a| a.0),
            i64::try_from(file_size).unwrap_or(i64::MAX),
            modified_ms,
            now_ms,
            now_ms,
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
fn update_track_fields(
    tx: &Transaction,
    track_id: i64,
    parsed: &ParsedFile,
    identity: &TrackIdentity,
    norm_title: Option<&str>,
    artist_id: Option<i64>,
    album_id: Option<i64>,
    album_artist_id: Option<i64>,
    genre_id: Option<i64>,
    artwork_id: Option<ArtworkId>,
    file_size: u64,
    modified_ms: i64,
    now_ms: i64,
) -> crate::error::Result<()> {
    let m = &parsed.metadata;
    tx.execute(
        "UPDATE tracks SET
            identity_key = ?1, audio_sig = ?2, title = ?3, norm_title = ?4,
            artist_id = ?5, album_id = ?6, genre_id = ?7, album_artist_id = ?8,
            year = ?9, track_number = ?10, track_total = ?11, disc_number = ?12, disc_total = ?13,
            duration_frames = ?14, sample_rate = ?15, channels = ?16, codec = ?17, container = ?18,
            cue_start_seconds = ?19, cue_duration_seconds = ?20,
            artwork_id = ?21, file_size = ?22, modified_ms = ?23, last_scanned_ms = ?24
         WHERE id = ?25",
        params![
            identity.identity_key,
            identity.audio_sig,
            m.title,
            norm_title,
            artist_id,
            album_id,
            genre_id,
            album_artist_id,
            m.year,
            m.track_number.map(i64::from),
            m.track_total.map(i64::from),
            m.disc_number.map(i64::from),
            m.disc_total.map(i64::from),
            i64::try_from(m.duration_frames.unwrap_or(0)).unwrap_or(i64::MAX),
            m.sample_rate.map(i64::from),
            m.channels.map(i64::from),
            m.codec,
            m.container,
            m.cue_start_seconds,
            m.cue_duration_seconds,
            artwork_id.map(|a| a.0),
            i64::try_from(file_size).unwrap_or(i64::MAX),
            modified_ms,
            now_ms,
            track_id,
        ],
    )?;
    Ok(())
}

/// 同步 track 的 FTS 索引行（事务内；旧行按记录的 rowid 删除，§49）。
#[allow(clippy::too_many_arguments)]
fn sync_fts(
    tx: &Transaction,
    track_id: i64,
    title: Option<&str>,
    artist: Option<&str>,
    album: Option<&str>,
    album_artist: Option<&str>,
    genre: Option<&str>,
    path: &str,
) -> crate::error::Result<()> {
    let old: Option<Option<i64>> = tx
        .query_row(
            "SELECT fts_rowid FROM tracks WHERE id = ?1",
            params![track_id],
            |r| r.get::<_, Option<i64>>(0),
        )
        .ok();
    crate::search::delete_track(tx, old.flatten())?;
    let doc = crate::search::FtsDoc {
        title,
        artist,
        album,
        album_artist,
        genre,
        path,
    };
    let rowid = crate::search::insert_track(tx, TrackId(track_id), &doc)?;
    tx.execute(
        "UPDATE tracks SET fts_rowid = ?1 WHERE id = ?2",
        params![rowid, track_id],
    )?;
    Ok(())
}
