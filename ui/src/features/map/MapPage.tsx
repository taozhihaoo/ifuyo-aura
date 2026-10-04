// Music Map 页面（M9 中篇）：状态握手（§234）→ 数据按需加载 →
// 渲染器/工具栏/详情面板装配。构建期间轮询 status（§161 真实状态；
// 空闲 0 轮询 §198）。§180：同 dataset 恢复上次 zoom/pan（localStorage，
// 损坏/非有限 → 缺省 fit §181）。
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ipc } from '../../ipc/client';
import type { MapOverviewDto, MapStatusDto } from '../../ipc/map';
import { usePlayerStore } from '../../state/stores';
import { useTranslation } from '../../i18n/react';
import { clampCamera, MapDataModel } from './mapData';
import { useMapStore } from './mapStore';
import { MapDetailPanel } from './MapDetailPanel';
import { MapToolbar } from './MapToolbar';
import { MusicMapView, type MapViewCommand } from './MusicMapView';

/** 模块级单例：非响应式 renderer state（§146，不进 React 树）。 */
const mapModel = new MapDataModel();

const VIEW_KEY = 'aura.map.view';

interface SavedView {
  hash: string;
  zoom: number;
  panX: number;
  panY: number;
}

function loadSavedView(hash: string): { zoom: number; panX: number; panY: number } | null {
  try {
    const raw = window.localStorage.getItem(VIEW_KEY);
    if (raw === null) return null;
    const v = JSON.parse(raw) as Partial<SavedView>;
    if (
      v.hash !== hash ||
      typeof v.zoom !== 'number' ||
      typeof v.panX !== 'number' ||
      typeof v.panY !== 'number' ||
      !Number.isFinite(v.zoom) ||
      !Number.isFinite(v.panX) ||
      !Number.isFinite(v.panY)
    ) {
      return null; // §181 无效 → 缺省视图，不 crash
    }
    const cam = clampCamera({ zoom: v.zoom, panX: v.panX, panY: v.panY });
    return { zoom: cam.zoom, panX: cam.panX, panY: cam.panY };
  } catch {
    return null;
  }
}

function saveView(hash: string | null): void {
  if (hash === null) return;
  try {
    const st = useMapStore.getState();
    const v: SavedView = { hash, zoom: st.zoom, panX: st.pan.x, panY: st.pan.y };
    window.localStorage.setItem(VIEW_KEY, JSON.stringify(v));
  } catch {
    /* 忽略（隐私模式等） */
  }
}

