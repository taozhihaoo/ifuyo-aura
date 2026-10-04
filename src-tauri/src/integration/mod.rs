//! Windows Integration 协调层（M10 上篇，§5–§8/§138–§155）：
//! Playback Authority 的唯一 system 快照出口——SMTC 与 Tray 同源消费
//! （§139）。本模块是**纯逻辑**（无 Windows API），平台适配器实现
//! [`IntegrationSink`]；全部行为有确定性 mock 测试。
//!
//! - §7/§8：SMTC/Tray 不是第二套播放器——命令只进入 Playback
//!   Authority（AudioHandle / 前端 queue 桥）。
//! - §37/§148/§149：revision 单调；旧 snapshot 迟到不得覆盖新。
//! - §91：只有 track/state/queue-boundary/language 变化才更新 sink。
//! - §141/§144/§182–§184：position 走独立低频 tick（500ms，D-124），
//!   仅 Playing；Paused/Stopped 停止。
//! - §133–§137：字符串限长（char 边界，保留 Unicode）+ 无伪 metadata。

pub mod single_instance;
#[cfg(windows)]
pub mod smtc;
pub mod tray;
pub mod worker;

use std::path::Path;

/// §134 字符串上限（Windows API + UI 需求；D-124）。
pub const MAX_TITLE_CHARS: usize = 256;
pub const MAX_ARTIST_CHARS: usize = 256;
pub const MAX_ALBUM_CHARS: usize = 256;
pub const MAX_STATION_CHARS: usize = 128;

/// §144/§144 D-124：position tick 节流间隔（秒）。
pub const POSITION_TICK_INTERVAL_SECS: f64 = 0.5;

/// 播放状态名（与 PlayerSnapshot.state 同词汇）。
pub type StateName = String;

/// §138/§40 统一系统快照（不可变；SMTC 与 Tray 同源）。
#[derive(Debug, Clone, PartialEq)]
pub struct SystemSnapshot {
    /// §148 monotonic revision（来自 Playback Authority revision）。
    pub revision: u64,
    pub track_id: Option<i64>,
    /// §43：已应用 fallback（空 → 文件名 stem）后的展示标题。
    pub title: String,
    /// §137：缺失 = None（不伪造 "Unknown Artist"）。
    pub artist: Option<String>,
    pub album: Option<String>,
    /// §45：缺失回退 artist；仍无 → None（策略一致，D-124）。
    pub album_artist: Option<String>,
    /// §49：应用内 artwork 引用（字节由 coordinator 提供，适配器不碰
    /// 用户文件系统 §48）。
    pub artwork: Option<Vec<u8>>,
    pub playback_state: StateName,
    pub position_seconds: f64,
    /// §29/§30：未知 = None（不得填 0 冒充零秒）。
    pub duration_seconds: Option<f64>,
    pub seekable: bool,
    /// §28：live 不伪造 duration/position。
    pub live: bool,
    /// §65/§66 queue 边界（前端上报）。
    pub has_previous: bool,
    pub has_next: bool,
}

/// 平台适配器出口（SMTC / Tray 各自实现；coordinator 决定何时调用）。
pub trait IntegrationSink {
    /// §37 边界提交：track 变化 = metadata+artwork+buttons+status 一次。
    fn apply_track(&mut self, snap: &SystemSnapshot);
    /// §57/§61/§62 仅状态变化。
    fn apply_status(&mut self, snap: &SystemSnapshot);
    /// §141 低频 position（仅 Playing 时会被调用）。
    fn apply_position(&mut self, position_seconds: f64, duration_seconds: Option<f64>);
}

/// §69/§70/§71 seek 请求验证结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekDecision {
    /// 在 [0, duration] 内（已按边界 clamp）。
    Seek(f64),
    /// 不可 seek / 未知时长 / 非有限 → 安全忽略（§27/§70）。
    Ignore,
}

/// 变化检测键（§91/§95：只有真实变化才更新 sink）。
#[derive(Debug, Clone, PartialEq)]
struct TrackKey {
    track_id: Option<i64>,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    album_artist: Option<String>,
    has_artwork: bool,
}

