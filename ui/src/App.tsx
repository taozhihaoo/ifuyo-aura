// App shell（§6/§7/§16/§31）：titlebar + navigation + pages + hosts。
// 高频视觉走 Aurora/位置插值（§10/§14），不经 Zustand 全树渲染。
import { useEffect } from 'react';
import { FeatureBoundary } from './components/ErrorBoundary';
import { useTranslation } from './i18n/react';
import { useUiStore, usePlayerStore, useSettingsStore } from './state/stores';
import { ipc } from './ipc/client';
import { TitleBar } from './shell/TitleBar';
import { Sidebar } from './shell/Sidebar';
import { NowPlayingPage } from './features/nowplaying/NowPlayingPage';
import { LibraryPage } from './features/library/LibraryPage';
import { MapPage } from './features/map/MapPage';
import { DreamscapeView } from './features/dreamscape/DreamscapeView';
import { CommandPalette } from './features/palette/CommandPalette';
import { AuroraCanvas } from './visual/AuroraCanvas';
import { SettingsPanel } from './features/settings/SettingsPanel';
import { QueueDrawer } from './features/queue/QueueDrawer';
import { isMiniPlayer, MiniPlayer, useMiniPlayerOpener } from './windows/MiniPlayer';
import { translate } from './i18n';

export default function App() {
  const isMini = isMiniPlayer();
  const { t } = useTranslation();
  useMiniPlayerOpener();
  const view = useUiStore((s) => s.view);
  const setView = useUiStore((s) => s.setView);
  const setPaletteOpen = useUiStore((s) => s.setPaletteOpen);
  const notice = useUiStore((s) => s.notice);
  const setNotice = useUiStore((s) => s.setNotice);

  // 启动优先级（§155）：shell → settings → player snapshot → library。
  useEffect(() => {
    ipc
      .getSettings()
      .then((s) =>
        useSettingsStore.getState().setAll({
          locale: s.language === 'en' ? 'en' : 'zh-CN',
          uiScale: s.uiScale,
          reducedMotion: s.reducedMotion,
          loudnessEnabled: s.loudnessEnabled,
          crossfadeSeconds: s.crossfadeSeconds,
          eqEnabled: s.eqEnabled,
          eqPreset: s.eqPreset,
          micaEnabled: s.micaEnabled,
        }),
      )
      .catch(() => undefined);
    const pull = () =>
      ipc
        .playerSnapshot()
        .then((s) => usePlayerStore.getState().setSnapshot(s))
        .catch(() => undefined);
    pull();
    const timer = window.setInterval(pull, 250);
    return () => window.clearInterval(timer);
  }, []);

  // UI scale（§107）：root font-size 真实影响布局/字号/间距。
  const uiScale = useSettingsStore((s) => s.uiScale);
  useEffect(() => {
    document.documentElement.style.fontSize = `${16 * uiScale}px`;
  }, [uiScale]);

  // 全局快捷键（§103）：输入中不拦截 Space。
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const typing =
        target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setPaletteOpen(true);
        return;
      }
      if (typing) return;
      if (e.code === 'Space') {
        e.preventDefault();
        const snap = usePlayerStore.getState().snapshot;
        if (snap?.state === 'playing') ipc.pause().catch(() => undefined);
        else ipc.play().catch(() => undefined);
      }
      // §260：Alt+1/2/3 主导航（记录于 DECISIONS D-078）。
      if (e.altKey && ['1', '2', '3'].includes(e.key)) {
        e.preventDefault();
        const views = ['now-playing', 'library', 'map'] as const;
        const target = views[Number(e.key) - 1];
        if (target) setView(target);
        return;
      }
      // Ctrl+Q：queue 抽屉；Ctrl+M：mini 窗口（记录于 DECISIONS D-078；
      // mini 开启避免经 palette 输入——中文 IME 会拦截字母输入，§260）。
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'q') {
        e.preventDefault();
        useUiStore.getState().setQueueOpen(!useUiStore.getState().queueOpen);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'm') {
        e.preventDefault();
        useUiStore.getState().setMiniPlayerOpen(!useUiStore.getState().miniPlayerOpen);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        setView('library');
        window.setTimeout(() => {
          document.getElementById('library-search')?.focus();
        }, 50);
      }
      if (e.key === 'Escape') {
        // §183/§198：Esc 逐层退出（palette → dreamscape → 抽层）。
        const st = useUiStore.getState();
        if (st.commandPaletteOpen) {
          st.setPaletteOpen(false);
        } else if (st.view === 'dreamscape') {
          st.setView('now-playing'); // §285：明确退出方式
        } else {
          st.setQueueOpen(false);
          st.setSettingsOpen(false);
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [setPaletteOpen, setView]);

  if (isMini) {
    return <MiniPlayer />;
  }

  return (
    <div className="app-root" data-view={view}>
      <TitleBar />
      <div className="app-body">
        <Sidebar />
        <main className="app-main" role="main">
          {view === 'now-playing' && (
            <FeatureBoundary name="now-playing" message={t('error.boundaryNowPlaying')} retryLabel={t('common.retry')}>
              <NowPlayingPage />
            </FeatureBoundary>
          )}
          {view === 'library' && (
            <FeatureBoundary name="library" message={t('error.boundaryLibrary')} retryLabel={t('common.retry')}>
              <LibraryPage />
            </FeatureBoundary>
          )}
          {view === 'map' && (
            <FeatureBoundary name="map" message={t('error.boundaryMap')} retryLabel={t('common.retry')}>
              <MapPage />
            </FeatureBoundary>
          )}
          {view === 'dreamscape' && (
            <FeatureBoundary name="dreamscape" message={t('error.boundaryDreamscape')} retryLabel={t('common.retry')}>
              <DreamscapeView />
            </FeatureBoundary>
          )}
        </main>
        {/* §130/§131：Dreamscape 激活时不挂 Aurora（单高负载 WebGL 规则）。 */}
        {view !== 'dreamscape' && <AuroraCanvas />}
        {view === 'dreamscape' && <div className="aurora-fallback" aria-hidden />}
      </div>
      <CommandPalette />
      <SettingsPanel />
      <QueueDrawer />
      {notice && (
        <div className="notice-bar" role="alert">
          <span>{translate(useSettingsStore.getState().locale, notice)}</span>
          <button className="icon-btn" onClick={() => setNotice(null)} aria-label={t('common.ok')}>
            ✕
          </button>
        </div>
      )}
    </div>
  );
}
