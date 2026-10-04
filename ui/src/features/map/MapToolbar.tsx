// Map 工具栏（§129–§132 搜索/focus、§210–§216 配色+legend、§217–§219
// 过滤 chips、§138 视图命令、§161/§235/§236/§164 构建/重建/取消）。
// §220：改过滤不自动播放；§235 提示里的数字全部来自真实 backend。
import { useMemo, useRef, useState } from 'react';
import type { MapOverviewDto, MapStatusDto } from '../../ipc/map';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';
import { clusterColor, PITCH_CLASSES, pitchClassColor } from './mapData';
import type { MapViewCommand } from './MusicMapView';
import { useMapStore, type MapColorMode } from './mapStore';

interface MapToolbarProps {
  status: MapStatusDto | null;
  overview: MapOverviewDto | null;
  onCommand: (kind: MapViewCommand['kind']) => void;
  onFocusTrack: (trackId: number) => void;
  onSearchMatches: (ids: ReadonlySet<number>) => void;
}

const COLOR_MODES: ReadonlyArray<{ id: MapColorMode; key: string }> = [
  { id: 'cluster', key: 'map.colorCluster' },
  { id: 'energy', key: 'map.colorEnergy' },
  { id: 'key', key: 'map.colorKey' },
  { id: 'brightness', key: 'map.colorBrightness' },
];

const ENERGY_STEPS: ReadonlyArray<{ v: number | null; key: string }> = [
  { v: null, key: 'map.filterAny' },
  { v: 0.3, key: 'map.filterEnergy30' },
  { v: 0.6, key: 'map.filterEnergy60' },
];

