//! ICY 协议（§31–§36/§102）：纯解析器 + 流式元数据剥离。
//!
//! 协议形状（icy-metaint = N）：
//! `audio(N bytes) [len_byte] [metadata(len×16 bytes)] audio ...`
//! len_byte = 0 表示无 metadata 块。剥离后音频逐字节透传；
//! metadata 解析失败仅丢弃该块，不影响音频（§36）。

use std::io::Read;

/// 元数据到达回调（feeder 线程调用；§35 事件通道的注入点）。
pub type MetaCallback = Box<dyn FnMut(&IcyMeta) + Send>;

use crate::model::StreamMetadataEvent;

/// metadata 块上限（§33）：长度字节协议上限 255 → 255×16 = 4080；
/// 超过协议形状的声明（不可能）按恶意处理并跳过。D-050。
pub const MAX_ICY_METADATA_BYTES: usize = 255 * 16;

/// 单块解析结果（§34）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IcyMeta {
    pub stream_title: Option<String>,
    pub stream_url: Option<String>,
}

/// 纯解析：`StreamTitle='Artist - Title';StreamUrl='...';`
/// - 非 UTF-8 → `None`（§36：忽略，不 panic）；
/// - 未知键 → 忽略；缺 StreamTitle → 空结构（不是错误）；
/// - 引号缺失/转义残缺 → 尽力提取，不 panic。
#[must_use]
pub fn parse_icy_metadata(raw: &[u8]) -> Option<IcyMeta> {
    let text = std::str::from_utf8(raw).ok()?;
    let mut out = IcyMeta::default();
    for segment in text.split(';') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let Some((key, value)) = segment.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('\'').trim_matches('"');
        match key.trim() {
            "StreamTitle" => {
                let v = sanitize(value);
                if !v.is_empty() {
                    out.stream_title = Some(v);
                }
            }
            "StreamUrl" => {
                let v = sanitize(value);
                if !v.is_empty() {
                    out.stream_url = Some(v);
                }
            }
            _ => {} // §80：未知字段忽略
        }
    }
    Some(out)
}

/// StreamTitle → (artist, title)：约定 `Artist - Title` 分隔；
/// 没有分隔符 → 全部算 title（不猜测 artist）。
#[must_use]
pub fn split_stream_title(title: &str) -> (Option<String>, Option<String>) {
    match title.split_once(" - ") {
        Some((artist, t)) if !artist.trim().is_empty() && !t.trim().is_empty() => {
            (Some(artist.trim().to_string()), Some(t.trim().to_string()))
        }
        _ => (None, Some(title.trim().to_string())),
    }
}

fn sanitize(v: &str) -> String {
    // 控制字符剔除；长度截断（防御畸形大值）。
    let cleaned: String = v.chars().filter(|c| !c.is_control()).collect();
    let mut s: String = cleaned.chars().take(512).collect();
    if cleaned.chars().count() > 512 {
        s.push('…');
    }
    s
}

/// StreamTitle → 上层事件（§35/§91）。
#[must_use]
pub fn meta_to_event(meta: &IcyMeta, station: Option<&str>) -> Option<StreamMetadataEvent> {
    let raw = meta.stream_title.as_deref()?;
    let (artist, title) = split_stream_title(raw);
    Some(StreamMetadataEvent {
        station: station.map(str::to_string),
        artist,
        title,
        raw: Some(raw.to_string()),
    })
}

/// 流式剥离器（§32）：任意 read 边界安全。
pub struct IcyReader<R: Read> {
    inner: R,
    metaint: usize,
    /// 当前 audio 块剩余字节。
    audio_left: usize,
    /// 剥离出的最新 metadata（消费侧读取）。
    pub last_meta: Option<IcyMeta>,
    /// 解析失败/跳过的 metadata 块计数（诊断）。
    pub malformed_meta_count: u64,
    pub on_meta: Option<MetaCallback>,
    eof: bool,
}

