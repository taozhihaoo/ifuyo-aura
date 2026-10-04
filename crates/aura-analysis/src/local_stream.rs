//! 本地文件字节流（§12 analysis-side adapter；与 M1 LocalFileStream 同
//! 形态，避免依赖 aura-core 之外的生产播放流实现）。

use std::io::{Read, Seek};

/// 本地文件流：M1 [`AudioByteStream`](aura_core::decoder::AudioByteStream)
/// 适配。
pub struct LocalFileStream {
    file: std::fs::File,
    len: u64,
}

impl LocalFileStream {
    /// # Errors
    /// 文件不存在/不可读。
    pub fn open(path: &std::path::Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }
}

impl Read for LocalFileStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for LocalFileStream {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}

impl aura_core::decoder::AudioByteStream for LocalFileStream {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}
