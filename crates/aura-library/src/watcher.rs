//! 文件系统监听（§43–§46）：notify 事件 → 确定性合并 → rescan 候选。
//!
//! 架构（§44）：watcher 只产生候选，不碰数据库；应用层拿到候选后
//! 走 [`crate::service::LibraryService::scan_root`]（或定向重扫）。
//! 合并器注入时钟（测试确定性）；真实时钟由应用层装配。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::Watcher;

/// 合并后的候选类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    /// 新建或内容修改：重导入。
    Changed,
    /// 删除：sweep 停用。
    Removed,
    /// rename 事件的一侧；配对由扫描层经身份解析处理（§48/D-039）。
    Renamed,
}

/// 单个 rescan 候选。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RescanCandidate {
    pub path: PathBuf,
    pub kind: CandidateKind,
}

/// 确定性事件合并器（§45 debounce）。
///
/// 同一路径多事件折叠为一种（Removed > Renamed > Changed 优先级），
/// `flush` 由应用层按时间窗调用；窗口内不解析任何文件（§45）。
#[derive(Debug)]
pub struct EventCoalescer {
    window: Duration,
    pending: BTreeMap<PathBuf, CandidateKind>,
    window_start_ms: i64,
    last_event_ms: i64,
}

impl EventCoalescer {
    /// 窗口建议值 500ms（D-043）。
    #[must_use]
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            pending: BTreeMap::new(),
            window_start_ms: 0,
            last_event_ms: i64::MIN / 2,
        }
    }

    #[must_use]
    pub fn window(&self) -> Duration {
        self.window
    }

    /// 灌入一个原始事件（`now_ms` 由注入时钟提供）。
    pub fn push(&mut self, path: impl Into<PathBuf>, kind: CandidateKind, now_ms: i64) {
        let path = path.into();
        if !self.pending.contains_key(&path) {
            if self.pending.is_empty() {
                self.window_start_ms = now_ms;
            }
            self.last_event_ms = self.last_event_ms.max(now_ms);
        }
        let merged = match (self.pending.get(&path), kind) {
            (Some(CandidateKind::Removed), _) | (_, CandidateKind::Removed) => {
                CandidateKind::Removed
            }
            (Some(CandidateKind::Renamed), _) | (_, CandidateKind::Renamed) => {
                CandidateKind::Renamed
            }
            _ => CandidateKind::Changed,
        };
        self.pending.insert(path, merged);
    }

    /// 窗口是否已满（自窗口开始已过 window，或自最后事件已过 window）。
    #[must_use]
    pub fn should_flush(&self, now_ms: i64) -> bool {
        if self.pending.is_empty() {
            return false;
        }
        now_ms.saturating_sub(self.window_start_ms)
            >= self.window.as_millis().try_into().unwrap_or(i64::MAX)
            || now_ms.saturating_sub(self.last_event_ms)
                >= self.window.as_millis().try_into().unwrap_or(i64::MAX)
    }

    /// 取出全部候选（清空）。
    pub fn flush(&mut self) -> Vec<RescanCandidate> {
        let out: Vec<RescanCandidate> = self
            .pending
            .iter()
            .map(|(path, kind)| RescanCandidate {
                path: path.clone(),
                kind: *kind,
            })
            .collect();
        self.pending.clear();
        out
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// notify 原始事件 → 候选（供装配层使用；纯函数，可测）。
#[must_use]
pub fn classify_notify_event(event: &notify::Event) -> Vec<(PathBuf, CandidateKind)> {
    use notify::EventKind;
    let mut out = Vec::new();
    for path in &event.paths {
        let kind = match event.kind {
            EventKind::Create(_) => CandidateKind::Changed,
            EventKind::Modify(notify::event::ModifyKind::Name(_)) => CandidateKind::Renamed,
            EventKind::Modify(_) => CandidateKind::Changed,
            EventKind::Remove(_) => CandidateKind::Removed,
            EventKind::Access(_) | EventKind::Other | EventKind::Any => continue, // 读取/未知不触发重扫
        };
        out.push((path.clone(), kind));
    }
    out
}

/// 平台 watcher 装配：把 notify 事件灌入合并器。
/// 应用层持有返回的 watcher；事件线程只 push 内存结构，绝不写 DB（§44）。
///
/// # Errors
/// watcher 建立失败（root 不存在/权限）→ [`crate::error::LibraryError`]。
pub fn spawn_watcher(
    roots: &[PathBuf],
    coalescer: std::sync::Arc<std::sync::Mutex<EventCoalescer>>,
) -> crate::error::Result<notify::RecommendedWatcher> {
    let mut watcher =
        notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            let Ok(event) = res else {
                return; // watch 错误（如 root 被移除）：丢弃，等下轮全量扫描兜底
            };
            let now_ms = default_now_ms();
            if let Ok(mut c) = coalescer.lock() {
                for (path, kind) in classify_notify_event(&event) {
                    c.push(path, kind, now_ms);
                }
            }
        })
        .map_err(|e| crate::error::LibraryError::Io {
            path: None,
            source: std::io::Error::other(e.to_string()),
        })?;
    for root in roots {
        watcher
            .watch(root, notify::RecursiveMode::Recursive)
            .map_err(|e| crate::error::LibraryError::Io {
                path: Some(root.clone()),
                source: std::io::Error::other(e.to_string()),
            })?;
    }
    Ok(watcher)
}

