//! content key / chunk key / URL 规范化（§11–§14/§116）。
//!
//! content key = `"v1:" ++ hex(SipHash-2-4-128(normalized_url))`。
//! 输入**只有规范化 URL**（M4 复用同一 hash 原语，D-037 同源）：
//! - scheme/host 小写；默认端口剔除；fragment 剔除；
//! - path 与 query **原样保留**（§12：合法直链依赖 query 参数）。
//!
//! 安全（§13/§85/§116/§118）：
//! - key 是定长 hex，永远不直接成为文件名的用户可控部分；
//! - chunk 路径 = `chunks/<ck[0..2]>/<ck>/<index:08>.bin`（固定深度映射，
//!   无任何用户字符串拼接）；
//! - 指纹不含任何 secret 逆推可能；index 只存 `url_hash`（同指纹）+
//!   redacted URL（host/path，无 query）。
//!
//! chunk key = `(content_key, chunk_index)`（§14）。

use siphasher::sip128::SipHasher24;

use crate::error::{CacheError, Result};

/// 缓存键版本前缀；规范化或指纹定义变化时必须升级。
pub const KEY_VERSION: &str = "v1";

/// 固定密钥（产品命名派生；与 M4 内容身份的密钥**不同**，防跨域碰撞解释）。
const CACHE_KEY: (u64, u64) = (0x6361_6368_655f_6b31, 0x7265_6469_7265_6374);

/// URL 规范化（§12）：返回可用于指纹的规范形式。
/// scheme/host 小写、默认端口剔除、fragment 剔除；path/query 原样。
#[must_use]
pub fn normalize_url(raw: &str) -> Option<String> {
    let mut u = url::Url::parse(raw.trim()).ok()?;
    if !matches!(u.scheme(), "http" | "https") {
        return None;
    }
    // fragment 不参与内容身份。
    u.set_fragment(None);
    // 默认端口剔除。
    let default_port = match u.scheme() {
        "https" => Some(443),
        _ => Some(80),
    };
    if u.port() == default_port {
        let _ = u.set_port(None);
    }
    // scheme/host 小写（url crate 已对 scheme/host 做规范化，这里显式保证）。
    let host = u.host_str()?.to_ascii_lowercase();
    let rebuilt = match (u.port(), u.path()) {
        (Some(p), path) => format!("{}://{}:{p}{}", u.scheme(), host, path),
        (None, path) => format!("{}://{}{}", u.scheme(), host, path),
    };
    let query = u.query().map(|q| format!("?{q}")).unwrap_or_default();
    Some(format!("{rebuilt}{query}"))
}

/// content key（§11）：`"v1:<hex sip128>"`；确定性、稳定、抗碰撞。
///
/// # Errors
/// URL 无法解析/scheme 非法 → [`CacheError::InvalidKey`]。
pub fn content_key(raw_url: &str) -> Result<String> {
    let normalized = normalize_url(raw_url).ok_or_else(|| CacheError::InvalidKey {
        reason: "URL is not a valid http(s) URL".into(),
    })?;
    let fp = fingerprint_hex(normalized.as_bytes());
    Ok(fp)
}

fn fingerprint_hex(data: &[u8]) -> String {
    let out = SipHasher24::new_with_keys(CACHE_KEY.0, CACHE_KEY.1).hash(data);
    let mut hex = String::with_capacity(32);
    for b in out.h1.to_le_bytes().into_iter().chain(out.h2.to_le_bytes()) {
        hex.push(char::from_digit(u32::from(b >> 4), 16).unwrap_or('0'));
        hex.push(char::from_digit(u32::from(b & 0xf), 16).unwrap_or('0'));
    }
    hex
}

