//! 查询/过滤/恢复（§121/§122/§211/§223–§228）。
//!
//! - NULL 语义（§225）：unavailable（NULL）不匹配任何数值谓词——SQL
//!   比较天然排除 NULL，不当作 0。
//! - 数值语义（§227/§228）：bpm/energy 为 REAL 列，范围比较数值。
//! - key 规范化（§226）：查询层把缩写（"Am"）转换为 canonical
//!   （"A Minor"）后精确匹配。
//! - 重启恢复（§211）：Running → Pending（应用崩溃/退出后的状态修复）。
//! - 查询返回轻量 track_id 列表（§122 批量读取入口）；完整行经
//!   [`crate::store::AnalysisStore::load_by_track`]。SQL 全参数化
//!   （无字符串拼接注入面，§327）。

use rusqlite::types::Value;
use rusqlite::Connection;

use crate::store::StoreError;

/// 数值范围过滤（None = 不过滤）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RangeFilter {
    pub min: Option<f32>,
    pub max: Option<f32>,
}

/// 分析查询过滤器（§224：application layer 的 filter AST 读取这些字段；
/// 不新造 filter language）。
#[derive(Debug, Clone, Default)]
pub struct AnalysisFilter {
    pub bpm: RangeFilter,
    pub energy: RangeFilter,
    /// canonical key name（如 "Fs Minor"）；None = 不过滤。
    pub key: Option<String>,
    pub completed_only: bool,
}

/// §226：key 缩写/canonical 统一转换。
/// "Am"/"A minor"/"A Minor" → "A Minor"；"C"/"C major" → "C Major"；
/// "Db major" → "Cs Major"（§66 enharmonic canonical 升号）。
#[must_use]
pub fn canonical_key_name(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let (pc_str, minor) = if let Some(stripped) = s.strip_suffix('m') {
        // "Am" 风格：短音名 + m（排除 "minor" 全词——全词走下分支）。
        let base = stripped.trim_end();
        if base.len() <= 2 && base.chars().all(|c| c.is_ascii_alphabetic()) {
            (base.to_string(), true)
        } else {
            (s.to_string(), false)
        }
    } else {
        let lower = s.to_ascii_lowercase();
        if let Some(rest) = lower.strip_suffix(" minor") {
            (s[..rest.len()].to_string(), true)
        } else if let Some(rest) = lower.strip_suffix(" major") {
            (s[..rest.len()].to_string(), false)
        } else {
            (s.to_string(), false)
        }
    };
    let pitch = match pc_str.to_ascii_lowercase().as_str() {
        "c" => 0usize,
        "c#" | "cs" | "db" => 1,
        "d" => 2,
        "d#" | "ds" | "eb" => 3,
        "e" => 4,
        "f" => 5,
        "f#" | "fs" | "gb" => 6,
        "g" => 7,
        "g#" | "gs" | "ab" => 8,
        "a" => 9,
        "a#" | "as" | "bb" => 10,
        "b" => 11,
        _ => return None,
    };
    // canonical 升号表示（§66）。
    let names = [
        "C", "Cs", "D", "Ds", "E", "F", "Fs", "G", "Gs", "A", "As", "B",
    ];
    Some(format!(
        "{} {}",
        names[pitch],
        if minor { "Minor" } else { "Major" }
    ))
}

/// §211：重启恢复——上次 Running（应用退出/崩溃遗留）→ Pending。
///
/// # Errors
/// [`StoreError::Database`]。
pub fn recover_running_to_pending(conn: &Connection) -> Result<u64, StoreError> {
    let n = conn
        .execute(
            "UPDATE track_analysis SET status = 'pending' WHERE status = 'running'",
            [],
        )
        .map_err(|e| StoreError::Database(e.to_string()))?;
    Ok(n as u64)
}

