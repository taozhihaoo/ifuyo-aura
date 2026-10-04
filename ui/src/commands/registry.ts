// 命令注册架构（§99/§100）：Command { id, title, keywords, shortcut, execute }。
import type { ViewId } from '../state/stores';

export interface Command {
  id: string;
  title: string;
  titleKey: string;
  keywords: string;
  shortcut?: string;
  execute: () => void;
}

export interface CommandContext {
  setView: (v: ViewId) => void;
  play: () => void;
  pause: () => void;
  next: () => void;
  previous: () => void;
  stop: () => void;
  openFolder: () => void;
  openUrl: () => void;
  toggleQueue: () => void;
  toggleEq: () => void;
  toggleLoudness: () => void;
  toggleMiniPlayer: () => void;
  focusLibrarySearch: () => void;
  isPlaying: () => boolean;
  openDreamscape: () => void;
  regenerateDreamscape: () => void;
  copyDreamscapeSeed: () => void;
  saveDreamscapeSnapshot: () => void;
  rebuildMap: () => void;
  fitMap: () => void;
  resetMapView: () => void;
}

/** 命令表：声明式注册（§99），无 if/else 链。 */
export function buildCommands(ctx: CommandContext, t: (key: string) => string): Command[] {
  return [
    { id: 'nav.now-playing', titleKey: 'nav.nowPlaying', keywords: 'now playing home', shortcut: '1', execute: () => ctx.setView('now-playing') },
    { id: 'nav.library', titleKey: 'nav.library', keywords: 'library music list', shortcut: '2', execute: () => ctx.setView('library') },
    { id: 'nav.map', titleKey: 'nav.map', keywords: 'map graph', shortcut: '3', execute: () => ctx.setView('map') },
    { id: 'play.toggle', titleKey: 'controls.play', keywords: 'play pause', shortcut: 'Space', execute: () => (ctx.isPlaying() ? ctx.pause() : ctx.play()) },
    { id: 'play.next', titleKey: 'controls.next', keywords: 'next track', execute: ctx.next },
    { id: 'play.previous', titleKey: 'controls.previous', keywords: 'previous track', execute: ctx.previous },
    { id: 'play.stop', titleKey: 'controls.stop', keywords: 'stop', execute: ctx.stop },
    { id: 'library.open-folder', titleKey: 'library.addFolder', keywords: 'add music folder scan', execute: ctx.openFolder },
    { id: 'library.open-url', titleKey: 'library.openUrl', keywords: 'open url stream', execute: ctx.openUrl },
    { id: 'view.queue', titleKey: 'queue.title', keywords: 'queue drawer', execute: ctx.toggleQueue },
    { id: 'view.eq', titleKey: 'settings.eq', keywords: 'equalizer eq', execute: ctx.toggleEq },
    { id: 'settings.loudness', titleKey: 'settings.loudness', keywords: 'loudness normalization', execute: ctx.toggleLoudness },
    { id: 'view.mini', titleKey: 'view.miniPlayer', keywords: 'mini player window', execute: ctx.toggleMiniPlayer },
    { id: 'library.focus-search', titleKey: 'library.search', keywords: 'search focus', shortcut: 'Ctrl+F', execute: ctx.focusLibrarySearch },
    { id: 'dreamscape.open', titleKey: 'dreamscape.title', keywords: 'dreamscape world enter open', execute: ctx.openDreamscape },
    { id: 'dreamscape.regenerate', titleKey: 'dreamscape.regenerate', keywords: 'dreamscape regenerate new world', execute: ctx.regenerateDreamscape },
    { id: 'dreamscape.copy-seed', titleKey: 'dreamscape.copySeed', keywords: 'dreamscape seed copy', execute: ctx.copyDreamscapeSeed },
    { id: 'dreamscape.save-snapshot', titleKey: 'dreamscape.saveSnapshot', keywords: 'dreamscape snapshot save', execute: ctx.saveDreamscapeSnapshot },
    // §411：Map 命令（少量核心；§412 不做命令爆炸；Open Music Map =
    // nav.map 已存在）。
    { id: 'map.rebuild', titleKey: 'map.rebuild', keywords: 'map rebuild music graph', execute: ctx.rebuildMap },
    { id: 'map.fit', titleKey: 'map.fitLibrary', keywords: 'map fit library view', execute: ctx.fitMap },
    { id: 'map.reset', titleKey: 'map.resetView', keywords: 'map reset view camera', execute: ctx.resetMapView },
  ].map((c) => ({ ...c, title: t(c.titleKey) }));
}

/** 简单 fuzzy 子序列匹配（§101：不引入大型搜索框架）。 */
export function matchCommand(cmd: Command, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const hay = `${cmd.title} ${cmd.keywords} ${cmd.shortcut ?? ''}`.toLowerCase();
  if (hay.includes(q)) return true;
  // 子序列匹配。
  let i = 0;
  for (const ch of hay) {
    if (ch === q[i]) i += 1;
    if (i >= q.length) return true;
  }
  return false;
}
