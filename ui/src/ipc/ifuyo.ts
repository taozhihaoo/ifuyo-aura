// ifuyo 远程曲库 DTO + 校验（§11：IPC 返回值不可信任，先校验再进 state）。
//
// 字段与服务器 `scripts/gen-ifuyo-manifest.py` 的产出一致（v1）；这里多一层
// 形状校验，避免服务端改动后 UI 拿到 undefined 直接渲染成 "null"。

export interface IfuyoTrack {
  id: string;
  title: string;
  artist: string | null;
  album: string | null;
  url: string;
  /** 无标签曲目是真实数据形态 ⇒ null；UI 必须显示 `--:--`，不能当 0 秒。 */
  durationSeconds: number | null;
  sizeBytes: number | null;
  format: string | null;
  etag: string | null;
}

export interface IfuyoList {
  generatedAt: string | null;
  tracks: IfuyoTrack[];
  /** 被安全校验丢弃/去重的条数（如实上报，不静默消失）。 */
  skipped: number;
  truncated: boolean;
}

function isStrOrNull(v: unknown): v is string | null {
  return v === null || typeof v === 'string';
}

function isNumOrNull(v: unknown): v is number | null {
  return v === null || (typeof v === 'number' && Number.isFinite(v));
}

/** 只接受 http(s)：其它 scheme（file:/blob:/data:）不该出现在远程清单里。 */
function isHttpUrl(v: unknown): v is string {
  return typeof v === 'string' && (v.startsWith('http://') || v.startsWith('https://'));
}

function isTrack(v: unknown): v is IfuyoTrack {
  if (typeof v !== 'object' || v === null) return false;
  const t = v as Record<string, unknown>;
  return (
    typeof t.id === 'string' &&
    typeof t.title === 'string' &&
    isHttpUrl(t.url) &&
    isStrOrNull(t.artist) &&
    isStrOrNull(t.album) &&
    isNumOrNull(t.durationSeconds) &&
    isNumOrNull(t.sizeBytes) &&
    isStrOrNull(t.format) &&
    isStrOrNull(t.etag)
  );
}

export function isIfuyoList(v: unknown): v is IfuyoList {
  if (typeof v !== 'object' || v === null) return false;
  const o = v as Record<string, unknown>;
  return (
    isStrOrNull(o.generatedAt) &&
    Array.isArray(o.tracks) &&
    o.tracks.every(isTrack) &&
    typeof o.skipped === 'number' &&
    typeof o.truncated === 'boolean'
  );
}

/** 秒 → m:ss；null/负数一律 `--:--`（不伪造时长）。 */
export function fmtDuration(seconds: number | null): string {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) return '--:--';
  const s = Math.round(seconds);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}
