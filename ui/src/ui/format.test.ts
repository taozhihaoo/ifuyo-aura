// §253/§254/§255/§256：时长与数值安全。
import { describe, expect, it } from 'vitest';
import { clampNumber, formatDuration, safeProgress } from './format';

describe('formatDuration（§253/§254）', () => {
  it('mm:ss（<1h）', () => {
    expect(formatDuration(0)).toBe('0:00');
    expect(formatDuration(65)).toBe('1:05');
    expect(formatDuration(599)).toBe('9:59');
  });

  it('h:mm:ss（≥1h）', () => {
    expect(formatDuration(3600)).toBe('1:00:00');
    expect(formatDuration(7325)).toBe('2:02:05');
  });

  it('非法输入 → null（调用方展示 Live/Unknown，绝不 NaN:NaN）', () => {
    expect(formatDuration(null)).toBeNull();
    expect(formatDuration(undefined)).toBeNull();
    expect(formatDuration(Number.NaN)).toBeNull();
    expect(formatDuration(Number.POSITIVE_INFINITY)).toBeNull();
    expect(formatDuration(-1)).toBeNull();
  });
});

describe('clampNumber（§255）', () => {
  it('超界钳制', () => {
    expect(clampNumber(1.5, 0, 1)).toBe(1);
    expect(clampNumber(-0.2, 0, 1)).toBe(0);
    expect(clampNumber(0.4, 0, 1)).toBeCloseTo(0.4);
  });

  it('NaN → min', () => {
    expect(clampNumber(Number.NaN, 0, 1)).toBe(0);
  });
});

describe('safeProgress（§256）', () => {
  it('0 ≤ progress ≤ 1，duration 非法 → 0', () => {
    expect(safeProgress(30, 100)).toBe(0.3);
    expect(safeProgress(150, 100)).toBe(1); // 后端短暂超出 → 不溢出
    expect(safeProgress(-5, 100)).toBe(0);
    expect(safeProgress(30, null)).toBe(0);
    expect(safeProgress(30, 0)).toBe(0);
    expect(safeProgress(Number.NaN, 100)).toBe(0);
  });
});
