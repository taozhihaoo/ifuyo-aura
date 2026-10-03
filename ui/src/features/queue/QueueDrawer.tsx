// Queue drawer（§48）：应用层 in-memory 队列（引用 TrackId，§49）；
// M3-B queue persistence 存在但接线留待 queue store 演进（§50 允许）。
import { useEffect, useState } from 'react';
import { useUiStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';
import type { TrackSummary } from '../../ipc/types';

interface QueueItem {
  track: TrackSummary;
}

/** in-memory queue（§48 允许：application-layer in-memory queue）。 */
const items: QueueItem[] = [];
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

export function enqueue(track: TrackSummary) {
  items.push({ track });
  emit();
}

export function clearQueue() {
  items.length = 0;
  emit();
}

export function useQueue(): QueueItem[] {
  const [, force] = useState(0);
  useEffect(() => {
    const l = () => force((n) => n + 1);
    listeners.add(l);
    return () => {
      listeners.delete(l);
    };
  }, []);
  return items;
}

export function QueueDrawer() {
  const open = useUiStore((s) => s.queueOpen);
  const setOpen = useUiStore((s) => s.setQueueOpen);
  const queue = useQueue();
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
          <button className="btn" onClick={() => clearQueue()}>{t('controls.stop')}</button>
          <button className="icon-btn" onClick={() => setOpen(false)} aria-label="Esc">✕</button>
        </div>
      </div>
      {queue.length === 0 ? (
        <p className="queue-empty">{t('queue.empty')}</p>
      ) : (
        <ul className="queue-list">
          {queue.map((item, i) => (
            <li key={`${item.track.id}-${i}`} className="queue-item">
              <button
                className="queue-play"
                onClick={() => void ipc.playTrack(item.track.id).catch(() => undefined)}
              >
                ▶ {item.track.title ?? item.track.path}
              </button>
            </li>
          ))}
        </ul>
      )}
    </aside>
  );
}
