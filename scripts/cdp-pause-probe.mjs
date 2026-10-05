// M-诊断：暂停语义验证——play → 采样 2 次（应前进）→ pause →
// 采样 4 次（必须停住 §183/§61）。
const CDP = 'http://127.0.0.1:9223';
const targets = await (await fetch(`${CDP}/json`)).json();
const page = targets.find((t) => t.type === 'page');
if (!page) process.exit(1);
const ws = new WebSocket(page.webSocketDebuggerUrl);
let id = 0;
const pending = new Map();
function send(method, params) {
  return new Promise((resolve, reject) => {
    const mid = ++id;
    pending.set(mid, { resolve, reject });
    ws.send(JSON.stringify({ id: mid, method, params }));
  });
}
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    const { resolve, reject } = pending.get(msg.id);
    pending.delete(msg.id);
    msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
  }
};
await new Promise((r) => (ws.onopen = r));

async function invoke(cmd, args) {
  const r = await send('Runtime.evaluate', {
    expression: `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args ?? {})})`,
    awaitPromise: true,
    returnByValue: true,
  });
  return r.exceptionDetails
    ? { threw: r.exceptionDetails.exception?.description ?? r.exceptionDetails.text }
    : r.result.value;
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const tracks = await invoke('library_list', { sort: 'title', limit: 1, offset: 0 });
await invoke('play_track', { trackId: tracks[0].id });
await sleep(1200);
const s1 = await invoke('player_snapshot');
console.log(`PLAYING state=${s1.state} pos=${s1.positionSeconds.toFixed(2)}`);
await sleep(800);
const s2 = await invoke('player_snapshot');
console.log(`PLAYING state=${s2.state} pos=${s2.positionSeconds.toFixed(2)} (Δ=${(s2.positionSeconds - s1.positionSeconds).toFixed(2)})`);

await invoke('pause');
await sleep(300);
const p0 = (await invoke('player_snapshot')).positionSeconds;
for (let i = 1; i <= 4; i++) {
  await sleep(700);
  const s = await invoke('player_snapshot');
  const drift = (s.positionSeconds - p0).toFixed(3);
  console.log(`PAUSED +${i * 0.7}s state=${s.state} pos=${s.positionSeconds.toFixed(2)} drift=${drift} ${Math.abs(s.positionSeconds - p0) < 0.05 ? 'OK' : 'FAIL'}`);
}
ws.close();
process.exit(0);
