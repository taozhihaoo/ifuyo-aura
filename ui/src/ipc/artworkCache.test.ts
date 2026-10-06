// artworkCache 单测（M13.2）：并发去重 / 负缓存 / 容量淘汰 / 失效。
import { beforeEach, describe, expect, it, vi } from 'vitest';

const trackArtwork = vi.fn(async (id: number) => (id === 7 ? null : `data:${id}`));

vi.mock('../ipc/client', () => ({
  ipc: { trackArtwork: (id: number) => trackArtwork(id) },
}));

import { clearArtworkCache, loadArtwork, peekArtwork } from './artworkCache';

beforeEach(() => {
  clearArtworkCache();
  trackArtwork.mockClear();
});

describe('artworkCache（M13.2）', () => {
  it('并发去重：同 id 多次 load 只发一次 IPC', async () => {
    const [a, b, c] = await Promise.all([loadArtwork(1), loadArtwork(1), loadArtwork(1)]);
    expect(a).toBe('data:1');
    expect(b).toBe('data:1');
    expect(c).toBe('data:1');
    expect(trackArtwork).toHaveBeenCalledTimes(1);
  });

  it('负缓存：无封面(null)也缓存，不反复打空枪', async () => {
    expect(await loadArtwork(7)).toBeNull();
    expect(peekArtwork(7)).toBeNull();
    expect(await loadArtwork(7)).toBeNull();
    expect(trackArtwork).toHaveBeenCalledTimes(1);
  });

  it('peek：命中返回值，未命中 undefined', async () => {
    expect(peekArtwork(2)).toBeUndefined();
    await loadArtwork(2);
    expect(peekArtwork(2)).toBe('data:2');
  });

  it('容量上限：FIFO 淘汰最旧条目', async () => {
    // MAX_ENTRIES=512：写入 600 个，最早的一批应被淘汰
    for (let i = 0; i < 600; i += 1) {
      await loadArtwork(i);
    }
    expect(peekArtwork(0)).toBeUndefined();
    expect(peekArtwork(599)).toBe('data:599');
  });

  it('clear 全量失效', async () => {
    await loadArtwork(3);
    expect(peekArtwork(3)).toBe('data:3');
    clearArtworkCache();
    expect(peekArtwork(3)).toBeUndefined();
  });
});
