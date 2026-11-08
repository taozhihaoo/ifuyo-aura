// Queue drawer（§48）：应用层 in-memory 队列（引用 TrackId，§49）；
// M3-B queue persistence 存在但接线留待 queue store 演进（§50 允许）。
// §197：全应用唯一 queue 权威（palette / drawer / main / mini 都走这里）。
import { useEffect, useState } from 'react';
import { toast, useUiStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { useTranslation } from '../../i18n/react';
import type { TrackSummary } from '../../ipc/types';
import type { IfuyoTrack } from '../../ipc/ifuyo';
import { reportIpcError } from '../../ipc/errors';
import { Icon } from '../../components/Icon';

/** 队列项有两种来源：本地库 TrackId，或远程清单条目（按 URL 播）。
 *  §197「全应用唯一 queue 权威」不变，放宽的只是播放目标——此前远程曲目根本
 *  进不了队列（IfuyoPage 直接 ifuyoPlay，从不建上下文），队列恒空，于是
 *  next/previous 与四种播放模式对在线音乐全部失效。 */
export type QueueItem =
  | { kind: 'local'; track: TrackSummary }
  | { kind: 'remote'; track: IfuyoTrack };

/** in-memory queue（§48 允许：application-layer in-memory queue）。 */
const items: QueueItem[] = [];
let currentIndex = -1; // -1 = 无当前项；next/previous 的游标。
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

/** 播放目标分派：本地用 TrackId，远程用 URL（Rust 侧仍按严格策略再校验一次）。 */
async function playItem(item: QueueItem): Promise<void> {
  if (item.kind === 'local') await ipc.playTrack(item.track.id);
  else await ipc.ifuyoPlay(item.track.url, item.track.title, item.track.artist);
}

function itemKey(item: QueueItem): string {
  return item.kind === 'local' ? `l${item.track.id}` : `r${item.track.url}`;
}

function itemLabel(item: QueueItem): string {
  return item.kind === 'local'
    ? item.track.title ?? item.track.path
    : item.track.title || item.track.url;
}

/** §307：双击库内曲目 → 以该曲目为当前项替换队列上下文（真实 next/previous）。 */
export function playWithQueueContext(track: TrackSummary, context: TrackSummary[]) {
  items.length = 0;
  for (const tr of context) items.push({ kind: 'local', track: tr });
  currentIndex = items.findIndex((it) => it.kind === 'local' && it.track.id === track.id);
  emit();
}

/** 远程同理：以远程清单为上下文，否则在线播放时队列恒空、传输键全部失效。 */
export function playWithRemoteContext(track: IfuyoTrack, context: IfuyoTrack[]) {
  items.length = 0;
  for (const tr of context) items.push({ kind: 'remote', track: tr });
  currentIndex = items.findIndex((it) => it.kind === 'remote' && it.track.url === track.url);
  emit();
}

export function setCurrentIndexByTrack(trackId: number) {
  const i = items.findIndex((it) => it.track.id === trackId);
  if (i >= 0) {
    currentIndex = i;
    emit();
  }
}

/** 按播放模式求「下一曲」游标；null = 到队尾且不应继续（order）。
 *  手动「下一曲」与曲终自动前进共用，避免两条路径语义分叉。 */
function nextIndexFor(mode: PlayMode, from: number): number | null {
  if (items.length === 0) return null;
  switch (mode) {
    case 'order':
    case 'repeatOne': // 单曲循环只影响自动续播，不劫持手动「下一曲」
      return from + 1 < items.length ? from + 1 : null;
    case 'repeatAll':
      return (from + 1) % items.length;
    case 'shuffle': {
      if (items.length === 1) return 0;
      let next = from;
      while (next === from) next = Math.floor(Math.random() * items.length);
      return next;
    }
  }
}

/** §257：执行前检查边界；无下一曲返回 false（调用方不盲目执行）。
 *  repeatAll / shuffle 在队尾仍有下一曲（回绕/随机），与 playNext 一致。 */
export function hasNext(): boolean {
  if (items.length === 0 || currentIndex < 0) return false;
  if (playMode === 'repeatAll' || playMode === 'shuffle') return true;
  return currentIndex + 1 < items.length;
}

export function hasPrevious(): boolean {
  return currentIndex > 0;
}

export async function playNext(): Promise<boolean> {
  // 没有当前项（队列刚建立但游标未定位）时不要自作主张播第一首——
  // 与 hasNext() 的边界语义保持一致。
  if (currentIndex < 0) return false;
  const next = nextIndexFor(playMode, currentIndex);
  if (next === null) return false;
  currentIndex = next;
  emit();
  const item = items[currentIndex];
  if (!item) return false;
  await playItem(item);
  return true;
}

export async function playPrevious(): Promise<boolean> {
  if (!hasPrevious()) return false;
  currentIndex -= 1;
  emit();
  const item = items[currentIndex];
  if (!item) return false;
  await playItem(item);
  return true;
}

/** 点击任意项 → 从该处继续（游标跟随）。 */
export function playItemAt(index: number) {
  if (index < 0 || index >= items.length) return;
  currentIndex = index;
  emit();
  const item = items[index];
  if (item) void playItem(item).catch(reportIpcError);
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
  const had = items.length > 0;
  items.length = 0;
  currentIndex = -1;
  emit();
  if (had) toast('toast.queueCleared', 'success');
}

/** 非 hook 读取（命令层/测试用；React 渲染请用 useQueue）。 */
export function getQueueState(): { items: QueueItem[]; currentIndex: number } {
  return { items, currentIndex };
}

// ---- M12.6 播放模式与曲终自动前进 ----

/** order=顺序（播完停止）/ repeatAll=列表循环 / repeatOne=单曲循环 / shuffle=随机。 */
export type PlayMode = 'order' | 'repeatAll' | 'repeatOne' | 'shuffle';

const PLAY_MODES: PlayMode[] = ['order', 'repeatAll', 'repeatOne', 'shuffle'];
let playMode: PlayMode = 'order';
const modeListeners = new Set<() => void>();

function emitMode() {
  for (const l of modeListeners) l();
}

/** 循环切换播放模式（传输条模式按钮）。 */
export function cyclePlayMode(): PlayMode {
  playMode = PLAY_MODES[(PLAY_MODES.indexOf(playMode) + 1) % PLAY_MODES.length] ?? 'order';
  emitMode();
  return playMode;
}

/** 非 hook 读取（测试用）。 */
export function getPlayMode(): PlayMode {
  return playMode;
}

export function usePlayMode(): PlayMode {
  const [, force] = useState(0);
  useEffect(() => {
    const l = () => force((n) => n + 1);
    modeListeners.add(l);
    return () => {
      modeListeners.delete(l);
    };
  }, []);
  return playMode;
}

/** 曲目自然播完（后端 track-ended 事件）→ 按播放模式自动前进。
 *  后端每曲播完即 Stop 并只发事件，队列全在前端内存里，因此 order 也必须
 *  由这里推进（播到队尾才停）；否则「顺序播放」只播第一首就结束。
 *  返回是否触发了播放。 */
export function advanceOnTrackEnded(): boolean {
  if (items.length === 0) return false;
  if (playMode === 'repeatOne') {
    const i = currentIndex >= 0 ? currentIndex : 0;
    const item = items[i];
    if (!item) return false;
    void playItem(item).catch(reportIpcError);
    return true;
  }
  const next = nextIndexFor(playMode, currentIndex);
  if (next === null) return false;
  playItemAt(next);
  return true;
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
  const mode = usePlayMode();

  // M10 §65/§153：queue 边界上报（SMTC/托盘 Next/Previous enablement）。
  // items 是模块级数组且原地增删，引用永远不变——按 length/游标/模式
  // 作依赖，否则删除队尾项或切换播放模式时上报会静默漏掉。
  useEffect(() => {
    void ipc
      .reportQueueState(hasPrevious(), hasNext())
      .catch(reportIpcError);
  }, [items.length, currentIndex, mode]);

  // M10 §23/§24：SMTC/托盘 next/previous → 复用应用层 queue 语义。
  useEffect(() => {
    // StrictMode 下首次 cleanup 可能早于 Promise resolve；`let un = null`
    // 会让第一次注册的监听永远无法注销，托盘/SMTC 按一次「下一曲」
    // 会执行两次 playNext（一次跳两首）。
    let cancelled = false;
    let off: (() => void) | null = null;
    void ipc
      .listenIntegrationCommand((cmd) => {
        if (cmd === 'next') void playNext();
        else void playPrevious();
      })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        off = fn;
      });
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  // M12.6：后端曲终事件 → 按播放模式自动前进（顺序/循环/随机）。
  useEffect(() => {
    let cancelled = false;
    let off: (() => void) | null = null;
    void ipc
      .listenTrackEnded(() => {
        advanceOnTrackEnded();
      })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        off = fn;
      });
    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

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
          <button className="icon-btn" onClick={() => setOpen(false)} aria-label={t('common.close')}>
            <Icon name="close" size={15} />
          </button>
        </div>
      </div>
      {items.length === 0 ? (
        <p className="queue-empty">{t('queue.empty')}</p>
      ) : (
        <ul className="queue-list">
          {items.map((item, i) => (
            <li key={`${itemKey(item)}-${i}`} className="queue-item">
              <button
                className={`queue-play${i === currentIndex ? ' current' : ''}`}
                aria-current={i === currentIndex ? 'true' : undefined}
                onClick={() => playItemAt(i)}
              >
                {i === currentIndex && <Icon name="music" size={13} className="queue-now-icon" />}
                {itemLabel(item)}
                {item.kind === 'remote' && <span className="queue-src">{t('queue.online')}</span>}
              </button>
              <button
                className="icon-btn queue-remove"
                onClick={() => removeAt(i)}
                aria-label={`${t('queue.remove')} ${itemLabel(item)}`}
                title={t('queue.remove')}
              >
                <Icon name="close" size={12} />
              </button>
            </li>
          ))}
        </ul>
      )}
    </aside>
  );
}
