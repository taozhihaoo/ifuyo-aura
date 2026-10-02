//! ifuyo Aura 音频内核：decode → resample → lock-free SPSC ring buffer → output。
//!
//! 边界约束（ARCHITECTURE.md）：
//! - 不得依赖 Tauri、UI、filesystem、network；
//! - 播放位置以 `consumed_frames / sample_rate` 为准，不用 wall clock；
//! - audio callback 内禁止 allocation / lock / logging / syscall / blocking / IO / panic；
//! - 必须可通过 `aura-testkit` 的 NullOutput 独立测试。
//!
//! Phase 0：仅 crate 边界与编译基线；M1 起填充实现。

/// 阶段标识；M1 实装后移除。
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
