// M11 §5/§6：UpdateService——UI 与 Tauri Updater Adapter 之间唯一入口
// （§6 禁止 React 直执行 installer/PowerShell）。策略面：§8 检查策略
// （延迟/最小间隔/失败有界重试）、§47 降级拒绝、§69 取消语义、§99
// 进度节流。状态机转移经 updateTypes 校验。
// 注意：Windows 文件系统大小写不敏感——本文件即唯一服务模块（无
// updateService.ts 别名文件）。
import { create } from 'zustand';
import { check, type Update } from '@tauri-apps/plugin-updater';
import {
  canTransition,
  coerceProgress,
  isNewerVersion,
  validateManifest,
  type UpdateError,
  type UpdateErrorKind,
  type UpdateState,
} from './updateTypes';

/** tauri-plugin-updater 的 check() 元数据（§14 子集）。 */
type UpdaterPluginLike = {
  check: (opts?: { timeout?: number }) => Promise<
    (Update & { body?: string; currentVersion?: string }) | null
  >;
};

/** §8 最小检查间隔（ms）——D-126：启动延迟 15s + 间隔 24h。 */
export const MIN_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;
export const STARTUP_CHECK_DELAY_MS = 15 * 1000;

/** §99：下载进度 UI 刷新节流（ms）——下载速度不决定渲染频率。 */
export const PROGRESS_THROTTLE_MS = 300;

export interface UpdateServiceDeps {
  /** 可注入插件（测试 mock §265）；缺省 = 官方插件。 */
  plugin?: UpdaterPluginLike;
  currentVersion: () => string;
  now: () => number;
  localStorage?: Pick<Storage, 'getItem' | 'setItem'> | null;
  autoCheckEnabled: () => boolean;
}

function defaultDeps(): UpdateServiceDeps {
  const g = globalThis as { localStorage?: Storage };
  return {
    plugin: { check: (o) => check(o) },
    currentVersion: () => {
      // 版本来源 = 单一权威（version:check 保证一致；App 启动注入）。
      return (globalThis as { __AURA_VERSION__?: string }).__AURA_VERSION__ ?? '';
    },
    now: () => Date.now(),
    localStorage: g.localStorage ?? null,
    autoCheckEnabled: () => true,
  };
}

export interface UpdateStore {
  state: UpdateState;
  lastCheckAt: number | null;
  /** 更新内部状态（仅接受合法转移 §7）。 */
  apply: (to: UpdateState) => void;
  setLastCheck: (at: number) => void;
  reset: () => void;
}

export const useUpdateStore = create<UpdateStore>((set) => ({
  state: { kind: 'idle' },
  lastCheckAt: null,
  apply: (to) => set((s) => (canTransition(s.state, to) ? { state: to } : s)),
  setLastCheck: (lastCheckAt) => set({ lastCheckAt }),
  reset: () => set({ state: { kind: 'idle' } }),
}));

const LAST_CHECK_KEY = 'aura.update.lastCheckAt';

export class UpdateService {
  private readonly deps: UpdateServiceDeps;
  private active: Update | null = null;
  private checking = false;
  private lastProgressEmit = 0;

  constructor(deps: Partial<UpdateServiceDeps> = {}) {
    this.deps = { ...defaultDeps(), ...deps };
  }

  get state(): UpdateState {
    return useUpdateStore.getState().state;
  }

  private transition(to: UpdateState): void {
    const cur = useUpdateStore.getState().state;
    if (!canTransition(cur, to)) {
      throw new Error(`illegal update transition ${cur.kind} -> ${to.kind}`);
    }
    useUpdateStore.getState().apply(to);
  }

  private lastCheck(): number | null {
    const raw = this.deps.localStorage?.getItem(LAST_CHECK_KEY) ?? null;
    const n = raw === null ? Number.NaN : Number(raw);
    return Number.isFinite(n) ? n : null;
  }

  private touchCheck(): void {
    const now = this.deps.now();
    useUpdateStore.getState().setLastCheck(now);
    try {
      this.deps.localStorage?.setItem(LAST_CHECK_KEY, String(now));
    } catch {
      /* 隐私模式等：仅影响最小间隔记忆 */
    }
  }

  /** §8：是否允许自动检查（非手动）；无循环重试。 */
  shouldAutoCheck(): boolean {
    if (!this.deps.autoCheckEnabled()) return false;
    if (this.checking) return false;
    const last = this.lastCheck();
    return last === null || this.deps.now() - last >= MIN_CHECK_INTERVAL_MS;
  }

