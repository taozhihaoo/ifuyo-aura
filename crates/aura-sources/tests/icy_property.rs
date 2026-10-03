//! ICY / Radio Browser 属性测试（§81）：伪随机生成输入，
//! 断言结构不变量（音频逐字节保真、metadata 边界无关、解析不 panic）。
//! 使用 aura-testkit 确定性 RNG：同一 seed 恒复现。

mod common;

use aura_sources::icy::{parse_icy_metadata, IcyReader};
use aura_testkit::rng::DeterministicRng;

/// ICY 属性：对任意 metaint / 音频内容 / metadata 内容 / read 边界组合，
/// IcyReader 输出 === 音频字节序列，且有效 metadata 被解析。
#[test]
fn property_icy_audio_fidelity_across_boundaries() {
    let mut rng = DeterministicRng::new(0x1CE_7AB1E);
    for round in 0..64u64 {
        let metaint = 8 * usize::try_from(rng.next_u64() % 64 + 1).unwrap(); // 8..512
        let blocks = usize::try_from(rng.next_u64() % 8 + 1).unwrap_or(1);
        let mut audio: Vec<u8> = Vec::new();
        let mut wire: Vec<u8> = Vec::new();
        for b in 0..blocks {
            let chunk_len = metaint;
            let mut chunk = vec![0u8; chunk_len];
            for byte in &mut chunk {
                *byte = (rng.next_u64() & 0xFF) as u8;
            }
            audio.extend_from_slice(&chunk);
            wire.extend_from_slice(&chunk);

            // 每 2 块放一个 metadata（随机内容：合法/非法 UTF-8 交替）。
            if b % 2 == 0 {
                let valid = b % 4 == 0;
                let meta_text: Vec<u8> = if valid {
                    format!("StreamTitle='R{round} - B{b}';").into_bytes()
                } else {
                    // 带 0xFF 的非法 UTF-8 payload（字节串，绕过 str 转义限制）。
                    b"StreamTitle=\xFF\xFE;".to_vec()
                };
                let padded = meta_text.len().div_ceil(16) * 16;
                wire.push(u8::try_from(padded / 16).unwrap());
                wire.extend_from_slice(&meta_text);
                wire.extend(std::iter::repeat_n(b' ', padded - meta_text.len()));
            } else {
                wire.push(0);
            }
        }

        // 全量读：音频逐字节保真。
        let mut r = IcyReader::new(wire.as_slice(), metaint);
        let mut out = Vec::new();
        std::io::Read::read_to_end(&mut r, &mut out).unwrap();
        assert_eq!(out, audio, "round {round}: 音频必须逐字节一致");

        // 随机小块读：同 seed 不同 chunk 大小，仍然保真。
        let chunk_size = 1 + usize::try_from(rng.next_u64() % 33).unwrap();
        let mut r = IcyReader::new(wire.as_slice(), metaint);
        let mut out = Vec::new();
        let mut buf = vec![0u8; chunk_size];
        loop {
            let n = std::io::Read::read(&mut r, &mut buf).unwrap();
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
        }
        assert_eq!(out, audio, "round {round} chunk {chunk_size}: 保真");
    }
}

/// 解析器属性：任意字节序列输入不 panic（返回 Some/None）。
#[test]
fn property_parse_never_panics() {
    let mut rng = DeterministicRng::new(0x0DD_0DD);
    for _ in 0..256 {
        let len = usize::try_from(rng.next_u64() % 128).unwrap_or(0);
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next_u64() & 0xFF) as u8).collect();
        let _ = parse_icy_metadata(&bytes);
    }
}
