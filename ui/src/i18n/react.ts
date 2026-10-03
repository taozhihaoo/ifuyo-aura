import { useSettingsStore } from '../state/stores';
import { translate, type Locale } from './index';

export function useTranslation(locale?: Locale) {
  const current = useSettingsStore((s) => s.locale);
  const active = locale ?? current;
  return { t: (key: string, params?: Record<string, string | number>) => translate(active, key, params), locale: active };
}
