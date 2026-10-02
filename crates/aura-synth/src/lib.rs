//! ifuyo Aura Dreamscape：确定性程序化音乐引擎。
//!
//! 同一软件版本、同一平台，同 seed 必须逐位一致；seed 为 8 位 Base32 + mood。
//!
//! Phase 0：仅 crate 边界；M7 起填充实现。

/// 阶段标识；M7 实装后移除。
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
