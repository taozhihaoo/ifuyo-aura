// 自绘标题栏（§16/§17）：drag region + window controls + 主导航。
// Windows 权限经 capabilities 最小授权（§22）。
import { getCurrentWindow } from '@tauri-apps/api/window';
import { DREAMSCAPE_ENABLED, useUiStore, useSettingsStore } from '../state/stores';
import { useTranslation } from '../i18n/react';

const win = getCurrentWindow();

/** §263：双击 drag region → 最大化/还原（Windows 预期；交互元素不触发）。 */
function onDragRegionDoubleClick(e: React.MouseEvent) {
  const target = e.target as HTMLElement;
  if (!target.hasAttribute('data-tauri-drag-region')) return;
  void getCurrentWindow().toggleMaximize();
}

export function TitleBar() {
  const { t } = useTranslation(useSettingsStore((s) => s.locale));
  const view = useUiStore((s) => s.view);
  const setView = useUiStore((s) => s.setView);
  const setPaletteOpen = useUiStore((s) => s.setPaletteOpen);
  const setSettingsOpen = useUiStore((s) => s.setSettingsOpen);

  return (
    <header className="titlebar" data-tauri-drag-region onDoubleClick={onDragRegionDoubleClick}>
      <div className="titlebar-brand" data-tauri-drag-region>
        ifuyo <span className="brand-accent" data-tauri-drag-region>Aura</span>
      </div>
      {/* M13.1：空白容器也标记 drag region——Tauri 只认 mousedown 目标
          元素自身的属性，flex:1 的 nav 空白此前吞掉了绝大部分拖动区域。 */}
      <nav className="titlebar-nav" aria-label="Primary" data-tauri-drag-region>
        {(['now-playing', 'library', 'map', 'dreamscape'] as const)
          .filter((v) => v !== 'dreamscape' || DREAMSCAPE_ENABLED)
          .map((v) => (
          <button
            key={v}
            className={`nav-btn${view === v ? ' active' : ''}`}
            onClick={() => setView(v)}
            aria-current={view === v ? 'page' : undefined}
          >
            {t(
              v === 'now-playing'
                ? 'nav.nowPlaying'
                : v === 'library'
                  ? 'nav.library'
                  : v === 'map'
                    ? 'nav.map'
                    : 'dreamscape.title',
            )}
          </button>
        ))}
      </nav>
      <div className="titlebar-actions" data-tauri-drag-region>
        <button className="icon-btn" onClick={() => setPaletteOpen(true)} aria-label="Ctrl+K" title="Ctrl+K">
          ⌘
        </button>
        <button className="icon-btn" onClick={() => setSettingsOpen(true)} aria-label={t('settings.title')} title={t('settings.title')}>
          ⚙
        </button>
        <div className="window-controls">
          <button className="win-btn" onClick={() => void win.minimize()} aria-label={t('title.minimize')}>
            —
          </button>
          <button className="win-btn" onClick={() => void win.toggleMaximize()} aria-label={t('title.maximize')}>
            ▢
          </button>
          <button className="win-btn close" onClick={() => void win.close()} aria-label={t('title.close')}>
            ✕
          </button>
        </div>
      </div>
    </header>
  );
}
