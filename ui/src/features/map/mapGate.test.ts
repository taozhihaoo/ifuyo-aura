// M9 下篇前端门禁测试：视图恢复（§180/§181/§331）+ 搜索竞态守卫
// （§345/§485——stale 响应不得覆盖最新）+ LOD 阈值序（§338 精神）。
import { describe, expect, it, vi } from 'vitest';
import { CLUSTER_LOD_ZOOM, LABEL_LOD_ZOOM, loadSavedView, MAP_VIEW_STORAGE_KEY, saveView } from './mapData';
import { LibrarySearchRunner } from './mapSearch';
import { useMapStore } from './mapStore';

interface StorageStub {
  getItem: (k: string) => string | null;
  setItem: (k: string, v: string) => void;
}

function stubStorage(): StorageStub {
  const m = new Map<string, string>();
  return {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => void m.set(k, v),
  };
}

describe('视图持久化（§180/§181/§331）', () => {
  it('save → load 往返（同 hash）；跨 hash/损坏/非有限 → null', () => {
    const st = stubStorage();
    useMapStore.setState({ zoom: 2.5, pan: { x: 0.3, y: -0.4 } });
    saveView('h1', st);
    const v = loadSavedView('h1', st);
    expect(v).not.toBeNull();
    expect(v?.zoom).toBe(2.5);
    expect(v?.panX).toBeCloseTo(0.3, 6);
    expect(v?.panY).toBeCloseTo(-0.4, 6);

    // 跨 dataset hash → 拒绝恢复（§180 条件）。
    expect(loadSavedView('h2', st)).toBeNull();

    // 损坏 JSON → null 不 throw（§181）。
    st.setItem(MAP_VIEW_STORAGE_KEY, '{broken');
    expect(loadSavedView('h1', st)).toBeNull();

    // 非有限 → null（§181/§192）。
    st.setItem(
      MAP_VIEW_STORAGE_KEY,
      JSON.stringify({ hash: 'h1', zoom: Number.NaN, panX: 0, panY: 0 }),
    );
    expect(loadSavedView('h1', st)).toBeNull();

    // null hash 不写。
    saveView(null, st);
  });
});

describe('搜索竞态守卫（§345/§485）', () => {
  it('旧 query 的迟到响应不得覆盖最新', async () => {
    let resolveA: (rows: { id: number }[]) => void = () => undefined;
    let resolveB: (rows: { id: number }[]) => void = () => undefined;
    const search = vi.fn((q: string) => {
      if (q === 'a') {
        return new Promise<{ id: number }[]>((res) => {
          resolveA = res;
        });
      }
      return new Promise<{ id: number }[]>((res) => {
        resolveB = res;
      });
    });
    const seen: Array<{ q: string; ids: number[] }> = [];
    const runner = new LibrarySearchRunner(search, (q, ids) => seen.push({ q, ids }));

    runner.run('a');
    runner.run('b');
    expect(search).toHaveBeenCalledTimes(2);

    // 旧响应迟到 → 丢弃。
    resolveA([{ id: 1 }]);
    await Promise.resolve();
    await Promise.resolve();
    expect(seen).toHaveLength(0);

    // 新响应 → 生效。
    resolveB([{ id: 2 }, { id: 3 }]);
    await Promise.resolve();
    await Promise.resolve();
    expect(seen).toEqual([{ q: 'b', ids: [2, 3] }]);

    // 空 query → 立即清空。
    runner.run('  ');
    expect(seen[1]).toEqual({ q: '', ids: [] });
  });

  it('失败只影响最新请求', async () => {
    let rejectB: (e: Error) => void = () => undefined;
    const search = vi.fn((q: string) =>
      q === 'ok' ? Promise.resolve([{ id: 9 }]) : new Promise<never>((_, rej) => { rejectB = rej; }),
    );
    const seen: Array<{ q: string; ids: number[] }> = [];
    const runner = new LibrarySearchRunner(search, (q, ids) => seen.push({ q, ids }));
    runner.run('ok');
    await Promise.resolve();
    await Promise.resolve();
    expect(seen).toEqual([{ q: 'ok', ids: [9] }]);
    runner.run('bad');
    rejectB(new Error('x'));
    await Promise.resolve();
    await Promise.resolve();
    expect(seen[1]).toEqual({ q: 'bad', ids: [] });
  });
});

describe('LOD 阈值（§338 精神：far < mid < near 严格递增）', () => {
  it('cluster LOD < label LOD', () => {
    expect(CLUSTER_LOD_ZOOM).toBeLessThan(LABEL_LOD_ZOOM);
  });
});
