// §277 renderer-failure fallback + §188 reinit 卫生 + §189 M6 集成 +
// §287 reducedMotion。jsdom 无 WebGL → create() 失败即真实 fallback 路径。
// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, render } from '@testing-library/react';
import { createElement } from 'react';

const ipcMocks = vi.hoisted(() => ({
  dreamscapeManifest: vi.fn(async () => {
    throw { code: 'DREAMSCAPE_CORRUPT', message: 'boom', recoverable: false };
  }),
  dreamscapeState: vi.fn(async () => null),
  dreamscapeOpen: vi.fn(async () => null),
  dreamscapeClose: vi.fn(async () => null),
  dreamscapeAdvance: vi.fn(async () => null),
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
    play: vi.fn(async () => undefined),
    pause: vi.fn(async () => undefined),
    stop: vi.fn(async () => undefined),
    setVolume: vi.fn(async () => undefined),
    seek: vi.fn(async () => undefined),
    setMica: vi.fn(async () => undefined),
    listenScanProgress: vi.fn(async () => vi.fn()),
    listenLibraryChanged: vi.fn(async () => vi.fn()),
    reportQueueState: vi.fn(async () => undefined),
    listenIntegrationCommand: vi.fn(async () => vi.fn()),
    listenTrackEnded: vi.fn(async () => vi.fn()),
    listenIntegrationNavigate: vi.fn(async () => vi.fn()),
    playerLevel: vi.fn(async () => null),
    playerPosition: vi.fn(async () => 0),
    trackFeatures: vi.fn(async () => null),

  },
}));

const winMocks = vi.hoisted(() => ({
  minimize: vi.fn(async () => undefined),
  toggleMaximize: vi.fn(async () => undefined),
  close: vi.fn(async () => undefined),
}));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => winMocks }));

import App from '../App';
import { LightFieldRenderer } from '../visual/dreamscape/LightFieldRenderer';
import { DREAMSCAPE_ENABLED, useUiStore } from '../state/stores';

afterEach(() => {
  cleanup();
  useUiStore.setState({
    view: 'now-playing',
    commandPaletteOpen: false,
    queueOpen: false,
    settingsOpen: false,
    miniPlayerOpen: false,
    notice: null,
    dreamscapeOpen: false,
    dreamscapeStatus: 'idle',
  });
  vi.clearAllMocks();
});

describe('§277 renderer 失败 fallback', () => {
  it('jsdom 无 WebGL2：create() 返回 null 且不抛异常（§278 不悬挂 loading）', () => {
    const canvas = document.createElement('canvas');
    expect(() => LightFieldRenderer.create(canvas)).not.toThrow();
    expect(LightFieldRenderer.create(canvas)).toBeNull();
  });

  it('manifest 加载失败 → fallback 状态 + 不白屏（§275）', async () => {
    const { DreamscapeView } = await import('../features/dreamscape/DreamscapeView');
    const view = render(createElement(DreamscapeView));
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });
    expect(useUiStore.getState().dreamscapeStatus).toBe('fallback');
    // §275：非白屏——容器有 fallback 提示文本。
    expect(view.container.textContent).toContain('暂时无法呈现');
    view.unmount();
    expect(useUiStore.getState().dreamscapeStatus).toBe('idle');
    expect(ipcMocks.dreamscapeClose).toHaveBeenCalled(); // §257 卸载接线
  });
});

// 功能停用（DREAMSCAPE_ENABLED=false）时 App 壳不再渲染 Dreamscape，
// 这组经 shell 导航的集成测试按事实跳过（渲染器自身测试在上方独立跑）。
describe.skipIf(!DREAMSCAPE_ENABLED)('§189 M6 集成', () => {
  it('导航进入 Dreamscape：Aurora 不挂载（§130/§131）；退出恢复', async () => {
    const view = render(createElement(App));
    // 初始 now-playing → Aurora 存在（aria-hidden canvas 或 fallback div）。
    expect(view.container.querySelector('.aurora-canvas, .aurora-fallback')).toBeTruthy();
    act(() => useUiStore.getState().setView('dreamscape'));
    // Dreamscape 视图激活：Aurora 移除（§130 单高负载 WebGL 规则）。
    expect(view.container.querySelector('.dreamscape')).toBeTruthy();
    expect(view.container.querySelector('.aurora-canvas')).toBeNull();
    // Esc 退出（§198/§285）。
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    });
    expect(useUiStore.getState().view).toBe('now-playing');
    expect(view.container.querySelector('.aurora-canvas, .aurora-fallback')).toBeTruthy();
  });

  it('进入/退出 Dreamscape 不触碰播放命令（§190 playback continuity）', async () => {
    render(createElement(App));
    act(() => useUiStore.getState().setView('dreamscape'));
    act(() => useUiStore.getState().setView('now-playing'));
    // 无 play/pause/stop/seek 调用（切换视图 ≠ 播放操作）。
    expect(ipcMocks.playerSnapshot).toHaveBeenCalled(); // 常规快照轮询
    // 播放命令零调用：
    const { ipc } = vi.mocked(await import('../ipc/client')) as unknown as {
      ipc: Record<string, ReturnType<typeof vi.fn>>;
    };
    expect(ipc.play).not.toHaveBeenCalled();
    expect(ipc.pause).not.toHaveBeenCalled();
    expect(ipc.stop).not.toHaveBeenCalled();
    expect(ipc.seek).not.toHaveBeenCalled();
  });



  it('×20 进入/退出循环无残留崩溃（§188 reinit 卫生/§248 短循环）', () => {
    const view = render(createElement(App));
    for (let i = 0; i < 20; i += 1) {
      act(() => useUiStore.getState().setView('dreamscape'));
      expect(view.container.querySelector('.dreamscape')).toBeTruthy();
      act(() => useUiStore.getState().setView('now-playing'));
    }
    expect(view.container.querySelector('.app-root')).toBeTruthy();
  });
});
