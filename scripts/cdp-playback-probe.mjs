// M-诊断：复现"播放中但位置不走"——play_track → 每 500ms 采样
// player_snapshot（state/position/duration），共 6 次；同时读
// integration diagnostics。
const CDP = 'http://127.0.0.1:9223';
const targets = await (await fetch(`${CDP}/json`)).json();
const page = targets.find((t) => t.type === 'page');
if (!page) {
  console.error('no page target');
  process.exit(1);
}
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
  if (r.exceptionDetails) {
    return { threw: r.exceptionDetails.exception?.description ?? r.exceptionDetails.text };
  }
  return r.result.value;
}

const tracks = await invoke('library_list', { sort: 'title', limit: 5, offset: 0 });
console.log('library:', JSON.stringify(tracks)?.slice(0, 300));
const first = Array.isArray(tracks) ? tracks[0] : null;
if (!first) {
  console.log('no track in library');
  process.exit(0);
}
console.log('playing track id=', first.id, first.title);

const playErr = await invoke('play_track', { trackId: first.id });
console.log('play_track:', JSON.stringify(playErr));

for (let i = 0; i < 6; i++) {
  await new Promise((r) => setTimeout(r, 500));
  const snap = await invoke('player_snapshot');
  console.log(`t+${(i + 1) * 0.5}s state=${snap?.state} pos=${snap?.positionSeconds?.toFixed(2)} dur=${snap?.durationSeconds} seekable=${snap?.seekable} rev=${snap?.revision}`);
}

console.log('diagnostics:', JSON.stringify(await invoke('get_integration_diagnostics')));
ws.close();
process.exit(0);
