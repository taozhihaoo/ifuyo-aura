//! 输出抽象（M1 §24/§29–31）。
//!
//! - [`AudioOutput`] trait：最小输出设备接口；
//! - [`OutputStage`]：**回调路径的唯一逻辑**——generation-aware 读取、underrun
//!   补静音、10ms 增益平滑、原子计数。真实设备回调与 NullOutput 确定性渲染
//!   共用这一份代码。
//!
//! Callback 铁律（M1 §31）：`OutputStage::render` 路径零 allocation、零锁、
//! 零 logging、零 syscall、零 panic；由 `tests/zero_alloc.rs` 用计数分配器证明。

pub mod device;
pub mod device_state;
pub mod null;

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use crate::audio::buffer::ConsumerHandle;
use crate::audio::gain::GainSmoother;
use crate::types::{AudioFormat, Channels, Generation, PlaybackState, SampleRate};

/// 输出共享状态（callback 与控制面之间的全部通道：仅原子量，§39）。
#[derive(Clone)]
pub struct SharedOutputState {
    /// 权威播放位置（输出侧已消费帧数）。
    pub position: Arc<AtomicU64>,
    /// underrun 计数。
    pub underruns: Arc<AtomicU64>,
    /// 目标增益（f32 bits；由控制面写入）。
    pub target_gain_bits: Arc<AtomicU32>,
    /// 音轨静态增益（loudness × preamp；f32 bits）。
    pub track_gain_bits: Arc<AtomicU32>,
    /// 电平探针（驱动线程写入 → 可视化读取；f32 bits ×4，无锁）。
    pub level_bits: Arc<[std::sync::atomic::AtomicU32; 4]>,
    /// 播放状态（控制面发布；设备回调据此决定静音/推进位置——暂停时
    /// 位置必须停住 §183）。
    playback_state_bits: Arc<AtomicU32>,
    /// M12.5 传输淡变命令（driver→callback）。
    pub fade_cmd: Arc<AtomicU32>,
    /// M12.5 淡变已渲染帧数（callback→driver 进度）。
    pub fade_progress_frames: Arc<AtomicU64>,
    /// M12.5 淡变总帧数（driver 写入；0=无淡变）。
    pub fade_total_frames: Arc<AtomicU64>,
    /// M12.5 本轮淡变的起点增益（请求时定格；callback 每块读取，f32 bits）。
    fade_start_gain_bits: Arc<AtomicU32>,
    /// M12.5 淡变当前增益（callback 每块发布块末值；driver 发起新淡变时
    /// 由此连续出发，中途反转不跳变；f32 bits）。
    pub fade_gain_bits: Arc<AtomicU32>,
}

