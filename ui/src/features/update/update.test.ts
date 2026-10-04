// M11 §96 Core 测试矩阵：版本比较/清单验证/状态转移/错误分类/进度节流/
// 检查策略（§47/§46/§7/§65/§68/§8/§264 精神——全部纯逻辑，node 可跑）。
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  canTransition,
  coerceProgress,
  isNewerVersion,
  parseSemver,
  validateManifest,
  TRANSITIONS,
  type UpdateState,
} from './updateTypes';
import { UpdateService, MIN_CHECK_INTERVAL_MS, useUpdateStore } from './UpdateService';

// ---------- §47 版本比较 ----------
describe('版本比较（§47 降级保护）', () => {
  it('semver 解析与比较', () => {
    expect(parseSemver('0.2.0')).toEqual([0, 2, 0]);
    expect(parseSemver('0.2')).toBeNull();
    expect(parseSemver('v0.2.0')).toBeNull();
    expect(parseSemver('01.2.3')).toEqual([1, 2, 3]);
    expect(isNewerVersion('0.2.0', '0.1.0')).toBe(true);
    expect(isNewerVersion('1.0.0', '0.9.9')).toBe(true);
    expect(isNewerVersion('0.1.0', '0.2.0')).toBe(false) // 降级拒绝;
    expect(isNewerVersion('0.1.0', '0.1.0')).toBe(false) // 同版本非更新;
    expect(isNewerVersion('garbage', '0.1.0')).toBe(false) // 非法版本拒绝;
  });
});

// ---------- §46 清单验证 ----------
describe('清单 schema 验证（§46 不信任远程字段）', () => {
  it('合法清单通过', () => {
    expect(
      validateManifest({
        version: '1.2.3',
        notes: 'n',
        pub_date: '2026-10-05T00:00:00Z',
        platforms: { 'windows-x86_64': { signature: 'sig', url: 'https://x/y' } },
      }),
    ).toBeNull();
  });
  it('非法版本/缺签名/http url/非对象拒绝', () => {
    expect(validateManifest({ version: 'x.y.z' })?.kind).toBe('Manifest');
    expect(validateManifest({ version: '1.0.0', platforms: {} })?.kind).toBe('Manifest');
    expect(
      validateManifest({ version: '1.0.0', platforms: { w: { signature: '', url: 'https://x' } } })?.kind,
    ).toBe('Manifest');
    expect(
      validateManifest({ version: '1.0.0', platforms: { w: { signature: 's', url: 'http://x' } } })?.detail,
    ).toContain('https');
    expect(validateManifest('str')?.kind).toBe('Manifest');
    expect(validateManifest(null)?.kind).toBe('Manifest');
  });
});

// ---------- §7 状态机 ----------
describe('状态机转移（§7 typed union）', () => {
  it('全合法路径可走通（check→available→download→install→restart）', () => {
    const path: UpdateState[] = [
      { kind: 'idle' },
      { kind: 'checking' },
      { kind: 'available', version: '1.0.0', notes: '' },
      { kind: 'downloading', progress: 10 },
      { kind: 'downloaded' },
      { kind: 'installing' },
      { kind: 'restartRequired' },
    ];
    for (let i = 1; i < path.length; i++) {
      expect(canTransition(path[i - 1] as UpdateState, path[i] as UpdateState)).toBe(true);
    }
  });
  it('非法转移拒绝（idle→downloaded / installed 后取消等）', () => {
    expect(canTransition({ kind: 'idle' }, { kind: 'downloaded' })).toBe(false);
    expect(canTransition({ kind: 'installing' }, { kind: 'cancelled' })).toBe(false) // §69 提交后不可取消;
    expect(canTransition({ kind: 'upToDate' }, { kind: 'available', version: 'x', notes: '' })).toBe(false);
  });
  it('转移表无死路（每状态至少一个出边）', () => {
    for (const [k, outs] of Object.entries(TRANSITIONS)) {
      if (k === 'restartRequired') continue; // 终点前状态
      expect(outs.length, `${k} 出边`).toBeGreaterThan(0);
    }
  });
});

