// 侧栏（§34 布局）：导航辅助 + 快捷入口；可折叠（UI state）。
import { useUiStore, useSettingsStore } from '../state/stores';
import { useTranslation } from '../i18n/react';

export function Sidebar() {
  const view = useUiStore((s) => s.view);
  const setView = useUiStore((s) => s.setView);
  const setQueueOpen = useUiStore((s) => s.setQueueOpen);
  const { t } = useTranslation(useSettingsStore((s) => s.locale));

  return (
    <aside className="sidebar">
      <button
        className={`side-btn${view === 'now-playing' ? ' active' : ''}`}
        onClick={() => setView('now-playing')}
      >
        ♪ {t('nav.nowPlaying')}
      </button>
      <button
        className={`side-btn${view === 'library' ? ' active' : ''}`}
        onClick={() => setView('library')}
      >
        ☰ {t('nav.library')}
      </button>
      <button className="side-btn" onClick={() => setQueueOpen(true)}>
        ≡ {t('queue.title')}
      </button>
    </aside>
  );
}
