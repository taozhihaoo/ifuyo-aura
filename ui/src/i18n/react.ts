import { useCallback } from 'react';
import { useSettingsStore } from '../state/stores';
import { translate, type Locale } from './index';

export function useTranslation(locale?: Locale) {
  const current = useSettingsStore((s) => s.locale);
  const active = locale ?? current;
  // t 常被当作 effect/useMemo 依赖：按 locale 缓存，避免每次渲染都重建闭包。
  const t = useCallback(
    (key: string, params?: Record<string, string | number>) => translate(active, key, params),
    [active],
  );
  return { t, locale: active };
}
