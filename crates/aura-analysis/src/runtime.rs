//! AnalysisService 运行时（§205–§214/§244–§246，D-105）：
//! 有界 batch API + analysis_key 去重 + stale job 保护（revision）+
//! panic 边界 + worker 生命周期。
//!
//! - 去重（§207）：同 track 已有 Running 作业 → 新请求 join（不重复入队）。
//! - Stale 保护（§208–§210）：作业提交时记录 analysis_key；完成落库前
//!   比对仓库当前 key——不一致（版本已升级/内容已替换）→ 丢弃（不覆盖
//!   新版本结果）。
//! - Panic 边界（§244/§245）：单首坏歌曲的 panic 被 catch_unwind 捕获
//!   → 该 job Failed，服务不死。
//! - 特征独立（§205）：管线层已按特征各自 availability 落库——BPM 失败
//!   不影响 LUFS（pipeline.rs 单遍提取各自 try 语义）。
//! - 生命周期（§213）：Idle/Running/Stopping/Stopped；shutdown 后不再
//!   接受新作业（§212/§155）。

use crate::domain::{AnalysisError, AnalysisStatus, TrackAnalysis};
use crate::pipeline::{self, CancelToken};
use crate::scheduler::{AnalysisRepository, Priority};

/// Worker 生命周期状态（§213）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WorkerState {
    #[default]
    Idle,
    Running,
    Stopping,
    Stopped,
}

/// 单个 active 作业的守卫信息（§208/§210）。
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveJob {
    pub track_id: i64,
    /// 提交时派生的 analysis_key（§23 完整键）。
    pub analysis_key: String,
    /// 优先级（§96）。
    pub priority: Priority,
}

/// batch API 上限（§206/§241：bounded）。
pub const MAX_BATCH: usize = 500;

/// 分析服务（单 worker 顺序模型 §97；M8 下篇 shell 侧桥接线程/队列）。
#[derive(Debug, Default)]
pub struct AnalysisService {
    cancel: CancelToken,
    /// §207：当前 active 作业（Some = 有作业运行，同 track 新请求 join）。
    active: std::sync::Mutex<Option<ActiveJob>>,
    state: std::sync::Mutex<WorkerState>,
    stopped: std::sync::Mutex<bool>,
}

/// 分析输入抽象（§139：不让 feature 提取知 SQL；测试可注入）。
pub trait AnalyzerFn {
    /// 执行分析（解码→特征→TrackAnalysis）。
    ///
    /// # Errors
    /// [`AnalysisError`]。
    fn analyze(
        &self,
        stream: Box<dyn aura_core::decoder::AudioByteStream>,
        track_id: i64,
        content_identity: &str,
        cancel: &CancelToken,
    ) -> Result<TrackAnalysis, AnalysisError>;
}

impl AnalyzerFn for () {
    fn analyze(
        &self,
        stream: Box<dyn aura_core::decoder::AudioByteStream>,
        track_id: i64,
        content_identity: &str,
        cancel: &CancelToken,
    ) -> Result<TrackAnalysis, AnalysisError> {
        pipeline::analyze_stream(
            stream,
            track_id,
            content_identity,
            &crate::embedding::SpectralStatsProvider,
            cancel,
            || 0,
        )
    }
}

/// §207：同 track 的 active 作业去重（join 语义——调用方等待现有作业）。
#[must_use]
pub fn dedup_active(active: &[ActiveJob], track_id: i64) -> bool {
    active.iter().any(|j| j.track_id == track_id)
}

/// §208–§210：stale job 保护——作业完成时当前 analysis_key 与作业键
/// 不同 → 结果丢弃（v1 迟到不覆盖 v2）。
#[must_use]
pub fn result_is_current(job_key: &str, current_key: Option<&str>) -> bool {
    match current_key {
        // 仓库无记录/仍是同键 → 可写。
        None => true,
        Some(k) => k == job_key,
    }
}

