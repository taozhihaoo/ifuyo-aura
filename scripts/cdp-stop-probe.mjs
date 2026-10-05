// M-诊断：完整错误回显版 stop→play 探针。
const CDP = 'http://127.0.0.1:9223';
const run = async () => {
  const targets = await (await fetch(CDP + '/json')).json();
  console.log('targets:', targets.filter((t) => t.type === 'page').map((t) => t.url).join(' | '));
  const page = targets.find((t) => t.type === 'page');
  if (!page) process.exit(1);
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let id = 0;
  const pending = new Map();
  const send = (m, p) => new Promise((res) => { const i = ++id; pending.set(i, res); ws.send(JSON.stringify({ id: i, method: m, params: p })); });
  ws.onmessage = (ev) => { const m = JSON.parse(ev.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m.result); pending.delete(m.id); } };
  await new Promise((r) => (ws.onopen = r));
  const evalJs = async (expr) => {
    const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
    if (r.exceptionDetails) {
      return { threw: r.exceptionDetails.exception?.description ?? r.exceptionDetails.text };
    }
    return r.result.value;
  };
  const inv = async (c, a) =>
    evalJs(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(c)}, ${JSON.stringify(a ?? {})})`);
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  console.log('internals:', await evalJs("typeof window.__TAURI_INTERNALS__"));
  const p1 = await inv('play_track', { trackId: 1 });
  console.log('play_track:', JSON.stringify(p1));
  await sleep(3000);
  const mid = await inv('player_snapshot');
  console.log('MID:', JSON.stringify(mid));
  const st = await inv('stop');
  console.log('stop:', JSON.stringify(st));
  await sleep(500);
  await inv('play');
  await sleep(1500);
  console.log('REPLAY:', JSON.stringify(await inv('player_snapshot')));
  ws.close();
  process.exit(0);
};
run().catch((e) => { console.log('FATAL', e.message); process.exit(1); });
