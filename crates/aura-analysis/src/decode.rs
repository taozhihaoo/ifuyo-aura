//! 解码适配器（§12/§34）：复用 M1 `StreamingDecoder`，不复制 codec 代码。
//! 输出单声道 f32 PCM（嵌入/特征输入，§43）。多声道 ≥3 时 M1 无 downmix
//! ——返回 `DecodeFailed`（不偷渡新 codec 能力）。

use aura_core::decoder::streaming::StreamingDecoder;
use aura_core::decoder::{AudioByteStream, Decoder};

use crate::domain::AnalysisError;
use crate::embedding::EmbeddingInput;

/// 解码结果：单声道 f32 + 采样率。
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedMono {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl From<DecodedMono> for EmbeddingInput {
    fn from(d: DecodedMono) -> Self {
        EmbeddingInput::new(d.samples, d.sample_rate)
    }
}

/// 把任意字节流经 M1 解码为单声道 PCM（全长）。
///
/// # Errors
/// [`AnalysisError::DecodeFailed`]：decoder 打开失败 / M1 不支持的声道
/// 数（>2 无 downmix）/ 解码中途失败。
pub fn decode_to_mono(stream: Box<dyn AudioByteStream>) -> Result<DecodedMono, AnalysisError> {
    let mut decoder =
        StreamingDecoder::open(stream, None).map_err(|e| AnalysisError::DecodeFailed {
            reason: e.to_string(),
        })?;
    let format = decoder.format();
    let channels = format.channels.get();
    if channels > 2 {
        return Err(AnalysisError::DecodeFailed {
            reason: format!("{channels}-channel downmix unsupported (M1 capability)"),
        });
    }
    let sample_rate = format.sample_rate.get();

    let mut mono = Vec::new();
    let mut buf = aura_core::decoder::pcm::PcmBuffer::new();
    loop {
        let n = decoder
            .decode_next(&mut buf)
            .map_err(|e| AnalysisError::DecodeFailed {
                reason: e.to_string(),
            })?;
        if n == 0 {
            break; // EOF（§110）
        }
        let data = buf.data();
        if channels == 2 {
            // 交错 stereo → mono（均值 downmix，自有实现）。
            for frame in data.chunks_exact(2) {
                mono.push((frame[0] + frame[1]) * 0.5);
            }
        } else {
            mono.extend_from_slice(data);
        }
        buf.clear();
    }
    Ok(DecodedMono {
        samples: mono,
        sample_rate,
    })
}

/// 本地文件便捷入口（LocalFileStream 经 M1）。
///
/// # Errors
/// 文件打开失败 → [`AnalysisError::DecodeFailed`]。
pub fn decode_file_to_mono(path: &std::path::Path) -> Result<DecodedMono, AnalysisError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let stream = crate::local_stream::LocalFileStream::open(path).map_err(|e| {
        AnalysisError::DecodeFailed {
            reason: e.to_string(),
        }
    })?;
    let mut decoder = StreamingDecoder::open(Box::new(stream), ext.as_deref()).map_err(|e| {
        AnalysisError::DecodeFailed {
            reason: e.to_string(),
        }
    })?;
    let format = decoder.format();
    let channels = format.channels.get();
    if channels > 2 {
        return Err(AnalysisError::DecodeFailed {
            reason: format!("{channels}-channel downmix unsupported (M1 capability)"),
        });
    }
    let sample_rate = format.sample_rate.get();
    let mut mono = Vec::new();
    let mut buf = aura_core::decoder::pcm::PcmBuffer::new();
    loop {
        let n = decoder
            .decode_next(&mut buf)
            .map_err(|e| AnalysisError::DecodeFailed {
                reason: e.to_string(),
            })?;
        if n == 0 {
            break;
        }
        let data = buf.data();
        if channels == 2 {
            for frame in data.chunks_exact(2) {
                mono.push((frame[0] + frame[1]) * 0.5);
            }
        } else {
            mono.extend_from_slice(data);
        }
        buf.clear();
    }
    Ok(DecodedMono {
        samples: mono,
        sample_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 内存 WAV（16-bit PCM mono/stereo，自有最小编码器，测试专用）。
    fn wav_bytes(samples: &[i16], channels: u16, sample_rate: u32) -> Vec<u8> {
        let data_len = samples.len() * 2;
        let mut v = Vec::with_capacity(44 + data_len);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&sample_rate.to_le_bytes());
        let byte_rate = sample_rate * u32::from(channels) * 2;
        v.extend_from_slice(&byte_rate.to_le_bytes());
        let block_align = channels * 2;
        v.extend_from_slice(&block_align.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    struct MemStream(std::io::Cursor<Vec<u8>>);
    impl std::io::Read for MemStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl std::io::Seek for MemStream {
        fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
            self.0.seek(pos)
        }
    }
    impl AudioByteStream for MemStream {
        fn is_seekable(&self) -> bool {
            true
        }
        fn byte_len(&self) -> Option<u64> {
            Some(self.0.get_ref().len() as u64)
        }
    }

    #[test]
    fn decodes_mono_wav_via_m1() {
        // 8 采样正弦（i16），mono 8k。
        let samples: Vec<i16> = (0..800)
            .map(|i| {
                let amp = (i16::MAX as f32) * 0.5;
                ((std::f32::consts::TAU * 440.0 * i as f32 / 8000.0).sin() * amp) as i16
            })
            .collect();
        let wav = wav_bytes(&samples, 1, 8_000);
        let decoded = decode_to_mono(Box::new(MemStream(std::io::Cursor::new(wav)))).unwrap();
        assert_eq!(decoded.sample_rate, 8_000);
        assert_eq!(decoded.samples.len(), 800);
        // 非零（真实解码非静默）。
        assert!(decoded.samples.iter().any(|v| v.abs() > 0.01));
    }

    #[test]
    fn decodes_stereo_wav_with_downmix() {
        // stereo：L=+0.5、R=-0.25 交替块 → mono 均值。
        let mut samples = Vec::new();
        for _ in 0..400 {
            samples.push(16_384); // ≈ +0.5
            samples.push(-8_192); // ≈ -0.25
        }
        let wav = wav_bytes(&samples, 2, 8_000);
        let decoded = decode_to_mono(Box::new(MemStream(std::io::Cursor::new(wav)))).unwrap();
        assert_eq!(decoded.samples.len(), 400);
        let expect = (16_384.0 - 8_192.0) / 2.0 / 32_768.0;
        assert!((decoded.samples[0] - expect).abs() < 1e-3, "均值 downmix");
    }

    #[test]
    fn garbage_input_reports_decode_failed() {
        let err =
            decode_to_mono(Box::new(MemStream(std::io::Cursor::new(vec![0u8; 64])))).unwrap_err();
        assert!(matches!(err, AnalysisError::DecodeFailed { .. }));
    }

    #[test]
    fn embedding_input_conversion() {
        let d = DecodedMono {
            samples: vec![0.1; 100],
            sample_rate: 8_000,
        };
        let e: EmbeddingInput = d.into();
        assert_eq!(e.sample_rate, 8_000);
        assert_eq!(e.samples.len(), 100);
    }
}
