// ESLint flat config。Phase 0 建立结构；裸色值禁用规则等专项 lint 规则随 M6 落地。
import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'src/generated'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    rules: {
      'no-console': 'warn',
    },
  },
);
