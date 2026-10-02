// ifuyo Aura 设计 token —— 唯一事实来源（Charter §4.2 / §27）。
//
// `colors` 必须与根目录 brand.json 的 `colors` 完全一致；
// `npm run tokens`（scripts/generate-tokens.mjs）负责校验一致性并生成
// ui/src/generated/tokens.css。业务代码禁止出现裸色值，一律消费
// 本文件的 token 或生成的 CSS variables。

export const colors = {
  bg: '#0B0E14',
  rose: '#F2789F',
  violet: '#8B7CF6',
  sky: '#5CC8F2',
  mint: '#6FE3C1',
} as const;

export type ColorToken = keyof typeof colors;

/** 表面：4%~8% white alpha（Charter §27） */
export const surface = {
  low: 'rgba(255, 255, 255, 0.04)',
  high: 'rgba(255, 255, 255, 0.08)',
  border: 'rgba(255, 255, 255, 0.08)',
} as const;

/** 文本：primary 92% / secondary 60% white（Charter §27） */
export const text = {
  primary: 'rgba(255, 255, 255, 0.92)',
  secondary: 'rgba(255, 255, 255, 0.60)',
} as const;

/** 圆角：14px（Charter §27） */
export const radius = {
  md: '14px',
} as const;

/** 字号阶梯：12 / 14 / 16 / 20 / 32（Charter §27） */
export const typeScale = {
  xs: '12px',
  sm: '14px',
  md: '16px',
  lg: '20px',
  xl: '32px',
} as const;

/** 动效：120~240ms，只优先 transform / opacity（Charter §28） */
export const motion = {
  fast: '120ms',
  base: '180ms',
  slow: '240ms',
} as const;
