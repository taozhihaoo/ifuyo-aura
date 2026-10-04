// 前端自动化测试（A-36）：stores / i18n / 命令注册 / IPC DTO 守卫 / 插值工具。
import { describe, expect, it, beforeEach } from 'vitest';
import { useUiStore, usePlayerStore, useLibraryStore, useSettingsStore } from './state/stores';
import { translate } from './i18n';
import { buildCommands, matchCommand, type Command } from './commands/registry';
import { isTrackSummary, isAppError } from './ipc/types';
import { visualSeed } from './visual/AuroraCanvas';

const resetStores = () => {
  useUiStore.setState({
    view: 'now-playing',
    commandPaletteOpen: false,
    queueOpen: false,
    settingsOpen: false,
    miniPlayerOpen: false,
    sidebarCollapsed: false,
  });
  useLibraryStore.setState({ query: '', sort: 'title', refreshNonce: 0 });
};

describe('uiStore', () => {
  beforeEach(resetStores);
  it('typed ViewId navigation', () => {
    useUiStore.getState().setView('library');
    expect(useUiStore.getState().view).toBe('library');
    useUiStore.getState().setView('map');
    expect(useUiStore.getState().view).toBe('map');
  });
  it('palette/queue/settings toggles', () => {
    useUiStore.getState().setPaletteOpen(true);
    expect(useUiStore.getState().commandPaletteOpen).toBe(true);
    useUiStore.getState().setQueueOpen(true);
    expect(useUiStore.getState().queueOpen).toBe(true);
    useUiStore.getState().setSettingsOpen(true);
    expect(useUiStore.getState().settingsOpen).toBe(true);
  });
});

describe('playerStore (domain state §8)', () => {
  it('snapshot replaces wholesale', () => {
    usePlayerStore.getState().setSnapshot({
      state: 'playing',
      positionSeconds: 1,
      durationSeconds: 100,
      volume: 0.5,
      trackPath: '/a.mp3',
      title: 'A',
      seekable: true,
      revision: 7,
    });
    expect(usePlayerStore.getState().snapshot?.revision).toBe(7);
  });
});

describe('i18n (§117/118)', () => {
  it('zh-CN primary + en fallback', () => {
    expect(translate('zh-CN', 'nav.library')).toBe('音乐库');
    expect(translate('en', 'nav.library')).toBe('Library');
  });
  it('missing key falls back en → key itself', () => {
    expect(translate('zh-CN', 'nonexistent.key')).toBe('nonexistent.key');
    expect(translate('en', 'nonexistent.key')).toBe('nonexistent.key');
  });
  it('params interpolation (§153 no fake percent)', () => {
    expect(translate('en', 'library.scanProgress', { processed: 5, discovered: 0, imported: 4, failed: 1 })).toContain('Processed 5');
  });
});

describe('command registry (§99–101)', () => {
  const ctx = {
    setView: () => undefined,
    play: () => undefined,
    pause: () => undefined,
    next: () => undefined,
    previous: () => undefined,
    stop: () => undefined,
    openFolder: () => undefined,
    openUrl: () => undefined,
    toggleQueue: () => undefined,
    toggleEq: () => undefined,
    toggleLoudness: () => undefined,
    toggleMiniPlayer: () => undefined,
    focusLibrarySearch: () => undefined,
    isPlaying: () => false,
    openDreamscape: () => undefined,
    regenerateDreamscape: () => undefined,
    copyDreamscapeSeed: () => undefined,
    saveDreamscapeSnapshot: () => undefined,
  };
  const t = (k: string) => k;
  const cmds = buildCommands(ctx, t);

  it('covers minimal command set (§100)', () => {
    const ids = cmds.map((c) => c.id);
    for (const must of [
      'nav.now-playing', 'nav.library', 'nav.map',
      'play.toggle', 'play.next', 'play.previous', 'play.stop',
      'library.open-folder', 'view.queue', 'library.focus-search',
    ]) {
      expect(ids).toContain(must);
    }
  });
  it('fuzzy matching (§101)', () => {
    const first: Command | undefined = cmds[0];
    expect(first).toBeDefined();
    if (first) {
      expect(matchCommand(first, 'play')).toBe(true);
      expect(matchCommand(first, 'zzzz')).toBe(false);
    }
  });
  it('deterministic registration order', () => {
    expect(buildCommands(ctx, t).map((c) => c.id)).toEqual(cmds.map((c) => c.id));
  });
});

describe('DTO guards (§11)', () => {
  it('isTrackSummary', () => {
    expect(isTrackSummary({ id: 1, path: 'x' })).toBe(true);
    expect(isTrackSummary({ id: 'x' })).toBe(false);
    expect(isTrackSummary(null)).toBe(false);
  });
  it('isAppError', () => {
    expect(isAppError({ code: 'X', message: 'y', recoverable: true })).toBe(true);
    expect(isAppError({ code: 'X' })).toBe(false);
  });
});

describe('Aurora visual seed (§73/§74)', () => {
  it('stable per identity, differs across identities', () => {
    expect(visualSeed('track-a')).toBe(visualSeed('track-a'));
    expect(visualSeed('track-a')).not.toBe(visualSeed('track-b'));
  });
});

describe('settings store defaults (§114)', () => {
  it('language default zh-CN, scale 1.0', () => {
    expect(useSettingsStore.getState().locale).toBe('zh-CN');
    expect(useSettingsStore.getState().uiScale).toBe(1.0);
  });
});