impl<R: Read> IcyReader<R> {
    /// `metaint` 来自响应头 `icy-metaint`（0 或缺失 = 纯音频直通）。
    #[must_use]
    pub fn new(inner: R, metaint: usize) -> Self {
        Self {
            inner,
            metaint,
            audio_left: metaint,
            last_meta: None,
            malformed_meta_count: 0,
            on_meta: None,
            eof: false,
        }
    }

    #[must_use]
    pub fn into_inner(self) -> R {
        self.inner
    }

    /// 读满长度字节 + metadata 块（任意边界安全）。
    fn read_metadata_block(&mut self) -> std::io::Result<()> {
        let mut len_byte = [0u8; 1];
        match self.inner.read_exact(&mut len_byte) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // 流在 metadata 边界结束：正常 EOF。
                self.eof = true;
                return Ok(());
            }
            Err(e) => return Err(e),
        }
        let len = usize::from(len_byte[0]) * 16;
        if len == 0 {
            return Ok(());
        }
        if len > MAX_ICY_METADATA_BYTES {
            // 协议上不可能；防御：跳过声明的字节数不可行（>4080 无法安全跳），
            // 视为流损坏，报告 EOF 防止误消费音频。
            self.malformed_meta_count += 1;
            self.eof = true;
            return Ok(());
        }
        let mut buf = vec![0u8; len];
        match self.inner.read_exact(&mut buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // 截断的 metadata：丢弃，流结束。
                self.malformed_meta_count += 1;
                self.eof = true;
                return Ok(());
            }
            Err(e) => return Err(e),
        }
        match parse_icy_metadata(&buf) {
            Some(m) => {
                if let Some(cb) = self.on_meta.as_mut() {
                    cb(&m);
                }
                self.last_meta = Some(m);
            }
            None => self.malformed_meta_count += 1,
        }
        Ok(())
    }
}

