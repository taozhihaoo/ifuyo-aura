// M9 中篇前端逻辑测试（§113/§130/§135/§139/§145/§146/§156/§191/§216/
// §233）：配色稳定、空间 hash、相机安全、过滤、dataset 失效、store 形状。
import { beforeEach, describe, expect, it } from 'vitest';
import type { MapNodeDto } from '../../ipc/map';
import { isMapNodeDto } from '../../ipc/map';
import {
  clampCamera,
  fitCamera,
  MapDataModel,
  matchesFilters,
  NEUTRAL_COLOR,
  nodeColor,
  parseKeyPitchClass,
  keyColor,
  scalarColor,
  SpatialHash,
  screenToWorld,
  worldToScreen,
  zoomAt,
  clusterColor,
} from './mapData';
import { clampZoom, EMPTY_FILTERS, useMapStore } from './mapStore';

function node(partial: Partial<MapNodeDto>): MapNodeDto {
  return {
    trackId: 1,
    x: 0,
    y: 0,
    clusterId: 0,
    title: 'T',
    artist: null,
    album: null,
    genre: null,
    bpm: null,
    musicalKey: null,
    energy: null,
    brightness: null,
    integratedLufs: null,
    ...partial,
  };
}

beforeEach(() => {
  useMapStore.setState({
    mapOpen: false,
    mapBuildStatus: 'pending',
    selectedTrackId: null,
    selectedClusterId: null,
    zoom: 1,
    pan: { x: 0, y: 0 },
    filters: { ...EMPTY_FILTERS },
    colorMode: 'cluster',
  });
});

describe('mapStore（§145 仅允许的 8 个字段）', () => {
  it('状态形状不超出 §145 白名单', () => {
    const allowed = new Set([
      'mapOpen',
      'mapBuildStatus',
      'selectedTrackId',
      'selectedClusterId',
      'zoom',
      'pan',
      'filters',
      'colorMode',
      'setMapOpen',
      'setMapBuildStatus',
      'selectTrack',
      'selectCluster',
      'setZoom',
      'setPan',
      'setFilters',
      'clearFilters',
      'setColorMode',
    ]);
    for (const key of Object.keys(useMapStore.getState())) {
      expect(allowed.has(key), `unexpected key ${key}`).toBe(true);
    }
    // 节点数据绝不进 store（§147）。
    expect('nodes' in useMapStore.getState()).toBe(false);
  });

  it('zoom 边界钳制（§135）/pan 非有限回零（§192）', () => {
    expect(clampZoom(0.001)).toBeGreaterThan(0);
    expect(clampZoom(1e9)).toBeLessThan(1e3);
    expect(clampZoom(Number.NaN)).toBe(1);
    useMapStore.getState().setPan({ x: Number.NaN, y: 3 });
    expect(useMapStore.getState().pan).toEqual({ x: 0, y: 3 });
    useMapStore.getState().setPan({ x: 99, y: -99 });
    expect(useMapStore.getState().pan).toEqual({ x: 4, y: -4 });
    useMapStore.getState().selectTrack(7);
    useMapStore.getState().selectTrack(null);
    expect(useMapStore.getState().selectedClusterId).toBeNull();
  });
});

describe('稳定配色（§112/§113/§213/§216）', () => {
  it('cluster 色确定且稳定', () => {
    expect(clusterColor(3)).toBe(clusterColor(3));
    const seen = new Set([1, 2, 3, 4, 5, 6, 7, 8].map(clusterColor));
    expect(seen.size).toBeGreaterThan(2);
  });

  it('key 解析与色相稳定；失败 → null（中性 §216）', () => {
    expect(parseKeyPitchClass('A Minor')).toBe(9);
    expect(parseKeyPitchClass('F# Major')).toBe(6);
    expect(parseKeyPitchClass('C')).toBe(0);
    expect(parseKeyPitchClass('noise')).toBeNull();
    expect(keyColor(null)).toBeNull();
    expect(keyColor('A Minor')).not.toBe(keyColor('A Major'));
    expect(keyColor('A Minor')).toBe(keyColor('A Minor'));
  });

  it('标量色：缺失 → null；单调映射；nodeColor 缺数据 → 中性灰', () => {
    expect(scalarColor(null)).toBeNull();
    expect(scalarColor(-5)).toBe(scalarColor(0));
    expect(scalarColor(2)).toBe(scalarColor(1));
    expect(
      nodeColor({ clusterId: 0, energy: null, musicalKey: null, brightness: null }, 'energy'),
    ).toBe(NEUTRAL_COLOR);
  });
});

