//! M15a 编码修复：Latin-1 声明下实际为 GBK 的标签文本恢复。
//!
//! 背景：ID3v2.3 encoding=0（Latin-1）帧按字节透明解码，中文老歌的
//! GBK 原字节会以 U+0080..=U+00FF 区间字符呈现（乱码）。Latin-1 是
//! 字节透明映射——可无损还原原始字节后按 GBK 二次解码。
//!
//! 启发式（双轨防误判）：仅当 GBK 解码**无错误**且产出 ≥2 个 CJK 字符
//! 时才采用——欧洲语文的 é/ü 等高位字符不会命中（GBK 解码报错或无
//! CJK），保持原文。开关由应用设置（AppSettings.mojibake_fix，默认开）。

use encoding_rs::GBK;
use std::sync::atomic::{AtomicBool, Ordering};

static MOJIBAKE_FIX: AtomicBool = AtomicBool::new(true);

/// 应用设置切换（启动加载 + save_settings 时同步）。
pub fn set_mojibake_fix(on: bool) {
    MOJIBAKE_FIX.store(on, Ordering::Relaxed);
}

#[must_use]
pub fn mojibake_fix_enabled() -> bool {
    MOJIBAKE_FIX.load(Ordering::Relaxed)
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'   // CJK 统一表意文字
        | '\u{3000}'..='\u{303F}' // CJK 标点
        | '\u{FF00}'..='\u{FFEF}' // 全角形式
    )
}

/// 尝试修复 GBK 乱码；不命中启发式则原样返回。
#[must_use]
pub fn fix_mojibake(s: &str) -> String {
    if !mojibake_fix_enabled() || s.is_empty() {
        return s.to_string();
    }
    // 可无损还原字节的前提：非 ASCII 字符全部落在 U+0080..=U+00FF
    // （若混入 ≥U+0100 字符，说明已是真实 Unicode，不动）
    let has_high_latin = s.chars().any(|c| ('\u{80}'..='\u{FF}').contains(&c));
    let has_non_latin = s.chars().any(|c| c >= '\u{100}');
    if !has_high_latin || has_non_latin {
        return s.to_string();
    }
    let bytes: Vec<u8> = s.chars().map(|c| c as u32 as u8).collect();
    let (decoded, _, had_errors) = GBK.decode(&bytes);
    if had_errors {
        return s.to_string();
    }
    let cjk = decoded.chars().filter(|&c| is_cjk(c)).count();
    let orig_cjk = s.chars().filter(|&c| is_cjk(c)).count();
    if cjk >= 2 && cjk > orig_cjk {
        decoded.into_owned()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod mojibake_tests {
    use super::*;

    fn latin1_mojibake_of(text: &str) -> String {
        let (bytes, _, _) = GBK.encode(text);
        bytes.iter().map(|&b| b as char).collect()
    }

    #[test]
    fn recovers_gbk_title() {
        let moji = latin1_mojibake_of("周杰伦");
        assert!(moji.chars().any(|c| c >= '\u{80}'));
        assert_eq!(fix_mojibake(&moji), "周杰伦");
    }

    #[test]
    fn recovers_gbk_album_with_ascii_prefix() {
        let moji = latin1_mojibake_of("2006live十五周年");
        let fixed = fix_mojibake(&moji);
        assert!(fixed.contains("十五周年"), "got {fixed}");
    }

    #[test]
    fn leaves_european_text_alone() {
        // é 的 Latin-1 字节 E9 是 GBK 引导字节，但与后续 ASCII 组合解码
        // 报错/无 CJK → 启发式拒绝，保持原文
        assert_eq!(fix_mojibake("Café Melancholy"), "Café Melancholy");
    }

    #[test]
    fn leaves_ascii_and_pure_cjk() {
        assert_eq!(fix_mojibake("Plain Title"), "Plain Title");
        assert_eq!(fix_mojibake("周杰伦"), "周杰伦");
    }

    #[test]
    fn toggle_disables_fix() {
        let moji = latin1_mojibake_of("周杰伦");
        set_mojibake_fix(false);
        assert_eq!(fix_mojibake(&moji), moji);
        set_mojibake_fix(true);
        assert_eq!(fix_mojibake(&moji), "周杰伦");
    }
}
