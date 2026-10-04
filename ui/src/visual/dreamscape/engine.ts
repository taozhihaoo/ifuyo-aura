// Dreamscape 渲染引擎的纯逻辑层（§128–§150/§265–§267/§287）。
// 全部决策（质量档/节流/插值/取消）在此模块——无 WebGL 依赖，可单测；
// GL 装配在 scene.ts / DreamscapeRenderer.ts。

/** 质量档（§141）。 */
export type QualityMode = 'low' | 'medium' | 'high' | 'auto';
export type QualityLevel = 'low' | 'medium' | 'high';

/** 档位的具体参数（§141/§142/§150）。 */
export interface QualitySettings {
  level: QualityLevel;
  /** 内部分辨率 cap（§150：4K×200% 不失控）。 */
  pixelRatioCap: number;
  /** 地形网格细分。 */
  terrainSegments: number;
  /** 粒子数量比例（§142）。 */
  particleScale: number;
  /** 后处理开关（§139 可关闭降级）。 */
  postEnabled: boolean;
}

export const QUALITY_SETTINGS: Record<QualityLevel, QualitySettings> = {
  low: { level: 'low', pixelRatioCap: 1.0, terrainSegments: 32, particleScale: 0.35, postEnabled: false },
  medium: { level: 'medium', pixelRatioCap: 1.25, terrainSegments: 64, particleScale: 0.7, postEnabled: false },
  high: { level: 'high', pixelRatioCap: 1.5, terrainSegments: 96, particleScale: 1.0, postEnabled: true },
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

/** 环境目标插值（§266：renderer 本地平滑；§34/§37/§163 无突变）。 */
export function lerp(current: number, target: number, t: number): number {
  return current + (target - current) * clamp01(t);
}

/** 遥测平滑（§267：backend 低频快照 → renderer 本地平滑）。 */
export class SmoothedValue {
  private value: number;
  constructor(initial: number, private readonly speedPerSecond = 2.0) {
    this.value = initial;
  }
  get(): number {
    return this.value;
  }
  /** 向 target 以固定速率推进（dt 秒）。 */
  step(target: number, dtSeconds: number): number {
    const delta = target - this.value;
    const maxStep = this.speedPerSecond * dtSeconds;
    if (Math.abs(delta) <= maxStep) {
      this.value = target;
    } else {
      this.value += Math.sign(delta) * maxStep;
    }
    return this.value;
  }
}

/** 生成请求顺序保护（§258–§261：generation_id，最新者胜）。 */
export class GenerationGate {
  private latest = 0;
  private settled = new Set<number>();

  /** 开启新代；返回 generation_id。 */
  begin(): number {
    this.latest += 1;
    this.settled.add(this.latest);
    // 只保留近期代（有界，§219 精神）。
    if (this.settled.size > 8) {
      const min = this.latest - 7;
      for (const id of this.settled) {
        if (id < min) this.settled.delete(id);
      }
    }
    return this.latest;
  }

  /** 该代是否仍为最新（完成回调里检查；陈旧代丢弃 §259/§260）。 */
  isCurrent(id: number): boolean {
    return id === this.latest;
  }

  /** 世界加载完成标记（用于「陈旧代完成不得覆盖新世界」测试）。 */
  complete(id: number): boolean {
    if (!this.settled.has(id) || id !== this.latest) return false;
    return true;
  }
}

/** 渲染器生命周期状态机（§129；非法迁移抛错由调用方防御）。 */
export type LifecycleState =
  | 'idle'
  | 'created'
  | 'loading'
  | 'ready'
  | 'rendering'
  | 'paused'
  | 'contextLost'
  | 'disposed';

export class Lifecycle {
  private state: LifecycleState = 'idle';

  get(): LifecycleState {
    return this.state;
  }

  can(next: LifecycleState): boolean {
    const allowed: Record<LifecycleState, LifecycleState[]> = {
      idle: ['created'],
      created: ['loading', 'disposed'],
      loading: ['ready', 'disposed'],
      ready: ['rendering', 'disposed'],
      rendering: ['paused', 'contextLost', 'disposed'],
      paused: ['rendering', 'contextLost', 'disposed'],
      contextLost: ['rendering', 'disposed'], // §151 恢复后重建
      disposed: [],
    };
    return allowed[this.state].includes(next);
  }

  transition(next: LifecycleState): boolean {
    if (!this.can(next)) return false;
    this.state = next;
    return true;
  }
}
