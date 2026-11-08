// Mini Player（§109–§113）：独立 frameless always-on-top 窗口；
// 与主窗口共享同一 Rust 应用层（§157 单一 audio engine）+ 同一 dist。
import { useEffect } from 'react';
import { useUiStore } from '../state/stores';
import { usePlayerStore } from '../state/stores';
import { ipc } from '../ipc/client';
import { useTranslation } from '../i18n/react';
import { Icon } from '../components/Icon';

export function isMiniPlayer(): boolean {
  return new URLSearchParams(window.location.search).get('view') === 'mini';
}

/** 首次挂载时 miniPlayerOpen=false 不应去 hide 一个还不存在的窗口；
 *  一旦窗口被创建过，关闭开关就必须真的隐藏它（Ctrl+M / 命令面板路径）。 */
let windowSeen = false;

export function useMiniPlayerOpener() {
  const miniPlayerOpen = useUiStore((s) => s.miniPlayerOpen);
  const setMiniPlayerOpen = useUiStore((s) => s.setMiniPlayerOpen);
  useEffect(() => {
    if (!miniPlayerOpen && !windowSeen) return;
    let cancelled = false;
    void import('@tauri-apps/api/webviewWindow').then(async ({ WebviewWindow }) => {
      const existing = await WebviewWindow.getByLabel('mini');
      if (cancelled) return;
      windowSeen = true;
      if (existing) {
        if (miniPlayerOpen) await existing.show().catch(() => undefined);
        else await existing.hide().catch(() => undefined);
        return;
      }
      if (!miniPlayerOpen) return;
      const mini = new WebviewWindow('mini', {
        url: 'index.html?view=mini',
        title: 'ifuyo Aura — Mini',
        width: 340,
        height: 110,
        resizable: false,
        alwaysOnTop: true,
        decorations: false,
      });
      mini.once('tauri://created', () => undefined);
      mini.once('tauri://error', () => setMiniPlayerOpen(false));
    });
    return () => {
      cancelled = true;
    };
  }, [miniPlayerOpen, setMiniPlayerOpen]);
}

/** 队列真相在主窗（每个 webview 是独立 JS realm），next/previous 经
 *  事件桥交回主窗处理——与 SMTC/托盘同一条路径（D-123）。 */
function requestTransport(cmd: 'next' | 'previous'): void {
  void import('@tauri-apps/api/event').then(({ emitTo }) => {
    void emitTo('main', 'integration://command', { cmd }).catch(() => undefined);
  });
}

export function MiniPlayer() {
  const snapshot = usePlayerStore((s) => s.snapshot);
  const { t } = useTranslation();
  const playing = snapshot?.state === 'playing';

  useEffect(() => {
    // 关闭 mini = 隐藏窗口，不停止播放（§113）。保存 unlisten 并在
    // cleanup 注销，避免 StrictMode 双挂载残留两个处理器。
    let cancelled = false;
    let off: (() => void) | null = null;
    void import('@tauri-apps/api/webviewWindow').then(async ({ getCurrentWebviewWindow }) => {
      const fn = await getCurrentWebviewWindow().onCloseRequested((e) => {
        e.preventDefault();
        void getCurrentWebviewWindow().hide();
      });
      if (cancelled) fn();
      else off = fn;
    });
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  return (
    <div className="mini-player" data-tauri-drag-region>
      <div className="mini-artwork" aria-hidden>
        <Icon name="music" size={22} />
      </div>
      <div className="mini-info">
        <div className="mini-title" title={snapshot?.trackPath ?? undefined}>
          {snapshot?.title ?? t('np.noTrack')}
        </div>
        <div className="mini-state">{playing ? t('np.playingNow') : t('np.pausedState')}</div>
      </div>
      <div className="mini-controls">
        <button
          type="button"
          className="mini-btn"
          aria-label={t('controls.previous')}
          onClick={() => requestTransport('previous')}
        >
          <Icon name="previous" size={15} />
        </button>
        <button
          type="button"
          className="mini-btn mini-btn-primary"
          aria-label={playing ? t('controls.pause') : t('controls.play')}
          onClick={() => (playing ? ipc.pause() : ipc.play()).catch(() => undefined)}
        >
          <Icon name={playing ? 'pause' : 'play'} size={15} />
        </button>
        <button
          type="button"
          className="mini-btn"
          aria-label={t('controls.next')}
          onClick={() => requestTransport('next')}
        >
          <Icon name="next" size={15} />
        </button>
      </div>
    </div>
  );
}
