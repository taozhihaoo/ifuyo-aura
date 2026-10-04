//! 分析调度器与仓库抽象（§95–§99/§102/§109–§112/§121/§140–§142，
//! D-104）：AnalysisScheduler + AnalysisRepository trait + InMemory
//! 实现。
//!
//! - 仓库抽象（§140）：save/load/mark_stale/list_pending——单元测试
//!   不依赖 SQLite（§141 InMemory）；SQLite 实现由 shell/应用层桥接
//!   （AnalysisStore 已有）。
//! - 调度（§95/§97）：单 worker 顺序执行（max_workers 有界=1，M8 下篇
//!   可扩）；优先级 User > Background（§96 两级）；取消（§102）；
//!   重试有界（§109/§110：损坏音频不重试）。
//! - 状态时间戳（§108）：注入 clock（VirtuaClock 或真实），分析结果
//!   不依赖 wall-clock。

use std::collections::HashMap;

use crate::domain::{AnalysisStatus, TrackAnalysis};

/// 优先级（§96 两级）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Background,
    UserRequested,
}

/// 仓库抽象（§140）。
pub trait AnalysisRepository {
    /// 保存/覆盖结果（all-or-nothing 由实现保证）。
    fn save_result(&mut self, analysis: &TrackAnalysis);
    /// 按 TrackId 读取。
    fn load_result(&self, track_id: i64) -> Option<TrackAnalysis>;
    /// §148 标记 Stale（不删除）。
    fn mark_stale(&mut self, track_id: i64);
    /// §140：列出待分析（Pending/Stale）track_id。
    fn list_pending(&self) -> Vec<i64>;
}

/// §141 In-memory 仓库（测试/调度器默认）。
#[derive(Debug, Default, Clone)]
pub struct InMemoryRepository {
    map: HashMap<i64, TrackAnalysis>,
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl AnalysisRepository for InMemoryRepository {
    fn save_result(&mut self, analysis: &TrackAnalysis) {
        self.map.insert(analysis.track_id, analysis.clone());
    }

    fn load_result(&self, track_id: i64) -> Option<TrackAnalysis> {
        self.map.get(&track_id).cloned()
    }

    fn mark_stale(&mut self, track_id: i64) {
        if let Some(a) = self.map.get_mut(&track_id) {
            a.status = AnalysisStatus::Stale;
        }
    }

    fn list_pending(&self) -> Vec<i64> {
        let mut ids: Vec<i64> = self
            .map
            .values()
            .filter(|a| matches!(a.status, AnalysisStatus::Pending | AnalysisStatus::Stale))
            .map(|a| a.track_id)
            .collect();
        ids.sort_unstable();
        ids
    }
}

/// 作业（§95 job model）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub track_id: i64,
    pub priority: Priority,
    /// 已重试次数（§109 有界）。
    pub retries: u8,
}

/// 最大重试（§109/§110：无限 retry 禁止）。
pub const MAX_RETRIES: u8 = 1;

/// 调度器（单进程内存版；worker 线程在 M8 下篇接 shell 任务队列）。
#[derive(Debug, Default)]
pub struct AnalysisScheduler {
    queue: Vec<Job>,
    /// 已取消集合（§102：出队时检查丢弃）。
    cancelled: std::collections::HashSet<i64>,
}

