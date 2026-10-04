//! 稀有事件框架（§89–§94）：安静世界的可发现瞬间。
//!
//! - 事件由 `world seed + 日桶 + event 流` 决定（§91：可重放）。
//! - 真稀有（§90）：默认期望 ≈ 每 30 逻辑日一次量级。
//! - 事件有生命周期（§94）：fade_in → active → fade_out，自然过期，
//!   不积累；无奖励语义（§96：被发现即全部价值）。
//! - 事件日志（§93）只记 event_id/type/tick/params；按日志重放与
//!   直接模拟得到相同 logical state（§92）。

use serde::{Deserialize, Serialize};

use crate::rng::{streams, Rng};
use crate::seed::{stable_hash64, DreamscapeSeed};

use crate::env::TICKS_PER_DAY;

/// 稀有事件形态（§89 示例集合；克制、氛围性）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RareEventKind {
    /// 发光绽放。
    LuminousBloom,
    /// 缓漂光群。
    DriftingLights,
    /// 异常薄雾。
    UnusualMist,
    /// 慢速流光。
    SlowStreak,
    /// 萤光原野。
    FireflyField,
}

/// 单个稀有事件（逻辑态；视觉呈现在下篇渲染层）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RareEvent {
    pub event_id: u64,
    pub kind: RareEventKind,
    /// 触发 tick。
    pub start_tick: u64,
    /// 总时长（tick，含淡入淡出）。
    pub duration_ticks: u64,
    /// 淡入 tick（§94）。
    pub fade_in_ticks: u64,
    /// 淡出 tick（§94）。
    pub fade_out_ticks: u64,
    /// 事件中心（世界坐标）。
    pub x: f32,
    pub z: f32,
    /// 强度 [0.4, 0.9]（§73 精神：轻微，不喧宾夺主）。
    pub intensity: f32,
}

impl RareEvent {
    /// 触发时刻起的包络 [0,1]（§94：fade_in → active → fade_out）。
    #[must_use]
    pub fn envelope_at(&self, tick: u64) -> f32 {
        if tick < self.start_tick || tick >= self.start_tick + self.duration_ticks {
            return 0.0;
        }
        let t = tick - self.start_tick;
        let fin = self.fade_in_ticks.max(1);
        let fout = self.fade_out_ticks.max(1);
        let rising = (t as f32 / fin as f32).min(1.0);
        let falling = ((self.duration_ticks - t) as f32 / fout as f32).min(1.0);
        (rising.min(falling)).clamp(0.0, 1.0)
    }
}

/// 事件日志条目（§93 恰好四类字段）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EventEntry {
    pub event_id: u64,
    pub kind: RareEventKind,
    pub logical_tick: u64,
    /// 事件参数（坐标/强度/时长的紧凑编码；不含逐帧视觉位置）。
    pub params: EventParams,
}

/// 事件参数（可序列化的最小集合）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EventParams {
    pub x: f32,
    pub z: f32,
    pub intensity: f32,
    pub duration_ticks: u64,
}

/// 稀有度：每事件日桶的触发概率（期望 ≈ 30 逻辑日一次，§90）。
const RARE_CHANCE_PER_DAY: f32 = 1.0 / 30.0;
/// 单事件时长上限（tick）：2–6 逻辑小时。
const DURATION_MIN: u64 = 120;
const DURATION_MAX: u64 = 360;

/// 检查某「事件日桶」是否触发（确定性；同 seed 同桶同结果，§91）。
#[must_use]
fn event_for_bucket(seed: &DreamscapeSeed, day_bucket: u64) -> Option<RareEvent> {
    let mut r = Rng::from_stream_bucket(seed, streams::EVENT, day_bucket);
    if !r.chance(RARE_CHANCE_PER_DAY) {
        return None;
    }
    let kinds = [
        RareEventKind::LuminousBloom,
        RareEventKind::DriftingLights,
        RareEventKind::UnusualMist,
        RareEventKind::SlowStreak,
        RareEventKind::FireflyField,
    ];
    let kind = kinds[(r.next_u64() as usize) % kinds.len()];
    // 桶内触发时刻：桶起点 + 随机偏移（0..=1 日）。
    let start_tick = day_bucket * TICKS_PER_DAY + r.next_u64() % TICKS_PER_DAY;
    let duration_ticks = DURATION_MIN + r.next_u64() % (DURATION_MAX - DURATION_MIN);
    let event_id = stable_hash64(
        b"event-id/v1",
        &[seed.value.to_le_bytes(), day_bucket.to_le_bytes()].concat(),
    );
    Some(RareEvent {
        event_id,
        kind,
        start_tick,
        duration_ticks,
        fade_in_ticks: duration_ticks / 4,
        fade_out_ticks: duration_ticks / 4,
        x: r.next_f32_range(-200.0, 200.0),
        z: r.next_f32_range(-200.0, 200.0),
        intensity: r.next_f32_range(0.4, 0.9),
    })
}

/// 直接模拟：`from_tick..=to_tick` 间触发的全部事件（升序）。
/// 这是事件日志的「真值来源」；日志重放必须与其一致（§92）。
#[must_use]
pub fn simulate_events(seed: &DreamscapeSeed, from_tick: u64, to_tick: u64) -> Vec<RareEvent> {
    if to_tick < from_tick {
        return Vec::new();
    }
    let first_bucket = from_tick / TICKS_PER_DAY;
    let last_bucket = to_tick / TICKS_PER_DAY;
    (first_bucket..=last_bucket)
        .filter_map(|b| event_for_bucket(seed, b))
        .filter(|e| e.start_tick >= from_tick && e.start_tick <= to_tick)
        .collect()
}

