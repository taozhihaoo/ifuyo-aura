// IfuyoPage 状态面（加载/就绪/失败/不完整上报）与点击播放走的是哪个命令。
// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { IfuyoPage } from '../features/ifuyo/IfuyoPage';
import { useUiStore } from '../state/stores';

const mocks = vi.hoisted(() => ({
  ifuyoList: vi.fn(),
  ifuyoPlay: vi.fn(async () => 'ok'),
}));

vi.mock('../ipc/client', () => ({
  ipc: {
    ifuyoList: mocks.ifuyoList,
    ifuyoPlay: mocks.ifuyoPlay,
    playerSnapshot: vi.fn(async () => null),
  },
}));
const list = {
  generatedAt: '2026-10-10T07:04:08+00:00',
  tracks: [
    {
      id: 'a',
      title: '青花瓷',
      artist: '周杰伦',
      album: null,
      url: 'https://ifuyo.com/audio/a.mp3',
      durationSeconds: 268.4,
      sizeBytes: 1,
      format: 'mp3',
      etag: null,
    },
    {
      id: 'b',
      title: 'test',
      artist: null,
      album: null,
      url: 'https://ifuyo.com/audio/b.mp3',
      durationSeconds: null,
      sizeBytes: 1,
      format: 'mp3',
      etag: null,
    },
  ],
  skipped: 2,
  truncated: true,
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('IfuyoPage', () => {
  it('就绪态列出曲目，null 时长显示 --:-- 而不是 0:00', async () => {
    mocks.ifuyoList.mockResolvedValue(list);
    render(<IfuyoPage />);
    await waitFor(() => expect(screen.getByText('青花瓷')).toBeTruthy());
    expect(screen.getByText('周杰伦')).toBeTruthy();
    // 无标签曲目：--:--（伪造 0:00 会让人以为曲子是空的）
    expect(screen.getByText('--:--')).toBeTruthy();
    expect(screen.queryByText('0:00')).toBeNull();
  });

  it('被丢弃与截断必须看得见', async () => {
    mocks.ifuyoList.mockResolvedValue(list);
    render(<IfuyoPage />);
    await waitFor(() => expect(screen.getByText('青花瓷')).toBeTruthy());
    const note = screen.getByText(/跳过/);
    expect(note.textContent).toContain('2');
    expect(note.textContent).toContain('5000');
  });

  it('失败态给重试入口，不白屏', async () => {
    mocks.ifuyoList.mockRejectedValue({ code: 'IFUYO_UNAVAILABLE', message: 'x', recoverable: true });
    render(<IfuyoPage />);
    await waitFor(() => expect(screen.getByRole('button', { name: '重试' })).toBeTruthy());
    expect(screen.getByText(/无法获取 ifuyo/)).toBeTruthy();
  });

  it('空清单说"还没有曲目"，不说错误', async () => {
    mocks.ifuyoList.mockResolvedValue({ ...list, tracks: [], skipped: 0, truncated: false });
    render(<IfuyoPage />);
    await waitFor(() => expect(screen.getByText(/还没有曲目/)).toBeTruthy());
    expect(screen.queryByText(/重试/)).toBeNull();
  });

  it('点击曲目走 ifuyo_play（严格策略面），失败时提示而不是静默', async () => {
    mocks.ifuyoList.mockResolvedValue(list);
    mocks.ifuyoPlay.mockRejectedValue({ code: 'SOURCE_BLOCKED', message: 'x', recoverable: false });
    render(<IfuyoPage />);
    await waitFor(() => expect(screen.getByText('青花瓷')).toBeTruthy());
    await act(async () => {
      screen.getByText('青花瓷').closest('button')?.click();
    });
    await waitFor(() =>
      // 标题/艺术家要一起传：否则播放条只能显示 percent-encoded 的 URL 末段
      expect(mocks.ifuyoPlay).toHaveBeenCalledWith(
        'https://ifuyo.com/audio/a.mp3',
        '青花瓷',
        '周杰伦',
      ),
    );
    // 提示写进真实 ui store（不 mock 它，避免漏导出第三个 hook）
    await waitFor(() => expect(useUiStore.getState().notice).toBeTruthy());
  });
});
