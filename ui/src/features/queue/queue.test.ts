// §197/§257/§295：queue 单一权威的边界与大量操作防护（ipc mock）。
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../ipc/client', () => ({
  ipc: {
    playTrack: vi.fn(async () => undefined),
    ifuyoPlay: vi.fn(async () => 'played'),
  },
}));

import { ipc } from '../../ipc/client';
import { useUiStore } from '../../state/stores';
import {
  advanceOnTrackEnded,
  clearQueue,
  cyclePlayMode,
  getPlayMode,
  getQueueState,
  hasNext,
  hasPrevious,
  playNext,
  playPrevious,
  playWithQueueContext,
  playWithRemoteContext,
  removeAt,
} from './QueueDrawer';
import type { TrackSummary } from '../../ipc/types';
import type { IfuyoTrack } from '../../ipc/ifuyo';

function track(id: number): TrackSummary {
  return {
    id,
    title: `Track ${id}`,
    artist: null,
    album: null,
    durationSeconds: 100,
    year: null,
    path: `C:/music/${id}.flac`,
    hasArtwork: false,
  };
}

beforeEach(() => clearQueue());

function remote(n: number): IfuyoTrack {
  return {
    id: `a${n}.mp3`,
    title: `Remote ${n}`,
    artist: `Artist ${n}`,
    album: null,
    url: `https://ifuyo.com/audio/a${n}.mp3`,
    durationSeconds: null,
    sizeBytes: 1024,
    format: 'mp3',
    etag: null,
    lyricsUrl: null,
  };
}

describe('queue engine（§197）', () => {
  it('playWithQueueContext 设定上下文与游标', () => {
    playWithQueueContext(track(2), [track(1), track(2), track(3)]);
    const { items, currentIndex } = getQueueState();
    expect(items).toHaveLength(3);
    expect(currentIndex).toBe(1);
  });

  it('next/previous 沿游标行走，边界返回 false（§257）', async () => {
    playWithQueueContext(track(1), [track(1), track(2), track(3)]);
    expect(hasPrevious()).toBe(false);
    expect(await playNext()).toBe(true);
    expect(await playNext()).toBe(true);
    expect(hasNext()).toBe(false);
    expect(await playNext()).toBe(false);
    expect(await playPrevious()).toBe(true);
    expect(await playPrevious()).toBe(true);
    expect(hasPrevious()).toBe(false);
    expect(await playPrevious()).toBe(false);
  });

  it('removeAt：当前项之前的移除前移游标；移除当前项钳制游标', () => {
    playWithQueueContext(track(2), [track(1), track(2), track(3)]);
    removeAt(0); // 移除当前项之前 → 游标 1→0
    expect(getQueueState().currentIndex).toBe(0);
    expect(getQueueState().items).toHaveLength(2);
    removeAt(0); // 移除当前项 → 游标钳到末尾
    expect(getQueueState().currentIndex).toBe(0);
  });

  it('×200 反复 replace/remove/clear 后状态一致（§295 大量操作）', () => {
    for (let i = 0; i < 200; i += 1) {
      playWithQueueContext(track(i % 5), [track(0), track(1), track(2), track(3), track(4)]);
      removeAt(2);
      removeAt(0);
      const { items, currentIndex } = getQueueState();
      expect(items).toHaveLength(3);
      expect(currentIndex).toBeGreaterThanOrEqual(0);
      expect(currentIndex).toBeLessThan(3);
    }
    clearQueue();
    expect(getQueueState().items).toHaveLength(0);
    expect(getQueueState().currentIndex).toBe(-1);
  });
});

describe('play mode & track-ended advance（M12.6）', () => {
  beforeEach(() => {
    while (getPlayMode() !== 'order') cyclePlayMode();
    vi.mocked(ipc.playTrack).mockClear();
  });

  it('模式循环切换 order→repeatAll→repeatOne→shuffle→order', () => {
    expect(getPlayMode()).toBe('order');
    expect(cyclePlayMode()).toBe('repeatAll');
    expect(cyclePlayMode()).toBe('repeatOne');
    expect(cyclePlayMode()).toBe('shuffle');
    expect(cyclePlayMode()).toBe('order');
  });

  it('order：播完不前进；空队列不前进', () => {
    playWithQueueContext(track(1), [track(1), track(2)]);
    expect(advanceOnTrackEnded()).toBe(false);
    expect(ipc.playTrack).not.toHaveBeenCalled();
    clearQueue();
    expect(advanceOnTrackEnded()).toBe(false);
  });

  it('repeatAll：末曲回绕到首曲', () => {
    playWithQueueContext(track(2), [track(1), track(2)]);
    cyclePlayMode(); // repeatAll
    expect(advanceOnTrackEnded()).toBe(true);
    expect(ipc.playTrack).toHaveBeenLastCalledWith(1);
    expect(getQueueState().currentIndex).toBe(0);
  });

  it('repeatOne：重播当前曲', () => {
    playWithQueueContext(track(2), [track(1), track(2)]);
    cyclePlayMode();
    cyclePlayMode(); // repeatOne
    expect(advanceOnTrackEnded()).toBe(true);
    expect(ipc.playTrack).toHaveBeenLastCalledWith(2);
  });

  it('shuffle：多项队列随机不等于当前', () => {
    playWithQueueContext(track(2), [track(1), track(2), track(3)]);
    for (let i = 0; i < 3; i += 1) cyclePlayMode(); // shuffle
    expect(advanceOnTrackEnded()).toBe(true);
    const called = vi.mocked(ipc.playTrack).mock.lastCall?.[0];
    expect(called).not.toBe(2);
    expect([1, 3]).toContain(called);
  });
});

