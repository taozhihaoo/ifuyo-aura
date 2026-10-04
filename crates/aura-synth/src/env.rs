//! Environment 状态（§31–§42）：time of day / weather / wind / fog /
//! light / season + 受约束 palette。
//!
//! - 全部参数有 min/max/default 且确定性（§32）。
//! - 连续 `time_phase ∈ [0,1)`（§33），渲染器负责平滑插值；本层保证
//!   时间推进与天气过渡平滑（§34/§37）。
//! - 天气只有氛围档（§36 无灾害）；fog 上限约束（§39 不灰雾吞场景）。
//! - palette 由 seed+biome+environment 决定且受 sat/lum/对比约束
//!   （§41/§42 长时间观看不疲劳）。

use serde::{Deserialize, Serialize};

use crate::rng::{streams, Rng};
use crate::seed::DreamscapeSeed;

/// environment 语义版本（字段/范围变更 = 升版本）。
pub const ENV_VERSION: u16 = 1;

/// 1 逻辑日 = 24×60 tick（§30：1 tick = 1 逻辑分钟）。
pub const TICKS_PER_DAY: u64 = 24 * 60;
/// 一季 = 30 逻辑日；一年 4 季。
pub const TICKS_PER_SEASON: u64 = TICKS_PER_DAY * 30;

/// 连续时间相位（§33）。0 = 午夜，0.5 = 正午；渲染器插值。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimePhase(pub f32);

impl TimePhase {
    /// 由逻辑 tick 计算（午夜对齐 tick 0）。
    #[must_use]
    pub fn from_tick(tick: u64) -> Self {
        Self((tick % TICKS_PER_DAY) as f32 / TICKS_PER_DAY as f32)
    }

    /// 推进并回绕 [0,1)。
    pub fn advance(&mut self, delta_ticks: u64) {
        self.0 = (self.0 + delta_ticks as f32 / TICKS_PER_DAY as f32).fract();
    }

    /// 四段（§33 Dawn/Day/Dusk/Night）；边界连续，无跳变（§34）。
    #[must_use]
    pub fn time_of_day(self) -> TimeOfDay {
        let p = self.0;
        if (0.23..0.31).contains(&p) {
            TimeOfDay::Dawn
        } else if (0.31..0.69).contains(&p) {
            TimeOfDay::Day
        } else if (0.69..0.79).contains(&p) {
            TimeOfDay::Dusk
        } else {
            TimeOfDay::Night
        }
    }
}

/// 四段时间（§33）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeOfDay {
    Dawn,
    Day,
    Dusk,
    Night,
}

/// 季节（§31；低频背景属性）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

#[must_use]
pub fn season_at_tick(tick: u64) -> Season {
    match (tick / TICKS_PER_SEASON) % 4 {
        0 => Season::Spring,
        1 => Season::Summer,
        2 => Season::Autumn,
        _ => Season::Winter,
    }
}

/// 天气档（§35 五档；§36 全部为氛围，无灾害档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weather {
    Clear,
    Misty,
    Drizzle,
    Rain,
    Overcast,
}

impl Weather {
    /// 该天气的基础雾密度倾向（§39：fog ≤ FOG_MAX，永不吞场景）。
    #[must_use]
    pub const fn base_fog(self) -> f32 {
        match self {
            Self::Clear => 0.05,
            Self::Misty => 0.45,
            Self::Drizzle => 0.3,
            Self::Rain => 0.35,
            Self::Overcast => 0.25,
        }
    }

    /// 相邻可过渡档（§37：平滑过渡图；无 Clear↔Rain 直跳）。
    #[must_use]
    pub fn neighbors(self) -> &'static [Self] {
        match self {
            Self::Clear => &[Self::Misty, Self::Overcast],
            Self::Misty => &[Self::Clear, Self::Drizzle, Self::Overcast],
            Self::Drizzle => &[Self::Misty, Self::Rain],
            Self::Rain => &[Self::Drizzle, Self::Overcast],
            Self::Overcast => &[Self::Clear, Self::Misty, Self::Rain],
        }
    }
}

/// fog 硬上限（§39）。
pub const FOG_MAX: f32 = 0.7;