impl SharedOutputState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: Arc::new(AtomicU64::new(0)),
            underruns: Arc::new(AtomicU64::new(0)),
            target_gain_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            track_gain_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            level_bits: Arc::new([
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
                AtomicU32::new(0),
            ]),
            playback_state_bits: Arc::new(AtomicU32::new(state_bits(PlaybackState::Stopped))),
            fade_cmd: Arc::new(AtomicU32::new(FADE_CMD_NONE)),
            fade_progress_frames: Arc::new(AtomicU64::new(0)),
            fade_total_frames: Arc::new(AtomicU64::new(0)),
            fade_start_gain_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            fade_gain_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
        }
    }

    /// M12.5：请求传输淡变（OUT=淡出 / IN=淡入；total_frames=1s 帧）。
    ///
    /// 起始增益取回调最近发布的块末增益——中途反转（淡出中按播放等）
    /// 从当前响度连续出发，不产生增益跳变。
    pub fn request_fade(&self, cmd: u32, total_frames: u64) {
        let start = f32::from_bits(self.fade_gain_bits.load(Ordering::Acquire)).clamp(0.0, 1.0);
        self.request_fade_gain(cmd, total_frames, start);
    }

    /// M12.5：显式起始增益的淡变（IN 从静音恢复时用 0.0，不依赖发布值）。
    pub fn request_fade_gain(&self, cmd: u32, total_frames: u64, start_gain: f32) {
        self.fade_total_frames
            .store(total_frames, Ordering::Release);
        self.fade_progress_frames.store(0, Ordering::Release);
        self.fade_start_gain_bits
            .store(start_gain.clamp(0.0, 1.0).to_bits(), Ordering::Release);
        self.fade_cmd.store(cmd, Ordering::Release);
    }

    /// M12.5：取消进行中的淡变（增益立即回满）。
    pub fn cancel_fade(&self) {
        self.fade_cmd.store(FADE_CMD_CANCEL, Ordering::Release);
        self.fade_total_frames.store(0, Ordering::Release);
    }

    /// M12.5：淡变已渲染帧数（driver 轮询判定完成）。
    #[must_use]
    pub fn fade_progress(&self) -> u64 {
        self.fade_progress_frames.load(Ordering::Acquire)
    }

    /// M12.5：淡变总帧数。
    #[must_use]
    pub fn fade_total(&self) -> u64 {
        self.fade_total_frames.load(Ordering::Acquire)
    }

    /// M12.5：清除淡变命令槽（driver 在应用挂起命令后调用）。
    ///
    /// 保留 total 并把 progress 推满：相位钳位（OUT=静音/IN=全增益）在
    /// 挂起命令生效前后保持一致，直到下一条淡变命令复位。
    pub fn clear_fade(&self) {
        let total = self.fade_total_frames.load(Ordering::Acquire);
        self.fade_progress_frames.store(total, Ordering::Release);
        self.fade_cmd.store(FADE_CMD_NONE, Ordering::Release);
    }

    /// 控制面：发布当前播放状态（driver 线程低频写入）。
    pub fn set_playback_state(&self, state: PlaybackState) {
        self.playback_state_bits
            .store(state_bits(state), Ordering::Release);
    }

    /// 回调侧：读取最近发布的播放状态。
    #[must_use]
    pub fn playback_state(&self) -> PlaybackState {
        match self.playback_state_bits.load(Ordering::Acquire) {
            b if b == state_bits(PlaybackState::Playing) => PlaybackState::Playing,
            b if b == state_bits(PlaybackState::Paused) => PlaybackState::Paused,
            _ => PlaybackState::Stopped,
        }
    }

    /// 控制面：设置目标音量（值域已由 `Volume` 校验）。
    pub fn set_target_gain(&self, v: f32) {
        self.target_gain_bits.store(v.to_bits(), Ordering::Release);
    }

    fn target_gain(&self) -> f32 {
        f32::from_bits(self.target_gain_bits.load(Ordering::Acquire))
    }

    /// 控制面：设置音轨静态增益（loudness × preamp 线性积）。
    pub fn set_track_gain(&self, v: f32) {
        self.track_gain_bits.store(v.to_bits(), Ordering::Release);
    }

    /// 发布电平探针（驱动线程；非有限值一律按 0 处理，避免原子量被 NaN
    /// 永久污染——读取侧无法自愈）。
    pub fn set_level(&self, probe: crate::audio::level::LevelProbe) {
        let clean = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
        for (slot, v) in self
            .level_bits
            .iter()
            .zip([probe.rms, probe.low, probe.mid, probe.high])
        {
            slot.store(clean(v).to_bits(), Ordering::Release);
        }
    }

    /// 读取当前电平（可视化侧；无锁、无分配）。
    #[must_use]
    pub fn level(&self) -> crate::audio::level::LevelProbe {
        let load = |i: usize| f32::from_bits(self.level_bits[i].load(Ordering::Acquire));
        crate::audio::level::LevelProbe {
            rms: load(0),
            low: load(1),
            mid: load(2),
            high: load(3),
        }
    }

    fn track_gain(&self) -> f32 {
        f32::from_bits(self.track_gain_bits.load(Ordering::Acquire))
    }

    pub fn add_position(&self, frames: u64) {
        self.position.fetch_add(frames, Ordering::Relaxed);
    }

    /// 仅在确认 Playing 状态下由回调调用（暂停时位置必须停住 §183）。
    pub fn add_position_playing(&self, frames: u64) {
        self.position.fetch_add(frames, Ordering::Relaxed);
    }

    pub fn add_underrun(&self) {
        self.underruns.fetch_add(1, Ordering::Relaxed);
    }
}

