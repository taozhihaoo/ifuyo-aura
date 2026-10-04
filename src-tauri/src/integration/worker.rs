//! Windows Integration worker（§186–§189）：单线程拥有 Coordinator 与
//! sinks；SMTC/Tray 事件经有界 channel 汇入（§187/§74 回调不阻塞）；
//! position 以 500ms tick 低频同步（§141/§144，D-124）；空闲 2Hz 快照
//! 轮询（§180 近似零开销；§181/§182 暂停时不推 position）。

use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, Wry};

use super::tray;
use super::{
    Coordinator, Event, GenerationGate, IntegrationSink, Shared, SystemCommand, SystemSnapshot,
};

struct Worker {
    app: AppHandle<Wry>,
    audio: Arc<crate::audio::AudioHandle>,
    library: Arc<crate::library::LibraryState>,
    shared: Shared,
    coordinator: Coordinator,
    artwork_gate: GenerationGate,
    /// (track_id, artwork_id) → 已加载的封面（§52 仅变化时加载）。
    artwork_loaded: Option<(Option<i64>, Option<i64>)>,
    last_tray_track: Option<String>,
    diag: super::SharedDiag,
}

impl Worker {
    fn build_snapshot(&mut self) -> SystemSnapshot {
        let ps = self.audio.snapshot();
        let position = self.audio.position_seconds();
        let meta = self
            .shared
            .meta
            .lock()
            .expect("meta")
            .clone()
            .unwrap_or_default();
        let (has_prev, has_next) = *self.shared.queue.lock().expect("queue");

        // §52：封面仅在 (track_id, artwork_id) 变化时加载（worker 线程，
        // 非回调/非音频线程 §178/§179）。
        let artwork_key = (meta.track_id, meta.artwork_id);
        let mut artwork: Option<Vec<u8>> = None;
        if self.artwork_loaded.as_ref() != Some(&artwork_key) {
            if let (Some(_track_id), Some(aid)) = (meta.track_id, meta.artwork_id) {
                let generation = aid.unsigned_abs();
                if self.artwork_gate.begin(generation) {
                    if let Ok(service) = self.library.service.lock() {
                        if let Ok(Some((_, bytes))) =
                            service.get_artwork(aura_library::types::ArtworkId(aid))
                        {
                            if self.artwork_gate.valid(generation) {
                                artwork = Some(bytes);
                            }
                        }
                    }
                }
            }
            self.artwork_loaded = Some(artwork_key);
            if artwork.is_some() {
                self.last_tray_track = None; // 封面变化 → tray 行允许刷新
            }
        }

        // §43：title fallback 链 = M3/resolver 标题 → decoder 标题 → 文件名。
        let display = ps.track_path.clone().unwrap_or_default();
        let title = super::fallback_title(meta.title.as_deref().or(ps.title.as_deref()), &display);
        let artist = crate::integration::sanitize(meta.artist.as_deref(), 256);
        let duration = ps.duration_seconds.filter(|d| d.is_finite() && *d > 0.0);

        SystemSnapshot {
            revision: self.audio.revision(),
            track_id: meta.track_id,
            title,
            artist,
            album: crate::integration::sanitize(meta.album.as_deref(), 256),
            album_artist: meta
                .album_artist
                .as_deref()
                .map(str::to_string)
                .or_else(|| meta.artist.clone()),
            artwork,
            playback_state: super::normalize_state(&ps.state).to_string(),
            position_seconds: position,
            duration_seconds: duration,
            seekable: ps.seekable && duration.is_some(),
            // D-124：未知时长按 live 处理（无 timeline；§28/§29/§185）。
            live: duration.is_none(),
            has_previous: has_prev,
            has_next,
        }
    }

