// §253/§254/§255/§256：统一的时长/数值安全格式化。
// 所有来自 Rust 的数值（可能 null/负/超界）在此防御；组件不再各自实现。

/** 秒 → `m:ss`（<1h）或 `h:mm:ss`（≥1h）。非法输入（null/NaN/负/∞）→ null，
 *  由调用方展示明确的 Live/Unknown 文案（§254，绝不 NaN:NaN）。 */
export function formatDuration(seconds: number | null | undefined): string | null {
  if (seconds === null || seconds === undefined) return null;
  if (!Number.isFinite(seconds) || seconds < 0) return null;
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  return `${m}:${String(s).padStart(2, '0')}`;
}

/** 数值钳制（§255：volume 等超界防御）。 */
export function clampNumber(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(Math.max(value, min), max);
}

/** §256：进度比例 0..1；duration 非法/为 0 → 0（进度条不溢出）。 */
export function safeProgress(position: number, duration: number | null | undefined): number {
  if (duration === null || duration === undefined || !Number.isFinite(duration) || duration <= 0) {
    return 0;
  }
  if (!Number.isFinite(position) || position < 0) return 0;
  return Math.min(position / duration, 1);
}
