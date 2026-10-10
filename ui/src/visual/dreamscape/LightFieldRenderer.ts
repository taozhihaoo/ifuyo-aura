// 光场渲染器（Dreamscape v2）：单张全屏片元着色器画极光幕 + 星野 + 镜面
// + 光尘，由曲目特征与实时电平驱动。裸 WebGL2，不依赖 three；无逐帧分配
// （uniform 位置一次解析）。
//
// 确定性：同 field + 同 telemetry + 同 time → 同画面（测试模式冻结时间）。
import type { LightField } from '../../ipc/dreamscape';
import {
  QUALITY_SETTINGS,
  shouldRender,
  targetFps,
  type QualitySettings,
} from './engine';

/** 每帧输入：曲目特征（慢变量）+ 实时电平（快变量）。 */
export interface FieldTelemetry {
  playing: boolean;
  /** 0..1 节拍包络（前端由 bpm/offset/position 推导并平滑）。 */
  pulse: number;
  /** 0..1 曲目能量（分析值；未分析 = 0.5 中性，不伪造具体特征）。 */
  energy: number;
  /** 0..1 频谱亮度重心（分析值）。 */
  brightness: number;
  /** 0..1 实时 RMS（映射后）。 */
  level: number;
  low: number;
  mid: number;
  high: number;
}


const NEUTRAL = 0.5;

const VERT = `#version 300 es
in vec2 a_pos;
void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }`;

