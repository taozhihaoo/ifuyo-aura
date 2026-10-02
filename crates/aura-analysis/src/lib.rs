//! ifuyo Aura 音频分析：BPM、beat grid、key/Camelot、LUFS、true peak、
//! energy、brightness、MFCC、embedding。
//!
//! 后台低优先级、可取消、可恢复、可缓存、content-key 绑定、逐位确定性。
//!
//! Phase 0：仅 crate 边界；M8 起填充实现。

/// 阶段标识；M8 实装后移除。
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
