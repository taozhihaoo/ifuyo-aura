// DreamscapeRenderer（§128–§132/§144–§153/§265–§267）：Three.js 装配 +
// 纯逻辑 engine 驱动。生命周期：create → loadWorld → render ⇄ pause →
// dispose；context lost → paused → restored 重建（§151）。
import { Scene, PerspectiveCamera, WebGLRenderer, Clock } from 'three';
import {
  AutoQuality,
  GenerationGate,
  Lifecycle,
  SmoothedValue,
  lerp,
  shouldRender,
  targetFps,
  type QualityMode,
  type QualitySettings,
  QUALITY_SETTINGS,
} from './engine';
import { buildScene, Disposables, terrainHeight, type BuiltScene } from './scene';
import type { DreamscapeStateDto, SceneManifest } from '../../ipc/dreamscape';

export interface RendererStats {
  fps: number;
  frameTimeMs: number;
  objects: number;
  triangles: number;
}

export class DreamscapeRenderer {
  private lifecycle = new Lifecycle();
  private gate = new GenerationGate();
  private auto = new AutoQuality();
  private disposables = new Disposables();
  private scene: Scene | null = null;
  private camera: PerspectiveCamera | null = null;
  private gl: WebGLRenderer | null = null;
  private built: BuiltScene | null = null;
  private manifest: SceneManifest | null = null;
  private clock = new Clock();
  private raf = 0;
  private lastFrame = 0;
  private frames = 0;
  private fpsWindow = 0;
  private qualityMode: QualityMode = 'medium';
  private settings: QualitySettings = QUALITY_SETTINGS.medium;
  private reducedMotion = false;
  private playing = true;
  private documentVisible = true;
  private smoothSun = new SmoothedValue(0.5, 0.8);
  private smoothFog = new SmoothedValue(0.2, 0.8);
  private orbitAngle = 0;
  private onStats: ((s: RendererStats) => void) | null = null;

  /** 状态读取（§129）。 */
  get state(): string {
    return this.lifecycle.get();
  }

  setStatsListener(cb: (s: RendererStats) => void): void {
    this.onStats = cb;
  }

  /** §129/§277 create：GL 不可用（无 WebGL/上下文创建失败）→ false，
   *  调用方进入 fallback；不抛异常。 */
  create(canvas: HTMLCanvasElement): boolean {
    if (!this.lifecycle.transition('created')) return false;
    try {
      this.gl = new WebGLRenderer({ canvas, antialias: false, powerPreference: 'low-power' });
    } catch {
      return false;
    }
    this.applyPixelRatio();
    this.scene = new Scene();
    this.camera = new PerspectiveCamera(55, 1, 0.1, 600);
    // §151：context loss → paused；restored → 重建 GPU 资源。
    canvas.addEventListener('webglcontextlost', this.onContextLost);
    canvas.addEventListener('webglcontextrestored', this.onContextRestored);
    this.clock.start();
    return true;
  }

  private onContextLost = (e: Event): void => {
    e.preventDefault();
    if (this.lifecycle.get() === 'rendering') this.lifecycle.transition('contextLost');
  };

  private onContextRestored = (): void => {
    // §151：重建 GPU 资源（几何/材质全部经 Disposables 重构建）。
    const scene = this.scene;
    if (this.manifest && scene) {
      this.disposables.disposeAll();
      this.built = buildScene(this.manifest, this.settings, this.disposables);
      scene.add(this.built.root);
    }
    if (this.lifecycle.get() === 'contextLost') this.lifecycle.transition('rendering');
  };

