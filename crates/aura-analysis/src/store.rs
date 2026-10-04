//! Analysis persistence（§24–§32）：M3 Library SQLite 单表存储。
//!
//! - 不建第二数据库（§25）；schema 由 M3 迁移 V2 提供（§6 M3 拥有 DB 真相）。
//! - 单事务 all-or-nothing（§30/§31）；feature availability 明确列存（§32）。
//! - 身份判定（§15/§19/§20）：content_identity 不符 → Stale（replacement
//!   检测）；analysis_version 低于当前 → Stale（§18 版本过期）。

use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::ANALYSIS_VERSION;
use crate::domain::{
    AnalysisStatus, Embedding, FeatureAvailability, KeyAnalysis, KeyMode, MfccFeatures,
    MfccFeatures as Mfcc, MusicalKey, SpectralFeatures, TempoAnalysis, TrackAnalysis,
};

/// 持久化错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Database(String),
    /// 状态字符串无法反解（损坏/未知版本写入）。
    UnknownEnum {
        column: String,
        value: String,
    },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "analysis store: {e}"),
            Self::UnknownEnum { column, value } => {
                write!(f, "unknown enum value in {column}: {value}")
            }
        }
    }
}

type Result<T> = std::result::Result<T, StoreError>;

fn db(e: rusqlite::Error) -> StoreError {
    StoreError::Database(e.to_string())
}

/// availability 列的字符串编码（枚举 snake_case 名）。
fn avail_str(a: FeatureAvailability) -> &'static str {
    match a {
        FeatureAvailability::Pending => "pending",
        FeatureAvailability::Available => "available",
        FeatureAvailability::Unavailable => "unavailable",
        FeatureAvailability::Unsupported => "unsupported",
        FeatureAvailability::Failed => "failed",
        FeatureAvailability::Stale => "stale",
    }
}

fn avail_from(s: Option<String>, column: &str) -> Result<FeatureAvailability> {
    let s = s.ok_or_else(|| StoreError::UnknownEnum {
        column: column.to_string(),
        value: "<null>".to_string(),
    })?;
    Ok(match s.as_str() {
        "pending" => FeatureAvailability::Pending,
        "available" => FeatureAvailability::Available,
        "unavailable" => FeatureAvailability::Unavailable,
        "unsupported" => FeatureAvailability::Unsupported,
        "failed" => FeatureAvailability::Failed,
        "stale" => FeatureAvailability::Stale,
        other => {
            return Err(StoreError::UnknownEnum {
                column: column.to_string(),
                value: other.to_string(),
            })
        }
    })
}

fn status_str(s: AnalysisStatus) -> &'static str {
    match s {
        AnalysisStatus::Pending => "pending",
        AnalysisStatus::Running => "running",
        AnalysisStatus::Completed => "completed",
        AnalysisStatus::Failed => "failed",
        AnalysisStatus::Cancelled => "cancelled",
        AnalysisStatus::Unsupported => "unsupported",
        AnalysisStatus::Stale => "stale",
    }
}

fn status_from(s: String) -> Result<AnalysisStatus> {
    Ok(match s.as_str() {
        "pending" => AnalysisStatus::Pending,
        "running" => AnalysisStatus::Running,
        "completed" => AnalysisStatus::Completed,
        "failed" => AnalysisStatus::Failed,
        "cancelled" => AnalysisStatus::Cancelled,
        "unsupported" => AnalysisStatus::Unsupported,
        "stale" => AnalysisStatus::Stale,
        other => {
            return Err(StoreError::UnknownEnum {
                column: "status".to_string(),
                value: other.to_string(),
            })
        }
    })
}

