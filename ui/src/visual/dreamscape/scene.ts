// Dreamscape GL 场景装配（§128/§132–§138）：Three.js 资源构建与释放。
// 只做「世界怎么被看见」；几何/材质全部由 manifest 数值驱动（§110），
// 无外部模型/贴图（§59/§135 程序化）。
import {
  AmbientLight,
  BufferGeometry,
  Color,
  ConeGeometry,
  DirectionalLight,
  DodecahedronGeometry,
  Float32BufferAttribute,
  Fog,
  Group,
  HemisphereLight,
  InstancedMesh,
  Mesh,
  MeshLambertMaterial,
  Object3D,
  PlaneGeometry,
  Points,
  PointsMaterial,
  SphereGeometry,
  TorusGeometry,
} from 'three';
import type { SceneManifest } from '../../ipc/dreamscape';

/** 可释放资源登记表（§132/§158：退出全部 dispose）。 */
export class Disposables {
  private geometries: { dispose(): void }[] = [];
  private materials: { dispose(): void }[] = [];
  private objects: { dispose(): void }[] = [];

  /** Fog 等无 dispose 的资源以占位登记（保持释放审计完整）。 */
  passive(): void {
    this.objects.push({ dispose() {} });
  }

  geo<T extends { dispose(): void }>(g: T): T {
    this.geometries.push(g);
    return g;
  }
  mat<T extends { dispose(): void }>(m: T): T {
    this.materials.push(m);
    return m;
  }
  obj<T extends { dispose(): void }>(o: T): T {
    this.objects.push(o);
    return o;
  }
  /** 释放全部并清册（§158）。 */
  disposeAll(): void {
    for (const g of this.geometries) g.dispose();
    for (const m of this.materials) m.dispose();
    for (const o of this.objects) o.dispose();
    this.geometries = [];
    this.materials = [];
    this.objects = [];
  }
  get count(): number {
    return this.geometries.length + this.materials.length + this.objects.length;
  }
}

/** 家族 → 共享几何（§133：重复对象共享，不 clone）。 */
function familyGeometry(
  family: string,
  d: Disposables,
): ConeGeometry | DodecahedronGeometry | SphereGeometry | TorusGeometry {
  switch (family) {
    case 'flora':
      return d.geo(new ConeGeometry(0.5, 2.2, 6)); // 低多边形树
    case 'stone':
      return d.geo(new DodecahedronGeometry(0.6, 0));
    case 'water':
    case 'light':
      return d.geo(new SphereGeometry(0.4, 10, 8));
    default:
      return d.geo(new TorusGeometry(0.5, 0.12, 8, 16));
  }
}

/** palette HSL → 颜色（约束值来自 synth，§42）。 */
function hsl(hue: number, sat: number, lum: number): Color {
  return new Color().setHSL(((hue % 360) + 360) % 360 / 360, sat, lum);
}

/** 构建场景根（一次 loadWorld 一次构建；§160 旧场景整体替换）。 */
export interface BuiltScene {
  root: Group;
  sun: DirectionalLight;
  ambient: HemisphereLight;
  fill: AmbientLight;
  /** 粒子系统（动画由渲染器驱动）。 */
  particles: Points | null;
  particleBase: Float32Array | null;
  /** fog 颜色（环境插值用）。 */
  fog: Fog;
  terrainMaterial: MeshLambertMaterial;
}

