// 远程/本地歌词解析器：两个歌词面共用的那条链（此前远程曲目在这里断掉，
// 表现为"在线歌永远没词"）。
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  __resetRemoteLyrics,
  isRemotePath,
  lyricsForPath,
  registerRemoteLyrics,
} from './remoteLyrics';

const calls = vi.hoisted(() => ({
  ifuyoLyrics: vi.fn(async () => ({ synced: true, lines: [{ timeSeconds: 0.5, text: '第一句' }] })),
  trackIdByPath: vi.fn(async (): Promise<number | null> => 7),
  lyricsForTrack: vi.fn(async () => ({ synced: false, lines: [{ timeSeconds: null, text: '标签词' }] })),
}));

vi.mock('./client', () => ({ ipc: calls }));

beforeEach(() => {
  __resetRemoteLyrics();
  vi.clearAllMocks();
});

describe('isRemotePath', () => {
  it('只认 http(s)，本地路径与 verbatim 前缀都算本地', () => {
    expect(isRemotePath('https://ifuyo.com/audio/a.mp3')).toBe(true);
    expect(isRemotePath('http://h/a.mp3')).toBe(true);
    expect(isRemotePath('M:\\music\\a.flac')).toBe(false);
    expect(isRemotePath('\\\\?\\C:\\music\\a.mp3')).toBe(false);
  });
});

describe('lyricsForPath', () => {
  it('远程曲目：命中登记表才去取，且取的是 lyricsUrl 而不是音频 URL', async () => {
    registerRemoteLyrics([{ url: 'https://h/a.mp3', lyricsUrl: 'https://h/a.lrc' }]);
    const got = await lyricsForPath('https://h/a.mp3');
    expect(calls.ifuyoLyrics).toHaveBeenCalledWith('https://h/a.lrc');
    expect(got?.synced).toBe(true);
    expect(got?.lines[0]?.text).toBe('第一句');
    // 远程曲目绝不能走本地 TrackId 查询（那条路对 URL 永远返回 null）
    expect(calls.trackIdByPath).not.toHaveBeenCalled();
  });

  it('远程但清单没给歌词地址 → null，且不发请求', async () => {
    registerRemoteLyrics([{ url: 'https://h/b.mp3', lyricsUrl: null }]);
    expect(await lyricsForPath('https://h/b.mp3')).toBeNull();
    expect(calls.ifuyoLyrics).not.toHaveBeenCalled();
  });

  it('未登记的远程路径（如清单还没拉过）→ null，不瞎猜地址', async () => {
    expect(await lyricsForPath('https://h/unknown.mp3')).toBeNull();
    expect(calls.ifuyoLyrics).not.toHaveBeenCalled();
  });

  it('本地曲目：path → trackId → lyricsForTrack', async () => {
    const got = await lyricsForPath('M:\\music\\a.flac');
    expect(calls.trackIdByPath).toHaveBeenCalledWith('M:\\music\\a.flac');
    expect(calls.lyricsForTrack).toHaveBeenCalledWith(7);
    expect(got?.lines[0]?.text).toBe('标签词');
    expect(calls.ifuyoLyrics).not.toHaveBeenCalled();
  });

  it('本地曲目查不到 id → null（不抛）', async () => {
    calls.trackIdByPath.mockResolvedValueOnce(null);
    expect(await lyricsForPath('M:\\music\\missing.flac')).toBeNull();
    expect(calls.lyricsForTrack).not.toHaveBeenCalled();
  });

  it('空路径 / null → null', async () => {
    expect(await lyricsForPath(null)).toBeNull();
    expect(await lyricsForPath('')).toBeNull();
  });

  it('重复登记同一曲目时，新歌词地址覆盖旧的（清单更新过）', async () => {
    registerRemoteLyrics([{ url: 'https://h/a.mp3', lyricsUrl: 'https://h/old.lrc' }]);
    registerRemoteLyrics([{ url: 'https://h/a.mp3', lyricsUrl: 'https://h/new.lrc' }]);
    await lyricsForPath('https://h/a.mp3');
    expect(calls.ifuyoLyrics).toHaveBeenCalledWith('https://h/new.lrc');
  });
});
