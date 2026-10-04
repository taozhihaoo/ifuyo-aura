// Dreamscape typed IPC 契约（§255/§110）：与 src-tauri/src/dreamscape.rs +
// aura-synth manifest DTO 一一镜像。禁止 any；解析前置校验（§253
// validation-first：畸形 manifest 拒绝而非渲染崩溃）。

export interface DreamscapeSeedDto {
  value: number;
  seedVersion: number;
}

export interface DreamscapeTerrainPatch {
  amplitude: number;
  frequency: number;
  terrainSeed: number;
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

export type LandmarkKind = 'OldTree' | 'StandingStone' | 'StillPool' | 'Arch' | 'GlowCluster';

export interface DreamscapeLandmark {
  id: number;
  kind: LandmarkKind;
  x: number;
  y: number;
  z: number;
  radius: number;
  rotation: number;
}

export type ObjectFamily = 'flora' | 'stone' | 'water' | 'light' | 'particle' | 'landmark';

export interface DreamscapeInstanceGroup {
  family: ObjectFamily;
  x: number[];
  z: number[];
  scale: number[];
  rotation: number[];
  nonce: number[];
}

export interface SceneManifest {
  manifestVersion: number;
  /** u64（JSON number；UI 不做身份比较，仅透传）。 */
  worldId: number;
  seed: DreamscapeSeedDto;
  terrain: DreamscapeTerrainPatch;
  palette: DreamscapePalette;
  light: DreamscapeLight;
  atmosphere: DreamscapeAtmosphere;
  landmarks: DreamscapeLandmark[];
  instances: DreamscapeInstanceGroup[];
}

/** §168/§169 数值安全：非有限值一律拒绝。 */
function finite(v: unknown): v is number {
  return typeof v === 'number' && Number.isFinite(v);
}

function finiteArray(v: unknown): v is number[] {
  return Array.isArray(v) && v.every(finite);
}

/** manifest 前置校验（§253：畸形输入拒绝；上限检查 §115/§254）。 */
export function isSceneManifest(v: unknown): v is SceneManifest {
  if (typeof v !== 'object' || v === null) return false;
  const o = v as Record<string, unknown>;
  if (o.manifestVersion !== 1) return false;
  if (!finite(o.worldId)) return false;
  const seed = o.seed as Record<string, unknown> | undefined;
  if (!seed || !finite(seed.value) || seed.seedVersion !== 1) return false;
  const t = o.terrain as Record<string, unknown> | undefined;
  if (!t || !finite(t.amplitude) || !finite(t.frequency) || !finite(t.terrainSeed)) return false;
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
  if (!li || !finite(li.sunIntensity) || !finite(li.ambientIntensity) || !finite(li.horizonGlow)) {
    return false;
  }
  const at = o.atmosphere as Record<string, unknown> | undefined;
  if (!at || !finite(at.fog) || !finite(at.wind) || !finite(at.mistBias)) return false;
  if (!Array.isArray(o.landmarks) || o.landmarks.length > 16) return false;
  for (const lm of o.landmarks) {
    const m = lm as Record<string, unknown>;
    if (!finite(m.id) || !finite(m.x) || !finite(m.y) || !finite(m.z)) return false;
    if (typeof m.kind !== 'string') return false;
  }
  if (!Array.isArray(o.instances) || o.instances.length > 12) return false;
  let total = 0;
  for (const g of o.instances) {
    const m = g as Record<string, unknown>;
    if (typeof m.family !== 'string') return false;
    if (!finiteArray(m.x) || !finiteArray(m.z) || !finiteArray(m.scale) || !finiteArray(m.rotation)) {
      return false;
    }
    total += (m.x as number[]).length;
  }
  // §115/§63 预算：manifest 实例总数超界 → 拒绝（防 object-count abuse）。
  return total <= 2048;
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
