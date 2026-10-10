// Dreamscape 视图（v2 光场）：极光/星野/镜面/光尘由曲目特征与实时电平驱动。
// §270 数据走 ref 不进 React state；§274 由 App 级 FeatureBoundary 包裹；
// §275 GL 不可用 → CSS fallback，不悬挂 loading；§293 卸载即停机。
import { useEffect, useRef } from 'react';
import { LightFieldRenderer } from '../../visual/dreamscape/LightFieldRenderer';
import { QUALITY_SETTINGS, AutoQuality, clamp01, targetFps } from '../../visual/dreamscape/engine';
import type { LightField } from '../../ipc/dreamscape';
import { useUiStore, usePlayerStore, useSettingsStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { lyricsForPath } from '../../ipc/remoteLyrics';
import type { TrackFeatures } from '../../ipc/types';
import type { LyricsDto } from '../../ipc/client';
import { errorToI18nKey } from '../../ipc/errors';
import { useTranslation } from '../../i18n/react';
import { useCurrentTrackId } from '../../components/Artwork';

/** 未分析时的中性形态值（不是"伪造的特征"，而是不偏向任何形态）。 */
const NEUTRAL = 0.5;

/** 电平映射：引擎给的是未归一 RMS（满幅正弦约 0.35，高频带远小于低频带）。 */
function mapLevel(rms: number, low: number, mid: number, high: number) {
  return {
    level: clamp01(rms * 2.6),
    low: clamp01(low * 2.8),
    mid: clamp01(mid * 4.5),
    high: clamp01(high * 9.0),
  };
}

/** 节拍包络：由分析出的 bpm + 首拍偏移对齐真实拍点；无可靠节拍则不假装踩点。 */
function beatPulse(f: TrackFeatures | null, positionSeconds: number): number {
  const bpm = f?.bpm ?? null;
  if (bpm === null || bpm < 40 || bpm > 220) return 0;
  const beats = (positionSeconds - (f?.firstBeatOffsetSeconds ?? 0)) * (bpm / 60);
  if (!Number.isFinite(beats) || beats < 0) return 0;
  return Math.exp(-3.2 * (beats - Math.floor(beats)));
}

export function DreamscapeView() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<LightFieldRenderer | null>(null);
  const fieldRef = useRef<LightField | null>(null);
  const featuresRef = useRef<TrackFeatures | null>(null);
  const setStatus = useUiStore((s) => s.setDreamscapeStatus);
  const setNotice = useUiStore((s) => s.setNotice);
  const reducedMotion = useSettingsStore((s) => s.reducedMotion);
  const reducedRef = useRef(reducedMotion);
  reducedRef.current = reducedMotion;
  const { t } = useTranslation();
  const trackId = useCurrentTrackId();
  const trackPath = usePlayerStore((s) => s.snapshot?.trackPath ?? null);
  const nowRef = useRef<HTMLParagraphElement>(null);
  const nextRef = useRef<HTMLParagraphElement>(null);
  const lyricsRef = useRef<LyricsDto | null>(null);
  const lineIdxRef = useRef(-1);

  // 换曲 → 重取歌词（同步到 DOM；无歌词 → 舞台隐藏并给出如实提示）。
  // 走 lyricsForPath 而不是 lyricsForTrack(trackId)：远程曲目的 trackPath 是
  // URL、没有本地 TrackId，按 id 取词会让在线歌永远显示"没有歌词"。
  useEffect(() => {
    lineIdxRef.current = -1;
    if (trackPath === null) {
      lyricsRef.current = null;
      return;
    }
    let alive = true;
    void lyricsForPath(trackPath)
      .then((l) => {
        if (!alive) return;
        lyricsRef.current = l;
        paintLyrics(nowRef.current, nextRef.current, lineIdxRef, l, 0, t('lyrics.none'));
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [trackPath, t]);

  // 换曲 → 重取特征（慢通路；电平/位置在渲染循环里取）。
  useEffect(() => {
    if (trackId === null) {
      featuresRef.current = null;
      return;
    }
    let alive = true;
    void ipc
      .trackFeatures(trackId)
      .then((f) => {
        if (alive) featuresRef.current = f;
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [trackId]);

  // ---- 渲染器生命周期 ----
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    setStatus('loading');
    void ipc.dreamscapeOpen().catch(() => undefined); // §257 幂等
    const renderer = LightFieldRenderer.create(canvas);
    rendererRef.current = renderer;
    if (!renderer) {
      // §275/§278：GL 不可用 = 立即 fallback，不悬挂 loading。
      setStatus('fallback');
      return () => {
        rendererRef.current = null;
        void ipc.dreamscapeClose().catch(() => undefined);
        setStatus('idle');
      };
    }

    let alive = true;
    let stateTimer = 0;
    const auto = new AutoQuality();
    let inFlight = false;

    // §151：TDR/显卡重置/唤醒会让 WebGL 上下文丢失。渲染器内部重建 GL 对象
    // 并自动续跑循环；丢失期间这里切到 CSS fallback，避免"冻在一帧/纯黑"。
    renderer.onContextChange = (lost) => {
      if (!alive) return;
      setStatus(lost ? 'fallback' : 'ready');
    };

    const applyEnv = (s: {
      sunIntensity: number;
      ambientIntensity: number;
      horizonGlow: number;
      fog: number;
      wind: number;
    }) => {
      renderer.setEnvironment({
        sunIntensity: s.sunIntensity,
        ambientIntensity: s.ambientIntensity,
        horizonGlow: s.horizonGlow,
        fog: s.fog,
        wind: s.wind,
        mistBias: 0.05,
      });
    };

    void (async () => {
      try {
        const field = await ipc.dreamscapeManifest(); // validation-first（§253）
        if (!alive) return;
        fieldRef.current = field;
        applyEnv(await ipc.dreamscapeState());
        if (!alive) return;
        setStatus('ready');
        // §263/§264：低频环境推进（2s = 30 逻辑分钟），不进逐帧通路。
        stateTimer = window.setInterval(() => {
          void ipc
            .dreamscapeAdvance(30)
            .then((s) => {
              if (alive) applyEnv(s);
            })
            .catch(() => undefined);
        }, 2000);
      } catch (e) {
        if (!alive) return;
        setStatus('fallback');
        setNotice(errorToI18nKey(e));
      }
    })();

    let visible = document.visibilityState === 'visible';
    const onVis = () => {
      visible = document.visibilityState === 'visible';
    };
    document.addEventListener('visibilitychange', onVis);

    renderer.start(() => {
      const now = performance.now();
      const playing = usePlayerStore.getState().snapshot?.state === 'playing';
      if (!renderer.shouldDraw(now, playing, reducedRef.current, visible)) return;
      const field = fieldRef.current;
      if (field === null || inFlight) return;
      inFlight = true;
      const level = playing ? ipc.playerLevel().catch(() => null) : Promise.resolve(null);
      const position = playing ? ipc.playerPosition().catch(() => 0) : Promise.resolve(0);
      void Promise.all([level, position])
        .then(([lv, pos]) => {
          const bands = lv ? mapLevel(lv.rms, lv.low, lv.mid, lv.high) : null;
          const f = featuresRef.current;
          renderer.resize(canvas);
          renderer.render(
            canvas,
            field,
            {
              playing,
              pulse: playing ? beatPulse(f, pos) : 0,
              energy: f?.energy ?? NEUTRAL,
              brightness:
                f?.brightness === null || f?.brightness === undefined
                  ? NEUTRAL
                  : clamp01(f.brightness * 2.4),
              level: bands?.level ?? 0,
              low: bands?.low ?? 0,
              mid: bands?.mid ?? 0,
              high: bands?.high ?? 0,
            },
            reducedRef.current,
          );
          paintLyrics(nowRef.current, nextRef.current, lineIdxRef, lyricsRef.current, pos, t('lyrics.none'));
          // 自动画质必须看"达成率"，不能看 GPU 提交耗时（~1ms ⇒ 永远判快，
          // 慢机器上失去降档保护）。单 quad 片元渲染器被节流到目标帧率，
          // 因此信号 = 实测帧率相对当前目标掉了多少；只降不升——
          // 升档需要真实 GPU 耗时（EXT_disjoint_timer_query_webgl2），没有
          // 就宁可停在当前档，不做基于猜测的升级。
          const nowMs = performance.now();
          const fpsNow = renderer.fps(nowMs);
          const target = targetFps(playing, reducedRef.current, visible);
          if (fpsNow > 0 && target > 0 && fpsNow < target * 0.7) {
            const next = auto.sample(1000 / fpsNow, nowMs);
            if (next) renderer.setQuality(QUALITY_SETTINGS[next]);
          }
          canvas.dataset.fps = String(fpsNow);
          if (fpsNow > 0) {
            // 1 秒窗口滚动时记一次驱动信号（诊断"晃眼"：看它是否逐帧跳）
            canvas.dataset.telemetry = JSON.stringify(renderer.smoothed());
          }
        })
        .catch((e: unknown) => {
          // 静默吞异常 = "为什么不画了"无从查起；与 data-fps 同类本地诊断。
          canvas.dataset.fieldError = String((e as Error)?.message ?? e).slice(0, 200);
        })
        .finally(() => {
          inFlight = false;
        });
    });

    return () => {
      alive = false;
      window.clearInterval(stateTimer);
      document.removeEventListener('visibilitychange', onVis);
      renderer.dispose(); // §293：stop/dispose 一并收尾
      void ipc.dreamscapeClose().catch(() => undefined); // §257 幂等
      rendererRef.current = null;
      fieldRef.current = null;
      setStatus('idle');
    };
  }, [setStatus, setNotice]);

  return (
    <section className="dreamscape" aria-label={t('dreamscape.title')}>
      <canvas ref={canvasRef} className="dreamscape-canvas" aria-hidden />
      <div className="dreamscape-hud" role="status">
        <span className="visually-hidden" aria-live="polite">
          {t('dreamscape.entered')}
        </span>
      </div>
      {/* 地面舞台：正在唱的一句偏左上，下一句预读偏右下（经典卡拉 OK 排布）。 */}
      <div className="lyric-stage" data-empty="true">
        <p className="lyric-now" ref={nowRef} aria-live="polite" />
        <p className="lyric-next" ref={nextRef} aria-hidden />
      </div>
      <DreamscapeHint />
    </section>
  );
}

/**
 * 歌词绘制（命令式）：只在行号变化时改 DOM。
 * `lineIdxRef` 记录已呈现的行，避免每帧重排。
 */
function paintLyrics(
  now: HTMLParagraphElement | null,
  next: HTMLParagraphElement | null,
  lineIdxRef: { current: number },
  lyrics: LyricsDto | null,
  positionSeconds: number,
  emptyText: string,
): void {
  if (!now || !next) return;
  const stage = now.parentElement;
  const lines = lyrics?.lines ?? [];
  if (lines.length === 0) {
    if (stage?.dataset.empty !== 'true') stage?.setAttribute('data-empty', 'true');
    if (now.textContent !== emptyText) now.textContent = emptyText;
    next.textContent = '';
    lineIdxRef.current = -1;
    return;
  }
  stage?.setAttribute('data-empty', 'false');
  if (!lyrics?.synced) {
    // 纯文本标签歌词：整段呈现在主行，无预读行。
    if (lineIdxRef.current !== -2) {
      now.textContent = lines.map((l) => l.text).join(' / ');
      next.textContent = '';
      lineIdxRef.current = -2;
    }
    return;
  }
  // 正在唱 = 最后一个 time <= position 的行
  let idx = -1;
  for (let i = 0; i < lines.length; i++) {
    const ts = lines[i]?.timeSeconds;
    if (typeof ts === 'number' && ts <= positionSeconds) idx = i;
    else break;
  }
  if (idx === lineIdxRef.current) return;
  lineIdxRef.current = idx;
  const safe = Math.max(idx, 0);
  now.textContent = lines[safe]?.text ?? '';
  next.textContent = lines[safe + 1]?.text ?? '';
  // 重触进入动画（同一行不换、新行才换）
  now.classList.remove('lyric-enter');
  void now.offsetWidth; // 强制回流以重启动画
  now.classList.add('lyric-enter');
}

function DreamscapeHint() {
  const status = useUiStore((s) => s.dreamscapeStatus);
  const { t } = useTranslation();
  if (status === 'ready') {
    return <p className="dreamscape-hint">{t('dreamscape.hint')}</p>;
  }
  if (status === 'fallback') {
    // §275：CSS 渐变 fallback（同 palette），光场数据本身仍有效。
    return <p className="dreamscape-hint">{t('dreamscape.fallback')}</p>;
  }
  return <p className="dreamscape-hint">{t('dreamscape.loading')}</p>;
}