export function buildScene(
  manifest: SceneManifest,
  settings: { terrainSegments: number; particleScale: number },
  d: Disposables,
): BuiltScene {
  const root = d.obj(new Group());
  const palette = manifest.palette;
  const base = hsl(palette.baseHue, palette.saturation, palette.luminance);

  // ---- 大气（§136 atmosphere/§39 fog：depth，不吞场景）----
  const fog = new Fog(base.clone().multiplyScalar(0.5), 60, 260);
  d.passive();

  // ---- 地形（§48 高度场重建；§133 单一共享几何）----
  const seg = settings.terrainSegments;
  const plane = d.geo(new PlaneGeometry(512, 512, seg, seg));
  plane.rotateX(-Math.PI / 2);
  const pos = plane.getAttribute('position');
  if (pos) {
    for (let i = 0; i < pos.count; i += 1) {
      const x = pos.getX(i);
      const z = pos.getZ(i);
      pos.setY(i, terrainHeight(manifest.terrain, x, z));
    }
    plane.computeVertexNormals();
  }
  const terrainMaterial = d.mat(new MeshLambertMaterial({ color: base }));
  const terrain = new Mesh(plane, terrainMaterial);
  root.add(terrain);

  // ---- 光（§40 抽象光态 → Three.js；§134 共享材质）----
  const sun = new DirectionalLight(0xffffff, 1.0);
  sun.position.set(60, 90, 30);
  root.add(sun);
  const ambient = new HemisphereLight(base, 0x0b0e14, 0.6);
  root.add(ambient);
  const fill = new AmbientLight(0xffffff, 0.25);
  root.add(fill);

  // ---- 实例组（§61 InstancedMesh；§133 共享几何 + 每族一个材质）----
  const dummy = new Object3D();
  for (const group of manifest.instances) {
    if (group.family === 'particle' || group.x.length === 0) continue;
    const geo = familyGeometry(group.family, d);
    const mat = d.mat(
      new MeshLambertMaterial({
        color: group.family === 'flora'
          ? hsl(palette.accentHue, palette.saturation, palette.luminance * 0.9)
          : hsl(palette.baseHue + 20, palette.saturation * 0.7, palette.luminance),
      }),
    );
    const mesh = new InstancedMesh(geo, mat, group.x.length);
    for (let i = 0; i < group.x.length; i += 1) {
      const x = group.x[i] ?? 0;
      const z = group.z[i] ?? 0;
      const scale = group.scale[i] ?? 1;
      const rot = group.rotation[i] ?? 0;
      dummy.position.set(x, terrainHeight(manifest.terrain, x, z) + 1.0 * scale, z);
      dummy.scale.setScalar(scale);
      dummy.rotation.set(0, rot, 0);
      dummy.updateMatrix();
      mesh.setMatrixAt(i, dummy.matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
    root.add(mesh);
  }

  // ---- landmark（§55/§58 primitive composition，程序化组合）----
  for (const lm of manifest.landmarks) {
    const lmGroup = new Group();
    const bodyMat = d.mat(new MeshLambertMaterial({ color: hsl(palette.accentHue, palette.saturation, palette.luminance * 0.8) }));
    switch (lm.kind) {
      case 'OldTree': {
        const trunk = new Mesh(d.geo(new ConeGeometry(0.6, 6, 7)), bodyMat);
        trunk.position.y = 3;
        lmGroup.add(trunk);
        break;
      }
      case 'StandingStone': {
        const stone = new Mesh(d.geo(new DodecahedronGeometry(1.8, 0)), bodyMat);
        stone.position.y = 1.6;
        lmGroup.add(stone);
        break;
      }
      case 'StillPool': {
        const pool = new Mesh(d.geo(new SphereGeometry(lm.radius * 0.8, 14, 10)), bodyMat);
        pool.scale.y = 0.15;
        pool.position.y = 0.1;
        lmGroup.add(pool);
        break;
      }
      case 'Arch': {
        const arch = new Mesh(d.geo(new TorusGeometry(lm.radius * 0.8, 0.35, 8, 20, Math.PI)), bodyMat);
        arch.position.y = 0.2;
        lmGroup.add(arch);
        break;
      }
      default: {
        // GlowCluster：光球簇
        for (let i = 0; i < 5; i += 1) {
          const orb = new Mesh(d.geo(new SphereGeometry(0.35, 10, 8)), bodyMat);
          orb.position.set(Math.sin(i * 2.4) * 1.2, 0.6 + (i % 2) * 0.7, Math.cos(i * 2.4) * 1.2);
          lmGroup.add(orb);
        }
      }
    }
    lmGroup.position.set(lm.x, lm.y, lm.z);
    lmGroup.rotation.y = lm.rotation;
    root.add(lmGroup);
  }

  // ---- 粒子（§63 预算内；§142 质量比例）----
  let particles: Points | null = null;
  let particleBase: Float32Array | null = null;
  const pGroup = manifest.instances.find((g) => g.family === 'particle');
  if (pGroup && pGroup.x.length > 0) {
    const n = Math.max(1, Math.floor(pGroup.x.length * settings.particleScale));
    const positions = new Float32Array(n * 3);
    for (let i = 0; i < n; i += 1) {
      positions[i * 3] = pGroup.x[i] ?? 0;
      positions[i * 3 + 1] = 2 + (i % 7) * 0.8;
      positions[i * 3 + 2] = pGroup.z[i] ?? 0;
    }
    particleBase = positions.slice();
    const pGeo = d.geo(positionsGeometry(positions));
    const pMat = d.mat(
      new PointsMaterial({
        color: hsl(palette.accentHue, palette.saturation, Math.min(0.85, palette.luminance + 0.2)),
        size: 0.35,
        sizeAttenuation: true,
        transparent: true,
        opacity: 0.7,
      }),
    );
    particles = new Points(pGeo, pMat);
    root.add(particles);
  }

  return { root, sun, ambient, fill, particles, particleBase, fog, terrainMaterial };
}

/** 位置属性 BufferGeometry。 */
function positionsGeometry(positions: Float32Array): BufferGeometry {
  const g = new BufferGeometry();
  g.setAttribute('position', new Float32BufferAttribute(positions, 3));
  return g;
}

/** 地形高度重建（与 Rust 侧同参数的两倍频程 value-noise 近似；
 *  §171：manifest 携带参数，视觉重建允许 GPU 差异，逻辑以 Rust 为准）。 */
export function terrainHeight(
  terrain: { amplitude: number; frequency: number; terrainSeed: number },
  x: number,
  z: number,
): number {
  let h = 0;
  let amp = 1;
  let freq = terrain.frequency;
  for (let octave = 0; octave < 2; octave += 1) {
    h += valueNoise(terrain.terrainSeed + octave * 0x9e37, x * freq, z * freq) * amp;
    amp *= 0.5;
    freq *= 2;
  }
  return (h - 0.75) * terrain.amplitude * 2;
}

function hash2(seed: number, ix: number, iz: number): number {
  let h = (seed ^ Math.imul(ix, 0x27d4eb2d) ^ Math.imul(iz, 0x165667b1)) >>> 0;
  h = Math.imul(h ^ (h >>> 15), 0x2c1b3c6d) >>> 0;
  h = Math.imul(h ^ (h >>> 12), 0x297a2d39) >>> 0;
  return ((h ^ (h >>> 15)) >>> 0) / 4294967296;
}

function valueNoise(seed: number, x: number, z: number): number {
  const ix = Math.floor(x);
  const iz = Math.floor(z);
  const tx = smooth(x - ix);
  const tz = smooth(z - iz);
  const a = hash2(seed, ix, iz);
  const b = hash2(seed, ix + 1, iz);
  const c = hash2(seed, ix, iz + 1);
  const d = hash2(seed, ix + 1, iz + 1);
  const top = a + (b - a) * tx;
  const bottom = c + (d - c) * tx;
  return top + (bottom - top) * tz;
}

function smooth(t: number): number {
  const c = Math.min(Math.max(t, 0), 1);
  return c * c * (3 - 2 * c);
}
