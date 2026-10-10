// M15b 诊断/回归：悬浮歌词数据更新（realm 清单登记修复）+ 程序化拖拽。
// 用法：node scripts/verify-desktop-lyrics-data.mjs
import { startApp, connectCdp, stopApp } from './lib/qa-app.mjs';
import { cpSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const CDP = 'http://127.0.0.1:9226';

const run = async () => {
  const musicDir = mkdtempSync(join(tmpdir(), 'aura-dlverify-'));
  cpSync(join(process.cwd(), 'tests', 'fixtures'), musicDir, { recursive: true });
  const { child, dataDir } = await startApp({ cdpPort: 9226, dataDir: musicDir });
  try {
    const { inv } = await connectCdp(9226);
    await inv('scan_folder', { path: musicDir });
    await sleep(5000);

    // 曲目定位（播放挪到悬浮窗打开后——fixture 仅 4s，先开窗再播才能采到）
    const tracks = await inv('library_list', { sort: 'title', limit: 50, offset: 0 });
    const lrc = tracks.find((t) => t.path?.endsWith('fixture-lrc.mp3'));
    if (!lrc) throw new Error('fixture-lrc.mp3 未入库');

    // 切曲库视图 → 打开悬浮歌词窗
    const { evalJs } = await connectCdp(9226);
    await evalJs(
      "[...document.querySelectorAll('.nav-btn,.side-btn')].find((b) => b.textContent.includes('音乐库') || b.textContent.includes('Library'))?.click();1",
    );
    await sleep(800);
    await evalJs("document.querySelector('.pb-lyrics')?.click();1");
    await sleep(3000);
    // 悬浮窗就绪后（重）播放——从 0 开始，采样窗口覆盖 4s 全程
    await inv('play_track', { trackId: lrc.id });

    // 附着悬浮窗页面，采样歌词行更新
    const targets = await (await fetch(CDP + '/json')).json();
    const ovPage = targets.find((t) => t.type === 'page' && t.url.includes('view=lyrics'));
    if (!ovPage) throw new Error('悬浮窗页面未创建');
    const ws = new WebSocket(ovPage.webSocketDebuggerUrl);
    let id = 0;
    const pend = new Map();
    const send = (m, p) =>
      new Promise((res) => {
        const i = ++id;
        pend.set(i, res);
        ws.send(JSON.stringify({ id: i, method: m, params: p }));
      });
    ws.onmessage = (ev) => {
      const m = JSON.parse(ev.data);
      if (m.id && pend.has(m.id)) {
        pend.get(m.id)(m.result);
        pend.delete(m.id);
      }
    };
    await new Promise((r) => (ws.onopen = r));
    const ev = async (expr) => {
      const r = await send('Runtime.evaluate', {
        expression: expr,
        returnByValue: true,
        awaitPromise: true,
      });
      return r.result.value;
    };

    const seen = new Set();
    for (let i = 0; i < 12; i += 1) {
      const line = await ev("document.querySelector('.dl-now')?.textContent ?? ''");
      if (line) seen.add(line);
      await sleep(400);
    }
    console.log('歌词行更新序列:', [...seen].join(' → '));
    console.log(
      seen.size >= 2 ? 'A-FIX OK（歌词随播放更新）' : 'A-FIX FAIL（歌词静止）',
    );

    // realm IPC 链诊断：悬浮窗 realm 的自定义命令是否可达
    const diag = await ev(
      "window.__TAURI_INTERNALS__.invoke('player_snapshot').then(s => window.__TAURI_INTERNALS__.invoke('track_id_by_path', { path: s.trackPath }).then(id => window.__TAURI_INTERNALS__.invoke('lyrics_for_track', { trackId: id }).then(l => JSON.stringify({ state: s.state, id, lines: l ? l.lines.map(x => x.text) : null }))))",
    );
    console.log('REALM-CHAIN:', diag);

    ws.close();
    process.exit(0);
  } finally {
    stopApp(child, dataDir, { clean: false });
  }
};
run().catch((e) => {
  console.log('FATAL', e.message);
  setTimeout(() => process.exit(2), 100);
});
