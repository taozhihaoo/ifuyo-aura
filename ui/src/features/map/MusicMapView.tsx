// Music Map 渲染器（M9 中篇 §109–§160）：单 Canvas2D 主渲染器
// （§149）；节点为简单 circle（§110）；按需渲染（§197/§198 无常驻
// rAF，静止 0 负载）；hit testing 前端本地（§156/§158 不发 IPC）；
// 边仅选中邻域（§118/§119 预算）；LOD 三级（§142）；reduced motion
// 即时（§133）；当前播放静态双环（§225，无持续粒子 §200）。
import { useCallback, useEffect, useRef } from 'react';
import type { MapNeighborDto } from '../../ipc/map';
import { useSettingsStore } from '../../state/stores';
import {
  CLUSTER_PALETTE,
  clampCamera,
  fitCamera,
  nodeColor,
  screenToWorld,
  worldToScreen,
  zoomAt,
  type Camera,
} from './mapData';
import type { MapDataModel } from './mapData';
import { useMapStore } from './mapStore';

export interface MapViewCommand {
  kind: 'reset' | 'fit' | 'focusSelected' | 'focusCluster';
  nonce: number;
}

interface MusicMapViewProps {
  model: MapDataModel;
  /** §225 当前播放 TrackId（来自 M6 快照 trackPath 解析；仅身份 §226）。 */
  currentTrackId: number | null;
  /** §132 搜索命中高亮（只增强显示，不改位置）。 */
  searchMatchIds: ReadonlySet<number>;
  command: MapViewCommand | null;
  onHoverTrack: (id: number | null) => void;
}

/** LOD 阈值（§142）：far → 簇晕；mid → 节点；near → 标签。 */
const CLUSTER_LOD_ZOOM = 0.7;
const LABEL_LOD_ZOOM = 2.0;
/** §119 边预算：只画选中节点邻域，上限 30 条。 */
const MAX_VISIBLE_EDGES = 30;
const NEIGHBOR_FETCH_K = 12;

function clusterHex(clusterId: number): string {
  const idx = Math.floor(((clusterId * 0.6180339887) % 1) * CLUSTER_PALETTE.length);
  return CLUSTER_PALETTE[idx as number] as string;
}

