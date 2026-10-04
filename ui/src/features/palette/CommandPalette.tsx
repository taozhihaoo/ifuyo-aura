// Command Palette（§98–§102）：Ctrl+K、键盘导航、focus trap/restore。
import { useEffect, useMemo, useRef, useState } from 'react';
import { useUiStore, usePlayerStore, useSettingsStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { errorToI18nKey } from '../../ipc/errors';
import { buildCommands, matchCommand, type Command } from '../../commands/registry';
import { useTranslation } from '../../i18n/react';
import { playNext, playPrevious } from '../queue/QueueDrawer';

// §274：与 M3 SUPPORTED_EXTENSIONS 同步（crates/aura-library/src/scanner/mod.rs）。
const AUDIO_EXTENSIONS = ['flac', 'mp3', 'ogg', 'oga', 'wav', 'm4a', 'alac'];

/** §276/§277：带 scheme 的输入（合法或非法）都交给 M4 resolver 判定。 */
function looksLikeUri(q: string): boolean {
  return /^[a-z][a-z0-9+.-]*:\S+/i.test(q.trim());
}

async function runOpenInput(input: string, setNotice: (k: string | null) => void) {
  try {
    await ipc.playInput(input);
  } catch (e) {
    setNotice(errorToI18nKey(e));
  }
}

export function CommandPalette() {
  const open = useUiStore((s) => s.commandPaletteOpen);
  const setOpen = useUiStore((s) => s.setPaletteOpen);
  const setQueueOpen = useUiStore((s) => s.setQueueOpen);
  const setSettingsOpen = useUiStore((s) => s.setSettingsOpen);
  const setView = useUiStore((s) => s.setView);
  const setMiniPlayerOpen = useUiStore((s) => s.setMiniPlayerOpen);
  const setNotice = useUiStore((s) => s.setNotice);
  const snapshot = usePlayerStore((s) => s.snapshot);
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const restoreRef = useRef<HTMLElement | null>(null);

  const ctx = useMemo(
    () => ({
      setView,
      play: () => void ipc.play().catch(() => undefined),
      pause: () => void ipc.pause().catch(() => undefined),
      next: () => void playNext().catch((e) => setNotice(errorToI18nKey(e))),
      previous: () => void playPrevious().catch((e) => setNotice(errorToI18nKey(e))),
      stop: () => void ipc.stop().catch(() => undefined),
      openFolder: () => {
        void import('@tauri-apps/plugin-dialog').then(({ open }) =>
          open({ directory: true, multiple: false, title: t('library.addFolder') })
            .then((path) => {
              if (typeof path === 'string') {
                setView('library');
                void ipc.scanFolder(path).catch((e) => setNotice(errorToI18nKey(e)));
              }
            })
            .catch(() => undefined),
        );
      },
      openUrl: () => {
        void import('@tauri-apps/plugin-dialog').then(({ open }) =>
          open({
            multiple: false,
            title: t('library.openFile'),
            filters: [{ name: 'Audio', extensions: AUDIO_EXTENSIONS }],
          })
            .then((path) => {
              if (typeof path === 'string') void runOpenInput(path, setNotice);
            })
            .catch(() => undefined),
        );
      },
      toggleQueue: () => setQueueOpen(!useUiStore.getState().queueOpen),
      toggleEq: () => setSettingsOpen(true),
      toggleLoudness: () => {
        const s = useSettingsStore.getState();
        const next = !s.loudnessEnabled;
        s.setAll({ ...s, loudnessEnabled: next });
        void ipc.setLoudness(next, true).catch(() => undefined);
      },
      toggleMiniPlayer: () => setMiniPlayerOpen(!useUiStore.getState().miniPlayerOpen),
      focusLibrarySearch: () => {
        setView('library');
        window.setTimeout(() => document.getElementById('library-search')?.focus(), 50);
      },
      isPlaying: () => snapshot?.state === 'playing',
      // ---- Dreamscape（§197；§103 再生仅显式；§104 只复制安全 seed）----
      openDreamscape: () => setView('dreamscape'),
      regenerateDreamscape: () => {
        // §103：用户显式再生。输入 = 当前本地时间串（D-088：可再现于
        // 同一分钟；需要精确世界请用「导入 seed」）。
        const stamp = new Date().toISOString().slice(0, 16);
        void ipc
          .dreamscapeRegenerate(stamp)
          .then(() => setView('dreamscape'))
          .catch((e) => setNotice(errorToI18nKey(e)));
      },
      copyDreamscapeSeed: () => {
        void ipc
          .dreamscapeExportSeed()
          .then((seed) => navigator.clipboard.writeText(seed))
          .then(() => setNotice('dreamscape.seedCopied'))
          .catch(() => setNotice('dreamscape.copyFailed'));
      },
      saveDreamscapeSnapshot: () => {
        void ipc
          .dreamscapeSaveSnapshot()
          .then(() => setNotice('dreamscape.snapshotSaved'))
          .catch((e) => setNotice(errorToI18nKey(e)));
      },
    }),
    [setView, setQueueOpen, setSettingsOpen, setMiniPlayerOpen, snapshot?.state],
  );

  const commands: Command[] = useMemo(
    () => buildCommands(ctx, (k) => t(k)),
    [ctx],
  );
  // §276：输入形如 scheme:… 时给出动态「打开 URL/文件」命令（含非法
  // scheme——一律交 M4 拒绝并经错误映射提示，§277）。
  const synthetic: Command[] = looksLikeUri(query)
    ? [
        {
          id: 'open.url',
          title: `${t('palette.openUrl')} — ${query.trim()}`,
          titleKey: 'palette.openUrl',
          keywords: 'open url http https stream',
          execute: () => void runOpenInput(query.trim(), setNotice),
        },
      ]
    : [];
  const filtered = [...synthetic, ...commands].filter((c) => matchCommand(c, query));

  // 打开：focus input + 记录恢复目标；关闭：恢复 focus（§102/§159）。
  useEffect(() => {
    if (open) {
      restoreRef.current = document.activeElement as HTMLElement | null;
      setQuery('');
      setSel(0);
      window.setTimeout(() => inputRef.current?.focus(), 20);
    } else {
      restoreRef.current?.focus?.();
      restoreRef.current = null;
    }
  }, [open]);

  if (!open) return null;

  const execute = (c: Command | undefined) => {
    if (!c) return;
    setOpen(false);
    c.execute();
  };

  return (
    <div className="palette-overlay" role="presentation" onClick={() => setOpen(false)}>
      <div
        className="palette"
        role="dialog"
        aria-modal="true"
        aria-label={t('palette.placeholder')}
        onClick={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          className="palette-input"
          value={query}
          placeholder={t('palette.placeholder')}
          onChange={(e) => {
            setQuery(e.target.value);
            setSel(0);
          }}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') {
              e.preventDefault();
              setSel((s) => Math.min(s + 1, filtered.length - 1));
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              setSel((s) => Math.max(s - 1, 0));
            } else if (e.key === 'Enter') {
              execute(filtered[sel]);
            } else if (e.key === 'Escape') {
              setOpen(false);
            }
          }}
          aria-label={t('palette.placeholder')}
        />
        <ul className="palette-list" role="listbox">
          {filtered.map((c, i) => (
            <li key={c.id}>
              <button
                className={`palette-item${i === sel ? ' selected' : ''}`}
                role="option"
                aria-selected={i === sel}
                onMouseEnter={() => setSel(i)}
                onClick={() => execute(c)}
              >
                <span>{c.title}</span>
                {c.shortcut && <kbd>{c.shortcut}</kbd>}
              </button>
            </li>
          ))}
          {filtered.length === 0 && (
            <li className="palette-empty">{t('palette.noResults')}</li>
          )}
        </ul>
      </div>
    </div>
  );
}