// M12.5 传输淡变命令（driver→callback 经共享原子）。
pub const FADE_CMD_NONE: u32 = 0;
/// M12.5 淡变总帧数 @48k（1 秒；其它采样率按比例换算）。
pub const FADE_TOTAL_AT_48K: u64 = 48_000;
pub const FADE_CMD_OUT: u32 = 1;
pub const FADE_CMD_IN: u32 = 2;
pub const FADE_CMD_CANCEL: u32 = 3;

fn state_bits(state: PlaybackState) -> u32 {
    match state {
        PlaybackState::Stopped => 0,
        PlaybackState::Playing => 1,
        PlaybackState::Paused => 2,
    }
}

impl Default for SharedOutputState {
    fn default() -> Self {
        Self::new()
    }
}

/// 输出渲染阶段：回调路径唯一逻辑（见模块文档）。
pub struct OutputStage {
    smoother: GainSmoother,
    seen_generation: Generation,
    channels: Channels,
    eq_cascade: crate::dsp::EqCascade,
    local_eq: crate::dsp::EqCoeffs,
    last_eq_version: u32,
    /// limiter 开关（生产默认开；测试 provenance 可关，D-034）。
    limiter_enabled: bool,
    /// 电平探针（可视化面）。取样必须在**消费侧**：生产侧（解码 push 时）
    /// 在环形缓冲预满时根本没有数据可推——实测播放 1.5s 内 24 次读数 22 次
    /// 为 0，画面几乎不动。消费回调每块都真的在出声，读数因此持续新鲜。
    level: crate::audio::level::LevelTap,
    /// 上一块渲染状态（检测 Playing↔非 Playing 切换，做短淡变消爆音 §突刺）。
    last_state: PlaybackState,
    /// 淡变长度（帧，10ms @当前采样率；§突刺消爆音）。
    fade_frames: usize,
    /// M12.5 传输淡变相位（driver 经共享原子请求；callback 消费）。
    ///
    /// 淡变完成后相位**钳位不清除**：OUT 钳在增益 0（静音）、IN 钳在 1
    /// （无操作）——driver 应用挂起暂停前有 ≤1 个循环周期的间隙，钳位
    /// 保证该间隙不回跳全增益（爆音）；相位仅被下一条淡变命令复位。
    fade_phase: TransportFade,
}

/// M12.5 传输淡变相位（1s 淡变；driver 请求、callback 消费）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransportFade {
    None,
    Out,
    In,
}

impl OutputStage {
    #[must_use]
    pub fn new(sample_rate: SampleRate, channels: Channels) -> Self {
        Self {
            smoother: GainSmoother::new(sample_rate),
            seen_generation: Generation::FIRST,
            channels,
            eq_cascade: crate::dsp::EqCascade::new(usize::from(channels.get())),
            local_eq: crate::dsp::EqCoeffs::passthrough(),
            last_eq_version: 0,
            limiter_enabled: true,
            level: crate::audio::level::LevelTap::new(
                sample_rate.get(),
                usize::from(channels.get()),
            ),
            last_state: PlaybackState::Stopped,
            fade_frames: sample_rate.get() as usize / 100, // 10ms
            fade_phase: TransportFade::None,
        }
    }

    #[must_use]
    pub const fn channels(&self) -> Channels {
        self.channels
    }

    /// 重建 EQ 级联状态（输出声道数变化时，M2 §31）。
    pub fn reset_eq(&mut self, channels: usize) {
        self.eq_cascade = crate::dsp::EqCascade::new(channels);
        self.local_eq = crate::dsp::EqCoeffs::passthrough();
        self.last_eq_version = 0;
    }

    /// limiter 开关（仅测试 provenance 场景关闭；生产恒开，D-034）。
    pub fn set_limiter_enabled(&mut self, enabled: bool) {
        self.limiter_enabled = enabled;
    }

    /// 10ms 淡变帧数（§突刺消爆音；按采样率换算）。
    fn fade_frames(&self) -> usize {
        self.fade_frames
    }