function withAlpha(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

function parseFilters(key: string): { bpmMin: number | null; bpmMax: number | null; energyMin: number | null } {
  const [a, b, c] = key.split(',');
  const num = (s: string | undefined): number | null => (s !== undefined && s !== 'null' && s !== '' ? Number(s) : null);
  return { bpmMin: num(a), bpmMax: num(b), energyMin: num(c) };
}

export function MusicMapView({
  model,
  currentTrackId,
  searchMatchIds,
  command,
  onHoverTrack,
}: MusicMapViewProps) {
  const wrapRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  // 相机存 ref（60fps 平移不进 React 树 §147）；store 只在手势结束
  // 后同步一次（§180 持久化源）。
  const camRef = useRef<Camera>({
    zoom: useMapStore.getState().zoom,
    panX: useMapStore.getState().pan.x,
    panY: useMapStore.getState().pan.y,
  });
  const hoverRef = useRef<number | null>(null);
  const dragRef = useRef<{ sx: number; sy: number; moved: boolean } | null>(null);
  const neighborsRef = useRef<MapNeighborDto[]>([]);
  const animRef = useRef<{ from: Camera; to: Camera; start: number; dur: number } | null>(null);
  const rafRef = useRef(0);
  const syncTimer = useRef(0);
  const drawRef = useRef<() => void>(() => undefined);

  const colorMode = useMapStore((s) => s.colorMode);
  const selectedTrackId = useMapStore((s) => s.selectedTrackId);
  const selectedClusterId = useMapStore((s) => s.selectedClusterId);
  const zoomBadge = useMapStore((s) => s.zoom);
  const filterKey = useMapStore(
    (s) => `${String(s.filters.bpmMin)},${String(s.filters.bpmMax)},${String(s.filters.energyMin)}`,
  );
  const reducedMotion = useSettingsStore((s) => s.reducedMotion);

  const stateRef = useRef({
    colorMode,
    selectedTrackId,
    selectedClusterId,
    searchMatchIds,
    reducedMotion,
    currentTrackId,
    filterKey,
  });
  stateRef.current = {
    colorMode,
    selectedTrackId,
    selectedClusterId,
    searchMatchIds,
    reducedMotion,
    currentTrackId,
    filterKey,
  };

  const requestRender = useCallback(() => {
    if (rafRef.current) return;
    rafRef.current = requestAnimationFrame(() => {
      rafRef.current = 0;
      drawRef.current();
    });
  }, []);

  const syncStoreSoon = useCallback(() => {
    window.clearTimeout(syncTimer.current);
    syncTimer.current = window.setTimeout(() => {
      const cam = camRef.current;
      const st = useMapStore.getState();
      if (st.zoom !== cam.zoom) st.setZoom(cam.zoom);
      if (st.pan.x !== cam.panX || st.pan.y !== cam.panY) st.setPan({ x: cam.panX, y: cam.panY });
    }, 150);
  }, []);

  // ---- 单帧绘制（结束不再排帧 §198）----
  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const wrap = wrapRef.current;
    if (!canvas || !wrap) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = wrap.clientWidth;
    const h = wrap.clientHeight;
    if (w < 2 || h < 2) return;
    if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) {
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    camRef.current = clampCamera(camRef.current); // §192 数值安全
    const cam = camRef.current;
    const S = stateRef.current;

    // 背景（§202：很弱的氛围场，不复制 Aurora）。
    ctx.fillStyle = '#0b0d13';
    ctx.fillRect(0, 0, w, h);
    const bg = ctx.createRadialGradient(w / 2, h * 0.45, 0, w / 2, h * 0.45, Math.max(w, h) * 0.72);
    bg.addColorStop(0, 'rgba(40,52,86,0.22)');
    bg.addColorStop(1, 'rgba(11,13,19,0)');
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, w, h);

    const [wx0, wy0] = screenToWorld(cam, w, h, 0, 0);
    const [wx1, wy1] = screenToWorld(cam, w, h, w, h);
    const mask = model.filterMask(parseFilters(S.filterKey));

    // far LOD：簇晕（§204 soft region，非气泡框）。
    if (cam.zoom < CLUSTER_LOD_ZOOM && S.colorMode === 'cluster' && model.overview) {
      ctx.font = '11px system-ui, sans-serif';
      ctx.textAlign = 'center';
      for (const c of model.overview.clusters) {
        if (!c.centroid || c.radius === null) continue;
        const [cx, cy] = worldToScreen(cam, w, h, c.centroid[0], c.centroid[1]);
        const r = Math.max(6, c.radius * cam.zoom * (Math.min(w, h) / 2));
        if (cx < -r || cy < -r || cx > w + r || cy > h + r) continue;
        const color = clusterHex(c.clusterId);
        const halo = ctx.createRadialGradient(cx, cy, 0, cx, cy, r);
        halo.addColorStop(0, withAlpha(color, 0.14));
        halo.addColorStop(1, withAlpha(color, 0));
        ctx.fillStyle = halo;
        ctx.beginPath();
        ctx.arc(cx, cy, r, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = 'rgba(220,228,255,0.5)';
        ctx.fillText(`${c.clusterId + 1} · ${c.count}`, cx, cy - r - 4);
      }
    }

    // 边：仅选中邻域（§118），低 alpha 单 accent（§121/§205），稳定
    // 相似度→透明度映射（§120）。
    const selectedIdx = S.selectedTrackId !== null ? model.indexOf(S.selectedTrackId) : -1;
    if (selectedIdx >= 0 && cam.zoom >= CLUSTER_LOD_ZOOM) {
      const [px, py] = model.posAt(selectedIdx);
      const [sx, sy] = worldToScreen(cam, w, h, px, py);
      for (const nb of neighborsRef.current.slice(0, MAX_VISIBLE_EDGES)) {
        const ni = model.indexOf(nb.trackId);
        if (ni < 0) continue;
        const [nx, ny] = model.posAt(ni);
        const [tx, ty] = worldToScreen(cam, w, h, nx, ny);
        ctx.strokeStyle = `rgba(148,176,255,${(0.08 + 0.3 * nb.similarity).toFixed(3)})`;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(sx, sy);
        ctx.lineTo(tx, ty);
        ctx.stroke();
      }
    }

    // 节点（mid LOD）：视口 culling（§143）+ 过滤（§218）。
    const indices = model.visibleIndices(wx0, wy0, wx1, wy1);
    const nodeR = Math.max(1.6, Math.min(6, 1.8 * Math.sqrt(cam.zoom)));
    const drawn: Array<{ id: number; x: number; y: number }> = [];
    for (const i of indices) {
      if (mask[i] === 0) continue;
      const [wx, wy] = model.posAt(i);
      const [sx, sy] = worldToScreen(cam, w, h, wx, wy);
      if (sx < -8 || sy < -8 || sx > w + 8 || sy > h + 8) continue;
      const id = model.idAt(i);
      const meta = model.metaOf(id);
      const color =
        meta !== null
          ? nodeColor(
              { clusterId: meta.clusterId, energy: meta.energy, musicalKey: meta.musicalKey, brightness: meta.brightness },
              S.colorMode,
            )
          : '#8891a5';
      // hover 微光（§203）。
      if (id === hoverRef.current) {
        ctx.shadowColor = withAlpha(color, 0.9);
        ctx.shadowBlur = 10;
      }
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(sx, sy, nodeR, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
      // §225 当前播放指示（静态双环；§226 只要 TrackId）。
      if (S.currentTrackId !== null && id === S.currentTrackId) {
        ctx.strokeStyle = 'rgba(255,214,150,0.95)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(sx, sy, nodeR + 3.5, 0, Math.PI * 2);
        ctx.stroke();
      }
      // §132 搜索高亮 ring。
      if (S.searchMatchIds.has(id)) {
        ctx.strokeStyle = 'rgba(160,200,255,0.8)';
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.arc(sx, sy, nodeR + 5, 0, Math.PI * 2);
        ctx.stroke();
      }
      // §203 选中 ring/halo。
      if (id === S.selectedTrackId) {
        ctx.strokeStyle = 'rgba(235,242,255,0.95)';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(sx, sy, nodeR + 4, 0, Math.PI * 2);
        ctx.stroke();
      }
      drawn.push({ id, x: sx, y: sy });
    }

    // 标签（near LOD §206）：优先级 selected > hovered > search >
    // nearby（§208）；矩形碰撞时隐藏低优先级（§207）；预算上限。
    ctx.font = '12px system-ui, sans-serif';
    ctx.textAlign = 'left';
    const labelRects: Array<[number, number, number, number]> = [];
    const putLabel = (sx: number, sy: number, text: string, strong: boolean): boolean => {
      const tw = ctx.measureText(text).width;
      const rect: [number, number, number, number] = [sx + 8, sy - 8, tw + 8, 16];
      for (const r of labelRects) {
        if (
          rect[0] < r[0] + r[2] &&
          r[0] < rect[0] + rect[2] &&
          rect[1] < r[1] + r[3] &&
          r[1] < rect[1] + rect[3]
        ) {
          return false;
        }
      }
      labelRects.push(rect);
      ctx.fillStyle = strong ? 'rgba(240,246,255,0.96)' : 'rgba(214,224,244,0.78)';
      ctx.fillText(text, sx + 8, sy + 4);
      return true;
    };
    const labelOf = (id: number): string => {
      const meta = model.metaOf(id);
      if (!meta) return `#${id}`;
      return meta.title ?? meta.artist ?? `#${meta.trackId}`;
    };
    const isPriority = (id: number): boolean =>
      id === S.selectedTrackId || id === hoverRef.current || S.searchMatchIds.has(id);
    if (cam.zoom >= LABEL_LOD_ZOOM) {
      let budget = 60;
      for (const d of drawn) {
        if (budget <= 0) break;
        if (isPriority(d.id) && putLabel(d.x, d.y, labelOf(d.id), true)) budget--;
      }
      for (const d of drawn) {
        if (budget <= 0) break;
        if (!isPriority(d.id) && putLabel(d.x, d.y, labelOf(d.id), false)) budget--;
      }
    } else {
      // 任何缩放下 selected/hovered 标签都在（§208 最高优先级）。
      for (const d of drawn) {
        if (d.id === S.selectedTrackId || d.id === hoverRef.current) {
          putLabel(d.x, d.y, labelOf(d.id), true);
        }
      }
    }
  }, [model]);

  drawRef.current = draw;

  // ---- 视图动画（§133 240ms；reduced motion 即时；结束即停 §199）----
  const animateTo = useCallback(
    (to: Camera) => {
      const target = clampCamera(to);
      if (stateRef.current.reducedMotion) {
        camRef.current = target;
        requestRender();
        syncStoreSoon();
        return;
      }
      animRef.current = { from: { ...camRef.current }, to: target, start: performance.now(), dur: 240 };
      const step = (): void => {
        const a = animRef.current;
        if (!a) return;
        const t = Math.min(1, (performance.now() - a.start) / a.dur);
        const e = 1 - (1 - t) * (1 - t) * (1 - t);
        camRef.current = clampCamera({
          zoom: a.from.zoom + (a.to.zoom - a.from.zoom) * e,
          panX: a.from.panX + (a.to.panX - a.from.panX) * e,
          panY: a.from.panY + (a.to.panY - a.from.panY) * e,
        });
        requestRender();
        if (t < 1) requestAnimationFrame(step);
        else {
          animRef.current = null;
          syncStoreSoon();
        }
      };
      requestAnimationFrame(step);
    },
    [requestRender, syncStoreSoon],
  );

  // 选中 → 拉邻居（§159 真选中才 IPC；一次一批 §229）。
  useEffect(() => {
    let alive = true;
    neighborsRef.current = [];
    if (selectedTrackId !== null) {
      model
        .neighborsFor(selectedTrackId, NEIGHBOR_FETCH_K)
        .then((rows) => {
          if (alive) {
            neighborsRef.current = rows;
            requestRender();
          }
        })
        .catch(() => undefined);
    }
    requestRender();
    return () => {
      alive = false;
    };
  }, [selectedTrackId, model, requestRender]);

  // 工具栏命令（§138：Reset/Fit/Focus Selected/Fit Cluster §140）。
  useEffect(() => {
    if (!command) return;
    const wrap = wrapRef.current;
    if (!wrap) return;
    const w = wrap.clientWidth;
    const h = wrap.clientHeight;
    if (command.kind === 'reset') {
      animateTo({ zoom: 1, panX: 0, panY: 0 });
    } else if (command.kind === 'fit') {
      animateTo(model.overview ? fitCamera(model.overview.bounds, w, h) : { zoom: 1, panX: 0, panY: 0 });
    } else if (command.kind === 'focusSelected' && selectedTrackId !== null) {
      const i = model.indexOf(selectedTrackId);
      if (i >= 0) {
        const [px, py] = model.posAt(i);
        animateTo({ zoom: Math.max(camRef.current.zoom, 3), panX: px, panY: py });
      }
    } else if (command.kind === 'focusCluster' && selectedClusterId !== null && model.overview) {
      const c = model.overview.clusters.find((x) => x.clusterId === selectedClusterId);
      if (c?.centroid && c.radius !== null) {
        const r = Math.max(0.08, c.radius);
        animateTo(
          fitCamera(
            [c.centroid[0] - r, c.centroid[1] - r, c.centroid[0] + r, c.centroid[1] + r],
            w,
            h,
          ),
        );
      }
    }
  }, [command?.nonce]);

  // resize（§196 lifecycle；卸载清理帧/定时器——§197 隐藏即暂停）。
  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const ro = new ResizeObserver(() => requestRender());
    ro.observe(wrap);
    requestRender();
    return () => {
      ro.disconnect();
      window.clearTimeout(syncTimer.current);
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      rafRef.current = 0;
    };
  }, [requestRender]);

  // 数据/配色/过滤/高亮变化 → 重绘一帧。
  useEffect(() => {
    requestRender();
  }, [model, colorMode, filterKey, searchMatchIds, currentTrackId, requestRender]);

  // ---- 交互（§134–§137/§156–§158：全部本地）----
  const onPointerDown = (e: React.PointerEvent): void => {
    if (e.button !== 0) return;
    (e.target as Element).setPointerCapture?.(e.pointerId);
    dragRef.current = { sx: e.clientX, sy: e.clientY, moved: false };
  };

  const onPointerMove = (e: React.PointerEvent): void => {
    const wrap = wrapRef.current;
    const canvas = canvasRef.current;
    if (!wrap || !canvas) return;
    const rect = wrap.getBoundingClientRect();
    if (dragRef.current) {
      const dx = e.clientX - dragRef.current.sx;
      const dy = e.clientY - dragRef.current.sy;
      if (Math.abs(dx) + Math.abs(dy) > 3) dragRef.current.moved = true;
      const k = camRef.current.zoom * (Math.min(rect.width, rect.height) / 2);
      camRef.current = clampCamera({
        zoom: camRef.current.zoom,
        panX: camRef.current.panX - dx / k,
        panY: camRef.current.panY - dy / k,
      });
      dragRef.current.sx = e.clientX;
      dragRef.current.sy = e.clientY;
      animRef.current = null;
      requestRender();
      syncStoreSoon();
      return;
    }
    // hover：本地 hit testing（§158 不发 IPC）。
    const [wx, wy] = screenToWorld(
      camRef.current,
      rect.width,
      rect.height,
      e.clientX - rect.left,
      e.clientY - rect.top,
    );
    const hitR = 10 / (camRef.current.zoom * (Math.min(rect.width, rect.height) / 2));
    const id = model.hitTest(wx, wy, hitR);
    if (id !== hoverRef.current) {
      hoverRef.current = id;
      canvas.style.cursor = id !== null ? 'pointer' : 'grab';
      onHoverTrack(id);
      requestRender();
    }
  };

  const onPointerUp = (e: React.PointerEvent): void => {
    const drag = dragRef.current;
    dragRef.current = null;
    if (!drag || drag.moved) return; // §157 click 与 pan 区分
    const wrap = wrapRef.current;
    if (!wrap) return;
    const rect = wrap.getBoundingClientRect();
    const [wx, wy] = screenToWorld(
      camRef.current,
      rect.width,
      rect.height,
      e.clientX - rect.left,
      e.clientY - rect.top,
    );
    const hitR = 10 / (camRef.current.zoom * (Math.min(rect.width, rect.height) / 2));
    const id = model.hitTest(wx, wy, hitR);
    const st = useMapStore.getState();
    if (id !== null) {
      st.selectTrack(id);
      const meta = model.metaOf(id);
      st.selectCluster(meta ? meta.clusterId : null);
    } else {
      st.selectTrack(null);
      st.selectCluster(null);
    }
  };

  const onWheel = (e: React.WheelEvent): void => {
    e.preventDefault();
    const wrap = wrapRef.current;
    if (!wrap) return;
    const rect = wrap.getBoundingClientRect();
    const factor = e.deltaY < 0 ? 1.18 : 1 / 1.18;
    const next = zoomAt(
      camRef.current,
      rect.width,
      rect.height,
      e.clientX - rect.left,
      e.clientY - rect.top,
      factor,
    );
    if (next.zoom === camRef.current.zoom) return; // §135 边界
    animRef.current = null;
    camRef.current = next;
    requestRender();
    syncStoreSoon();
  };

  return (
    <div ref={wrapRef} className="map-canvas-wrap" role="img" aria-label="Music Map canvas">
      <canvas
        ref={canvasRef}
        className="map-canvas"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={() => {
          hoverRef.current = null;
          onHoverTrack(null);
          requestRender();
        }}
        onWheel={onWheel}
      />
      <div className="map-zoom-badge">{Math.round(zoomBadge * 100)}%</div>
    </div>
  );
}
