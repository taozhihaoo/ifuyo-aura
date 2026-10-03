//! FTS5 索引文本预处理（§33/§34/§35）。
//!
//! 方案（D-042）：索引侧把每个字段切分为字母数字段，规范化为
//! 「小写英文词 + CJK 单字与相邻二元组」，空格连接；FTS5 表用内建
//! `unicode61` tokenizer 按空格切分。查询侧同样分段后逐词加引号+前缀
//! 星号，杜绝 FTS 语法注入（§65）。
//!
//! - 英文：`Pink Floyd` → `pink floyd`；查询 `pink` → `"pink"*` 匹配；
//! - 中文：`流光音乐` → `流 光 音 乐 流光 光音 音乐`；查询 `音乐` →
//!   `"音乐"*`，查询单字 `音` → `"音"*` 匹配以该字开头的 token（§34）；
//! - emoji/标点：丢弃（不可搜索，display 值完整保留）。

/// 索引侧分词。
#[must_use]
pub fn index_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    for seg in alnum_segments(raw) {
        let norm = crate::normalize::search_text(&seg);
        if norm.is_empty() {
            continue;
        }
        for run in script_runs(&norm) {
            if crate::normalize::is_cjk(run.chars().next().unwrap_or(' ')) {
                let chars: Vec<char> = run.chars().collect();
                for &c in &chars {
                    out.push(c);
                    out.push(' ');
                }
                for w in 0..chars.len() {
                    if w + 1 < chars.len() {
                        out.push(chars[w]);
                        out.push(chars[w + 1]);
                        out.push(' ');
                    }
                }
            } else {
                out.push_str(&run);
                out.push(' ');
            }
        }
    }
    out.trim().to_string()
}

/// 查询侧：把用户输入转成安全的 FTS5 匹配表达式（全部 AND，前缀匹配）。
/// 每个 token 加引号 → FTS 语法字符（引号/冒号/括号/连字符/星号）全部失效化；
/// 无有效 token 时返回 `None`（调用方退化为无结果，而非全表匹配）。
#[must_use]
pub fn build_match_query(input: &str) -> Option<String> {
    let mut terms: Vec<String> = Vec::new();
    for seg in alnum_segments(input) {
        let norm = crate::normalize::search_text(&seg);
        if norm.is_empty() {
            continue;
        }
        for run in script_runs(&norm) {
            if crate::normalize::is_cjk(run.chars().next().unwrap_or(' ')) {
                let chars: Vec<char> = run.chars().collect();
                if chars.len() == 1 {
                    // 单字前缀（§34）：匹配以该字起始的索引 token。
                    terms.push(format!("\u{22}{}\u{22}*", chars[0]));
                } else {
                    for w in 0..chars.len() {
                        if w + 1 < chars.len() {
                            let bigram: String = chars[w..w + 2].iter().collect();
                            terms.push(format!("\u{22}{bigram}\u{22}*"));
                        }
                    }
                }
            } else {
                terms.push(format!("\u{22}{run}\u{22}*"));
            }
        }
    }
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

/// 规范化串再按文字系统切成 CJK / 非 CJK 连续段。
fn script_runs(norm: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut current = String::new();
    let mut current_cjk: Option<bool> = None;
    for c in norm.chars() {
        let cjk = crate::normalize::is_cjk(c);
        match current_cjk {
            Some(prev) if prev == cjk => current.push(c),
            Some(_) => {
                runs.push(std::mem::take(&mut current));
                current.push(c);
                current_cjk = Some(cjk);
            }
            None => {
                current.push(c);
                current_cjk = Some(cjk);
            }
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    runs
}

/// 原始串按非字母数字切成段（保序；CJK 是字母数字，连续 CJK 成一段）。
fn alnum_segments(raw: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    for c in raw.chars() {
        if c.is_alphanumeric() {
            current.push(c);
        } else if !current.is_empty() {
            segments.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_word_indexing() {
        assert_eq!(index_text("Pink Floyd"), "pink floyd");
    }

    /// §33：CJK 二元组。
    #[test]
    fn cjk_bigram_indexing() {
        assert_eq!(index_text("流光音乐"), "流 光 音 乐 流光 光音 音乐");
    }

    #[test]
    fn mixed_script_indexing() {
        assert_eq!(index_text("YOASOBI\u{30A2}\u{30A4}\u{30C9}\u{30EB}"), "yoasobi \u{30A2} \u{30A4} \u{30C9} \u{30EB} \u{30A2}\u{30A4} \u{30A4}\u{30C9} \u{30C9}\u{30EB}");
    }

    #[test]
    fn single_cjk_char() {
        // 单字：只有单字 token（无二元组）。
        assert_eq!(index_text("夜"), "夜");
    }

    /// §34：单字前缀查询。
    #[test]
    fn single_char_prefix_query() {
        assert_eq!(build_match_query("音"), Some("\u{22}音\u{22}*".into()));
    }

    /// §35：英文前缀 + 多词 AND。
    #[test]
    fn english_prefix_query() {
        assert_eq!(build_match_query("Pink"), Some("\u{22}pink\u{22}*".into()));
        assert_eq!(
            build_match_query("Pink Floyd"),
            Some("\u{22}pink\u{22}* \u{22}floyd\u{22}*".into())
        );
    }

    /// 查询二元组与索引二元组对齐：多字 CJK 查询 → 逐二元组 AND。
    #[test]
    fn cjk_query_bigrams() {
        assert_eq!(build_match_query("音乐"), Some("\u{22}音乐\u{22}*".into()));
        assert_eq!(
            build_match_query("流光音"),
            Some("\u{22}流光\u{22}* \u{22}光音\u{22}*".into())
        );
    }

    /// §65：FTS 语法注入字符全部失效化，不 panic。
    #[test]
    fn fts_injection_neutralized() {
        for evil in [
            "\"; DROP TABLE tracks; --",
            "a:b (c) -d *e",
            "NEAR(a b)",
            "colon:quote\"star*",
            "🎵🎶",
            "''",
            "-",
            "  ",
        ] {
            let q = build_match_query(evil);
            if let Some(q) = q {
                // 输出只允许：引号包裹的 token、前缀星、空格、CJK/字母数字。
                for ch in q.chars() {
                    assert!(
                        ch == '\u{22}'
                            || ch == '*'
                            || ch == ' '
                            || ch.is_alphanumeric()
                            || crate::normalize::is_cjk(ch),
                        "非法字符 {ch:?} in query for {evil:?}: {q:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn emoji_only_input_has_no_terms() {
        assert_eq!(build_match_query("🎵🎶"), None);
        assert_eq!(build_match_query(""), None);
        assert_eq!(build_match_query("!!!"), None);
    }
}