fn default_now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// §73 的确定性等待：有界轮询条件（不依赖固定 sleep 猜测）。
pub fn wait_until(mut cond: impl FnMut() -> bool, timeout: Duration, poll: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(poll);
    }
    cond()
}

/// 路径是否位于任一 root 内（root boundary 双保险，§14/§62）。
#[must_use]
pub fn under_any_root(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|r| path.starts_with(r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalescer_folds_multiple_events_per_path() {
        let mut c = EventCoalescer::new(Duration::from_millis(500));
        c.push("a.flac", CandidateKind::Changed, 0);
        c.push("a.flac", CandidateKind::Changed, 10);
        c.push("a.flac", CandidateKind::Removed, 20);
        c.push("b.flac", CandidateKind::Changed, 25);
        assert_eq!(c.pending_count(), 2);
        let out = c.flush();
        let a = out
            .iter()
            .find(|r| r.path.as_path() == Path::new("a.flac"))
            .unwrap();
        assert_eq!(a.kind, CandidateKind::Removed, "Removed 优先（终态）");
        assert_eq!(out.len(), 2);
        assert_eq!(c.pending_count(), 0);
    }

    #[test]
    fn coalescer_window_logic_deterministic() {
        let mut c = EventCoalescer::new(Duration::from_millis(500));
        c.push("x.flac", CandidateKind::Changed, 1_000);
        assert!(!c.should_flush(1_200), "窗口未满");
        assert!(c.should_flush(1_501), "窗口已过");
        // 最后事件后过窗也 flush（慢速流）。
        let mut c2 = EventCoalescer::new(Duration::from_millis(500));
        c2.push("x.flac", CandidateKind::Changed, 1_000);
        c2.push("x.flac", CandidateKind::Changed, 1_400);
        assert!(c2.should_flush(1_901));
    }

    #[test]
    fn notify_event_classification() {
        use notify::event::{CreateKind, DataChange, ModifyKind, RenameMode};
        let event = notify::Event::new(notify::EventKind::Create(CreateKind::File))
            .add_path(PathBuf::from("new.flac"));
        assert_eq!(
            classify_notify_event(&event),
            vec![(PathBuf::from("new.flac"), CandidateKind::Changed)]
        );
        let rename = notify::Event::new(notify::EventKind::Modify(ModifyKind::Name(
            RenameMode::From,
        )))
        .add_path(PathBuf::from("old.flac"));
        assert_eq!(classify_notify_event(&rename)[0].1, CandidateKind::Renamed);
        let access = notify::Event::new(notify::EventKind::Access(notify::event::AccessKind::Read))
            .add_path(PathBuf::from("x.flac"));
        assert!(classify_notify_event(&access).is_empty(), "读取不触发");
        let data = notify::Event::new(notify::EventKind::Modify(ModifyKind::Data(DataChange::Any)))
            .add_path(PathBuf::from("y.flac"));
        assert_eq!(classify_notify_event(&data)[0].1, CandidateKind::Changed);
    }

    #[test]
    fn root_boundary_check() {
        let roots = vec![PathBuf::from("C:/music")];
        assert!(under_any_root(Path::new("C:/music/a/b.flac"), &roots));
        assert!(!under_any_root(Path::new("C:/other/a.flac"), &roots));
        assert!(
            !under_any_root(Path::new("C:/music-evil/a.flac"), &roots),
            "前缀歧义不算"
        );
    }
}