impl TrackKey {
    fn of(s: &SystemSnapshot) -> Self {
        Self {
            track_id: s.track_id,
            title: s.title.clone(),
            artist: s.artist.clone(),
            album: s.album.clone(),
            album_artist: s.album_artist.clone(),
            has_artwork: s.artwork.is_some(),
        }
    }
}

/// 系统集成协调器（单线程拥有；SMTC/Tray 事件经 channel 汇入）。
#[derive(Debug, Default)]
pub struct Coordinator {
    seen_revision: u64,
    track_key: Option<TrackKey>,
    last_state: Option<StateName>,
    last_buttons: (bool, bool, bool),
    last_position_tick: Option<f64>,
    /// §55/§196 ICY 去重（相同 StreamTitle 不重复提交）。
    last_stream_title: Option<String>,
}

impl Coordinator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 提交快照：内部判定 track/status/buttons 变化并调用 sink（§91）。
    /// revision 旧于已见 → 忽略（§148/§149）。
    pub fn offer(&mut self, snap: &SystemSnapshot, sink: &mut dyn IntegrationSink) {
        if snap.revision < self.seen_revision {
            return; // §149 迟到旧快照丢弃
        }
        self.seen_revision = snap.revision;

        let key = TrackKey::of(snap);
        let track_changed = self.track_key.as_ref() != Some(&key);
        if track_changed {
            self.track_key = Some(key);
            sink.apply_track(snap); // §37 一致边界
            self.last_state = Some(snap.playback_state.clone());
            self.last_buttons = (snap.seekable, snap.has_previous, snap.has_next);
            self.last_position_tick = None; // 新 track 立即允许一次 position tick
            return;
        }

        let buttons = (snap.seekable, snap.has_previous, snap.has_next);
        let buttons_changed = buttons != self.last_buttons;
        if buttons_changed {
            self.last_buttons = buttons;
        }
        let state_changed = self.last_state.as_ref() != Some(&snap.playback_state);
        if state_changed {
            self.last_state = Some(snap.playback_state.clone());
        }
        // buttons 变化伴随 track 内容不变 → 仍需把 enabled 态推给系统；
        // 复用 apply_track 边界（metadata 不变，只是 buttons 更新载体）。
        if buttons_changed {
            sink.apply_track(snap);
        } else if state_changed {
            sink.apply_status(snap);
        }
    }

    /// §141/§144 低频 position tick（ Playing 之外的调用被忽略）。
    /// 返回 true = 已提交给 sink。
    pub fn position_tick(
        &mut self,
        snap: &SystemSnapshot,
        now_secs: f64,
        sink: &mut dyn IntegrationSink,
    ) -> bool {
        if snap.playback_state != "playing" {
            return false; // §182/§183/§184 Paused/Stopped 停止更新
        }
        if !position_is_sane(snap.position_seconds) {
            return false; // §234 非有限拒绝
        }
        if let Some(last) = self.last_position_tick {
            if (now_secs - last) < POSITION_TICK_INTERVAL_SECS {
                return false; // §144 节流
            }
        }
        self.last_position_tick = Some(now_secs);
        sink.apply_position(snap.position_seconds, snap.duration_seconds);
        true
    }

    /// §69/§70：SMTC/托盘 seek 请求验证（外部输入 = untrusted §232）。
    #[must_use]
    pub fn validate_seek(&self, snap: &SystemSnapshot, requested_seconds: f64) -> SeekDecision {
        if !snap.seekable {
            return SeekDecision::Ignore; // §27 non-seekable 安全忽略
        }
        if !requested_seconds.is_finite() || requested_seconds < 0.0 {
            return SeekDecision::Ignore; // §70 非法
        }
        match snap.duration_seconds {
            Some(d) if d.is_finite() && d > 0.0 => SeekDecision::Seek(requested_seconds.min(d)),
            _ => SeekDecision::Ignore, // §29 未知时长不给假 seek 面
        }
    }

    /// §54/§55/§196：ICY StreamTitle 更新（同值去重 = coalesce）。
    /// 返回 true = 有变化需提交（调用方组装新快照后 offer）。
    pub fn stream_title_changed(&mut self, artist: &str, title: &str) -> bool {
        let key = format!("{artist}\u{1f}{title}");
        if self.last_stream_title.as_ref() == Some(&key) {
            return false;
        }
        self.last_stream_title = Some(key);
        true
    }
}

