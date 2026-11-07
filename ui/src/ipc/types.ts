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
  /** Rust 侧扫描失败原因；null/缺失表示成功。done 只代表「本轮结束」。 */
  error?: string | null;
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
  return (
    typeof o.code === 'string' &&
    typeof o.message === 'string' &&
    typeof o.recoverable === 'boolean'
  );
}

/**
 * IPC 设置契约守卫（§11 不信任 IPC 返回值）。
 *
 * 后端 AppSettings 曾缺 `serde(rename_all = "camelCase")`，返回 snake_case
 * 时前端会把一整包 undefined 写进 store（uiScale → NaNpx、自动更新静默
 * 失效），而单测/E2E 都绕过了真实消费端没抓到。此处按 camelCase 逐字段
 * 校验，契约再漂移时明确抛错，而不是静默污染状态。
 */
export function isAppSettings(v: unknown): v is AppSettings {
  if (typeof v !== 'object' || v === null) return false;
  const o = v as Record<string, unknown>;
  const num = (k: string) => typeof o[k] === 'number' && Number.isFinite(o[k] as number);
  const bool = (k: string) => typeof o[k] === 'boolean';
  return (
    num('version') &&
    typeof o.language === 'string' &&
    bool('updatesAutoCheck') &&
    bool('mojibakeFix') &&
    bool('analysisAuto') &&
    num('uiScale') &&
    bool('reducedMotion') &&
    bool('loudnessEnabled') &&
    num('crossfadeSeconds') &&
    bool('eqEnabled') &&
    num('eqPreset') &&
    bool('micaEnabled')
  );
}
