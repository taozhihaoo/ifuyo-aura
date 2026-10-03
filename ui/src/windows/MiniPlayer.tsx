// Mini Player（§109–§113）：独立 frameless always-on-top 窗口；
// 与主窗口共享同一 Rust 应用层（§157 单一 audio engine）+ 同一 dist。
import { useEffect } from 'react';
import { useUiStore } from '../state/stores';
import { usePlayerStore } from '../state/stores';
import { ipc } from '../ipc/client';
import { useTranslation } from '../i18n/react';

export function isMiniPlayer(): boolean {
  return new URLSearchParams(window.location.search).get('view') === 'mini';
}

export function useMiniPlayerOpener() {
  const miniPlayerOpen = useUiStore((s) => s.miniPlayerOpen);
  const setMiniPlayerOpen = useUiStore((s) => s.setMiniPlayerOpen);
  useEffect(() => {
    if (!miniPlayerOpen) return;
    void import('@tauri-apps/api/webviewWindow').then(async ({ WebviewWindow }) => {
      const existing = await WebviewWindow.getByLabel('mini');
      if (existing) {
        await existing.show();
        return;
      }
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
  }, [miniPlayerOpen, setMiniPlayerOpen]);
}

export function MiniPlayer() {
  const snapshot = usePlayerStore((s) => s.snapshot);
  const { t } = useTranslation();
  const playing = snapshot?.state === 'playing';

  useEffect(() => {
    // 关闭 mini = 隐藏窗口，不停止播放（§113）。
    const win = (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    void win;
    void import('@tauri-apps/api/webviewWindow').then(({ getCurrentWebviewWindow }) => {
      void getCurrentWebviewWindow().onCloseRequested((e) => {
        e.preventDefault();
        void getCurrentWebviewWindow().hide();
      });
    });
  }, []);

  return (
    <div className="mini-player" data-tauri-drag-region>
      <div className="mini-artwork" aria-hidden>♪</div>
      <div className="mini-info">
        <div className="mini-title">{snapshot?.title ?? t('np.noTrack')}</div>
        <div className="mini-state">{playing ? '▶' : '⏸'}</div>
      </div>
      <div className="mini-controls">
        <button className="mini-btn" aria-label={t('controls.previous')}>⏮</button>
        <button
          className="mini-btn"
          aria-label={playing ? t('controls.pause') : t('controls.play')}
          onClick={() => (playing ? ipc.pause() : ipc.play()).catch(() => undefined)}
        >
          {playing ? '⏸' : '▶'}
        </button>
        <button className="mini-btn" aria-label={t('controls.next')}>⏭</button>
      </div>
    </div>
  );
}
