// Aurora（§56–§75/§132–§137）：Three.js 全屏视觉背景。
// 架构：单一 WebGL canvas（§58）、生命周期钩子（§59）、播放/暂停节流（§60）、
// reduced-motion（§61）、预分配 uniforms（§62）、telemetry 合同（§63）、
// graceful CSS fallback（§69）、context loss 处理（§70）、稳定 seed（§73/§74）。
import { useEffect, useRef, useState } from 'react';
import { usePlayerStore } from '../state/stores';
import { useSettingsStore } from '../state/stores';

/** §63 VisualTelemetry：最小合同；M8 字段（bpm/key/energy）禁止伪造。 */
export interface VisualTelemetry {
  isPlaying: boolean;
  progress: number;
  volume: number;
  sourceLive: boolean;
}

/** §74：visual seed = 简单稳定 hash（非 M7 Dreamscape seed）。 */
export function visualSeed(trackIdentity: string): number {
  let h = 0x9e3779b9;
  for (let i = 0; i < trackIdentity.length; i += 1) {
    h ^= trackIdentity.charCodeAt(i);
    h = Math.imul(h, 0x85ebca6b);
    h ^= h >>> 13;
  }
  return h >>> 0;
}

/** §60/§132：FPS 目标。playing 30fps，idle 8fps，reduced-motion 2fps。 */
function targetFps(playing: boolean, reducedMotion: boolean): number {
  if (reducedMotion) return 2;
  return playing ? 30 : 8;
}

/** §179/§180：VISUAL_TEST_MODE —— URL 带 ?visualTest 时启用。固定
 * seed/time/telemetry，只渲染一帧即停（截图可重复，无 rAF 抖动）。 */
export const VISUAL_TEST_TIME_SECONDS = 42;
function visualTestMode(): boolean {
  return (
    typeof window !== 'undefined' &&
    new URLSearchParams(window.location.search).has('visualTest')
  );
}