impl AnalysisScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 入队（§121 request_analysis）。
    pub fn enqueue(&mut self, track_id: i64, priority: Priority) {
        // 同 track 去重：保留更高优先级。
        if let Some(existing) = self.queue.iter_mut().find(|j| j.track_id == track_id) {
            if priority > existing.priority {
                existing.priority = priority;
            }
            return;
        }
        self.queue.push(Job {
            track_id,
            priority,
            retries: 0,
        });
    }

    /// §121 cancel_analysis。
    pub fn cancel(&mut self, track_id: i64) -> bool {
        let before = self.queue.len();
        self.queue.retain(|j| j.track_id != track_id);
        let removed = self.queue.len() != before;
        self.cancelled.insert(track_id);
        removed
    }

    /// §121 出队最高优先级作业（User > Background；FIFO 内部序）。
    /// 已取消的作业被静默丢弃。
    pub fn dequeue(&mut self) -> Option<Job> {
        let mut best: Option<usize> = None;
        for (i, j) in self.queue.iter().enumerate() {
            if self.cancelled.contains(&j.track_id) {
                continue;
            }
            best = match best {
                None => Some(i),
                Some(bi) if j.priority > self.queue[bi].priority => Some(i),
                other => other,
            };
        }
        let idx = best?;
        let job = self.queue.remove(idx);
        self.cancelled.remove(&job.track_id);
        Some(job)
    }

    /// §109：可重试失败（transient）→ 重排队（有界）；损坏音频不重试
    /// 的语义由调用方判定（§110 直接 Failed）。
    pub fn retry(&mut self, job: &Job) -> bool {
        if job.retries >= MAX_RETRIES {
            return false;
        }
        self.queue.push(Job {
            retries: job.retries + 1,
            ..job.clone()
        });
        true
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.queue
            .iter()
            .filter(|j| !self.cancelled.contains(&j.track_id))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FeatureAvailability, TempoAnalysis};

    fn completed(id: i64) -> TrackAnalysis {
        let mut a = TrackAnalysis::skeleton(id, &format!("c-{id}"));
        a.status = AnalysisStatus::Completed;
        a.tempo_availability = FeatureAvailability::Available;
        a.tempo = Some(TempoAnalysis {
            bpm: 120.0,
            confidence: 0.8,
            first_beat_offset_seconds: 0.0,
            algorithm_version: 1,
        });
        a
    }

    #[test]
    fn inmemory_repository_roundtrip_and_pending() {
        let mut repo = InMemoryRepository::new();
        assert!(repo.list_pending().is_empty());
        let mut pending = TrackAnalysis::skeleton(1, "c1");
        pending.status = AnalysisStatus::Pending;
        repo.save_result(&pending);
        repo.save_result(&completed(2));
        assert_eq!(repo.list_pending(), vec![1]);
        assert_eq!(repo.load_result(2).unwrap().track_id, 2);
        repo.mark_stale(2);
        assert_eq!(repo.load_result(2).unwrap().status, AnalysisStatus::Stale);
        // Stale → pending 列表（可重算判定）。
        assert!(repo.list_pending().contains(&2));
    }

    #[test]
    fn priority_ordering_user_first() {
        let mut s = AnalysisScheduler::new();
        s.enqueue(1, Priority::Background);
        s.enqueue(2, Priority::UserRequested);
        let job = s.dequeue().unwrap();
        assert_eq!(job.track_id, 2, "User 优先（§96）");
        assert_eq!(s.dequeue().unwrap().track_id, 1);
    }

    #[test]
    fn dedup_keeps_higher_priority() {
        let mut s = AnalysisScheduler::new();
        s.enqueue(5, Priority::Background);
        s.enqueue(5, Priority::UserRequested);
        assert_eq!(s.pending_count(), 1);
        assert_eq!(s.dequeue().unwrap().priority, Priority::UserRequested);
    }

    #[test]
    fn cancel_drops_job() {
        let mut s = AnalysisScheduler::new();
        s.enqueue(7, Priority::UserRequested);
        assert!(s.cancel(7));
        assert_eq!(s.dequeue(), None, "§102 取消的作业不出队");
        assert_eq!(s.pending_count(), 0);
        // 重复 cancel 幂等。
        assert!(!s.cancel(7));
    }

    #[test]
    fn retry_bounded() {
        let mut s = AnalysisScheduler::new();
        let job = Job {
            track_id: 9,
            priority: Priority::Background,
            retries: 0,
        };
        assert!(s.retry(&job));
        let retried = s.dequeue().unwrap();
        assert_eq!(retried.retries, 1);
        assert!(!s.retry(&retried), "§109 有界重试");
    }
}
