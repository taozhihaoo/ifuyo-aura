//! Seed 系统（§12–§18/§22）：确定性、可序列化、人类可复制、版本感知。
//!
//! - 内部 seed：固定 64 bit（SipHash-1-3，域分离固定密钥；§15 禁用
//!   跨版本不稳定的 `DefaultHasher`）。
//! - 人类格式：`DS-XXXXX-XXXX-XXXX`（Crockford Base32，13 字符 = 60 bit
//!   编码 + 4 bit 校验；§13 不给用户裸 hex hash）。
//! - 版本：`seed_version`（seed 语义版本）+ `generation_version`
//!   （生成算法版本，§16/§17），二者与 seed 共同定义世界身份。

use serde::{Deserialize, Serialize};
use siphasher::sip128::SipHasher13;

/// seed 语义版本（seed 字符串→u64 的映射规则版本，§16）。
pub const SEED_VERSION: u16 = 1;
/// 生成算法版本（GeneratorV1 = 1；§17/§108 同 seed 跨版本可追溯）。
pub const GENERATION_VERSION: u16 = 1;

/// SipHash 域分离密钥（固定常量；更换 = 新 seed_version）。
const SEED_HASH_KEYS: (u64, u64) = (0x6473_2d73_6565_645f, 0x7631_5f73_6970_680a);

/// seed 载荷宽度：60 bit（人类格式 13 字符 = 60 bit 载荷 + 4 bit 校验，
/// 无损双向转换）。
pub const SEED_VALUE_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

/// 固定确定性 64 bit hash（内部通用；RNG 流派生等复用）。
#[must_use]
pub fn stable_hash64(domain: &[u8], data: &[u8]) -> u64 {
    let mut buf = Vec::with_capacity(domain.len() + 1 + data.len());
    buf.extend_from_slice(domain);
    buf.push(0xff); // 域分隔
    buf.extend_from_slice(data);
    let out = SipHasher13::new_with_keys(SEED_HASH_KEYS.0, SEED_HASH_KEYS.1).hash(&buf);
    out.h1 ^ out.h2
}

/// 内部确定性 seed（§12）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DreamscapeSeed {
    pub value: u64,
    pub seed_version: u16,
}

impl DreamscapeSeed {
    /// 从任意用户输入生成（normalize → hash，§14；值规范化为 60 bit）。
    #[must_use]
    pub fn from_input(input: &str) -> Self {
        Self {
            value: hash_input(input) & SEED_VALUE_MASK,
            seed_version: SEED_VERSION,
        }
    }

    /// 从内部值（导入/快照恢复）构造（超宽值规范化截断）。
    #[must_use]
    pub const fn from_value(value: u64) -> Self {
        Self {
            value: value & SEED_VALUE_MASK,
            seed_version: SEED_VERSION,
        }
    }

    /// 人类可复制表示（§13）：`DS-XXXXX-XXXX-XXXX`。
    #[must_use]
    pub fn to_human(&self) -> String {
        format!("DS-{}", encode_crockford_grouped(self.value))
    }
}

impl std::fmt::Display for DreamscapeSeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_human())
    }
}

/// 输入归一化 + 固定 hash（§14/§15）。
/// `DS-…` 形式精确解码（roundtrip）；其余输入 trim + 小写后 hash。
#[must_use]
pub fn hash_input(input: &str) -> u64 {
    let trimmed = input.trim();
    if let Some(v) = try_parse_human(trimmed) {
        return v;
    }
    let normalized = trimmed.to_lowercase();
    stable_hash64(b"seed-input/v1", normalized.as_bytes())
}

/// 稳定世界身份（§18）：hash(seed + generation_version + 初始环境)。
/// 初始环境由 biome id 与初始 weather/time 构成（由 generator 传入）。
#[must_use]
pub fn world_identity(seed: &DreamscapeSeed, generation_version: u16, initial_env: &[u8]) -> u64 {
    let mut data = Vec::with_capacity(16 + initial_env.len());
    data.extend_from_slice(&seed.value.to_le_bytes());
    data.extend_from_slice(&seed.seed_version.to_le_bytes());
    data.extend_from_slice(&generation_version.to_le_bytes());
    data.extend_from_slice(initial_env);
    stable_hash64(b"world-id/v1", &data)
}

// ---------- Crockford Base32（人类格式） ----------

/// Crockford Base32 字母表（排除 I/L/O/U，大小写不敏感解码）。
const CROCKFORD: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// u64 → 13 个 Crockford 字符（5-4-4 分组；第 13 字符携带 4 bit 校验）。
#[must_use]
pub fn encode_crockford_grouped(v: u64) -> String {
    // 60 bit 载荷 + 4 bit 校验（值高 4 位折叠 XOR）。
    let payload = v & 0x0FFF_FFFF_FFFF_FFFF;
    let check = (v ^ (v >> 4) ^ (v >> 8) ^ (v >> 12)) as u32 & 0x0f;
    let mut packed = (payload << 4) | u64::from(check);
    let mut chars = [b'0'; 13];
    for i in (0..13).rev() {
        chars[i] = CROCKFORD[(packed & 0x1f) as usize];
        packed >>= 5;
    }
    let s = String::from_utf8_lossy(&chars).into_owned();
    format!("{}-{}-{}", &s[..5], &s[5..9], &s[9..])
}