describe('过滤（§216/§218）', () => {
  it('字段缺失不匹配启用中的过滤（不得用 0 冒充）', () => {
    const f = { bpmMin: 120, bpmMax: 130, energyMin: null };
    expect(matchesFilters({ bpm: 125, energy: null }, f)).toBe(true);
    expect(matchesFilters({ bpm: 100, energy: null }, f)).toBe(false);
    expect(matchesFilters({ bpm: null, energy: null }, f)).toBe(false);
    expect(matchesFilters({ bpm: 125, energy: null }, EMPTY_FILTERS)).toBe(true);
  });
});

describe('空间 hash（§156）', () => {
  it('nearest 命中最近点；超距 → -1', () => {
    const h = new SpatialHash(0.1);
    const pts: Array<[number, number]> = [
      [0, 0],
      [0.5, 0.5],
      [0.52, 0.5],
    ];
    pts.forEach(([x, y], i) => h.insert(i, x, y));
    const at = (i: number): [number, number] => pts[i] as [number, number];
    expect(h.nearest(0.51, 0.5, 0.1, at)).toBe(2);
    expect(h.nearest(0.01, 0, 0.05, at)).toBe(0);
    expect(h.nearest(0.9, 0.9, 0.05, at)).toBe(-1);
    expect(h.query(0.4, 0.4, 0.6, 0.6).length).toBeGreaterThanOrEqual(2);
  });
});

describe('相机（§135/§137/§139/§192）', () => {
  it('world↔screen 往返一致', () => {
    const cam = { zoom: 2.5, panX: 0.3, panY: -0.2 };
    const [sx, sy] = worldToScreen(cam, 800, 600, 0.123, -0.456);
    const [wx, wy] = screenToWorld(cam, 800, 600, sx, sy);
    expect(wx).toBeCloseTo(0.123, 5);
    expect(wy).toBeCloseTo(-0.456, 5);
  });

  it('zoomAt 以光标为中心（§137）', () => {
    const cam = { zoom: 1, panX: 0, panY: 0 };
    const [wx0, wy0] = screenToWorld(cam, 800, 600, 500, 300);
    const next = zoomAt(cam, 800, 600, 500, 300, 2);
    const [wx1, wy1] = screenToWorld(next, 800, 600, 500, 300);
    expect(wx1).toBeCloseTo(wx0, 5);
    expect(wy1).toBeCloseTo(wy0, 5);
    expect(next.zoom).toBeGreaterThan(cam.zoom);
  });

  it('非有限相机被钳制回安全值（§192）', () => {
    const cam = clampCamera({ zoom: Number.NaN, panX: 1e30, panY: Number.POSITIVE_INFINITY });
    expect(Number.isFinite(cam.zoom)).toBe(true);
    expect(Math.abs(cam.panX)).toBeLessThan(2);
    expect(Number.isFinite(cam.panY)).toBe(true);
  });

  it('fitCamera：0 尺寸 bounds → 缺省；正常 bounds 在缩放界内（§135/§139）', () => {
    expect(fitCamera([0, 0, 0, 0], 800, 600)).toEqual({ zoom: 1, panX: 0, panY: 0 });
    const cam = fitCamera([-1, -1, 1, 1], 800, 600);
    expect(cam.zoom).toBeGreaterThanOrEqual(0.4);
    expect(cam.panX).toBeCloseTo(0, 5);
  });
});