export function MapPage() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<MapStatusDto | null>(null);
  const [overview, setOverview] = useState<MapOverviewDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searchMatches, setSearchMatches] = useState<ReadonlySet<number>>(new Set());
  const [command, setCommand] = useState<MapViewCommand | null>(null);
  const [currentTrackId, setCurrentTrackId] = useState<number | null>(null);
  const commandNonce = useRef(0);

  const issueCommand = useCallback((kind: MapViewCommand['kind']) => {
    commandNonce.current += 1;
    setCommand({ kind, nonce: commandNonce.current });
  }, []);

  const focusTrack = useCallback(
    (trackId: number) => {
      useMapStore.getState().selectTrack(trackId);
      issueCommand('focusSelected');
    },
    [issueCommand],
  );

  const refresh = useCallback(async (): Promise<void> => {
    const s = await ipc.mapStatus();
    setStatus(s);
    useMapStore.getState().setMapBuildStatus(s.status);
    if (s.status !== 'ready' || s.datasetHash === null) return;
    const ov = await ipc.mapOverview();
    mapModel.resetForDataset(ov.datasetHash, ov);
    if (mapModel.count === 0) await mapModel.loadAll();
    setOverview(ov);
    const st = useMapStore.getState();
    if (st.zoom === 1 && st.pan.x === 0 && st.pan.y === 0) {
      const saved = loadSavedView(ov.datasetHash); // §180 同 dataset 恢复
      if (saved) {
        st.setZoom(saved.zoom);
        st.setPan({ x: saved.panX, y: saved.panY });
      } else {
        issueCommand('fit');
      }
    }
  }, [issueCommand]);

  // 首次进入：状态握手（触发惰性 restore）。
  useEffect(() => {
    useMapStore.getState().setMapOpen(true);
    refresh().catch((e: { message?: string }) => setError(e?.message ?? String(e)));
    return () => useMapStore.getState().setMapOpen(false);
  }, [refresh]);

  // 构建期间轮询（§161 真实状态；结束后停止 §198）。
  const building = status?.status === 'building';
  useEffect(() => {
    if (!building) return;
    const timer = window.setInterval(() => refresh().catch(() => undefined), 800);
    return () => window.clearInterval(timer);
  }, [building, refresh]);

  // §180：zoom/pan 变化（手势结束同步）→ 持久化。
  useEffect(() => {
    const unsub = useMapStore.subscribe(() => saveView(overview?.datasetHash ?? null));
    return () => {
      unsub();
    };
  }, [overview?.datasetHash]);

  // §225 当前播放 TrackId：M6 快照只给 path → 一次 trackIdByPath 解析
  // （§226 只要身份）。
  const trackPath = usePlayerStore((s) => s.snapshot?.trackPath ?? null);
  useEffect(() => {
    if (trackPath === null) {
      setCurrentTrackId(null);
      return;
    }
    let alive = true;
    void ipc
      .trackIdByPath(trackPath)
      .then((id) => {
        if (alive) setCurrentTrackId(id);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [trackPath]);

  const ready = status?.status === 'ready' && overview !== null;
  const selectedTrackId = useMapStore((s) => s.selectedTrackId);
  const requestBuild = (): void => {
    void ipc
      .requestMapBuild()
      .then(() => refresh())
      .catch((e: { message?: string }) => setError(e?.message ?? String(e)));
  };

  const emptyLibrary = status?.status === 'pending';
  const body = useMemo(() => {
    if (error !== null) {
      return (
        <div className="map-placeholder">
          <p className="map-status">{t('map.loadError')}</p>
          <button className="ghost-btn" onClick={() => window.location.reload()}>
            {t('common.retry')}
          </button>
        </div>
      );
    }
    if (ready) {
      return (
        <>
          <MapToolbar
            status={status}
            overview={overview}
            onCommand={issueCommand}
            onFocusTrack={focusTrack}
            onSearchMatches={setSearchMatches}
          />
          <div className="map-stage">
            <MusicMapView
              model={mapModel}
              currentTrackId={currentTrackId}
              searchMatchIds={searchMatches}
              command={command}
              onHoverTrack={() => undefined}
            />
            <MapDetailPanel model={mapModel} trackId={selectedTrackId} onFocus={focusTrack} />
          </div>
        </>
      );
    }
    return (
      <div className="map-placeholder">
        {building && (
          <>
            <p className="map-status">{status?.message ?? t('map.building')}</p>
            <div className="map-blob b1" aria-hidden />
            <button className="ghost-btn" onClick={() => void ipc.cancelMapBuild()}>
              {t('map.cancelBuild')}
            </button>
          </>
        )}
        {!building && emptyLibrary && <p className="map-status">{t('map.analyzeFirst')}</p>}
        {!building && (status?.status === 'stale' || status?.status === 'failed') && (
          <>
            <p className="map-status">
              {status.status === 'failed'
                ? (status.message ?? t('map.buildFailed'))
                : t('map.needsRebuild')}
            </p>
            <button className="primary-btn" onClick={requestBuild}>
              {t('map.rebuild')}
            </button>
          </>
        )}
        {!building && status?.status === 'cancelled' && (
          <>
            <p className="map-status">{t('map.wasCancelled')}</p>
            <button className="primary-btn" onClick={requestBuild}>
              {t('map.buildNow')}
            </button>
          </>
        )}
      </div>
    );
  }, [error, ready, status, overview, building, emptyLibrary, searchMatches, command, currentTrackId, selectedTrackId, t]);

  return (
    <section className="map-page" aria-label={t('map.title')}>
      <h1>{t('map.title')}</h1>
      {body}
    </section>
  );
}
