//! CPAL 真实输出设备适配器（M1 §29–30，最小骨架）。
//!
//! - 默认输出设备 + 设备默认格式（f32 / i16 两种采样格式适配）；
//! - 回调路径复用 [`OutputStage`]（同一条零分配渲染代码）；
//! - 错误回调安全：只递增原子计数，不 panic、不打日志；
//! - 已知限制（D-021）：设备采样率/声道与生产端不一致时的重采样接线属后续
//!   里程碑（M1 的真实格式重采样在 decode 侧，设备格式协商在 M6+ 完善）。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};

use crate::audio::buffer::ConsumerHandle;
use crate::error::AudioError;
use crate::output::{AudioOutput, OutputStage, SharedOutputState};
use crate::types::{AudioFormat, Channels, SampleRate};

/// 设备错误计数（错误回调内仅递增，§45）。
#[derive(Clone)]
pub struct DeviceErrorCounter(Arc<AtomicU64>);

impl DeviceErrorCounter {
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }

    #[must_use]
    pub fn count(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for DeviceErrorCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// CPAL 真实输出。消费端句柄与渲染阶段移入流回调。
pub struct CpalOutput {
    _stream: Stream,
    format: AudioFormat,
    shared: SharedOutputState,
    device_errors: DeviceErrorCounter,
    _eq_bus: Arc<crate::dsp::EqBus>,
}

impl CpalOutput {
    /// 打开默认输出设备并创建流（立即开始播放）。
    ///
    /// `consumer` 与 `shared` 来自 Engine 的生产管线；`eq_bus` 为 EQ 快照总线。
    /// 设备实际格式（采样率/声道）以设备默认配置为准。
    pub fn open_default(
        consumer: ConsumerHandle,
        shared: SharedOutputState,
        eq_bus: Arc<crate::dsp::EqBus>,
    ) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::OutputError("no default output device"))?;
        let supported = device
            .default_output_config()
            .map_err(|_| AudioError::OutputError("no default output config"))?;

        let config = supported.config();
        let channels = Channels::new(
            u16::try_from(config.channels)
                .map_err(|_| AudioError::InvalidParameter("device channel count out of range"))?,
        )?;
        let sample_rate = SampleRate::new(config.sample_rate)?;
        let format = AudioFormat::new(sample_rate, channels);

        let device_errors = DeviceErrorCounter::new();
        let err_counter = device_errors.clone();

        let stream = match supported.sample_format() {
            SampleFormat::F32 => {
                let mut stage = OutputStage::new(sample_rate, channels);
                let mut consumer = consumer;
                let shared_cb = shared.clone();
                let eq_cb = Arc::clone(&eq_bus);
                device.build_output_stream(
                    config,
                    move |data: &mut [f32], _info| {
                        stage.render(
                            data,
                            &mut consumer,
                            &shared_cb,
                            crate::types::PlaybackState::Playing,
                            Some(&eq_cb),
                        );
                    },
                    move |err| {
                        let _ = err;
                        err_counter.0.fetch_add(1, Ordering::Relaxed);
                    },
                    None,
                )
            }
            SampleFormat::I16 => {
                let mut stage = OutputStage::new(sample_rate, channels);
                let mut consumer = consumer;
                let shared_cb = shared.clone();
                let eq_cb = Arc::clone(&eq_bus);
                // 预分配 f32 工作区（回调内零分配；按固定块渲染后转换）
                const SCRATCH_FRAMES: usize = 1024;
                let mut scratch = vec![0.0f32; SCRATCH_FRAMES * usize::from(channels.get())];
                device.build_output_stream(
                    config,
                    move |data: &mut [i16], _info| {
                        let ch = usize::from(stage.channels().get());
                        // 分块：每块 ≤ 预分配工作区
                        for block in data.chunks_mut(scratch.len()) {
                            let frames = block.len() / ch.max(1);
                            let want = frames * ch;
                            let rendered = stage.render(
                                &mut scratch[..want],
                                &mut consumer,
                                &shared_cb,
                                crate::types::PlaybackState::Playing,
                                Some(&eq_cb),
                            );
                            let _ = rendered;
                            for (dst, src) in block.iter_mut().zip(scratch[..want].iter()) {
                                // f32 [-1,1] → i16（clamp 后缩放，确定性）
                                let v = src.clamp(-1.0, 1.0);
                                *dst = (v * 32_767.0) as i16;
                            }
                        }
                    },
                    move |err| {
                        let _ = err;
                        err_counter.0.fetch_add(1, Ordering::Relaxed);
                    },
                    None,
                )
            }
            _ => return Err(AudioError::UnsupportedFormat("device sample format")),
        }
        .map_err(|_| AudioError::OutputError("failed to build output stream"))?;

        stream
            .play()
            .map_err(|_| AudioError::OutputError("failed to start output stream"))?;

        Ok(Self {
            _stream: stream,
            format,
            shared,
            device_errors,
            _eq_bus: eq_bus,
        })
    }

    #[must_use]
    pub fn device_errors(&self) -> u64 {
        self.device_errors.count()
    }
}

impl AudioOutput for CpalOutput {
    fn format(&self) -> AudioFormat {
        self.format
    }

    fn start(&mut self) -> Result<(), AudioError> {
        // 流在构造时已 play；M1 的 pause/resume 语义由 Engine 状态驱动
        Ok(())
    }

    fn pause(&mut self) -> Result<(), AudioError> {
        // M1 最小骨架：状态暂停由 Engine 驱动（渲染输出静音），设备流保持运行
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AudioError> {
        // Stream 在 drop 时关闭
        Ok(())
    }

    fn consumed_frames(&self) -> u64 {
        self.shared.position.load(Ordering::Relaxed)
    }

    fn underruns(&self) -> u64 {
        self.shared.underruns.load(Ordering::Relaxed)
    }
}
