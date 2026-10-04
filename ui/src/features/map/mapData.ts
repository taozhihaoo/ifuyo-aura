// Map 非响应式数据模型（§146/§147/§156）：节点数据走 typed arrays +
// 空间 hash，绝不进 React state。§232 有界缓存；§233 dataset_hash 变化
// 即失效。配色/过滤/相机为纯函数（稳定、可测：§113/§213/§135）。
import type { MapNodeDto, MapOverviewDto } from '../../ipc/map';
import { ipc } from '../../ipc/client';
import type { MapColorMode, MapFilters } from './mapStore';
import { clampZoom } from './mapStore';

// ---------- 稳定配色（§112/§113：来自稳定映射，非随机） ----------

/** 簇色板：12 个暗底可辨的柔和色；clusterId → 黄金角散列（稳定）。 */
export const CLUSTER_PALETTE: readonly string[] = [
  '#7aa2f7', '#bb9af7', '#7dcfff', '#9ece6a', '#e0af68', '#f7768e',
  '#ff9e64', '#73daca', '#b4f9f8', '#c0caf5', '#89ddff', '#cfc9c2',
];

export function clusterColor(clusterId: number): string {
  if (!Number.isFinite(clusterId) || clusterId < 0) return CLUSTER_PALETTE[0] as string;
  // 黄金角散列：确定性且分布均匀（§113 同一 Map 不换色）。
  const idx = Math.floor((clusterId * 0.6180339887) % 1 * CLUSTER_PALETTE.length);
  return CLUSTER_PALETTE[idx as number] as string;
}

export const PITCH_CLASSES: readonly string[] = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const PITCH_HUES: readonly number[] = [
  210, 260, 300, 340, 20, 50, 80, 120, 160, 190, 235, 275,
];

