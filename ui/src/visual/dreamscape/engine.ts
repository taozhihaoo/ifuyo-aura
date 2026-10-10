// Dreamscape 渲染引擎的纯逻辑层（§128–§150/§265–§267/§287）。
// 全部决策（质量档/节流/插值/取消）在此模块——无 WebGL 依赖，可单测；
// GL 装配在 LightFieldRenderer.ts（裸 WebGL2，无 three）。

/** 质量档（§141）。 */
export type QualityMode = 'low' | 'medium' | 'high' | 'auto';
export type QualityLevel = 'low' | 'medium' | 'high';

/** 档位的具体参数（§141/§142/§150）。 */
export interface QualitySettings {
  level: QualityLevel;
  /** 内部分辨率 cap（§150：4K×200% 不失控）。 */
  pixelRatioCap: number;
  /** fbm 迭代层数（光场细节；取代 3D 地形网格密度）。 */
  fieldDetail: number;
  /** 光尘密度系数。 */
  moteDensity: number;
  /** 辉光/镜面反射等叠加层。 */
  glowEnabled: boolean;
}

export const QUALITY_SETTINGS: Record<QualityLevel, QualitySettings> = {
  low: { level: 'low', pixelRatioCap: 1.0, fieldDetail: 3, moteDensity: 0.35, glowEnabled: false },
  medium: { level: 'medium', pixelRatioCap: 1.25, fieldDetail: 4, moteDensity: 0.7, glowEnabled: true },
  high: { level: 'high', pixelRatioCap: 1.75, fieldDetail: 5, moteDensity: 1.0, glowEnabled: true },
};

/** Auto 控制器（§142/§143：按帧时长升降档 + 迟滞冷却，不震荡）。 */
export class AutoQuality {
  private level: QualityLevel = 'medium';
  private lastChangeMs = 0;
  private highMs = 0;
  private lowMs = 0;

  constructor(
    private readonly cooldownMs = 4000,
    private readonly upThresholdMs = 20, // 帧时长 < 20ms（>50fps）持续 3s → 升档
    private readonly downThresholdMs = 45, // 帧时长 > 45ms（<22fps）持续 2s → 降档
  ) {}

  current(): QualityLevel {
    return this.level;
  }

  /** 每帧采样；返回新档位（变化时）。 */
  sample(frameMs: number, nowMs: number): QualityLevel | null {
    if (frameMs > this.downThresholdMs) {
      this.lowMs += frameMs;
      this.highMs = 0;
    } else if (frameMs < this.upThresholdMs) {
      this.highMs += frameMs;
      this.lowMs = 0;
    } else {
      this.highMs = 0;
      this.lowMs = 0;
      return null;
    }
    if (nowMs - this.lastChangeMs < this.cooldownMs) {
      return null; // §143 迟滞/冷却
    }
    if (this.lowMs >= 2000 && this.level !== 'low') {
      this.level = this.level === 'high' ? 'medium' : 'low';
      this.commit(nowMs);
      return this.level;
    }
    if (this.highMs >= 3000 && this.level !== 'high') {
      this.level = this.level === 'low' ? 'medium' : 'high';
      this.commit(nowMs);
      return this.level;
    }
    return null;
  }

  private commit(nowMs: number) {
    this.lastChangeMs = nowMs;
    this.highMs = 0;
    this.lowMs = 0;
  }
}

/** 渲染节流决策（§144/§145/§146/§147）。 */
export function targetFps(playing: boolean, reducedMotion: boolean, visible: boolean): number {
  if (!visible) return 2; // 最小化/隐藏（§146/§147）
  if (reducedMotion) return 5;
  return playing ? 30 : 10; // 播放 30；idle 5–15 区间取 10（§145）
}

/** 是否应渲染本帧（节流；§128 render loop 内部判定）。 */
export function shouldRender(nowMs: number, lastFrameMs: number, fps: number): boolean {
  return nowMs - lastFrameMs >= 1000 / fps;
}

/** 钳制到 [0,1]。 */
export function clamp01(t: number): number {
  return Math.min(Math.max(t, 0), 1);
}