const FRAG = `#version 300 es
precision highp float;
out vec4 frag;

uniform vec2 uRes;
uniform float uTime;
uniform float uSeed;
uniform vec4 uPal;   // baseHue, accentHue, saturation, luminance
uniform vec4 uLight; // sun, ambient, horizonGlow, fog
uniform vec4 uEnv;   // wind, mistBias, detail, glowEnabled
uniform vec4 uPar;   // curtainDepth, moteDensity, veil, glowSoftness
uniform vec4 uPar2;  // mirror, starScale, horizon, motif
uniform vec4 uMus;   // pulse, energy, brightness, level
uniform vec4 uBand;  // low, mid, high, playing
uniform float uReduced;

float h21(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123); }
float vnoise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  f = f * f * (3.0 - 2.0 * f);
  return mix(mix(h21(i), h21(i + vec2(1.0, 0.0)), f.x),
             mix(h21(i + vec2(0.0, 1.0)), h21(i + vec2(1.0, 1.0)), f.x), f.y);
}
float fbm(vec2 p, int oct) {
  float s = 0.0, a = 0.5;
  for (int i = 0; i < 6; i++) {
    if (i >= oct) break;
    s += vnoise(p) * a;
    p = p * 2.07 + vec2(13.1, 7.3);
    a *= 0.5;
  }
  return s;
}
vec3 hsl3(float h, float s, float l) {
  vec3 k = mod(vec3(0.0, 8.0, 4.0) + h * 12.0, 12.0);
  float a = s * min(l, 1.0 - l);
  return l - a * clamp(min(k - 3.0, 9.0 - k), -1.0, 1.0);
}
vec3 palColor(float t) {
  // 走 base→accent 的下降弧（紫/蓝一侧）；最短弧会穿过橙，低明度下糊成锈红。
  float h = mix(uPal.x, uPal.y - 360.0, clamp(t, 0.0, 1.0)) / 360.0;
  return hsl3(fract(h), clamp(uPal.z * (0.85 + 0.5 * t), 0.0, 0.95),
              clamp(uPal.w * (0.7 + 0.8 * t), 0.02, 0.9));
}

// 极光幕：q 为画面坐标（y 向上，horizon 以上）。返回叠加光。
vec3 curtains(vec2 q, float t) {
  vec3 acc = vec3(0.0);
  float depth = 0.5 + uPar.x * 1.2;
  float drift = 0.030 + uEnv.x * 0.055 + uMus.z * 0.030;
  float boost = 1.0 + uMus.x * 0.50 + uMus.w * 0.30 + uBand.z * 0.18;
  for (int i = 0; i < 5; i++) {
    if (float(i) > uEnv.z - 1.0) break;
    float fi = float(i);
    float seed = uSeed * 7.0 + fi * 3.71;
    // 幕帘中心线（缓慢起伏）+ 每层不同高度
    float base = 0.16 + 0.13 * fi + 0.10 * uPar2.w;
    float wob = fbm(vec2(q.x * (1.1 + fi * 0.35) + seed, t * drift + seed), 3) - 0.5;
    float center = base + wob * (0.30 + depth * 0.28);
    float thick = (0.045 + 0.05 * uPar.x) * (1.0 + fi * 0.35) * (0.75 + 0.5 * uMus.y);
    // 上锐下柔：顶部收口、向下拖尾，像垂落的光幕而非等高条带
    float rel = (q.y - center) / thick;
    float band = exp(-rel * rel * (rel > 0.0 ? 5.5 : 1.5));
    // 双层流苏（粗疏密 + 细光丝），并沿高度拉伸形成竖向纹理
    float fil = fbm(vec2(q.x * (3.2 + fi * 0.8) + seed * 2.0, t * drift * 0.55 - fi), 3);
    float hair = vnoise(vec2(q.x * (26.0 + fi * 7.0) + seed, t * 0.10 + fi * 2.3));
    float body = band * smoothstep(0.30, 0.86, fil) * (0.60 + 0.40 * hair)
               * (0.55 + 0.45 * sin(t * 0.6 + fi + uSeed));
    acc += palColor(0.15 + fi * 0.2 + uMus.z * 0.25) * body * boost * (0.55 - fi * 0.06);
  }
  return acc;
}

// 星野：hash 网格点，夜里浮出，随低频轻微呼吸。
float stars(vec2 q, float t) {
  float dens = 80.0 + uPar2.y * 90.0;
  vec2 sp = vec2(q.x * dens, (q.y - 0.5) * dens * 0.6) + uSeed * 33.0;
  vec2 cell = floor(sp);
  vec2 fr = fract(sp) - 0.5;
  float r = h21(cell);
  float on = step(0.972 - 0.012 * uPar2.y, r);
  float tw = 0.55 + 0.45 * sin(t * (1.2 + r * 3.0) + r * 40.0);
  float d = length(fr) * (1.0 + 3.0 * (1.0 - r));
  return on * exp(-d * d * 22.0) * tw * (0.6 + 0.4 * uBand.x);
}

// 光尘：几层缓慢上浮的柔光点。
vec3 motes(vec2 q, float t) {
  vec3 acc = vec3(0.0);
  float n = 3.0 + uPar.y * 4.0;
  for (int i = 0; i < 7; i++) {
    if (float(i) > n) break;
    float fi = float(i);
    float s = uSeed * 3.0 + fi * 11.7;
    vec2 m = vec2(h21(vec2(s, 1.7)), h21(vec2(s, 3.1)));
    float rise = fract(m.y + t * (0.018 + 0.03 * m.x) + fi * 0.11);
    vec2 pos = vec2(m.x * 2.4 - 1.2 + 0.12 * sin(t * 0.7 + s), rise * 1.1 - 0.15);
    float d = length((q - pos) * vec2(1.0, 1.0));
    float glow = exp(-d * d * 260.0) * (0.35 + 0.65 * uBand.y) * (0.4 + uMus.x);
    acc += palColor(0.55 + 0.4 * m.x) * glow * uPar.y;
  }
  return acc;
}

vec3 aces(vec3 x) {
  return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}

void main() {
  vec2 uv = gl_FragCoord.xy / uRes;
  vec2 p = (gl_FragCoord.xy - 0.5 * uRes) / uRes.y;
  float horizon = mix(-0.16, 0.16, uPar2.z);
  float t = uTime * (1.0 - uReduced * 0.97);

  float day = clamp(uLight.x, 0.0, 1.0);
  float night = 1.0 - day;

  // 天空基底：深紫底 → 地平线色；白天整体提亮并去饱和
  vec3 ground = hsl3(fract(uPal.x / 360.0 + 0.02), 0.45, 0.035);
  vec3 skyLow = palColor(0.18 + 0.25 * day);
  vec3 skyHigh = palColor(0.85);
  float up = clamp((p.y - horizon) / max(0.001, 1.0 - horizon), 0.0, 1.0);
  float seam = smoothstep(-0.055, 0.055, p.y - horizon);
  vec3 col = mix(ground * 0.72, skyLow, 0.55);
  col = mix(col, skyHigh * 0.35, pow(up, 0.75) * 0.55);
  col += palColor(0.35) * uLight.z * exp(-abs(p.y - horizon) * 7.0) * 0.9;
  col *= 0.55 + uLight.y * 0.9 + day * 0.5;

  // 光场叠加（幕帘 + 星 + 光尘）
  vec2 q = vec2(p.x, p.y - horizon);
  vec3 field = curtains(q, t) + motes(p, t);
  field += vec3(0.88, 0.92, 1.0) * stars(q, t) * night * smoothstep(0.02, 0.30, q.y);
  // 雾幕：压低远处对比、抬升地平线灰度
  float veil = uPar.z * 0.55 + uLight.w * 0.7 + uEnv.y * 0.4;
  field *= 1.0 - clamp(veil * 0.75, 0.0, 0.85);
  // 岸线辉光：用一条窄亮带取代"上下各画一半"的硬缝
  col += palColor(0.42) * exp(-abs(p.y - horizon) * 16.0) * (0.10 + uPar2.x * 0.22);
  col += field * mix(0.30, 1.0, seam);

  // 地平镜面：把上方的光沿 y 镜像并加涟漪
  if (uPar2.x > 0.001 && p.y < horizon) {
    float d = (horizon - p.y);
    float ripple = 0.006 * (1.0 + uBand.z * 2.0) * sin(p.x * 46.0 - t * 2.2) * (0.3 + d * 2.0);
    vec2 mr = vec2(p.x + ripple * 0.4, horizon + d * (1.0 + ripple));
    // 水面不是镜子：越远越暗越糊，扰动随距离增大
    vec3 refl = curtains(vec2(mr.x, mr.y - horizon), t) * 0.42;
    refl += vec3(0.8, 0.86, 1.0) * stars(vec2(mr.x, mr.y - horizon), t) * night * 0.22;
    float fade = exp(-d * 3.4);
    col = mix(col * 0.55, col * 0.42 + refl, clamp(uPar2.x * fade, 0.0, 0.8));
  }

  // 低频推动整体明度（鼓点感）
  col *= 1.0 + uBand.x * 0.08 + uMus.w * 0.10;

  // 暗角 + 抖动去色带 + 色调映射
  float vig = 1.0 - 0.42 * dot(p * vec2(0.85, 1.0), p * vec2(0.85, 1.0));
  col *= clamp(vig, 0.35, 1.0);
  col = aces(col * (1.05 + uMus.x * 0.08));
  col += (h21(gl_FragCoord.xy + fract(t) * 91.0) - 0.5) / 255.0;
  frag = vec4(col, 1.0);
}`;