/// §244/§245：panic 边界——单 job panic 转为 Failed 错误（服务不死）。
///
/// # Errors
/// panic → [`AnalysisError::NumericalSafety`]（panic 隔离语义）；
/// 正常错误原样。
pub fn run_job_isolated<F>(job: F) -> Result<TrackAnalysis, AnalysisError>
where
    F: FnOnce() -> Result<TrackAnalysis, AnalysisError>,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
    match result {
        Ok(r) => r,
        Err(_) => Err(AnalysisError::NumericalSafety {
            feature: "worker-panic-boundary".to_string(),
        }),
    }
}

impl AnalysisService {
    pub fn new() -> Self {
        Self::default()
    }

    /// §213 生命周期读取。
    #[must_use]
    pub fn state(&self) -> WorkerState {
        *self.state.lock().expect("service state")
    }

    /// §212/§155：shutdown——Stopped 后拒绝新作业；取消活动作业。
    pub fn shutdown(&self) {
        *self.stopped.lock().expect("service stopped") = true;
        *self.state.lock().expect("service state") = WorkerState::Stopped;
        self.cancel.cancel();
    }

    /// §206/§241：有界 batch 入口——id 去重、上限 [`MAX_BATCH`]、
    /// User 优先级。返回实际入队 id 列表。
    #[must_use]
    pub fn analyze_tracks(&self, track_ids: &[i64]) -> Vec<i64> {
        if *self.stopped.lock().expect("service stopped") {
            return Vec::new();
        }
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for id in track_ids.iter().take(MAX_BATCH).copied() {
            if seen.insert(id) {
                out.push(id);
            }
        }
        out
    }

    /// 取消令牌透出（§102 batch cancellable）。
    #[must_use]
    pub fn cancel_token(&self) -> &CancelToken {
        &self.cancel
    }

    /// §207：同 track 已有 active 作业 → true（join 语义，不重复启动）。
    #[must_use]
    pub fn is_duplicate(&self, track_id: i64) -> bool {
        let guard = self.active.lock().expect("service active");
        match guard.as_ref() {
            Some(job) => dedup_active(std::slice::from_ref(job), track_id),
            None => false,
        }
    }

    /// §208：登记 active 作业（提交时调用）。
    pub fn set_active(&self, job: ActiveJob) {
        *self.active.lock().expect("service active") = Some(job);
    }

    /// §208：清除 active 作业（完成/取消时调用）。
    pub fn clear_active(&self) {
        *self.active.lock().expect("service active") = None;
    }
}

