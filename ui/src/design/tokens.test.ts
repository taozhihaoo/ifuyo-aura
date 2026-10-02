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
});
