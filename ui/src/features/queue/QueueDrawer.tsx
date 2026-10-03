// Queue drawer（§48）：应用层 in-memory 队列（引用 TrackId，§49）；
// M3-B queue persistence 存在但接线留待 queue store 演进（§50 允许）。
// §197：全应用唯一 queue 权威（palette / drawer / main / mini 都走这里）。
import { useEffect, useState } from 'react';
import { useUiStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';
import type { TrackSummary } from '../../ipc/types';

export interface QueueItem {
  track: TrackSummary;
}

/** in-memory queue（§48 允许：application-layer in-memory queue）。 */
const items: QueueItem[] = [];
let currentIndex = -1; // -1 = 无当前项；next/previous 的游标。
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

/** §307：双击库内曲目 → 以该曲目为当前项替换队列上下文（真实 next/previous）。 */
export function playWithQueueContext(track: TrackSummary, context: TrackSummary[]) {
  items.length = 0;
  for (const tr of context) items.push({ track: tr });
  currentIndex = items.findIndex((it) => it.track.id === track.id);
  emit();
}

export function setCurrentIndexByTrack(trackId: number) {
  const i = items.findIndex((it) => it.track.id === trackId);
  if (i >= 0) {
    currentIndex = i;
    emit();
  }
}

/** §257：执行前检查边界；无下一曲返回 false（调用方不盲目执行）。 */
export function hasNext(): boolean {
  return currentIndex >= 0 && currentIndex + 1 < items.length;
}

export function hasPrevious(): boolean {
  return currentIndex > 0;
}

export async function playNext(): Promise<boolean> {
  if (!hasNext()) return false;
  currentIndex += 1;
  emit();
  const item = items[currentIndex];
  if (!item) return false;
  await ipc.playTrack(item.track.id);
  return true;
}

export async function playPrevious(): Promise<boolean> {
  if (!hasPrevious()) return false;
  currentIndex -= 1;
  emit();
  const item = items[currentIndex];
  if (!item) return false;
  await ipc.playTrack(item.track.id);
  return true;
}

/** 点击任意项 → 从该处继续（游标跟随）。 */
export function playItemAt(index: number) {
  if (index < 0 || index >= items.length) return;
  currentIndex = index;
  emit();
  const item = items[index];
  if (item) void ipc.playTrack(item.track.id).catch(() => undefined);
}

/** §307 queue remove：移除任意项；当前项之前的移除使游标前移。 */
export function removeAt(index: number) {
  if (index < 0 || index >= items.length) return;
  items.splice(index, 1);
  if (index < currentIndex) currentIndex -= 1;
  else if (index === currentIndex) currentIndex = Math.min(currentIndex, items.length - 1);
  emit();
}

export function clearQueue() {
  items.length = 0;
  currentIndex = -1;
  emit();
}

/** 非 hook 读取（命令层/测试用；React 渲染请用 useQueue）。 */
export function getQueueState(): { items: QueueItem[]; currentIndex: number } {
  return { items, currentIndex };
}

export function useQueue(): { items: QueueItem[]; currentIndex: number } {
  const [, force] = useState(0);
  useEffect(() => {
    const l = () => force((n) => n + 1);
    listeners.add(l);
    return () => {
      listeners.delete(l);
    };
  }, []);
  return { items, currentIndex };
}

export function QueueDrawer() {
  const open = useUiStore((s) => s.queueOpen);
  const setOpen = useUiStore((s) => s.setQueueOpen);
  const { items, currentIndex } = useQueue();
  const { t } = useTranslation();

  // Esc 关闭（全局快捷键也处理；这里保证 focus 返回）。
  useEffect(() => {
    if (open) {
      const first = document.querySelector<HTMLElement>('.queue-drawer button');
      first?.focus();
    }
  }, [open]);

  if (!open) return null;

  return (
    <aside className="queue-drawer" role="dialog" aria-label={t('queue.title')}>
      <div className="queue-head">
        <h2>{t('queue.title')}</h2>
        <div>
          <button className="btn" onClick={() => clearQueue()}>{t('queue.clear')}</button>
          <button className="icon-btn" onClick={() => setOpen(false)} aria-label="Esc">✕</button>
        </div>
      </div>
      {items.length === 0 ? (
        <p className="queue-empty">{t('queue.empty')}</p>
      ) : (
        <ul className="queue-list">
          {items.map((item, i) => (
            <li key={`${item.track.id}-${i}`} className="queue-item">
              <button
                className="queue-play"
                aria-current={i === currentIndex ? 'true' : undefined}
                onClick={() => playItemAt(i)}
              >
                {i === currentIndex ? '▶ ' : ''}
                {item.track.title ?? item.track.path}
              </button>
              <button
                className="icon-btn queue-remove"
                onClick={() => removeAt(i)}
                aria-label={`${t('queue.remove')} ${item.track.title ?? item.track.path}`}
                title={t('queue.remove')}
              >
                ✕
              </button>
            </li>
          ))}
        </ul>
      )}
    </aside>
  );
}
