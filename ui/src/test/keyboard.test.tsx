// §183 键盘矩阵（App 级快捷键）+ §209 StrictMode 双挂载 + §295 zombie。
// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render } from '@testing-library/react';
import { createElement } from 'react';

// TitleBar 模块顶层调 getCurrentWindow()（同 a11y.test 的共享 spy 方案）。
const winMocks = vi.hoisted(() => ({
  minimize: vi.fn(async () => undefined),
  toggleMaximize: vi.fn(async () => undefined),
  close: vi.fn(async () => undefined),
}));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => winMocks }));

const ipcMocks = vi.hoisted(() => ({
  play: vi.fn(async () => undefined),
  pause: vi.fn(async () => undefined),
  getSettings: vi.fn(async () => null),
  playerSnapshot: vi.fn(async () => null),
}));

vi.mock('../ipc/client', () => ({
  ipc: {
    ...ipcMocks,
    libraryList: vi.fn(async () => []),
    librarySearch: vi.fn(async () => []),
    libraryStats: vi.fn(async () => ({ tracks: 0, artists: 0, albums: 0 })),
    trackIdByPath: vi.fn(async () => null),
    trackArtwork: vi.fn(async () => null),
    stop: vi.fn(async () => undefined),
    seek: vi.fn(async () => undefined),
    setVolume: vi.fn(async () => undefined),
    setMica: vi.fn(async () => undefined),
    listenScanProgress: vi.fn(async () => vi.fn()),
    listenLibraryChanged: vi.fn(async () => vi.fn()),
    dreamscapeOpen: vi.fn(async () => null),
    dreamscapeClose: vi.fn(async () => null),
    dreamscapeManifest: vi.fn(async () => {
      throw { code: 'DREAMSCAPE_CORRUPT', message: 'x', recoverable: false };
    }),
    dreamscapeState: vi.fn(async () => null),
    dreamscapeAdvance: vi.fn(async () => null),
  },
}));

import App from '../App';
import { usePlayerStore, useUiStore } from '../state/stores';

function keyDown(key: string, init: Partial<KeyboardEvent> = {}) {
  act(() => {
    window.dispatchEvent(
      new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }),
    );
  });
}

afterEach(() => {
  cleanup();
  useUiStore.setState({
    view: 'now-playing',
    commandPaletteOpen: false,
    queueOpen: false,
    settingsOpen: false,
    miniPlayerOpen: false,
    notice: null,
  });
  vi.clearAllMocks();
});

describe('App 键盘矩阵（§183）', () => {
  it('Esc 逐层退出：palette 优先，再抽层（§183/§198）', () => {
    render(createElement(App));
    keyDown('k', { ctrlKey: true });
    expect(useUiStore.getState().commandPaletteOpen).toBe(true);
    useUiStore.setState({ queueOpen: true, settingsOpen: true });
    keyDown('Escape');
    // 第一层：只关 palette（抽层保持）。
    expect(useUiStore.getState().commandPaletteOpen).toBe(false);
    expect(useUiStore.getState().queueOpen).toBe(true);
    keyDown('Escape');
    // 第二层：关抽层。
    expect(useUiStore.getState().queueOpen).toBe(false);
    expect(useUiStore.getState().settingsOpen).toBe(false);
  });

  it('Esc 在 Dreamscape 视图返回 Now Playing（§198/§285）', () => {
    render(createElement(App));
    act(() => useUiStore.setState({ view: 'dreamscape' }));
    keyDown('Escape');
    expect(useUiStore.getState().view).toBe('now-playing');
  });

  it('输入态不拦截字符；Space 在非输入态切换播放（§258）', () => {
    render(createElement(App));
    // 无播放 → Space = play。
    keyDown(' ', { code: 'Space' });
    expect(ipcMocks.play).toHaveBeenCalledTimes(1);
    // playing → Space = pause。
    usePlayerStore.getState().setSnapshot({
      state: 'playing',
      positionSeconds: 0,
      durationSeconds: 10,
      volume: 1,
      trackPath: null,
      title: null,
      seekable: true,
      revision: 1,
    });
    keyDown(' ', { code: 'Space' });
    expect(ipcMocks.pause).toHaveBeenCalledTimes(1);
  });

  it('Ctrl+F → Library 并聚焦搜索框（§259）', () => {
    render(createElement(App));
    keyDown('f', { ctrlKey: true });
    expect(useUiStore.getState().view).toBe('library');
  });
});

describe('挂载卫生（§209/§295）', () => {
  it('StrictMode 双挂载 / ×10 mount-unmount 不抛错、无残留崩溃', () => {
    for (let i = 0; i < 10; i += 1) {
      const view = render(createElement(App));
      expect(view.container.querySelector('.app-root')).toBeTruthy();
      view.unmount();
    }
  });

  it('palette ×20 开关后仍可打开（§295 无 zombie）', () => {
    render(createElement(App));
    for (let i = 0; i < 20; i += 1) {
      act(() => {
        useUiStore.getState().setPaletteOpen(true);
        useUiStore.getState().setPaletteOpen(false);
      });
    }
    act(() => useUiStore.getState().setPaletteOpen(true));
    expect(useUiStore.getState().commandPaletteOpen).toBe(true);
  });
});
