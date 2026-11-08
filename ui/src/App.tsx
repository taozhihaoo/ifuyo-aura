// App shell（§6/§7/§16/§31）：titlebar + navigation + pages + hosts。
// 高频视觉走 Aurora/位置插值（§10/§14），不经 Zustand 全树渲染。
import { useEffect } from 'react';
import { FeatureBoundary } from './components/ErrorBoundary';
import { Icon } from './components/Icon';
import { useTranslation } from './i18n/react';
import { DREAMSCAPE_ENABLED, useUiStore, usePlayerStore, useSettingsStore } from './state/stores';
import { ipc } from './ipc/client';
import { errorToI18nKey } from './ipc/errors';
import { STARTUP_CHECK_DELAY_MS, updateService } from './features/update/UpdateService';
import { getAppInfo } from './ipc/appInfo';
import { TitleBar } from './shell/TitleBar';
import { Sidebar } from './shell/Sidebar';
import { NowPlayingPage } from './features/nowplaying/NowPlayingPage';
import { LibraryPage } from './features/library/LibraryPage';
import { IfuyoPage } from './features/ifuyo/IfuyoPage';
import { MapPage } from './features/map/MapPage';
import { DreamscapeView } from './features/dreamscape/DreamscapeView';
import { CommandPalette } from './features/palette/CommandPalette';
import { AuroraCanvas } from './visual/AuroraCanvas';
import { SettingsPanel } from './features/settings/SettingsPanel';
import { QueueDrawer } from './features/queue/QueueDrawer';
import { PlayerBar } from './features/playerbar/PlayerBar';
import { isMiniPlayer, MiniPlayer, useMiniPlayerOpener } from './windows/MiniPlayer';
import { DesktopLyrics, isDesktopLyrics, useDesktopLyricsOpener } from './windows/DesktopLyrics';
import { translate } from './i18n';

/**
 * 每个 webview 是独立 JS realm，主窗与 Mini 各自需要一份播放快照。
 * 悬浮歌词窗自带 100ms 高精度轮询（含快照），不再叠加这条 250ms 轮询。
 * 独立成组件后，主窗专属副作用不会被 mini / 悬浮窗重复执行。
 */
function PlayerSnapshotPolling() {
  useEffect(() => {
    const pull = () => {
      void ipc
        .playerSnapshot()
        .then((s) => usePlayerStore.getState().setSnapshot(s))
        .catch(() => undefined);
    };
    pull();
    const timer = window.setInterval(pull, 250);
    return () => window.clearInterval(timer);
  }, []);
  return null;
}

export default function App() {
  // realm 分支在调用任何主窗 hooks 之前完成：此前 Mini / 悬浮窗也会跑
  // 设置读取、250ms 轮询、15s 更新检查、全局快捷键与导航桥，导致
  // 多窗口重复 IPC、状态被无关 realm 改写。
  if (isMiniPlayer()) {
    return (
      <>
        <PlayerSnapshotPolling />
        <MiniPlayer />
      </>
    );
  }
  if (isDesktopLyrics()) return <DesktopLyrics />;
  return <MainWindow />;
}

