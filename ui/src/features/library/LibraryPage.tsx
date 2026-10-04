// Library（§76–§94）：真实 FTS5 搜索（debounce §79）+ 虚拟化列表（§82）
// + 分页（§85）+ 扫描进度（§88/§89）+ 空态/错误态（§91/§92）。
import { useEffect, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useLibraryStore, useSettingsStore, useUiStore } from '../../state/stores';
import { requestMapFocus } from '../map/mapBus';
import { ipc } from '../../ipc/client';
import type { ScanProgress, SortKey, TrackSummary } from '../../ipc/types';
import { useTranslation } from '../../i18n/react';
import { formatDuration } from '../../ui/format';
import { playWithQueueContext } from '../queue/QueueDrawer';

type LoadState =
  | { kind: 'loading' }
  | { kind: 'ready'; tracks: TrackSummary[] }
  | { kind: 'empty' }
  | { kind: 'error' };

const PAGE_SIZE = 200;
const SORTS: SortKey[] = ['title', 'artist', 'album', 'dateAdded', 'duration', 'year'];

function sortLabelKey(k: SortKey): string {
  const cap = k.charAt(0).toUpperCase() + k.slice(1);
  return `library.sort${cap}`;
}

export function LibraryPage() {
  const query = useLibraryStore((s) => s.query);
  const sort = useLibraryStore((s) => s.sort);
  const refreshNonce = useLibraryStore((s) => s.refreshNonce);
  const setQuery = useLibraryStore((s) => s.setQuery);
  const setSort = useLibraryStore((s) => s.setSort);
  const locale = useSettingsStore((s) => s.locale);
  const { t } = useTranslation(locale);
  const [debounced, setDebounced] = useState('');
  const [state, setState] = useState<LoadState>({ kind: 'loading' });
  const [scan, setScan] = useState<ScanProgress | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const setView = useUiStore((s) => s.setView);
  const focusTrack = useLibraryStore((s) => s.focusTrack);

  // §312：消费 Map→Library 定位请求（真实 TrackId）：切换查询保证
  // 可见，再滚动高亮该行。
  useEffect(() => {
    if (focusTrack === null) return;
    if (query !== focusTrack.title) {
      setQuery(focusTrack.title);
    }
    let attempts = 0;
    const timer = window.setInterval(() => {
      attempts += 1;
      const el = listRef.current?.querySelector(`[data-track-id="${focusTrack.id}"]`);
      if (el) {
        el.scrollIntoView({ block: 'center' });
        el.classList.add('tr-focused');
        window.setTimeout(() => el.classList.remove('tr-focused'), 2400);
        window.clearInterval(timer);
        useLibraryStore.getState().setFocusTrack(null);
      } else if (attempts >= 12) {
        window.clearInterval(timer);
        useLibraryStore.getState().setFocusTrack(null);
      }
    }, 250);
    return () => window.clearInterval(timer);
  }, [focusTrack, query, setQuery]);

  // §79：搜索 debounce 250ms。
  useEffect(() => {
    const h = window.setTimeout(() => setDebounced(query), 250);
    return () => window.clearTimeout(h);
  }, [query]);

  // 数据加载：搜索 or 列表；library-changed → 保持查询重取（§152）。
  useEffect(() => {
    let alive = true;
    setState({ kind: 'loading' });
    void (async () => {
      try {
        const tracks = debounced
          ? await ipc.librarySearch(debounced, PAGE_SIZE, 0)
          : await ipc.libraryList(sort, PAGE_SIZE, 0);
        if (!alive) return;
        setState(tracks.length === 0 && !debounced ? { kind: 'empty' } : { kind: 'ready', tracks });
      } catch {
        if (alive) setState({ kind: 'error' });
      }
    })();
    return () => {
      alive = false;
    };
  }, [debounced, sort, refreshNonce]);

  // §88/§151：scan progress / library-changed 事件。
  useEffect(() => {
    const un1 = ipc.listenScanProgress((p) => setScan(p));
    const un2 = ipc.listenLibraryChanged(() => useLibraryStore.getState().bumpRefresh());
    return () => {
      void un1.then((f) => f());
      void un2.then((f) => f());
    };
  }, []);

  // §82：虚拟化。
  const tracks = state.kind === 'ready' ? state.tracks : [];
  const virtualizer = useVirtualizer({
    count: tracks.length,
    getScrollElement: () => listRef.current,
    estimateSize: () => 52,
    overscan: 12,
  });

  const onAddFolder = async () => {
    // §87/§90：native dialog + IPC 委托；React 不做 FS 扫描。
    const dialog = await import('@tauri-apps/plugin-dialog').catch(() => null);
    if (!dialog) return;
    const picked = await dialog.open({ directory: true }).catch(() => null);
    if (typeof picked === 'string') {
      await ipc.scanFolder(picked).catch(() => undefined);
    }
  };

  const rows = virtualizer.getVirtualItems();

  return (
    <section className="library" aria-label={t('nav.library')}>
      <div className="library-toolbar">
        <input
          id="library-search"
          className="search-input"
          type="search"
          placeholder={t('library.search')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          aria-label={t('library.search')}
        />
        <select
          className="sort-select"
          value={sort}
          onChange={(e) => setSort(e.target.value as SortKey)}
          aria-label="sort"
        >
          {SORTS.map((k) => (
            <option key={k} value={k}>
              {t(sortLabelKey(k))}
            </option>
          ))}
        </select>
        <button className="btn" onClick={() => void onAddFolder()}>
          {t('library.addFolder')}
        </button>
      </div>

      {scan && !scan.done && (
        <div className="scan-progress" role="status">
          <span>
            {t('library.scanProgress', {
              processed: scan.processed,
              discovered: scan.discovered,
              imported: scan.imported,
              failed: scan.failed,
            })}
          </span>
          <button className="btn" onClick={() => void ipc.cancelScan()}>
            {t('library.cancelScan')}
          </button>
        </div>
      )}

      {state.kind === 'error' && (
        <div className="state-error" role="alert">
          <p>{t('library.error')}</p>
          <button className="btn" onClick={() => useLibraryStore.getState().bumpRefresh()}>
            {t('library.retry')}
          </button>
        </div>
      )}

      {state.kind === 'empty' && (
        <div className="state-empty">
          <h2>{t('library.empty')}</h2>
          <p>{t('library.emptyHint')}</p>
          <button className="btn primary" onClick={() => void onAddFolder()}>
            {t('library.addFolder')}
          </button>
        </div>
      )}

      {state.kind === 'loading' && (
        <div className="state-loading" role="status" aria-busy>
          <div className="skeleton" />
          <div className="skeleton" />
          <div className="skeleton" />
        </div>
      )}

      {state.kind === 'ready' && (
        <div className="track-list" ref={listRef}>
          <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
            {rows.map((row) => {
              const track: TrackSummary | undefined = tracks[row.index];
              if (!track) return null;
              return (
                <div
                  key={track.id}
                  className="track-row"
                  data-track-id={track.id}
                  style={{ position: 'absolute', top: row.start, height: 52 }}
                  onDoubleClick={() => {
                    playWithQueueContext(track, tracks);
                    void ipc.playTrack(track.id).catch(() => undefined);
                  }}
                >
                  <span className="tr-artwork" aria-hidden>
                    {track.hasArtwork ? '▣' : '♪'}
                  </span>
                  <span className="tr-title">{track.title ?? track.path}</span>
                  <span className="tr-artist">{track.artist ?? ''}</span>
                  <span className="tr-album">{track.album ?? ''}</span>
                  <span className="tr-duration">
                    {formatDuration(track.durationSeconds) ?? ''}
                  </span>
                  {/* §311：Library→Map "Open on Map"。 */}
                  <button
                    className="tr-act"
                    title={t('map.openOnMap')}
                    aria-label={t('map.openOnMap')}
                    onClick={(e) => {
                      e.stopPropagation();
                      requestMapFocus(track.id);
                      setView('map');
                    }}
                  >
                    ⌖
                  </button>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </section>
  );
}
