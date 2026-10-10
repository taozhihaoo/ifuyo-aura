// Typed IPC 契约（§11/§12）：与 src-tauri/src/ipc.rs 的 DTO 一一镜像。
// 禁止 any / Record<string, any> 成为业务 DTO。

export type PlaybackStateName = 'idle' | 'stopped' | 'playing' | 'paused';

export interface PlayerSnapshot {
  state: PlaybackStateName;
  positionSeconds: number;
  durationSeconds: number | null;
  volume: number;
  trackPath: string | null;
  title: string | null;
  seekable: boolean;
  revision: number;
}

export interface TrackSummary {
  id: number;
  title: string | null;
  artist: string | null;
  album: string | null;
  durationSeconds: number | null;
  year: number | null;
  path: string;
  hasArtwork: boolean;
}

export interface ScanProgress {
  discovered: number;
  processed: number;
  imported: number;
  failed: number;
  currentPath: string;
  done: boolean;
}

export interface AppError {
  code: string;
  message: string;
  recoverable: boolean;
}

export interface AppSettings {
  version: number;
  language: string;
  updatesAutoCheck: boolean;
  mojibakeFix: boolean;
  analysisAuto: boolean;
  uiScale: number;
  reducedMotion: boolean;
  loudnessEnabled: boolean;
  crossfadeSeconds: number;
  eqEnabled: boolean;
  eqPreset: number;
  micaEnabled: boolean;
}

/** 实时电平（驱动线程每轮刷新；0..~0.7 未归一能量）。 */
export interface PlayerLevel {
  rms: number;
  low: number;
  mid: number;
  high: number;
}

/** 可视化消费的曲目特征（未分析字段一律 null，不伪造）。 */
export interface TrackFeatures {
  bpm: number | null;
  firstBeatOffsetSeconds: number | null;
  energy: number | null;
  brightness: number | null;
}

/** M8 分析进度（shell worker → `analysis-progress` 事件）。 */
export interface AnalysisProgress {
  processed: number;
  total: number;
  failed: number;
  running: boolean;
}

/** M8 分析状态：进度 + 曲库覆盖度。 */
export interface AnalysisStatus {
  progress: AnalysisProgress;
  analyzed: number;
  pending: number;
}

export type SortKey =
  | 'title'
  | 'artist'
  | 'album'
  | 'dateAdded'
  | 'duration'
  | 'year';

export function isTrackSummary(v: unknown): v is TrackSummary {
  if (typeof v !== 'object' || v === null) return false;
  const o = v as Record<string, unknown>;
  return typeof o.id === 'number' && typeof o.path === 'string';
}

export function isAppError(v: unknown): v is AppError {
  if (typeof v !== 'object' || v === null) return false;
  const o = v as Record<string, unknown>;
  return typeof o.code === 'string' && typeof o.message === 'string';
}
