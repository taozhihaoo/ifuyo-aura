//! 合成音频夹具与 NullOutput（无设备音频输出）。
//!
//! 规格依据（Charter §6 / §8.3）：
//! - 合成音频：正弦、静音、直流等确定性 PCM 生成器（float32 单声道交错帧）；
//! - NullOutput：无声卡环境下接收 PCM，统计已消费帧与 underrun；
//!   underflow 语义与真实输出一致：输出静音、计数、可恢复、绝不 panic。

/// 生成 n_samples 个单声道 f32 采样的正弦波。
#[must_use]
pub fn sine_pcm(sample_rate: u32, freq_hz: f32, n_samples: usize) -> Vec<f32> {
    let omega = std::f32::consts::TAU * freq_hz / sample_rate as f32;
    (0..n_samples).map(|i| (omega * i as f32).sin()).collect()
}

/// 生成静音 PCM。
#[must_use]
pub fn silence_pcm(n_samples: usize) -> Vec<f32> {
    vec![0.0; n_samples]
}

/// 生成直流电平 PCM（用于增益/限幅类测试）。
#[must_use]
pub fn dc_pcm(n_samples: usize, level: f32) -> Vec<f32> {
    vec![level; n_samples]
}

/// 无设备音频输出：消费 PCM 并做帧/underrun 记账。
///
/// 帧消费按 `pcm.len() / channels` 计算（交错帧布局）。
/// 与真实输出的契约一致：underrun 不 panic、由测试显式标记、之后可继续写入。
#[derive(Debug)]
pub struct NullOutput {
    channels: u16,
    sample_rate: u32,
    frames_played: u64,
    underruns: u64,
}

impl NullOutput {
    #[must_use]
    pub const fn new(channels: u16, sample_rate: u32) -> Self {
        Self {
            channels,
            sample_rate,
            frames_played: 0,
            underruns: 0,
        }
    }

    #[must_use]
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// 写入交错 PCM；返回消费的帧数。非整帧长度的尾部采样被丢弃（与真实设备行为一致）。
    pub fn write(&mut self, pcm: &[f32]) -> u64 {
        let consumed = pcm.len() / usize::from(self.channels);
        self.frames_played += consumed as u64;
        consumed as u64
    }

    /// 标记一次 underrun（输出静音由测试断言 PCM 为零来模拟）。
    pub fn mark_underrun(&mut self) {
        self.underruns += 1;
    }

    #[must_use]
    pub const fn frames_played(&self) -> u64 {
        self.frames_played
    }

    #[must_use]
    pub const fn underruns(&self) -> u64 {
        self.underruns
    }

    pub fn reset(&mut self) {
        self.frames_played = 0;
        self.underruns = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_is_deterministic_and_bounded() {
        let a = sine_pcm(48_000, 440.0, 480);
        let b = sine_pcm(48_000, 440.0, 480);
        assert_eq!(a, b, "合成音频必须确定性");
        assert!(a.iter().all(|s| s.is_finite() && (-1.0..=1.0).contains(s)));
    }

    #[test]
    fn sine_frequency_peak_count() {
        // 48000Hz 采样、1000Hz、1 秒 → 1000 个完整周期，正峰 1000 个
        let pcm = sine_pcm(48_000, 1_000.0, 48_000);
        let peaks = pcm
            .windows(3)
            .filter(|w| w[0] < w[1] && w[1] >= w[2] && w[1] > 0.9)
            .count();
        assert_eq!(peaks, 1_000);
    }

    #[test]
    fn silence_and_dc() {
        assert!(silence_pcm(10).iter().all(|s| *s == 0.0));
        assert!(dc_pcm(10, 0.25).iter().all(|s| *s == 0.25));
    }

    #[test]
    fn null_output_counts_frames_and_discards_tail() {
        let mut out = NullOutput::new(2, 48_000);
        assert_eq!(out.write(&dc_pcm(2 * 100, 0.0)), 100);
        assert_eq!(out.write(&dc_pcm(3, 0.0)), 1); // 尾采样 1 个被丢弃
        assert_eq!(out.frames_played(), 101);
    }

    #[test]
    fn null_output_underrun_is_counted_and_recoverable() {
        let mut out = NullOutput::new(2, 48_000);
        out.mark_underrun();
        out.mark_underrun();
        assert_eq!(out.underruns(), 2);
        assert_eq!(out.write(&silence_pcm(2 * 10)), 10);
        assert_eq!(out.frames_played(), 10, "underrun 后继续写入可恢复");
        out.reset();
        assert_eq!(out.frames_played(), 0);
        assert_eq!(out.underruns(), 0);
    }
}
