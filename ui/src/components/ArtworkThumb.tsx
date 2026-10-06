// ArtworkThumb（M13.2）：曲库 32px 封面缩略图。
// 仅 hasArtwork=true 的行发 IPC；走 artworkCache（并发去重+负缓存），
// 未加载完显示 ▣ 骨架，无封面显示通用音符图标。
import { useEffect, useState } from 'react';
import { loadArtwork, peekArtwork } from '../ipc/artworkCache';

export function ArtworkThumb({ trackId, hasArtwork }: { trackId: number; hasArtwork: boolean }) {
  const [src, setSrc] = useState<string | null | undefined>(() => peekArtwork(trackId));

  useEffect(() => {
    setSrc(peekArtwork(trackId));
    if (!hasArtwork) return;
    if (peekArtwork(trackId) !== undefined) return;
    let alive = true;
    void loadArtwork(trackId).then((url) => {
      if (alive) setSrc(url);
    });
    return () => {
      alive = false;
    };
  }, [trackId, hasArtwork]);

  if (hasArtwork && src) {
    return <img className="tr-art-img" src={src} alt="" loading="lazy" />;
  }
  return <span className="art-glyph">{hasArtwork ? '▣' : '♪'}</span>;
}
