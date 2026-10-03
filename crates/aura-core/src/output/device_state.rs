//! 输出设备状态机（M2 §26–33）。
//!
//! Connected = 设备可用；Removed = 设备断开（安全暂停、可恢复）。
//! 检测本身由外部适配器负责（轮询/平台事件，M10 接线）——回调内禁止设备枚举。

/// 输出设备状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputDeviceState {
    Connected,
    Removed,
}
