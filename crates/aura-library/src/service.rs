//! Library service：root 管理 + 扫描编排（§14–§18/§67/§68）。
//! 查询 API 在本文件后续小节（§40–§42）。

use std::fmt;
use std::path::Path;
use std::time::Instant;

use rusqlite::params;

use crate::db::{repo, Database};
use crate::error::{ErrorCategory, LibraryError, Result};
use crate::scanner::{self, WalkEntry};
use crate::types::{CancelToken, RootId, ScanProgress, ScanResult};

/// 时间源：应用层注入（核心不读 wall-clock；测试注入固定/递增时钟）。
pub type NowMs = Box<dyn Fn() -> i64 + Send>;

/// 库服务。上层应用持有；时间由注入的时钟提供。
pub struct LibraryService {
    db: Database,
    now_ms: NowMs,
}

impl fmt::Debug for LibraryService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LibraryService")
            .field("db", &self.db)
            .field("now_ms", &"<clock>")
            .finish()
    }
}

impl LibraryService {
    /// 打开/创建库数据库。
    ///
    /// # Errors
    /// 数据库打开/迁移失败（§53）。
    pub fn open(db_path: &Path) -> Result<Self> {
        Ok(Self {
            db: Database::open(db_path)?,
            now_ms: Box::new(default_now_ms),
        })
    }

    /// 注入时间源（测试确定性 / 应用层自定义）。
    pub fn set_clock(&mut self, now_ms: NowMs) {
        self.now_ms = now_ms;
    }

    /// 测试/嵌入式：内存库。
    ///
    /// # Errors
    /// 同 [`LibraryService::open`]。
    pub fn open_in_memory() -> Result<Self> {
        Ok(Self {
            db: Database::open_in_memory()?,
            now_ms: Box::new(default_now_ms),
        })
    }

