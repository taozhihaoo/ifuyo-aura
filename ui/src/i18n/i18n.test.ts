import { describe, expect, it } from 'vitest';
import { catalogs } from './index';

// 文案契约：zh/en 键集一致、代码里引用的键必须存在、用户可见文案不得
// 泄漏内部里程碑编号（此前 "先在 M8 分析" 直接进了 Map 空态）。
const zh = catalogs['zh-CN'];
const en = catalogs.en;

describe('i18n catalogs', () => {
  it('keeps the zh and en key sets identical', () => {
    const onlyZh = Object.keys(zh).filter((k) => !(k in en)).sort();
    const onlyEn = Object.keys(en).filter((k) => !(k in zh)).sort();
    expect({ onlyZh, onlyEn }, `键集不一致：${JSON.stringify({ onlyZh, onlyEn })}`).toEqual({
      onlyZh: [],
      onlyEn: [],
    });
  });

  it('defines every key referenced by a literal t() call', () => {
    const sources = import.meta.glob<string>('../**/*.{ts,tsx}', {
      query: '?raw',
      import: 'default',
      eager: true,
    });
    const missing: string[] = [];
    let scanned = 0;
    for (const [file, raw] of Object.entries(sources)) {
      if (file.includes('/i18n/') || /\.test\.[tj]sx?$/.test(file)) continue;
      scanned += 1;
      const src = raw.replace(/\/\/[^\n]*/g, '');
      for (const m of src.matchAll(/(?<![\w.$])t\(\s*'([\w.]+)'/g)) {
        if (m[1] && !(m[1] in zh)) missing.push(`${m[1]} ← ${file}`);
      }
    }
    // glob 失配会让本断言空转。
    expect(scanned).toBeGreaterThan(10);
    expect([...new Set(missing)].sort()).toEqual([]);
  });

  it('keeps internal milestone codes out of user-facing copy', () => {
    const offenders = [...Object.entries(zh), ...Object.entries(en)]
      .filter(([, value]) => /\bM\d{1,2}\b/.test(value))
      .map(([key]) => key);
    expect([...new Set(offenders)].sort()).toEqual([]);
  });
});