export function AuroraCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [webglFailed, setWebglFailed] = useState(false);
  const snapshot = usePlayerStore((s) => s.snapshot);
  const reducedMotion = useSettingsStore((s) => s.reducedMotion);

  const telemetryRef = useRef<VisualTelemetry>({
    isPlaying: false,
    progress: 0,
    volume: 1,
    sourceLive: false,
  });
  const seedRef = useRef(visualSeed('aura-default'));

  // telemetry 同步（低频，不经 React 渲染 §10/§62）。
  useEffect(() => {
    telemetryRef.current = {
      isPlaying: snapshot?.state === 'playing',
      progress:
        snapshot?.durationSeconds && snapshot.durationSeconds > 0
          ? Math.min(1, snapshot.positionSeconds / snapshot.durationSeconds)
          : 0,
      volume: snapshot?.volume ?? 1,
      sourceLive: false,
    };
    if (snapshot?.trackPath) {
      seedRef.current = visualSeed(snapshot.trackPath);
    }
  }, [snapshot]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    let disposed = false;
    let raf = 0;
    let gl: WebGLRenderingContext | null = null;
    let program: WebGLProgram | null = null;
    const startTime = performance.now();
    let lastFrame = 0;

    // §69：WebGL 不可用 → CSS fallback（不白屏）。
    gl = canvas.getContext('webgl', { antialias: false, alpha: true });
    if (!gl) {
      setWebglFailed(true);
      return undefined;
    }

    const vsSrc = `
      attribute vec2 a_pos;
      void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }
    `;
    // §68：紧凑 fragment —— soft organic luminous（品牌 palette 派生色）。
    const fsSrc = `
      precision mediump float;
      uniform vec2 u_res;
      uniform float u_time;
      uniform float u_play;      // 0=idle 1=playing
      uniform float u_progress;
      uniform float u_reduced;   // 1=reduced motion
      uniform vec3 u_seed;       // 稳定 seed 派生

      float hash(vec2 p) {
        return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
      }
      float noise(vec2 p) {
        vec2 i = floor(p); vec2 f = fract(p);
        vec2 u = f * f * (3.0 - 2.0 * f);
        return mix(mix(hash(i), hash(i + vec2(1,0)), u.x),
                   mix(hash(i + vec2(0,1)), hash(i + vec2(1,1)), u.x), u.y);
      }
      float fbm(vec2 p) {
        float v = 0.0; float a = 0.5;
        for (int i = 0; i < 4; i++) { v += a * noise(p); p *= 2.0; a *= 0.5; }
        return v;
      }

      void main() {
        vec2 uv = gl_FragCoord.xy / u_res.xy;
        float t = u_time * mix(0.03, 0.12, u_play) * mix(1.0, 0.15, u_reduced);
        vec2 q = uv * 3.0 + u_seed.xy * 0.37;
        float n = fbm(q + vec2(t, -t * 0.6) + u_progress * 0.8);
        float glow = smoothstep(0.25, 0.95, n);
        // 品牌 palette：deep navy base + rose/violet/sky luminous（tokens.ts 派生）。
        vec3 base = vec3(0.043, 0.055, 0.078);
        vec3 rose = vec3(0.949, 0.471, 0.624);
        vec3 violet = vec3(0.545, 0.486, 0.965);
        vec3 sky = vec3(0.361, 0.784, 0.949);
        vec3 col = base;
        col += rose * glow * 0.16 * (0.4 + 0.6 * u_play);
        col += violet * fbm(q * 1.7 - t * 0.5) * 0.12;
        col += sky * smoothstep(0.6, 1.0, fbm(q * 0.6 + t * 0.3)) * 0.08;
        // vignette。
        float vig = smoothstep(1.25, 0.35, length(uv - vec2(0.5, 0.55)));
        col *= mix(0.85, 1.0, vig);
        gl_FragColor = vec4(col, 1.0);
      }
    `;

    const compile = (type: number, src: string): WebGLShader | null => {
      const sh = gl!.createShader(type);
      if (!sh) return null;
      gl!.shaderSource(sh, src);
      gl!.compileShader(sh);
      if (!gl!.getShaderParameter(sh, gl!.COMPILE_STATUS)) {
        return null;
      }
      return sh;
    };

    const vs = compile(gl.VERTEX_SHADER, vsSrc);
    const fs = compile(gl.FRAGMENT_SHADER, fsSrc);
    program = gl.createProgram();
    if (!vs || !fs || !program) {
      setWebglFailed(true);
      return undefined;
    }
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      setWebglFailed(true);
      return undefined;
    }
    gl.useProgram(program);
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(program, 'a_pos');
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    const uRes = gl.getUniformLocation(program, 'u_res');
    const uTime = gl.getUniformLocation(program, 'u_time');
    const uPlay = gl.getUniformLocation(program, 'u_play');
    const uProgress = gl.getUniformLocation(program, 'u_progress');
    const uReduced = gl.getUniformLocation(program, 'u_reduced');
    const uSeed = gl.getUniformLocation(program, 'u_seed');

    const testMode = visualTestMode();
    if (testMode) {
      seedRef.current = visualSeed('visual-test');
    }

    // §134/§135：内部分辨率 cap（避免 4K DPI GPU 失控）。
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const w = Math.max(1, Math.floor(rect.width * dpr));
      const h = Math.max(1, Math.floor(rect.height * dpr));
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
        gl!.viewport(0, 0, w, h);
      }
    };
    resize();

    let resizeTimer = 0;
    const onResize = () => {
      window.clearTimeout(resizeTimer);
      resizeTimer = window.setTimeout(resize, 150); // §136 debounce
    };
    window.addEventListener('resize', onResize);

    // §70：context loss → fallback，不 crash。
    const onLost = (e: Event) => {
      e.preventDefault();
      setWebglFailed(true);
    };
    canvas.addEventListener('webglcontextlost', onLost);

    const drawFrame = (elapsedSeconds: number, tel: VisualTelemetry, reduced: boolean) => {
      resize();
      gl!.uniform2f(uRes, canvas!.width, canvas!.height);
      gl!.uniform1f(uTime, elapsedSeconds);
      gl!.uniform1f(uPlay, tel.isPlaying ? 1 : 0);
      gl!.uniform1f(uProgress, tel.progress);
      gl!.uniform1f(uReduced, reduced ? 1 : 0);
      const s = seedRef.current;
      gl!.uniform3f(
        uSeed,
        ((s & 0xff) / 255) * 2,
        (((s >> 8) & 0xff) / 255) * 2,
        (((s >> 16) & 0xff) / 255) * 2,
      );
      gl!.drawArrays(gl!.TRIANGLES, 0, 3);
    };

    if (testMode) {
      // §180：确定帧 —— 固定 seed+time+telemetry，渲染一帧后停。
      drawFrame(
        VISUAL_TEST_TIME_SECONDS,
        { isPlaying: true, progress: 0.5, volume: 1, sourceLive: false },
        false,
      );
      return () => {
        disposed = true;
        if (program) gl!.deleteProgram(program);
        if (buf) gl!.deleteBuffer(buf);
      };
    }

    const loop = (now: number) => {
      if (disposed) return;
      const tel = telemetryRef.current;
      const fps = targetFps(tel.isPlaying, reducedMotion);
      const interval = 1000 / fps;
      if (now - lastFrame >= interval) {
        lastFrame = now;
        drawFrame((now - startTime) / 1000, tel, reducedMotion);
      }
      raf = window.requestAnimationFrame(loop);
    };
    raf = window.requestAnimationFrame(loop);

    return () => {
      disposed = true;
      window.cancelAnimationFrame(raf);
      window.removeEventListener('resize', onResize);
      canvas.removeEventListener('webglcontextlost', onLost);
      window.clearTimeout(resizeTimer);
      if (program) gl!.deleteProgram(program);
      if (buf) gl!.deleteBuffer(buf);
    };
  }, [reducedMotion]);

  if (webglFailed) {
    // §69：CSS 渐变 fallback —— 同 palette，无 WebGL。
    return <div className="aurora-fallback" aria-hidden />;
  }

  return <canvas ref={canvasRef} className="aurora-canvas" aria-hidden />;
}
