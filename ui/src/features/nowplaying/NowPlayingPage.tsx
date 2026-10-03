// Now Playing（§33–§45）：真实播放状态 + 位置插值（§14 低频快照 + rAF 补间）。
import { useEffect, useRef, useState } from 'react';
import { usePlayerStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';

function fmtTime(sec: number): string {
  if (!Number.isFinite(sec) || sec < 0) return '0:00';
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${String(s).padStart(2, '0')}`;
}

export function NowPlayingPage() {
  const snapshot = usePlayerStore((s) => s.snapshot);
  const { t } = useTranslation();
  const [displayPos, setDisplayPos] = useState(0);
  const base = useRef({ at: 0, pos: 0, playing: false });
  const playing = snapshot?.state === 'playing';
  const duration = snapshot?.durationSeconds ?? null;
  const seekable = snapshot?.seekable ?? false;

  // 快照到达：重置插值基准。
  useEffect(() => {
    if (!snapshot) return;
    base.current = { at: performance.now(), pos: snapshot.positionSeconds, playing: snapshot.state === 'playing' };
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

  return (
    <section className="now-playing" aria-label={t('nav.nowPlaying')}>
      <div className="np-artwork" role="img" aria-label="artwork">
        <div className="artwork-placeholder">♪</div>
      </div>
      <div className="np-info">
        <h1 className="np-title">{snapshot?.title ?? t('np.noTrack')}</h1>
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
      </div>
      <div className="np-controls" role="group" aria-label={t('controls.play')}>
        <button className="ctl-btn" aria-label={t('controls.previous')}>⏮</button>
        <button
          className="ctl-btn primary"
          aria-label={playing ? t('controls.pause') : t('controls.play')}
          onClick={() => (playing ? ipc.pause() : ipc.play()).catch(() => undefined)}
        >
          {playing ? '⏸' : '▶'}
        </button>
        <button className="ctl-btn" aria-label={t('controls.stop')} onClick={() => ipc.stop().catch(() => undefined)}>
          ⏹
        </button>
        <button className="ctl-btn" aria-label={t('controls.next')}>⏭</button>
        <label className="volume">
          <span aria-hidden>🔊</span>
          <input
            type="range"
            min={0}
            max={1}
            step={0.01}
            defaultValue={snapshot?.volume ?? 1}
            onChange={(e) => ipc.setVolume(Number(e.target.value)).catch(() => undefined)}
            aria-label={t('controls.volume')}
          />
        </label>
      </div>
    </section>
  );
}
