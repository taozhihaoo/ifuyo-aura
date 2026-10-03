//! 内容身份（§8/§9）：`file size + 首/尾 1MB` 的确定性 128 位指纹。
//!
//! 算法：SipHash-2-4 输出 128 位（`siphasher` crate，MIT/Apache-2.0，
//! 无传递依赖）。选择理由与 metadata-only 变更的分辨规则记录于
//! `DECISIONS.md`（D-037/D-038）：身份不含路径、不含标签字节，
//! 故改名/移动/标签改写均不变；音频内容替换经 `audio_sig` 分辨后生成新身份。

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use siphasher::sip128::SipHasher24;

use crate::error::{LibraryError, Result};
use crate::types::TrackIdentity;

/// 身份键版本前缀；指纹定义变更时必须升级。
pub const IDENTITY_VERSION: &str = "v1";

/// 首/尾采样窗口：1 MiB。
const CHUNK: u64 = 1 << 20;

/// 固定密钥（产品命名派生，非保密参数；仅保证跨版本稳定）。
const KEY: (u64, u64) = (0x6966_7579_6f5f_6175, 0x7261_2122_2324_2526);

/// 128 位指纹（SipHash-2-4）。
#[must_use]
pub fn fingerprint128(data: &[u8]) -> [u8; 16] {
    let out = SipHasher24::new_with_keys(KEY.0, KEY.1).hash(data);
    out.h1
        .to_le_bytes()
        .into_iter()
        .chain(out.h2.to_le_bytes())
        .collect::<Vec<_>>()[..16]
        .try_into()
        .expect("16 bytes")
}

/// 采样：`size <= 2MiB` 读全文件；否则头 1MiB + 尾 1MiB（中部不采样，
/// 性能与章程推荐的等价方案一致；碰撞与取舍见 D-037）。
fn sample_file(path: &Path, size: u64) -> Result<Vec<u8>> {
    let mut file = File::open(path).map_err(|e| LibraryError::Io {
        path: Some(path.to_path_buf()),
        source: e,
    })?;
    let read_len = if size <= 2 * CHUNK { size } else { 2 * CHUNK };
    let mut buf = Vec::with_capacity(read_len as usize);
    let head = CHUNK.min(size) as usize;
    read_at(&mut file, 0, head, &mut buf)?;
    if size > CHUNK {
        let tail_off = size - CHUNK;
        read_at(&mut file, tail_off, CHUNK as usize, &mut buf)?;
    }
    Ok(buf)
}

fn read_at(file: &mut File, offset: u64, len: usize, buf: &mut Vec<u8>) -> Result<()> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| LibraryError::Io {
            path: None,
            source: e,
        })?;
    let start = buf.len();
    buf.resize(start + len, 0);
    file.read_exact(&mut buf[start..])
        .map_err(|e| LibraryError::Io {
            path: None,
            source: e,
        })?;
    Ok(())
}

/// 身份键：`"v1:<size:016x>:<fp128 hex>"`（确定性；小写十六进制）。
#[must_use]
pub fn identity_key(size: u64, fp: &[u8; 16]) -> String {
    let mut hex = String::with_capacity(32);
    for b in fp {
        hex.push(char::from_digit(u32::from(b >> 4), 16).unwrap_or('0'));
        hex.push(char::from_digit(u32::from(b & 0xf), 16).unwrap_or('0'));
    }
    format!("{IDENTITY_VERSION}:{size:016x}:{hex}")
}

/// 读取文件并计算内容身份。`audio_sig` 来自标签解析（见
/// `TrackMetadata::audio_sig`），与内容指纹共同构成完整身份语义（§10）。
///
/// # Errors
/// 文件打不开/读不完时返回 [`LibraryError::Io`]。
pub fn content_identity(path: &Path, audio_sig: String) -> Result<TrackIdentity> {
    let size = std::fs::metadata(path)
        .map_err(|e| LibraryError::Io {
            path: Some(path.to_path_buf()),
            source: e,
        })?
        .len();
    let sample = sample_file(path, size)?;
    let fp = fingerprint128(&sample);
    Ok(TrackIdentity {
        identity_key: identity_key(size, &fp),
        audio_sig,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_deterministic_and_sensitive() {
        assert_eq!(fingerprint128(b"hello"), fingerprint128(b"hello"));
        assert_ne!(fingerprint128(b"hello"), fingerprint128(b"hellp"));
        assert_ne!(fingerprint128(b"hello"), fingerprint128(b"hello world"));
        // 长度进键：同前缀不同长度不同指纹。
        assert_ne!(fingerprint128(&[0u8; 7]), fingerprint128(&[0u8; 8]));
    }

    #[test]
    fn key_shape_and_determinism() {
        let fp = fingerprint128(b"hello");
        let key = identity_key(1234, &fp);
        assert!(key.starts_with("v1:00000000000004d2:"));
        assert_eq!(key.len(), "v1:".len() + 16 + 1 + 32);
        assert_eq!(key, identity_key(1234, &fp), "同输入同键");
    }

    #[test]
    fn sample_small_and_large_files() {
        let dir = std::env::temp_dir().join("aura-m3-identity-test");
        std::fs::create_dir_all(&dir).unwrap();
        let small = dir.join("small.bin");
        std::fs::write(&small, [7u8; 100]).unwrap();
        let id_small = content_identity(&small, "x/0/0/0".into()).unwrap();

        let big = dir.join("big.bin");
        let mut big_data = vec![0u8; 3 << 20];
        big_data[0] = 1;
        big_data[(3 << 20) - 1] = 3;
        std::fs::write(&big, &big_data).unwrap();
        let id_big = content_identity(&big, "x/0/0/0".into()).unwrap();
        let id_big2 = content_identity(&big, "x/0/0/0".into()).unwrap();
        assert_eq!(id_big, id_big2, "确定性");
        assert_ne!(id_small.identity_key, id_big.identity_key);

        // 中部（>2MiB 窗口之外）变化不影响指纹。
        big_data[(1 << 20) + 7] = 9;
        std::fs::write(&big, &big_data).unwrap();
        let id_big3 = content_identity(&big, "x/0/0/0".into()).unwrap();
        assert_eq!(id_big.identity_key, id_big3.identity_key);

        // 头部变化影响指纹。
        big_data[5] = 9;
        std::fs::write(&big, &big_data).unwrap();
        let id_big4 = content_identity(&big, "x/0/0/0".into()).unwrap();
        assert_ne!(id_big.identity_key, id_big4.identity_key);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
