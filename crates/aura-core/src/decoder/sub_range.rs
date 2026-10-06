//! M15a CUE 子轨解码器：把整轨解码器限制到 [start, start+duration) 帧区间。
//! 播放层在打开整轨后按 CUE 偏移包装——EOF/时长/seek 全部在子轨坐标系内。

use crate::decoder::{Decoder, PcmBuffer};
use crate::error::AudioError;
use crate::types::AudioFormat;

pub struct SubRangeDecoder {
    inner: Box<dyn Decoder>,
    fmt: AudioFormat,
    start_frame: u64,
    total_frames: u64,
    consumed: u64,
    scratch: PcmBuffer,
}

impl SubRangeDecoder {
    #[must_use]
    pub fn new(inner: Box<dyn Decoder>, start_frame: u64, total_frames: u64) -> Self {
        let fmt = inner.format();
        Self {
            inner,
            fmt,
            start_frame,
            total_frames,
            consumed: 0,
            scratch: PcmBuffer::new(),
        }
    }
}

impl Decoder for SubRangeDecoder {
    fn format(&self) -> AudioFormat {
        self.inner.format()
    }

    fn duration_frames(&self) -> Option<u64> {
        Some(self.total_frames)
    }

    fn decode_next(&mut self, out: &mut PcmBuffer) -> Result<u64, AudioError> {
        if self.consumed >= self.total_frames {
            return Ok(0);
        }
        let n = self.inner.decode_next(&mut self.scratch)?;
        if n == 0 {
            return Ok(0);
        }
        let take = n.min(self.total_frames - self.consumed);
        let ch = usize::from(self.fmt.channels.get());
        let mut data = self.scratch.take_data();
        data.truncate((take as usize).saturating_mul(ch));
        out.append_interleaved(self.fmt, &data);
        self.consumed += take;
        Ok(take)
    }

    fn seek_to_frame(&mut self, frame: u64) -> Result<(), AudioError> {
        self.inner
            .seek_to_frame(self.start_frame.saturating_add(frame))?;
        self.consumed = frame.min(self.total_frames);
        Ok(())
    }
}