/// §133/§134/§135：限长（char 边界截断，不产生半个 emoji）+ 去控制
/// 字符（保留全部可打印 Unicode：CJK/emoji/特殊符号 §135/§136）。
#[must_use]
pub fn sanitize(input: Option<&str>, max_chars: usize) -> Option<String> {
    let raw = input?;
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control() || *c == '\u{9}')
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return None;
    }
    Some(cleaned.chars().take(max_chars).collect::<String>())
}

/// §43 metadata fallback：空标题 → 文件名 stem（不造第二套 parser）。
#[must_use]
pub fn fallback_title<'a>(title: Option<&'a str>, display: &'a str) -> String {
    let t = sanitize(title, MAX_TITLE_CHARS);
    if let Some(t) = t {
        return t;
    }
    let stem = Path::new(display)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| display.to_string());
    sanitize(Some(&stem), MAX_TITLE_CHARS).unwrap_or_else(|| "Unknown".into())
}

/// §234：position 合法性（有限 + 非负）。
#[must_use]
pub fn position_is_sane(secs: f64) -> bool {
    secs.is_finite() && secs >= 0.0
}

/// §32/§233：秒 → 100ns ticks（WinRT TimeSpan），checked、无负值。
#[must_use]
pub fn seconds_to_ticks(secs: f64) -> u64 {
    if !position_is_sane(secs) {
        return 0;
    }
    (secs * 10_000_000.0).round() as u64
}

/// §32/§233：帧 + 采样率 → 秒（checked；除零/溢出安全）。
#[must_use]
pub fn frames_to_seconds(frames: u64, sample_rate: u64) -> f64 {
    if sample_rate == 0 {
        return 0.0;
    }
    (frames as f64) / (sample_rate as f64)
}

// ---------- 系统命令（§7/§8/§72/§73/§155） ----------

/// 外部系统命令（SMTC 按钮）；Next/Previous 桥接前端 queue（§23/§24）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemCommand {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
}

// ---------- 共享槽（应用层 IPC 写入；worker 读取） ----------

/// 当前曲目元数据快照来源（play_track/play_input 在命令线程写入；
/// §41：SMTC 适配器绝不自查 SQLite——由应用层提供）。
#[derive(Debug, Clone, Default)]
pub struct CurrentTrackMeta {
    pub track_id: Option<i64>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub artwork_id: Option<i64>,
}

pub type SharedMeta = std::sync::Arc<std::sync::Mutex<Option<CurrentTrackMeta>>>;
/// (has_previous, has_next)（§65/§66 前端 queue 上报）。
pub type SharedQueue = std::sync::Arc<std::sync::Mutex<(bool, bool)>>;
/// M4 ICY 事件接收端（radio 播放时挂入；§53–§56）。
pub type SharedIcy = std::sync::Arc<
    std::sync::Mutex<Option<std::sync::mpsc::Receiver<aura_sources::model::StreamMetadataEvent>>>,
>;

/// §225–§227 本地 diagnostics（不作 source of truth）。
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationDiagnostics {
    pub smtc_available: bool,
    pub smtc_state: String,
    pub last_revision: u64,
    pub last_error: Option<String>,
    pub tray_initialized: bool,
}
pub type SharedDiag = std::sync::Arc<std::sync::Mutex<IntegrationDiagnostics>>;

/// 应用层共享槽集合（AppState 持有；smoke 可空转）。
#[derive(Clone, Default)]
pub struct Shared {
    pub meta: SharedMeta,
    pub queue: SharedQueue,
    pub icy: SharedIcy,
    pub diag: SharedDiag,
    /// worker 事件入口（spawn 后接入；IPC 用它推动即时刷新）。
    pub events: std::sync::Arc<std::sync::Mutex<Option<std::sync::mpsc::Sender<Event>>>>,
}

impl Shared {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// §65/§153：前端上报 queue 边界。
    pub fn report_queue(&self, has_previous: bool, has_next: bool) {
        *self.queue.lock().expect("queue") = (has_previous, has_next);
        self.notify(Event::Refresh);
    }

