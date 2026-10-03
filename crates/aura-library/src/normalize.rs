//! metadata 规范化（§23/§24/§25）：确定性、可测试、不破坏原始信息。
//!
//! - `display_text`：NFC + trim + 折叠内部空白 → 用户可见值；
//! - `search_text`：NFKD → 去变音符（combining marks 随字母分解后过滤）→
//!   小写。CJK 不受影响；emoji 等非字母数字从搜索文本剔除（仅影响可搜索性，
//!   display 值完整保留）。原始标签字符串不写库（§63 privacy）。

use unicode_normalization::UnicodeNormalization;

/// 展示文本：空/纯空白 → `None`。
#[must_use]
pub fn display_text(raw: &str) -> Option<String> {
    let nfc: String = raw.nfc().collect();
    let mut out = String::with_capacity(nfc.len());
    let mut pending_space = false;
    for ch in nfc.chars() {
        if ch.is_whitespace() {
            pending_space = !out.is_empty();
        } else {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(ch);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// 搜索/去重文本：确定性折叠。
///
/// CJK（表意/假名/谚文）原样保留 —— NFKD 会把浊音假名（ド→ト+浊点）
/// 拆解后丢失浊点，必须整体绕过；非 CJK 走 NFKD + 过滤 combining marks
/// 完成变音折叠，再小写。
#[must_use]
pub fn search_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if is_cjk(c) {
            out.push(c);
        } else {
            for d in std::iter::once(c).nfkd() {
                if d.is_alphanumeric() {
                    out.extend(d.to_lowercase());
                }
            }
        }
    }
    out
}

/// CJK 统一表意文字 + 假名 + 谚文（搜索域内的非空格文字）。
#[must_use]
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF   // 平假名/片假名
        | 0x31F0..=0x31FF // 片假名音标扩展
        | 0x3400..=0x4DBF // CJK 扩展 A
        | 0x4E00..=0x9FFF // CJK 基本区
        | 0xAC00..=0xD7AF // 谚文音节
        | 0xF900..=0xFAFF // CJK 兼容表意
        | 0x20000..=0x2A6DF // CJK 扩展 B
    )
}

/// 宽容年份解析：`1999` / `1999-03-01` / `1999/03/01` / `1999.x` 等，
/// 取首个 4 位数字段；`None` = 不可解析（不猜）。
#[must_use]
pub fn parse_year(raw: &str) -> Option<i32> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 4 {
        return None;
    }
    let year: i32 = digits[..4].parse().ok()?;
    (1000..=9999).contains(&year).then_some(year)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_trims_and_collapses() {
        assert_eq!(display_text("  Hello   World "), Some("Hello World".into()));
        assert_eq!(display_text(" \t\n "), None);
        assert_eq!(display_text(""), None);
        // NFC 合成：组合序列折叠为预组合字符。
        assert_eq!(
            display_text("e\u{0301}toile"),
            Some("\u{e9}toile".to_string())
        );
    }

    #[test]
    fn search_folds_accents_and_case_keeps_cjk() {
        assert_eq!(search_text("Pink Floyd"), "pinkfloyd");
        assert_eq!(search_text("Beyoncé"), "beyonce");
        assert_eq!(search_text("流光音乐"), "流光音乐");
        assert_eq!(search_text("ＡＢＣ"), "abc"); // 全角 NFKD
        assert_eq!(search_text("日本語テスト"), "日本語テスト");
        assert_eq!(search_text("café ☕"), "cafe"); // emoji 剔除
    }

    #[test]
    fn year_parsing_lenient_but_bounded() {
        assert_eq!(parse_year("1999"), Some(1999));
        assert_eq!(parse_year("1999-03-01"), Some(1999));
        assert_eq!(parse_year("© 2001 Sony"), Some(2001));
        assert_eq!(parse_year("99"), None);
        assert_eq!(parse_year("abc"), None);
    }
}