/// MFCC BLOB：13 维 mean + 13 维 variance，f32 LE（§27 带元数据列）。
fn mfcc_to_blob(m: &Mfcc) -> Vec<u8> {
    let mut out = Vec::with_capacity(m.mean.len() * 8);
    for v in m.mean.iter().chain(&m.variance) {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn mfcc_from_blob(blob: &[u8], algo_version: u16) -> Result<Option<MfccFeatures>> {
    if !blob.len().is_multiple_of(8) || blob.len() / 8 != Mfcc::DIMENSION {
        return Err(StoreError::Database(format!(
            "mfcc blob size {} != {} dims x2",
            blob.len(),
            Mfcc::DIMENSION
        )));
    }
    let floats: Vec<f32> = blob
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    let (mean, variance) = floats.split_at(Mfcc::DIMENSION);
    Ok(Some(MfccFeatures {
        mean: mean.to_vec(),
        variance: variance.to_vec(),
        algorithm_version: algo_version,
    }))
}

/// 分析存储（对 M3 连接的持久化门面；不建连接池/不做缓存——§25）。
pub struct AnalysisStore;

impl AnalysisStore {
    /// §30/§31：单事务 all-or-nothing 写入（INSERT OR REPLACE）。
    ///
    /// # Errors
    /// [`StoreError::Database`]。
    pub fn save(conn: &Connection, a: &TrackAnalysis) -> Result<()> {
        let mfcc_blob = a.mfcc.as_ref().map(mfcc_to_blob);
        let embedding_blob = a.embedding.as_ref().map(Embedding::to_blob);
        let embedding_dim = a.embedding.as_ref().map(|e| i64::from(e.dimension));
        let embedding_model_id = a.embedding.as_ref().map(|e| e.model_id.clone());
        let embedding_model_version = a.embedding.as_ref().map(|e| i64::from(e.model_version));
        conn.execute(
            "INSERT OR REPLACE INTO track_analysis (
                track_id, content_identity, analysis_key, analysis_version, status,
                bpm, bpm_confidence, first_beat_offset_s, tempo_algo_version, tempo_availability,
                musical_key, key_mode, key_confidence, camelot_key, key_algo_version, key_availability,
                lufs, lra, true_peak_dbtp, loudness_algo_version, loudness_availability,
                rms, peak, centroid_hz, rolloff_hz, brightness, spectral_algo_version, spectral_availability,
                mfcc_blob, mfcc_algo_version, mfcc_availability,
                embedding_blob, embedding_dim, embedding_model_id, embedding_model_version, embedding_availability,
                analyzed_at_ms
            ) VALUES (?1,?2,?3,?4,?5, ?6,?7,?8,?9,?10, ?11,?12,?13,?14,?15,?16,
                ?17,?18,?19,?20,?21, ?22,?23,?24,?25,?26,?27,?28,
                ?29,?30,?31, ?32,?33,?34,?35,?36, ?37)",
            params![
                a.track_id,
                a.content_identity,
                a.analysis_key,
                i64::from(a.analysis_version),
                status_str(a.status),
                a.tempo.as_ref().map(|t| f64::from(t.bpm)),
                a.tempo.as_ref().and_then(|t| if t.confidence.is_finite() { Some(f64::from(t.confidence)) } else { None }),
                a.tempo.as_ref().map(|t| f64::from(t.first_beat_offset_seconds)),
                a.tempo.as_ref().map(|t| i64::from(t.algorithm_version)),
                avail_str(a.tempo_availability),
                a.key.as_ref().map(|k| format!("{:?}", k.key)),
                a.key.as_ref().map(|k| format!("{:?}", k.mode)),
                a.key.as_ref().map(|k| f64::from(k.confidence)),
                a.key.as_ref().and_then(|k| k.camelot.clone()),
                a.key.as_ref().map(|k| i64::from(k.algorithm_version)),
                avail_str(a.key_availability),
                a.loudness.as_ref().map(|l| f64::from(l.integrated_lufs)),
                a.loudness.as_ref().map(|l| f64::from(l.lra)),
                a.loudness.as_ref().map(|l| f64::from(l.true_peak_dbtp)),
                a.loudness.as_ref().map(|l| i64::from(l.algorithm_version)),
                avail_str(a.loudness_availability),
                a.spectral.as_ref().map(|s| f64::from(s.rms)),
                a.spectral.as_ref().map(|s| f64::from(s.peak)),
                a.spectral.as_ref().map(|s| f64::from(s.centroid_hz)),
                a.spectral.as_ref().map(|s| f64::from(s.rolloff_hz)),
                a.spectral.as_ref().map(|s| f64::from(s.brightness)),
                a.spectral.as_ref().map(|s| i64::from(s.algorithm_version)),
                avail_str(a.spectral_availability),
                mfcc_blob,
                a.mfcc.as_ref().map(|m| i64::from(m.algorithm_version)),
                avail_str(a.mfcc_availability),
                embedding_blob,
                embedding_dim,
                embedding_model_id,
                embedding_model_version,
                avail_str(a.embedding_availability),
                a.analyzed_at_ms,
            ],
        )
        .map_err(db)?;
        Ok(())
    }

    /// 按 TrackId 读取。
    ///
    /// # Errors
    /// [`StoreError`]。
    pub fn load_by_track(conn: &Connection, track_id: i64) -> Result<Option<TrackAnalysis>> {
        let mut stmt = conn
            .prepare("SELECT * FROM track_analysis WHERE track_id = ?1")
            .map_err(db)?;
        stmt.query_row(params![track_id], Self::map_row)
            .optional()
            .map_err(db)
    }

    /// §23：按 analysis_key 读取（内容寻址）。
    ///
    /// # Errors
    /// [`StoreError`]。
    pub fn load_by_key(conn: &Connection, key: &str) -> Result<Option<TrackAnalysis>> {
        let mut stmt = conn
            .prepare("SELECT * FROM track_analysis WHERE analysis_key = ?1")
            .map_err(db)?;
        stmt.query_row(params![key], Self::map_row)
            .optional()
            .map_err(db)
    }

    /// §20：内容替换检测——同 track_id 的现存结果若 content_identity
    /// 不同 → Stale（旧 BPM 不得关联新音频）。
    ///
    /// # Errors
    /// [`StoreError`]。
    pub fn detect_replacement(
        conn: &Connection,
        track_id: i64,
        new_content_identity: &str,
    ) -> Result<bool> {
        let changed = conn
            .query_row(
                "SELECT content_identity FROM track_analysis WHERE track_id = ?1",
                params![track_id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(db)?
            .is_some_and(|old| old != new_content_identity);
        if changed {
            conn.execute(
                "UPDATE track_analysis SET status = 'stale' WHERE track_id = ?1",
                params![track_id],
            )
            .map_err(db)?;
        }
        Ok(changed)
    }

    /// §18：算法/合同版本升级后——低于 current_version 的结果全部 Stale。
    ///
    /// # Errors
    /// [`StoreError`]。
    pub fn mark_stale_below_version(conn: &Connection, current_version: u16) -> Result<u64> {
        let n = conn
            .execute(
                "UPDATE track_analysis SET status = 'stale' WHERE analysis_version < ?1 AND status != 'stale'",
                params![i64::from(current_version)],
            )
            .map_err(db)?;
        Ok(n as u64)
    }

    /// §22：文件移动/改名（track_id 不变）→ 分析保持（无需操作，
    /// 此查询仅供上层确认存在性）。
    ///
    /// # Errors
    /// [`StoreError`]。
    pub fn exists_for_track(conn: &Connection, track_id: i64) -> Result<bool> {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM track_analysis WHERE track_id = ?1",
                params![track_id],
                |r| r.get(0),
            )
            .map_err(db)?;
        Ok(n > 0)
    }

    /// §21：metadata-only 变更不需要重算——结果按 content_identity 命中。
    ///
    /// # Errors
    /// [`StoreError`]。
    pub fn find_valid_by_content(
        conn: &Connection,
        content_identity: &str,
    ) -> Result<Option<TrackAnalysis>> {
        let mut stmt = conn
            .prepare(
                "SELECT * FROM track_analysis WHERE content_identity = ?1 AND analysis_version = ?2 AND status = 'completed'",
            )
            .map_err(db)?;
        stmt.query_row(
            params![content_identity, i64::from(ANALYSIS_VERSION)],
            Self::map_row,
        )
        .optional()
        .map_err(db)
    }

    #[allow(clippy::too_many_lines)]
    fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<TrackAnalysis> {
        let tempo_avail =
            avail_from(r.get("tempo_availability")?, "tempo_availability").map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )),
                )
            })?;
        let key_avail =
            avail_from(r.get("key_availability")?, "key_availability").map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )),
                )
            })?;
        let loudness_avail = avail_from(r.get("loudness_availability")?, "loudness_availability")
            .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            )
        })?;
        let spectral_avail = avail_from(r.get("spectral_availability")?, "spectral_availability")
            .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            )
        })?;
        let mfcc_avail =
            avail_from(r.get("mfcc_availability")?, "mfcc_availability").map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e.to_string(),
                    )),
                )
            })?;
        let embedding_avail =
            avail_from(r.get("embedding_availability")?, "embedding_availability").map_err(
                |e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e.to_string(),
                        )),
                    )
                },
            )?;
        let status = status_from(r.get::<_, String>("status")?).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )),
            )
        })?;

        let tempo = if tempo_avail == FeatureAvailability::Available {
            Some(TempoAnalysis {
                bpm: r.get::<_, Option<f64>>("bpm")?.unwrap_or(0.0) as f32,
                confidence: r.get::<_, Option<f64>>("bpm_confidence")?.unwrap_or(0.0) as f32,
                first_beat_offset_seconds: r
                    .get::<_, Option<f64>>("first_beat_offset_s")?
                    .unwrap_or(0.0) as f32,
                algorithm_version: u16::try_from(
                    r.get::<_, Option<i64>>("tempo_algo_version")?.unwrap_or(0),
                )
                .unwrap_or(0),
            })
        } else {
            None
        };
        let key = if key_avail == FeatureAvailability::Available {
            let key_str: Option<String> = r.get("musical_key")?;
            let mode_str: Option<String> = r.get("key_mode")?;
            Some(KeyAnalysis {
                key: key_str
                    .and_then(|s| {
                        MusicalKey::ALL
                            .iter()
                            .copied()
                            .find(|k| format!("{k:?}") == s)
                    })
                    .unwrap_or(MusicalKey::C),
                mode: mode_str
                    .and_then(|s| match s.as_str() {
                        "Major" => Some(KeyMode::Major),
                        "Minor" => Some(KeyMode::Minor),
                        _ => None,
                    })
                    .unwrap_or(KeyMode::Major),
                confidence: r.get::<_, Option<f64>>("key_confidence")?.unwrap_or(0.0) as f32,
                camelot: r.get("camelot_key")?,
                algorithm_version: u16::try_from(
                    r.get::<_, Option<i64>>("key_algo_version")?.unwrap_or(0),
                )
                .unwrap_or(0),
            })
        } else {
            None
        };
        let loudness = if loudness_avail == FeatureAvailability::Available {
            Some(crate::domain::LoudnessAnalysis {
                integrated_lufs: r.get::<_, Option<f64>>("lufs")?.unwrap_or(0.0) as f32,
                lra: r.get::<_, Option<f64>>("lra")?.unwrap_or(0.0) as f32,
                true_peak_dbtp: r.get::<_, Option<f64>>("true_peak_dbtp")?.unwrap_or(0.0) as f32,
                algorithm_version: u16::try_from(
                    r.get::<_, Option<i64>>("loudness_algo_version")?
                        .unwrap_or(0),
                )
                .unwrap_or(0),
            })
        } else {
            None
        };
        let spectral = if spectral_avail == FeatureAvailability::Available {
            Some(SpectralFeatures {
                rms: r.get::<_, Option<f64>>("rms")?.unwrap_or(0.0) as f32,
                peak: r.get::<_, Option<f64>>("peak")?.unwrap_or(0.0) as f32,
                centroid_hz: r.get::<_, Option<f64>>("centroid_hz")?.unwrap_or(0.0) as f32,
                rolloff_hz: r.get::<_, Option<f64>>("rolloff_hz")?.unwrap_or(0.0) as f32,
                brightness: r.get::<_, Option<f64>>("brightness")?.unwrap_or(0.0) as f32,
                algorithm_version: u16::try_from(
                    r.get::<_, Option<i64>>("spectral_algo_version")?
                        .unwrap_or(0),
                )
                .unwrap_or(0),
            })
        } else {
            None
        };
        let mfcc = if mfcc_avail == FeatureAvailability::Available {
            let blob: Option<Vec<u8>> = r.get("mfcc_blob")?;
            let algo = u16::try_from(r.get::<_, Option<i64>>("mfcc_algo_version")?.unwrap_or(0))
                .unwrap_or(0);
            match blob {
                Some(b) => mfcc_from_blob(&b, algo).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        Type::Blob,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            e.to_string(),
                        )),
                    )
                })?,
                None => None,
            }
        } else {
            None
        };
        let embedding = if embedding_avail == FeatureAvailability::Available {
            let blob: Option<Vec<u8>> = r.get("embedding_blob")?;
            let dim: Option<i64> = r.get("embedding_dim")?;
            let model_id: Option<String> = r.get("embedding_model_id")?;
            let model_version = u16::try_from(
                r.get::<_, Option<i64>>("embedding_model_version")?
                    .unwrap_or(0),
            )
            .unwrap_or(0);
            match (blob, dim, model_id) {
                (Some(b), Some(d), Some(m)) => {
                    match Embedding::from_blob(&b, u32::try_from(d).unwrap_or(0), &m, model_version)
                    {
                        Ok(e) => Some(e),
                        Err(e) => {
                            return Err(rusqlite::Error::FromSqlConversionFailure(
                                0,
                                Type::Blob,
                                Box::new(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    e.to_string(),
                                )),
                            ))
                        }
                    }
                }
                _ => None,
            }
        } else {
            None
        };

        Ok(TrackAnalysis {
            track_id: r.get("track_id")?,
            content_identity: r.get("content_identity")?,
            analysis_key: r.get("analysis_key")?,
            analysis_version: u16::try_from(r.get::<_, i64>("analysis_version")?).unwrap_or(0),
            status,
            tempo,
            tempo_availability: tempo_avail,
            key,
            key_availability: key_avail,
            loudness,
            loudness_availability: loudness_avail,
            spectral,
            spectral_availability: spectral_avail,
            mfcc,
            mfcc_availability: mfcc_avail,
            embedding,
            embedding_availability: embedding_avail,
            analyzed_at_ms: r.get("analyzed_at_ms")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FeatureAvailability as FA, EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION};

    fn mem_db() -> Connection {
        // 与 M3 迁移系统同 schema（dev-dep aura-library）。
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(aura_library::db::schema::V1).unwrap();
        conn.execute_batch(aura_library::db::schema::V2).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
        // 外键依赖：为测试 track_id 1..=99 预置 tracks 行。
        for i in 1..100 {
            conn.execute(
                "INSERT INTO tracks (id, identity_key, audio_sig, file_size, modified_ms, date_added_ms, last_scanned_ms) VALUES (?1, ?2, ?3, 1, 0, 0, 0)",
                rusqlite::params![i, format!("ik-{i}"), format!("sig-{i}")],
            )
            .unwrap();
        }
        conn
    }

    fn sample(track_id: i64, content: &str) -> TrackAnalysis {
        let mut a = TrackAnalysis::skeleton(track_id, content);
        a.status = AnalysisStatus::Completed;
        a.tempo = Some(TempoAnalysis {
            bpm: 128.0,
            confidence: 0.9,
            first_beat_offset_seconds: 0.01,
            algorithm_version: 1,
        });
        a.tempo_availability = FA::Available;
        a.key = Some(KeyAnalysis {
            key: MusicalKey::Fs,
            mode: KeyMode::Minor,
            confidence: 0.7,
            camelot: Some("11A".into()),
            algorithm_version: 1,
        });
        a.key_availability = FA::Available;
        a.loudness_availability = FA::Unsupported;
        a.spectral = Some(SpectralFeatures {
            rms: 0.3,
            peak: 0.9,
            centroid_hz: 2400.0,
            rolloff_hz: 12000.0,
            brightness: 0.55,
            algorithm_version: 1,
        });
        a.spectral_availability = FA::Available;
        a.mfcc = Some(MfccFeatures {
            mean: vec![0.5; 13],
            variance: vec![0.1; 13],
            algorithm_version: 1,
        });
        a.mfcc_availability = FA::Available;
        a.embedding = Some(Embedding::normalized(
            vec![1.0; 16],
            EMBEDDING_MODEL_ID,
            EMBEDDING_MODEL_VERSION,
        ));
        a.embedding_availability = FA::Available;
        a.analyzed_at_ms = 42;
        a
    }

    #[test]
    fn save_load_roundtrip_all_features() {
        let conn = mem_db();
        let a = sample(1, "content-a");
        AnalysisStore::save(&conn, &a).unwrap();
        let back = AnalysisStore::load_by_track(&conn, 1).unwrap().unwrap();
        assert_eq!(back, a, "§30 全特征原子往返");
        let by_key = AnalysisStore::load_by_key(&conn, &a.analysis_key)
            .unwrap()
            .unwrap();
        assert_eq!(by_key, a);
    }

    #[test]
    fn save_is_atomic_replace() {
        let conn = mem_db();
        let a = sample(1, "content-a");
        AnalysisStore::save(&conn, &a).unwrap();
        // 同 track 重写（新内容）→ REPLACE 单行。
        let mut b = sample(1, "content-b");
        b.tempo = None;
        b.tempo_availability = FA::Failed;
        AnalysisStore::save(&conn, &b).unwrap();
        let back = AnalysisStore::load_by_track(&conn, 1).unwrap().unwrap();
        assert_eq!(back.content_identity, "content-b");
        assert_eq!(back.tempo_availability, FA::Failed);
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM track_analysis", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn partial_availability_preserved() {
        // §32：BPM 成功 + Key 失败 + Embedding Unsupported —— 不丢成功项。
        let conn = mem_db();
        let mut a = TrackAnalysis::skeleton(2, "c");
        a.status = AnalysisStatus::Completed;
        a.tempo = Some(TempoAnalysis {
            bpm: 90.0,
            confidence: 0.5,
            first_beat_offset_seconds: 0.0,
            algorithm_version: 1,
        });
        a.tempo_availability = FA::Available;
        a.key_availability = FA::Failed;
        a.embedding_availability = FA::Unsupported;
        a.loudness_availability = FA::Unavailable;
        a.spectral_availability = FA::Stale;
        a.mfcc_availability = FA::Available;
        a.mfcc = Some(MfccFeatures {
            mean: vec![1.0; 13],
            variance: vec![0.0; 13],
            algorithm_version: 1,
        });
        AnalysisStore::save(&conn, &a).unwrap();
        let back = AnalysisStore::load_by_track(&conn, 2).unwrap().unwrap();
        assert_eq!(back.tempo_availability, FA::Available);
        assert!(back.tempo.is_some());
        assert_eq!(back.key_availability, FA::Failed);
        assert!(back.key.is_none());
        assert_eq!(back.embedding_availability, FA::Unsupported);
        assert_eq!(back.loudness_availability, FA::Unavailable);
        assert_eq!(back.spectral_availability, FA::Stale);
        assert_eq!(back.mfcc_availability, FA::Available);
        assert!(back.mfcc.is_some());
    }

    #[test]
    fn replacement_detection_marks_stale() {
        // §20：同路径新内容 → 旧结果 Stale，且不删除（可追溯）。
        let conn = mem_db();
        AnalysisStore::save(&conn, &sample(3, "old-content")).unwrap();
        let changed = AnalysisStore::detect_replacement(&conn, 3, "new-content").unwrap();
        assert!(changed);
        let row = AnalysisStore::load_by_track(&conn, 3).unwrap().unwrap();
        assert_eq!(row.status, AnalysisStatus::Stale);
        assert_eq!(row.content_identity, "old-content");
        // 同内容重复检测 → false，不改状态。
        let changed = AnalysisStore::detect_replacement(&conn, 3, "old-content").unwrap();
        assert!(!changed);
        let row = AnalysisStore::load_by_track(&conn, 3).unwrap().unwrap();
        assert_eq!(row.status, AnalysisStatus::Stale, "已 Stale 不复活");
    }

    #[test]
    fn version_upgrade_marks_stale_below() {
        // §18：analysis_version 升级 → 旧结果 Stale（可重算判定）。
        let conn = mem_db();
        AnalysisStore::save(&conn, &sample(4, "c1")).unwrap(); // v1
        let n = AnalysisStore::mark_stale_below_version(&conn, 2).unwrap();
        assert_eq!(n, 1);
        let row = AnalysisStore::load_by_track(&conn, 4).unwrap().unwrap();
        assert_eq!(row.status, AnalysisStatus::Stale);
        // 幂等：再跑不再计新。
        let n = AnalysisStore::mark_stale_below_version(&conn, 2).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn metadata_only_change_hits_same_content() {
        // §21/§22：metadata/路径变化不重算——按 content_identity 命中。
        let conn = mem_db();
        AnalysisStore::save(&conn, &sample(5, "stable-content")).unwrap();
        let hit = AnalysisStore::find_valid_by_content(&conn, "stable-content").unwrap();
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().track_id, 5);
        assert!(!AnalysisStore::exists_for_track(&conn, 999).unwrap());
        // 非 completed 状态不命中。
        let mut pend = TrackAnalysis::skeleton(6, "pending-content");
        pend.status = AnalysisStatus::Pending;
        AnalysisStore::save(&conn, &pend).unwrap();
        assert!(
            AnalysisStore::find_valid_by_content(&conn, "pending-content")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn corrupt_enum_rejected() {
        // 未知状态串 → 明确错误（不静默映射）。
        let conn = mem_db();
        AnalysisStore::save(&conn, &sample(7, "c")).unwrap();
        conn.execute("UPDATE track_analysis SET status = 'weird'", [])
            .unwrap();
        // 未知状态串 → 读取被拒绝（包装为 Database 错误），不静默映射。
        assert!(AnalysisStore::load_by_track(&conn, 7).is_err());
    }

    #[test]
    fn embedding_integrity_enforced_on_load() {
        // §29：BLOB 尺寸与 dim 不符 → 读回时错误（不越界）。
        let conn = mem_db();
        AnalysisStore::save(&conn, &sample(8, "c")).unwrap();
        conn.execute(
            "UPDATE track_analysis SET embedding_blob = X'01020304' WHERE track_id = 8",
            [],
        )
        .unwrap();
        let err = AnalysisStore::load_by_track(&conn, 8).unwrap_err();
        assert!(matches!(err, StoreError::Database(_)));
    }
}
