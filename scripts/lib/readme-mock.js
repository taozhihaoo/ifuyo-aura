/* README 截图用的 Tauri IPC 仿真层（仅在无头浏览器里加载）。
 *
 * 目的：让真实的 React UI 在浏览器里渲染，并提供一份**演示数据集**，
 * 以便为 README 生成英/中两套界面截图。它不参与产品构建，也不改变
 * 应用的任何真实行为——所有数据都是虚构的，仅用于展示界面。
 *
 * 由 scripts/capture-readme-shots.mjs 注入到 index.html 的 <head> 里，
 * 早于应用 bundle 执行；通过 ?lang=en|zh-CN 选择界面语言。
 */
(function mockAura() {
  const params = new URLSearchParams(location.search);
  const lang = params.get('lang') === 'en' ? 'en' : 'zh-CN';
  const zh = lang === 'zh-CN';

  const COPY = {
    zh: {
      nowTitle: '光与梦的边界',
      nowArtist: '林间海',
      nowAlbum: '夜航',
      tracks: [
        ['夜航星图', '林间海', '夜航', 236],
        ['玻璃海岸', '林间海', '玻璃海岸', 212],
        ['光与梦的边界', '林间海', '夜航', 254],
        ['潮汐来信', '白鲸电台', '深海邮局', 198],
        ['在云端失重', '林间海', '失重', 241],
        ['微光城市', '远山', '微光城市', 227],
        ['木星环', '伊芙', '行星组曲', 305],
        ['雨落无声', '白鲸电台', '深海邮局', 189],
        ['夏日回声', '远山', '微光城市', 216],
        ['极夜微光', '林间海', '夜航', 268],
        ['星尘列车', '伊芙', '行星组曲', 243],
        ['月亮与六便士', '远山', '微光城市', 231],
      ],
      lyrics: [
        '穿过夜的缝隙',
        '我听见光的呼吸',
        '在梦与醒的边界',
        '有人轻轻哼着旋律',
        '风把星星吹得很远',
        '落在你摊开的掌心',
        '如果这是一场航行',
        '就让时间慢一点',
        '让每一束微光',
        '都有名字',
        '让每一次心跳',
        '都落在同一个节拍',
        '我们沿着银河散步',
        '把沉默走成了歌',
        '天亮以前',
        '请再听一遍',
      ],
    },
    en: {
      nowTitle: 'Edge of Light',
      nowArtist: 'Harbor Lights',
      nowAlbum: 'Nightfall',
      tracks: [
        ['Night Chart', 'Harbor Lights', 'Nightfall', 236],
        ['Glass Coast', 'Harbor Lights', 'Glass Coast', 212],
        ['Edge of Light', 'Harbor Lights', 'Nightfall', 254],
        ['Letters from the Tide', 'Whale Radio', 'Deep Sea Post', 198],
        ['Weightless', 'Harbor Lights', 'Weightless', 241],
        ['City of Glimmer', 'Far Hills', 'City of Glimmer', 227],
        ['Jupiter Ring', 'Ifuyo', 'Planet Suite', 305],
        ['Silent Rain', 'Whale Radio', 'Deep Sea Post', 189],
        ['Summer Echo', 'Far Hills', 'City of Glimmer', 216],
        ['Polar Light', 'Harbor Lights', 'Nightfall', 268],
        ['Stardust Train', 'Ifuyo', 'Planet Suite', 243],
        ['Moon and Sixpence', 'Far Hills', 'City of Glimmer', 231],
      ],
      lyrics: [
        'Through the seams of the night',
        'I can hear the light breathe',
        'On the edge of dream and waking',
        'Someone hums a quiet melody',
        'The wind carries the stars away',
        'And leaves them in your open hand',
        'If this is a voyage',
        'Let the hours move slowly',
        'Let every sliver of light',
        'Find itself a name',
        'Let every heartbeat',
        'Land on the same beat',
        'We walk along the Milky Way',
        'And turn the silence into song',
        'Before the morning comes',
        'Play it one more time',
      ],
    },
  };

  const c = zh ? COPY.zh : COPY.en;

  /** 程序化生成的封面（无版权素材，纯展示）。 */
  function cover(from, to, accent, seed) {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="600" height="600" viewBox="0 0 600 600">
      <defs>
        <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stop-color="${from}"/><stop offset="1" stop-color="${to}"/>
        </linearGradient>
        <radialGradient id="h" cx="${0.3 + (seed % 4) * 0.1}" cy="0.28" r="0.7">
          <stop offset="0" stop-color="${accent}" stop-opacity=".85"/>
          <stop offset="1" stop-color="${accent}" stop-opacity="0"/>
        </radialGradient>
      </defs>
      <rect width="600" height="600" fill="url(#g)"/>
      <circle cx="${180 + (seed % 3) * 60}" cy="200" r="240" fill="url(#h)"/>
      <circle cx="${430 - (seed % 3) * 40}" cy="430" r="120" fill="none" stroke="#ffffff" stroke-opacity=".22" stroke-width="1.5"/>
      <circle cx="${430 - (seed % 3) * 40}" cy="430" r="72" fill="none" stroke="#ffffff" stroke-opacity=".14" stroke-width="1"/>
      <path d="M0 470 Q150 400 300 470 T600 470 V600 H0 Z" fill="#0B0E14" fill-opacity=".35"/>
    </svg>`;
    return `data:image/svg+xml,${encodeURIComponent(svg)}`;
  }

  const PALETTE = [
    ['#1b2440', '#0b0e14', '#5CC8F2'],
    ['#3a1f38', '#0b0e14', '#F2789F'],
    ['#251f45', '#0b0e14', '#8B7CF6'],
    ['#123028', '#0b0e14', '#6FE3C1'],
    ['#2c2033', '#111624', '#F2789F'],
    ['#14243a', '#0b0e14', '#5CC8F2'],
  ];

  const tracks = c.tracks.map((t, i) => ({
    id: i + 1,
    title: t[0],
    artist: t[1],
    album: t[2],
    durationSeconds: t[3],
    year: 2024 + (i % 3),
    path: `C:\\Music\\ifuyo\\${String(i + 1).padStart(2, '0')} ${t[0]}.flac`,
    hasArtwork: true,
  }));

  const covers = {};
  tracks.forEach((t, i) => {
    const p = PALETTE[i % PALETTE.length];
    covers[t.id] = cover(p[0], p[1], p[2], i + 1);
  });

  const current = tracks[2];
  const positionSeconds = 92;
  let snapshot = {
    state: 'playing',
    positionSeconds,
    durationSeconds: current.durationSeconds,
    volume: 0.72,
    trackPath: current.path,
    title: current.title,
    seekable: true,
    revision: 7,
  };

  const settings = {
    version: 1,
    language: lang,
    updatesAutoCheck: true,
    mojibakeFix: true,
    analysisAuto: true,
    uiScale: 1,
    reducedMotion: false,
    loudnessEnabled: false,
    crossfadeSeconds: 4,
    eqEnabled: true,
    eqPreset: 2,
    micaEnabled: true,
  };

  const LYRICS = {
    synced: true,
    lines: c.lyrics.map((text, i) => ({ timeSeconds: i * 15 + 4, text })),
  };

  let callbackId = 1;
  const callbacks = {};

  async function invoke(cmd, args) {
    switch (cmd) {
      case 'app_info':
        return {
          name: 'ifuyo Aura',
          version: '0.1.0',
          slogan: 'Weave light and dreams into every session.',
          phase: 'M15',
        };
      case 'get_settings':
        return settings;
      case 'save_settings':
        return null;
      case 'player_snapshot':
        return snapshot;
      case 'player_position':
        return snapshot.positionSeconds;
      case 'player_level':
        return { rms: 0.19, low: 0.26, mid: 0.13, high: 0.09 };
      case 'library_list':
        return tracks;
      case 'library_search': {
        const q = String(args?.query ?? '').toLowerCase();
        return tracks.filter((t) =>
          `${t.title} ${t.artist} ${t.album}`.toLowerCase().includes(q),
        );
      }
      case 'library_stats':
        return { tracks: tracks.length, artists: 6, albums: 8 };
      case 'track_id_by_path':
        return tracks.find((t) => t.path === args?.path)?.id ?? null;
      case 'track_artwork':
        return covers[args?.trackId] ?? null;
      case 'track_meta': {
        const t = tracks.find((x) => x.id === args?.trackId);
        return t ? { title: t.title, artist: t.artist, album: t.album } : null;
      }
      case 'lyrics_for_track':
        return LYRICS;
      case 'ifuyo_list':
        return { tracks: [], skipped: 0, truncated: false };
      case 'analysis_status':
        return { progress: { processed: 0, total: 0, failed: 0, running: false }, analyzed: tracks.length, pending: 0 };
      case 'set_volume':
        snapshot = { ...snapshot, volume: args?.volume ?? snapshot.volume };
        return null;
      case 'pause':
        snapshot = { ...snapshot, state: 'paused' };
        return null;
      case 'play':
        snapshot = { ...snapshot, state: 'playing' };
        return null;
      case 'stop':
        snapshot = { ...snapshot, state: 'stopped', positionSeconds: 0 };
        return null;
      case 'seek':
        snapshot = { ...snapshot, positionSeconds: args?.positionSeconds ?? 0 };
        return null;
      case 'play_track': {
        const t = tracks.find((x) => x.id === args?.trackId);
        if (t) {
          snapshot = {
            ...snapshot,
            state: 'playing',
            trackPath: t.path,
            title: t.title,
            positionSeconds: 0,
            durationSeconds: t.durationSeconds,
            revision: snapshot.revision + 1,
          };
        }
        return null;
      }
      case 'plugin:event|listen':
        return callbackId++;
      case 'plugin:event|unlisten':
        return null;
      case 'plugin:window|get_all_windows':
      case 'plugin:webview|get_all_webviews':
        return [];
      default:
        return null;
    }
  }

  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { windowLabel: 'main', label: 'main' },
    },
    invoke,
    transformCallback(cb) {
      const id = callbackId++;
      callbacks[id] = cb;
      return id;
    },
    unregisterCallback(id) {
      delete callbacks[id];
    },
    convertFileSrc(p) {
      return p;
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener() {
      /* 单页截图无需真正注销 */
    },
  };
})();