/// content key 合法性（§116：只能是我们生成的 32 位小写 hex）。
#[must_use]
pub fn is_valid_content_key(key: &str) -> bool {
    key.len() == 32
        && key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// chunk 相对路径（§14/§116）：固定深度、固定宽度、无用户字符串。
/// `chunks/<ck[0..2]>/<ck>/00000042.bin`
pub fn chunk_rel_path(content_key: &str, chunk_index: u64) -> Result<String> {
    if !is_valid_content_key(content_key) {
        return Err(CacheError::InvalidKey {
            reason: "content key must be 32 lowercase hex chars".into(),
        });
    }
    Ok(format!(
        "chunks/{}/{}/{:08}.bin",
        &content_key[..2],
        content_key,
        chunk_index
    ))
}

/// 条目元数据目录相对路径。
pub fn entry_rel_dir(content_key: &str) -> Result<String> {
    if !is_valid_content_key(content_key) {
        return Err(CacheError::InvalidKey {
            reason: "content key must be 32 lowercase hex chars".into(),
        });
    }
    Ok(format!("chunks/{}/", &content_key[..2]))
}

/// URL 脱敏展示（§12/§13/§86/§96）：scheme://host[:port]/path（无 query）。
#[must_use]
pub fn redact_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(u) => {
            let host = u.host_str().unwrap_or("");
            let port = u.port().map(|p| format!(":{p}")).unwrap_or_default();
            format!("{}://{}{}{}", u.scheme(), host, port, u.path())
        }
        Err(_) => "<unparsed url>".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_key_deterministic_and_stable() {
        let a = content_key("https://example.com/audio.mp3").unwrap();
        let b = content_key("https://example.com/audio.mp3").unwrap();
        assert_eq!(a, b);
        assert!(
            is_valid_content_key(&a),
            "content key = 32 hex（版本语义由 schema 记录，D-056）"
        );
        assert_eq!(a.len(), 32);
    }

    /// §12：规范化定义 —— host 大小写/默认端口/fragment 折叠；
    /// query 保留（大小写敏感、顺序敏感 —— 不随意删除）。
    #[test]
    fn normalization_rules() {
        let n1 = normalize_url("HTTPS://EXAMPLE.com:443/a.mp3?tok=1").unwrap();
        let n2 = normalize_url("https://example.com/a.mp3?tok=1").unwrap();
        assert_eq!(n1, n2, "scheme/host 小写 + 默认端口剔除");

        let n3 = normalize_url("https://example.com/a.mp3#frag").unwrap();
        assert_eq!(
            n3,
            normalize_url("https://example.com/a.mp3").unwrap(),
            "fragment 剔除"
        );

        assert_ne!(
            normalize_url("https://example.com/a.mp3?tok=1").as_deref(),
            normalize_url("https://example.com/a.mp3?tok=2").as_deref(),
            "不同 query = 不同内容（§12 不随意删 query）"
        );
        assert_ne!(
            normalize_url("http://example.com/a").as_deref(),
            normalize_url("https://example.com/a").as_deref(),
            "scheme 参与身份"
        );
        // 非默认端口保留。
        assert!(normalize_url("http://example.com:8080/a")
            .unwrap()
            .contains(":8080"));
    }

    #[test]
    fn invalid_inputs_rejected() {
        assert!(content_key("ftp://x/y").is_err());
        assert!(content_key("not a url").is_err());
        assert!(content_key("").is_err());
    }

    /// §116：chunk 路径固定形态，非法 key 拒绝。
    #[test]
    fn chunk_path_shape_and_validation() {
        let ck = content_key("https://example.com/a.mp3").unwrap();
        let p = chunk_rel_path(&ck, 42).unwrap();
        assert_eq!(p, format!("chunks/{}/{}/00000042.bin", &ck[..2], ck));
        assert!(chunk_rel_path("../../etc/passwd", 0).is_err());
        assert!(chunk_rel_path("ZONK", 0).is_err());
        assert!(chunk_rel_path(&ck, u64::MAX).is_ok());
    }

    /// §13：脱敏 —— query（可能含 token）不出现在 redacted URL。
    #[test]
    fn redaction_drops_query() {
        let r = redact_url("https://cdn.example.com/a.mp3?token=SECRET&x=1");
        assert_eq!(r, "https://cdn.example.com/a.mp3");
        assert!(!r.contains("SECRET"));
    }
}
