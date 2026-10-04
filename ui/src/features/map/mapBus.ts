// Map 视图命令/焦点总线（§311/§411）：Library→Map "Open on Map" 与
// palette→Map 命令经模块级单例传递（非响应式 UI 域——不进 §145 白名单
// store，也不属地图数据 §146）。MapPage 在数据就绪后消费。
import type { MapViewCommand } from './MusicMapView';

let pendingFocus: number | null = null;
let pendingCommand: MapViewCommand['kind'] | null = null;

export function requestMapFocus(trackId: number): void {
  pendingFocus = trackId;
}

export function requestMapCommand(kind: MapViewCommand['kind']): void {
  pendingCommand = kind;
}

export function consumeMapFocus(): number | null {
  const f = pendingFocus;
  pendingFocus = null;
  return f;
}

export function consumeMapCommand(): MapViewCommand['kind'] | null {
  const c = pendingCommand;
  pendingCommand = null;
  return c;
}
