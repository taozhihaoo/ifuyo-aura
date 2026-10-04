// Typed IPC client（§11）：invoke 封装 + AppError 归一。
import { invoke } from '@tauri-apps/api/core';
import { isAppError, type AppSettings, type PlayerSnapshot, type ScanProgress, type SortKey, type TrackSummary } from './types';
import { isDreamscapeState, isSceneManifest, type DreamscapeStateDto, type SceneManifest } from './dreamscape';

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
};

export function isDreamscapeStateDto(v: unknown): v is DreamscapeStateDto {
  return isDreamscapeState(v);
}
