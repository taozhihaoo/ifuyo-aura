// M12.5 L2 全旅程探针矩阵（Phase 0/2 固化）——CDP 黑盒驱动真实应用。
//
// 覆盖：播放→位置推进→seek→暂停冻结→恢复→音量→停止→重播→曲终(order)。
// 每步输出 `[M-xx] PASS/FAIL/SKIP 描述` 证据行，汇总退出码。
//
// 用法：
//   1) 启动独立构建（须内嵌 dist 的 tauri build 产物，plain cargo build 走 devUrl）：
//      $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9223'
//      Start-Process 'target\release\aura-shell.exe'
//   2) node scripts/verify-playback.mjs [--cdp 9223]
//
// 漂移说明：计划原文为 verify-playback.ps1——CDP 驱动既有工具链是 .mjs
// （cdp-*.mjs 同构），按实际代码沿用 .mjs。

const args = process.argv.slice(2);
const cdpPort = args.includes('--cdp') ? Number(args[args.indexOf('--cdp') + 1]) : 9223;
const CDP = `http://127.0.0.1:${cdpPort}`;

const results = [];
const line = (id, ok, desc) => {
  const verdict = ok === 'skip' ? 'SKIP' : ok ? 'PASS' : 'FAIL';
  results.push(verdict);
  console.log(`[${id}] ${verdict} ${desc}`);
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const run = async () => {
  const targets = await (await fetch(CDP + '/json')).json();
  const page = targets.find((t) => t.type === 'page');
  if (!page) throw new Error('无 CDP page——应用未启动或调试端口不对');
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let id = 0;
  const pending = new Map();
  const send = (m, p) =>
    new Promise((res) => {
      const i = ++id;
      pending.set(i, res);
      ws.send(JSON.stringify({ id: i, method: m, params: p }));
    });
  ws.onmessage = (ev) => {
    const m = JSON.parse(ev.data);
    if (m.id && pending.has(m.id)) {
      pending.get(m.id)(m.result);
      pending.delete(m.id);
    }
  };
  await new Promise((r) => (ws.onopen = r));
  const inv = async (c, a) => {
    const r = await send('Runtime.evaluate', {
      expression: `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(c)}, ${JSON.stringify(a ?? {})})`,
      awaitPromise: true,
      returnByValue: true,
    });
    if (r.exceptionDetails) throw new Error(`${c}: ${r.exceptionDetails.text ?? 'threw'}`);
    return r.result.value;
  };

  // C01 库就绪
  let tracks = [];
  try {
    tracks = await inv('library_list', { sort: 'title', limit: 50, offset: 0 });
    line('C01', Array.isArray(tracks) && tracks.length > 0, `音乐库可用（${tracks.length} 首）`);
  } catch (e) {
    line('C01', false, `音乐库不可用：${e.message}`);
  }
  if (!tracks.length) {
    ['P01','P02','P03','P04','P05','P06','P07','P08','P09'].forEach((id) => line(id, 'skip', '无曲目'));
  } else {
    const track = tracks[0];
    const dur = track.durationSeconds;
    const snap = async () => inv('player_snapshot', {});
    const pos = async () => (await snap()).positionSeconds;

    // P01 播放起态
    await inv('play_track', { trackId: track.id });
    await sleep(1500);
    let s = await snap();
    line('P01', s.state === 'playing', `play_track → playing（pos=${s.positionSeconds.toFixed(2)}）`);

    // P02 位置推进
    const a1 = await pos();
    await sleep(1000);
    const a2 = await pos();
    line('P02', a2 - a1 >= 0.5, `位置推进 Δ=${(a2 - a1).toFixed(2)}s/1s`);

    // P03 seek 中点
    const mid = Math.floor(dur / 3);
    await inv('seek', { positionSeconds: mid });
    await sleep(700);
    s = await snap();
    line('P03', Math.abs(s.positionSeconds - mid) < 0.8, `seek(${mid}s) → ${s.positionSeconds.toFixed(2)}s`);

    // P04 暂停冻结
    await inv('pause', {});
    await sleep(1400);
    const b1 = await pos();
    await sleep(1000);
    const b2 = await pos();
    line('P04', Math.abs(b2 - b1) < 0.05 && (await snap()).state === 'paused', `暂停冻结 drift=${(b2 - b1).toFixed(3)}s`);

    // P05 恢复
    await inv('play', {});
    await sleep(1200);
    const c1 = await pos();
    await sleep(1000);
    const c2 = await pos();
    line('P05', c2 - c1 >= 0.5, `恢复推进 Δ=${(c2 - c1).toFixed(2)}s/1s`);

    // P06 音量
    await inv('set_volume', { volume: 0.5 });
    await sleep(300);
    s = await snap();
    line('P06', Math.abs(s.volume - 0.5) < 0.01, `set_volume(0.5) → snapshot.volume=${s.volume}`);
    await inv('set_volume', { volume: 1 });

    // P07 停止归零
    await inv('stop', {});
    await sleep(1600);
    s = await snap();
    line('P07', s.state === 'stopped' && s.positionSeconds < 0.05, `stop → stopped pos=${s.positionSeconds.toFixed(2)}`);

    // P08 重播从头
    await inv('play', {});
    await sleep(1200);
    s = await snap();
    line('P08', s.state === 'playing' && s.positionSeconds < 1.6, `重播 → playing pos=${s.positionSeconds.toFixed(2)}`);

    // P09 曲终 order 模式（seek 到末尾前 1.5s，等待自然播完 → stopped 归零）
    await inv('seek', { positionSeconds: dur - 1.5 });
    await sleep(4500);
    s = await snap();
    line('P09', s.state === 'stopped' && s.positionSeconds < 0.05, `曲终(order) → stopped pos=${s.positionSeconds.toFixed(2)}`);
  }

  ws.close();
  const pass = results.filter((r) => r === 'PASS').length;
  const fail = results.filter((r) => r === 'FAIL').length;
  console.log(`MATRIX: ${pass}/${results.length} PASS, ${fail} FAIL`);
  process.exit(fail > 0 ? 1 : 0);
};
run().catch((e) => {
  console.log('FATAL', e.message);
  process.exit(2);
});