/// 作业完成落库门（§208–§210/§31）：校验 revision/key + 结果有效性，
/// 通过则写入仓库。返回是否写入。
pub fn commit_result(
    repo: &mut dyn AnalysisRepository,
    job: &ActiveJob,
    result: &TrackAnalysis,
) -> bool {
    // §208：stale protection。
    let current_key = repo.load_result(job.track_id).map(|a| a.analysis_key);
    if !result_is_current(&job.analysis_key, current_key.as_deref()) {
        return false;
    }
    // §335/§336：持久化前校验。
    if result.status == AnalysisStatus::Completed && !result.is_valid_completed() {
        return false;
    }
    repo.save_result(result);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ANALYSIS_VERSION, EMBEDDING_MODEL_ID, EMBEDDING_MODEL_VERSION};
    use crate::embedding::{EmbeddingInput, EmbeddingProvider, SpectralStatsProvider};
    use crate::key::analysis_key as key_derive;
    use crate::scheduler::InMemoryRepository;

    #[test]
    fn dedup_active_joins() {
        // §207：同 track Running → 第二请求 join。
        let active = vec![ActiveJob {
            track_id: 5,
            analysis_key: "k5".into(),
            priority: Priority::UserRequested,
        }];
        assert!(dedup_active(&active, 5));
        assert!(!dedup_active(&active, 6));
    }

    #[test]
    fn stale_late_v1_cannot_overwrite_v2() {
        // §208–§210：v1 迟到 → 丢弃；v2 保持权威。
        let mut repo = InMemoryRepository::new();
        let v2 = TrackAnalysis::skeleton(1, "content-1");
        repo.save_result(&v2);
        let late_v1_job = ActiveJob {
            track_id: 1,
            analysis_key: key_derive(
                "content-1-OLD",
                1,
                EMBEDDING_MODEL_ID,
                EMBEDDING_MODEL_VERSION,
            ),
            priority: Priority::Background,
        };
        let mut v1_result = TrackAnalysis::skeleton(1, "content-1-OLD");
        v1_result.status = AnalysisStatus::Completed;
        assert!(
            !commit_result(&mut repo, &late_v1_job, &v1_result),
            "v1 迟到被丢弃"
        );
        assert_eq!(
            repo.load_result(1).unwrap().analysis_key,
            v2.analysis_key,
            "v2 保持权威"
        );
    }

    #[test]
    fn current_result_commits() {
        let mut repo = InMemoryRepository::new();
        let content = "content-A";
        let job = ActiveJob {
            track_id: 2,
            analysis_key: key_derive(
                content,
                ANALYSIS_VERSION,
                EMBEDDING_MODEL_ID,
                EMBEDDING_MODEL_VERSION,
            ),
            priority: Priority::UserRequested,
        };
        let result = TrackAnalysis::skeleton(2, content);
        assert!(commit_result(&mut repo, &job, &result));
        assert!(repo.load_result(2).is_some());
    }

    #[test]
    fn invalid_completed_rejected_at_commit() {
        // §335/§336：Completed 但全特征 Pending → 拒绝持久化。
        let mut repo = InMemoryRepository::new();
        let content = "content-B";
        let job = ActiveJob {
            track_id: 3,
            analysis_key: key_derive(
                content,
                ANALYSIS_VERSION,
                EMBEDDING_MODEL_ID,
                EMBEDDING_MODEL_VERSION,
            ),
            priority: Priority::Background,
        };
        let mut result = TrackAnalysis::skeleton(3, content);
        result.status = AnalysisStatus::Completed;
        assert!(!commit_result(&mut repo, &job, &result));
        assert!(repo.load_result(3).is_none());
    }

    #[test]
    fn panic_boundary_catches() {
        // §244/§245：job panic → 错误而非服务崩溃。
        let err = run_job_isolated(|| -> Result<TrackAnalysis, AnalysisError> {
            panic!("bad track");
        })
        .unwrap_err();
        assert!(matches!(err, AnalysisError::NumericalSafety { .. }));
    }

    #[test]
    fn lifecycle_and_shutdown() {
        let svc = AnalysisService::new();
        assert_eq!(svc.state(), WorkerState::Idle);
        assert_eq!(
            svc.analyze_tracks(&[1, 1, 2, 3]),
            vec![1, 2, 3],
            "§206 去重"
        );
        assert_eq!(
            svc.analyze_tracks(&(0..(MAX_BATCH as i64 + 10)).collect::<Vec<i64>>())
                .len(),
            MAX_BATCH,
            "§241 有界"
        );
        svc.shutdown();
        assert_eq!(svc.state(), WorkerState::Stopped);
        assert!(
            svc.analyze_tracks(&[9]).is_empty(),
            "§212 shutdown 后拒新作业"
        );
    }

    /// §246：provider 失败隔离——嵌入失败不产生 partial 污染。
    #[test]
    fn embedding_failure_isolated() {
        let provider = SpectralStatsProvider;
        let err = provider
            .embed(&EmbeddingInput::new(vec![], 22_050))
            .unwrap_err();
        assert!(matches!(err, AnalysisError::TooShort { .. }));
        // 其它 provider 调用不受影响。
        let ok = provider
            .embed(&EmbeddingInput::new(vec![0.1; 40_000], 22_050))
            .is_ok();
        assert!(ok, "后续调用正常（§246 隔离）");
    }
}
