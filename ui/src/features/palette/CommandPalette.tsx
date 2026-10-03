// Command Palette（§98–§102）：Ctrl+K、键盘导航、focus trap/restore。
import { useEffect, useMemo, useRef, useState } from 'react';
import { useUiStore, usePlayerStore, useSettingsStore } from '../../state/stores';
import { ipc } from '../../ipc/client';
import { buildCommands, matchCommand, type Command } from '../../commands/registry';
import { useTranslation } from '../../i18n/react';

export function CommandPalette() {
  const open = useUiStore((s) => s.commandPaletteOpen);
  const setOpen = useUiStore((s) => s.setPaletteOpen);
  const setQueueOpen = useUiStore((s) => s.setQueueOpen);
  const setSettingsOpen = useUiStore((s) => s.setSettingsOpen);
  const setView = useUiStore((s) => s.setView);
  const setMiniPlayerOpen = useUiStore((s) => s.setMiniPlayerOpen);
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
      next: () => undefined, // M3 queue 集成后续补充
      previous: () => undefined,
      stop: () => void ipc.stop().catch(() => undefined),
      openFolder: () => setView('library'),
      openUrl: () => setView('library'),
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
    }),
    [setView, setQueueOpen, setSettingsOpen, setMiniPlayerOpen, snapshot?.state],
  );

  const commands: Command[] = useMemo(
    () => buildCommands(ctx, (k) => t(k)),
    [ctx, t],
  );
  const filtered = commands.filter((c) => matchCommand(c, query));

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
