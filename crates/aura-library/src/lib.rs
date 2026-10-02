//! ifuyo Aura 音乐库：目录扫描（初始/增量/watcher）、标签读取（lofty）、
//! SQLite（WAL + FTS5，中文 CJK 二元组）、内容哈希身份、smart search。
//!
//! Phase 0：仅 crate 边界；M3 起填充实现。

/// 阶段标识；M3 实装后移除。
#[must_use]
pub const fn phase() -> &'static str {
    "phase-0-skeleton"
}

#[cfg(test)]
mod tests {
    #[test]
    fn skeleton_compiles() {
        assert_eq!(super::phase(), "phase-0-skeleton");
    }
}
