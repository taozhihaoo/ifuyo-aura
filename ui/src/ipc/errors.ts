// §189：Rust Error Code → Application Error → Localized UI Message。
// 组件禁止 switch(error.message)；统一经此映射到 i18n key，
// 未知 code 回退 error.internal（不向用户暴露内部串）。
import { useUiStore } from '../state/stores';

const CODE_TO_KEY: Record<string, string> = {
  LIBRARY_DB_UNAVAILABLE: 'error.libraryDb',
  TRACK_UNAVAILABLE: 'error.trackUnavailable',
  AUDIO_UNAVAILABLE: 'error.audioUnavailable',
  SOURCE_TIMEOUT: 'error.sourceTimeout',
  SOURCE_UNAVAILABLE: 'error.sourceUnavailable',
  SOURCE_BLOCKED: 'error.sourceBlocked',
  UNSUPPORTED_SOURCE: 'error.unsupportedSource',
  CACHE_FULL_PINNED: 'error.cacheFullPinned',
  IFUYO_UNAVAILABLE: 'error.ifuyoUnavailable',
  IFUYO_MANIFEST: 'error.ifuyoManifest',
  SCAN_RUNNING: 'error.scanRunning',
  SETTINGS_WRITE: 'error.settingsWrite',
  SETTINGS_CORRUPT: 'error.settingsCorrupt',
  MAP_CORRUPT: 'error.mapCorrupt',
  INVALID_PARAMETER: 'error.invalidParameter',
  SCAN_FAILED: 'error.scanFailed',
  DREAMSCAPE_PERSIST: 'error.dreamscapePersist',
  DREAMSCAPE_LOCK: 'error.dreamscapeLock',
  CANCELLED: 'error.cancelled',
  INVALID_SEED: 'error.invalidSeed',
  DREAMSCAPE_VERSION_UNAVAILABLE: 'error.dreamscapeVersion',
  DREAMSCAPE_CORRUPT: 'error.dreamscapeCorrupt',
  DREAMSCAPE_NO_SNAPSHOT: 'error.dreamscapeNoSnapshot',
  INTERNAL: 'error.internal',
};

export function errorToI18nKey(err: unknown): string {
  const code = (err as { code?: string } | null | undefined)?.code;
  return (code && CODE_TO_KEY[code]) || 'error.internal';
}

export function isRecoverableError(err: unknown): boolean {
  const v = (err as { recoverable?: boolean } | null | undefined)?.recoverable;
  return v !== false;
}

/**
 * 用户主动触发的 IPC 操作统一错误出口。
 *
 * 此前大量按钮写 `.catch(() => undefined)`，失败时「点了没反应」且无任何
 * 诊断。高频轮询 / 幂等占位 / 窗口显隐仍应静默（否则会刷屏），只把用户
 * 点击类接到这里。
 */
export function reportIpcError(err: unknown): void {
  // 延迟 require 式引用避免与调用方的 store import 形成静态环；
  // stores.ts 本身不依赖本模块，静态导入同样安全。
  useUiStore.getState().setNotice(errorToI18nKey(err), 'error');
}