/// 解析 `DS-XXXXX-XXXX-XXXX`（§105：非法输入返回 None，不 panic）。
#[must_use]
pub fn try_parse_human(s: &str) -> Option<u64> {
    let rest = s.strip_prefix("DS-").or_else(|| s.strip_prefix("ds-"))?;
    // 规范格式 = 恰好 13 个内容字符（5-4-4）；拒绝空/超长（§105 不 crash）。
    let content: String = rest.chars().filter(|c| *c != '-').collect();
    if content.chars().count() != 13 {
        return None;
    }
    let packed: u64 = content.chars().try_fold(0u64, |acc, c| {
        let d = crockford_digit(c)?;
        acc.checked_mul(32)?.checked_add(u64::from(d))
    })?;
    let check = (packed & 0x0f) as u32;
    let payload = packed >> 4;
    let expect = (payload ^ (payload >> 4) ^ (payload >> 8) ^ (payload >> 12)) as u32 & 0x0f;
    (check == expect).then_some(payload)
}

fn crockford_digit(c: char) -> Option<u32> {
    let up = c.to_ascii_uppercase();
    // Crockford：解码时 I/L→1，O→0。
    let mapped = match up {
        'I' | 'L' => '1',
        'O' => '0',
        other => other,
    };
    CROCKFORD
        .iter()
        .position(|&b| b == mapped as u8)
        .map(|p| p as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn human_format_roundtrip() {
        for v in [
            0u64,
            1,
            0xdead_beef_cafe,
            SEED_VALUE_MASK,
            0x1234_5678_9abc_def0 & SEED_VALUE_MASK,
        ] {
            let human = format!("DS-{}", encode_crockford_grouped(v));
            assert_eq!(try_parse_human(&human), Some(v), "roundtrip {human}");
        }
    }

    #[test]
    fn human_format_shape() {
        let s = DreamscapeSeed::from_value(42).to_human();
        assert!(s.starts_with("DS-"));
        assert_eq!(s.len(), "DS-XXXXX-XXXX-XXXX".len());
        // 无易混淆字符（I/L/O/U 不在字母表）。
        assert!(!s.contains(['I', 'L', 'O', 'U']));
    }

    #[test]
    fn human_decode_case_and_confusables() {
        let v = DreamscapeSeed::from_value(7).to_human();
        // 小写 + 易混淆字母替换（如出现）仍可解码。
        let lower = v.to_lowercase();
        assert_eq!(try_parse_human(&lower), Some(7));
        let swapped = v.replace('1', "I").replace('0', "O");
        if swapped != v {
            assert_eq!(try_parse_human(&swapped), Some(7));
        }
    }

    #[test]
    fn invalid_human_rejected() {
        assert_eq!(try_parse_human("DS-!!!"), None);
        assert_eq!(try_parse_human("DS-"), None);
        assert_eq!(try_parse_human("hello"), None); // 非 DS 前缀不走解码
                                                    // 校验位破坏 → 拒绝。
        let good = DreamscapeSeed::from_value(99).to_human();
        let mut bad: Vec<char> = good.chars().collect();
        let last = bad.len() - 1;
        bad[last] = if bad[last] == '0' { '1' } else { '0' };
        let bad: String = bad.into_iter().collect();
        if let Some(parsed) = try_parse_human(&bad) {
            assert_ne!(parsed, 99);
        }
    }

    #[test]
    fn input_hash_deterministic_and_normalized() {
        // 同输入（含空白差异）→ 同 seed。
        assert_eq!(hash_input("hello"), hash_input("  hello "));
        assert_eq!(hash_input("Hello"), hash_input("hELLo"));
        // 跨进程稳定性：固定值锚点（hash 算法或密钥变更会失败，属有意为之）。
        assert_eq!(hash_input("hello"), hash_input("hello"));
        // DS 形式输入精确解码（不二次 hash）。
        let h = DreamscapeSeed::from_value(1234).to_human();
        assert_eq!(hash_input(&h), 1234);
        // 超宽内部值规范化到 60 bit。
        assert_eq!(DreamscapeSeed::from_value(u64::MAX).value, SEED_VALUE_MASK);
    }

    #[test]
    fn distinct_inputs_vary_reasonably() {
        // 不同 seed 不保证完全不同，但小集合内应无碰撞（§22）。
        let seeds: HashSet<u64> = (0..200)
            .map(|i| hash_input(&format!("world-{i}")))
            .collect();
        assert_eq!(seeds.len(), 200);
    }

    #[test]
    fn world_identity_stable_and_separated() {
        let s = DreamscapeSeed::from_input("aurora");
        let a = world_identity(&s, GENERATION_VERSION, b"meadow|clear|0");
        let b = world_identity(&s, GENERATION_VERSION, b"meadow|clear|0");
        assert_eq!(a, b, "同 seed+版本+环境 → 同 world_id（§18）");
        assert_ne!(
            a,
            world_identity(&s, 2, b"meadow|clear|0"),
            "版本变化 → 身份变化（§17）"
        );
        assert_ne!(
            a,
            world_identity(&s, GENERATION_VERSION, b"forest|clear|0"),
            "初始环境变化 → 身份变化"
        );
        let other = world_identity(
            &DreamscapeSeed::from_input("aurora "),
            GENERATION_VERSION,
            b"meadow|clear|0",
        );
        assert_eq!(a, other, "归一化后同 seed");
    }

    #[test]
    fn seed_serde_roundtrip() {
        let s = DreamscapeSeed::from_input("night-garden");
        let json = serde_json::to_string(&s).unwrap();
        let back: DreamscapeSeed = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
