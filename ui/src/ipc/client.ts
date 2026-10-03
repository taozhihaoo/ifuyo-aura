// Typed IPC client（§11）：invoke 封装 + AppError 归一。
import { invoke } from '@tauri-apps/api/core';
import { isAppError, type AppSettings, type PlayerSnapshot, type ScanProgress, type SortKey, type TrackSummary } from './types';

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
  setVolume: (volume: number) => call<void>('set_volume', { volume }),
  playTrack: (trackId: number) => call<void>('play_track', { trackId }),
  setCrossfade: (seconds: number, persist: boolean) => call<void>('set_crossfade', { seconds, persist }),
  setLoudness: (enabled: boolean, persist: boolean) => call<void>('set_loudness', { enabled, persist }),
  setEqPreset: (preset: number, persist: boolean) => call<void>('set_eq_preset', { preset, persist }),
  libraryList: (sort: SortKey, limit: number, offset: number) =>
    call<TrackSummary[]>('library_list', { sort, limit, offset }),
  librarySearch: (query: string, limit: number, offset: number) =>
    call<TrackSummary[]>('library_search', { query, limit, offset }),
  scanFolder: (path: string) => call<void>('scan_folder', { path }),
  cancelScan: () => call<void>('cancel_scan'),
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
};
