//! Analysis key（§23）：hash(content_identity + analysis_version +
//! model versions) —— 确定性派生键，判定结果是否仍对应现状。

use siphasher::sip128::SipHasher13;

/// 固定域分离密钥（跨进程稳定；更换 = 新 analysis_version）。
const ANALYSIS_KEY_HASH_KEYS: (u64, u64) = (0x616e_616c_5f6b_6579, 0x0076_315f_7369_7068);

/// 稳定分析键（§23）。任意输入组合 → 确定性字符串键。
#[must_use]
pub fn analysis_key(
    content_identity: &str,
    analysis_version: u16,
    embedding_model_id: &str,
    embedding_model_version: u16,
) -> String {
    let mut buf = Vec::with_capacity(content_identity.len() + 24);
    buf.extend_from_slice(content_identity.as_bytes());
    buf.push(0xff);
    buf.extend_from_slice(&analysis_version.to_le_bytes());
    buf.extend_from_slice(embedding_model_id.as_bytes());
    buf.push(0xff);
    buf.extend_from_slice(&embedding_model_version.to_le_bytes());
    let out =
        SipHasher13::new_with_keys(ANALYSIS_KEY_HASH_KEYS.0, ANALYSIS_KEY_HASH_KEYS.1).hash(&buf);
    let v = out.h1 ^ out.h2;
    format!("{v:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_distinct() {
        let a = analysis_key("content-x", 1, "m", 1);
        let b = analysis_key("content-x", 1, "m", 1);
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert_ne!(a, analysis_key("content-y", 1, "m", 1));
        assert_ne!(a, analysis_key("content-x", 2, "m", 1));
        assert_ne!(a, analysis_key("content-x", 1, "n", 1));
        assert_ne!(a, analysis_key("content-x", 1, "m", 2));
    }

    #[test]
    fn domain_separated_from_components() {
        // 拼接歧义防御：("ab","c") 与 ("a","bc") 不同键。
        assert_ne!(
            analysis_key("ab-c", 1, "m", 1),
            analysis_key("a-bc", 1, "m", 1)
        );
    }
}
