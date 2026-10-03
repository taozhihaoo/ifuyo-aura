// §189：Rust 错误码 → i18n key 映射（组件不得 switch(error.message)）。
import { describe, expect, it } from 'vitest';
import { errorToI18nKey, isRecoverableError } from './errors';
import { translate } from '../i18n';

const appErr = (code: string, message = 'x', recoverable = true) => ({ code, message, recoverable });

describe('errorToI18nKey（§189）', () => {
  it('已知错误码全部映射到存在的 i18n key', () => {
    const codes = [
      'LIBRARY_DB_UNAVAILABLE',
      'TRACK_UNAVAILABLE',
      'AUDIO_UNAVAILABLE',
      'SOURCE_TIMEOUT',
      'SOURCE_UNAVAILABLE',
      'SOURCE_BLOCKED',
      'UNSUPPORTED_SOURCE',
      'CACHE_FULL_PINNED',
      'SCAN_RUNNING',
      'SETTINGS_WRITE',
      'CANCELLED',
    ];
    for (const code of codes) {
      const key = errorToI18nKey(appErr(code));
      expect(key).not.toBe('error.internal');
      // 两种语言都能翻译（不回退 key 本身）。
      expect(translate('zh-CN', key)).not.toBe(key);
      expect(translate('en', key)).not.toBe(key);
    }
  });

  it('未知/畸形错误 → error.internal，且可翻译', () => {
    expect(errorToI18nKey(appErr('SOMETHING_NEW'))).toBe('error.internal');
    expect(errorToI18nKey(null)).toBe('error.internal');
    expect(errorToI18nKey('plain string')).toBe('error.internal');
    expect(translate('zh-CN', 'error.internal')).not.toBe('error.internal');
  });
});

describe('isRecoverableError', () => {
  it('recoverable=false 不可恢复；缺省视为可恢复', () => {
    expect(isRecoverableError(appErr('X', 'x', false))).toBe(false);
    expect(isRecoverableError(appErr('X', 'x', true))).toBe(true);
    expect(isRecoverableError({})).toBe(true);
  });
});
