// Dreamscape typed IPC 契约（§255/§110）：与 src-tauri/src/dreamscape.rs +
// aura-synth 光场 DTO 一一镜像。禁止 any；解析前置校验（§253
// validation-first：畸形数据拒绝而非渲染崩溃）。
//
// v2：Dreamscape 不再建模 3D 世界（terrain 高度场 / landmark / instance
// 数组全部撤除），改为音乐驱动的极光光场——契约只剩参数。

export interface DreamscapeSeedDto {
  value: number;
  seedVersion: number;
}

export interface DreamscapePalette {
  baseHue: number;
  accentHue: number;
  saturation: number;
  luminance: number;
  accentFrequency: number;
}

export interface DreamscapeLight {
  sunIntensity: number;
  ambientIntensity: number;
  horizonGlow: number;
}

export interface DreamscapeAtmosphere {
  fog: number;
  wind: number;
  mistBias: number;
}

/** 光场形态参数（genome 语义重映射；全部 [0,1]）。 */
export interface DreamscapeFieldParams {
  /** 幕帘纵深与层叠 */
  curtainDepth: number;
  /** 光尘密度 */
  moteDensity: number;
  /** 雾幕不透明度 */
  veil: number;
  /** 辉光扩散半径 */
  glowSoftness: number;
  /** 地平镜面强度 */
  mirror: number;
  /** 星野尺度与疏密 */
  starScale: number;
  /** 地平线高度 */
  horizon: number;
}

export type FieldMotif = 'veil' | 'cascade' | 'radiance' | 'drift';

const MOTIFS: readonly string[] = ['veil', 'cascade', 'radiance', 'drift'];

/** 光场清单（渲染层唯一输入）。 */
export interface LightField {
  manifestVersion: number;
  /** u64（JSON number；UI 不做身份比较，仅透传）。 */
  worldId: number;
  seed: DreamscapeSeedDto;
  fieldSeed: number;
  motif: FieldMotif;
  palette: DreamscapePalette;
  light: DreamscapeLight;
  atmosphere: DreamscapeAtmosphere;
  params: DreamscapeFieldParams;
}

/** §168/§169 数值安全：非有限值一律拒绝。 */
function finite(v: unknown): v is number {
  return typeof v === 'number' && Number.isFinite(v);
}

function inUnit(v: unknown): v is number {
  return finite(v) && v >= 0 && v <= 1;
}

/** 逐字段区间必须与 Rust `GenomeField::bounds` 一致——放宽会漏放畸形，
 *  收紧会把真实光场判非法（视图永久 fallback）。 */
function between(v: unknown, lo: number, hi: number): v is number {
  return finite(v) && v >= lo && v <= hi;
}

/** 光场前置校验（§253：畸形输入拒绝；参数必须落在声明区间内）。 */
export function isLightField(v: unknown): v is LightField {
  if (typeof v !== 'object' || v === null) return false;
  const o = v as Record<string, unknown>;
  // 版本双端硬钉：Rust MANIFEST_VERSION 改动必须同步这里。
  if (o.manifestVersion !== 2) return false;
  if (!finite(o.worldId) || !finite(o.fieldSeed)) return false;
  const seed = o.seed as Record<string, unknown> | undefined;
  if (!seed || !finite(seed.value) || seed.seedVersion !== 1) return false;
  if (typeof o.motif !== 'string' || !MOTIFS.includes(o.motif)) return false;
  const p = o.palette as Record<string, unknown> | undefined;
  if (
    !p ||
    !finite(p.baseHue) ||
    !finite(p.accentHue) ||
    !finite(p.saturation) ||
    !finite(p.luminance) ||
    !finite(p.accentFrequency)
  ) {
    return false;
  }
  const li = o.light as Record<string, unknown> | undefined;
  if (!li || !inUnit(li.sunIntensity) || !inUnit(li.ambientIntensity) || !inUnit(li.horizonGlow)) {
    return false;
  }
  const at = o.atmosphere as Record<string, unknown> | undefined;
  if (!at || !inUnit(at.fog) || !inUnit(at.wind) || !inUnit(at.mistBias)) return false;
  const pa = o.params as Record<string, unknown> | undefined;
  if (!pa) return false;
  return (
    between(pa.curtainDepth, 0.5, 4.0) &&
    inUnit(pa.moteDensity) &&
    inUnit(pa.veil) &&
    inUnit(pa.glowSoftness) &&
    inUnit(pa.mirror) &&
    between(pa.starScale, 0.5, 2.0) &&
    inUnit(pa.horizon)
  );
}

/** 会话状态 DTO（dreamscape_state 命令）。 */
export interface DreamscapeStateDto {
  /** §262 revision：UI 只接受最新状态。 */
  revision: number;
  worldId: string;
  seedHuman: string;
  generationVersion: number;
  biomeId: string;
  tick: number;
  timePhase: number;
  weather: string;
  fog: number;
  wind: number;
  sunIntensity: number;
  ambientIntensity: number;
  horizonGlow: number;
  mutationCount: number;
  eventCount: number;
}

export function isDreamscapeState(v: unknown): v is DreamscapeStateDto {
  if (typeof v !== 'object' || v === null) return false;
  const o = v as Record<string, unknown>;
  return typeof o.worldId === 'string' && typeof o.seedHuman === 'string' && finite(o.tick);
}