    fn offer(&mut self, sink: &mut Sinks) -> SystemSnapshot {
        let snap = self.build_snapshot();
        self.coordinator.offer(&snap, sink);
        // §95/§84/§85/§86：tray 只在 track/state 变化时更新文本/启用态。
        let line = tray::track_line(snap.artist.as_deref(), &snap.title);
        if self.last_tray_track.as_ref() != Some(&line) {
            self.last_tray_track = Some(line.clone());
            let playing = snap.playback_state == "playing";
            let (hp, hn) = (snap.has_previous, snap.has_next);
            let line2 = line;
            let _ = self.app.run_on_main_thread(move || {
                tray::apply_state(playing, hp, hn, Some(&line2));
            });
        }
        // §225：本地 diagnostics（不影响状态 §227）。
        if let Ok(mut d) = self.diag.lock() {
            d.smtc_available = sink.available();
            d.smtc_state = snap.playback_state.clone();
            d.last_revision = snap.revision;
            if let Some(e) = sink.last_error() {
                d.last_error = Some(e);
            }
            d.tray_initialized = tray::is_initialized();
        }
        snap
    }
}

/// 聚合 sink：SMTC（Windows）+ 无操作回退（§13/§119 失败隔离）。
struct Sinks {
    smtc: Option<super::smtc::SmtcBridge>,
    last_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl Sinks {
    fn new(app: &AppHandle<Wry>, shared: &Shared) -> Self {
        #[cfg(windows)]
        {
            let hwnd = app
                .get_webview_window("main")
                .and_then(|w| w.hwnd().ok())
                .map(|h| h.0 as isize);
            if let Some(hwnd) = hwnd {
                let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel(super::smtc::CHANNEL_CAP);
                let last_error: std::sync::Arc<std::sync::Mutex<Option<String>>> =
                    std::sync::Arc::default();
                match super::smtc::SmtcBridge::create(hwnd, cmd_tx, last_error.clone()) {
                    Ok(bridge) => {
                        // SMTC 事件 → worker 事件（§74 只投递；§187 有界）。
                        let ev_tx = shared.events.clone();
                        std::thread::Builder::new()
                            .name("smtc-events".into())
                            .spawn(move || {
                                while let Ok(c) = cmd_rx.recv() {
                                    if let Ok(guard) = ev_tx.lock() {
                                        if let Some(tx) = guard.as_ref() {
                                            let _ = tx.send(Event::Cmd(c));
                                        }
                                    }
                                }
                            })
                            .ok();
                        return Self {
                            smtc: Some(bridge),
                            last_error,
                        };
                    }
                    Err(e) => {
                        if let Ok(mut d) = shared.diag.lock() {
                            d.last_error = Some(e);
                        }
                    }
                }
            }
            if let Ok(mut d) = shared.diag.lock() {
                d.last_error = d
                    .last_error
                    .take()
                    .or_else(|| Some("smtc unavailable (no hwnd)".into()));
            }
        }
        Self {
            smtc: None,
            last_error: std::sync::Arc::default(),
        }
    }
}

impl IntegrationSink for Sinks {
    fn apply_track(&mut self, snap: &SystemSnapshot) {
        if let Some(b) = self.smtc.as_mut() {
            b.apply_track(snap);
        }
    }
    fn apply_status(&mut self, snap: &SystemSnapshot) {
        if let Some(b) = self.smtc.as_mut() {
            b.apply_status(snap);
        }
    }
    fn apply_position(&mut self, position_seconds: f64, duration_seconds: Option<f64>) {
        if let Some(b) = self.smtc.as_mut() {
            b.apply_position(position_seconds, duration_seconds);
        }
    }
}

impl Sinks {
    fn available(&self) -> bool {
        self.smtc.is_some()
    }
    fn last_error(&self) -> Option<String> {
        self.last_error.lock().ok().and_then(|g| g.clone())
    }
}

/// 启动 integration worker（真实 app setup 调用；smoke 不启动）。
pub fn spawn(
    app: AppHandle<Wry>,
    audio: Arc<crate::audio::AudioHandle>,
    library: Arc<crate::library::LibraryState>,
    shared: Shared,
) {
    let _ = std::thread::Builder::new()
        .name("windows-integration".into())
        .spawn(move || run(app, audio, library, shared));
}

fn run(
    app: AppHandle<Wry>,
    audio: Arc<crate::audio::AudioHandle>,
    library: Arc<crate::library::LibraryState>,
    shared: Shared,
) {
    let (tx, rx) = std::sync::mpsc::channel::<Event>();
    *shared.events.lock().expect("events") = Some(tx.clone());

    let mut worker = Worker {
        app: app.clone(),
        audio,
        library,
        shared: shared.clone(),
        coordinator: Coordinator::new(),
        artwork_gate: GenerationGate::default(),
        artwork_loaded: None,
        last_tray_track: None,
        diag: shared.diag.clone(),
    };
    let mut sinks = Sinks::new(&app, &shared);

    loop {
        // §144：500ms tick（position + 惰性轮询；§145 无忙循环）。
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Event::Cmd(cmd)) => {
                // §7/§8：统一进 Playback Authority；§155 幂等由引擎保证。
                let posted: Result<(), String> = match cmd {
                    SystemCommand::Play => worker.audio.post(aura_core::AudioCommand::Play),
                    SystemCommand::Pause => worker.audio.post(aura_core::AudioCommand::Pause),
                    SystemCommand::Stop => worker.audio.post(aura_core::AudioCommand::Stop),
                    // §23/§24：next/previous 复用前端 queue 语义（事件桥）。
                    SystemCommand::Next => worker
                        .app
                        .emit(
                            "integration://command",
                            serde_json::json!({ "cmd": "next" }),
                        )
                        .map_err(|e| e.to_string()),
                    SystemCommand::Previous => worker
                        .app
                        .emit(
                            "integration://command",
                            serde_json::json!({ "cmd": "previous" }),
                        )
                        .map_err(|e| e.to_string()),
                };
                let _ = posted;
                // 即时刷新（状态变化 1 tick 内反映到系统 UI §37）。
                worker.offer(&mut sinks);
                continue;
            }
            Ok(Event::Seek(secs)) => {
                let snap = worker.build_snapshot();
                if let super::SeekDecision::Seek(t) = worker.coordinator.validate_seek(&snap, secs)
                {
                    // §32/§233：秒→帧 checked（防溢出/负数）。
                    let sr = worker.audio.sample_rate();
                    let frames = (t * sr as f64).round();
                    if frames.is_finite() && frames >= 0.0 && frames <= u64::MAX as f64 {
                        let _ = worker.audio.post(aura_core::AudioCommand::Seek {
                            frame: frames as u64,
                        });
                    }
                }
                continue;
            }
            Ok(Event::Refresh) => {
                worker.offer(&mut sinks);
                continue;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        }

        // Tick：§53–§56 ICY 事件（非阻塞轮询）→ 快照 → coordinator。
        let icy_event = {
            let icy = worker.shared.icy.lock().expect("icy");
            icy.as_ref().and_then(|rx| rx.try_recv().ok())
        };
        if let Some(ev) = icy_event {
            let mut changed = false;
            {
                let mut meta_guard = worker.shared.meta.lock().expect("meta");
                if let Some(m) = meta_guard.as_mut() {
                    let artist = ev.artist.clone().or_else(|| m.artist.clone());
                    let title = ev.title.clone().or_else(|| ev.station.clone());
                    if let (Some(a), Some(t)) = (&artist, &title) {
                        if worker.coordinator.stream_title_changed(a, t) {
                            m.artist = artist;
                            m.title = title;
                            changed = true;
                        }
                    }
                }
            }
            if changed {
                worker.offer(&mut sinks);
            }
        }

        let now = Instant::now().elapsed().as_secs_f64();
        let snap = worker.offer(&mut sinks);
        worker.coordinator.position_tick(&snap, now, &mut sinks);
    }
}
