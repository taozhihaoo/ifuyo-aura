// ifuyo 远程清单 DTO 校验面（§11 IPC 不可信）。
// 样例形状取自线上真实清单（含无标签曲目：duration/artist 为 null）。
import { describe, expect, it } from 'vitest';
import { fmtDuration, isIfuyoList } from './ifuyo';

const track = {
  id: '%E7%8E%B0.mp3',
  title: '现在那边是几点',
  artist: '黄小琥',
  album: null,
  url: 'https://ifuyo.com/audio/%E7%8E%B0.mp3',
  durationSeconds: 268.4,
  sizeBytes: 10738628,
  format: 'mp3',
  etag: 'a3dbc4-6ac9db16',
};
/** 无标签占位曲目：duration/artist 为 null 是线上真实形态。 */
const bare = { ...track, id: 'test.mp3', title: 'test', artist: null, durationSeconds: null };
const okList = {
  generatedAt: '2026-10-10T07:04:08+00:00',
  tracks: [bare, track],
  skipped: 0,
  truncated: false,
};

describe('isIfuyoList', () => {
  it('接受线上真实形状', () => {
    expect(isIfuyoList(okList)).toBe(true);
  });

  it('拒绝缺字段/类型漂移', () => {
    expect(isIfuyoList(null)).toBe(false);
    expect(isIfuyoList({ ...okList, tracks: null })).toBe(false);
    expect(isIfuyoList({ ...okList, skipped: undefined })).toBe(false);
    expect(isIfuyoList({ ...okList, truncated: 'false' })).toBe(false);
    expect(isIfuyoList({ ...okList, tracks: [{ ...track, title: undefined }] })).toBe(false);
  });

  /// 非 http(s) 的曲目地址一律不合格（file:/blob: 不该出现在远程清单里）。
  it('拒绝非 http(s) 地址', () => {
    for (const bad of ['file:///C:/x.mp3', 'blob:https://x/y', 'ftp://x/y', '', 42, null]) {
      expect(isIfuyoList({ ...okList, tracks: [{ ...track, url: bad }] })).toBe(false);
    }
  });

  it('拒绝 NaN 时长（会让 UI 显示 NaN:NaN）', () => {
    expect(
      isIfuyoList({ ...okList, tracks: [{ ...track, durationSeconds: Number.NaN }] }),
    ).toBe(false);
  });
});

describe('fmtDuration', () => {
  it('null/负数/NaN 都是 --:--，不伪造 0:00', () => {
    expect(fmtDuration(null)).toBe('--:--');
    expect(fmtDuration(-1)).toBe('--:--');
    expect(fmtDuration(Number.NaN)).toBe('--:--');
  });

  it('整分与秒进位', () => {
    expect(fmtDuration(0)).toBe('0:00');
    expect(fmtDuration(61.4)).toBe('1:01');
    expect(fmtDuration(268.4)).toBe('4:28');
  });
});
