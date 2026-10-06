// Artwork（§35–§37）：loading / loaded / missing / corrupt 四态；
// 字节经 IPC 命令读 M3，以 data URL 注入（任意本地路径不进 webview，§37）。
import { useEffect, useState } from 'react';
import { ipc } from '../ipc/client';
import { usePlayerStore } from '../state/stores';

export type ArtworkState =
  | { kind: 'loading' }
  | { kind: 'loaded'; dataUrl: string }
  | { kind: 'missing' }
  | { kind: 'corrupt' };

/** 当前曲目的 TrackId（snapshot.trackPath → trackIdByPath 归一化查询；
 *  PlayerBar 与 NowPlayingPage 共用）。 */
export function useCurrentTrackId(): number | null {
  const trackPath = usePlayerStore((s) => s.snapshot?.trackPath ?? null);
  const [trackId, setTrackId] = useState<number | null>(null);
  useEffect(() => {
    if (!trackPath) {
      setTrackId(null);
      return;
    }
    let alive = true;
    void ipc
      .trackIdByPath(trackPath)
      .then((id) => {
        if (alive) setTrackId(id);
      })
      .catch(() => {
        if (alive) setTrackId(null);
      });
    return () => {
      alive = false;
    };
  }, [trackPath]);
  return trackId;
}

export function useArtwork(trackId: number | null): ArtworkState {
  const [state, setState] = useState<ArtworkState>({ kind: 'loading' });

  useEffect(() => {
    if (trackId === null) {
      setState({ kind: 'missing' });
      return;
    }
    let alive = true;
    setState({ kind: 'loading' });
    void (async () => {
      try {
        const dataUrl = await ipc.trackArtwork(trackId);
        if (!alive) return;
        if (dataUrl === null) {
          setState({ kind: 'missing' });
          return;
        }
        // 数据 URL 自校验：图坏 → corrupt（§36）。
        const img = new Image();
        img.onload = () => {
          if (alive) setState({ kind: 'loaded', dataUrl });
        };
        img.onerror = () => {
          if (alive) setState({ kind: 'corrupt' });
        };
        img.src = dataUrl;
      } catch {
        if (alive) setState({ kind: 'corrupt' });
      }
    })();
    return () => {
      alive = false;
    };
  }, [trackId]);

  return state;
}