  /** §129 loadWorld：旧场景整体替换（§160 dispose-then-load）。 */
  loadWorld(manifest: SceneManifest): boolean {
    if (!this.lifecycle.transition('loading')) return false;
    const id = this.gate.begin();
    const scene = this.scene;
    const gl = this.gl;
    const camera = this.camera;
    if (!this.gate.isCurrent(id) || !scene || !gl || !camera) return false;
    // §160：先释放旧世界，再装新世界（不叠加）。
    if (this.built) {
      scene.remove(this.built.root);
      this.disposables.disposeAll();
      this.built = null;
    }
    this.manifest = manifest;
    this.built = buildScene(manifest, this.settings, this.disposables);
    scene.add(this.built.root);
    this.applyEnvironment(manifest, 1);
    // 初始相机位（§65 passive；有界 §68）。
    const lm = manifest.landmarks[0];
    const cx = lm ? lm.x : 0;
    const cz = lm ? lm.z : 0;
    camera.position.set(cx + 18, 14, cz + 18);
    camera.lookAt(cx, terrainHeight(manifest.terrain, cx, cz) + 2, cz);
    this.lifecycle.transition('ready');
    return true;
  }

  /** §129 render loop 启动（ready → rendering）。 */
  start(): boolean {
    if (!this.lifecycle.transition('rendering')) return false;
    this.lastFrame = 0;
    const loop = () => {
      this.raf = requestAnimationFrame(loop);
      this.frame();
    };
    this.raf = requestAnimationFrame(loop);
    return true;
  }

  private frame(): void {
    const now = performance.now();
    const fps = targetFps(this.playing, this.reducedMotion, this.documentVisible);
    if (!shouldRender(now, this.lastFrame, fps)) return;
    const dt = this.lastFrame === 0 ? 0.016 : Math.min((now - this.lastFrame) / 1000, 0.25);
    this.lastFrame = now;
    if (this.lifecycle.get() !== 'rendering' || !this.built || !this.manifest || !this.camera) return;

    // 相机（§65 passive orbit；§69 极慢；§70 reduced-motion 近静止；§68 有界）。
    const center = this.cameraCenter();
    if (!this.reducedMotion) {
      this.orbitAngle += dt * 0.02;
      const r = 26;
      const y = 13 + Math.sin(now * 0.00008) * 1.5; // breathing（§66）
      this.camera.position.set(
        center.x + Math.cos(this.orbitAngle) * r,
        Math.max(y, terrainHeight(this.manifest.terrain, this.camera.position.x, this.camera.position.z) + 3),
        center.z + Math.sin(this.orbitAngle) * r,
      );
      this.camera.lookAt(center.x, 2, center.z);
    }

    // 环境本地插值（§265/§266：目标来自快照，平滑在本地）。
    this.applyEnvironment(this.manifest, Math.min(1, dt * 0.8));

    // 粒子漂移（§70 reduced-motion 停）。
    if (this.built.particles && this.built.particleBase && !this.reducedMotion) {
      const pos = this.built.particles.geometry.getAttribute('position');
      if (pos) {
        for (let i = 0; i < pos.count; i += 1) {
          const baseY = this.built.particleBase[i * 3 + 1] ?? 0;
          pos.setY(i, baseY + Math.sin(now * 0.0006 + i) * 0.5);
        }
        pos.needsUpdate = true;
      }
    }

    const scene = this.scene;
    if (scene && this.camera && this.gl) this.gl.render(scene, this.camera);

    // Auto 质量（§142/§143）。
    if (this.qualityMode === 'auto') {
      const frameMs = dt * 1000;
      const next = this.auto.sample(frameMs, now);
      if (next) this.applySettings(QUALITY_SETTINGS[next]);
    }

    // 诊断（§155 本地）。
    this.frames += 1;
    if (now - this.fpsWindow >= 2000) {
      const fps = Math.round((this.frames * 1000) / (now - this.fpsWindow));
      this.frames = 0;
      this.fpsWindow = now;
      this.onStats?.({
        fps,
        frameTimeMs: Math.round(dt * 1000),
        objects: this.built.root.children.length,
        triangles: this.gl?.info.render.triangles ?? 0,
      });
    }
  }