/// §121/§224–§228：过滤查询返回 track_id（有界 §241：limit 钳制 1..=500）。
/// NULL（unavailable）不匹配数值谓词（§225：`IS NOT NULL` 显式保证）。
///
/// # Errors
/// [`StoreError::Database`]。
pub fn query_track_ids(
    conn: &Connection,
    filter: &AnalysisFilter,
    limit: i64,
) -> Result<Vec<i64>, StoreError> {
    let mut clauses: Vec<String> = vec!["1=1".to_string()];
    let mut values: Vec<Value> = Vec::new();
    if filter.completed_only {
        clauses.push("status = 'completed'".to_string());
    }
    if let Some(key) = &filter.key {
        clauses.push("musical_key || ' ' || key_mode = ?".to_string());
        values.push(Value::Text(key.clone()));
    }
    for (col, range) in [("bpm", filter.bpm), ("energy", filter.energy)] {
        if let Some(min) = range.min {
            clauses.push(format!("{col} IS NOT NULL AND {col} >= ?"));
            values.push(Value::Real(f64::from(min)));
        }
        if let Some(max) = range.max {
            clauses.push(format!("{col} IS NOT NULL AND {col} <= ?"));
            values.push(Value::Real(f64::from(max)));
        }
    }

    let sql = format!(
        "SELECT track_id FROM track_analysis WHERE {} ORDER BY track_id LIMIT {}",
        clauses.join(" AND "),
        limit.clamp(1, 500)
    );
    if std::env::var("AURA_QUERY_DEBUG").is_ok() {
        eprintln!("QUERYDBG sql={sql} values={values:?}");
    }
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StoreError::Database(e.to_string()))?;
    let mut out = Vec::new();
    let rows = stmt
        .query_map(rusqlite::params_from_iter(values), |r| r.get::<_, i64>(0))
        .map_err(|e| StoreError::Database(e.to_string()))?;
    for row in rows {
        let id = row.map_err(|e| StoreError::Database(e.to_string()))?;
        out.push(id);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AnalysisStatus, FeatureAvailability as FA, KeyAnalysis, KeyMode, MusicalKey,
        SpectralFeatures, TempoAnalysis, TrackAnalysis,
    };
    use crate::store::AnalysisStore;
    use rusqlite::params;

    fn mem_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(aura_library::db::schema::V1).unwrap();
        conn.execute_batch(aura_library::db::schema::V2).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
        for i in 1..100 {
            conn.execute(
                "INSERT INTO tracks (id, identity_key, audio_sig, file_size, modified_ms, date_added_ms, last_scanned_ms) VALUES (?1, ?2, ?3, 1, 0, 0, 0)",
                params![i, format!("ik-{i}"), format!("sig-{i}")],
            )
            .unwrap();
        }
        conn
    }

    fn with_features(
        id: i64,
        bpm: Option<f32>,
        energy: Option<f32>,
        key: Option<(MusicalKey, KeyMode)>,
    ) -> TrackAnalysis {
        let mut a = TrackAnalysis::skeleton(id, &format!("c{id}"));
        a.status = AnalysisStatus::Completed;
        if let Some(b) = bpm {
            a.tempo = Some(TempoAnalysis {
                bpm: b,
                confidence: 0.8,
                first_beat_offset_seconds: 0.0,
                algorithm_version: 1,
            });
            a.tempo_availability = FA::Available;
        }
        if let Some((k, m)) = key {
            a.key = Some(KeyAnalysis {
                key: k,
                mode: m,
                confidence: 0.9,
                camelot: None,
                algorithm_version: 1,
            });
            a.key_availability = FA::Available;
        }
        if let Some(e) = energy {
            a.spectral = Some(SpectralFeatures {
                rms: 0.2,
                peak: 0.5,
                centroid_hz: 1000.0,
                rolloff_hz: 5000.0,
                brightness: 0.3,
                energy: e,
                algorithm_version: 1,
            });
            a.spectral_availability = FA::Available;
        }
        a
    }

    #[test]
    fn canonical_key_name_normalization() {
        // §226：Am ↔ A Minor 等价（query 层转换）；Db → Cs（§66 升号）。
        assert_eq!(canonical_key_name("Am").as_deref(), Some("A Minor"));
        assert_eq!(canonical_key_name("A minor").as_deref(), Some("A Minor"));
        assert_eq!(canonical_key_name("A Minor").as_deref(), Some("A Minor"));
        assert_eq!(canonical_key_name("C major").as_deref(), Some("C Major"));
        assert_eq!(canonical_key_name("Db major").as_deref(), Some("Cs Major"));
        assert_eq!(canonical_key_name("Fs minor").as_deref(), Some("Fs Minor"));
        assert_eq!(canonical_key_name("X"), None);
    }

    #[test]
    fn bpm_range_query_null_not_matched() {
        // §225：unavailable（NULL）不当作 0/不匹配。
        let conn = mem_db();
        AnalysisStore::save(&conn, &with_features(1, Some(125.0), None, None)).unwrap();
        AnalysisStore::save(&conn, &with_features(2, None, None, None)).unwrap();

        let f = AnalysisFilter {
            bpm: RangeFilter {
                min: Some(120.0),
                max: Some(130.0),
            },
            ..Default::default()
        };
        assert_eq!(query_track_ids(&conn, &f, 100).unwrap(), vec![1]);
    }

    #[test]
    fn energy_numeric_and_key_canonical() {
        let conn = mem_db();
        AnalysisStore::save(
            &conn,
            &with_features(3, None, Some(0.7), Some((MusicalKey::Fs, KeyMode::Minor))),
        )
        .unwrap();
        AnalysisStore::save(&conn, &with_features(4, None, Some(0.3), None)).unwrap();

        let f = AnalysisFilter {
            energy: RangeFilter {
                min: Some(0.6),
                max: None,
            },
            ..Default::default()
        };
        assert_eq!(query_track_ids(&conn, &f, 100).unwrap(), vec![3]);

        // §226：查询 "Fs minor" 缩写 → canonical "Fs Minor" 命中。
        let f_fs = AnalysisFilter {
            key: canonical_key_name("Fs minor"),
            ..Default::default()
        };
        assert_eq!(query_track_ids(&conn, &f_fs, 100).unwrap(), vec![3]);
        let f_am = AnalysisFilter {
            key: canonical_key_name("A minor"),
            ..Default::default()
        };
        assert!(query_track_ids(&conn, &f_am, 100).unwrap().is_empty());
    }

    #[test]
    fn restart_recovery_running_to_pending() {
        // §211：Running 永久态禁止——恢复为 Pending（幂等）。
        let conn = mem_db();
        let mut a = with_features(10, Some(120.0), None, None);
        a.status = AnalysisStatus::Running;
        AnalysisStore::save(&conn, &a).unwrap();
        assert_eq!(recover_running_to_pending(&conn).unwrap(), 1);
        let row = AnalysisStore::load_by_track(&conn, 10).unwrap().unwrap();
        assert_eq!(row.status, AnalysisStatus::Pending);
        assert_eq!(recover_running_to_pending(&conn).unwrap(), 0);
    }

    #[test]
    fn limit_bounded() {
        // §241：limit 钳制。
        let conn = mem_db();
        for i in 1..50 {
            AnalysisStore::save(&conn, &with_features(i, Some(120.0), None, None)).unwrap();
        }
        let f = AnalysisFilter {
            bpm: RangeFilter {
                min: Some(119.0),
                max: Some(121.0),
            },
            ..Default::default()
        };
        assert_eq!(query_track_ids(&conn, &f, 10).unwrap().len(), 10);
        assert!(
            query_track_ids(&conn, &f, 0).unwrap().len() <= 500,
            "clamp 下界 1"
        );
    }
}