/// 环境状态（§78 持久化字段之一）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentState {
    pub env_version: u16,
    /// 绝对逻辑 tick（1 tick = 1 逻辑分钟，§30）；相位由此推导。
    pub tick: u64,
    /// 天气决策已检查到的日桶（240 tick/桶；确定性 O(1) 决策）。
    pub weather_day: u64,
    /// 当前天气。
    pub weather: Weather,
    /// 正在过渡到的目标档（`None` = 稳定态，§37）。
    pub weather_target: Option<Weather>,
    /// 过渡进度 [0,1)（§37 平滑 0%→100%）。
    pub weather_blend: f32,
    /// 湿度 [0,1]。
    pub humidity: f32,
    /// 温度 [-10,40] ℃。
    pub temperature: f32,
    /// 风 [0,1]（§38 影响 foliage/mist/particle，无物理模拟）。
    pub wind: f32,
    /// 雾 [0, FOG_MAX]（§39 depth/atmosphere）。
    pub fog: f32,
    /// 太阳光强度 [0,1]（§40 抽象光态）。
    pub sun_intensity: f32,
    /// 环境光 [0,1]。
    pub ambient_intensity: f32,
    /// 地平线辉光 [0,1]。
    pub horizon_glow: f32,
}

impl EnvironmentState {
    /// 初始环境（由 seed 的 weather 流决定初始档与相位偏移；§45 精神：
    /// 不能每次打开随机换——相位/天气由 seed 与逻辑时间共同决定）。
    #[must_use]
    pub fn initial(seed: &DreamscapeSeed, tick: u64) -> Self {
        let mut w = Rng::from_stream(seed, streams::WEATHER);
        // 初始档从 Clear 的相邻档温和起步（Misty/Overcast，§36 氛围档）。
        let weather = Weather::neighbors(Weather::Clear)[(w.next_u64() as usize) % 2];
        let mut st = Self {
            env_version: ENV_VERSION,
            tick,
            weather_day: 0,
            weather,
            weather_target: None,
            weather_blend: 0.0,
            humidity: 0.4 + w.next_f32_unit() * 0.2,
            temperature: 12.0 + w.next_f32_range(-4.0, 8.0),
            wind: 0.2 + w.next_f32_unit() * 0.3,
            fog: weather.base_fog(),
            sun_intensity: 0.0,
            ambient_intensity: 0.5,
            horizon_glow: 0.3,
        };
        st.update_light();
        st
    }

    /// 连续时间相位 [0,1)（§33 推导值）。
    #[must_use]
    pub fn time_phase(&self) -> TimePhase {
        TimePhase::from_tick(self.tick)
    }

    /// 由时间相位推导光态（§40；夜正午差异明显但无突变，§34）。
    fn update_light(&mut self) {
        // 太阳：正午峰值的余弦曲线（夜间 0）。
        let phase = self.time_phase().0;
        let daylight = (phase * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2)
            .sin()
            .max(0.0);
        let overcast = f32::from(matches!(self.weather, Weather::Overcast | Weather::Rain)) * 0.35;
        self.sun_intensity = (daylight * (1.0 - overcast)).clamp(0.0, 1.0);
        // 环境：夜间不再低于 0.25（§99：凌晨不刺眼也不漆黑）。
        self.ambient_intensity = (0.25 + 0.55 * daylight * (1.0 - overcast * 0.5)).clamp(0.0, 1.0);
        // 地平线辉光：黎明/黄昏峰（中心 0.26/0.74）。
        let p = phase;
        let glow = ((p - 0.26).abs() * 12.0)
            .min(1.0)
            .min((p - 0.74).abs() * 12.0)
            .min(1.0);
        self.horizon_glow = (1.0 - glow).clamp(0.0, 1.0) * 0.8;
    }