    /// 只读访问底层库（诊断工具）。
    #[must_use]
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// 写事务入口（crate 内查询扩展使用）。
    pub(crate) fn db_tx(&mut self) -> Result<rusqlite::Transaction<'_>> {
        self.db.transaction()
    }

    /// 添加 library root（§14：canonicalize 后落库）。
    ///
    /// # Errors
    /// 路径不存在/非目录 → [`LibraryError::Io`]。
    pub fn add_root(&mut self, path: &Path, now_ms: i64) -> Result<RootId> {
        let canonical = std::fs::canonicalize(path).map_err(|e| LibraryError::Io {
            path: Some(path.to_path_buf()),
            source: e,
        })?;
        if !canonical.is_dir() {
            return Err(LibraryError::Io {
                path: Some(path.to_path_buf()),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "library root is not a directory",
                ),
            });
        }
        let tx = self.db.transaction()?;
        let id = repo::insert_root(&tx, &canonical.to_string_lossy(), now_ms)?;
        tx.commit()?;
        Ok(id)
    }

    /// 当前 roots。
    ///
    /// # Errors
    /// 查询失败时返回 [`LibraryError::Database`]。
    pub fn roots(&self) -> Result<Vec<(RootId, String)>> {
        repo::get_roots(self.db.connection())
    }

    /// 全量/增量扫描一个 root（§15–§17）。
    ///
    /// - 未变化文件（size+mtime 相同）直接跳过，不重复解析/写库（§17）；
    /// - 单文件失败只计数（§2/§15），事务按文件粒度提交；
    /// - 取消在每个文件边界生效，已完成文件保持已提交（§67）；
    /// - 收尾 sweep 把本轮未见的路径停用（§47）。
    ///
    /// # Errors
    /// root 不可用或用户取消（[`LibraryError::Cancelled`]）。
    pub fn scan_root(
        &mut self,
        root: &Path,
        cancel: &CancelToken,
        mut on_progress: Option<&mut dyn FnMut(&ScanProgress)>,
    ) -> Result<ScanResult> {
        let canonical = std::fs::canonicalize(root).map_err(|e| LibraryError::Io {
            path: Some(root.to_path_buf()),
            source: e,
        })?;
        let started = Instant::now();
        let mut result = ScanResult::default();

        let entries = scanner::walk(&canonical, cancel, |p| {
            if let Some(cb) = on_progress.as_mut() {
                cb(p);
            }
        })?;
        result.files_seen = entries
            .iter()
            .filter(|e| matches!(e, WalkEntry::Candidate(_)))
            .count() as u64;

        let root_id = {
            let tx = self.db.transaction()?;
            repo::insert_root(&tx, &canonical.to_string_lossy(), (self.now_ms)())?
        };

        let mut seen = Vec::with_capacity(result.files_seen as usize);
        let mut processed = 0u64;
        for entry in &entries {
            if cancel.is_cancelled() {
                result.duration_ms =
                    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                return Err(LibraryError::Cancelled);
            }
            match entry {
                WalkEntry::Candidate(path) => {
                    let path_str = path.to_string_lossy().into_owned();
                    seen.push(path_str.clone());
                    processed += 1;
                    match self.import_if_changed(path, &path_str, root_id) {
                        Ok(ImportDelta::Unchanged) => result.files_skipped += 1,
                        Ok(ImportDelta::Created) => result.files_imported += 1,
                        Ok(ImportDelta::Updated) => result.files_updated += 1,
                        Err(e) => {
                            result.files_failed += 1;
                            result.errors.push((path.clone(), error_category(&e)));
                        }
                    }
                }
                WalkEntry::Failed { .. } => result.files_failed += 1,
                WalkEntry::ReparseSkipped(_) => result.files_skipped += 1,
            }
            if let Some(cb) = on_progress.as_mut() {
                cb(&ScanProgress {
                    discovered: result.files_seen,
                    processed,
                    imported: result.files_imported,
                    failed: result.files_failed,
                    current_path: match entry {
                        WalkEntry::Candidate(p)
                        | WalkEntry::Failed { path: p, .. }
                        | WalkEntry::ReparseSkipped(p) => p.clone(),
                    },
                });
            }
        }

        {
            let tx = self.db.transaction()?;
            result.files_deleted = repo::mark_missing(&tx, root_id, &seen, (self.now_ms)())?;
            tx.commit()?;
        }

        result.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        Ok(result)
    }

    /// 增量判断（§17）：path + size + mtime 都没变 → 不解析不写库。
    fn import_if_changed(
        &mut self,
        path: &Path,
        path_str: &str,
        root_id: RootId,
    ) -> Result<ImportDelta> {
        let meta = std::fs::metadata(path).map_err(|e| LibraryError::Io {
            path: Some(path.to_path_buf()),
            source: e,
        })?;
        let size = i64::try_from(meta.len()).unwrap_or(i64::MAX);
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0);

        {
            let tx = self.db.transaction()?;
            let unchanged: Option<(i64, i64)> = tx
                .query_row(
                    "SELECT t.file_size, t.modified_ms
                     FROM track_paths p JOIN tracks t ON t.id = p.track_id
                     WHERE p.path = ?1 AND p.is_active = 1 AND t.is_active = 1",
                    params![path_str],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .ok();
            if unchanged.is_some_and(|(s, m)| s == size && m == mtime) {
                return Ok(ImportDelta::Unchanged);
            }
        }

        let tx = self.db.transaction()?;
        let outcome = repo::import_file(&tx, root_id, path, (self.now_ms)())?;
        tx.commit()?;
        Ok(if outcome.created {
            ImportDelta::Created
        } else {
            ImportDelta::Updated
        })
    }
}

/// 单文件导入增量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportDelta {
    Unchanged,
    Created,
    Updated,
}

/// 默认时间源：系统时钟（应用层可 [`LibraryService::set_clock`] 覆盖）。
fn default_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// [`LibraryError`] → 稳定错误分类（§16）。
#[must_use]
pub fn error_category(e: &LibraryError) -> ErrorCategory {
    match e {
        LibraryError::Io { source, .. } => match source.kind() {
            std::io::ErrorKind::PermissionDenied => ErrorCategory::PermissionDenied,
            std::io::ErrorKind::NotFound => ErrorCategory::NotFound,
            _ => ErrorCategory::Unreadable,
        },
        LibraryError::Metadata { .. } => ErrorCategory::UnrecognizedFormat,
        LibraryError::PathOutsideRoot { .. } => ErrorCategory::OutsideRoot,
        _ => ErrorCategory::Unknown,
    }
}
