// Map UI 状态（§145）：Zustand 只保存 open/status/selection/zoom/pan/
// filters/colorMode。节点数据（100k 级）绝不进 React state（§146/§147）
// ——走 features/map/mapData.ts 的非响应式 renderer state。
import { create } from 'zustand';

export type MapBuildStatus =
  | 'pending'
  | 'building'
  | 'ready'
  | 'failed'
  | 'cancelled'
  | 'stale';

/** §112/§210：颜色模式（仅提供有真实 M8 数据的模式；Source 暂无）。 */
export type MapColorMode = 'cluster' | 'energy' | 'key' | 'brightness';

/** §218：过滤 chips（字段缺失的曲目不匹配该过滤 §216 精神）。 */
export interface MapFilters {
  bpmMin: number | null;
  bpmMax: number | null;
  energyMin: number | null;
}

export const EMPTY_FILTERS: MapFilters = { bpmMin: null, bpmMax: null, energyMin: null };

/** §135 缩放边界（防 Infinity/NaN/坐标溢出）。 */
export const MIN_ZOOM = 0.4;
export const MAX_ZOOM = 12;

export interface MapUiState {
  mapOpen: boolean;
  mapBuildStatus: MapBuildStatus;
  selectedTrackId: number | null;
  selectedClusterId: number | null;
  zoom: number;
  pan: { x: number; y: number };
  filters: MapFilters;
  colorMode: MapColorMode;
  setMapOpen: (open: boolean) => void;
  setMapBuildStatus: (s: MapBuildStatus) => void;
  selectTrack: (id: number | null) => void;
  selectCluster: (id: number | null) => void;
  setZoom: (z: number) => void;
  setPan: (p: { x: number; y: number }) => void;
  setFilters: (f: MapFilters) => void;
  clearFilters: () => void;
  setColorMode: (m: MapColorMode) => void;
}

export const clampZoom = (z: number): number =>
  Number.isFinite(z) ? Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, z)) : 1;

export const useMapStore = create<MapUiState>((set) => ({
  mapOpen: false,
  mapBuildStatus: 'pending',
  selectedTrackId: null,
  selectedClusterId: null,
  zoom: 1,
  pan: { x: 0, y: 0 },
  filters: EMPTY_FILTERS,
  colorMode: 'cluster',
  setMapOpen: (mapOpen) => set({ mapOpen }),
  setMapBuildStatus: (mapBuildStatus) => set({ mapBuildStatus }),
  selectTrack: (selectedTrackId) =>
    set(selectedTrackId === null ? { selectedTrackId, selectedClusterId: null } : { selectedTrackId }),
  selectCluster: (selectedClusterId) => set({ selectedClusterId }),
  setZoom: (zoom) => set({ zoom: clampZoom(zoom) }),
  setPan: (pan) =>
    set({
      pan: {
        x: Number.isFinite(pan.x) ? Math.max(-4, Math.min(4, pan.x)) : 0,
        y: Number.isFinite(pan.y) ? Math.max(-4, Math.min(4, pan.y)) : 0,
      },
    }),
  setFilters: (filters) => set({ filters }),
  clearFilters: () => set({ filters: EMPTY_FILTERS }),
  setColorMode: (colorMode) => set({ colorMode }),
}));