/// 由事件日志重建「某 tick 的可见事件集合」（§92 重放路径；
/// 只消费日志，不重掷 RNG）。
#[must_use]
pub fn replay_visible(log: &[EventEntry], at_tick: u64) -> Vec<EventEntry> {
    log.iter()
        .filter(|e| at_tick >= e.logical_tick && at_tick < e.logical_tick + e.params.duration_ticks)
        .copied()
        .collect()
}

/// 模拟结果压缩为事件日志（记录路径：session 推进时调用）。
#[must_use]
pub fn to_log(events: &[RareEvent]) -> Vec<EventEntry> {
    events
        .iter()
        .map(|e| EventEntry {
            event_id: e.event_id,
            kind: e.kind,
            logical_tick: e.start_tick,
            params: EventParams {
                x: e.x,
                z: e.z,
                intensity: e.intensity,
                duration_ticks: e.duration_ticks,
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truly_rare() {
        // §90：一年逻辑时间（365 日）期望 ≈ 12 次；上限 2×（防每分钟彩蛋）。
        let seed = DreamscapeSeed::from_input("rare");
        let year = TICKS_PER_DAY * 365;
        let n = simulate_events(&seed, 0, year).len();
        assert!(n >= 1, "一年至少可被发现一次");
        assert!(n <= 40, "真稀有：{n}/年（期望 ≈12，§90）");
    }

    #[test]
    fn deterministic_by_seed() {
        let a_seed = DreamscapeSeed::from_input("evt-det");
        let a = simulate_events(&a_seed, 0, TICKS_PER_DAY * 400);
        let b = simulate_events(&a_seed, 0, TICKS_PER_DAY * 400);
        assert_eq!(a, b, "同 seed → 同事件集（§91）");
        let c = simulate_events(
            &DreamscapeSeed::from_input("evt-det2"),
            0,
            TICKS_PER_DAY * 400,
        );
        assert_ne!(a, c);
    }

    #[test]
    fn log_replay_equals_simulation() {
        // §92：模拟 → 日志 → 重放 → 相同可见集合。
        let seed = DreamscapeSeed::from_input("replay");
        let t0 = TICKS_PER_DAY * 10;
        let t1 = TICKS_PER_DAY * 210;
        let events = simulate_events(&seed, t0, t1);
        assert!(!events.is_empty(), "200 逻辑日窗口应覆盖到事件");
        let log = to_log(&events);
        for probe in [t0 + 60, t0 + TICKS_PER_DAY, t1 - 60] {
            let mut sim_visible: Vec<EventEntry> = events
                .iter()
                .filter(|e| probe >= e.start_tick && probe < e.start_tick + e.duration_ticks)
                .map(|e| EventEntry {
                    event_id: e.event_id,
                    kind: e.kind,
                    logical_tick: e.start_tick,
                    params: EventParams {
                        x: e.x,
                        z: e.z,
                        intensity: e.intensity,
                        duration_ticks: e.duration_ticks,
                    },
                })
                .collect();
            let mut replayed = replay_visible(&log, probe);
            sim_visible.sort_by_key(|e| e.event_id);
            replayed.sort_by_key(|e| e.event_id);
            assert_eq!(sim_visible, replayed, "probe tick={probe}");
        }
    }

    #[test]
    fn events_expire_no_accumulation() {
        // §94：事件都有 fade_in/out；同时可见数有界（不积累）。
        let seed = DreamscapeSeed::from_input("expire");
        let events = simulate_events(&seed, 0, TICKS_PER_DAY * 200);
        assert!(!events.is_empty());
        for e in &events {
            assert!(e.fade_in_ticks > 0 && e.fade_out_ticks > 0);
            assert_eq!(e.envelope_at(e.start_tick), 0.0, "起点淡入从 0");
            let peak = e.start_tick + e.fade_in_ticks;
            assert_eq!(e.envelope_at(peak), 1.0, "淡入后到峰值");
            assert_eq!(
                e.envelope_at(e.start_tick + e.duration_ticks),
                0.0,
                "结束归零（过期）"
            );
            assert_eq!(e.envelope_at(e.start_tick + e.duration_ticks * 10), 0.0);
            let intensity_sum: f32 = (0..e.duration_ticks)
                .map(|t| e.envelope_at(e.start_tick + t))
                .sum();
            assert!(intensity_sum.is_finite());
        }
        // 任意时刻可见 ≤ 3（时长 ≤ 6h << 事件间隔期望 30 日）。
        for probe in (0..TICKS_PER_DAY * 200).step_by(997) {
            let visible = events
                .iter()
                .filter(|e| probe >= e.start_tick && probe < e.start_tick + e.duration_ticks)
                .count();
            assert!(visible <= 3, "同时可见数有界（§94 不积累）");
        }
    }

    #[test]
    fn params_bounded_and_serializable() {
        let seed = DreamscapeSeed::from_input("params");
        let events = simulate_events(&seed, 0, TICKS_PER_DAY * 100);
        for e in &events {
            assert!((0.4..=0.9).contains(&e.intensity), "强度轻微（§73 精神）");
            assert!(e.x.abs() <= 200.0 && e.z.abs() <= 200.0);
            let entry = to_log(&[*e])[0];
            let json = serde_json::to_string(&entry).unwrap();
            let back: EventEntry = serde_json::from_str(&json).unwrap();
            assert_eq!(entry, back);
        }
    }
}