function makeModel(rows: MapNodeDto[]): MapDataModel {
  return new MapDataModel(async () => rows);
}

describe('MapDataModel（§130/§146/§191/§233）', () => {
  it('加载/去重/hitTest/视口 culling', async () => {
    const m = makeModel([
      node({ trackId: 1, x: -0.5, y: -0.5, clusterId: 0 }),
      node({ trackId: 2, x: 0.5, y: 0.5, clusterId: 1, bpm: 125, energy: 0.7 }),
    ]);
    m.resetForDataset('h1', {
      datasetHash: 'h1',
      trackCount: 2,
      isolatedCount: 0,
      clusterCount: 2,
      bounds: [-0.5, -0.5, 0.5, 0.5],
      clusters: [],
    });
    await m.loadAll();
    await m.loadAll(); // 幂等
    expect(m.count).toBe(2);
    expect(m.hitTest(0.49, 0.49, 0.05)).toBe(2);
    expect(m.hitTest(0.9, 0.9, 0.05)).toBeNull();
    expect(m.visibleIndices(0.4, 0.4, 0.6, 0.6)).toEqual([1]);
    expect(m.presentTrackIds([1, 2, 99])).toEqual([1, 2]);
  });

  it('§233：dataset hash 变化 → 全部失效', async () => {
    const m = makeModel([node({ trackId: 1, x: 0, y: 0, clusterId: 0 })]);
    m.resetForDataset('h1', {
      datasetHash: 'h1',
      trackCount: 1,
      isolatedCount: 0,
      clusterCount: 1,
      bounds: [0, 0, 0, 0],
      clusters: [],
    });
    await m.loadAll();
    expect(m.count).toBe(1);
    m.resetForDataset('h1', {
      datasetHash: 'h1',
      trackCount: 1,
      isolatedCount: 0,
      clusterCount: 1,
      bounds: [0, 0, 0, 0],
      clusters: [],
    });
    expect(m.count).toBe(1); // 同 hash：保留
    m.resetForDataset('h2', {
      datasetHash: 'h2',
      trackCount: 0,
      isolatedCount: 0,
      clusterCount: 0,
      bounds: [0, 0, 0, 0],
      clusters: [],
    });
    expect(m.count).toBe(0);
    expect(m.hitTest(0, 0, 0.5)).toBeNull();
  });

  it('过滤掩码：无过滤全可见；启用后缺失字段隐藏（§216）', async () => {
    const m = makeModel([
      node({ trackId: 1, x: 0, y: 0, clusterId: 0, bpm: 125, energy: 0.8 }),
      node({ trackId: 2, x: 0.1, y: 0, clusterId: 0, bpm: null, energy: null }),
    ]);
    m.resetForDataset('h1', {
      datasetHash: 'h1',
      trackCount: 2,
      isolatedCount: 0,
      clusterCount: 1,
      bounds: [0, 0, 0.1, 0],
      clusters: [],
    });
    await m.loadAll();
    expect(m.filterMask(EMPTY_FILTERS)).toEqual(new Uint8Array([1, 1]));
    expect(m.filterMask({ bpmMin: 120, bpmMax: null, energyMin: null })).toEqual(new Uint8Array([1, 0]));
    expect(m.filterMask({ bpmMin: null, bpmMax: null, energyMin: 0.6 })).toEqual(new Uint8Array([1, 0]));
  });
});

describe('节点校验（§191）', () => {
  it('非有限位置/相似度字段拒绝', () => {
    expect(isMapNodeDto(node({}))).toBe(true);
    expect(isMapNodeDto(node({ x: Number.NaN }))).toBe(false);
    expect(isMapNodeDto(node({ energy: Number.POSITIVE_INFINITY }))).toBe(false);
    expect(isMapNodeDto({ ...node({}), trackId: 'x' })).toBe(false);
  });
});
