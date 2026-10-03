//! 音轨帧记账（M2 §8）。

/// 一条音轨的帧范围记账。
///
/// - `source_frames`：容器/编码声明的总帧数；
/// - `valid_frames`：实际解码得到的有效帧数（解码器已消费 gapless trim）；
/// - `rendered_frames`：输出侧实际渲染的帧数（含跨轨衔接）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackFrameRange {
    pub source_frames: u64,
    pub valid_frames: u64,
    pub rendered_frames: u64,
}

impl TrackFrameRange {
    #[must_use]
    pub const fn new(source_frames: u64) -> Self {
        Self {
            source_frames,
            valid_frames: 0,
            rendered_frames: 0,
        }
    }
}
