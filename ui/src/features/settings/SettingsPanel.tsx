// 设置面板（§114/§119/§107）：语言/缩放/reducedMotion/loudness/crossfade/EQ。
// 全部真实持久化（Rust JSON v1）并即时生效。
import { useEffect, useState } from 'react';
import { useSettingsStore, useUiStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';
import type { Locale } from '../../i18n';

export function SettingsPanel() {
  const open = useUiStore((s) => s.settingsOpen);
  const setOpen = useUiStore((s) => s.setSettingsOpen);
  const s = useSettingsStore();
  const { t } = useTranslation(s.locale);

  const [crossfade, setCrossfade] = useState(s.crossfadeSeconds);

  useEffect(() => {
    setCrossfade(s.crossfadeSeconds);
  }, [s.crossfadeSeconds, open]);

  if (!open) return null;

  const persist = (patch: {
    language?: string;
    uiScale?: number;
    reducedMotion?: boolean;
    loudnessEnabled?: boolean;
    crossfadeSeconds?: number;
    eqEnabled?: boolean;
    eqPreset?: number;
    micaEnabled?: boolean;
  }) => {
    const merged = {
      version: 1,
      language: patch.language ?? s.locale,
      uiScale: patch.uiScale ?? s.uiScale,
      reducedMotion: patch.reducedMotion ?? s.reducedMotion,
      loudnessEnabled: patch.loudnessEnabled ?? s.loudnessEnabled,
      crossfadeSeconds: patch.crossfadeSeconds ?? s.crossfadeSeconds,
      eqEnabled: patch.eqEnabled ?? s.eqEnabled,
      eqPreset: patch.eqPreset ?? s.eqPreset,
      micaEnabled: patch.micaEnabled ?? s.micaEnabled,
    };
    void ipc.saveSettings(merged).catch(() => undefined);
  };

  return (
    <div className="palette-overlay" role="presentation" onClick={() => setOpen(false)}>
      <div
        className="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-label={t('settings.title')}
        onClick={(e) => e.stopPropagation()}
      >
        <h2>{t('settings.title')}</h2>

        <label className="settings-row">
          <span>{t('settings.language')}</span>
          <select
            value={s.locale}
            onChange={(e) => {
              const locale = (e.target.value === 'en' ? 'en' : 'zh-CN') as Locale;
              s.setLocale(locale);
              persist({ language: locale });
            }}
          >
            <option value="zh-CN">简体中文</option>
            <option value="en">English</option>
          </select>
        </label>

        <label className="settings-row">
          <span>{t('settings.uiScale')}</span>
          <select
            value={String(s.uiScale)}
            onChange={(e) => {
              const v = Number(e.target.value);
              s.setUiScale(v);
              persist({ uiScale: v });
            }}
          >
            {[0.9, 1.0, 1.1, 1.2, 1.3, 1.4].map((v) => (
              <option key={v} value={String(v)}>
                {Math.round(v * 100)}%
              </option>
            ))}
          </select>
        </label>

        <label className="settings-row">
          <span>{t('settings.mica')}</span>
          <input
            type="checkbox"
            checked={s.micaEnabled}
            onChange={(e) => {
              s.setAll({ ...s, micaEnabled: e.target.checked });
              void ipc.setMica(e.target.checked, true).catch(() => undefined);
            }}
          />
        </label>

        <label className="settings-row">
          <span>{t('settings.reducedMotion')}</span>
          <input
            type="checkbox"
            checked={s.reducedMotion}
            onChange={(e) => {
              s.setReducedMotion(e.target.checked);
              persist({ reducedMotion: e.target.checked });
            }}
          />
        </label>

        <label className="settings-row">
          <span>{t('settings.loudness')}</span>
          <input
            type="checkbox"
            checked={s.loudnessEnabled}
            onChange={(e) => {
              s.setAll({ ...s, loudnessEnabled: e.target.checked });
              void ipc.setLoudness(e.target.checked, true).catch(() => undefined);
            }}
          />
        </label>

        <label className="settings-row">
          <span>{t('settings.crossfade')}</span>
          <input
            type="range"
            min={0}
            max={12}
            step={0.5}
            value={crossfade}
            onChange={(e) => setCrossfade(Number(e.target.value))}
            onMouseUp={() => {
              s.setAll({ ...s, crossfadeSeconds: crossfade });
              void ipc.setCrossfade(crossfade, true).catch(() => undefined);
            }}
            onTouchEnd={() => {
              s.setAll({ ...s, crossfadeSeconds: crossfade });
              void ipc.setCrossfade(crossfade, true).catch(() => undefined);
            }}
          />
          <span>{crossfade}s</span>
        </label>

        <label className="settings-row">
          <span>{t('settings.eq')}</span>
          <select
            value={String(s.eqPreset)}
            onChange={(e) => {
              const preset = Number(e.target.value);
              s.setAll({ ...s, eqPreset: preset });
              void ipc.setEqPreset(preset, true).catch(() => undefined);
            }}
          >
            {[0, 1, 2, 3, 4, 5].map((i) => (
              <option key={i} value={String(i)}>
                {t(`settings.eq${['Flat', 'Bass', 'Treble', 'Vocal', 'Electronic', 'Acoustic'][i]}`)}
              </option>
            ))}
          </select>
        </label>

        <div className="settings-actions">
          <button className="btn" onClick={() => setOpen(false)}>
            {t('common.ok')}
          </button>
        </div>
      </div>
    </div>
  );
}