    /// 推进环境（§30 逻辑 tick；天气决策按 240-tick 日桶 + weather 流，
    /// 确定性且 O(1)，§37）。
    pub fn advance(&mut self, seed: &DreamscapeSeed, delta_ticks: u64) {
        self.tick = self.tick.saturating_add(delta_ticks);
        // 天气过渡推进（§37 平滑，全程 ≈ 240 tick）。
        if let Some(target) = self.weather_target {
            self.weather_blend = (self.weather_blend + delta_ticks as f32 / 240.0).min(1.0);
            if self.weather_blend >= 1.0 {
                self.weather = target;
                self.weather_target = None;
                self.weather_blend = 0.0;
            }
        } else {
            // 无目标时每个日桶最多决策一次（同桶同结果 → 可重放）。
            let day = self.tick / 240;
            if day > self.weather_day {
                self.weather_day = day;
                let mut w = Rng::from_stream_bucket(seed, streams::WEATHER, day);
                if w.chance(0.15) {
                    let options = self.weather.neighbors();
                    self.weather_target = Some(options[(w.next_u64() as usize) % options.len()]);
                    self.weather_blend = 0.0;
                }
            }
        }
        // 渲染插值用的展示档：过渡中按 blend 混合雾/湿/风（§37 平滑）。
        let base_fog = if let (Some(target), true) = (self.weather_target, self.weather_blend > 0.0)
        {
            self.weather.base_fog() * (1.0 - self.weather_blend)
                + target.base_fog() * self.weather_blend
        } else {
            self.weather.base_fog()
        };
        self.fog = base_fog.min(FOG_MAX);
        self.humidity = (self.humidity
            + match self.weather {
                Weather::Rain => 0.01,
                Weather::Drizzle => 0.005,
                Weather::Clear => -0.004,
                _ => 0.0,
            } * delta_ticks as f32)
            .clamp(0.0, 1.0);
        // 风的微波动也按日桶派生（同桶同波动相位，确定性）。
        let mut wb = Rng::from_stream_bucket(seed, streams::WEATHER, self.tick / 60);
        self.wind =
            (self.wind + (wb.next_f32_unit() - 0.5) * 0.02 * delta_ticks as f32).clamp(0.05, 0.8);
        self.temperature = (self.temperature + (0.5 - self.humidity) * 0.001 * delta_ticks as f32)
            .clamp(-10.0, 40.0);
        self.update_light();
    }

    /// 反序列化矫正（§32 边界）。
    pub fn sanitize(&mut self) {
        self.env_version = ENV_VERSION;
        self.weather_blend = self.weather_blend.clamp(0.0, 1.0);
        self.humidity = self.humidity.clamp(0.0, 1.0);
        self.temperature = self.temperature.clamp(-10.0, 40.0);
        self.wind = self.wind.clamp(0.0, 1.0);
        self.fog = self.fog.clamp(0.0, FOG_MAX);
        self.sun_intensity = self.sun_intensity.clamp(0.0, 1.0);
        self.ambient_intensity = self.ambient_intensity.clamp(0.0, 1.0);
        self.horizon_glow = self.horizon_glow.clamp(0.0, 1.0);
        if self.weather_target == Some(self.weather) {
            self.weather_target = None;
            self.weather_blend = 0.0;
        }
        self.update_light();
    }
}

/// 受约束 palette（§41/§42）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    /// 基调色相（度）。
    pub base_hue: f32,
    /// 点缀色相（度；accent 频率受限，§42）。
    pub accent_hue: f32,
    /// 饱和度（0.20–0.55，§42）。
    pub saturation: f32,
    /// 亮度（0.35–0.75，§42）。
    pub luminance: f32,
    /// 点缀频率 [0,0.2]（低频点缀，§42）。
    pub accent_frequency: f32,
}

pub const PALETTE_VERSION: u16 = 1;
pub const SAT_MIN: f32 = 0.20;
pub const SAT_MAX: f32 = 0.55;
pub const LUM_MIN: f32 = 0.35;
pub const LUM_MAX: f32 = 0.75;
pub const ACCENT_FREQ_MAX: f32 = 0.2;

