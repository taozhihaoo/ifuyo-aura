// Artwork（§35–§37）：loading / loaded / missing / corrupt 四态；
// 字节经 IPC 命令读 M3，以 data URL 注入（任意本地路径不进 webview，§37）。
import { useEffect, useState } from 'react';
import { ipc } from '../ipc/client';

export type ArtworkState =
  | { kind: 'loading' }
  | { kind: 'loaded'; dataUrl: string }
  | { kind: 'missing' }
  | { kind: 'corrupt' };

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
