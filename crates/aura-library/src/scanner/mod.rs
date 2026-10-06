//! 本地目录扫描器（§11–§17）：迭代遍历、扩展名过滤、root 边界、
//! reparse point 策略、可取消、单文件失败不中断（§2/§15）。

use std::path::{Path, PathBuf};

use crate::error::ErrorCategory;
use crate::types::{CancelToken, ScanProgress};

/// 支持的音频扩展名（§11 + Charter §14：FLAC/MP3/OGG/WAV/M4A/ALAC；
/// m4a 的实际 codec 由 metadata 层判定，扫描层只做发现）。
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "ogg", "oga", "wav", "m4a", "alac", "aac", "aiff",
];

/// 扩展名快速过滤（§12：先过滤，format detection 留给 metadata 层）。
#[must_use]
pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// 遍历产出：候选音频文件或带分类的跳过/失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalkEntry {
    Candidate(PathBuf),
    /// 目录/文件 IO 失败（如 PermissionDenied）——不中断扫描。
    Failed {
        path: PathBuf,
        category: ErrorCategory,
    },
    /// reparse point：按策略跳过（D-041）。
    ReparseSkipped(PathBuf),
}

/// 迭代遍历一个 root（不跟随任何 symlink/junction，§13/D-041）。
///
/// - 目录与文件都通过 `DirEntry::file_type()` 判定（Windows 上不额外
///   follow reparse point）；
/// - `cancel` 在每个条目边界轮询（§67）；
/// - `on_progress` 可选；`current_path` 只进回调，不进日志（§68/§80）。
///
/// # Errors
/// 仅 root 本身不可用时返回 `Err`；子条目失败进 `WalkEntry::Failed`。
pub fn walk(
    root: &Path,
    cancel: &CancelToken,
    mut on_progress: impl FnMut(&ScanProgress),
) -> crate::error::Result<Vec<WalkEntry>> {
    let root_meta = std::fs::metadata(root).map_err(|e| crate::error::LibraryError::Io {
        path: Some(root.to_path_buf()),
        source: e,
    })?;
    if !root_meta.is_dir() {
        return Err(crate::error::LibraryError::Io {
            path: Some(root.to_path_buf()),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "library root is not a directory",
            ),
        });
    }

    let mut out = Vec::new();
    let mut discovered = 0u64;
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        if cancel.is_cancelled() {
            return Err(crate::error::LibraryError::Cancelled);
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(iter) => iter,
            Err(e) => {
                out.push(WalkEntry::Failed {
                    path: dir,
                    category: categorize_io(&e),
                });
                continue;
            }
        };
        for entry in entries {
            if cancel.is_cancelled() {
                return Err(crate::error::LibraryError::Cancelled);
            }
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    out.push(WalkEntry::Failed {
                        path: dir.clone(),
                        category: categorize_io(&e),
                    });
                    continue;
                }
            };
            let path = entry.path();
            // §13/D-041：不跟随 symlink/junction/reparse point。
            // DirEntry::file_type 用 readdir 数据，不触发最终组件的 follow。
            match entry.file_type() {
                Ok(ft) if ft.is_symlink() => {
                    out.push(WalkEntry::ReparseSkipped(path));
                    continue;
                }
                Ok(ft) if ft.is_dir() => dirs.push(path.clone()),
                Ok(ft) if ft.is_file() => {
                    if is_supported(&path) {
                        discovered += 1;
                        out.push(WalkEntry::Candidate(path.clone()));
                    }
                    // 不支持的扩展名：静默（不计入 seen/错误）。
                }
                Ok(_) => {} // 其他（设备文件等）：忽略
                Err(e) => out.push(WalkEntry::Failed {
                    path: path.clone(),
                    category: categorize_io(&e),
                }),
            }
            on_progress(&ScanProgress {
                discovered,
                processed: 0,
                imported: 0,
                failed: 0,
                current_path: path.clone(),
            });
        }
    }
    Ok(out)
}

/// Windows/Unix 通用的 IO 错误分类（§16）。
#[must_use]
pub fn categorize_io(e: &std::io::Error) -> ErrorCategory {
    match e.kind() {
        std::io::ErrorKind::PermissionDenied => ErrorCategory::PermissionDenied,
        std::io::ErrorKind::NotFound => ErrorCategory::NotFound,
        _ => ErrorCategory::Unreadable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CancelToken;

    fn temp_dir(tag: &str) -> PathBuf {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("aura-m3-walk-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn extension_filter() {
        assert!(is_supported(Path::new("a/b.flac")));
        assert!(is_supported(Path::new("A/B.FLAC")));
        assert!(is_supported(Path::new("x.m4a")));
        assert!(!is_supported(Path::new("x.png")));
        assert!(!is_supported(Path::new("x")));
        assert!(!is_supported(Path::new("x.mp3.exe")));
    }

    #[test]
    fn walk_finds_nested_supported_files_and_skips_reparse() {
        let root = temp_dir("nested");
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::write(root.join("a/one.flac"), b"x").unwrap();
        std::fs::write(root.join("a/b/two.MP3"), b"x").unwrap();
        std::fs::write(root.join("not_audio.txt"), b"x").unwrap();

        let cancel = CancelToken::new();
        let entries = walk(&root, &cancel, |_| {}).unwrap();
        let candidates: Vec<_> = entries
            .iter()
            .filter_map(|e| match e {
                WalkEntry::Candidate(p) => Some(p.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(candidates.len(), 2, "{candidates:?}");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn walk_symlink_dir_is_skipped_not_followed() {
        let root = temp_dir("sym");
        let outside = temp_dir("outside");
        std::fs::write(outside.join("secret.flac"), b"x").unwrap();
        // Windows 无需特权的 reparse point（junction，cmd mklink /J）；Unix 用 symlink。
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        #[cfg(windows)]
        {
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(root.join("link"))
                .arg(&outside)
                .status()
                .expect("mklink 可用");
            assert!(status.success(), "junction 创建失败");
        }

        let cancel = CancelToken::new();
        let entries = walk(&root, &cancel, |_| {}).unwrap();
        assert!(
            entries
                .iter()
                .any(|e| matches!(e, WalkEntry::ReparseSkipped(_))),
            "junction/symlink 目录必须记 ReparseSkipped: {entries:?}"
        );
        assert!(
            !entries
                .iter()
                .any(|e| matches!(e, WalkEntry::Candidate(p) if p.starts_with(&outside))),
            "绝不越界扫描 reparse point 目标"
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[test]
    fn walk_cancel_stops_promptly() {
        let root = temp_dir("cancel");
        std::fs::write(root.join("a.flac"), b"x").unwrap();
        let cancel = CancelToken::new();
        cancel.cancel();
        assert!(matches!(
            walk(&root, &cancel, |_| {}),
            Err(crate::error::LibraryError::Cancelled)
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn walk_missing_root_is_error() {
        let cancel = CancelToken::new();
        let r = walk(Path::new("Z:/definitely/not/here"), &cancel, |_| {});
        assert!(r.is_err());
    }
}
