// Music Map typed IPC 契约（M9 中篇；与 src-tauri/src/map_ipc.rs DTO 镜像）。
// §191：收到 positions 逐一 Number.isFinite 校验；非有限节点丢弃
// （§190 不让整个 renderer 变 NaN）。禁止 any。

export type MapBuildStatus = 'pending' | 'building' | 'ready' | 'failed' | 'cancelled' | 'stale';

export interface MapStatusDto {
  status: MapBuildStatus;
  datasetHash: string | null;
  trackCount: number;
  clusterCount: number;
  buildId: number;
  message: string | null;
}

export interface MapClusterDto {
  clusterId: number;
  count: number;
  centroid: [number, number] | null;
  radius: number | null;
}

export interface MapOverviewDto {
  datasetHash: string;
  trackCount: number;
  isolatedCount: number;
  clusterCount: number;
  /** min_x/min_y/max_x/max_y（有限，§155）。 */
  bounds: [number, number, number, number];
  clusters: MapClusterDto[];
}

export interface MapNodeDto {
  trackId: number;
  x: number;
  y: number;
  clusterId: number;
  title: string | null;
  artist: string | null;
  album: string | null;
  genre: string | null;
  bpm: number | null;
  musicalKey: string | null;
  energy: number | null;
  brightness: number | null;
  integratedLufs: number | null;
}

export interface MapNeighborDto {
  trackId: number;
  similarity: number;
  title: string | null;
  artist: string | null;
}

function finite(v: unknown): v is number {
  return typeof v === 'number' && Number.isFinite(v);
}

function optFinite(v: unknown): v is number {
  return v === null || finite(v);
}

function optStr(v: unknown): v is string | null {
  return v === null || typeof v === 'string';
}

/** §191/§190：单节点校验——非有限位置/相似度特征 → 丢弃该节点。 */
export function isMapNodeDto(v: unknown): v is MapNodeDto {
  if (typeof v !== 'object' || v === null) return false;
  const n = v as Record<string, unknown>;
  return (
    finite(n.trackId) &&
    finite(n.x) &&
    finite(n.y) &&
    finite(n.clusterId) &&
    optStr(n.title) &&
    optStr(n.artist) &&
    optStr(n.album) &&
    optStr(n.genre) &&
    optFinite(n.bpm) &&
    optStr(n.musicalKey) &&
    optFinite(n.energy) &&
    optFinite(n.brightness) &&
    optFinite(n.integratedLufs)
  );
}

export function isMapStatusDto(v: unknown): v is MapStatusDto {
  if (typeof v !== 'object' || v === null) return false;
  const n = v as Record<string, unknown>;
  const statuses: readonly string[] = [
    'pending',
    'building',
    'ready',
    'failed',
    'cancelled',
    'stale',
  ];
  return (
    typeof n.status === 'string' &&
    statuses.includes(n.status) &&
    (n.datasetHash === null || typeof n.datasetHash === 'string') &&
    finite(n.trackCount) &&
    finite(n.clusterCount) &&
    finite(n.buildId) &&
    (n.message === null || typeof n.message === 'string')
  );
}

function isFiniteTuple4(v: unknown): v is [number, number, number, number] {
  return Array.isArray(v) && v.length === 4 && v.every(finite);
}

export function isMapOverviewDto(v: unknown): v is MapOverviewDto {
  if (typeof v !== 'object' || v === null) return false;
  const n = v as Record<string, unknown>;
  return (
    typeof n.datasetHash === 'string' &&
    finite(n.trackCount) &&
    finite(n.isolatedCount) &&
    finite(n.clusterCount) &&
    isFiniteTuple4(n.bounds) &&
    Array.isArray(n.clusters)
  );
}

export function isMapNeighborDto(v: unknown): v is MapNeighborDto {
  if (typeof v !== 'object' || v === null) return false;
  const n = v as Record<string, unknown>;
  return (
    finite(n.trackId) &&
    finite(n.similarity) &&
    optStr(n.title) &&
    optStr(n.artist)
  );
}