    /// 渲染至多 `out.len()` 个采样：从 ring buffer 弹出 → underrun 补静音 →
    /// 平滑增益。返回实际渲染的帧数（含 underrun 静音帧）。
    ///
    /// 约束：`out` 由调用方预分配；本函数不分配、不加锁、不 panic。
    pub fn render(
        &mut self,
        out: &mut [f32],
        consumer: &mut ConsumerHandle,
        shared: &SharedOutputState,
        state: PlaybackState,
        eq_bus: Option<&crate::dsp::EqBus>,
    ) -> u64 {
        // M12.5：消费传输淡变命令。OUT 仅 Playing 态有意义（淡出期间引擎
        // 保持 Playing 供数据，fade 完成后 driver 才应用暂停/停止）；IN
        // 不设状态守卫——driver 在发布 Playing 之后才请求，但回调的 state
        // 快照可能滞后一拍，守卫会整体丢掉淡入命令。
        match shared.fade_cmd.swap(FADE_CMD_NONE, Ordering::AcqRel) {
            FADE_CMD_OUT if state == PlaybackState::Playing => {
                self.fade_phase = TransportFade::Out;
            }
            FADE_CMD_IN => self.fade_phase = TransportFade::In,
            FADE_CMD_CANCEL => self.fade_phase = TransportFade::None,
            _ => {}
        }

        // 非 Playing：切换块（last_state==Playing）消费缓冲残余并做 10ms
        // 淡出（消除波形硬切爆音"突刺"），位置按实际消费推进——此后纯静音。
        // §183：淡出后位置冻结。
        if state != PlaybackState::Playing {
            let ch = usize::from(self.channels.get());
            // 非播放态一律发布"静音电平"：暂停/停止时画面必须暗下来，
            // 不能冻结在最后一次读数（finish_block 顺带清掉累加器，下一段
            // 播放从干净状态起算）。
            let _ = self.level.finish_block();
            shared.set_level(crate::audio::level::LevelProbe::default());
            let was_playing = self.last_state == PlaybackState::Playing;
            self.last_state = state;
            if !was_playing {
                out.fill(0.0);
                return 0;
            }
            let got = consumer.pop_frames_until_generation(out, &mut self.seen_generation) as usize;
            let rendered = got * ch;
            out[rendered..].fill(0.0);
            // 10ms 线性淡出（起点=已发布的传输淡变增益）：1s 淡出完成后
            // 钳位增益为 0，残余缓冲必须静音出声——否则暂停落地的瞬间，
            // ring 里从未被衰减过的全增益残余以 1.0 起点出 10ms 声（用户
            // 报告的"渐静后突刺"；停止路径因 bump_generation 丢弃残余而无
            // 此问题）。无传输淡变的硬暂停（设备热拔等）发布值=1.0，行为
            // 与原 10ms 保护一致。
            let g0 = f32::from_bits(shared.fade_gain_bits.load(Ordering::Acquire)).clamp(0.0, 1.0);
            let fade = self.fade_frames().min(got.max(1));
            for i in 0..got {
                let g = if i < fade {
                    g0 * (1.0 - i as f32 / fade as f32)
                } else {
                    0.0
                };
                for c in 0..ch {
                    out[i * ch + c] *= g;
                }
            }
            shared.add_position(got as u64);
            return got as u64;
        }
        self.last_state = state;

        let frames = out.len() / usize::from(self.channels.get());
        // 恢复淡入（暂停/停止 → 播放的切换块）：消除恢复瞬间的爆音。
        let fade_in = self.last_state != PlaybackState::Playing;
        self.last_state = state;
        let got = consumer.pop_frames_until_generation(out, &mut self.seen_generation);
        if (got as usize) < frames {
            // underrun：尾部保持/置为静音，计数一次
            let silence_from = got as usize * usize::from(self.channels.get());
            out[silence_from..].fill(0.0);
            shared.add_underrun();
        }

        // PCM 安全（§37）：非有限值确定性替换为 0（D-025），随后平滑增益
        for s in out.iter_mut() {
            if !s.is_finite() {
                *s = 0.0;
            }
        }

        // 电平探针：取"任何增益之前"的信号——用户音量、响度归一、preamp、
        // EQ 都不参与。语义是零门控而非按音量缩放：静音（增益 0）时画面
        // 必须静止，但任何非零音量下亮度不随音量衰减，否则视觉替房间音量
        // 站台（实测 2% 音量时画面几乎全黑）。underrun 补的静音也在 out 里，
        // 读数如实反映"这一块实际出声的能量"。
        self.level.push(out);
        let target = shared.target_gain();
        let mut probe = self.level.finish_block();
        if target <= 0.0 {
            probe = crate::audio::level::LevelProbe::default();
        }
        shared.set_level(probe);

        self.smoother.set_target(target);
        self.smoother.process(out);
        // 音轨静态增益（loudness × preamp，§44 第一级）
        let track_gain = shared.track_gain();
        if track_gain != 1.0 {
            for s in out.iter_mut() {
                *s *= track_gain;
            }
        }
        // 恢复淡入（10ms 线性）：非 Playing → Playing 的切换块起始。
        if fade_in {
            let ch = usize::from(self.channels.get());
            let fade = self.fade_frames().min(frames);
            for f in 0..fade {
                let g = f as f32 / fade as f32;
                for c in 0..ch {
                    out[f * ch + c] *= g;
                }
            }
        }
        // EQ（§44 第三级；快照经 EqBus 原子发布，回调 load 无锁）
        if let Some(bus) = eq_bus {
            let v = bus.version();
            if v != self.last_eq_version && bus.load_into(&mut self.local_eq) {
                self.last_eq_version = v;
            }
            self.eq_cascade.process(&self.local_eq, out);
        }
        // Limiter（§44 末级，§21；D-034：provenance 测试可关）
        if self.limiter_enabled {
            crate::dsp::limit_block(out);
        }
        // M12.5 传输淡变（1s；起始增益取发布值保证中途反转连续；每块
        // 发布块末增益；完成后相位钳位——见 OutputStage::fade_phase 文档）。
        if self.fade_phase != TransportFade::None {
            let total = (shared.fade_total_frames.load(Ordering::Acquire)).max(1) as f32;
            let base = shared.fade_progress_frames.load(Ordering::Acquire) as f32;
            let g0 =
                f32::from_bits(shared.fade_start_gain_bits.load(Ordering::Acquire)).clamp(0.0, 1.0);
            let end_target = if self.fade_phase == TransportFade::Out {
                0.0f32
            } else {
                1.0f32
            };
            // total/base 以帧计，out 为交错采样——按声道数折算帧进度。
            let ch = f32::from(self.channels.get());
            let n_frames = out.len() as f32 / ch;
            for (i, s) in out.iter_mut().enumerate() {
                let t = ((base + i as f32 / ch) / total).clamp(0.0, 1.0);
                *s *= g0 + (end_target - g0) * t;
            }
            let done = (base + n_frames).min(total) as u64;
            shared.fade_progress_frames.store(done, Ordering::Release);
            let t_end = ((base + n_frames) / total).clamp(0.0, 1.0);
            shared.fade_gain_bits.store(
                (g0 + (end_target - g0) * t_end).to_bits(),
                Ordering::Release,
            );
        }
        // 位置按输出实际消费的帧数推进（underrun 静音帧也是被消费的时间；
        // 本分支仅在 Playing 进入——Paused/Stopped 已在函数头静音返回）。
        shared.add_position_playing(frames as u64);
        frames as u64
    }
}

/// 最小输出设备抽象（M1 §29）。
///
/// 真实 CPAL 适配器（output/device）与 NullOutput 都实现此 trait；
/// M1 的重点与全部测试基于 NullOutput。
pub trait AudioOutput: Send {
    fn format(&self) -> AudioFormat;

    /// 开始/恢复消费。
    fn start(&mut self) -> Result<(), crate::error::AudioError>;

    /// 暂停消费（设备保持开启）。
    fn pause(&mut self) -> Result<(), crate::error::AudioError>;

    /// 停止并关闭。
    fn stop(&mut self) -> Result<(), crate::error::AudioError>;

    /// 输出侧已消费帧数（权威位置的来源）。
    fn consumed_frames(&self) -> u64;

    /// underrun 计数。
    fn underruns(&self) -> u64;
}