// ---------- §68 进度 ----------
describe('进度有界单调（§68/§99）', () => {
  it('未知大小 → null；回跳抑制；>100 封顶', () => {
    expect(coerceProgress(null, 5, null)).toBeNull();
    expect(coerceProgress(null, 50, 200)).toBe(25);
    expect(coerceProgress(50, 10, 200)).toBe(50) // 不回跳;
    expect(coerceProgress(50, 250, 200)).toBe(100) // 封顶;
    expect(coerceProgress(null, Number.NaN, 100)).toBeNull();
  });
});

// ---------- §8/§65 UpdateService（mock 插件 §265） ----------
function makeService(
  pluginReturn: ReturnType<typeof vi.fn>,
  opts: { lastCheck?: number; autoCheck?: boolean } = {},
): UpdateService {
  const store: Record<string, string> = {};
  if (opts.lastCheck !== undefined) store['aura.update.lastCheckAt'] = String(opts.lastCheck);
  useUpdateStore.getState().reset();
  useUpdateStore.setState({ state: { kind: 'idle' } });
  return new UpdateService({
    plugin: { check: pluginReturn },
    currentVersion: () => '0.1.0',
    now: () => 1_000_000,
    localStorage: {
      getItem: (k) => store[k] ?? null,
      setItem: (k, v) => {
        store[k] = v;
      },
    },
    autoCheckEnabled: () => opts.autoCheck ?? true,
  });
}

describe('UpdateService（§265 mock 插件）', () => {
  beforeEach(() => {
    useUpdateStore.getState().reset();
  });

  it('无更新 → upToDate', async () => {
    const svc = makeService(vi.fn(async () => null));
    const st = await svc.checkForUpdate();
    expect(st.kind).toBe('upToDate');
  });

  it('可用更新 → available（版本/notes 落入状态）', async () => {
    const svc = makeService(
      vi.fn(async () => ({
        version: '0.2.0',
        body: 'notes',
        currentVersion: '0.1.0',
      })) as unknown as ReturnType<typeof vi.fn>,
    );
    const st = await svc.checkForUpdate();
    expect(st.kind).toBe('available');
    if (st.kind === 'available') expect(st.version).toBe('0.2.0');
  });

  it('降级清单 → failed(Version)（§47）', async () => {
    const svc = makeService(
      vi.fn(async () => ({ version: '0.0.9', currentVersion: '0.1.0' })) as unknown as ReturnType<typeof vi.fn>,
    );
    const st = await svc.checkForUpdate();
    expect(st.kind).toBe('failed');
    if (st.kind === 'failed') expect(st.error.kind).toBe('Version');
  });

  it('网络异常 → failed(Network)（§36 不影响播放——状态只进更新状态机）', async () => {
    const svc = makeService(vi.fn(async () => {
      throw new Error('connection refused');
    }));
    const st = await svc.checkForUpdate();
    expect(st.kind).toBe('failed');
    if (st.kind === 'failed') expect(st.error.kind).toBe('Network');
  });

  it('检查策略：24h 间隔 + 关闭自动检查（§8/§84）', () => {
    const fresh = makeService(vi.fn(), {});
    expect(fresh.shouldAutoCheck()).toBe(true);
    const recent = makeService(vi.fn(), { lastCheck: 1_000_000 - 1000 });
    expect(recent.shouldAutoCheck()).toBe(false) // 间隔内不重复;
    const old = makeService(vi.fn(), { lastCheck: 1_000_000 - MIN_CHECK_INTERVAL_MS - 1 });
    expect(old.shouldAutoCheck()).toBe(true);
    const disabled = makeService(vi.fn(), { autoCheck: false });
    expect(disabled.shouldAutoCheck()).toBe(false);
  });

  it('并发检查去重（§8 不疯狂请求）', async () => {
    const fn = vi.fn(
      () => new Promise((resolve) => setTimeout(resolve, 20, null)),
    ) as unknown as ReturnType<typeof vi.fn>;
    const svc = makeService(fn);
    await Promise.all([svc.checkForUpdate(), svc.checkForUpdate(), svc.checkForUpdate()]);
    expect(fn).toHaveBeenCalledTimes(1);
  });
});
