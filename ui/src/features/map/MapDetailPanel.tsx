// Track Detail Panel（§124–§128）：只展示 M3/M8 真实存在的字段
// （缺失 → 隐藏该行）；"Similar Tracks" 邻居列表来自真实 M9 top-K
// （§126/§127 不叫 Recommendation；§128 标注 cosine）；播放走 M6 既有
// 命令（§221/§222 单曲，不自动连播）。
import { useEffect, useState } from 'react';
import type { MapNeighborDto, MapNodeDto } from '../../ipc/map';
import { ipc } from '../../ipc/client';
import { useArtwork } from '../../components/Artwork';
import { useLibraryStore, useUiStore } from '../../state/stores';
import { useTranslation } from '../../i18n/react';
import type { MapDataModel } from './mapData';
import { useMapStore } from './mapStore';

interface MapDetailPanelProps {
  model: MapDataModel;
  trackId: number | null;
  onFocus: (trackId: number) => void;
}

function Row({ label, value }: { label: string; value: string | null }): React.JSX.Element | null {
  if (value === null || value === '') return null; // §124 缺失 → 不显示
  return (
    <div className="map-detail-row">
      <span className="map-detail-label">{label}</span>
      <span className="map-detail-value">{value}</span>
    </div>
  );
}

export function MapDetailPanel({ model, trackId, onFocus }: MapDetailPanelProps): React.JSX.Element | null {
  const { t } = useTranslation();
  const [detail, setDetail] = useState<MapNodeDto | null>(null);
  const [neighbors, setNeighbors] = useState<MapNeighborDto[]>([]);
  const artwork = useArtwork(trackId);

  useEffect(() => {
    if (trackId === null) {
      setDetail(null);
      setNeighbors([]);
      return;
    }
    let alive = true;
    const meta = model.metaOf(trackId);
    if (meta) {
      setDetail(meta);
    } else {
      void ipc
        .mapTrack(trackId)
        .then((n) => {
          if (alive) setDetail(n);
        })
        .catch(() => undefined);
    }
    void model
      .neighborsFor(trackId, 8)
      .then((rows) => {
        if (alive) setNeighbors(rows);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [trackId, model]);

  if (trackId === null || detail === null) return null;
  const close = (): void => useMapStore.getState().selectTrack(null);
  const cluster = detail.clusterId + 1;

  return (
    <aside className="map-detail" aria-label={t('map.detailTitle')}>
      <div className="map-detail-head">
        {artwork.kind === 'loaded' && <img className="map-detail-art" src={artwork.dataUrl} alt="" />}
        <div className="map-detail-titles">
          <h3>{detail.title ?? t('map.unknownTitle')}</h3>
          <p>{[detail.artist, detail.album].filter(Boolean).join(' — ') || ' '}</p>
        </div>
        <button className="icon-btn" onClick={close} aria-label={t('common.close')}>
          ✕
        </button>
      </div>
      <div className="map-detail-rows">
        <Row label={t('map.bpm')} value={detail.bpm !== null ? detail.bpm.toFixed(1) : null} />
        <Row label={t('map.key')} value={detail.musicalKey} />
        <Row
          label={t('map.loudness')}
          value={detail.integratedLufs !== null ? `${detail.integratedLufs.toFixed(1)} LUFS` : null}
        />
        <Row
          label={t('map.energy')}
          value={detail.energy !== null ? detail.energy.toFixed(2) : null}
        />
        <Row
          label={t('map.brightness')}
          value={detail.brightness !== null ? detail.brightness.toFixed(2) : null}
        />
        <Row label={t('map.cluster')} value={`Cluster ${String(cluster).padStart(2, '0')}`} />
        <Row label={t('map.genre')} value={detail.genre} />
      </div>
      <div className="map-detail-actions">
        <button className="primary-btn" onClick={() => void ipc.playTrack(detail.trackId)}>
          ▶ {t('map.play')}
        </button>
        <button className="ghost-btn" onClick={() => onFocus(detail.trackId)}>
          {t('map.focusThis')}
        </button>
        {/* §312：Map→Library，真实 TrackId。 */}
        <button
          className="ghost-btn"
          onClick={() => {
            useLibraryStore.getState().setFocusTrack({
              id: detail.trackId,
              title: detail.title ?? `#${detail.trackId}`,
            });
            useUiStore.getState().setView('library');
          }}
        >
          {t('map.openInLibrary')}
        </button>
      </div>
      <h4 className="map-neighbors-title" title={t('map.similarityMetric')}>
        {t('map.similarTracks')}
      </h4>
      <ul className="map-neighbors">
        {neighbors.map((n) => (
          <li key={n.trackId}>
            <button
              className="map-neighbor-row"
              onClick={() => {
                useMapStore.getState().selectTrack(n.trackId);
                onFocus(n.trackId);
              }}
            >
              <span className="map-neighbor-name">
                {n.title ?? `#${n.trackId}`}
                {n.artist ? <span className="map-neighbor-artist"> — {n.artist}</span> : null}
              </span>
              <span className="map-neighbor-sim" title={t('map.similarityMetric')}>
                {n.similarity.toFixed(2)}
              </span>
              <span
                className="icon-btn"
                role="button"
                tabIndex={0}
                aria-label={t('map.play')}
                onClick={(e) => {
                  e.stopPropagation();
                  void ipc.playTrack(n.trackId); // §222：单曲播放
                }}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    e.stopPropagation();
                    void ipc.playTrack(n.trackId);
                  }
                }}
              >
                ▶
              </span>
            </button>
          </li>
        ))}
        {neighbors.length === 0 && <li className="map-neighbors-empty">{t('map.noNeighbors')}</li>}
      </ul>
    </aside>
  );
}
