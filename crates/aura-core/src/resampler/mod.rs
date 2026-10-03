//! 重采样（M1 §34–35）：rubato 5 `Async`（sinc 插值，固定输入分块）。
//!
//! - 采样率一致时**旁路**（不经重采样器，零开销零延迟）；
//! - 流式分块：每次喂 `input_frames_next()` 帧，取回 `output_frames_next()` 帧；
//! - EOF 尾块用 `partial_len` 喂入（rubato 以静音补齐缺失输入，见 D-024）；
//! - `process` 在解码线程执行，允许分配；输出回调不接触本模块。

use rubato::Resampler as _;
use rubato::{Async, FixedAsync, SincInterpolationParameters, WindowFunction};

use crate::error::AudioError;
use crate::types::{Channels, SampleRate};

/// 流式重采样器包装。
pub struct Resampler {
    inner: Async<f32>,
    channels: Channels,
    ratio: f64,
}

impl Resampler {
    /// 构造 from → to 的重采样器。相同采样率返回 `None`（调用方走旁路）。
    pub fn new(
        from: SampleRate,
        to: SampleRate,
        channels: Channels,
    ) -> Result<Option<Self>, AudioError> {
        let ratio = f64::from(to.get()) / f64::from(from.get());
        if (ratio - 1.0).abs() < f64::EPSILON {
            return Ok(None);
        }
        // 256 阶 sinc + Blackman2 窗：质量/开销平衡点（M1 基线，性能调优属后续）
        let params = SincInterpolationParameters::new(256, WindowFunction::Blackman2);
        let chunk_size = 1024;
        // 相对可调带宽 2.0：必须覆盖固定 ratio 本身（44.1k↔48k 的 ratio≈1.088）
        let inner = Async::new_sinc(
            ratio,
            2.0,
            &params,
            chunk_size,
            usize::from(channels.get()),
            FixedAsync::Input,
        )
        .map_err(|_e| AudioError::ResampleError("resampler construction failed"))?;
        Ok(Some(Self {
            inner,
            channels,
            ratio,
        }))
    }

    /// 下一次 `process_chunk` 需要的输入帧数。
    #[must_use]
    pub fn input_frames_next(&self) -> usize {
        self.inner.input_frames_next()
    }

    /// 本次处理将产出的输出帧数。
    #[must_use]
    pub fn output_frames_next(&self) -> usize {
        self.inner.output_frames_next()
    }

    /// 滤波器延迟（输出帧）。整段对齐/频谱测试需计入。
    #[must_use]
    pub fn output_delay(&self) -> usize {
        self.inner.output_delay()
    }

    #[must_use]
    pub const fn channels(&self) -> Channels {
        self.channels
    }

    /// 重采样比（output/input）。
    #[must_use]
    pub const fn ratio(&self) -> f64 {
        self.ratio
    }

    /// 处理一整块交错输入（长度须为 `input_frames_next()` 帧，EOF 尾块用
    /// [`Self::process_chunk_partial_into`]）。输出写入调用方复用的缓冲。
    pub fn process_chunk_into(
        &mut self,
        interleaved: &[f32],
        out_buf: &mut Vec<f32>,
    ) -> Result<(), AudioError> {
        self.process_inner(interleaved, None, out_buf)
    }

    /// EOF 尾块：输入不足一个完整分块时使用（补静音语义，见 D-024）。
    pub fn process_chunk_partial_into(
        &mut self,
        interleaved: &[f32],
        out_buf: &mut Vec<f32>,
    ) -> Result<(), AudioError> {
        self.process_inner(interleaved, Some(interleaved.len()), out_buf)
    }

    fn process_inner(
        &mut self,
        interleaved: &[f32],
        partial_samples: Option<usize>,
        out_buf: &mut Vec<f32>,
    ) -> Result<(), AudioError> {
        let ch = usize::from(self.channels.get());
        if !interleaved.len().is_multiple_of(ch) {
            return Err(AudioError::InvalidParameter(
                "interleaved pcm length mismatch",
            ));
        }
        let input = rubato::audioadapter_buffers::direct::InterleavedSlice::new(
            interleaved,
            ch,
            interleaved.len() / ch,
        )
        .map_err(|_| AudioError::InvalidParameter("resampler input shape invalid"))?;

        let indexing = partial_samples.map(|samples| rubato::Indexing {
            input_offset: 0,
            output_offset: 0,
            partial_len: Some(samples / ch),
            active_channels_mask: None,
        });
        // 输出写入调用方复用缓冲（process_into_buffer 消除每块分配，D-035）。
        // out_buf 同时充当 InterleavedOwned 底层存储（new_from 零复制包装）。
        let out_frames = self.inner.output_frames_next();
        out_buf.clear();
        out_buf.resize(out_frames * ch, 0.0);
        let mut output = rubato::audioadapter_buffers::owned::InterleavedOwned::new_from(
            std::mem::take(out_buf),
            ch,
            out_frames,
        )
        .map_err(|_| AudioError::InvalidParameter("resampler output shape invalid"))?;
        let (_in_used, out_len) = self
            .inner
            .process_into_buffer(&input, &mut output, indexing.as_ref())
            .map_err(|_| AudioError::ResampleError("resample failed"))?;
        *out_buf = output.take_data();
        out_buf.truncate(out_len * ch);
        Ok(())
    }

