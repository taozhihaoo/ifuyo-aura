import { describe, expect, it } from 'vitest';
import { colors, motion, radius, text, typeScale } from './tokens';

// 设计 token 契约：颜色必须与 brand.json 一致（scripts/generate-tokens.mjs 亦做该校验）。
describe('design tokens', () => {
  it('exposes the five brand colors from brand.json', () => {
    expect(Object.keys(colors).sort()).toEqual(['bg', 'mint', 'rose', 'sky', 'violet']);
    expect(colors.bg).toBe('#0B0E14');
    expect(colors.rose).toBe('#F2789F');
    expect(colors.violet).toBe('#8B7CF6');
    expect(colors.sky).toBe('#5CC8F2');
    expect(colors.mint).toBe('#6FE3C1');
  });

  it('follows the charter design language', () => {
    // 文本 primary 92% / secondary 60% white
    expect(text.primary).toBe('rgba(255, 255, 255, 0.92)');
    expect(text.secondary).toBe('rgba(255, 255, 255, 0.60)');
    // 圆角 14px
    expect(radius.md).toBe('14px');
    // 字号阶梯 12/14/16/20/32
    expect(Object.values(typeScale)).toEqual(['12px', '14px', '16px', '20px', '32px']);
    // 动效 120~240ms
    expect(Object.values(motion)).toEqual(['120ms', '180ms', '240ms']);
  });

  // 生成链门禁：样式表引用的每个 var(--x) 必须有定义。未定义时整条声明
  // 按 CSS 级联语义失效（background/字号/transition 静归零），而 TS 侧
  // token 断言全绿拦不住——该校验必须落在 CSS 这一侧。
  it('defines every CSS variable the stylesheets reference', () => {
    const sheets = import.meta.glob<string>('../**/*.css', {
      query: '?raw',
      import: 'default',
      eager: true,
    });
    // glob 失配会让本断言空转，先钉住它确实扫到样式表。
    expect(Object.keys(sheets).length).toBeGreaterThan(1);
    const defined = new Set<string>();
    const referenced = new Set<string>();
    for (const raw of Object.values(sheets)) {
      // 注释里的 var(--color-*) 示例不算引用。
      const css = raw.replace(/\/\*[\s\S]*?\*\//g, '');
      for (const m of css.matchAll(/(--[\w-]+)\s*:/g)) if (m[1]) defined.add(m[1]);
      for (const m of css.matchAll(/var\(\s*(--[\w-]+)/g)) if (m[1]) referenced.add(m[1]);
    }
    const missing = [...referenced].filter((v) => !defined.has(v)).sort();
    expect(missing, `未定义的 CSS 变量：${missing.join(', ')}`).toEqual([]);
  });
});
