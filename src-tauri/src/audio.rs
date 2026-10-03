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
        duration_frames: Option<u64>,
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
}

impl AudioHandle {
    /// 启动驱动线程：Engine @48k stereo；消费端移交 CPAL 默认设备（D-064）。
    ///
    /// # Errors
    /// Engine/CPAL/线程初始化失败 → [`aura_core::AudioError`]。
    pub fn start() -> Result<Self, aura_core::AudioError> {
        let (mut engine, poster) =
            aura_core::Engine::new(SampleRate::new(48_000)?, Channels::new(2)?, 16_384)?;

        // D-064：消费端移交外部真实输出；此后驱动线程只 fill+process。
        let (consumer, shared_out, eq_out) = engine.detach_consumer_for_external_output()?;
        let _cpal = aura_core::output::device::CpalOutput::open_default(
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
                    duration_frames: Option<u64>,
                    sample_rate: u32,
                }
                let mut poster = poster;
                let mut st = DriverState {
                    duration_frames: None,
                    sample_rate: 48_000,
                };
                loop {
                    while let Ok(msg) = rx.try_recv() {
                        match msg {
                            Control::LoadDecoder {
                                decoder,
                                duration_frames,
                                title,
                                display,
                            } => {
                                st.duration_frames = duration_frames;
                                let _ = engine.load_track(decoder);
                                let _ = poster.post(AudioCommand::Stop);
                                let mut snap = snap2.lock().expect("snap");
                                snap.track_path = Some(display);
                                snap.title = title;
                                snap.state = "stopped".into();
                                snap.position_seconds = 0.0;
                                snap.duration_seconds =
                                    st.duration_frames.map(|f| f as f64 / 48_000.0);
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
                            Control::Post(cmd) => {
                                let _ = poster.post(cmd);
                            }
                            Control::Shutdown => return,
                        }
                    }
                    engine.fill_and_process();
                    // 快照刷新 ~10Hz（§14）。
                    let pos_frames = shared_out.position.load(Ordering::Relaxed);
                    pos2.store(pos_frames, Ordering::Relaxed);
                    let new_state = match engine.state() {
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
///
/// # Errors
/// 文件不存在/不可读/codec 不支持（decoder probing 失败）。
pub fn open_decoder(
    path: &Path,
) -> Result<(Box<dyn aura_core::decoder::Decoder>, Option<u64>), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stream = LocalFileStream::open(path).map_err(|e| e.to_string())?;
    let decoder =
        StreamingDecoder::open(Box::new(stream), Some(&ext)).map_err(|e| e.to_string())?;
    let duration = aura_core::decoder::Decoder::duration_frames(&decoder);
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
