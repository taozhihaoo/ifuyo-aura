// M11 §7/§65：更新状态机（typed union——禁止 string 弱类型核心状态）
// 与 typed 错误分类。纯类型 + 纯函数（可 node 测试，§267）。

export type UpdateState =
  | { kind: 'idle' }
  | { kind: 'checking' }
  | { kind: 'upToDate' }
  | { kind: 'available'; version: string; notes: string }
  | { kind: 'downloading'; progress: number | null } // null = 未知大小（§68 不伪造百分比）
  | { kind: 'downloaded' }
  | { kind: 'installing' }
  | { kind: 'restartRequired' }
  | { kind: 'failed'; error: UpdateError }
  | { kind: 'cancelled' };

/** §65 typed 错误分类（UI 映射为人类可读文案；技术细节进 diagnostics）。 */
export type UpdateErrorKind =
  | 'Network'
  | 'Timeout'
  | 'Manifest'
  | 'Signature'
  | 'Version'
  | 'Platform'
  | 'Architecture'
  | 'Permission'
  | 'Installer'
  | 'Cancelled'
  | 'Unknown';

export interface UpdateError {
  kind: UpdateErrorKind;
  /** 人类可读技术细节（不含 secret §65）。 */
  detail?: string;
}

/** §14/§46：更新清单（字段以 Tauri updater 协议为准；不信任远程字段）。 */
export interface UpdateManifest {
  version: string;
  notes: string;
  pubDate: string | null;
  platforms: Record<string, { signature: string; url: string }>;
}

/** §7 状态机合法转移表（纯数据；测试矩阵 §96 用）。 */
export const TRANSITIONS: Record<UpdateState['kind'], readonly UpdateState['kind'][]> = {
  idle: ['checking'],
  checking: ['upToDate', 'available', 'failed', 'cancelled'],
  upToDate: ['checking'],
  available: ['downloading', 'idle'],
  downloading: ['downloaded', 'failed', 'cancelled'],
  downloaded: ['installing', 'idle'],
  installing: ['restartRequired', 'failed'],
  restartRequired: ['idle'],
  failed: ['idle', 'checking'],
  cancelled: ['idle', 'checking'],
};

/** 状态转移是否合法（非法转移 = 编程错误，拒绝并保持原状态）。 */
export function canTransition(from: UpdateState, to: UpdateState): boolean {
  return TRANSITIONS[from.kind].includes(to.kind);
}

/** §47：semver MAJOR.MINOR.PATCH 比较；非法版本 = null（调用方拒绝）。 */
export function parseSemver(v: string): [number, number, number] | null {
  const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(v.trim());
  if (!m) return null;
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

export function isNewerVersion(remote: string, current: string): boolean {
  const r = parseSemver(remote);
  const c = parseSemver(current);
  if (!r || !c) return false; // §46 非法版本拒绝
  return r[0] !== c[0]
    ? r[0] > c[0]
    : r[1] !== c[1]
      ? r[1] > c[1]
      : r[2] > c[2]; // 相等/更低 = 非更新（§47 降级拒绝）
}

/** §46：远程清单 schema 验证（不信任远程 JSON 字段）。 */
export function validateManifest(raw: unknown): UpdateError | null {
  if (typeof raw !== 'object' || raw === null) {
    return { kind: 'Manifest', detail: 'manifest is not an object' };
  }
  const m = raw as Record<string, unknown>;
  if (typeof m.version !== 'string' || parseSemver(m.version) === null) {
    return { kind: 'Manifest', detail: 'invalid version' };
  }
  if (m.notes !== undefined && typeof m.notes !== 'string') {
    return { kind: 'Manifest', detail: 'invalid notes' };
  }
  if (m.pub_date !== undefined && m.pub_date !== null && typeof m.pub_date !== 'string') {
    return { kind: 'Manifest', detail: 'invalid pub_date' };
  }
  // platforms 仅在直接消费清单文件时校验（官方插件路径不暴露该字段）；
  // 存在则必须非空且每项合法（§46）。
  if (m.platforms !== undefined) {
    if (typeof m.platforms !== 'object' || m.platforms === null) {
      return { kind: 'Manifest', detail: 'invalid platforms' };
    }
    const platforms = m.platforms as Record<string, unknown>;
    if (Object.keys(platforms).length === 0) {
      return { kind: 'Manifest', detail: 'empty platforms' };
    }
    for (const [key, entry] of Object.entries(platforms)) {
      if (typeof entry !== 'object' || entry === null) {
        return { kind: 'Manifest', detail: `platform ${key} invalid` };
      }
      const e = entry as Record<string, unknown>;
      if (typeof e.signature !== 'string' || e.signature.length === 0) {
        return { kind: 'Manifest', detail: `platform ${key} missing signature` };
      }
      if (typeof e.url !== 'string' || !/^https:\/\//.test(e.url)) {
        // §9：production 仅 HTTPS（dev 本地端点走独立配置，不经此清单）。
        return { kind: 'Manifest', detail: `platform ${key} url must be https` };
      }
    }
  }
  return null;
}

/** §68：进度有界单调（0–100；未知大小 = null；不回跳不超界）。 */
export function coerceProgress(
  previous: number | null,
  downloaded: number,
  total: number | null,
): number | null {
  if (total === null || !Number.isFinite(total) || total <= 0) return null;
  if (!Number.isFinite(downloaded) || downloaded < 0) return previous;
  const pct = Math.min(100, Math.max(0, Math.round((downloaded / total) * 100)));
  if (previous !== null && pct < previous) return previous; // 单调
  return pct;
}
