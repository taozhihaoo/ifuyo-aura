// M11 §59 观测器第二段：经 __TAURI_INTERNALS__ 直发真实 updater check
// （走插件 Rust 侧 → 本地 fake server），并读取 UI 状态机结果。
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

async function evalJs(expr) {
  const r = await send('Runtime.evaluate', {
    expression: expr,
    awaitPromise: true,
    returnByValue: true,
  });
  if (r.exceptionDetails) {
    return `THREW: ${r.exceptionDetails.text} ${r.exceptionDetails.exception?.description ?? ''}`;
  }
  return JSON.stringify(r.result.value);
}

console.log('raw-check:', await evalJs(
  "(async () => { try { const r = await window.__TAURI_INTERNALS__.invoke('plugin:updater|check', { options: { timeout: 8000 } }); if (r === null || r === undefined) return 'NULL'; return 'keys=' + Object.keys(r).join(',') + ' version=' + r.version + ' name=' + r.constructor?.name; } catch (e) { return 'ERR: ' + (e?.message ?? String(e)); } })()",
));
console.log('lastCheck-after:', await evalJs("localStorage.getItem('aura.update.lastCheckAt')"));
ws.close();
process.exit(0);
