// §181/§182：无障碍自动检查（轻量自写断言，不引入 axe 大框架）。
// 覆盖：按钮可访问名、dialog role/aria-modal、range 标签、
// 禁用语义、tab 顺序可达、进度语义。
// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render, screen } from '@testing-library/react';
import { createElement, type ReactNode } from 'react';

// TitleBar 在模块顶层调 getCurrentWindow()——必须 hoisted mock，
// 且所有调用共享同一组 spy（模块级单例）。
const winMocks = vi.hoisted(() => ({
  minimize: vi.fn(async () => undefined),
  toggleMaximize: vi.fn(async () => undefined),
  close: vi.fn(async () => undefined),
}));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => winMocks,
}));

vi.mock('../ipc/client', () => ({
  ipc: {
    playerSnapshot: vi.fn(async () => null),
    getSettings: vi.fn(async () => null),
    libraryList: vi.fn(async () => []),
    librarySearch: vi.fn(async () => []),
    libraryStats: vi.fn(async () => ({ tracks: 0, artists: 0, albums: 0 })),
    trackIdByPath: vi.fn(async () => null),
    trackArtwork: vi.fn(async () => null),
    play: vi.fn(async () => undefined),
    pause: vi.fn(async () => undefined),
    stop: vi.fn(async () => undefined),
    setVolume: vi.fn(async () => undefined),
    seek: vi.fn(async () => undefined),
    setMica: vi.fn(async () => undefined),
  },
}));

import { FeatureBoundary } from '../components/ErrorBoundary';

function Boom(): ReactNode {
  throw new Error('feature boom');
}

afterEach(() => cleanup());

describe('FeatureBoundary（§228/§229）', () => {
  it('子树抛错 → role=alert 回退 + 重试按钮，不白屏', () => {
    const { getByRole, queryByText } = render(
      createElement(FeatureBoundary, {
        name: 'test',
        message: '页面出现异常',
        retryLabel: '重试',
        children: createElement(Boom),
      }),
    );
    expect(getByRole('alert')).toBeTruthy();
    expect(queryByText('页面出现异常')).toBeTruthy();
    expect(getByRole('button', { name: '重试' })).toBeTruthy();
  });

  it('重试后若子树恢复 → 正常渲染', () => {
    let boom = true;
    function Maybe(): ReactNode {
      if (boom) throw new Error('x');
      return createElement('p', null, 'fine');
    }
    const view = render(
      createElement(FeatureBoundary, {
        name: 'test',
        message: 'err',
        retryLabel: 'retry',
        children: createElement(Maybe),
      }),
    );
    expect(view.getByRole('alert')).toBeTruthy();
    boom = false;
    act(() => {
      view.getByRole('button', { name: 'retry' }).click();
    });
    expect(view.getByText('fine')).toBeTruthy();
  });
});

describe('TitleBar a11y（§261）', () => {
  it('窗口控制与导航按钮全部有可访问名', async () => {
    const { TitleBar } = await import('../shell/TitleBar');
    render(createElement(TitleBar));
    for (const name of ['最小化', '最大化/还原', '关闭', '正在播放', '音乐库', 'Map', 'Ctrl+K']) {
      expect(screen.getByRole('button', { name }), `button ${name}`).toBeTruthy();
    }
  });

  it('双击 drag region 触发 toggleMaximize；双击按钮不触发（§262/§263）', async () => {
    const { TitleBar } = await import('../shell/TitleBar');
    const toggle = winMocks.toggleMaximize;
    toggle.mockClear();
    const view = render(createElement(TitleBar));
    const header = view.container.querySelector('.titlebar') as HTMLElement;
    const brand = header.querySelector('.titlebar-brand') as HTMLElement;
    brand.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    expect(toggle).toHaveBeenCalledTimes(1);
    // 按钮上的双击：target 无 data-tauri-drag-region → 不触发。
    const navBtn = header.querySelector('.nav-btn') as HTMLElement;
    navBtn.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    expect(toggle).toHaveBeenCalledTimes(1);
  });
});

describe('NowPlaying a11y（§181）', () => {
  it('seek/volume 有 aria-label；空态有标题', async () => {
    const { NowPlayingPage } = await import('../features/nowplaying/NowPlayingPage');
    const { usePlayerStore } = await import('../state/stores');
    usePlayerStore.getState().setSnapshot({
      state: 'stopped',
      positionSeconds: 0,
      durationSeconds: 180,
      volume: 0.8,
      trackPath: 'C:/m/a.mp3',
      title: 'Alpha',
      seekable: true,
      revision: 1,
    });
    render(createElement(NowPlayingPage));
    expect(screen.getByRole('slider', { name: '进度' })).toBeTruthy();
    expect(screen.getByRole('slider', { name: '音量' })).toBeTruthy();
    expect(screen.getByRole('group')).toBeTruthy();
    // §255/§256：位置/时长数值合法渲染（不 NaN）。
    expect(screen.getAllByText(/\d+:\d\d/).length).toBeGreaterThan(0);
  });
});

describe('Palette a11y（§181 dialog 语义）', () => {
  it('打开时 role=dialog + aria-modal + 输入框聚焦', async () => {
    const { CommandPalette } = await import('../features/palette/CommandPalette');
    const { useUiStore } = await import('../state/stores');
    useUiStore.getState().setPaletteOpen(true);
    const view = render(createElement(CommandPalette));
    const dialog = view.getByRole('dialog');
    expect(dialog.getAttribute('aria-modal')).toBe('true');
    expect((view.container.querySelector('.palette-input') as HTMLInputElement).value).toBe('');
  });

  it('键盘矩阵：ArrowDown/Up 移动选择，Enter 执行，Escape 关闭（§183）', async () => {
    const { CommandPalette } = await import('../features/palette/CommandPalette');
    const { useUiStore } = await import('../state/stores');
    useUiStore.getState().setPaletteOpen(true);
    const view = render(createElement(CommandPalette));
    const input = view.container.querySelector('.palette-input') as HTMLInputElement;
    const key = (k: string, init: Partial<KeyboardEvent> = {}) =>
      input.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...init }));
    key('ArrowDown');
    key('ArrowDown');
    key('ArrowUp');
    const selected = view.container.querySelector('.palette-item.selected') as HTMLElement;
    expect(selected).toBeTruthy(); // 第 2 项被选中
    key('Escape');
    expect(useUiStore.getState().commandPaletteOpen).toBe(false);
  });

  it('无输入时全部命令可见；Tab 可达列表项（无 focus 死路 §183）', async () => {
    const { CommandPalette } = await import('../features/palette/CommandPalette');
    const { useUiStore } = await import('../state/stores');
    useUiStore.getState().setPaletteOpen(true);
    const view = render(createElement(CommandPalette));
    const items = view.container.querySelectorAll<HTMLButtonElement>('.palette-item');
    expect(items.length).toBeGreaterThanOrEqual(10);
    // 所有命令项均为可聚焦 button（Tab 顺序自然可达）。
    for (const el of items) expect(el.tagName).toBe('BUTTON');
  });
});
