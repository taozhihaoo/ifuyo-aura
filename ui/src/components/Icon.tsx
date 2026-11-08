// 统一的线性图标集（§28：克制、单一线宽、无彩色 emoji）。
// 全部以 currentColor 描边，尺寸随字号，天然跟随 hover/active/focus 态。
import type { ReactNode, SVGProps } from 'react';

export type IconName =
  | 'music'
  | 'library'
  | 'queue'
  | 'search'
  | 'settings'
  | 'minimize'
  | 'maximize'
  | 'close'
  | 'previous'
  | 'next'
  | 'stop'
  | 'play'
  | 'pause'
  | 'volume'
  | 'order'
  | 'repeatAll'
  | 'repeatOne'
  | 'shuffle'
  | 'lyrics'
  | 'refresh'
  | 'sparkle';

const SHAPES: Record<IconName, ReactNode> = {
  music: (
    <>
      <path d="M9 18V5l10-2v13" />
      <circle cx="6" cy="18" r="3" />
      <circle cx="16" cy="16" r="3" />
    </>
  ),
  library: (
    <>
      <rect x="3" y="4" width="4" height="4" rx="1" />
      <rect x="3" y="10" width="4" height="4" rx="1" />
      <rect x="3" y="16" width="4" height="4" rx="1" />
      <path d="M10 6h11M10 12h11M10 18h11" />
    </>
  ),
  queue: (
    <>
      <path d="M4 6h12M4 11h12M4 16h7" />
      <path d="m16 14 5 3-5 3z" />
    </>
  ),
  search: (
    <>
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.8-3.8" />
    </>
  ),
  settings: (
    <>
      <path d="M4 7h8M18 7h2M4 17h2M12 17h8" />
      <circle cx="15" cy="7" r="2.4" />
      <circle cx="9" cy="17" r="2.4" />
    </>
  ),
  minimize: <path d="M5 12h14" />,
  maximize: <rect x="5" y="5" width="14" height="14" rx="2.5" />,
  close: <path d="m6 6 12 12M18 6 6 18" />,
  previous: (
    <>
      <path d="M18 5 8 12l10 7V5z" />
      <path d="M6 5v14" />
    </>
  ),
  next: (
    <>
      <path d="M6 5l10 7L6 19V5z" />
      <path d="M18 5v14" />
    </>
  ),
  stop: <rect x="6" y="6" width="12" height="12" rx="2.5" />,
  play: <path d="M8 5.5 19 12 8 18.5V5.5z" fill="currentColor" stroke="none" />,
  pause: (
    <>
      <rect x="7" y="5" width="3.5" height="14" rx="1" fill="currentColor" stroke="none" />
      <rect x="13.5" y="5" width="3.5" height="14" rx="1" fill="currentColor" stroke="none" />
    </>
  ),
  volume: (
    <>
      <path d="M4 9v6h4l5 4V5L8 9H4z" />
      <path d="M17 9a4.2 4.2 0 0 1 0 6M19.5 6.5a7.6 7.6 0 0 1 0 11" />
    </>
  ),
  order: <path d="M4 12h15M13 6l6 6-6 6" />,
  repeatAll: (
    <>
      <path d="m17 2 4 4-4 4M3 11V9a3 3 0 0 1 3-3h15M7 22l-4-4 4-4M21 13v2a3 3 0 0 1-3 3H3" />
    </>
  ),
  repeatOne: (
    <>
      <path d="m17 2 4 4-4 4M3 11V9a3 3 0 0 1 3-3h15M7 22l-4-4 4-4M21 13v2a3 3 0 0 1-3 3H3" />
      <path d="M11 10.5 12.5 9l1.5 1.5M12.5 9v6" />
    </>
  ),
  shuffle: (
    <>
      <path d="M3 6h3c4.5 0 6.5 12 12 12h3M18 15l3 3-3 3M3 18h3c2 0 3.5-2 5-4M14 8c1.5-1.5 2.5-2 4-2h3M18 3l3 3-3 3" />
    </>
  ),
  lyrics: (
    <>
      <path d="M4 5h16v11H9l-5 4V5z" />
      <path d="M8 9h8M8 12h5" />
    </>
  ),
  refresh: <path d="M20 11a8 8 0 1 0-2 6M20 5v6h-6" />,
  sparkle: (
    <>
      <path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8L12 3z" />
      <path d="M19 17l.7 2 .3 2M5 4l.7 2L6 8" />
    </>
  ),
};

interface IconProps extends Omit<SVGProps<SVGSVGElement>, 'name'> {
  name: IconName;
  size?: number;
}

export function Icon({ name, size = 18, className, ...rest }: IconProps) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {SHAPES[name]}
    </svg>
  );
}
