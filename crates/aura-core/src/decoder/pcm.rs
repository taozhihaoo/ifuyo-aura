//! 统一内部 PCM 表示（M1 §7）与基础声道映射（§36）。
//!
//! 内部 PCM：**交错 f32**。采样率与声道数由 [`AudioFormat`] 显式携带，
//! 全 pipeline 保持一致。NaN 必须被拦截（在进入输出前的 sanitize 阶段处理，
//! 见 output::OutputStage）；有限极值允许存在（limiter 属 M2）。

use crate::error::AudioError;
use crate::types::{AudioFormat, Channels};

/// 交错 f32 PCM 缓冲，预分配复用（decode 路径不逐包分配）。
#[derive(Debug, Clone, Default)]
pub struct PcmBuffer {
    format: Option<AudioFormat>,
    data: Vec<f32>,
}

impl PcmBuffer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            format: None,
            data: Vec::new(),
        }
    }

    /// 预分配容量（帧）。
    pub fn reserve_frames(&mut self, format: AudioFormat, frames: usize) {
        self.format = Some(format);
        self.data
            .reserve(frames * usize::from(format.channels.get()));
    }

    /// 追加交错 PCM；首次追加时锁定 format。
    pub fn append_interleaved(&mut self, format: AudioFormat, interleaved: &[f32]) {
        match self.format {
            None => self.format = Some(format),
            Some(f) => debug_assert_eq!(f, format, "PcmBuffer format must stay consistent"),
        }
        self.data.extend_from_slice(interleaved);
    }

    /// 取出全部数据（供下游消费），保留 format。
    pub fn take_data(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.data)
    }

    #[must_use]
    pub const fn data(&self) -> &Vec<f32> {
        &self.data
    }

    #[must_use]
    pub const fn format(&self) -> Option<AudioFormat> {
        self.format
    }

    #[must_use]
    pub fn frames(&self) -> u64 {
        match self.format {
            Some(f) => (self.data.len() / usize::from(f.channels.get())) as u64,
            None => 0,
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// 声道映射：把 `src`（`src_channels` 交错）映射为 `dst_channels`。
///
/// M1 支持：mono→mono、mono→stereo（左右复制）、stereo→stereo。
/// 其余组合返回 `UnsupportedFormat`（多声道 downmix 属后续里程碑）。
pub fn map_channels(
    src: &[f32],
    src_channels: Channels,
    dst_channels: Channels,
) -> Result<Vec<f32>, AudioError> {
    let sc = usize::from(src_channels.get());
    let dc = usize::from(dst_channels.get());
    let src_frames = src.len() / sc;
    if !src.len().is_multiple_of(sc) {
        return Err(AudioError::InvalidFormat("interleaved pcm length mismatch"));
    }
    match (sc, dc) {
        (a, b) if a == b => Ok(src.to_vec()),
        (1, 2) => {
            let mut out = Vec::with_capacity(src_frames * 2);
            for frame in src.chunks_exact(1) {
                out.push(frame[0]);
                out.push(frame[0]);
            }
            Ok(out)
        }
        _ => Err(AudioError::UnsupportedFormat(
            "channel mapping unsupported in M1",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SampleRate;

    fn fmt(ch: u16) -> AudioFormat {
        AudioFormat::new(SampleRate::new(48_000).unwrap(), Channels::new(ch).unwrap())
    }

    #[test]
    fn pcm_buffer_tracks_frames_and_format() {
        let mut buf = PcmBuffer::new();
        buf.reserve_frames(fmt(2), 1024);
        buf.append_interleaved(fmt(2), &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
        assert_eq!(buf.frames(), 3);
        assert_eq!(buf.format(), Some(fmt(2)));
        buf.clear();
        assert_eq!(buf.frames(), 0);
    }

    #[test]
    fn mono_to_stereo_duplicates() {
        let out = map_channels(
            &[0.5, -0.5],
            Channels::new(1).unwrap(),
            Channels::new(2).unwrap(),
        )
        .unwrap();
        assert_eq!(out, vec![0.5, 0.5, -0.5, -0.5]);
    }

    #[test]
    fn same_channel_count_passthrough() {
        let src = [0.1, 0.2, 0.3, 0.4];
        let out = map_channels(&src, Channels::new(2).unwrap(), Channels::new(2).unwrap()).unwrap();
        assert_eq!(out, src);
    }

    #[test]
    fn unsupported_mapping_rejected() {
        assert!(
            map_channels(&[0.1], Channels::new(2).unwrap(), Channels::new(1).unwrap()).is_err()
        );
    }
}
