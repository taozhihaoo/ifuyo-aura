// Now Playing（§33–§45）：真实播放状态 + 位置插值（§14 低频快照 + rAF 补间）
// + 真实 artwork 四态（§35–§37，经 IPC data URL）。
import { useEffect, useRef, useState } from 'react';
import { usePlayerStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';
import { useArtwork, useCurrentTrackId } from '../../components/Artwork';
import { formatDuration, safeProgress, clampNumber, fileBaseName } from '../../ui/format';
import {
  cyclePlayMode,
  playNext,
  playPrevious,
  usePlayMode,
  type PlayMode,
} from '../queue/QueueDrawer';
import { toast } from '../../state/stores';

const MODE_ICON: Record<PlayMode, string> = {
  order: '→',
  repeatAll: '🔁',
  repeatOne: '🔂',
  shuffle: '🔀',
};

function fmtTime(sec: number | null | undefined): string {
  return formatDuration(sec) ?? '0:00';
}

export function NowPlayingPage() {
  const snapshot = usePlayerStore((s) => s.snapshot);
  const { t } = useTranslation();
  const playMode = usePlayMode();
  const [displayPos, setDisplayPos] = useState(0);
  const base = useRef({ at: 0, pos: 0, playing: false });
  const playing = snapshot?.state === 'playing';
  const duration = snapshot?.durationSeconds ?? null;
  const seekable = snapshot?.seekable ?? false;

  // 快照到达：重置插值基准。
  useEffect(() => {
    if (!snapshot) return;
    base.current = {
      at: performance.now(),
      pos: snapshot.positionSeconds,
      playing: snapshot.state === 'playing',
    };
    setDisplayPos(snapshot.positionSeconds);
  }, [snapshot]);

  // §14：rAF 本地插值，不推 Zustand。
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

  const onSeek = (e: React.ChangeEvent<HTMLInputElement>) => {
    const v = Number(e.target.value);
    base.current = { at: performance.now(), pos: v, playing };
    setDisplayPos(v);
    ipc.seek(v).catch(() => undefined);
  };

  // §35–§37：当前 track 的真实 artwork（path → id → data URL；M13.1 提取共用）。
  const trackId = useCurrentTrackId();
  const artwork = useArtwork(trackId);

  return (
    <section className="now-playing" aria-label={t('nav.nowPlaying')}>
      {artwork.kind === 'loaded' && (
        <div className="np-ambient" aria-hidden>
          <img src={artwork.dataUrl} alt="" />
        </div>
      )}
      <div className="np-artwork" role="img" aria-label="artwork">
        {artwork.kind === 'loading' && <div className="artwork-loading skeleton" aria-busy />}
        {artwork.kind === 'loaded' && <img className="artwork-img" src={artwork.dataUrl} alt="" />}
        {(artwork.kind === 'missing' || artwork.kind === 'corrupt') && (
          <div className="artwork-placeholder">♪</div>
        )}
      </div>
      <div className="np-info">
        <h1 className="np-title">
          {snapshot?.title?.trim() || (snapshot?.trackPath ? fileBaseName(snapshot.trackPath) : null) || t('np.noTrack')}
        </h1>
        {!snapshot?.title && <p className="np-hint">{t('np.noTrackHint')}</p>}
      </div>
      <div className="np-progress">
        <span className="time">{fmtTime(displayPos)}</span>
        <input
          className="seek-bar"
          type="range"
          min={0}
          max={duration ?? 0}
          step={0.5}
          value={Math.min(displayPos, duration ?? displayPos)}
          disabled={!seekable || duration === null}
          onChange={onSeek}
          aria-label={t('controls.seek')}
        />
        <span className="time">{duration === null ? t('np.unknownDuration') : fmtTime(duration)}</span>
        <span className="visually-hidden" aria-live="polite">
          {`${Math.round(safeProgress(displayPos, duration) * 100)}%`}
        </span>
      </div>
      <div className="np-controls" role="group" aria-label={t('controls.play')}>
        <button
          className="ctl-btn"
          aria-label={t(`playMode.${playMode}`)}
          title={t(`playMode.${playMode}`)}
          onClick={() => {
            const next = cyclePlayMode();
            toast(t(`playMode.${next}`), 'success');
          }}
        >
          {MODE_ICON[playMode]}
        </button>
        <button
          className="ctl-btn"
          aria-label={t('controls.previous')}
          title={t('controls.previous')}
          onClick={() => void playPrevious().catch(() => undefined)}
        >
          ⏮
        </button>
        <button
          className={`ctl-btn primary ${playing ? 'glyph-pause' : 'glyph-play'}${playing ? ' pop' : ''}`}
          key={playing ? 'playing' : 'paused'}
          aria-label={playing ? t('controls.pause') : t('controls.play')}
          title={playing ? t('controls.pause') : t('controls.play')}
          onClick={() =>
            (playing ? ipc.pause() : ipc.play()).catch(() => undefined)
          }
        />
        <button
          className="ctl-btn"
          aria-label={t('controls.stop')}
          title={t('controls.stop')}
          onClick={() => ipc.stop().catch(() => undefined)}
        >
          ⏹
        </button>
        <button
          className="ctl-btn"
          aria-label={t('controls.next')}
          title={t('controls.next')}
          onClick={() => void playNext().catch(() => undefined)}
        >
          ⏭
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
    </section>
  );
}