  /** §8/§36：检查更新。失败只影响更新功能；错误为 typed 分类 §65。 */
  async checkForUpdate(): Promise<UpdateState> {
    if (this.checking) return this.state;
    this.checking = true;
    this.transition({ kind: 'checking' });
    try {
      this.touchCheck();
      const found = await this.deps.plugin?.check({ timeout: 10_000 });
      if (!found) {
        this.transition({ kind: 'upToDate' });
        return this.state;
      }
      // §46/§47：远程版本/清单校验（不信任远程字段；降级拒绝）。
      const manifestError = this.validateRemote(found);
      if (manifestError) {
        this.transition({ kind: 'failed', error: manifestError });
        return this.state;
      }
      this.active = found;
      this.transition({
        kind: 'available',
        version: found.version,
        notes: found.body ?? '',
      });
      return this.state;
    } catch (e) {
      this.transition({ kind: 'failed', error: classify(e) });
      return this.state;
    } finally {
      this.checking = false;
    }
  }

  private validateRemote(found: {
    version: string;
    body?: string;
    currentVersion?: string;
  }): UpdateError | null {
    const current = found.currentVersion ?? this.deps.currentVersion();
    if (!isNewerVersion(found.version, current)) {
      return { kind: 'Version', detail: `${current} -> ${found.version}` };
    }
    // §46：清单字段面校验（官方插件已验签+下载；这里校验我们消费的字段）。
    return validateManifest({ version: found.version, notes: found.body ?? '' });
  }

  /** §69：下载安装（进度节流 §99；完成后 relaunch §70）。 */
  async downloadAndInstall(): Promise<UpdateState> {
    const update = this.active;
    if (this.state.kind !== 'available' || !update) {
      throw new Error('download requires state=available');
    }
    this.transition({ kind: 'downloading', progress: null });
    let total: number | null = null;
    let downloaded = 0;
    try {
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            total = event.data.contentLength ?? null;
            useUpdateStore.getState().apply({ kind: 'downloading', progress: 0 });
            break;
          case 'Progress': {
            downloaded += event.data.chunkLength;
            const now = Date.now();
            if (now - this.lastProgressEmit >= PROGRESS_THROTTLE_MS) {
              this.lastProgressEmit = now;
              const cur = useUpdateStore.getState().state;
              const p = coerceProgress(
                cur.kind === 'downloading' ? cur.progress : null,
                downloaded,
                total,
              );
              useUpdateStore.getState().apply({ kind: 'downloading', progress: p });
            }
            break;
          }
          case 'Finished':
            break;
        }
      });
      // §70：官方流程落地安装 → relaunch（process plugin）。
      this.transition({ kind: 'downloaded' });
      this.transition({ kind: 'installing' });
      const { relaunch } = await import('@tauri-apps/plugin-process');
      await relaunch();
      this.transition({ kind: 'restartRequired' });
      return this.state;
    } catch (e) {
      const err = classify(e);
      // 经 store 读取（避免 getter 窄化误判；仅下载阶段标记失败 §69）。
      if (useUpdateStore.getState().state.kind === 'downloading') {
        this.transition({ kind: 'failed', error: err });
      }
      return this.state;
    }
  }

  /** §69：Checking/Downloading 可取消；Installing 已提交 = 不可取消。 */
  cancel(): boolean {
    const kind = useUpdateStore.getState().state.kind;
    if (kind !== 'checking' && kind !== 'downloading') return false;
    try {
      this.active?.close();
    } catch {
      /* 忽略关闭错误 */
    }
    this.active = null;
    this.transition({ kind: 'cancelled' });
    return true;
  }
}

/** §65：错误分类（技术细节保留 kind+detail；UI 映射人类文案 §100）。 */
export function classify(e: unknown): UpdateError {
  const detail = e instanceof Error ? e.message : String(e);
  const kind = mapKind(detail.toLowerCase());
  return { kind, detail };
}

function mapKind(lower: string): UpdateErrorKind {
  if (lower.includes('timeout')) return 'Timeout';
  if (lower.includes('signature')) return 'Signature';
  if (lower.includes('manifest')) return 'Manifest';
  if (lower.includes('version')) return 'Version';
  if (lower.includes('network') || lower.includes('connection') || lower.includes('dns')) {
    return 'Network';
  }
  if (lower.includes('permission') || lower.includes('denied')) return 'Permission';
  if (lower.includes('cancel')) return 'Cancelled';
  return 'Unknown';
}

// ---------- 应用单例（App/SettingsPanel 共享同一状态机实例） ----------

export const updateService = new UpdateService();
