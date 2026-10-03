//! 测试共享夹具（M1 §10）：确定性合成音频解码器。
//!
//! Fixture A：440Hz 正弦；B：可调幅度正弦；C：impulse；D：带编号 ramp
//! （sample[n] = n，可精确验证 seek 后的 sample provenance）。
#![allow(dead_code)]

use aura_core::decoder::Decoder;
use aura_core::error::AudioError;
use aura_core::PcmBuffer;
use aura_core::{AudioFormat, Channels, SampleRate};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    /// Fixture A/B：正弦（可调频率与幅度）。
    Sine { freq_hz: f32, amplitude: f32 },
    /// Fixture C：impulse（首帧 1.0，其余 0）。
    Impulse,
    /// Fixture D：ramp，sample[n] = n as f32（绝对帧号，provenance 自证）。
    Ramp,
    /// 直流电平（增益/爆音测试）。
    Dc(f32),
    /// 非有限值注入（前 16 帧 NaN，接着 16 帧 Inf，其余 0.5）。
    NonFinite,
}

pub struct SyntheticDecoder {
    format: AudioFormat,
    total_frames: u64,
    next_frame: u64,
    chunk_frames: usize,
    waveform: Waveform,
}

impl SyntheticDecoder {
    #[must_use]
    pub fn new(
        sample_rate: u32,
        channels: u16,
        total_frames: u64,
        chunk_frames: usize,
        waveform: Waveform,
    ) -> Self {
        Self {
            format: AudioFormat::new(
                SampleRate::new(sample_rate).unwrap(),
                Channels::new(channels).unwrap(),
            ),
            total_frames,
            next_frame: 0,
            chunk_frames,
            waveform,
        }
    }

    fn sample_value(&self, frame: u64) -> f32 {
        let n = frame;
        match self.waveform {
            Waveform::Sine { freq_hz, amplitude } => {
                amplitude
                    * (std::f32::consts::TAU * freq_hz * n as f32
                        / self.format.sample_rate.get() as f32)
                        .sin()
            }
            Waveform::Impulse => {
                if n == 0 {
                    1.0
                } else {
                    0.0
                }
            }
            Waveform::Ramp => n as f32,
            Waveform::Dc(level) => level,
            Waveform::NonFinite => {
                if n < 16 {
                    f32::NAN
                } else if n < 32 {
                    f32::INFINITY
                } else {
                    0.5
                }
            }
        }
    }
}

impl Decoder for SyntheticDecoder {
    fn format(&self) -> AudioFormat {
        self.format
    }

    fn duration_frames(&self) -> Option<u64> {
        Some(self.total_frames)
    }

    fn decode_next(&mut self, out: &mut PcmBuffer) -> Result<u64, AudioError> {
        if self.next_frame >= self.total_frames {
            return Ok(0);
        }
        let ch = usize::from(self.format.channels.get());
        let take = usize::try_from(self.total_frames - self.next_frame)
            .unwrap_or(usize::MAX)
            .min(self.chunk_frames);
        let mut buf = Vec::with_capacity(take * ch);
        for i in 0..take {
            let v = self.sample_value(self.next_frame + i as u64);
            for _ in 0..ch {
                buf.push(v);
            }
        }
        self.next_frame += take as u64;
        out.append_interleaved(self.format, &buf);
        Ok(take as u64)
    }

    fn seek_to_frame(&mut self, frame: u64) -> Result<(), AudioError> {
        self.next_frame = frame.min(self.total_frames);
        Ok(())
    }
}
