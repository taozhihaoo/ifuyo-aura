//! 音频应用服务（M6-a，D-064）：驱动线程持有 M1 Engine；CPAL 回调消费。
//! 快照 ~10Hz（§14 低频）；实时位置走共享原子（UI 本地插值）。

use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aura_core::decoder::streaming::StreamingDecoder;

use aura_core::types::{AudioCommand, Channels, PlaybackState, SampleRate};

/// 本地文件的 [`aura_core::decoder::AudioByteStream`]（M3 支持格式直读）。
pub struct LocalFileStream {
    file: std::fs::File,
    len: u64,
}

impl LocalFileStream {
    /// # Errors
    /// 文件不存在/不可读。
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }
}

impl Read for LocalFileStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl std::io::Seek for LocalFileStream {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl aura_core::decoder::AudioByteStream for LocalFileStream {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

/// UI 快照（§11 PlayerSnapshot）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshot {
    pub state: String,
    pub position_seconds: f64,
    pub duration_seconds: Option<f64>,
    pub volume: f32,
    pub track_path: Option<String>,
    pub title: Option<String>,
    pub seekable: bool,
    pub revision: u64,
}

/// 驱动线程控制消息。
pub enum Control {
    /// decoder 已在命令线程打开（错误可回传 UI，§184）；驱动线程只装载。
    LoadDecoder {
        decoder: Box<dyn aura_core::decoder::Decoder>,
        /// 曲目时长（秒；源采样率已换算）。
        duration_seconds: Option<f64>,
        title: Option<String>,
        /// 快照展示标识（本地路径或流 URL）。
        display: String,
    },
    SetCrossfade(f64),
    SetLoudness(bool),
    SetEqPreset(usize),
    /// 实时命令（Play/Pause/Stop/Seek/SetVolume）统一经控制通道。
    Post(AudioCommand),
    Shutdown,
}

/// 音频服务句柄。
pub struct AudioHandle {
    control: Mutex<std::sync::mpsc::Sender<Control>>,
    snapshot: Arc<Mutex<PlayerSnapshot>>,
    position_frames: Arc<AtomicU64>,
    sample_rate: Arc<AtomicU64>,
    revision: Arc<AtomicU64>,
    /// CPAL 输出流所有权——**必须随句柄存活**。此前流绑定在 start()
    /// 的局部变量上，返回即 drop，回调永不触发（表现为"播放中但位置
    /// 不走"）；Mutex 仅用于满足 Arc<AudioHandle> 的 Sync 要求。
    _output: Mutex<aura_core::output::device::CpalOutput>,
}

impl AudioHandle {
    /// 启动驱动线程：Engine @48k stereo；消费端移交 CPAL 默认设备（D-064）。
    ///
    /// `on_track_ended` 在曲目自然播完（非用户停止）时于驱动线程调用
    /// （宿主用于向前端发 `track-ended` 事件，驱动队列自动前进）。
    ///
    /// # Errors
    /// Engine/CPAL/线程初始化失败 → [`aura_core::AudioError`]。
    pub fn start(
        on_track_ended: impl Fn() + Send + 'static,
    ) -> Result<Self, aura_core::AudioError> {
        let (mut engine, poster) =
            aura_core::Engine::new(SampleRate::new(48_000)?, Channels::new(2)?, 16_384)?;

        // D-064：消费端移交外部真实输出；此后驱动线程只 fill+process。
        let (consumer, shared_out, eq_out) = engine.detach_consumer_for_external_output()?;
        let output = aura_core::output::device::CpalOutput::open_default(
            consumer,
            shared_out.clone(),
            eq_out,
        )?;

        let snapshot = Arc::new(Mutex::new(PlayerSnapshot {
            state: "idle".into(),
            position_seconds: 0.0,
            duration_seconds: None,
            volume: 1.0,
            track_path: None,
            title: None,
            seekable: true,
            revision: 0,
        }));
        let position = Arc::new(AtomicU64::new(0));
        let rate = Arc::new(AtomicU64::new(48_000));
        let revision = Arc::new(AtomicU64::new(0));

        let (tx, rx) = std::sync::mpsc::channel::<Control>();
        let snap2 = Arc::clone(&snapshot);
        let pos2 = Arc::clone(&position);
        let rate2 = Arc::clone(&rate);
        let rev2 = Arc::clone(&revision);

        std::thread::Builder::new()
            .name("aura-audio-driver".into())
            .spawn(move || {
                struct DriverState {
                    duration_seconds: Option<f64>,
                    sample_rate: u32,
                }
                let mut poster = poster;
                let mut st = DriverState {
                    duration_seconds: None,
                    sample_rate: 48_000,
                };
                // M12.5：淡出期间挂起的暂停/停止（1s 淡变完成后生效 §183）。
                let mut pending_transport: Option<aura_core::types::AudioCommand> = None;
                // M12.5：Play 已入队、待引擎转入 Playing 后再请求淡入
                // （时序：回调的 state 快照可能滞后，须先发布 Playing）。
                let mut want_fade_in = false;
                loop {
                    while let Ok(msg) = rx.try_recv() {
                        match msg {
                            Control::LoadDecoder {
                                decoder,
                                duration_seconds,
                                title,
                                display,
                            } => {
                                st.duration_seconds = duration_seconds;
                                // 换曲使旧传输挂起失效：若上一首的停止/暂停
                                // 淡出仍挂起，随后的 play_track Play 会命中
                                // "淡出中反转"分支被吞（旧曲已不存在）——
                                // 新曲永远不播（用户场景：停止后立即双击
                                // 另一首）。load_track 自带归零 Stop，旧挂起
                                // 已无意义。
                                pending_transport = None;
                                want_fade_in = false;
                                let _ = engine.load_track(decoder);
                                let _ = poster.post(AudioCommand::Stop);
                                let mut snap = snap2.lock().expect("snap");
                                snap.track_path = Some(display);
                                snap.title = title;
                                snap.state = "stopped".into();
                                snap.position_seconds = 0.0;
                                snap.duration_seconds = st.duration_seconds;
                                snap.revision += 1;
                                rev2.store(snap.revision, Ordering::Relaxed);
                            }
                            Control::SetCrossfade(s) => engine.set_crossfade_seconds(s),
                            Control::SetLoudness(on) => {
                                engine.set_track_loudness(aura_core::TrackLoudness::missing(), on);
                            }
                            Control::SetEqPreset(i) => {
                                let presets = aura_core::dsp::EQ_PRESETS;
                                engine.set_eq_preset(&presets[i.min(presets.len() - 1)]);
                            }
                            Control::Post(cmd) => match cmd {
                                // M12.5：暂停/停止走 1s 淡出后生效（淡出期间
                                // 引擎保持 Playing 供数据）；播放立即生效并淡入。
                                AudioCommand::Pause | AudioCommand::Stop
                                    if engine.state() == PlaybackState::Playing =>
                                {
                                    eprintln!(
                                        "[fade-probe] driver: fade-out requested for {cmd:?}"
                                    );
                                    shared_out.request_fade(
                                        aura_core::output::FADE_CMD_OUT,
                                        aura_core::output::FADE_TOTAL_AT_48K,
                                    );
                                    pending_transport = Some(cmd);
                                }
                                AudioCommand::Play if pending_transport.is_some() => {
                                    // 淡出中改播放：从当前增益连续淡入回满
                                    // （request_fade 取已发布增益，反转不跳变）。
                                    shared_out.request_fade(
                                        aura_core::output::FADE_CMD_IN,
                                        aura_core::output::FADE_TOTAL_AT_48K,
                                    );
                                    pending_transport = None;
                                    eprintln!("[fade-probe] driver: reversal fade-in requested");
                                }
                                AudioCommand::Play => {
                                    let _ = poster.post(cmd);
                                    want_fade_in = true;
                                    eprintln!("[fade-probe] driver: play posted, fade-in armed");
                                }
                                other => {
                                    let _ = poster.post(other);
                                }
                            },
                            Control::Shutdown => return,
                        }
                    }
                    // M12.5：淡出完成后应用挂起的暂停/停止。曲目在淡出中
                    // 自然结束（EOF→Stopped）时 progress 不再推进，同样视为
                    // 完成，避免挂起命令永久滞留。
                    if let Some(pending) = pending_transport {
                        let progress = shared_out.fade_progress();
                        let total = shared_out.fade_total();
                        if total > 0
                            && (progress >= total || engine.state() == PlaybackState::Stopped)
                        {
                            eprintln!("[fade-probe] driver: pending {pending:?} applied");
                            match pending {
                                AudioCommand::Pause => {
                                    let _ = poster.post(AudioCommand::Pause);
                                }
                                AudioCommand::Stop => {
                                    let _ = poster.post(AudioCommand::Stop);
                                }
                                _ => {}
                            }
                            pending_transport = None;
                            shared_out.clear_fade();
                        }
                    }

                    engine.fill_and_process();
                    // 曲目自然播完：Stop（归零语义，引擎不再假播放）+ 通知
                    // 前端队列按播放模式自动前进（顺序/循环/随机）。
                    if engine.take_track_finished() {
                        let _ = poster.post(AudioCommand::Stop);
                        on_track_ended();
                        eprintln!("[fade-probe] driver: track finished -> stop + ended event");
                    }
                    // 快照刷新 ~10Hz（§14）。
                    let pos_frames = shared_out.position.load(Ordering::Relaxed);
                    pos2.store(pos_frames, Ordering::Relaxed);
                    let engine_state = engine.state();
                    shared_out.set_playback_state(engine_state); // §183：暂停时回调停止推进位置
                                                                 // M12.5：淡入请求须在 Playing 已发布之后（回调 state 快照
                                                                 // 滞后一拍时，无守卫的 IN 消费保证命令不丢）。从静音恢复
                                                                 // 用显式 0 起点（新装载/停止场景无已发布淡变增益）。
                    if want_fade_in && engine_state == PlaybackState::Playing {
                        want_fade_in = false;
                        shared_out.request_fade_gain(
                            aura_core::output::FADE_CMD_IN,
                            aura_core::output::FADE_TOTAL_AT_48K,
                            0.0,
                        );
                        eprintln!("[fade-probe] driver: fade-in requested (1s)");
                    }
                    let new_state = match engine_state {
                        PlaybackState::Playing => "playing",
                        PlaybackState::Paused => "paused",
                        PlaybackState::Stopped => "stopped",
                    };
                    let vol = engine.volume().get();
                    let mut snap = snap2.lock().expect("snap");
                    let changed =
                        snap.state != new_state || (snap.volume - vol).abs() > f32::EPSILON;
                    snap.state = new_state.into();
                    snap.volume = vol;
                    snap.position_seconds = pos_frames as f64 / f64::from(st.sample_rate.max(1));
                    if changed {
                        snap.revision += 1;
                        rev2.store(snap.revision, Ordering::Relaxed);
                    }
                    drop(snap);
                    rate2.store(48_000, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_millis(100));
                }
            })
            .map_err(|_| aura_core::AudioError::OutputError("driver spawn failed"))?;

        Ok(Self {
            control: Mutex::new(tx),
            snapshot,
            position_frames: position,
            sample_rate: rate,
            revision,
            _output: Mutex::new(output),
        })
    }

