// 歌词来源解析：本地曲目走 trackPath→TrackId→lyrics_for_track；远程曲目
// （snapshot.trackPath 就是音频 URL）走清单里登记的 lyricsUrl→ifuyo_lyrics。
//
// 为什么需要登记表：播放器快照只有 trackPath，没有"这首歌的歌词在哪"。列表页
// 拿到清单时把 audioUrl→lyricsUrl 记进来，两个歌词面（梦境舞台 / 桌面悬浮）
// 共用同一个解析器，避免各写一份分支逻辑（历史上远程曲目永远显示"没有歌词"
// 就是因为这条链在 trackIdByPath 上断了）。
import { ipc } from './client';
import type { LyricsDto } from './client';

const lyricsUrlByAudioUrl = new Map<string, string>();

export function isRemotePath(path: string): boolean {
  return /^https?:\/\//i.test(path);
}

/** 列表页拉到清单后登记；重复登记覆盖（清单可能已更新）。 */
export function registerRemoteLyrics(
  tracks: Array<{ url: string; lyricsUrl: string | null }>,
): void {
  for (const t of tracks) {
    if (t.lyricsUrl !== null && t.lyricsUrl !== '') {
      lyricsUrlByAudioUrl.set(t.url, t.lyricsUrl);
    } else {
      lyricsUrlByAudioUrl.delete(t.url);
    }
  }
}

/** 仅测试用：清掉登记，避免用例间串味。 */
export function __resetRemoteLyrics(): void {
  lyricsUrlByAudioUrl.clear();
}

/** 按曲目路径取歌词；无歌词或取不到都返回 null（调用方负责如实提示）。 */
export async function lyricsForPath(path: string | null): Promise<LyricsDto | null> {
  if (path === null || path === '') return null;
  if (isRemotePath(path)) {
    const lyricsUrl = lyricsUrlByAudioUrl.get(path);
    if (lyricsUrl === undefined) return null;
    return await ipc.ifuyoLyrics(lyricsUrl);
  }
  const trackId = await ipc.trackIdByPath(path);
  if (trackId === null) return null;
  return await ipc.lyricsForTrack(trackId);
}
