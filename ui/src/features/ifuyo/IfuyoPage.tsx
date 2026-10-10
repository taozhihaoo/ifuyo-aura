// ifuyo 远程曲库（自托管音乐清单页）。
//
// 清单由 Rust 侧拉取与校验（应用 CSP 是 `connect-src ipc:`，WebView 不能直接
// fetch 远程地址），本页只消费已校验的 DTO。播放走 `ifuyo_play`：服务器给出的
// URL 不享受 "Open URL" 的私网确认待遇（D-070），一律 deny_private 策略。
// 本地缓存优先播放属下一批，本页不假装已支持。
import { useCallback, useEffect, useState } from 'react';
import { ipc } from '../../ipc/client';
import { errorToI18nKey } from '../../ipc/errors';
import { fmtDuration, type IfuyoList, type IfuyoTrack } from '../../ipc/ifuyo';
import { useTranslation } from '../../i18n/react';
import { usePlayerStore, useUiStore } from '../../state/stores';

type ListState = { kind: 'loading' } | { kind: 'error'; key: string } | { kind: 'ready'; data: IfuyoList };

export function IfuyoPage() {
  const { t } = useTranslation();
  const [state, setState] = useState<ListState>({ kind: 'loading' });
  const [busyUrl, setBusyUrl] = useState<string | null>(null);
  const setNotice = useUiStore((s) => s.setNotice);
  const snapshot = usePlayerStore((s) => s.snapshot);

  const load = useCallback(async () => {
    setState({ kind: 'loading' });
    try {
      setState({ kind: 'ready', data: await ipc.ifuyoList() });
    } catch (e) {
      setState({ kind: 'error', key: errorToI18nKey(e) });
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const play = async (track: IfuyoTrack) => {
    setBusyUrl(track.url);
    try {
      await ipc.ifuyoPlay(track.url, track.title, track.artist);
    } catch (e) {
      // 远程失败原因多样（网络/解码/被拦截），如实提示而不是静默不变。
      setNotice(t(errorToI18nKey(e)));
    } finally {
      setBusyUrl(null);
    }
  };

  if (state.kind === 'loading') {
    return (
      <section className="page ifuyo-page" aria-busy="true">
        <h1 className="page-title">{t('ifuyo.title')}</h1>
        <p className="ifuyo-status">{t('ifuyo.loading')}</p>
      </section>
    );
  }

  if (state.kind === 'error') {
    return (
      <section className="page ifuyo-page">
        <h1 className="page-title">{t('ifuyo.title')}</h1>
        <p className="ifuyo-status" role="alert">
          {t(state.key)}
        </p>
        <button type="button" className="btn" onClick={() => void load()}>
          {t('ifuyo.retry')}
        </button>
      </section>
    );
  }

  const { data } = state;
  const nowPath = snapshot?.trackPath ?? null;

  return (
    <section className="page ifuyo-page">
      <header className="ifuyo-head">
        <h1 className="page-title">{t('ifuyo.title')}</h1>
        <button type="button" className="btn" onClick={() => void load()}>
          {t('ifuyo.refresh')}
        </button>
      </header>

      {data.tracks.length === 0 ? (
        <p className="ifuyo-status">{t('ifuyo.empty')}</p>
      ) : (
        <>
          <ul className="ifuyo-list">
            {data.tracks.map((track) => {
              const isCurrent =
                nowPath !== null &&
                decodeURIComponent(nowPath.split(/[\\/]/).pop() ?? '') ===
                  decodeURIComponent(track.url.split('/').pop() ?? '');
              return (
                <li key={`${track.id}-${track.url}`}>
                  <button
                    type="button"
                    className={`ifuyo-row${isCurrent ? ' playing' : ''}`}
                    onClick={() => void play(track)}
                    disabled={busyUrl !== null}
                  >
                    <span className="ifuyo-title">{track.title}</span>
                    <span className="ifuyo-artist">{track.artist ?? ''}</span>
                    <span className="ifuyo-time">{fmtDuration(track.durationSeconds)}</span>
                  </button>
                </li>
              );
            })}
          </ul>
          {/* 不完整要看得见：被安全校验丢掉的条数与截断标志都如实上报。 */}
          {(data.skipped > 0 || data.truncated) && (
            <p className="ifuyo-note">
              {data.skipped > 0 ? t('ifuyo.skipped', { count: data.skipped }) : ''}
              {data.truncated ? ` ${t('ifuyo.truncated')}` : ''}
            </p>
          )}
        </>
      )}
    </section>
  );
}