impl Palette {
    /// seed + biome id + 环境决定（§41），全部约束在舒适区间（§42）。
    #[must_use]
    pub fn derive(seed: &DreamscapeSeed, biome_id: &str, env: &EnvironmentState) -> Self {
        let mut p = Rng::from_stream(seed, streams::PALETTE);
        let biome_anchor =
            crate::seed::stable_hash64(b"biome-palette/v1", biome_id.as_bytes()) % 360;
        // 基调 = biome 锚 + seed 漂移（±25°）+ 环境色温漂移（±12°）。
        let base = (biome_anchor as f32
            + (p.next_f32_unit() - 0.5) * 50.0
            + env_temperature_hue_shift(env))
        .rem_euclid(360.0);
        // 点缀 = 基调 ±90..150°（和谐间隔，避免随机丑陋组合 §41）。
        let accent = (base + 90.0 + p.next_f32_unit() * 60.0).rem_euclid(360.0);
        // 色温越暖 → 略高饱和、略高亮（有界联动，非随机）。
        let warmth = env_temperature_hue_shift(env).signum();
        Self {
            base_hue: base,
            accent_hue: accent,
            saturation: (0.32 + warmth * 0.05 + p.next_f32_unit() * 0.08).clamp(SAT_MIN, SAT_MAX),
            luminance: (0.52 + p.next_f32_range(-0.08, 0.08)).clamp(LUM_MIN, LUM_MAX),
            accent_frequency: (p.next_f32_unit() * ACCENT_FREQ_MAX * 0.8)
                .clamp(0.0, ACCENT_FREQ_MAX),
        }
    }

    /// 手改 JSON 后矫正（§42 约束不被破坏）。
    pub fn sanitize(&mut self) {
        self.base_hue = self.base_hue.rem_euclid(360.0);
        self.accent_hue = self.accent_hue.rem_euclid(360.0);
        self.saturation = self.saturation.clamp(SAT_MIN, SAT_MAX);
        self.luminance = self.luminance.clamp(LUM_MIN, LUM_MAX);
        self.accent_frequency = self.accent_frequency.clamp(0.0, ACCENT_FREQ_MAX);
    }
}

