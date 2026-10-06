//! M12 §54：安全资源上限集中配置（SecurityLimits）。
//!
//! 单一权威来源：各子系统（artwork/library/sources/updater/map）已有
//! 的内建限制在此集中登记与交叉引用——不重复执行，只提供统一审计面。
//! 任何新增限制必须加入此模块并配测试（§66 不做 30 处散落拷贝）。

/// 单文件/流上限：本地解码文件大小（由文件系统决定，无硬上限；网络
/// 流有 buffer_capacity 限制见 aura-sources）。
pub struct SecurityLimits;

impl SecurityLimits {
    /// §46 网络：更新检查 HTTP 超时（ms）。
    pub const UPDATER_CHECK_TIMEOUT_MS: u64 = 10_000;
    /// §68：下载进度 UI 节流（ms）。
    pub const UPDATE_PROGRESS_THROTTLE_MS: u64 = 300;
    /// §8：更新自动检查最小间隔（ms）。
    pub const UPDATE_MIN_CHECK_INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;
    /// §8：启动延迟自动检查（ms）。
    pub const UPDATE_STARTUP_DELAY_MS: u64 = 15 * 1000;

    /// §134（M10）：SMTC 元数据字符串上限（chars）。
    pub const MAX_METADATA_CHARS: usize = 256;
    /// M13.3 歌词文件/标签字节上限（.lrc 或标签歌词超限视为无歌词）。
    pub const MAX_LYRICS_BYTES: usize = 512 * 1024;
    /// §134：电台/流标题上限（chars）。
    pub const MAX_STATION_CHARS: usize = 128;

    /// §36–§38（M9）：视口节点单次返回上限。
    pub const MAP_NODES_LIMIT_MAX: usize = 20_000;
    /// §36（M9）：邻居查询 k 上限。
    pub const MAP_NEIGHBORS_K_MAX: usize = 50;
    /// §36（M9）：邻居缓存条目上限。
    pub const MAP_NEIGHBORS_CACHE_MAX: usize = 64;

    /// §232（M8）：单任务批量分析上限。
    pub const ANALYSIS_BATCH_MAX: usize = 500;

    /// M11 §63：IPC 字符串参数保守上限（inputs；命令各自再校验语义）。
    pub const MAX_IPC_STRING_CHARS: usize = 4096;
    /// §59：播放输入字符串上限（URL/路径）。
    pub const MAX_PLAY_INPUT_CHARS: usize = 4096;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §54：上限互相自洽（const 块编译期断言；无 0、无反常识组合）。
    #[test]
    fn limits_are_sane() {
        const {
            assert!(SecurityLimits::UPDATER_CHECK_TIMEOUT_MS > 0);
            assert!(
                SecurityLimits::UPDATE_MIN_CHECK_INTERVAL_MS
                    > SecurityLimits::UPDATE_STARTUP_DELAY_MS
            );
            assert!(SecurityLimits::MAX_METADATA_CHARS >= SecurityLimits::MAX_STATION_CHARS);
            assert!(SecurityLimits::MAP_NODES_LIMIT_MAX <= 25_000); // M9 预算内
            assert!(SecurityLimits::MAX_IPC_STRING_CHARS >= SecurityLimits::MAX_METADATA_CHARS);
        }
        // 运行时抽样（保持测试有行为断言）。
        assert_eq!(SecurityLimits::MAX_METADATA_CHARS, 256);
    }
}
