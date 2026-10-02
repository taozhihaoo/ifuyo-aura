//! ifuyo Aura 网络缓存：256 KiB 分块、原子改名、SQLite 元数据（WAL）、
//! Range、prefetch、LRU、pin、故障注入自愈。
//!
//! 只有 `Allowed` 来源允许持久缓存；`Forbidden` 来源必须从代码结构上
//! 保证所有执行路径零写盘。
//!
//! Phase 0：仅 crate 边界；M5 起填充实现。

/// 阶段标识；M5 实装后移除。
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
