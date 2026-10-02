//! ifuyo Aura 来源系统：统一的 `search(query)` / `resolve(input)` Source trait
//! 与其实现（local、direct URL、radio/ICY、Radio Browser、后续合法开放来源）。
//!
//! 每个 Source 必须显式返回缓存策略：`Allowed | SessionOnly | Forbidden`；
//! 命中 `config/unsupported_platforms.json` 的输入必须直接拒绝且不发出请求。
//!
//! Phase 0：仅 crate 边界；M4 起填充实现。

/// 阶段标识；M4 实装后移除。
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