/** §213："A Minor"/"F# Major" → pitch class 序号；解析失败 → null。 */
export function parseKeyPitchClass(key: string | null): number | null {
  if (!key) return null;
  const m = /^([A-G])(#|b|♯|♭)?\s*(Major|Minor|maj|min)?/i.exec(key.trim());
  if (!m || !m[1]) return null;
  let idx = PITCH_CLASSES.indexOf(m[1].toUpperCase());
  if (idx < 0) return null;
  if (m[2] === '#' || m[2] === '♯') idx = (idx + 1) % 12;
  if (m[2] === 'b' || m[2] === '♭') idx = (idx + 11) % 12;
  return idx;
}

/** Key 色（12 pitch-class 稳定色相；缺失 → null → 中性 §216）。 */
export function pitchClassColor(idx: number): string {
  const hue = PITCH_HUES[idx] as number;
  return `hsl(${hue} 60% 66%)`;
}

export function keyColor(key: string | null): string | null {
  const idx = parseKeyPitchClass(key);
  if (idx === null) return null;
  const hue = PITCH_HUES[idx as number] as number;
  const minor = /minor|min\b/i.test(key ?? '');
  return `hsl(${hue} ${minor ? 45 : 60}% ${minor ? 62 : 68}%)`;
}

/** [0,1] 标量（energy/brightness）→ 暗蓝→暖橙梯度；缺失 → null。 */
export function scalarColor(v: number | null): string | null {
  if (v === null || !Number.isFinite(v)) return null;
  const t = Math.min(1, Math.max(0, v));
  const hue = 220 - 190 * t; // 220（冷）→ 30（暖）
  return `hsl(${hue} 70% ${50 + 12 * t}%)`;
}

export const NEUTRAL_COLOR = '#5d6472';

/** §112/§212–§216：按模式取色；缺数据的特征 → 中性灰（不用 0）。 */
export function nodeColor(
  node: Pick<MapNodeDto, 'clusterId' | 'energy' | 'musicalKey' | 'brightness'>,
  mode: MapColorMode,
): string {
  switch (mode) {
    case 'energy':
      return scalarColor(node.energy) ?? NEUTRAL_COLOR;
    case 'key':
      return keyColor(node.musicalKey) ?? NEUTRAL_COLOR;
    case 'brightness':
      return scalarColor(node.brightness) ?? NEUTRAL_COLOR;
    default:
      return clusterColor(node.clusterId);
  }
}

// ---------- 过滤（§216/§218） ----------

/** 字段缺失的曲目不匹配启用中的过滤（不得用 0 冒充 §216）。 */
export function matchesFilters(
  node: Pick<MapNodeDto, 'bpm' | 'energy'>,
  f: MapFilters,
): boolean {
  if (f.bpmMin !== null && (node.bpm === null || node.bpm < f.bpmMin)) return false;
  if (f.bpmMax !== null && (node.bpm === null || node.bpm > f.bpmMax)) return false;
  if (f.energyMin !== null && (node.energy === null || node.energy < f.energyMin)) return false;
  return true;
}

// ---------- 相机（world [-1,1]² ↔ screen；§135/§137/§139） ----------

export interface Camera {
  zoom: number;
  panX: number;
  panY: number;
}

export const DEFAULT_CAMERA: Camera = { zoom: 1, panX: 0, panY: 0 };

function halfSpan(w: number, h: number): number {
  return Math.max(1, Math.min(w, h)) / 2;
}

export function worldToScreen(
  cam: Camera,
  w: number,
  h: number,
  wx: number,
  wy: number,
): [number, number] {
  const k = cam.zoom * halfSpan(w, h);
  return [(wx - cam.panX) * k + w / 2, (wy - cam.panY) * k + h / 2];
}

export function screenToWorld(
  cam: Camera,
  w: number,
  h: number,
  sx: number,
  sy: number,
): [number, number] {
  const k = cam.zoom * halfSpan(w, h);
  return [(sx - w / 2) / k + cam.panX, (sy - h / 2) / k + cam.panY];
}

/** 以光标为中心缩放（§137）：光标下的 world 点保持不动。 */
export function zoomAt(
  cam: Camera,
  w: number,
  h: number,
  sx: number,
  sy: number,
  factor: number,
): Camera {
  const [wx, wy] = screenToWorld(cam, w, h, sx, sy);
  const zoom = clampZoom(cam.zoom * factor);
  if (zoom === cam.zoom) return cam;
  const k = zoom * halfSpan(w, h);
  return {
    zoom,
    panX: wx - (sx - w / 2) / k,
    panY: wy - (sy - h / 2) / k,
  };
}

/** 平移钳制：中心不越出扩展世界盒（§135 防 1e30 级 transform）。 */
export function clampCamera(cam: Camera): Camera {
  const lim = 1.5;
  return {
    zoom: clampZoom(cam.zoom),
    panX: Number.isFinite(cam.panX) ? Math.max(-lim, Math.min(lim, cam.panX)) : 0,
    panY: Number.isFinite(cam.panY) ? Math.max(-lim, Math.min(lim, cam.panY)) : 0,
  };
}

/** §138/§139：fit bounds；0 节点 → 缺省视图（不除零）。 */
export function fitCamera(
  bounds: [number, number, number, number],
  w: number,
  h: number,
): Camera {
  const [minX, minY, maxX, maxY] = bounds;
  if (
    !bounds.every(Number.isFinite) ||
    (maxX - minX <= 1e-6 && maxY - minY <= 1e-6)
  ) {
    return { ...DEFAULT_CAMERA };
  }
  const half = halfSpan(w, h);
  const zoom = clampZoom((0.9 * Math.min(w, h)) / (half * Math.max(maxX - minX, maxY - minY, 1e-6)));
  return clampCamera({
    zoom,
    panX: (minX + maxX) / 2,
    panY: (minY + maxY) / 2,
  });
}

// ---------- 空间 hash（§156：hit testing 不遍历全图） ----------

export class SpatialHash {
  private readonly cell: number;
  private cells = new Map<number, number[]>();

  constructor(cell = 0.08) {
    this.cell = cell > 1e-4 ? cell : 0.08;
  }

  private key(cx: number, cy: number): number {
    return (cx + 4096) * 100_000 + (cy + 4096);
  }

  insert(index: number, x: number, y: number): void {
    const cx = Math.floor(x / this.cell);
    const cy = Math.floor(y / this.cell);
    const k = this.key(cx, cy);
    const list = this.cells.get(k);
    if (list) list.push(index);
    else this.cells.set(k, [index]);
  }

  /** 矩形内的候选索引（可能重复/超界，由调用方精筛）。 */
  query(minX: number, minY: number, maxX: number, maxY: number): number[] {
    const out: number[] = [];
    const x0 = Math.floor(minX / this.cell);
    const x1 = Math.floor(maxX / this.cell);
    const y0 = Math.floor(minY / this.cell);
    const y1 = Math.floor(maxY / this.cell);
    for (let cx = x0; cx <= x1; cx++) {
      for (let cy = y0; cy <= y1; cy++) {
        const list = this.cells.get(this.key(cx, cy));
        if (list) out.push(...list);
      }
    }
    return out;
  }

  /** 最近索引；无 → -1（§157 hover/click 区分由调用方处理）。 */
  nearest(x: number, y: number, maxDist: number, posAt: (i: number) => [number, number]): number {
    let best = -1;
    let bestD2 = maxDist * maxDist;
    for (const i of this.query(x - maxDist, y - maxDist, x + maxDist, y + maxDist)) {
      const [px, py] = posAt(i);
      const d2 = (px - x) * (px - x) + (py - y) * (py - y);
      if (d2 <= bestD2) {
        bestD2 = d2;
        best = i;
      }
    }
    return best;
  }

  get size(): number {
    return this.cells.size;
  }
}

// ---------- 数据模型（§146 imperative renderer state） ----------

type FetchNodes = (
  minX: number | null,
  minY: number | null,
  maxX: number | null,
  maxY: number | null,
  limit: number | null,
) => Promise<MapNodeDto[]>;

const WHOLE_WORLD: [number, number, number, number] = [-1, -1, 1, 1];

export class MapDataModel {
  datasetHash: string | null = null;
  overview: MapOverviewDto | null = null;
  count = 0;
  private ids: Int32Array = new Int32Array(0);
  private positions = new Float32Array(0);
  private clusterIds = new Int32Array(0);
  private idToIndex = new Map<number, number>();
  /** hover/detail 元数据（对象仅存已取节点；有界 §232）。 */
  metas = new Map<number, MapNodeDto>();
  private hash = new SpatialHash();
  private loadedAll = false;
  private neighborsCache = new Map<number, Awaited<ReturnType<typeof ipc.mapNeighbors>>>();
  private fetchNodes: FetchNodes;

  constructor(fetchNodes: FetchNodes = (a, b, c, d, e) => ipc.mapNodes(a, b, c, d, e)) {
    this.fetchNodes = fetchNodes;
  }

  /** §233：dataset_hash 变化 → 旧数据全部失效。 */
  resetForDataset(hash: string, overview: MapOverviewDto): void {
    if (this.datasetHash === hash) return;
    this.datasetHash = hash;
    this.overview = overview;
    this.count = 0;
    this.ids = new Int32Array(0);
    this.positions = new Float32Array(0);
    this.clusterIds = new Int32Array(0);
    this.idToIndex.clear();
    this.metas.clear();
    this.hash = new SpatialHash();
    this.loadedAll = false;
    this.neighborsCache.clear();
  }

  private grow(need: number): void {
    if (need <= this.ids.length) return;
    const cap = Math.max(1024, this.ids.length * 2, need);
    const ids = new Int32Array(cap);
    ids.set(this.ids.subarray(0, this.count));
    const pos = new Float32Array(cap * 2);
    pos.set(this.positions.subarray(0, this.count * 2));
    const cl = new Int32Array(cap);
    cl.set(this.clusterIds.subarray(0, this.count));
    this.ids = ids;
    this.positions = pos;
    this.clusterIds = cl;
  }

  private insert(node: MapNodeDto): void {
    if (this.idToIndex.has(node.trackId)) return;
    this.grow(this.count + 1);
    const i = this.count;
    this.ids[i] = node.trackId;
    this.positions[i * 2] = node.x;
    this.positions[i * 2 + 1] = node.y;
    this.clusterIds[i] = node.clusterId;
    this.idToIndex.set(node.trackId, i);
    this.metas.set(node.trackId, node);
    this.hash.insert(i, node.x, node.y);
    this.count = i + 1;
  }

  posAt(i: number): [number, number] {
    return [this.positions[i * 2] as number, this.positions[i * 2 + 1] as number];
  }

  idAt(i: number): number {
    return this.ids[i] as number;
  }

  indexOf(trackId: number): number {
    return this.idToIndex.get(trackId) ?? -1;
  }

  metaOf(trackId: number): MapNodeDto | null {
    return this.metas.get(trackId) ?? null;
  }

  /** §229/§230：按区域批量补数（小库一次全量，大库按视口）。 */
  async ensureRegion(minX: number, minY: number, maxX: number, maxY: number): Promise<void> {
    if (this.datasetHash === null || this.loadedAll) return;
    const nodes = await this.fetchNodes(minX, minY, maxX, maxY, null);
    for (const n of nodes) this.insert(n);
    if (minX <= -1 && minY <= -1 && maxX >= 1 && maxY >= 1) this.loadedAll = true;
  }

  /** 打开地图时全量加载（≤2 万节点；更大库走 ensureRegion 视口补数）。 */
  async loadAll(): Promise<void> {
    if (this.datasetHash === null || this.loadedAll) return;
    const total = this.overview?.trackCount ?? 0;
    if (total === 0 || total > 20_000) {
      await this.ensureRegion(...WHOLE_WORLD);
      return;
    }
    const nodes = await this.fetchNodes(-1, -1, 1, 1, total);
    for (const n of nodes) this.insert(n);
    this.loadedAll = true;
  }

  get isLoadedAll(): boolean {
    return this.loadedAll;
  }

  /** §156：hit testing 走空间 hash，不遍历全图。 */
  hitTest(wx: number, wy: number, radiusWorld: number): number | null {
    const i = this.hash.nearest(wx, wy, radiusWorld, (j) => this.posAt(j));
    return i >= 0 ? this.idAt(i) : null;
  }

  /** §143：视口 culling（空间 hash 查询 + 边界精筛）。 */
  visibleIndices(minX: number, minY: number, maxX: number, maxY: number): number[] {
    const out: number[] = [];
    for (const i of this.hash.query(minX, minY, maxX, maxY)) {
      const [x, y] = this.posAt(i);
      if (x >= minX && x <= maxX && y >= minY && y <= maxY) out.push(i);
    }
    return out;
  }

  /** §216/§218：过滤掩码（1 = 可见）。 */
  filterMask(f: MapFilters): Uint8Array {
    const mask = new Uint8Array(this.count).fill(1);
    if (f.bpmMin === null && f.bpmMax === null && f.energyMin === null) return mask;
    for (let i = 0; i < this.count; i++) {
      const id = this.ids[i] as number;
      const m = this.metas.get(id);
      const ok =
        m !== undefined &&
        matchesFilters(
          { bpm: m.bpm, energy: m.energy },
          f,
        );
      mask[i] = ok ? 1 : 0;
    }
    return mask;
  }

  /** §126/§232：邻居列表缓存（有界）。 */
  async neighborsFor(trackId: number, k = 8): Promise<Awaited<ReturnType<typeof ipc.mapNeighbors>>> {
    const hit = this.neighborsCache.get(trackId);
    if (hit) return hit;
    const rows = await ipc.mapNeighbors(trackId, k);
    if (this.neighborsCache.size >= 64) {
      const first = this.neighborsCache.keys().next();
      if (!first.done) this.neighborsCache.delete(first.value);
    }
    this.neighborsCache.set(trackId, rows);
    return rows;
  }

  /** §130：搜索结果与地图交集（匹配不高亮时也用于 focus）。 */
  presentTrackIds(ids: number[]): number[] {
    return ids.filter((id) => this.idToIndex.has(id));
  }
}
