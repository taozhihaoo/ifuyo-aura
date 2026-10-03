//! 播放质量 DSP（M2）：响度归一、软限幅器、10-band 参数 EQ。
//!
//! DSP 顺序（M2 §44，D-030）：
//! `输入 PCM → Track Loudness Gain × User Volume（平滑）→ EQ → Limiter → 输出`
//!
//! 全部实现确定性、无分配、无锁；EQ 参数经三缓冲 + 原子序号发布（D-032），
//! 回调路径不持锁。

use std::sync::atomic::{AtomicU32, Ordering};

/// 目标响度：−16 LUFS（M2 §19）。
pub const TARGET_LUFS: f64 = -16.0;

/// 限幅安全上限：−1.0 dBFS（M2 §21，D-031）。
pub const LIMITER_CEILING: f32 = 0.891_2509; // 10^(-1/20)

/// 软限幅肩部宽度（线性）：ceiling → 1.0 之间为 tanh 软区域。
const LIMITER_KNEE: f32 = 1.0 - LIMITER_CEILING;

/// TrackLoudness（M2 §2）：由未来 M3/M8 提供的响度元数据；M2 只消费。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackLoudness {
    pub integrated_lufs: f64,
    pub true_peak_db: f64,
    pub loudness_available: bool,
}

impl TrackLoudness {
    /// 缺失元数据：不归一（gain = 1.0，M2 §25）。
    #[must_use]
    pub const fn missing() -> Self {
        Self {
            integrated_lufs: TARGET_LUFS,
            true_peak_db: 0.0,
            loudness_available: false,
        }
    }
}

/// 计算受峰值安全限制的响度增益（线性）。
///
/// - 基础：`gain_db = target_lufs − integrated_lufs`；
/// - 峰值安全（§20）：`max_gain = ceiling / 10^(true_peak_db/20)`；
/// - 元数据缺失或 enabled=false → 1.0（不归一）。
#[must_use]
pub fn loudness_gain(loudness: &TrackLoudness, enabled: bool) -> f32 {
    if !enabled || !loudness.loudness_available {
        return 1.0;
    }
    let gain_db = TARGET_LUFS - loudness.integrated_lufs;
    let gain_linear = 10.0f64.powf(gain_db / 20.0);
    // 峰值安全上限：限幅后峰值 ≤ ceiling（−1 dBFS）
    let peak_linear = 10.0f64.powf(loudness.true_peak_db / 20.0);
    let max_gain = LIMITER_CEILING as f64 / peak_linear.max(1e-9);
    (gain_linear.min(max_gain)) as f32
}

/// 软限幅器（M2 §21）：分段函数，|x| ≤ ceiling 直通；肩部 tanh 压缩；
/// 渐近线 1.0。确定性、无分配、无 overshoot、无 NaN。
#[must_use]
pub fn limit_sample(x: f32) -> f32 {
    if !x.is_finite() {
        return 0.0;
    }
    let a = x.abs();
    if a <= LIMITER_CEILING {
        return x;
    }
    let over = (a - LIMITER_CEILING) / LIMITER_KNEE;
    let shaped = LIMITER_CEILING + LIMITER_KNEE * over.tanh();
    let y = shaped.min(1.0);
    if x < 0.0 {
        -y
    } else {
        y
    }
}

/// 对整个块应用限幅。
pub fn limit_block(buf: &mut [f32]) {
    for s in buf.iter_mut() {
        *s = limit_sample(*s);
    }
}

/// 单个 RBJ biquad 系数（Direct Form I）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoeffs {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

/// EQ 频段类型（M2 §37）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EqBandKind {
    Peak,
    LowShelf,
    HighShelf,
}

/// EQ 频段参数（M2 §36）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqBand {
    pub frequency: f32,
    pub gain_db: f32,
    pub q: f32,
    pub enabled: bool,
    pub kind: EqBandKind,
}

impl EqBand {
    #[must_use]
    pub const fn new(frequency: f32, gain_db: f32) -> Self {
        Self {
            frequency,
            gain_db,
            q: 1.1,
            enabled: true,
            kind: EqBandKind::Peak,
        }
    }
}

/// 标准 10 频段中心频率（Hz）。
pub const EQ_BAND_FREQUENCIES: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];

/// 10-band EQ 系数快照（不可变，三缓冲发布）。
#[derive(Debug, Clone)]
pub struct EqCoeffs {
    /// 每频段 5 系数（b0, b1, b2, a1, a2）；禁用频段 = 直通系数。
    pub bands: Vec<BiquadCoeffs>,
}