  private cameraCenter(): { x: number; z: number } {
    const lm = this.manifest?.landmarks[0];
    return { x: lm ? lm.x : 0, z: lm ? lm.z : 0 };
  }

  /** 环境/光/雾向目标推进（§163/§164 渐变；§138 无重编译只改参数）。 */
  private applyEnvironment(m: SceneManifest, t: number): void {
    if (!this.built) return;
    const sun = lerp(this.smoothSun.get(), m.light.sunIntensity, t);
    this.smoothSun.step(m.light.sunIntensity, 0.5);
    this.built.sun.intensity = 0.3 + sun * 1.2;
    this.built.ambient.intensity = 0.25 + m.light.ambientIntensity * 0.6;
    this.built.fill.intensity = 0.15 + m.light.horizonGlow * 0.3;
    const fog = lerp(this.smoothFog.get(), m.atmosphere.fog, t);
    this.smoothFog.step(m.atmosphere.fog, 0.5);
    this.built.fog.near = 40 + (1 - fog) * 60;
    this.built.fog.far = 140 + (1 - fog) * 160;
  }

  /** 环境目标更新（§129；由 state DTO 驱动，不重建场景 §138）。 */
  updateEnvironment(state: DreamscapeStateDto): void {
    if (!this.manifest) return;
    this.manifest = {
      ...this.manifest,
      light: {
        sunIntensity: state.sunIntensity,
        ambientIntensity: state.ambientIntensity,
        horizonGlow: state.horizonGlow,
      },
      atmosphere: {
        fog: state.fog,
        wind: state.wind,
        mistBias: this.manifest.atmosphere.mistBias,
      },
    };
  }

  setPlaying(playing: boolean): void {
    this.playing = playing;
  }

  setDocumentVisible(visible: boolean): void {
    this.documentVisible = visible;
  }

  setReducedMotion(reduced: boolean): void {
    this.reducedMotion = reduced;
  }

  /** §141 质量档；auto 内部用 AutoQuality（§143 迟滞）。 */
  setQuality(mode: QualityMode): void {
    this.qualityMode = mode;
    if (mode !== 'auto') {
      this.applySettings(QUALITY_SETTINGS[mode]);
    }
  }

  private applySettings(s: QualitySettings): void {
    this.settings = s;
    this.applyPixelRatio();
    // 地形细分变化需重建地形——重构建当前世界（§138 只更新参数不改 shader）。
    const scene = this.scene;
    if (this.manifest && scene) {
      this.disposables.disposeAll();
      this.built = buildScene(this.manifest, s, this.disposables);
      scene.add(this.built.root);
    }
  }

  private applyPixelRatio(): void {
    if (!this.gl) return;
    // §150 pixelRatio cap。
    this.gl.setPixelRatio(Math.min(window.devicePixelRatio || 1, this.settings.pixelRatioCap));
  }

  /** §129 resize（调用方 coalesce §148）。 */
  resize(width: number, height: number): void {
    if (!this.gl || !this.camera) return;
    this.gl.setSize(width, height, false);
    this.camera.aspect = width / Math.max(1, height);
    this.camera.updateProjectionMatrix();
  }

  pause(): boolean {
    return this.lifecycle.transition('paused');
  }

  resume(): boolean {
    return this.lifecycle.transition('rendering');
  }

  /** §129/§158 dispose：RAF/监听/GL 资源全部释放。 */
  dispose(): void {
    cancelAnimationFrame(this.raf);
    if (this.gl) {
      const canvas = this.gl.domElement;
      canvas.removeEventListener('webglcontextlost', this.onContextLost);
      canvas.removeEventListener('webglcontextrestored', this.onContextRestored);
    }
    this.disposables.disposeAll();
    this.built = null;
    this.gl?.dispose();
    this.gl = null;
    this.scene = null;
    this.camera = null;
    this.manifest = null;
    this.lifecycle.transition('disposed');
  }
}
