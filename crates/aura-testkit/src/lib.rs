//! ifuyo Aura 测试基础设施（仅测试用途，不得进入生产依赖）：
//! - NullOutput：无设备音频输出；
//! - FaultFS：可注入 write/truncate/rename/index 故障的文件系统；
//! - 合成音频：click track、正弦、和弦夹具；
//! - 固定随机种子与确定性时钟。
//!
//! Phase 0：仅 crate 边界与契约占位；M1 起随 aura-core 需求填充。

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