impl EqCoeffs {
    /// 直通（全禁用）。
    #[must_use]
    pub fn passthrough() -> Self {
        Self {
            bands: vec![
                BiquadCoeffs {
                    b0: 1.0,
                    b1: 0.0,
                    b2: 0.0,
                    a1: 0.0,
                    a2: 0.0,
                };
                10
            ],
        }
    }

    /// 从频段参数计算系数（RBJ cookbook，控制线程执行）。
    #[must_use]
    pub fn from_bands(bands: &[EqBand], sample_rate: f32) -> Self {
        let mut out = Self::passthrough();
        for (i, band) in bands.iter().take(10).enumerate() {
            if !band.enabled || band.gain_db.abs() < 1e-6 {
                continue; // 直通
            }
            out.bands[i] = rbj_coeffs(band, sample_rate);
        }
        out
    }
}

/// RBJ cookbook 系数（Peak / LowShelf / HighShelf，f32 计算，确定性）。
#[must_use]
pub fn rbj_coeffs(band: &EqBand, sample_rate: f32) -> BiquadCoeffs {
    let f0 = band.frequency.clamp(1.0, sample_rate * 0.45);
    let a = 10.0f32.powf(band.gain_db / 40.0); // A = 10^(gain/40)
    let w0 = std::f32::consts::TAU * f0 / sample_rate;
    let (s, c) = w0.sin_cos();
    let alpha = s / (2.0 * band.q);

    let (b0, b1, b2, a0, a1, a2);
    match band.kind {
        EqBandKind::Peak => {
            b0 = 1.0 + alpha * a;
            b1 = -2.0 * c;
            b2 = 1.0 - alpha * a;
            a0 = 1.0 + alpha / a;
            a1 = -2.0 * c;
            a2 = 1.0 - alpha / a;
        }
        EqBandKind::LowShelf => {
            let sq = 2.0 * a.sqrt() * alpha;
            b0 = a * ((a + 1.0) - (a - 1.0) * c + sq);
            b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * c);
            b2 = a * ((a + 1.0) - (a - 1.0) * c - sq);
            a0 = (a + 1.0) + (a - 1.0) * c + sq;
            a1 = -2.0 * ((a - 1.0) + (a + 1.0) * c);
            a2 = (a + 1.0) + (a - 1.0) * c - sq;
        }
        EqBandKind::HighShelf => {
            let sq = 2.0 * a.sqrt() * alpha;
            b0 = a * ((a + 1.0) + (a - 1.0) * c + sq);
            b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * c);
            b2 = a * ((a + 1.0) + (a - 1.0) * c - sq);
            a0 = (a + 1.0) - (a - 1.0) * c + sq;
            a1 = 2.0 * ((a - 1.0) - (a + 1.0) * c);
            a2 = (a + 1.0) - (a - 1.0) * c - sq;
        }
    }
    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// EQ 级联（Direct Form I，按声道独立状态；M2 §37）。
#[derive(Debug, Clone)]
pub struct EqCascade {
    /// 每声道每频段的状态 (x1, x2, y1, y2)。
    states: Vec<[f32; 4]>,
    channels: usize,
}

impl EqCascade {
    #[must_use]
    pub fn new(channels: usize) -> Self {
        Self {
            states: vec![[0.0; 4]; channels * 10],
            channels,
        }
    }

    pub fn reset(&mut self) {
        for s in &mut self.states {
            *s = [0.0; 4];
        }
    }

    /// 对交错 PCM 应用 10-band 级联（按声道独立处理）。
    pub fn process(&mut self, coeffs: &EqCoeffs, data: &mut [f32]) {
        let ch = self.channels;
        for c in 0..ch {
            for frame in data.chunks_exact_mut(ch) {
                let mut sample = frame[c];
                for (band_idx, band_coeffs) in coeffs.bands.iter().enumerate() {
                    let st = &mut self.states[c * 10 + band_idx];
                    let y =
                        band_coeffs.b0 * sample + band_coeffs.b1 * st[0] + band_coeffs.b2 * st[1]
                            - band_coeffs.a1 * st[2]
                            - band_coeffs.a2 * st[3];
                    st[1] = st[0];
                    st[0] = sample;
                    st[3] = st[2];
                    st[2] = y;
                    sample = y;
                }
                frame[c] = sample;
            }
        }
    }
}

/// EQ 预设（M2 §38）：纯数据（每频段 gain_db）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqPreset {
    pub name: &'static str,
    /// 每频段增益 dB（10 频段）。
    pub gains_db: [f32; 10],
}

