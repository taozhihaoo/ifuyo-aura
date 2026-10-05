//! 通用 probe 解码器（M4 §55/§58/§77/§78）：任意 `AudioByteStream`
//! （本地文件 / 网络 HTTP / ICY 剥离流）→ symphonia probe → 解码。
//!
//! 与 [`crate::decoder::flac::FlacDecoder`]（具体类型，M1）互补：
//! 网络来源的容器/codec 无法预知，走 probe + 默认 codec registry。
//! 不支持的 codec → `UnsupportedFormat`（§78：不自动换源、不伪造）。
//!
//! source 层实现 [`AudioByteStream`]（如 aura-sources 的
//! `NetworkAudioStream`），应用层负责把 Resolved 接到这里。
//! 单包损坏跳过沿用 D-016（流式容错）。

use std::io::Read;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, SeekedTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Timestamp;

use crate::decoder::pcm::PcmBuffer;
use crate::decoder::Decoder;
use crate::error::AudioError;
use crate::types::{AudioFormat, Channels, SampleRate};

/// 面向 audio core 的字节流抽象：Read + Seek + 能力声明。
/// aura-sources 的网络流实现本 trait（§55 边界：core 不认识 reqwest）。
pub trait AudioByteStream: Read + std::io::Seek + Send + Sync {
    /// 是否可 seek（网络 live 流 = false）。
    fn is_seekable(&self) -> bool;

    /// 字节长度（未知 → None）。
    fn byte_len(&self) -> Option<u64>;
}

/// symphonia `MediaSource` 适配（§55：source ↔ core 的唯一交汇点）。
struct MediaSourceAdapter(Box<dyn AudioByteStream>);

impl Read for MediaSourceAdapter {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl std::io::Seek for MediaSourceAdapter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(pos)
    }
}

impl symphonia::core::io::MediaSource for MediaSourceAdapter {
    fn is_seekable(&self) -> bool {
        self.0.is_seekable()
    }

    fn byte_len(&self) -> Option<u64> {
        self.0.byte_len()
    }
}

/// 通用流解码器。
pub struct StreamingDecoder {
    format: AudioFormat,
    duration_frames: Option<u64>,
    track_id: u32,
    reader: Box<dyn FormatReader + 'static>,
    decoder: Box<dyn AudioDecoder>,
    scratch: Vec<f32>,
    pending_discard: u64,
    eof: bool,
}

impl StreamingDecoder {
    /// 打开并 probe。`hint_ext` 为扩展名提示（如 "mp3"/"ogg"）；
    /// 解不出任何已知格式 → `InvalidFormat`。
    ///
    /// # Errors
    /// [`AudioError::InvalidFormat`] / [`AudioError::UnsupportedFormat`]。
    pub fn open(
        source: Box<dyn AudioByteStream>,
        hint_ext: Option<&str>,
    ) -> Result<Self, AudioError> {
        let mut hint = Hint::new();
        if let Some(ext) = hint_ext {
            let owned = ext.to_ascii_lowercase();
            hint.with_extension(&owned);
        }
        let mss = MediaSourceStream::new(Box::new(MediaSourceAdapter(source)), Default::default());
        let reader = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|_| AudioError::InvalidFormat("no recognized audio format in stream"))?;

        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.is_some())
            .ok_or(AudioError::InvalidFormat("no audio track"))?
            .clone();

        let audio_params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or(AudioError::InvalidFormat("no audio codec parameters"))?;
        let sample_rate = audio_params
            .sample_rate
            .ok_or(AudioError::InvalidFormat("missing sample rate"))?;
        let channels = audio_params
            .channels
            .as_ref()
            .map(|c| c.count())
            .ok_or(AudioError::InvalidFormat("missing channel count"))?;
        let channel_count = u16::try_from(channels)
            .map_err(|_| AudioError::InvalidFormat("channel count out of range"))?;
        if channel_count == 0 || sample_rate == 0 {
            return Err(AudioError::UnsupportedFormat(
                "zero channels or sample rate",
            ));
        }

        let codec_params = track
            .codec_params
            .as_ref()
            .ok_or(AudioError::InvalidFormat("no codec parameters"))?;
        let audio_codec_params = codec_params
            .audio()
            .ok_or(AudioError::InvalidFormat("no audio codec parameters"))?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(audio_codec_params, &AudioDecoderOptions::default())
            .map_err(|_| AudioError::UnsupportedFormat("codec not available in this build"))?;

        Ok(Self {
            format: AudioFormat::new(SampleRate::new(sample_rate)?, Channels::new(channel_count)?),
            duration_frames: track.num_frames,
            track_id: track.id,
            reader,
            decoder,
            scratch: Vec::new(),
            pending_discard: 0,
            eof: false,
        })
    }

    /// 从 URL/路径推断 probe hint（§77：仅提示，不是 codec 保证）。
    #[must_use]
    pub fn hint_from_url(url: &str) -> Option<String> {
        let path = url.split(['?', '#']).next().unwrap_or(url);
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
        let ext = file.rsplit('.').next()?;
        if ext.is_empty() || ext.len() > 5 || ext == file {
            return None;
        }
        Some(ext.to_ascii_lowercase())
    }
}

