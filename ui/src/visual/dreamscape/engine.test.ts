// 渲染引擎纯逻辑单测（§187 状态机/§142–143 Auto 质量/§144–147 节流/
// §258–261 代际顺序/§267 平滑）。
import { describe, expect, it } from 'vitest';
import {
  AutoQuality,
  GenerationGate,
  Lifecycle,
  SmoothedValue,
  clamp01,
  lerp,
  shouldRender,
  targetFps,
} from './engine';

describe('Lifecycle（§129/§187）', () => {
  it('合法迁移链 create→loading→ready→rendering→paused→rendering', () => {
    const l = new Lifecycle();
    expect(l.transition('created')).toBe(true);
    expect(l.transition('loading')).toBe(true);
    expect(l.transition('ready')).toBe(true);
    expect(l.transition('rendering')).toBe(true);
    expect(l.transition('paused')).toBe(true);
    expect(l.transition('rendering')).toBe(true);
  });

  it('非法迁移拒绝（idle→rendering 等）', () => {
    const l = new Lifecycle();
    expect(l.transition('rendering')).toBe(false);
    l.transition('created');
    expect(l.transition('rendering')).toBe(false);
    expect(l.get()).toBe('created');
  });

  it('disposed 是终点（§158：dispose 后不可复用）', () => {
    const l = new Lifecycle();
    l.transition('created');
    l.transition('disposed');
    for (const next of ['created', 'loading', 'ready', 'rendering', 'paused'] as const) {
      expect(l.transition(next)).toBe(false);
    }
  });
});

describe('AutoQuality（§142/§143）', () => {
  it('持续高帧时升档，持续低帧时降档', () => {
    const q = new AutoQuality(0, 20, 45);
    let now = 0;
    // 3s 高帧（16ms/帧 ≈ 187 帧）。
    for (let i = 0; i < 200; i += 1) {
      now += 16;
      q.sample(16, now);
    }
    expect(q.current()).toBe('high');
    // 2s 低帧。
    for (let i = 0; i < 60; i += 1) {
      now += 50;
      q.sample(50, now);
    }
    expect(q.current()).toBe('medium');
  });

  it('迟滞冷却：变化后冷却期内不再变化（§143 不震荡）', () => {
    const q = new AutoQuality(4000, 20, 45);
    let now = 0;
    // 5.6s 高帧：跨过 3s 升档阈值 + 4s 冷却。
    for (let i = 0; i < 350; i += 1) {
      now += 16;
      q.sample(16, now);
    }
    expect(q.current()).toBe('high');
    // 立刻低帧：冷却期内（4000ms）不得降档。
    for (let i = 0; i < 40; i += 1) {
      now += 50;
      const changed = q.sample(50, now);
      expect(changed).toBeNull();
    }
    expect(q.current()).toBe('high');
  });

  it('中等帧长不触发（滞后带）', () => {
    const q = new AutoQuality(0, 20, 45);
    let now = 0;
    for (let i = 0; i < 500; i += 1) {
      now += 30;
      q.sample(30, now);
    }
    expect(q.current()).toBe('medium');
  });
});

describe('targetFps + shouldRender（§144–§147）', () => {
  it('播放 30 / idle 10 / 隐藏 2 / reducedMotion 5', () => {
    expect(targetFps(true, false, true)).toBe(30);
    expect(targetFps(false, false, true)).toBe(10);
    expect(targetFps(true, false, false)).toBe(2);
    expect(targetFps(true, true, true)).toBe(5);
  });

  it('节流按帧间隔放行', () => {
    expect(shouldRender(100, 0, 30)).toBe(true); // 100ms > 33ms
    expect(shouldRender(20, 0, 30)).toBe(false);
  });
});

describe('GenerationGate（§258–§261）', () => {
  it('最新代胜出：旧代完成不得覆盖新世界（§259/§260）', () => {
    const g = new GenerationGate();
    const a = g.begin(); // world A
    const b = g.begin(); // world B（A 完成前重新请求）
    expect(g.isCurrent(a)).toBe(false);
    expect(g.isCurrent(b)).toBe(true);
    expect(g.complete(a)).toBe(false); // A 迟到 → 拒绝
    expect(g.complete(b)).toBe(true);
  });

  it('代号有界增长（§219 精神）', () => {
    const g = new GenerationGate();
    for (let i = 0; i < 100; i += 1) g.begin();
    expect(g.isCurrent(100)).toBe(true);
  });
});

describe('插值与平滑（§163/§164/§266/§267）', () => {
  it('clamp01/lerp 边界', () => {
    expect(clamp01(-0.5)).toBe(0);
    expect(clamp01(1.5)).toBe(1);
    expect(lerp(0, 10, 0.25)).toBeCloseTo(2.5);
    expect(lerp(0, 10, 2)).toBe(10); // 超界 t 钳制
  });

  it('SmoothedValue 固定速率逼近（无突变）', () => {
    const s = new SmoothedValue(0, 2); // 2/s
    expect(s.step(10, 1)).toBe(2); // 1s → 2
    expect(s.step(10, 1)).toBe(4);
    expect(s.step(10, 10)).toBe(10); // 大 dt 一步到位（钳制不越过）
    expect(s.get()).toBe(10);
  });
});
