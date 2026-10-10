// Music Map 页面（M9 中/下篇）：状态握手（§234）→ 数据按需加载 →
// 渲染器/工具栏/详情面板装配。构建期间轮询 status（§161 真实状态；
// 空闲 0 轮询 §198）。§180：同 dataset 恢复上次 zoom/pan（§181 损坏/
// 非有限 → 缺省 fit）。§317：键盘控制 +/-/F/Esc/Enter（§315 可达性）。
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ipc } from '../../ipc/client';
import type { MapOverviewDto, MapStatusDto } from '../../ipc/map';
import { toast, usePlayerStore } from '../../state/stores';
import { useTranslation } from '../../i18n/react';
import { MapDataModel, loadSavedView, saveView } from './mapData';
import { consumeMapCommand, consumeMapFocus } from './mapBus';
import { useMapStore } from './mapStore';
import { MapDetailPanel } from './MapDetailPanel';
import { MapToolbar } from './MapToolbar';
import { MusicMapView, type MapViewCommand } from './MusicMapView';

/** 模块级单例：非响应式 renderer state（§146，不进 React 树）。 */
const mapModel = new MapDataModel();

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

  // §311/§411：消费来自 Library/palette 的焦点与命令（数据就绪后）。
  useEffect(() => {
    if (overview === null) return;
    const f = consumeMapFocus();
    if (f !== null) focusTrack(f);
    const c = consumeMapCommand();
    if (c !== null) issueCommand(c);
  }, [overview, focusTrack, issueCommand]);

  // §317 键盘控制：+/- 缩放、F fit、Esc 清除选择、Enter 播放选中
  // （输入中不拦截；不与全局 Ctrl 组合冲突；§315 键盘可达性基线）。
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)
      ) {
        return;
      }
      const st = useMapStore.getState();
      if (e.key === '+' || e.key === '=') {
        e.preventDefault();
        issueCommand('zoomIn');
      } else if (e.key === '-' || e.key === '_') {
        e.preventDefault();
        issueCommand('zoomOut');
      } else if (e.key.toLowerCase() === 'f' && !e.ctrlKey && !e.metaKey && !e.altKey) {
        issueCommand('fit');
      } else if (e.key === 'Escape') {
        st.selectTrack(null);
        st.selectCluster(null);
      } else if (e.key === 'Enter' && st.selectedTrackId !== null) {
        void ipc.playTrack(st.selectedTrackId).catch(() => undefined); // §314 单曲语义
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [issueCommand]);

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
  // 空态要分清三件事：库里没歌 / 有歌但没分析 / 已分析但没构建地图。
  const [preflight, setPreflight] = useState<{
    libraryEmpty: boolean;
    analyzed: number;
    pending: number;
  } | null>(null);
  const refreshPreflight = useCallback(() => {
    void Promise.all([ipc.libraryStats(), ipc.analysisStatus()])
      .then(([lib, analysis]) => {
        setPreflight({
          libraryEmpty: lib.tracks === 0,
          analyzed: analysis.analyzed,
          pending: analysis.pending,
        });
      })
      .catch(() => undefined);
  }, []);
  useEffect(() => {
    if (status?.status !== 'pending') return;
    refreshPreflight();
    // 分析跑完 → 空态自动切到"可以构建地图"，不用用户手动刷新。
    let wasRunning = false;
    const un = ipc.listenAnalysisProgress((p) => {
      if (wasRunning && !p.running) refreshPreflight();
      wasRunning = p.running;
    });
    return () => {
      void un.then((f) => f());
    };
  }, [status?.status, refreshPreflight]);
  const runAnalysis = (): void => {
    void ipc
      .requestAnalysis()
      .then((queued) => toast(t('settings.analysisQueued', { count: queued }), 'success'))
      .catch(() => toast(t('settings.analysisFailed'), 'error'));
  };
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
        {!building && emptyLibrary && (
          <>
            <p className="map-status">
              {preflight === null
                ? t('map.checking')
                : preflight.libraryEmpty
                  ? t('map.addMusicFirst')
                  : preflight.pending > 0
                    ? t('map.analyzeFirst')
                    : preflight.analyzed > 0
                      ? t('map.buildReady', { count: preflight.analyzed })
                      : t('map.nothingToMap')}
            </p>
            {preflight !== null && !preflight.libraryEmpty && preflight.pending > 0 && (
              <button className="primary-btn" onClick={runAnalysis}>
                {t('settings.analysisRun')}
              </button>
            )}
            {preflight !== null &&
              preflight.analyzed > 0 &&
              preflight.pending === 0 && (
                <button className="primary-btn" onClick={requestBuild}>
                  {t('map.buildNow')}
                </button>
              )}
          </>
        )}
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
  }, [error, ready, status, overview, building, emptyLibrary, preflight, searchMatches, command, currentTrackId, selectedTrackId, t]);

  return (
    <section className="map-page" aria-label={t('map.title')}>
      <h1>{t('map.title')}</h1>
      {body}
    </section>
  );
}