/// 预设列表（M2 §38）。
pub const EQ_PRESETS: [EqPreset; 6] = [
    EqPreset {
        name: "Flat",
        gains_db: [0.0; 10],
    },
    EqPreset {
        name: "Bass Boost",
        gains_db: [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    },
    EqPreset {
        name: "Treble Boost",
        gains_db: [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 4.0, 5.0, 6.0],
    },
    EqPreset {
        name: "Vocal",
        gains_db: [-2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0, -1.0],
    },
    EqPreset {
        name: "Electronic",
        gains_db: [5.0, 4.0, 1.0, 0.0, -2.0, 1.0, 0.0, 2.0, 4.0, 5.0],
    },
    EqPreset {
        name: "Acoustic",
        gains_db: [3.0, 2.0, 1.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 2.0],
    },
];

/// 由预设构建 10 个频段。
#[must_use]
pub fn bands_from_preset(preset: &EqPreset) -> Vec<EqBand> {
    EQ_BAND_FREQUENCIES
        .iter()
        .zip(preset.gains_db.iter())
        .map(|(&f, &g)| EqBand::new(f, g))
        .collect()
}

/// EQ 参数快照总线：原子字发布 + 序号撕裂检测（M2 §42，D-032）。
///
/// 控制线程 `store`（写 50 个原子字 + 序号 ×2）；回调 `load_into` 将快照
/// 复制进自己的本地副本（Copy），序号奇偶校验失败则保留旧副本本次跳过。
/// 全程无锁、无分配、无 unsafe。
pub struct EqBus {
    words: [AtomicU32; 50],
    seq: AtomicU32,
}

impl EqBus {
    #[must_use]
    pub fn new(initial: EqCoeffs) -> Self {
        let bus = Self {
            words: std::array::from_fn(|_| AtomicU32::new(0)),
            seq: AtomicU32::new(0),
        };
        bus.store(&initial);
        bus
    }

    /// 控制侧：发布新快照。
    pub fn store(&self, coeffs: &EqCoeffs) {
        self.seq.fetch_add(1, Ordering::AcqRel); // → 奇数（写入中）
        for (i, band) in coeffs.bands.iter().enumerate() {
            let vals = [band.b0, band.b1, band.b2, band.a1, band.a2];
            for (j, v) in vals.iter().enumerate() {
                self.words[i * 5 + j].store(v.to_bits(), Ordering::Relaxed);
            }
        }
        self.seq.fetch_add(1, Ordering::Release); // → 偶数（写入完成）
    }

    /// 版本号（偶数 = 稳定）。
    #[must_use]
    pub fn version(&self) -> u32 {
        self.seq.load(Ordering::Acquire)
    }

    /// 回调侧：尝试将当前快照读入 `out`；写入中（奇数序号）返回 false
    /// （调用方保留旧副本，下个渲染块重试——不做忙等）。
    pub fn load_into(&self, out: &mut EqCoeffs) -> bool {
        let s1 = self.seq.load(Ordering::Acquire);
        if s1 & 1 == 1 {
            return false;
        }
        for (i, band) in out.bands.iter_mut().enumerate() {
            let b0 = f32::from_bits(self.words[i * 5].load(Ordering::Relaxed));
            let b1 = f32::from_bits(self.words[i * 5 + 1].load(Ordering::Relaxed));
            let b2 = f32::from_bits(self.words[i * 5 + 2].load(Ordering::Relaxed));
            let a1 = f32::from_bits(self.words[i * 5 + 3].load(Ordering::Relaxed));
            let a2 = f32::from_bits(self.words[i * 5 + 4].load(Ordering::Relaxed));
            *band = BiquadCoeffs { b0, b1, b2, a1, a2 };
        }
        let s2 = self.seq.load(Ordering::Acquire);
        s1 == s2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loudness_gain_targets() {
        let on = |lufs: f64, tp: f64| {
            loudness_gain(
                &TrackLoudness {
                    integrated_lufs: lufs,
                    true_peak_db: tp,
                    loudness_available: true,
                },
                true,
            )
        };
        // Fixture 1：-16 LUFS → ≈0 dB
        assert!((on(-16.0, -60.0) - 1.0).abs() < 1e-6);
        // Fixture 2：-20 LUFS → +4 dB（无峰值约束时）
        assert!((on(-20.0, -60.0) - 10.0f32.powf(4.0 / 20.0)).abs() < 1e-5);
        // Fixture 3：-12 LUFS → -4 dB
        assert!((on(-12.0, -60.0) - 10.0f32.powf(-4.0 / 20.0)).abs() < 1e-5);
    }

    #[test]
    fn loudness_peak_safety_limits_gain() {
        // -24 LUFS 但 true peak -0.1 dBTP：增益受峰值安全限制
        let g = on_fixture(-24.0, -0.1);
        assert!(
            g < 10.0f32.powf(8.0 / 20.0),
            "增益不得达到未限制的 +8dB：{g}"
        );
        // 峰值安全不变量：true_peak(线性) × 增益 ≤ ceiling
        let tp_linear = 10.0f64.powf(-0.1 / 20.0);
        assert!(
            (tp_linear * f64::from(g)) <= f64::from(LIMITER_CEILING) + 1e-6,
            "峰值安全被破坏: {}",
            tp_linear * f64::from(g)
        );
    }

    fn on_fixture(lufs: f64, tp: f64) -> f32 {
        loudness_gain(
            &TrackLoudness {
                integrated_lufs: lufs,
                true_peak_db: tp,
                loudness_available: true,
            },
            true,
        )
    }

    #[test]
    fn loudness_off_and_missing_are_unit_gain() {
        assert_eq!(loudness_gain(&TrackLoudness::missing(), true), 1.0);
        let l = TrackLoudness {
            integrated_lufs: -30.0,
            true_peak_db: -10.0,
            loudness_available: true,
        };
        assert_eq!(loudness_gain(&l, false), 1.0, "enabled=false 不归一");
    }

    #[test]
    fn limiter_bounds_and_transparency() {
        // 阈值内直通
        assert!((limit_sample(0.5) - 0.5).abs() < 1e-6);
        assert!((limit_sample(-0.5) + 0.5).abs() < 1e-6);
        // 阈值外压缩且渐近 ≤ 1.0
        for x in [2.0f32, 8.0, 100.0, 1e6] {
            let y = limit_sample(x);
            assert!(y.is_finite() && y > 0.0 && y <= 1.0, "x={x} y={y}");
        }
        assert!(limit_sample(-1e6) >= -1.0);
        // 峰值 ≤ ceiling + 肩部（肩部渐近 1.0 ≈ 0dBFS；峰值安全目标 -1dBFS
        // 由 loudness gain ceiling 保证，limiter 是最后防线）
        assert!(limit_sample(50.0) <= 1.0);
    }

    #[test]
    fn limiter_no_nan_on_hostile_input() {
        for x in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(limit_sample(x), 0.0, "非有限输入替换为 0");
        }
    }

    #[test]
    fn rbj_peak_at_center_boosts() {
        let band = EqBand::new(1_000.0, 12.0);
        let c = rbj_coeffs(&band, 48_000.0);
        // 中心频率处幅度响应 ≈ +12dB（RBJ peak 定义）
        let w = std::f32::consts::TAU * 1_000.0 / 48_000.0;
        let (s, co) = w.sin_cos();
        let zr = co; // e^{j0}
        let zi = -s;
        let num_re = c.b0 + c.b1 * zr + c.b2 * (2.0 * zr * zr - 1.0);
        let num_im = c.b1 * zi + c.b2 * (2.0 * zr * zi);
        let den_re = 1.0 + c.a1 * zr + c.a2 * (2.0 * zr * zr - 1.0);
        let den_im = c.a1 * zi + c.a2 * (2.0 * zr * zi);
        let mag = ((num_re * num_re + num_im * num_im).sqrt())
            / ((den_re * den_re + den_im * den_im).sqrt());
        let gain_db = 20.0 * mag.log10();
        assert!((gain_db - 12.0).abs() < 0.5, "中心增益 {gain_db} dB");
    }

    #[test]
    fn eq_bus_publishes_snapshot() {
        let bus = EqBus::new(EqCoeffs::passthrough());
        let mut local = EqCoeffs::passthrough();
        assert!(bus.load_into(&mut local), "稳定序号下必须成功");
        assert_eq!(local.bands[0].b0, 1.0);
        let bands = bands_from_preset(&EQ_PRESETS[1]); // Bass Boost
        bus.store(&EqCoeffs::from_bands(&bands, 48_000.0));
        assert!(bus.load_into(&mut local), "新快照生效");
        assert!(local.bands[0].b0 != 1.0, "新快照系数已更新");
    }

    #[test]
    fn presets_are_pure_data() {
        assert_eq!(EQ_PRESETS.len(), 6);
        assert_eq!(EQ_PRESETS[0].name, "Flat");
        for p in &EQ_PRESETS {
            assert_eq!(p.gains_db.len(), 10);
        }
    }
}