    #[must_use]
    pub fn snapshot(&self) -> PlayerSnapshot {
        self.snapshot.lock().expect("snap").clone()
    }

    /// 实时位置（秒；驱动线程写入，UI 插值基准）。
    #[must_use]
    pub fn position_seconds(&self) -> f64 {
        let frames = self.position_frames.load(Ordering::Relaxed);
        let rate = self.sample_rate.load(Ordering::Relaxed).max(1);
        frames as f64 / rate as f64
    }

    #[must_use]
    /// 当前输出采样率（SMTC 帧↔秒换算用，§32）。
    pub fn sample_rate(&self) -> u64 {
        self.sample_rate.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    /// 控制消息（Load/crossfade/eq…）。
    pub fn send(&self, msg: Control) -> Result<(), String> {
        self.control
            .lock()
            .expect("control lock")
            .send(msg)
            .map_err(|_| "audio driver stopped".into())
    }

    /// 实时命令（Play/Pause/Stop/Seek/SetVolume → 命令队列）。
    pub fn post(&self, cmd: AudioCommand) -> Result<(), String> {
        self.control
            .lock()
            .expect("control lock")
            .send(Control::Post(cmd))
            .map_err(|e| format!("command queue: {e}"))
    }
}

/// 本地文件 → StreamingDecoder（命令线程调用，错误可回传 UI）。
/// 时长返回**秒**（源采样率换算——帧数是源空间，除以 48k 会算错非 48k 文件）。
///
/// # Errors
/// 文件不存在/不可读/codec 不支持（decoder probing 失败）。
pub fn open_decoder(
    path: &Path,
) -> Result<(Box<dyn aura_core::decoder::Decoder>, Option<f64>), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stream = LocalFileStream::open(path).map_err(|e| e.to_string())?;
    let decoder =
        StreamingDecoder::open(Box::new(stream), Some(&ext)).map_err(|e| e.to_string())?;
    let src_rate = f64::from(
        aura_core::decoder::Decoder::format(&decoder)
            .sample_rate
            .get(),
    );
    let duration =
        aura_core::decoder::Decoder::duration_frames(&decoder).map(|f| f as f64 / src_rate);
    let boxed: Box<dyn aura_core::decoder::Decoder> = Box::new(decoder);
    Ok((boxed, duration))
}

#[allow(dead_code)]
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    let _ = assert_send_sync::<AudioHandle>;
};

#[allow(dead_code)]
fn _sync_probe() {
    fn check<T: Send + Sync>() {}
    check::<AudioHandle>();
    check::<crate::library::LibraryState>();
    check::<crate::library::TrackSummary>();
    check::<PlayerSnapshot>();
}
