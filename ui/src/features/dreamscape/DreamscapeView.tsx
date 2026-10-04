// Dreamscape 视图（§203–§204 独立入口；§189/§190 与 M6 共存互不影响；
// §270 manifest 走 ref 不进 React state；§274 由 App 级 FeatureBoundary
// 包裹；§275 fallback 场景不白屏）。
import { useEffect, useRef } from 'react';
import { DreamscapeRenderer, type RendererStats } from '../../visual/dreamscape/DreamscapeRenderer';
import { useUiStore, usePlayerStore, useSettingsStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { errorToI18nKey } from '../../ipc/errors';
import { useTranslation } from '../../i18n/react';

export function DreamscapeView() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<DreamscapeRenderer | null>(null);
  const setStatus = useUiStore((s) => s.setDreamscapeStatus);
  const setNotice = useUiStore((s) => s.setNotice);
  const reducedMotion = useSettingsStore((s) => s.reducedMotion);
  const { t } = useTranslation();

  // ---- 渲染器生命周期（§293 卸载即停机）----
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    const renderer = new DreamscapeRenderer();
    rendererRef.current = renderer;
    let alive = true;
    let stateTimer = 0;
    setStatus('loading');
    renderer.setStatsListener((s: RendererStats) => {
      canvas.dataset.fps = String(s.fps); // §155 本地诊断（无上传）
    });

    void (async () => {
      try {
        if (!renderer.create(canvas)) return;
        const manifest = await ipc.dreamscapeManifest(); // validation-first（§253）
        if (!alive) return;
        if (!renderer.loadWorld(manifest)) return;
        const state = await ipc.dreamscapeState();
        if (!alive) return;
        renderer.updateEnvironment(state);
        renderer.setReducedMotion(reducedMotion);
        renderer.start();
        setStatus('ready');
        // §263/§264：低频环境推进（30 逻辑秒/2s 实时；非 60FPS IPC）。
        stateTimer = window.setInterval(() => {
          void ipc
            .dreamscapeAdvance(30)
            .then((s) => renderer.updateEnvironment(s))
            .catch(() => undefined);
        }, 2000);
      } catch (e) {
        // §275/§276：生成失败 → CSS fallback 场景；用户世界状态不动。
        if (alive) setStatus('fallback');
        setNotice(errorToI18nKey(e));
      }
    })();

    const onVis = () => renderer.setDocumentVisible(document.visibilityState === 'visible');
    document.addEventListener('visibilitychange', onVis);

    return () => {
      alive = false;
      window.clearInterval(stateTimer);
      document.removeEventListener('visibilitychange', onVis);
      renderer.dispose(); // §293：cancel/stop/dispose/unsubscribe
      rendererRef.current = null;
      setStatus('idle');
    };
    // reducedMotion 变化经独立 effect 透传，避免重建渲染器。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 播放状态 → 帧率档（§144/§145；§190 不影响播放本身）。
  useEffect(() => {
    const unsub = usePlayerStore.subscribe((s) => {
      rendererRef.current?.setPlaying(s.snapshot?.state === 'playing');
    });
    return unsub;
  }, []);

  // reduced motion 透传（§70/§287）。
  useEffect(() => {
    rendererRef.current?.setReducedMotion(reducedMotion);
  }, [reducedMotion]);

  return (
    <section className="dreamscape" aria-label={t('dreamscape.title')}>
      <canvas ref={canvasRef} className="dreamscape-canvas" aria-hidden />
      <div className="dreamscape-hud" role="status">
        <span className="visually-hidden" aria-live="polite">
          {t('dreamscape.entered')}
        </span>
      </div>
      <DreamscapeHint />
    </section>
  );
}

function DreamscapeHint() {
  const status = useUiStore((s) => s.dreamscapeStatus);
  const { t } = useTranslation();
  if (status === 'ready') {
    return <p className="dreamscape-hint">{t('dreamscape.hint')}</p>;
  }
  if (status === 'fallback') {
    // §275：atmospheric CSS fallback（同 palette 渐变），世界数据仍有效。
    return <p className="dreamscape-hint">{t('dreamscape.fallback')}</p>;
  }
  // §194/§278：真实 loading 态（轻量环境描述，无假进度条）。
  return <p className="dreamscape-hint">{t('dreamscape.loading')}</p>;
}
