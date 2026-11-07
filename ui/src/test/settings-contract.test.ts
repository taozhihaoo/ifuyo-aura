// P0 回归：设置 DTO 的 IPC 契约。
// 后端 AppSettings 曾缺 serde(rename_all = "camelCase")，snake_case 载荷让
// 前端把一整包 undefined 写进 store，而 E2E 脚本直接按 snake_case 调 IPC
// 所以没抓到。这里固定「真实消费端」的字段名。
import { describe, expect, it } from 'vitest';
import { isAppSettings } from '../ipc/types';
import { errorToI18nKey } from '../ipc/errors';

const camelCasePayload = {
  version: 1,
  language: 'zh-CN',
  updatesAutoCheck: true,
  mojibakeFix: true,
  analysisAuto: true,
  uiScale: 1,
  reducedMotion: false,
  loudnessEnabled: false,
  crossfadeSeconds: 0,
  eqEnabled: true,
  eqPreset: 0,
  micaEnabled: true,
};

const legacySnakeCasePayload = {
  version: 1,
  language: 'zh-CN',
  updates_auto_check: true,
  mojibake_fix: true,
  analysis_auto: true,
  ui_scale: 1,
  reduced_motion: false,
  loudness_enabled: false,
  crossfade_seconds: 0,
  eq_enabled: true,
  eq_preset: 0,
  mica_enabled: true,
};

describe('AppSettings IPC contract', () => {
  it('accepts the complete camelCase payload', () => {
    expect(isAppSettings(camelCasePayload)).toBe(true);
    // 逐字段可读，避免 undefined 流进 useSettingsStore.setAll
    for (const key of Object.keys(camelCasePayload)) {
      expect((camelCasePayload as Record<string, unknown>)[key]).not.toBeUndefined();
    }
  });

  it('rejects the legacy snake_case payload instead of silently storing undefined', () => {
    expect(isAppSettings(legacySnakeCasePayload)).toBe(false);
  });

  it('rejects missing fields and non-finite numbers', () => {
    const { crossfadeSeconds: _cf, ...missing } = camelCasePayload;
    void _cf;
    expect(isAppSettings(missing)).toBe(false);
    expect(isAppSettings({ ...camelCasePayload, uiScale: Number.NaN })).toBe(false);
    expect(isAppSettings({ ...camelCasePayload, crossfadeSeconds: Infinity })).toBe(false);
    expect(isAppSettings(null)).toBe(false);
  });

  it('maps the corrupt-settings error to a localized message', () => {
    expect(errorToI18nKey({ code: 'SETTINGS_CORRUPT' })).toBe('error.settingsCorrupt');
    expect(errorToI18nKey({ code: 'MAP_CORRUPT' })).toBe('error.mapCorrupt');
  });
});
