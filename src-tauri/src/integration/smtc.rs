//! SMTC 适配器（§9–§35/§47–§62/§104，Windows 专用）：纯 platform
//! adapter——只消费 [`SystemSnapshot`](super::SystemSnapshot)（§41 不自
//! 查 metadata）、只投递 application 命令（§8/§74 回调不阻塞）。
//!
//! - §10/§11：单一 controller 负责 init/订阅/更新/teardown。
//! - §104/§105：token 保存，退出时移除订阅（防重复命令）。
//! - §65–§68：按钮 enabled 随 snapshot（queue 边界/seekable）驱动。
//! - §51：封面失败不影响媒体控制。
//! - §27/§70（D-125）：Windows SMTC v1 API **没有** position-request
//!   通道（系统 UI 不向 SMTC 应用发起 seek；PositionChanged 仅存在于
//!   无关的 MediaTimelineController）——seek 面由应用内 UI/托盘承担，
//!   走 coordinator.validate_seek；SMTC 侧不存在可到达的 seek 事件。
//! - §121/§122：WinRT 错误转诊断，绝不 panic。

#![cfg(windows)]

use windows::core::{factory, HSTRING};
use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
    SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
    SystemMediaTransportControlsTimelineProperties,
};
use windows::Storage::Streams::{
    DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;

use super::{IntegrationSink, SystemCommand, SystemSnapshot};

/// SMTC 命令投递通道容量（§187 有界队列）。
pub const CHANNEL_CAP: usize = 64;

pub struct SmtcBridge {
    smtc: SystemMediaTransportControls,
    /// §104/§105：token 保存（退出移除，防重启后重复命令）。
    button_token: i64,
    /// §104 诊断（最近一次 WinRT 错误）。
    pub last_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

fn err_string(e: &windows::core::Error) -> String {
    format!("smtc: {e}")
}

fn record(bridge: &SmtcBridge, e: &windows::core::Error) {
    if let Ok(mut slot) = bridge.last_error.lock() {
        *slot = Some(err_string(e));
    }
}

fn system_command_for(btn: SystemMediaTransportControlsButton) -> Option<SystemCommand> {
    match btn {
        SystemMediaTransportControlsButton::Play => Some(SystemCommand::Play),
        SystemMediaTransportControlsButton::Pause => Some(SystemCommand::Pause),
        SystemMediaTransportControlsButton::Stop => Some(SystemCommand::Stop),
        SystemMediaTransportControlsButton::Next => Some(SystemCommand::Next),
        SystemMediaTransportControlsButton::Previous => Some(SystemCommand::Previous),
        // §68：fast-forward/rewind 等未启用；防御性忽略。
        _ => None,
    }
}

impl SmtcBridge {
    /// 绑定主窗 HWND，创建 SMTC 会话并订阅按钮事件。
    ///
    /// # Errors
    /// WinRT 初始化失败 → `String`（调用方降级为 Unavailable §13）。
    pub fn create(
        hwnd: isize,
        cmd_tx: std::sync::mpsc::SyncSender<SystemCommand>,
        last_error: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    ) -> Result<Self, String> {
        let interop: ISystemMediaTransportControlsInterop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()
                .map_err(|e| err_string(&e))?;
        let smtc: SystemMediaTransportControls =
            unsafe { interop.GetForWindow::<SystemMediaTransportControls>(HWND(hwnd as *mut _)) }
                .map_err(|e| err_string(&e))?;

        smtc.SetIsEnabled(true).map_err(|e| err_string(&e))?;
        // §20/§68：只开启实际支持的命令（fast-forward/rewind/seek 不开）。
        smtc.SetIsPlayEnabled(true).map_err(|e| err_string(&e))?;
        smtc.SetIsPauseEnabled(true).map_err(|e| err_string(&e))?;
        smtc.SetIsStopEnabled(true).map_err(|e| err_string(&e))?;
        smtc.SetIsNextEnabled(false).map_err(|e| err_string(&e))?;
        smtc.SetIsPreviousEnabled(false)
            .map_err(|e| err_string(&e))?;

        let err_btn = last_error.clone();
        let button_token = smtc
            .ButtonPressed(&TypedEventHandler::new(
                move |_: windows::core::Ref<'_, SystemMediaTransportControls>,
                      args: windows::core::Ref<
                    '_,
                    SystemMediaTransportControlsButtonPressedEventArgs,
                >| {
                    // §74：回调只投递（有界 channel；满则丢弃并记录）。
                    let cmd = args
                        .ok()
                        .ok()
                        .and_then(|a| a.Button().ok())
                        .and_then(system_command_for);
                    if let Some(cmd) = cmd {
                        if cmd_tx.send(cmd).is_err() {
                            if let Ok(mut e) = err_btn.lock() {
                                *e = Some("button channel closed".into());
                            }
                        }
                    }
                    Ok(())
                },
            ))
            .map_err(|e| err_string(&e))?;

        Ok(Self {
            smtc,
            button_token,
            last_error,
        })
    }

    fn set_status(&self, state: &str) {
        let st = match state {
            "playing" => MediaPlaybackStatus::Playing,
            "paused" => MediaPlaybackStatus::Paused,
            // §59/§60：idle/error/buffering→Stopped 语义（不长期假 Playing）。
            _ => MediaPlaybackStatus::Stopped,
        };
        if let Err(e) = self.smtc.SetPlaybackStatus(st) {
            record(self, &e);
        }
    }

    /// §31–§35：低频 timeline（秒 → 100ns ticks checked §32/§233）。
    /// §28/§29：未知时长 → end=0（不伪造 live 的 duration/position）。
    fn update_timeline(&self, position_seconds: f64, duration_seconds: Option<f64>) {
        let end_ticks = duration_seconds.map(super::seconds_to_ticks).unwrap_or(0);
        let Ok(tl) = SystemMediaTransportControlsTimelineProperties::new() else {
            return;
        };
        let _ = tl.SetStartTime(TimeSpan { Duration: 0 });
        let _ = tl.SetMinSeekTime(TimeSpan { Duration: 0 });
        let _ = tl.SetEndTime(TimeSpan {
            Duration: end_ticks as i64,
        });
        let _ = tl.SetMaxSeekTime(TimeSpan {
            Duration: end_ticks as i64,
        });
        let _ = tl.SetPosition(TimeSpan {
            Duration: super::seconds_to_ticks(position_seconds) as i64,
        });
        if let Err(e) = self.smtc.UpdateTimelineProperties(&tl) {
            record(self, &e);
        }
    }
}

impl IntegrationSink for SmtcBridge {
    /// §37 边界：metadata + artwork + buttons + timeline + status 一次提交。
    fn apply_track(&mut self, s: &SystemSnapshot) {
        let updater = match self.smtc.DisplayUpdater() {
            Ok(u) => u,
            Err(e) => return record(self, &e),
        };
        let _ = updater.SetType(MediaPlaybackType::Music);
        let music = match updater.MusicProperties() {
            Ok(m) => m,
            Err(e) => return record(self, &e),
        };
        // §133–§137：字符串已被 coordinator sanitize；此处仅 HSTRING。
        let _ = music.SetTitle(&HSTRING::from(s.title.as_str()));
        let _ = music.SetArtist(&HSTRING::from(s.artist.as_deref().unwrap_or_default()));
        let _ = music.SetAlbumTitle(&HSTRING::from(s.album.as_deref().unwrap_or_default()));
        let _ = music.SetAlbumArtist(&HSTRING::from(
            s.album_artist.as_deref().unwrap_or_default(),
        ));

        // §47–§51：应用内封面字节（coordinator 提供）；任何失败 → 无封面，
        // 媒体控制仍可用。
        if let Some(png) = &s.artwork {
            if let Ok(stream) = InMemoryRandomAccessStream::new() {
                if let Ok(writer) = DataWriter::CreateDataWriter(&stream) {
                    let stored = writer
                        .WriteBytes(png)
                        .and_then(|()| writer.StoreAsync())
                        .and_then(|op| op.join().map(|_| ()))
                        .and_then(|()| writer.FlushAsync())
                        .and_then(|op| op.join().map(|_| ()));
                    if stored.is_ok() {
                        let _ = writer.DetachStream();
                        if let Ok(reference) =
                            RandomAccessStreamReference::CreateFromStream(&stream)
                        {
                            let _ = updater.SetThumbnail(&reference);
                        }
                    }
                }
            }
        }

        // §36/§52：整个 DisplayUpdater 一次 Update（无每秒刷新）。
        if let Err(e) = updater.Update() {
            return record(self, &e);
        }

        // §65–§67：按钮 enabled 随 queue 边界。
        let _ = self.smtc.SetIsNextEnabled(s.has_next);
        let _ = self.smtc.SetIsPreviousEnabled(s.has_previous);

        self.update_timeline(s.position_seconds, s.duration_seconds);
        self.set_status(&s.playback_state);
    }

    fn apply_status(&mut self, s: &SystemSnapshot) {
        self.set_status(&s.playback_state);
    }

    /// §141/§183：低频 position（coordinator 已节流；Paused 不 tick）。
    fn apply_position(&mut self, position_seconds: f64, duration_seconds: Option<f64>) {
        self.update_timeline(position_seconds, duration_seconds);
    }
}

impl Drop for SmtcBridge {
    /// §104/§105：退出移除订阅 + 关闭会话。
    fn drop(&mut self) {
        let _ = self.smtc.RemoveButtonPressed(self.button_token);
        let _ = self.smtc.SetIsEnabled(false);
    }
}