    pub fn notify(&self, ev: Event) {
        if let Ok(tx) = self.events.lock() {
            if let Some(tx) = tx.as_ref() {
                let _ = tx.send(ev);
            }
        }
    }
}

/// worker 入站事件（单通道；SMTC 回调只投递 §74）。
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// 显式触发一次刷新（queue/locale/外部请求）。
    Refresh,
    Cmd(SystemCommand),
    /// SMTC PositionChanged（秒；worker 侧验证 §69/§232）。
    Seek(f64),
}

/// §150 Artwork 竞态门：异步封面加载完成后凭 generation 校验，旧代
/// 不得覆盖新 track。
#[derive(Debug, Default)]
pub struct GenerationGate {
    current: u64,
}

impl GenerationGate {
    /// 开始一次加载：返回该代是否仍有效（false = 已过期，直接放弃）。
    pub fn begin(&mut self, generation: u64) -> bool {
        if generation < self.current {
            return false;
        }
        self.current = generation;
        true
    }

    /// 加载完成：仅当仍为当前代才允许提交。
    #[must_use]
    pub fn valid(&self, generation: u64) -> bool {
        generation == self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockSink {
        tracks: Vec<SystemSnapshot>,
        statuses: Vec<StateName>,
        positions: Vec<(f64, Option<f64>)>,
    }
    impl IntegrationSink for MockSink {
        fn apply_track(&mut self, s: &SystemSnapshot) {
            self.tracks.push(s.clone());
        }
        fn apply_status(&mut self, s: &SystemSnapshot) {
            self.statuses.push(s.playback_state.clone());
        }
        fn apply_position(&mut self, p: f64, d: Option<f64>) {
            self.positions.push((p, d));
        }
    }

    fn snap(rev: u64, title: &str, state: &str) -> SystemSnapshot {
        SystemSnapshot {
            revision: rev,
            track_id: Some(1),
            title: title.into(),
            artist: Some("A".into()),
            album: Some("AL".into()),
            album_artist: Some("A".into()),
            artwork: None,
            playback_state: state.into(),
            position_seconds: 0.0,
            duration_seconds: Some(100.0),
            seekable: true,
            live: false,
            has_previous: false,
            has_next: true,
        }
    }

    /// §148/§149：revision 单调，旧快照不覆盖。
    #[test]
    fn stale_revision_ignored() {
        let mut c = Coordinator::new();
        let mut sink = MockSink::default();
        c.offer(&snap(10, "B", "playing"), &mut sink);
        c.offer(&snap(5, "A", "playing"), &mut sink); // 迟到
        assert_eq!(sink.tracks.len(), 1);
        assert_eq!(sink.tracks[0].title, "B");
    }

    /// §37/§91：track 变化 → 一次完整边界；同 track 状态变化 → 仅 status。
    #[test]
    fn boundary_vs_status_updates() {
        let mut c = Coordinator::new();
        let mut sink = MockSink::default();
        c.offer(&snap(1, "A", "playing"), &mut sink);
        c.offer(&snap(2, "A", "paused"), &mut sink);
        c.offer(&snap(3, "B", "playing"), &mut sink);
        assert_eq!(sink.tracks.len(), 2, "track 边界 2 次");
        assert_eq!(sink.statuses, vec!["paused".to_string()]);
        assert_eq!(sink.tracks[1].title, "B");
    }

    /// §65/§66/§67：queue 边界变化 → buttons 更新（apply_track 载体）。
    #[test]
    fn queue_boundary_updates_buttons() {
        let mut c = Coordinator::new();
        let mut sink = MockSink::default();
        let mut s = snap(1, "A", "playing");
        c.offer(&s, &mut sink);
        s.revision = 2;
        s.has_next = false; // 队列尾
        c.offer(&s, &mut sink);
        assert_eq!(sink.tracks.len(), 2);
        assert!(!sink.tracks[1].has_next);
    }

    /// §141/§144/§182–§184：position 节流 + 非Playing 忽略。
    #[test]
    fn position_tick_throttle_and_states() {
        let mut c = Coordinator::new();
        let mut sink = MockSink::default();
        let mut s = snap(1, "A", "playing");
        s.position_seconds = 1.0;
        c.offer(&s, &mut sink);
        assert!(c.position_tick(&s, 0.0, &mut sink));
        assert!(!c.position_tick(&s, 0.2, &mut sink), "节流窗口内");
        assert!(c.position_tick(&s, 0.6, &mut sink));
        assert_eq!(sink.positions.len(), 2);
        let paused = snap(2, "A", "paused");
        assert!(!c.position_tick(&paused, 10.0, &mut sink), "Paused 停 tick");
    }

    /// §69/§70/§27：seek 验证（finite/负值/未知时长/不可 seek/clamp）。
    #[test]
    fn seek_validation() {
        let c = Coordinator::new();
        let s = snap(1, "A", "playing"); // seekable, 100s
        assert_eq!(c.validate_seek(&s, 30.0), SeekDecision::Seek(30.0));
        assert_eq!(
            c.validate_seek(&s, 250.0),
            SeekDecision::Seek(100.0),
            "clamp"
        );
        assert_eq!(c.validate_seek(&s, -5.0), SeekDecision::Ignore);
        assert_eq!(c.validate_seek(&s, f64::NAN), SeekDecision::Ignore);
        let live = snap(2, "R", "playing"); // live: 不可 seek
        let mut live = live;
        live.seekable = false;
        live.duration_seconds = None;
        live.live = true;
        assert_eq!(c.validate_seek(&live, 10.0), SeekDecision::Ignore);
    }

    /// §54/§55/§196：ICY 同值去重。
    #[test]
    fn icy_dedup() {
        let mut c = Coordinator::new();
        assert!(c.stream_title_changed("A", "T1"));
        assert!(!c.stream_title_changed("A", "T1"), "同值 coalesce");
        assert!(c.stream_title_changed("B", "T2"));
    }

    /// §133–§136：限长 char 边界 + 控制字符清理 + Unicode/特殊字符保真。
    #[test]
    fn sanitize_limits_and_unicode() {
        let long = "é".repeat(300);
        let cut = sanitize(Some(&long), MAX_TITLE_CHARS).unwrap();
        assert_eq!(cut.chars().count(), MAX_TITLE_CHARS);
        let emoji = "🎵🎵🎵";
        assert_eq!(sanitize(Some(emoji), 2).unwrap(), "🎵🎵");
        assert_eq!(sanitize(Some("a\u{0}b"), 10).unwrap(), "ab");
        // 特殊字符原样保留（§136）。
        let specials = "&<>\"':/\\|—…";
        assert_eq!(sanitize(Some(specials), 64).unwrap(), specials);
        assert_eq!(sanitize(Some("   "), 10), None);
        assert_eq!(sanitize(None, 10), None);
    }

    /// §43/§137/§44：标题 fallback 文件名；artist 缺失不伪造。
    #[test]
    fn title_fallback() {
        assert_eq!(fallback_title(None, "C:/m/夜曲.flac"), "夜曲");
        assert_eq!(fallback_title(Some(""), "/x/a song.mp3"), "a song");
        assert_eq!(fallback_title(Some("真标题"), "/x/a.mp3"), "真标题");
    }

    /// §32/§233：换算 checked（44.1/48/96k、超长、未知）。
    #[test]
    fn conversions_overflow_safe() {
        assert_eq!(seconds_to_ticks(1.0), 10_000_000);
        assert_eq!(seconds_to_ticks(0.5), 5_000_000);
        assert_eq!(seconds_to_ticks(-1.0), 0);
        assert_eq!(seconds_to_ticks(f64::NAN), 0);
        // 96kHz、100 年时长不溢出 u64 ticks。
        let frames = 96_000u64 * 3_600 * 24 * 365 * 100;
        let secs = frames_to_seconds(frames, 96_000);
        let _ = seconds_to_ticks(secs);
        assert_eq!(frames_to_seconds(frames, 0), 0.0, "除零安全");
    }

    /// §150：artwork 竞态门——旧代迟到不覆盖。
    #[test]
    fn artwork_generation_gate() {
        let mut gate = GenerationGate::default();
        assert!(gate.begin(1));
        assert!(gate.begin(2), "新代开始");
        assert!(!gate.valid(1), "旧代完成被拒");
        assert!(gate.valid(2));
        assert!(!gate.begin(0), "过期代开始被拒");
    }
}