export interface FieldEnvironment {
  sunIntensity: number;
  ambientIntensity: number;
  horizonGlow: number;
  fog: number;
  wind: number;
  mistBias: number;
}

const UNIFORM_NAMES = [
  'uRes',
  'uTime',
  'uSeed',
  'uPal',
  'uLight',
  'uEnv',
  'uPar',
  'uPar2',
  'uMus',
  'uBand',
  'uReduced',
] as const;

export class LightFieldRenderer {
  private readonly canvas: HTMLCanvasElement;
  private readonly gl: WebGL2RenderingContext;
  private readonly uniforms = new Map<string, WebGLUniformLocation | null>();
  /** 上下文丢失后全部 GL 对象失效，恢复时要重建 ⇒ 可空，不是 readonly。 */
  private program: WebGLProgram | null = null;
  private vao: WebGLVertexArrayObject | null = null;
  private raf = 0;
  private running = false;
  private lost = false;
  /** 丢失瞬间"本来在跑"：恢复后据此自动重启循环（§151）。 */
  private resumeOnRestore = false;
  private loop: (() => void) | null = null;
  /** 上下文丢失/恢复上报（true=丢失→应显示 fallback；false=已恢复）。 */
  onContextChange: ((lost: boolean) => void) | null = null;
  private lastFrameMs = 0;
  private startedAt = performance.now();
  private fpsWindowStart = 0;
  private fpsWindowCount = 0;
  /** 平滑后的遥测（非对称跟随：攻击快、释放慢）。逐块 RMS 帧间抖动很大，
   *  直接喂给亮度等于用噪声驱动曝光——实测就是"晃眼"。 */
  private readonly sm = { pulse: 0, energy: NEUTRAL, brightness: NEUTRAL, level: 0, low: 0, mid: 0, high: 0 };
  private smAt = 0;
  private disposed = false;
  private settings: QualitySettings = QUALITY_SETTINGS.medium;
  private env: FieldEnvironment = {
    sunIntensity: 0,
    ambientIntensity: 0.25,
    horizonGlow: 0,
    fog: 0.3,
    wind: 0.1,
    mistBias: 0.05,
  };

  private constructor(canvas: HTMLCanvasElement, gl: WebGL2RenderingContext) {
    this.canvas = canvas;
    this.gl = gl;
  }

  private readonly onLostEvent = (ev: Event): void => {
    // preventDefault 是浏览器肯再发 restored 事件的前提条件。
    ev.preventDefault();
    if (this.disposed) return;
    this.resumeOnRestore = this.running;
    this.lost = true;
    this.stop();
    this.canvas.dataset.fieldError = 'context-lost';
    this.onContextChange?.(true);
  };