impl Decoder for StreamingDecoder {
    fn format(&self) -> AudioFormat {
        self.format
    }

    fn duration_frames(&self) -> Option<u64> {
        self.duration_frames
    }

    fn decode_next(&mut self, out: &mut PcmBuffer) -> Result<u64, AudioError> {
        if self.eof {
            return Ok(0);
        }

        let mut returned: u64 = 0;
        let mut discarded: u64 = 0;

        loop {
            let packet = match self.reader.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => {
                    self.eof = true;
                    return Ok(returned);
                }
                // symphonia 以 IoError(UnexpectedEof) 报告流结束（含 live EOF）
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    self.eof = true;
                    return Ok(returned);
                }
                Err(symphonia::core::errors::Error::ResetRequired) => {
                    self.eof = true;
                    return Ok(returned);
                }
                Err(_) => return Err(AudioError::DecodeError("next packet failed")),
            };
            if packet.track_id != self.track_id {
                continue;
            }

            let decoded = match self.decoder.decode_ref(&packet.as_packet_ref()) {
                Ok(d) => d,
                Err(_) => continue, // 单包损坏不致命（D-016）
            };

            let frames_in_packet = decoded.frames() as u64;
            let mut take_from = 0usize;
            let mut take_frames = frames_in_packet;

            if discarded < self.pending_discard {
                let still_needed = self.pending_discard - discarded;
                if frames_in_packet <= still_needed {
                    discarded += frames_in_packet;
                    continue;
                }
                let skip = usize::try_from(still_needed).unwrap_or(0);
                take_from = skip;
                take_frames = frames_in_packet - still_needed;
                discarded += still_needed;
                self.pending_discard = 0;
            }

            if take_frames == 0 {
                continue;
            }

            self.scratch.clear();
            decoded.copy_to_vec_interleaved::<f32>(&mut self.scratch);
            let ch = usize::from(self.format.channels.get());
            let start = take_from * ch;
            let end = start + take_frames as usize * ch;
            let end = end.min(self.scratch.len());
            out.append_interleaved(self.format, &self.scratch[start..end]);
            returned += take_frames;

            if returned > 0 {
                return Ok(returned);
            }
        }
    }

    fn seek_to_frame(&mut self, frame: u64) -> Result<(), AudioError> {
        let seeked: SeekedTo = self
            .reader
            .seek(
                SeekMode::Coarse,
                SeekTo::Timestamp {
                    ts: Timestamp::new(i64::try_from(frame).unwrap_or(i64::MAX)),
                    track_id: self.track_id,
                },
            )
            .map_err(|_| AudioError::SeekError("format seek failed"))?;
        self.decoder.reset();
        self.eof = false;
        let actual = seeked.actual_ts.get().max(0) as u64;
        eprintln!(
            "[seek-probe] seek_to_frame({frame}) -> actual_ts={actual} pending_discard={}",
            frame.saturating_sub(actual)
        );
        self.pending_discard = frame.saturating_sub(actual);
        Ok(())
    }
}

/// 内存字节流：测试与工具用（公开导出供 source 层集成测试）。
pub struct MemoryStream {
    cursor: std::io::Cursor<Vec<u8>>,
    seekable: bool,
}

impl MemoryStream {
    #[must_use]
    pub fn new(data: Vec<u8>, seekable: bool) -> Self {
        Self {
            cursor: std::io::Cursor::new(data),
            seekable,
        }
    }
}

impl Read for MemoryStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl std::io::Seek for MemoryStream {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        if !self.seekable {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "not seekable",
            ));
        }
        self.cursor.seek(pos)
    }
}

impl AudioByteStream for MemoryStream {
    fn is_seekable(&self) -> bool {
        self.seekable
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.cursor.get_ref().len() as u64)
    }
}