export function MapToolbar({
  status,
  overview,
  onCommand,
  onFocusTrack,
  onSearchMatches,
}: MapToolbarProps): React.JSX.Element {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const [resultCount, setResultCount] = useState<number | null>(null);
  const resultIdsRef = useRef<number[]>([]);
  const resultCursorRef = useRef(0);
  const colorMode = useMapStore((s) => s.colorMode);
  const filters = useMapStore((s) => s.filters);

  // §129/§130/§131：搜索 M3 库 → 与地图交集 → focus + 高亮。
  const runSearch = (): void => {
    const q = query.trim();
    if (q === '') {
      setResultCount(null);
      resultIdsRef.current = [];
      onSearchMatches(new Set());
      return;
    }
    void ipc
      .librarySearch(q, 50, 0)
      .then((rows) => {
        const ids = rows.map((r) => r.id);
        resultIdsRef.current = ids;
        resultCursorRef.current = 0;
        setResultCount(ids.length);
        onSearchMatches(new Set(ids));
        const first = ids[0];
        if (first !== undefined) onFocusTrack(first);
      })
      .catch(() => {
        setResultCount(0);
      });
  };

  const cycle = (): void => {
    const ids = resultIdsRef.current;
    if (ids.length === 0) return;
    resultCursorRef.current = (resultCursorRef.current + 1) % ids.length;
    const id = ids[resultCursorRef.current];
    if (id !== undefined) onFocusTrack(id);
  };

  const chips: Array<{ label: string; clear: () => void }> = useMemo(() => {
    const st = useMapStore.getState();
    const out: Array<{ label: string; clear: () => void }> = [];
    if (filters.bpmMin !== null || filters.bpmMax !== null) {
      out.push({
        label: `BPM ${filters.bpmMin ?? 0}–${filters.bpmMax ?? '∞'}`,
        clear: () => st.setFilters({ ...st.filters, bpmMin: null, bpmMax: null }),
      });
    }
    if (filters.energyMin !== null) {
      out.push({
        label: `Energy ≥ ${filters.energyMin}`,
        clear: () => st.setFilters({ ...st.filters, energyMin: null }),
      });
    }
    return out;
  }, [filters]);

  const building = status?.status === 'building';
  const stale = status?.status === 'stale';
  const failed = status?.status === 'failed';
  const ready = status?.status === 'ready';

  return (
    <div className="map-toolbar">
      <div className="map-toolbar-row">
        <input
          id="map-search"
          className="map-search"
          type="text"
          value={query}
          placeholder={t('map.searchPlaceholder')}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') runSearch();
            if (e.key === 'Escape') {
              setQuery('');
              setResultCount(null);
              resultIdsRef.current = [];
              onSearchMatches(new Set());
            }
          }}
          aria-label={t('map.searchPlaceholder')}
        />
        {resultCount !== null && (
          <span className="map-search-count">
            {resultCount > 1 && (
              <button className="ghost-btn small" onClick={cycle}>
                ⤵
              </button>
            )}
            {t('map.searchCount', { count: resultCount })}
          </span>
        )}
        <select
          className="map-select"
          value={colorMode}
          onChange={(e) => useMapStore.getState().setColorMode(e.target.value as MapColorMode)}
          aria-label={t('map.colorBy')}
        >
          {COLOR_MODES.map((m) => (
            <option key={m.id} value={m.id}>
              {t(m.key)}
            </option>
          ))}
        </select>
        <select
          className="map-select"
          value={filters.energyMin === null ? 'null' : String(filters.energyMin)}
          onChange={(e) => {
            const v = e.target.value === 'null' ? null : Number(e.target.value);
            const st = useMapStore.getState();
            st.setFilters({ ...st.filters, energyMin: v });
          }}
          aria-label={t('map.filterEnergy')}
        >
          {ENERGY_STEPS.map((s) => (
            <option key={String(s.v)} value={s.v === null ? 'null' : String(s.v)}>
              {t(s.key)}
            </option>
          ))}
        </select>
        <span className="map-toolbar-spacer" />
        <button className="ghost-btn" onClick={() => onCommand('reset')}>
          {t('map.resetView')}
        </button>
        <button className="ghost-btn" onClick={() => onCommand('fit')}>
          {t('map.fitLibrary')}
        </button>
        {ready && overview && (
          <span className="map-status-line">
            {t('map.readyCounts', { tracks: overview.trackCount, clusters: overview.clusterCount })}
          </span>
        )}
        {building && (
          <>
            <span className="map-status-line building">{status?.message ?? t('map.building')}</span>
            <button className="ghost-btn" onClick={() => void ipc.cancelMapBuild()}>
              {t('map.cancelBuild')}
            </button>
          </>
        )}
        {(stale || failed) && (
          <>
            <span className="map-status-line warn">
              {failed ? (status?.message ?? t('map.buildFailed')) : t('map.needsRebuild')}
            </span>
            <button className="primary-btn" onClick={() => void ipc.requestMapBuild().catch(() => undefined)}>
              {t('map.rebuild')}
            </button>
          </>
        )}
        {status?.status === 'pending' && (
          <button className="primary-btn" onClick={() => void ipc.requestMapBuild().catch(() => undefined)}>
            {t('map.buildNow')}
          </button>
        )}
      </div>
      <div className="map-toolbar-row">
        {/* §210 legend：跟随当前 color mode。 */}
        {colorMode === 'cluster' && overview && (
          <div className="map-legend" aria-label={t('map.legend')}>
            {overview.clusters.slice(0, 12).map((c) => (
              <button
                key={c.clusterId}
                className="map-legend-item"
                title={t('map.legendCluster', { id: c.clusterId + 1, count: c.count })}
                onClick={() => {
                  const st = useMapStore.getState();
                  st.selectCluster(c.clusterId);
                  onCommand('focusCluster');
                }}
              >
                <span className="map-swatch" style={{ background: clusterColor(c.clusterId) }} />
                {c.clusterId + 1}
              </button>
            ))}
          </div>
        )}
        {colorMode === 'key' && (
          <div className="map-legend" aria-label={t('map.legend')}>
            {PITCH_CLASSES.map((p, i) => (
              <span key={p} className="map-legend-item">
                <span className="map-swatch" style={{ background: pitchClassColor(i) }} />
                {p}
              </span>
            ))}
          </div>
        )}
        {(colorMode === 'energy' || colorMode === 'brightness') && (
          <div className="map-legend" aria-label={t('map.legend')}>
            <span className="map-legend-gradient" data-mode={colorMode} />
            <span className="map-legend-item">{t('map.legendLow')}</span>
            <span className="map-legend-item">{t('map.legendHigh')}</span>
          </div>
        )}
        <span className="map-toolbar-spacer" />
        {chips.map((c) => (
          <button key={c.label} className="map-chip" onClick={c.clear} title={t('map.clearFilter')}>
            {c.label} ✕
          </button>
        ))}
        {chips.length > 1 && (
          <button className="map-chip clear-all" onClick={() => useMapStore.getState().clearFilters()}>
            {t('map.clearAllFilters')}
          </button>
        )}
      </div>
    </div>
  );
}