function MainWindow() {
  const { t } = useTranslation();
  useMiniPlayerOpener();
  useDesktopLyricsOpener();
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

  // 启动优先级（§155）：shell → settings（快照轮询见 PlayerSnapshotPolling）。
  useEffect(() => {
    void getAppInfo()
      .then((info) => {
        (globalThis as { __AURA_VERSION__?: string }).__AURA_VERSION__ = info.version;
      })
      .catch(() => undefined);
    void ipc
      .getSettings()
      .then((s) =>
        useSettingsStore.getState().setAll({
          locale: s.language === 'en' ? 'en' : 'zh-CN',
          updatesAutoCheck: s.updatesAutoCheck,
          mojibakeFix: s.mojibakeFix,
          analysisAuto: s.analysisAuto,
          uiScale: s.uiScale,
          reducedMotion: s.reducedMotion,
          loudnessEnabled: s.loudnessEnabled,
          crossfadeSeconds: s.crossfadeSeconds,
          eqEnabled: s.eqEnabled,
          eqPreset: s.eqPreset,
          micaEnabled: s.micaEnabled,
        }),
      )
      .catch((e) => {
        // 设置读取失败绝不能静默：否则整包偏好变成 undefined（uiScale
        // 变 NaNpx、自动更新永不触发）而用户毫不知情。
        useUiStore.getState().setNotice(errorToI18nKey(e), 'error');
      });
  }, []);

  // M11 §8/§34：启动延迟自动检查更新（非阻塞；§35 无弹窗；§44 零遥测）。
  // 24h 最小间隔由 shouldAutoCheck 把关——此前策略写了却没接线，
  // 每次启动都发请求，与「每周最多一次」的文案不符。
  useEffect(() => {
    const timer = window.setTimeout(() => {
      if (!updateService.shouldAutoCheck()) return;
      void updateService
        .checkForUpdate()
        .then((state) => {
          if (state.kind === 'available') {
            useUiStore.getState().setNotice('update.availableNotice');
          }
        })
        .catch(() => undefined);
    }, STARTUP_CHECK_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, []);

  // UI scale（§107）：root font-size 真实影响布局/字号/间距。
  const uiScale = useSettingsStore((s) => s.uiScale);
  useEffect(() => {
    const scale = Number.isFinite(uiScale) && uiScale > 0 ? uiScale : 1;
    document.documentElement.style.fontSize = `${16 * scale}px`;
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
      // §183/§198：Esc 逐层退出（palette → dreamscape → 抽层）。
      // 必须在 `typing` 之前处理，否则焦点在搜索框时按 Esc 关不掉抽屉。
      if (e.key === 'Escape') {
        const st = useUiStore.getState();
        if (st.commandPaletteOpen) {
          st.setPaletteOpen(false);
        } else if (st.view === 'dreamscape') {
          st.setView('now-playing');
        } else {
          st.setQueueOpen(false);
          st.setSettingsOpen(false);
        }
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
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [setPaletteOpen, setView]);

  // 桌面悬浮歌词是独立 realm：它自己关闭时主窗的开关不会变，
  // 靠这条跨窗事件把主窗按钮状态同步回来。
  useEffect(() => {
    let cancelled = false;
    let off: (() => void) | null = null;
    const internals = (globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    // 非 Tauri 环境（jsdom / 浏览器预览）没有 IPC 桥，listen 会抛出
    // "transformCallback is not a function" 并变成未处理 rejection。
    if (internals) {
      void import('@tauri-apps/api/event')
        .then(({ listen }) =>
          listen('aura://lyrics-closed', () =>
            useUiStore.getState().setDesktopLyricsOpen(false),
          ),
        )
        .then((fn) => {
          if (cancelled) fn();
          else off = fn;
        })
        .catch(() => undefined);
    }
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  // M10 §162–§166：托盘/系统集成的导航桥。
  useEffect(() => {
    // `let un = null; .then(off => un = off)` 在 StrictMode 下首次 cleanup
    // 早于 Promise resolve，监听永远无法注销 → 双注册。
    let cancelled = false;
    let off: (() => void) | null = null;
    void ipc
      .listenIntegrationNavigate((view) => {
        if (view === 'mini') {
          useUiStore.getState().setMiniPlayerOpen(true);
        } else if (view === 'dreamscape' ? DREAMSCAPE_ENABLED : ['now-playing', 'library', 'ifuyo', 'map'].includes(view)) {
          useUiStore.getState().setView(view as Parameters<typeof setView>[0]);
        }
      })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        off = fn;
      });
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  return (
    <div className="app-root" data-view={view}>
      <PlayerSnapshotPolling />
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
            {view === 'ifuyo' && (
              <FeatureBoundary name="ifuyo" message={t('error.boundaryIfuyo')} retryLabel={t('common.retry')}>
                <IfuyoPage />
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
      {/* M13.1：常驻播放条（正在播放页已有全尺寸控件，不重复显示） */}
      {view !== 'now-playing' && <PlayerBar />}
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
            <Icon name="close" size={14} />
          </button>
        </div>
      )}
    </div>
  );
}