    /// 重置内部状态（seek 场景由新 generation 重建管线，一般无需调用）。
    pub fn reset(&mut self) {
        self.inner.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::pcm::map_channels;
    use crate::types::SampleRate;

    /// 生成单频正弦（单声道）。
    fn sine(sample_rate: u32, freq: f32, seconds: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * seconds) as usize;
        let omega = std::f32::consts::TAU * freq / sample_rate as f32;
        (0..n).map(|i| (omega * i as f32).sin()).collect()
    }

    /// 过零率估频（带迟滞，避免近零抖动重复计数；输入为**单声道**序列；
    /// 用于验证频率保持，避免主观听感断言）。
    fn estimate_freq(pcm: &[f32], sample_rate: u32) -> f64 {
        const HYSTERESIS: f32 = 0.1;
        let mut crossings = 0usize;
        let mut armed = false;
        for &s in pcm {
            if s > HYSTERESIS {
                if !armed {
                    crossings += 1;
                }
                armed = true;
            } else if s < -HYSTERESIS {
                armed = false;
            }
        }
        crossings as f64 * sample_rate as f64 / pcm.len() as f64
    }

    /// 从交错立体声中抽取左声道（估频输入须为单声道）。
    fn left_channel(interleaved: &[f32]) -> Vec<f32> {
        interleaved.chunks_exact(2).map(|c| c[0]).collect()
    }

    fn stereo(pcm: Vec<f32>) -> Vec<f32> {
        map_channels(&pcm, Channels::new(1).unwrap(), Channels::new(2).unwrap()).unwrap()
    }

    fn resample_all(from: u32, to: u32, input: Vec<f32>) -> (Vec<f32>, usize, usize) {
        let sr_from = SampleRate::new(from).unwrap();
        let sr_to = SampleRate::new(to).unwrap();
        let mut r = Resampler::new(sr_from, sr_to, Channels::new(2).unwrap())
            .unwrap()
            .expect("rates differ");
        let mut out = Vec::new();
        let mut out_buf = Vec::new();
        let input_frames = input.len() / 2;
        let mut fed = 0usize;
        while fed + r.input_frames_next() <= input_frames {
            let chunk = &input[fed * 2..(fed + r.input_frames_next()) * 2];
            r.process_chunk_into(chunk, &mut out_buf).unwrap();
            out.extend_from_slice(&out_buf);
            fed += r.input_frames_next();
        }
        if fed < input_frames {
            let tail = &input[fed * 2..];
            r.process_chunk_partial_into(tail, &mut out_buf).unwrap();
            out.extend_from_slice(&out_buf);
        }
        (out, r.output_delay(), input_frames)
    }

    #[test]
    fn bypass_when_rates_equal() {
        let sr = SampleRate::new(48_000).unwrap();
        assert!(Resampler::new(sr, sr, Channels::new(2).unwrap())
            .unwrap()
            .is_none());
    }

    #[test]
    fn frame_counts_follow_ratio() {
        for (from, to) in [
            (44_100u32, 48_000u32),
            (48_000, 44_100),
            (22_050, 48_000),
            (48_000, 22_050),
        ] {
            let input = stereo(sine(from, 1_000.0, 1.0));
            let input_frames = input.len() / 2;
            let (out, delay, _in_frames) = resample_all(from, to, input);
            let out_frames = out.len() / 2;
            // 有效输出 = 总输出 − sinc 滤波器固有的前导延迟
            let usable = out_frames.saturating_sub(delay) as f64;
            let expected = input_frames as f64 * f64::from(to) / f64::from(from);
            // 容差 2%：覆盖分块余量（最后一个输出分块不满）与首块量化差
            let tolerance = expected * 0.02;
            assert!(
                (usable - expected).abs() <= tolerance,
                "{from}->{to}: usable {usable} vs expected {expected}"
            );
        }
    }

    #[test]
    fn frequency_is_preserved() {
        for (from, to) in [(44_100u32, 48_000u32), (48_000, 22_050)] {
            let input = stereo(sine(from, 1_000.0, 1.0));
            let (out, delay, _) = resample_all(from, to, input);
            // 跳过滤波器延迟区与 EOF 静音补齐尾区，取中段左声道估频
            let start = (delay * 2).min(out.len());
            let end = out.len();
            let span = end - start;
            let body = left_channel(&out[start + span / 10..end - span / 10]);
            let est = estimate_freq(&body, to);
            assert!(
                (est - 1_000.0).abs() < 25.0,
                "{from}->{to}: estimated {est} Hz"
            );
        }
    }

    #[test]
    fn channels_stay_consistent() {
        let input = stereo(sine(44_100, 440.0, 0.5));
        let (out, _delay, _f) = resample_all(44_100, 48_000, input);
        assert_eq!(out.len() % 2, 0);
        // 左右声道原本相同 → 重采样后仍近似相同
        for chunk in out.chunks_exact(2).step_by(97) {
            assert!(
                (chunk[0] - chunk[1]).abs() < 1e-3,
                "L/R diverged: {:?}",
                chunk
            );
        }
    }

    #[test]
    fn determinism_same_input_same_output() {
        let input = stereo(sine(44_100, 1_000.0, 0.5));
        let a = resample_all(44_100, 48_000, input.clone());
        let b = resample_all(44_100, 48_000, input);
        assert_eq!(a, b);
    }
}
