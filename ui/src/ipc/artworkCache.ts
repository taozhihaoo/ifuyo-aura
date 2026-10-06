// M13.2 曲库封面缩略图缓存：虚拟列表只渲染可视行，但快速滚动仍会
// 重复请求同一曲目——(1) Map 缓存（含"无封面"负缓存，避免反复打空枪）；
// (2) 并发去重（同 id 在途请求合并）；(3) 容量上限（FIFO 淘汰）。
// library-changed 事件时调用 clear() 失效全量（扫描可能新增封面）。
import { ipc } from '../ipc/client';

const MAX_ENTRIES = 512;

const cache = new Map<number, string | null>();
const inFlight = new Map<number, Promise<string | null>>();

/** 同步读取（渲染路径）：命中返回 dataUrl 或 null（已确认无封面）；
 *  未缓存返回 undefined。 */
export function peekArtwork(trackId: number): string | null | undefined {
  return cache.get(trackId);
}

/** 异步取封面：缓存命中直接返回；在途请求合并；完成后写缓存。 */
export function loadArtwork(trackId: number): Promise<string | null> {
  const hit = cache.get(trackId);
  if (hit !== undefined) return Promise.resolve(hit);
  const pending = inFlight.get(trackId);
  if (pending) return pending;
  const p = ipc
    .trackArtwork(trackId)
    .then((url) => {
      store(trackId, url);
      return url;
    })
    .catch(() => {
      store(trackId, null);
      return null;
    })
    .finally(() => {
      inFlight.delete(trackId);
    });
  inFlight.set(trackId, p);
  return p;
}

function store(trackId: number, url: string | null) {
  if (cache.has(trackId)) cache.delete(trackId); // Map 有序：重插=移到最新
  cache.set(trackId, url);
  while (cache.size > MAX_ENTRIES) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
  }
}

/** 库变更（扫描/删除）后全量失效。 */
export function clearArtworkCache() {
  cache.clear();
}
