// 桌面悬浮歌词（独立置顶透明窗）。与主窗是不同 JS realm，数据一律经 IPC
// 自取（与 Mini Player 同形态）；歌词文本命令式更新，不进 React 状态。
//
// 锁定语义 = 只锁"能不能拖动"。点击穿透依赖分层窗口的逐像素 alpha 命中
// 测试（透明区域天然穿透），因此**不**使用 setIgnoreCursorEvents——那会让
// 锁定态下的解锁按钮也点不到（正是要避免的）。
import { useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ipc } from '../ipc/client';
import type { LyricsDto } from '../ipc/client';
import { useSettingsStore, useUiStore } from '../state/stores';
import { useTranslation } from '../i18n/react';

const LABEL = 'desktop-lyrics';

/** 本会话是否已创建/见过该窗口——未见过时关闭态完全不碰窗口 API
 *  （jsdom 与首帧都不该有跨进程调用）。 */
let windowSeen = false;

export function isDesktopLyrics(): boolean {
  return new URLSearchParams(window.location.search).get('view') === 'lyrics';
}

/** 主窗侧：开/关悬浮歌词窗（关闭 = 隐藏，不停止播放）。 */
export function useDesktopLyricsOpener(): void {
  const open = useUiStore((s) => s.desktopLyricsOpen);
  const setOpen = useUiStore((s) => s.setDesktopLyricsOpen);
  useEffect(() => {
    if (!open && !windowSeen) return;
    let cancelled = false;
    void import('@tauri-apps/api/webviewWindow').then(async ({ WebviewWindow }) => {
      const existing = await WebviewWindow.getByLabel(LABEL);
      if (cancelled) return;
      windowSeen = true;
      if (existing) {
        if (open) await existing.show().catch(() => undefined);
        else await existing.hide().catch(() => undefined);
        return;
      }
      if (!open) return;
      const w = new WebviewWindow(LABEL, {
        url: 'index.html?view=lyrics',
        title: 'ifuyo Aura',
        width: 680,
        height: 148,
        transparent: true,
        decorations: false,
        alwaysOnTop: true,
        skipTaskbar: true,
        resizable: false,
        shadow: false,
        focus: false,
      });
      windowSeen = true;
      w.once('tauri://error', () => setOpen(false));
    });
    return () => {
      cancelled = true;
    };
  }, [open, setOpen]);
}

export function DesktopLyrics() {
  const locale = useSettingsStore((s) => s.locale);
  const { t } = useTranslation(locale);
  const [locked, setLocked] = useState(false);
  const nowRef = useRef<HTMLParagraphElement>(null);
  const nextRef = useRef<HTMLParagraphElement>(null);
  const lyricsRef = useRef<LyricsDto | null>(null);
  const idxRef = useRef(-1);
  const pathRef = useRef<string | null>(null);
  const lockedRef = useRef(locked);
  lockedRef.current = locked;

  // 透明底：整页背景必须真的透明，分层窗口的逐像素命中才会让空白处穿透。
  useEffect(() => {
    const root = document.documentElement;
    root.classList.add('desktop-lyrics-root');
    return () => {
      root.classList.remove('desktop-lyrics-root');
    };
  }, []);

  // 关闭请求 = 隐藏（与 Mini Player 同语义；主窗按钮状态经事件同步）。
  useEffect(() => {
    void getCurrentWindow().onCloseRequested((e) => {
      e.preventDefault();
      void getCurrentWindow().hide();
      useUiStore.getState().setDesktopLyricsOpen(false);
    });
    return () => undefined;
  }, []);

  // 数据轮询：位置 100ms（歌词换行要跟上），快照 500ms（换曲检测）。
  useEffect(() => {
    let alive = true;
    let ticks = 0;
    let position = 0;
    let title = '';

    const paint = () => {
      const now = nowRef.current;
      const next = nextRef.current;
      if (!now || !next) return;
      const lines = lyricsRef.current?.lines ?? [];
      if (lines.length === 0 || !lyricsRef.current?.synced) {
        // 无同步歌词：主行显示曲名（悬浮窗上留白只会让人以为坏了）。
        if (idxRef.current !== -2) {
          now.textContent = title || t('np.noTrack');
          next.textContent = '';
          idxRef.current = -2;
        }
        return;
      }
      let idx = -1;
      for (let i = 0; i < lines.length; i++) {
        const ts = lines[i]?.timeSeconds;
        if (typeof ts === 'number' && ts <= position) idx = i;
        else break;
      }
      if (idx === idxRef.current) return;
      idxRef.current = idx;
      const safe = Math.max(idx, 0);
      now.textContent = lines[safe]?.text ?? '';
      next.textContent = lines[safe + 1]?.text ?? '';
      now.classList.remove('dl-enter');
      void now.offsetWidth; // 重启动画
      now.classList.add('dl-enter');
    };

    const timer = window.setInterval(async () => {
      if (!alive) return;
      ticks += 1;
      try {
        if (ticks % 5 === 0) {
          const s = await ipc.playerSnapshot();
          if (!alive) return;
          const p = s.trackPath ?? null;
          title = s.title ?? '';
          if (p !== pathRef.current) {
            pathRef.current = p;
            idxRef.current = -1;
            lyricsRef.current = null;
            if (p !== null) {
              const id = await ipc.trackIdByPath(p);
              lyricsRef.current = id === null ? null : await ipc.lyricsForTrack(id);
            }
          }
          if (s.state !== 'playing') position = s.positionSeconds;
        } else {
          position = await ipc.playerPosition();
        }
      } catch {
        // IPC 抖动不打断悬浮窗：保留上一帧文本。
      }
      paint();
    }, 100);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
    // t 随 locale 稳定；轮询只读 ref
  }, [t]);

  return (
    <div
      className="desktop-lyrics"
      data-locked={locked ? 'true' : 'false'}
      {...(locked ? {} : { 'data-tauri-drag-region': true })}
    >
      <p className="dl-now" ref={nowRef} aria-live="polite" />
      <p className="dl-next" ref={nextRef} aria-hidden />
      <button
        type="button"
        className="dl-pill"
        aria-label={locked ? t('desktopLyrics.unlock') : t('desktopLyrics.lock')}
        title={locked ? t('desktopLyrics.unlock') : t('desktopLyrics.lock')}
        aria-pressed={locked}
        onClick={() => setLocked((v) => !v)}
      >
        {/* VS15 强制单色文本呈现：与 ⏮/⏭ 同一 glyph 体系，不引入彩底 emoji。 */}
        {locked ? '🔒︎' : '🔓︎'}
      </button>
    </div>
  );
}