describe('操作反馈（M12.7）', () => {
  it('clearQueue 有内容时给出成功 toast；空队列不提示', () => {
    
    playWithQueueContext(track(1), [track(1)]);
    useUiStore.getState().setNotice(null);
    clearQueue();
    expect(useUiStore.getState().notice).toBe('toast.queueCleared');
    expect(useUiStore.getState().noticeKind).toBe('success');
    // 空队列再清：不产生新 toast（seq 不变；自动消失是 App 层计时器）
    const seq = useUiStore.getState().noticeSeq;
    clearQueue();
    expect(useUiStore.getState().noticeSeq).toBe(seq);
  });
});

describe('远程队列项（修：在线音乐播放时队列恒空 → 传输键与播放模式全废）', () => {
  beforeEach(() => {
    while (getPlayMode() !== 'order') cyclePlayMode();
    vi.mocked(ipc.playTrack).mockClear();
    vi.mocked(ipc.ifuyoPlay).mockClear();
  });

  it('playWithRemoteContext 建上下文与游标；项是 remote 来源', () => {
    playWithRemoteContext(remote(2), [remote(1), remote(2), remote(3)]);
    const { items, currentIndex } = getQueueState();
    expect(items).toHaveLength(3);
    expect(currentIndex).toBe(1);
    expect(items[0]?.kind).toBe('remote');
  });

  it('远程 next/previous 走 ifuyoPlay(url)，不碰本地 playTrack', async () => {
    playWithRemoteContext(remote(1), [remote(1), remote(2)]);
    expect(await playNext()).toBe(true);
    expect(ipc.ifuyoPlay).toHaveBeenLastCalledWith(
      'https://ifuyo.com/audio/a2.mp3',
      'Remote 2',
      'Artist 2',
    );
    expect(ipc.playTrack).not.toHaveBeenCalled();
    expect(await playPrevious()).toBe(true);
    expect(ipc.ifuyoPlay).toHaveBeenLastCalledWith(
      'https://ifuyo.com/audio/a1.mp3',
      'Remote 1',
      'Artist 1',
    );
  });

  it('远程曲终自动前进：repeatAll 回绕、repeatOne 重播同一 URL', () => {
    playWithRemoteContext(remote(2), [remote(1), remote(2)]);
    cyclePlayMode(); // repeatAll
    expect(advanceOnTrackEnded()).toBe(true);
    expect(ipc.ifuyoPlay).toHaveBeenLastCalledWith(
      'https://ifuyo.com/audio/a1.mp3',
      'Remote 1',
      'Artist 1',
    );
    expect(getQueueState().currentIndex).toBe(0);

    vi.mocked(ipc.ifuyoPlay).mockClear();
    playWithRemoteContext(remote(1), [remote(1)]);
    while (getPlayMode() !== 'repeatOne') cyclePlayMode();
    expect(advanceOnTrackEnded()).toBe(true);
    expect(ipc.ifuyoPlay).toHaveBeenCalledTimes(1);
  });

  it('order 模式下远程播完不前进（与本地同语义）', () => {
    playWithRemoteContext(remote(1), [remote(1), remote(2)]);
    expect(advanceOnTrackEnded()).toBe(false);
    expect(ipc.ifuyoPlay).not.toHaveBeenCalled();
  });

  it('本地↔远程切换上下文：来源不混，removeAt 后游标仍自洽', () => {
    playWithQueueContext(track(1), [track(1), track(2)]);
    expect(getQueueState().items[0]?.kind).toBe('local');
    playWithRemoteContext(remote(1), [remote(1), remote(2), remote(3)]);
    const kinds = getQueueState().items.map((it) => it.kind);
    expect(kinds).toEqual(['remote', 'remote', 'remote']);
    removeAt(0);
    expect(getQueueState().currentIndex).toBe(0);
    expect(hasNext()).toBe(true);
  });
});