impl<R: Read> Read for IcyReader<R> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        if self.eof || out.is_empty() {
            return Ok(0);
        }
        if self.audio_left == 0 {
            self.read_metadata_block()?;
            if self.eof {
                return Ok(0);
            }
            self.audio_left = self.metaint;
        }
        let want = out.len().min(self.audio_left);
        let n = self.inner.read(&mut out[..want])?;
        if n == 0 {
            self.eof = true;
            return Ok(0);
        }
        self.audio_left -= n;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_stream_title() {
        let m = parse_icy_metadata(b"StreamTitle='Artist - Song';").unwrap();
        assert_eq!(m.stream_title.as_deref(), Some("Artist - Song"));
        assert_eq!(
            split_stream_title("Artist - Song").1.as_deref(),
            Some("Song")
        );
    }

    /// §102：invalid UTF-8 → 忽略（None）。
    #[test]
    fn invalid_utf8_ignored() {
        assert_eq!(parse_icy_metadata(&[b'S', b't', 0xFF, 0xFE]), None);
    }

    #[test]
    fn missing_title_and_unknown_keys() {
        let m = parse_icy_metadata(b"StreamUrl='http://x';Unknown='y';").unwrap();
        assert!(m.stream_title.is_none());
        assert_eq!(m.stream_url.as_deref(), Some("http://x"));
        // 空块 / 无引号 / 乱片段：不 panic。
        assert!(parse_icy_metadata(b"").is_some());
        assert!(parse_icy_metadata(b"StreamTitle=noquotes;").is_some());
        assert!(parse_icy_metadata(b"garbage'x'=;;;").is_some());
    }

    #[test]
    fn event_conversion_and_artist_split() {
        let m = IcyMeta {
            stream_title: Some("周杰伦 - 夜曲".into()),
            stream_url: None,
        };
        let ev = meta_to_event(&m, Some("Radio X")).unwrap();
        assert_eq!(ev.station.as_deref(), Some("Radio X"));
        assert_eq!(ev.artist.as_deref(), Some("周杰伦"));
        assert_eq!(ev.title.as_deref(), Some("夜曲"));
        // 无 " - " 分隔：不猜 artist。
        let ev2 = meta_to_event(
            &IcyMeta {
                stream_title: Some("Just A Song".into()),
                stream_url: None,
            },
            None,
        )
        .unwrap();
        assert!(ev2.artist.is_none());
        assert_eq!(ev2.title.as_deref(), Some("Just A Song"));
    }

    /// 合成 ICY 字节流：audio + metadata 交替。
    fn make_stream(metaint: usize, audio_pattern: u8, meta: &[u8], blocks: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for i in 0..blocks {
            out.extend(std::iter::repeat_n(
                audio_pattern.wrapping_add(i as u8),
                metaint,
            ));
            let padded = meta.len().div_ceil(16) * 16;
            out.push(u8::try_from(padded / 16).unwrap());
            out.extend_from_slice(meta);
            out.extend(std::iter::repeat_n(b' ', padded - meta.len()));
        }
        out
    }

    /// §32/§86：任意 read 边界 → 音频逐字节恢复 + metadata 全部解析。
    #[test]
    fn arbitrary_chunk_boundaries_strip_metadata() {
        let metaint = 10;
        let meta = b"StreamTitle='A - B';";
        let wire = make_stream(metaint, 0x40, meta, 5);
        let expected_audio: Vec<u8> = (0..5u8)
            .flat_map(|i| std::iter::repeat_n(0x40u8.wrapping_add(i), metaint))
            .collect();

        // 一把读。
        let wire_copy = wire.clone();
        let mut r = IcyReader::new(wire_copy.as_slice(), metaint);
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out, expected_audio);
        assert_eq!(
            r.last_meta.as_ref().unwrap().stream_title.as_deref(),
            Some("A - B")
        );

        // 逐 1/2/3/7 字节读（§86）。
        for chunk in [1usize, 2, 3, 7] {
            let mut r = IcyReader::new(wire.as_slice(), metaint);
            let mut out = Vec::new();
            let mut buf = [0u8; 64];
            loop {
                let end = chunk.min(buf.len());
                let n = r.read(&mut buf[..end]).unwrap();
                if n == 0 {
                    break;
                }
                out.extend_from_slice(&buf[..n]);
            }
            assert_eq!(out, expected_audio, "chunk={chunk}");
        }
    }

    /// §102：zero metadata length。
    #[test]
    fn zero_length_metadata_blocks() {
        let metaint = 4;
        let mut wire = Vec::new();
        for i in 0..3u8 {
            wire.extend([i, i, i, i]);
            wire.push(0);
        }
        let mut r = IcyReader::new(wire.as_slice(), metaint);
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out.len(), 12);
        assert!(r.last_meta.is_none());
    }

    /// §102：多块 metadata 变化 → last_meta 跟随最后一次。
    #[test]
    fn multiple_metadata_updates() {
        let metaint = 8;
        let mut wire = Vec::new();
        for title in ["StreamTitle='One';", "StreamTitle='Two - x';"] {
            wire.extend([0u8; 8]);
            let padded = title.len().div_ceil(16) * 16;
            wire.push(u8::try_from(padded / 16).unwrap());
            wire.extend_from_slice(title.as_bytes());
            wire.extend(std::iter::repeat_n(b' ', padded - title.len()));
        }
        let mut r = IcyReader::new(wire.as_slice(), metaint);
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out.len(), 16);
        assert_eq!(
            r.last_meta.unwrap().stream_title.as_deref(),
            Some("Two - x")
        );
    }

    /// §36：截断的 metadata → 丢弃块，音频不误读。
    #[test]
    fn truncated_metadata_is_dropped() {
        let metaint = 4;
        let mut wire = vec![1, 2, 3, 4];
        wire.push(2); // 声明 32 字节 metadata
        wire.extend_from_slice(b"StreamTitle='cut"); // 只有 16 字节就 EOF
        let mut r = IcyReader::new(wire.as_slice(), metaint);
        let mut out = Vec::new();
        r.read_to_end(&mut out).unwrap();
        assert_eq!(out, vec![1, 2, 3, 4], "只有音频字节");
        assert_eq!(r.malformed_meta_count, 1);
    }
}
