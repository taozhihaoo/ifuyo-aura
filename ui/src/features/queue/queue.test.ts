// §197/§257/§295：queue 单一权威的边界与大量操作防护（ipc mock）。
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../ipc/client', () => ({
  ipc: { playTrack: vi.fn(async () => undefined) },
}));

import {
  clearQueue,
  getQueueState,
  hasNext,
  hasPrevious,
  playNext,
  playPrevious,
  playWithQueueContext,
  removeAt,
} from './QueueDrawer';
import type { TrackSummary } from '../../ipc/types';

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
