// M11 §59 Case1 收尾：对已确认 available 的更新对象执行真实
// download + install（官方插件命令；NSIS 静默装 + 应用退出）。
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
    msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
  }
};
await new Promise((r) => (ws.onopen = r));

const expr = `(async () => {
  try {
    const { Channel } = window.__TAURI_INTERNALS__;
    const r = await window.__TAURI_INTERNALS__.invoke('plugin:updater|check', { options: { timeout: 8000 } });
    if (!r) return 'NULL (no update)';
    const events = [];
    const ch = new Channel();
    ch.onmessage = (m) => { events.push(m.event + (m.data?.chunkLength ? '+' + m.data.chunkLength : '')); };
    await window.__TAURI_INTERNALS__.invoke('plugin:updater|download_and_install', { rid: r.rid, onEvent: ch });
    return 'install-flow-finished events=' + events.length;
  } catch (e) {
    return 'ERR: ' + (e?.message ?? String(e));
  }
})()`;
const r = await send('Runtime.evaluate', {
  expression: expr,
  awaitPromise: true,
  returnByValue: true,
  timeout: 60000,
});
console.log('install result:', JSON.stringify(r.result?.value ?? r), r.exceptionDetails ? 'EXC: ' + JSON.stringify(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text) : '');
ws.close();
process.exit(0);
