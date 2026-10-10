// 渲染引擎纯逻辑单测（§142–143 Auto 质量 / §144–147 节流）。
import { describe, expect, it } from 'vitest';
import { AutoQuality, clamp01, shouldRender, targetFps } from './engine';

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

describe('节流与钳制（§144–§147）', () => {
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

  it('clamp01 边界', () => {
    expect(clamp01(-0.5)).toBe(0);
    expect(clamp01(1.5)).toBe(1);
  });
});
