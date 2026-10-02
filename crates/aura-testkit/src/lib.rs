//! ifuyo Aura 测试基础设施（仅测试用途，不得进入生产依赖）。
//!
//! Charter §6 规定本 crate 提供：NullOutput、FaultFS、合成音频、固定随机种子、
//! 确定性时钟。M0 落地其中 M1 直接依赖的部分（确定性时钟、种子 RNG、合成音频、
//! NullOutput 雏形）；FaultFS 与更完整的合成音频随 M3/M5/M8 需求补充。
//!
//! 纪律：全部实现确定性（禁 wall-clock、禁系统随机源）、零第三方依赖。

pub mod audio;
pub mod clock;
pub mod rng;
