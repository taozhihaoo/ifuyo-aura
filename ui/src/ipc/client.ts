// Typed IPC client（§11）：invoke 封装 + AppError 归一。
import { invoke } from '@tauri-apps/api/core';
import { isAppError, type AppSettings, type PlayerSnapshot, type ScanProgress, type SortKey, type TrackSummary } from './types';
import { isDreamscapeState, isSceneManifest, type DreamscapeStateDto, type SceneManifest } from './dreamscape';
import {
  isMapNeighborDto,
  isMapNodeDto,
  isMapOverviewDto,
  isMapStatusDto,
  type MapClusterDto,
  type MapNeighborDto,
  type MapNodeDto,
  type MapOverviewDto,
  type MapStatusDto,
} from './map';

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if (isAppError(e)) throw e;
    throw { code: 'INTERNAL', message: String(e), recoverable: false };
  }
}

export const ipc = {
  appInfo: () => call<{ name: string; version: string; slogan: string; phase: string }>('app_info'),
  playerSnapshot: () => call<PlayerSnapshot>('player_snapshot'),
  playerPosition: () => call<number>('player_position'),
  play: () => call<void>('play'),
  pause: () => call<void>('pause'),
  stop: () => call<void>('stop'),
  seek: (positionSeconds: number) => call<void>('seek', { positionSeconds }),
  trackIdByPath: (path: string) => call<number | null>('track_id_by_path', { path }),
  trackArtwork: (trackId: number) => call<string | null>('track_artwork', { trackId }),
  setMica: (enabled: boolean, persist: boolean) => call<void>('set_mica', { enabled, persist }),
  setVolume: (volume: number) => call<void>('set_volume', { volume }),
  playTrack: (trackId: number) => call<void>('play_track', { trackId }),
  // §275/§276：Open File/URL 统一经 M4 resolver（Rust 侧），frontend 不 fetch。
  playInput: (input: string) => call<string>('play_input', { input }),
  setCrossfade: (seconds: number, persist: boolean) => call<void>('set_crossfade', { seconds, persist }),
  setLoudness: (enabled: boolean, persist: boolean) => call<void>('set_loudness', { enabled, persist }),
  setEqPreset: (preset: number, persist: boolean) => call<void>('set_eq_preset', { preset, persist }),
  libraryList: (sort: SortKey, limit: number, offset: number) =>
    call<TrackSummary[]>('library_list', { sort, limit, offset }),
  librarySearch: (query: string, limit: number, offset: number) =>
    call<TrackSummary[]>('library_search', { query, limit, offset }),
  scanFolder: (path: string) => call<void>('scan_folder', { path }),
  cancelScan: () => call<void>('cancel_scan'),
  libraryStats: () => call<{ tracks: number; artists: number; albums: number }>('library_stats'),
  getSettings: () => call<AppSettings>('get_settings'),
  saveSettings: (settings: AppSettings) => call<void>('save_settings', { settings }),
  listenScanProgress: (cb: (p: ScanProgress) => void) =>
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<ScanProgress>('scan-progress', (ev) => cb(ev.payload)),
    ),
  listenLibraryChanged: (cb: () => void) =>
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen('library-changed', () => cb()),
    ),
  // ---- Dreamscape（M7；§255/§263：命令 + 按需拉取，无轮询循环）----
  dreamscapeState: () => call<DreamscapeStateDto>('dreamscape_state'),
  // §256/§257：幂等 open/close（重复调用无副作用）。
  dreamscapeOpen: () => call<DreamscapeStateDto>('dreamscape_open'),
  dreamscapeClose: () => call<DreamscapeStateDto>('dreamscape_close'),
  dreamscapeSetQuality: (quality: 'low' | 'medium' | 'high' | 'auto') =>
    call<void>('dreamscape_set_quality', { quality }),
  dreamscapeSetEnvironment: (frozen: boolean) =>
    call<DreamscapeStateDto>('dreamscape_set_environment', { frozen }),
  dreamscapeAdvance: (ticks: number) => call<DreamscapeStateDto>('dreamscape_advance', { ticks }),
  // §253：manifest 经 validation-first 守卫，畸形数据不进渲染层。
  dreamscapeManifest: async (): Promise<SceneManifest> => {
    const m = await call<unknown>('dreamscape_manifest');
    if (!isSceneManifest(m)) throw { code: 'DREAMSCAPE_CORRUPT', message: 'invalid manifest', recoverable: false };
    return m;
  },
  dreamscapeRegenerate: (input: string) => call<DreamscapeStateDto>('dreamscape_regenerate', { input }),
  dreamscapeExportSeed: () => call<string>('dreamscape_export_seed'),
  dreamscapeImportSeed: (input: string) => call<DreamscapeStateDto>('dreamscape_import_seed', { input }),
  dreamscapeSaveSnapshot: () => call<void>('dreamscape_save_snapshot'),
  dreamscapeRestoreSnapshot: () => call<DreamscapeStateDto>('dreamscape_restore_snapshot'),
  dreamscapeSetCamera: (yaw: number, pitch: number, distance: number) =>
    call<void>('dreamscape_set_camera', { yaw, pitch, distance }),
  // ---- Music Map（M9 中篇；§234 状态握手 → 按需取数，§158/§160
  // hover 不产生 IPC）----
  // §234：先 status 再数据；畸形响应拒绝进渲染层（§253 同精神）。
  mapStatus: async (): Promise<MapStatusDto> => {
    const s = await call<unknown>('get_map_status');
    if (!isMapStatusDto(s)) {
      throw { code: 'MAP_CORRUPT', message: 'invalid map status', recoverable: false };
    }
    return s;
  },
  mapOverview: async (): Promise<MapOverviewDto> => {
    const o = await call<unknown>('get_map_overview');
    if (!isMapOverviewDto(o)) {
      throw { code: 'MAP_CORRUPT', message: 'invalid map overview', recoverable: false };
    }
    return o;
  },
  // §191/§190：逐节点校验，非有限位置节点丢弃（不让 renderer 变 NaN）。
  mapNodes: async (
    minX: number | null,
    minY: number | null,
    maxX: number | null,
    maxY: number | null,
    limit: number | null,
  ): Promise<MapNodeDto[]> => {
    const rows = await call<unknown[]>('get_map_nodes', { minX, minY, maxX, maxY, limit });
    return Array.isArray(rows) ? rows.filter(isMapNodeDto) : [];
  },
  mapClusters: async (): Promise<MapClusterDto[]> => {
    const rows = await call<unknown[]>('get_map_clusters');
    return Array.isArray(rows) ? (rows as MapClusterDto[]) : [];
  },
  // §126/§159：真选中才调用。
  mapNeighbors: async (trackId: number, k: number | null): Promise<MapNeighborDto[]> => {
    const rows = await call<unknown[]>('get_map_neighbors', { trackId, k });
    return Array.isArray(rows) ? rows.filter(isMapNeighborDto) : [];
  },
  mapTrack: async (trackId: number): Promise<MapNodeDto | null> => {
    const n = await call<unknown>('get_map_track', { trackId });
    return n === null || n === undefined ? null : isMapNodeDto(n) ? n : null;
  },
  // §236 用户显式构建；§164 取消。
  requestMapBuild: () => call<number>('request_map_build'),
  cancelMapBuild: () => call<void>('cancel_map_build'),
  // ---- M10 Windows Integration 桥（§65/§153/§23/§24）----
  // queue 边界上报（SMTC/托盘 enablement 数据源）。
  reportQueueState: (hasPrevious: boolean, hasNext: boolean) =>
    call<void>('report_queue_state', { hasPrevious, hasNext }),
  listenIntegrationCommand: (cb: (cmd: 'next' | 'previous') => void) =>
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<{ cmd: string }>('integration://command', (ev) => {
        if (ev.payload.cmd === 'next' || ev.payload.cmd === 'previous') cb(ev.payload.cmd);
      }),
    ),
  listenIntegrationNavigate: (cb: (view: string) => void) =>
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<{ view: string }>('integration://navigate', (ev) => cb(ev.payload.view)),
    ),
};

export function isDreamscapeStateDto(v: unknown): v is DreamscapeStateDto {
  return isDreamscapeState(v);
}