fn env_temperature_hue_shift(env: &EnvironmentState) -> f32 {
    // 冷 → 偏蓝（负漂移），暖 → 偏琥珀（正漂移）。
    ((env.temperature - 12.0) / 28.0).clamp(-1.0, 1.0) * 12.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_wraps_and_maps_tod() {
        let mut p = TimePhase::from_tick(0);
        assert_eq!(p.0, 0.0);
        p.advance(TICKS_PER_DAY + 720);
        assert!((p.0 - 0.5).abs() < 1e-6, "整日回绕 + 半日 = 正午");
        assert_eq!(
            TimePhase::from_tick(TICKS_PER_DAY / 2).time_of_day(),
            TimeOfDay::Day
        );
        assert_eq!(
            TimePhase::from_tick(TICKS_PER_DAY / 4).time_of_day(),
            TimeOfDay::Dawn
        );
        assert_eq!(
            TimePhase::from_tick(TICKS_PER_DAY * 3 / 4).time_of_day(),
            TimeOfDay::Dusk
        );
        assert_eq!(TimePhase::from_tick(0).time_of_day(), TimeOfDay::Night);
    }

    #[test]
    fn light_smooth_no_hard_night_cut() {
        // §34/§99：相位扫描中光态连续（相邻采样差有界），夜间环境光 ≥ 0.25。
        let seed = DreamscapeSeed::from_input("light");
        let mut env = EnvironmentState::initial(&seed, 0);
        let mut prev_sun = env.sun_intensity;
        let mut prev_amb = env.ambient_intensity;
        for _ in 0..TICKS_PER_DAY {
            env.advance(&seed, 1);
            assert!((env.sun_intensity - prev_sun).abs() < 0.05, "太阳无跳变");
            assert!(
                (env.ambient_intensity - prev_amb).abs() < 0.05,
                "环境光无跳变"
            );
            assert!(env.ambient_intensity >= 0.24);
            prev_sun = env.sun_intensity;
            prev_amb = env.ambient_intensity;
        }
        assert!(prev_sun < 0.05, "午夜太阳 ≈ 0");
    }

    #[test]
    fn weather_transitions_smooth_and_bounded_set() {
        // 只有五档（§35）；过渡经过 blend，档间只能沿 neighbors 图（§37）。
        let seed = DreamscapeSeed::from_input("weather");
        let mut env = EnvironmentState::initial(&seed, 0);
        for _ in 0..2_000_000u64 {
            env.advance(&seed, 15);
            assert!(matches!(
                env.weather,
                Weather::Clear
                    | Weather::Misty
                    | Weather::Drizzle
                    | Weather::Rain
                    | Weather::Overcast
            ));
            if let Some(t) = env.weather_target {
                assert!(env.weather.neighbors().contains(&t), "只能过渡到相邻档");
                assert!((0.0..=1.0).contains(&env.weather_blend));
            }
        }
    }

    #[test]
    fn fog_capped_and_derived_from_weather() {
        // §39：任何天气序列后 fog ≤ FOG_MAX。
        let seed = DreamscapeSeed::from_input("fog");
        let mut env = EnvironmentState::initial(&seed, 0);
        for _ in 0..500_000u64 {
            env.advance(&seed, 7);
            assert!(env.fog <= FOG_MAX + 1e-6);
            assert!(env.wind <= 0.8);
            assert!((-10.0..=40.0).contains(&env.temperature));
            assert!((0.0..=1.0).contains(&env.humidity));
        }
    }

    #[test]
    fn environment_deterministic_by_seed_and_tick() {
        let a_seed = DreamscapeSeed::from_input("envdet");
        let mut a = EnvironmentState::initial(&a_seed, 1000);
        a.advance(&a_seed, 500);
        let mut b = EnvironmentState::initial(&a_seed, 1000);
        b.advance(&a_seed, 500);
        assert_eq!(a, b, "同 seed 同 tick 序列 → 同状态（§32）");
        let c_seed = DreamscapeSeed::from_input("envdet-2");
        let mut c = EnvironmentState::initial(&c_seed, 1000);
        c.advance(&c_seed, 500);
        assert_ne!(a, c, "不同 seed 合理变化");
    }

    #[test]
    fn season_mapping() {
        assert_eq!(season_at_tick(0), Season::Spring);
        assert_eq!(season_at_tick(TICKS_PER_SEASON), Season::Summer);
        assert_eq!(season_at_tick(TICKS_PER_SEASON * 3 + 5), Season::Winter);
        assert_eq!(season_at_tick(TICKS_PER_SEASON * 4), Season::Spring);
    }

    #[test]
    fn palette_constraints_hold_across_seeds() {
        // §42：大量 seed × 环境 → 全部落在舒适区间；点缀频率低。
        for i in 0..300u64 {
            let seed = DreamscapeSeed::from_input(&format!("pal{i}"));
            let env = EnvironmentState::initial(&seed, i * 1000);
            let p = Palette::derive(&seed, "misty-forest", &env);
            assert!((SAT_MIN..=SAT_MAX).contains(&p.saturation));
            assert!((LUM_MIN..=LUM_MAX).contains(&p.luminance));
            assert!((0.0..=ACCENT_FREQ_MAX).contains(&p.accent_frequency));
            assert!((0.0..360.0).contains(&p.base_hue));
            assert!((0.0..360.0).contains(&p.accent_hue));
        }
    }

    #[test]
    fn palette_depends_on_seed_biome_env() {
        let seed = DreamscapeSeed::from_input("pal-dep");
        let env = EnvironmentState::initial(&seed, 0);
        let a = Palette::derive(&seed, "misty-forest", &env);
        let b = Palette::derive(&seed, "misty-forest", &env);
        assert_eq!(a, b);
        let c = Palette::derive(&seed, "quiet-meadow", &env);
        assert_ne!(a, c, "biome 影响 palette（§41）");
        let mut warm = env;
        warm.temperature = 35.0;
        let d = Palette::derive(&seed, "misty-forest", &warm);
        assert_ne!(a, d, "环境参与 palette（§41）");
    }

    #[test]
    fn env_serde_and_sanitize() {
        let seed = DreamscapeSeed::from_input("envser");
        let mut env = EnvironmentState::initial(&seed, 123);
        env.advance(&seed, 456);
        let json = serde_json::to_string(&env).unwrap();
        let mut back: EnvironmentState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, env);
        back.fog = 5.0;
        back.ambient_intensity = -1.0;
        back.sanitize();
        assert!(back.fog <= FOG_MAX);
        assert!((0.0..=1.0).contains(&back.ambient_intensity));
        let mut pal: Palette = serde_json::from_str(
            r#"{"base_hue":0.0,"accent_hue":0.0,"saturation":0.9,"luminance":0.1,"accent_frequency":0.9}"#,
        )
        .unwrap();
        pal.sanitize();
        assert_eq!(pal.saturation, SAT_MAX);
        assert_eq!(pal.luminance, LUM_MIN);
        assert_eq!(pal.accent_frequency, ACCENT_FREQ_MAX);
    }
}
