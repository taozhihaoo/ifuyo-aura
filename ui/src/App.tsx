// App shell（§6/§7/§16/§31）：titlebar + navigation + pages + hosts。
// 高频视觉走 Aurora/位置插值（§10/§14），不经 Zustand 全树渲染。
import { useEffect } from 'react';
import { FeatureBoundary } from './components/ErrorBoundary';
import { useTranslation } from './i18n/react';
import { DREAMSCAPE_ENABLED, useUiStore, usePlayerStore, useSettingsStore } from './state/stores';
import { ipc } from './ipc/client';
import { updateService } from './features/update/UpdateService';
import { getAppInfo } from './ipc/appInfo';
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
  const noticeKind = useUiStore((s) => s.noticeKind);
  const noticeSeq = useUiStore((s) => s.noticeSeq);
  const setNotice = useUiStore((s) => s.setNotice);
  const locale = useSettingsStore((s) => s.locale);

  // 操作反馈自动消失：success/info 3.5s；error 停留待手动关闭。
  useEffect(() => {
    if (!notice || noticeKind === 'error') return;
    const timer = window.setTimeout(() => setNotice(null), 3500);
    return () => window.clearTimeout(timer);
  }, [notice, noticeKind, noticeSeq, setNotice]);

  // 启动优先级（§155）：shell → settings → player snapshot → library。
  useEffect(() => {
    void getAppInfo()
      .then((info) => {
        (globalThis as { __AURA_VERSION__?: string }).__AURA_VERSION__ = info.version;
      })
      .catch(() => undefined);
    ipc
      .getSettings()
      .then((s) =>
        useSettingsStore.getState().setAll({
          locale: s.language === 'en' ? 'en' : 'zh-CN',
          updatesAutoCheck: s.updatesAutoCheck,
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

  // M11 §8/§34：启动延迟自动检查更新（非阻塞；§35 无弹窗——状态仅在
  // 设置页呈现；§44 零遥测；失败只影响更新功能 §36）。
  useEffect(() => {
    const timer = window.setTimeout(() => {
      const st = useSettingsStore.getState();
      if (!st.updatesAutoCheck) return;
      void updateService
        .checkForUpdate()
        .then((state) => {
          if (state.kind === 'available') {
            useUiStore.getState().setNotice('update.availableNotice');
          }
        })
        .catch(() => undefined);
    }, 15_000);
    return () => window.clearTimeout(timer);
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
      // §260：Alt+1/2/3/4 主导航（记录于 DECISIONS D-078）。
      if (e.altKey && ['1', '2', '3', '4'].includes(e.key)) {
        e.preventDefault();
        const views = ['now-playing', 'library', 'map', 'dreamscape'] as const;
        const target = views[Number(e.key) - 1];
        if (target === 'dreamscape' && !DREAMSCAPE_ENABLED) return; // 功能停用
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

  // M10 §162–§166：托盘/系统集成的导航桥（窗口 focus 已由 Rust 侧完成；
  // 这里只切视图 / 打开 Mini——全部复用 M6 既有 entry point）。
  useEffect(() => {
    let un: (() => void) | null = null;
    void ipc
      .listenIntegrationNavigate((view) => {
        if (view === 'mini') {
          useUiStore.getState().setMiniPlayerOpen(true);
        } else if (view === 'dreamscape' ? DREAMSCAPE_ENABLED : ['now-playing', 'library', 'map'].includes(view)) {
          useUiStore.getState().setView(view as Parameters<typeof setView>[0]);
        }
      })
      .then((off) => {
        un = off;
      });
    return () => {
      un?.();
    };
  }, []);

  if (isMini) {
    return <MiniPlayer />;
  }

  return (
    <div className="app-root" data-view={view}>
      <TitleBar />
      <div className="app-body">
        <Sidebar />
        <main className="app-main" role="main">
          {/* key=view：切页重挂载触发内容淡入（M12.7 交互反馈） */}
          <div className="view-fade" key={view}>
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
            {view === 'dreamscape' && DREAMSCAPE_ENABLED && (
              <FeatureBoundary name="dreamscape" message={t('error.boundaryDreamscape')} retryLabel={t('common.retry')}>
                <DreamscapeView />
              </FeatureBoundary>
            )}
          </div>
        </main>
        {/* §130/§131：Dreamscape 激活时不挂 Aurora（单高负载 WebGL 规则）；
            Dreamscape 自带画布与 CSS fallback，无需占位背景。 */}
        {view !== 'dreamscape' && <AuroraCanvas />}
      </div>
      <CommandPalette />
      <SettingsPanel />
      <QueueDrawer />
      {notice && (
        <div
          key={noticeSeq}
          className={`notice-bar notice-${noticeKind}`}
          role={noticeKind === 'error' ? 'alert' : 'status'}
        >
          <span className="notice-icon" aria-hidden>
            {noticeKind === 'success' ? '✓' : noticeKind === 'error' ? '⚠' : 'ℹ'}
          </span>
          <span className="notice-text">{translate(locale, notice)}</span>
          <button className="icon-btn" onClick={() => setNotice(null)} aria-label={t('common.ok')}>
            ✕
          </button>
        </div>
      )}
    </div>
  );
}
