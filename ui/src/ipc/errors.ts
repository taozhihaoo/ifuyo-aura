// §189：Rust Error Code → Application Error → Localized UI Message。
// 组件禁止 switch(error.message)；统一经此映射到 i18n key，
// 未知 code 回退 error.internal（不向用户暴露内部串）。

const CODE_TO_KEY: Record<string, string> = {
  LIBRARY_DB_UNAVAILABLE: 'error.libraryDb',
  TRACK_UNAVAILABLE: 'error.trackUnavailable',
  AUDIO_UNAVAILABLE: 'error.audioUnavailable',
  SOURCE_TIMEOUT: 'error.sourceTimeout',
  SOURCE_UNAVAILABLE: 'error.sourceUnavailable',
  SOURCE_BLOCKED: 'error.sourceBlocked',
  UNSUPPORTED_SOURCE: 'error.unsupportedSource',
  CACHE_FULL_PINNED: 'error.cacheFullPinned',
  SCAN_RUNNING: 'error.scanRunning',
  SETTINGS_WRITE: 'error.settingsWrite',
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
