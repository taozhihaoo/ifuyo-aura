//! FLAC 真实解码路径（M1 §9）。
//!
//! 基于具体类型 `symphonia_bundle_flac::{FlacReader, FlacDecoder}`（M0 已锁定
//! 0.6.1），不经通用 probe/registry——M1 只支持 FLAC，具体类型最稳定且零 API 漂移。
//! 决策记录见 DECISIONS.md D-015。

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, SeekedTo};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::units::Timestamp;
use symphonia_bundle_flac::{FlacDecoder as SymphoniaFlacDecoder, FlacReader};

use crate::decoder::pcm::PcmBuffer;
use crate::decoder::Decoder;
use crate::error::AudioError;
use crate::types::{AudioFormat, Channels, SampleRate};

/// FLAC 解码器。数据源为内存中的 `MediaSource`（文件读取发生在本 crate 之上）。
pub struct FlacDecoder {
    format: AudioFormat,
    duration_frames: Option<u64>,
    track_id: u32,
    reader: FlacReader<'static>,
    decoder: SymphoniaFlacDecoder,
    scratch: Vec<f32>,
    pending_discard: u64,
    eof: bool,
}

impl FlacDecoder {
    /// 从内存媒体数据构造。非 FLAC 输入返回 `InvalidFormat`。
    pub fn from_media_source(source: Box<dyn MediaSource>) -> Result<Self, AudioError> {
        let mss = MediaSourceStream::new(source, Default::default());
        let reader = FlacReader::try_new(mss, FormatOptions::default())
            .map_err(|_| AudioError::InvalidFormat("not a valid FLAC stream"))?;

        let track = reader
            .tracks()
            .first()
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
        let duration_frames = track.num_frames;
        let time_base = track.time_base;

        let decoder_opts = {
            let mut opts = AudioDecoderOptions::default();
            opts.gapless = true;
            opts.verify = false;
            opts
        };
        let decoder = SymphoniaFlacDecoder::try_new(audio_params, &decoder_opts)
            .map_err(|_| AudioError::DecodeError("flac codec init failed"))?;

        let channel_count =
            u16::try_from(channels).map_err(|_| AudioError::InvalidFormat("too many channels"))?;
        let format = AudioFormat::new(SampleRate::new(sample_rate)?, Channels::new(channel_count)?);

        let _ = time_base; // FLAC timebase 为 1/sample_rate：timestamp 即帧号（seeked_to_frames 依赖此）

        Ok(Self {
            format,
            duration_frames,
            track_id: track.id,
            reader,
            decoder,
            scratch: Vec::new(),
            pending_discard: 0,
            eof: false,
        })
    }

    /// 便捷构造：内存字节。
    pub fn from_bytes(data: Vec<u8>) -> Result<Self, AudioError> {
        Self::from_media_source(Box::new(std::io::Cursor::new(data)))
    }

    fn seeked_to_frames(&self, seeked: SeekedTo) -> u64 {
        // FLAC 的 timebase 为 1/sample_rate：timestamp 即帧号
        let required = seeked.required_ts.get().max(0) as u64;
        let actual = seeked.actual_ts.get().max(0) as u64;
        required.saturating_sub(actual)
    }
}

impl Decoder for FlacDecoder {
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
                Err(_) => return Err(AudioError::DecodeError("next packet failed")),
            };

            let decoded = match self.decoder.decode_ref(&packet.as_packet_ref()) {
                Ok(d) => d,
                Err(_) => continue, // 单包损坏不致命：跳过（DECISIONS D-016）
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
            out.append_interleaved(self.format, &self.scratch[start..end]);
            returned += take_frames;

            if returned > 0 {
                return Ok(returned);
            }
        }
    }

    fn seek_to_frame(&mut self, frame: u64) -> Result<(), AudioError> {
        let seeked = self
            .reader
            .seek(
                SeekMode::Coarse,
                SeekTo::Timestamp {
                    ts: Timestamp::new(frame as i64),
                    track_id: self.track_id,
                },
            )
            .map_err(|_| AudioError::SeekError("format seek failed"))?;
        self.decoder.reset();
        self.eof = false;
        self.pending_discard = self.seeked_to_frames(seeked);
        Ok(())
    }
}
