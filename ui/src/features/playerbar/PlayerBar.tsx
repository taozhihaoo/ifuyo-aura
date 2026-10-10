// PlayerBar（M13.1）：常驻底部播放条——QQ 音乐式全局控制范式。
// 除"正在播放"页（已有全尺寸传输控件）外所有视图显示：
// 封面缩略 + 标题/状态（点击→正在播放页）+ 上一曲/播放/下一曲
// + 迷你进度条（rAF 本地插值，§14 同口径）+ 播放模式 + 音量。
import { useEffect, useRef, useState } from 'react';
import { toast, usePlayerStore, useUiStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';
import { useArtwork, useCurrentTrackId } from '../../components/Artwork';
import { clampNumber, fileBaseName } from '../../ui/format';
import { cyclePlayMode, playNext, playPrevious, usePlayMode, type PlayMode } from '../queue/QueueDrawer';

const MODE_ICON: Record<PlayMode, string> = {
  order: '→',
  repeatAll: '🔁',
  repeatOne: '🔂',
  shuffle: '🔀',
};

function fmt(sec: number): string {
  const total = Math.max(0, Math.floor(sec));
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
}

export function PlayerBar() {
  const snapshot = usePlayerStore((s) => s.snapshot);
  const setView = useUiStore((s) => s.setView);
  const desktopLyricsOpen = useUiStore((s) => s.desktopLyricsOpen);
  const setDesktopLyricsOpen = useUiStore((s) => s.setDesktopLyricsOpen);
  const { t } = useTranslation();
  const trackId = useCurrentTrackId();
  const artwork = useArtwork(trackId);
  const playMode = usePlayMode();
  const playing = snapshot?.state === 'playing';
  const duration = snapshot?.durationSeconds ?? null;
  const title =
    snapshot?.title?.trim() ||
    (snapshot?.trackPath ? fileBaseName(snapshot.trackPath) : null);

  // §14 同口径：快照基准 + rAF 本地插值（独立于 NowPlayingPage 的实例）。
  const [displayPos, setDisplayPos] = useState(0);
  const base = useRef({ at: 0, pos: 0, playing: false });
  useEffect(() => {
    if (!snapshot) return;
    base.current = {
      at: performance.now(),
      pos: snapshot.positionSeconds,
      playing: snapshot.state === 'playing',
    };
    setDisplayPos(snapshot.positionSeconds);
  }, [snapshot]);
  useEffect(() => {
    let raf = 0;
    const tick = () => {
      if (base.current.playing) {
        const delta = (performance.now() - base.current.at) / 1000;
        setDisplayPos(base.current.pos + delta);
      }
      raf = window.requestAnimationFrame(tick);
    };
    raf = window.requestAnimationFrame(tick);
    return () => window.cancelAnimationFrame(raf);
  }, []);

  const seek = (v: number) => {
    base.current = { at: performance.now(), pos: v, playing };
    setDisplayPos(v);
    ipc.seek(v).catch(() => undefined);
  };

  const cycleMode = () => {
    const next = cyclePlayMode();
    toast(t(`playMode.${next}`), 'success');
  };

  return (
    <footer className="player-bar" role="group" aria-label={t('controls.play')}>
      {/* 左：封面 + 标题（点击进正在播放页） */}
      <button
        className="pb-ident"
        onClick={() => setView('now-playing')}
        aria-label={t('nav.nowPlaying')}
        title={t('nav.nowPlaying')}
      >
        <span className="pb-art" aria-hidden>
          {artwork.kind === 'loaded' && <img className="pb-art-img" src={artwork.dataUrl} alt="" />}
          {artwork.kind === 'loading' && <span className="pb-art-glyph skeleton" aria-busy />}
          {(artwork.kind === 'missing' || artwork.kind === 'corrupt') && (
            <span className="pb-art-glyph">♪</span>
          )}
        </span>
        <span className="pb-meta">
          <span className="pb-title">{title ?? t('np.noTrack')}</span>
          <span className="pb-sub">
            {playing ? t('np.playingNow') : snapshot?.state === 'paused' ? t('np.pausedState') : ''}
          </span>
        </span>
      </button>

      {/* 中：传输键 + 迷你进度 */}
      <div className="pb-center">
        <div className="pb-controls">
          <button
            className="icon-btn"
            aria-label={t('controls.previous')}
            title={t('controls.previous')}
            onClick={() => void playPrevious().catch(() => undefined)}
          >
            ⏮
          </button>
          <button
            className={`ctl-btn primary pb-play ${playing ? 'glyph-pause' : 'glyph-play'}${playing ? ' pop' : ''}`}
            key={playing ? 'playing' : 'paused'}
            aria-label={playing ? t('controls.pause') : t('controls.play')}
            title={playing ? t('controls.pause') : t('controls.play')}
            onClick={() => (playing ? ipc.pause() : ipc.play()).catch(() => undefined)}
          />
          <button
            className="icon-btn"
            aria-label={t('controls.next')}
            title={t('controls.next')}
            onClick={() => void playNext().catch(() => undefined)}
          >
            ⏭
          </button>
        </div>
        <div className="pb-seek">
          <span className="time">{fmt(displayPos)}</span>
          <input
            className="seek-bar"
            type="range"
            min={0}
            max={duration ?? 0}
            step={0.5}
            value={Math.min(displayPos, duration ?? displayPos)}
            disabled={duration === null}
            onChange={(e) => seek(Number(e.target.value))}
            aria-label={t('controls.seek')}
          />
          <span className="time">{duration === null ? '--:--' : fmt(duration)}</span>
        </div>
      </div>

      {/* 右：桌面歌词开关 + 模式 + 音量 */}
      <div className="pb-right">
        <button
          className={`icon-btn pb-lyrics${desktopLyricsOpen ? ' active' : ''}`}
          aria-label={t('desktopLyrics.toggle')}
          title={t('desktopLyrics.toggle')}
          aria-pressed={desktopLyricsOpen}
          onClick={() => setDesktopLyricsOpen(!desktopLyricsOpen)}
        >
          {t('desktopLyrics.badge')}
        </button>
        <button
          className="icon-btn"
          aria-label={t(`playMode.${playMode}`)}
          title={t(`playMode.${playMode}`)}
          onClick={cycleMode}
        >
          {MODE_ICON[playMode]}
        </button>
        <label className="volume" title={t('controls.volume')}>
          <span aria-hidden>🔊</span>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            defaultValue={clampNumber(snapshot?.volume ?? 1, 0, 1)}
            onChange={(e) => ipc.setVolume(clampNumber(Number(e.target.value), 0, 1)).catch(() => undefined)}
            aria-label={t('controls.volume')}
          />
        </label>
      </div>
    </footer>
  );
}
