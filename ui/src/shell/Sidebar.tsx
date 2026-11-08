// 侧栏（§34 布局）：导航辅助 + 快捷入口；可折叠（UI state）。
import { useUiStore, useSettingsStore } from '../state/stores';
import { useTranslation } from '../i18n/react';
import { Icon } from '../components/Icon';

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
        <Icon name="music" size={16} />
        <span>{t('nav.nowPlaying')}</span>
      </button>
      <button
        className={`side-btn${view === 'library' ? ' active' : ''}`}
        onClick={() => setView('library')}
      >
        <Icon name="library" size={16} />
        <span>{t('nav.library')}</span>
      </button>
      <button className="side-btn" onClick={() => setQueueOpen(true)}>
        <Icon name="queue" size={16} />
        <span>{t('queue.title')}</span>
      </button>
    </aside>
  );
}
