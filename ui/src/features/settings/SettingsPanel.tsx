// 设置面板（§114/§119/§107）：语言/缩放/reducedMotion/loudness/crossfade/EQ。
// 全部真实持久化（Rust JSON v1）并即时生效。
import { useEffect, useState } from 'react';
import { useSettingsStore, useUiStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';
import type { Locale } from '../../i18n';
import { updateService, useUpdateStore } from '../update/UpdateService';
import type { UpdateState } from '../update/updateTypes';

export function SettingsPanel() {
  const open = useUiStore((s) => s.settingsOpen);
  const setOpen = useUiStore((s) => s.setSettingsOpen);
  const s = useSettingsStore();
  const { t } = useTranslation(s.locale);
  const update = useUpdateStore((st) => st.state);

  // §7 typed 状态 → i18n key（§66 本地化；§100 人类可读）。
  const updateState = (): UpdateState => update;
  const updateStatusText = (): string => {
    const k = update.kind;
    switch (k) {
      case 'idle':
        return '';
      case 'checking':
        return t('update.checking');
      case 'upToDate':
        return t('update.upToDate');
      case 'available':
        return t('update.available', { version: update.version });
      case 'downloading':
        return update.progress !== null
          ? t('update.downloadingProgress', { progress: update.progress })
          : t('update.downloading');
      case 'downloaded':
        return t('update.downloaded');
      case 'installing':
        return t('update.installing');
      case 'restartRequired':
        return t('update.restartRequired');
      case 'cancelled':
        return t('update.cancelled');
      case 'failed':
        return t('update.failed', { kind: t(`update.error.${update.error.kind}`) });
    }
  };

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
    updatesAutoCheck?: boolean;
  }) => {
    const merged = {
      version: 1,
      language: patch.language ?? s.locale,
      uiScale: patch.uiScale ?? s.uiScale,
      updatesAutoCheck: patch.updatesAutoCheck ?? s.updatesAutoCheck,
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

        {/* M11 §34/§84/§85：更新区（用户控制；无弹窗 §35；typed 状态 §7） */}
        <div className="settings-row settings-updates" role="group" aria-label={t('update.section')}>
          <span>{t('update.section')}</span>
          <div className="settings-updates-controls">
            <label className="settings-check">
              <input
                type="checkbox"
                checked={s.updatesAutoCheck}
                onChange={(e) => {
                  s.setUpdatesAutoCheck?.(e.target.checked);
                  persist({ updatesAutoCheck: e.target.checked });
                }}
              />
              {t('update.autoCheck')}
            </label>
            <div className="settings-update-status" role="status">
              {updateStatusText()}
            </div>
            <button
              className="ghost-btn"
              onClick={() => void updateService.checkForUpdate()}
              disabled={updateState().kind === 'checking'}
            >
              {t('update.check')}
            </button>
            {updateState().kind === 'available' && (
              <button
                className="primary-btn"
                onClick={() => void updateService.downloadAndInstall()}
              >
                {t('update.install')}
              </button>
            )}
            {updateState().kind === 'checking' || updateState().kind === 'downloading' ? (
              <button className="ghost-btn" onClick={() => updateService.cancel()}>
                {t('update.cancel')}
              </button>
            ) : null}
          </div>
        </div>

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
