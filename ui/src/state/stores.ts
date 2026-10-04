// UI 全局状态（§8/§9）：UI state 与 domain state 分离。
// domain（player/library）来自 Rust 快照；UI（nav/panels）来自 Zustand。
import { create } from 'zustand';
import type { Locale } from '../i18n';

export type ViewId = 'now-playing' | 'library' | 'map' | 'dreamscape';

interface UiState {
  view: ViewId;
  commandPaletteOpen: boolean;
  queueOpen: boolean;
  settingsOpen: boolean;
  miniPlayerOpen: boolean;
  sidebarCollapsed: boolean;
  /** 全局用户反馈（i18n key，§187/§189；null = 无）。 */
  notice: string | null;
  /** §269：Zustand 只存 open/status；manifest 走 renderer ref（§270）。 */
  dreamscapeOpen: boolean;
  dreamscapeStatus: 'idle' | 'loading' | 'ready' | 'fallback';
  setView: (v: ViewId) => void;
  setPaletteOpen: (open: boolean) => void;
  setQueueOpen: (open: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setMiniPlayerOpen: (open: boolean) => void;
  toggleSidebar: () => void;
  setNotice: (key: string | null) => void;
  setDreamscapeOpen: (open: boolean) => void;
  setDreamscapeStatus: (s: 'idle' | 'loading' | 'ready' | 'fallback') => void;
}

export const useUiStore = create<UiState>((set) => ({
  view: 'now-playing',
  commandPaletteOpen: false,
  queueOpen: false,
  settingsOpen: false,
  miniPlayerOpen: false,
  sidebarCollapsed: false,
  notice: null,
  dreamscapeOpen: false,
  dreamscapeStatus: 'idle',
  setView: (view) => set({ view }),
  setPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),
  setQueueOpen: (queueOpen) => set({ queueOpen }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setMiniPlayerOpen: (miniPlayerOpen) => set({ miniPlayerOpen }),
  toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
  setNotice: (notice) => set({ notice }),
  setDreamscapeOpen: (dreamscapeOpen) => set({ dreamscapeOpen }),
  setDreamscapeStatus: (dreamscapeStatus) => set({ dreamscapeStatus }),
}));

/** domain state：播放（来自 Rust 快照，§8）。 */
export interface PlayerDomain {
  snapshot: import('../ipc/types').PlayerSnapshot | null;
  setSnapshot: (s: import('../ipc/types').PlayerSnapshot) => void;
}
export const usePlayerStore = create<PlayerDomain>((set) => ({
  snapshot: null,
  setSnapshot: (snapshot) => set({ snapshot }),
}));

/** domain state：library 当前查询（§8）。 */
export interface LibraryDomain {
  query: string;
  sort: import('../ipc/types').SortKey;
  refreshNonce: number;
  /** §312：Map→Library 定位请求（真实 TrackId；LibraryPage 消费）。 */
  focusTrack: { id: number; title: string } | null;
  setQuery: (q: string) => void;
  setSort: (s: import('../ipc/types').SortKey) => void;
  setFocusTrack: (t: { id: number; title: string } | null) => void;
  bumpRefresh: () => void;
}
export const useLibraryStore = create<LibraryDomain>((set) => ({
  query: '',
  sort: 'title',
  refreshNonce: 0,
  focusTrack: null,
  setQuery: (query) => set({ query }),
  setSort: (sort) => set({ sort }),
  setFocusTrack: (focusTrack) => set({ focusTrack }),
  bumpRefresh: () => set((s) => ({ refreshNonce: s.refreshNonce + 1 })),
}));

/** 设置（§114）：语言/缩放/reducedMotion 等；持久化在 Rust 侧 JSON。 */
export interface SettingsState {
  locale: Locale;
  updatesAutoCheck: boolean;
  uiScale: number;
  reducedMotion: boolean;
  loudnessEnabled: boolean;
  crossfadeSeconds: number;
  eqEnabled: boolean;
  eqPreset: number;
  micaEnabled: boolean;
  loaded: boolean;
  setAll: (s: {
    locale: Locale;
    updatesAutoCheck: boolean;
    uiScale: number;
    reducedMotion: boolean;
    loudnessEnabled: boolean;
    crossfadeSeconds: number;
    eqEnabled: boolean;
    eqPreset: number;
    micaEnabled: boolean;
  }) => void;
  setLocale: (l: Locale) => void;
  setUiScale: (v: number) => void;
  setReducedMotion: (v: boolean) => void;
  setUpdatesAutoCheck: (v: boolean) => void;
}
export const useSettingsStore = create<SettingsState>((set) => ({
  locale: 'zh-CN',
  updatesAutoCheck: true,
  uiScale: 1.0,
  reducedMotion: false,
  loudnessEnabled: false,
  crossfadeSeconds: 0,
  eqEnabled: true,
  eqPreset: 0,
  micaEnabled: true,
  loaded: false,
  setAll: (s) => set({ ...s, loaded: true }),
  setLocale: (locale) => set({ locale }),
  setUiScale: (uiScale) => set({ uiScale }),
  setReducedMotion: (reducedMotion) => set({ reducedMotion }),
  setUpdatesAutoCheck: (updatesAutoCheck) => set({ updatesAutoCheck }),
}));