  private readonly onRestoredEvent = (): void => {
    if (this.disposed) return;
    this.lost = false;
    if (this.build()) {
      delete this.canvas.dataset.fieldError;
      this.onContextChange?.(false);
      if (this.resumeOnRestore && this.loop) {
        this.running = true;
        this.raf = requestAnimationFrame(this.tick);
      }
    } else {
      this.onContextChange?.(true); // 重建失败：留在 fallback，退出视图再进即恢复
    }
  };

  /** 编译/链接/建 VAO（首建与上下文恢复共用）。失败原因写进 dataset。 */
  private build(): boolean {
    const gl = this.gl;
    const fail = (why: string, log: string | null) => {
      this.canvas.dataset.fieldError = `${why}: ${(log ?? '').trim().slice(0, 220)}`;
    };
    const compile = (type: number, src: string): WebGLShader | null => {
      const sh = gl.createShader(type);
      if (!sh) return null;
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
        fail('compile', gl.getShaderInfoLog(sh));
        gl.deleteShader(sh);
        return null;
      }
      return sh;
    };
    const vs = compile(gl.VERTEX_SHADER, VERT);
    const fs = compile(gl.FRAGMENT_SHADER, FRAG);
    if (!vs || !fs) return false;
    const program = gl.createProgram();
    if (!program) return false;
    gl.attachShader(program, vs);
    gl.attachShader(program, fs);
    gl.bindAttribLocation(program, 0, 'a_pos');
    gl.linkProgram(program);
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      fail('link', gl.getProgramInfoLog(program));
      gl.deleteProgram(program);
      return false;
    }
    // 全屏三角形（一次 draw call，无逐帧缓冲操作）。
    const vao = gl.createVertexArray();
    if (!vao) return false;
    gl.bindVertexArray(vao);
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);
    this.program = program;
    this.vao = vao;
    this.uniforms.clear();
    gl.useProgram(program);
    for (const name of UNIFORM_NAMES) {
      this.uniforms.set(name, gl.getUniformLocation(program, name));
    }
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    return true;
  }

  /** 创建；WebGL2 不可用/编译失败 → null（调用方走 CSS fallback，不悬挂 loading）。
   *  失败原因写进 `canvas.dataset.fieldError`（与 data-fps 同类的本地诊断通道，
   *  不外传）——无声 fallback 会让"为什么没有画面"无从查起。 */
  static create(canvas: HTMLCanvasElement): LightFieldRenderer | null {
    const gl = canvas.getContext('webgl2', { antialias: false, alpha: false, powerPreference: 'low-power' });
    if (!gl) {
      canvas.dataset.fieldError = 'no-webgl2';
      return null;
    }
    const renderer = new LightFieldRenderer(canvas, gl);
    if (!renderer.build()) return null;
    delete canvas.dataset.fieldError;
    canvas.addEventListener('webglcontextlost', renderer.onLostEvent, false);
    canvas.addEventListener('webglcontextrestored', renderer.onRestoredEvent, false);
    return renderer;
  }

  setEnvironment(env: FieldEnvironment): void {
    this.env = env;
  }

  setQuality(settings: QualitySettings): void {
    this.settings = settings;
  }

  /** 尺寸按 pixelRatioCap 收敛（§150：4K×200% 不失控）。 */
  resize(canvas: HTMLCanvasElement): void {
    const cap = this.settings.pixelRatioCap;
    const dpr = Math.min(window.devicePixelRatio || 1, cap);
    const w = Math.max(1, Math.round(canvas.clientWidth * dpr));
    const h = Math.max(1, Math.round(canvas.clientHeight * dpr));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
      this.gl.viewport(0, 0, w, h);
    }
  }

  render(canvas: HTMLCanvasElement, field: LightField, tel: FieldTelemetry, reducedMotion: boolean): void {
    const program = this.program;
    const vao = this.vao;
    if (this.disposed || this.lost || program === null || vao === null) return;
    const gl = this.gl;
    const u = (n: string): WebGLUniformLocation | null => this.uniforms.get(n) ?? null;
    const now = performance.now();
    const t = (now - this.startedAt) / 1000;
    const dt = this.smAt === 0 ? 0.05 : Math.min(0.25, (now - this.smAt) / 1000);
    this.smAt = now;
    const follow = (key: 'pulse' | 'level' | 'low' | 'mid' | 'high', target: number, attack: number, release: number) => {
      const tau = target > this.sm[key] ? attack : release;
      const k = 1 - Math.exp(-dt / tau);
      this.sm[key] += (target - this.sm[key]) * k;
    };
    follow('pulse', tel.pulse, 0.030, 0.120);
    follow('level', tel.level, 0.140, 0.380);
    follow('low', tel.low, 0.120, 0.320);
    follow('mid', tel.mid, 0.150, 0.400);
    follow('high', tel.high, 0.100, 0.280);
    // 曲目级特征：极慢跟随（它们本来就不该逐帧变）。
    const slow = 1 - Math.exp(-dt / 1.2);
    this.sm.energy += (tel.energy - this.sm.energy) * slow;
    this.sm.brightness += (tel.brightness - this.sm.brightness) * slow;
    gl.useProgram(program);
    gl.bindVertexArray(vao);
    gl.uniform2f(u('uRes'), canvas.width, canvas.height);
    gl.uniform1f(u('uTime'), t);
    gl.uniform1f(u('uSeed'), ((field.fieldSeed % 100000) as number) / 100000);
    gl.uniform4f(u('uPal'), field.palette.baseHue, field.palette.accentHue, field.palette.saturation, field.palette.luminance);
    gl.uniform4f(u('uLight'), this.env.sunIntensity, this.env.ambientIntensity, this.env.horizonGlow, this.env.fog);
    gl.uniform4f(u('uEnv'), this.env.wind, this.env.mistBias, this.settings.fieldDetail, this.settings.glowEnabled ? 1 : 0);
    gl.uniform4f(u('uPar'), field.params.curtainDepth, field.params.moteDensity * this.settings.moteDensity, field.params.veil, field.params.glowSoftness);
    const motif = { veil: 0, cascade: 1, radiance: 2, drift: 3 }[field.motif] ?? 0;
    gl.uniform4f(u('uPar2'), this.settings.glowEnabled ? field.params.mirror : 0, field.params.starScale, field.params.horizon, motif);
    gl.uniform4f(u('uMus'), this.sm.pulse, this.sm.energy, this.sm.brightness, this.sm.level);
    gl.uniform4f(u('uBand'), this.sm.low, this.sm.mid, this.sm.high, tel.playing ? 1 : 0);
    gl.uniform1f(u('uReduced'), reducedMotion ? 1 : 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    this.fpsWindowCount += 1;
  }

  /** 实测绘制帧率（1 秒滑窗）——不是 GPU 提交耗时，否则读数永远是 1000。 */
  fps(now: number): number {
    if (this.fpsWindowStart === 0) this.fpsWindowStart = now;
    const span = now - this.fpsWindowStart;
    if (span >= 1000) {
      const fps = (this.fpsWindowCount * 1000) / span;
      this.fpsWindowStart = now;
      this.fpsWindowCount = 0;
      return Math.round(fps);
    }
    return Math.round(this.fpsWindowCount * (1000 / Math.max(1, span)));
  }

  /** 平滑后的遥测（诊断面：证明驱动信号不逐帧跳）。 */
  smoothed(): Readonly<typeof this.sm> {
    return this.sm;
  }

  private readonly tick = (): void => {
    if (!this.running) return;
    this.loop?.();
    this.raf = requestAnimationFrame(this.tick);
  };

  start(loop: () => void): void {
    this.loop = loop;
    if (this.running || this.disposed) return;
    if (this.lost) {
      // 上下文丢失期间只登记循环，恢复时由 onRestoredEvent 自动起跑。
      this.resumeOnRestore = true;
      return;
    }
    this.running = true;
    this.raf = requestAnimationFrame(this.tick);
  }

  /** 节流判定（§144：playing 30 / idle 10 / 隐藏 2 / reduced 5fps）。
   *  通过即记账——时间戳必须是"上一次真正绘制"，若记成"上一次 rAF 回调"
   *  则 now-last 恒为一个帧间隔（~16ms），节流门槛永远过不去。 */
  shouldDraw(now: number, playing: boolean, reduced: boolean, visible: boolean): boolean {
    if (!shouldRender(now, this.lastFrameMs, targetFps(playing, reduced, visible))) return false;
    this.lastFrameMs = now;
    return true;
  }

  stop(): void {
    this.running = false;
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
  }

  dispose(): void {
    if (this.disposed) return;
    this.stop();
    this.disposed = true;
    this.canvas.removeEventListener('webglcontextlost', this.onLostEvent, false);
    this.canvas.removeEventListener('webglcontextrestored', this.onRestoredEvent, false);
    if (this.program !== null) this.gl.deleteProgram(this.program);
    if (this.vao !== null) this.gl.deleteVertexArray(this.vao);
    this.program = null;
    this.vao = null;
    this.gl.getExtension('WEBGL_lose_context')?.loseContext();
  }
}
