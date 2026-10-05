// M11 §59 观测器：经 WebView2 远程调试协议连入已安装应用，
// 读取运行时状态（版本注入/localStorage/更新状态），并可手动触发
// 一次真实的 plugin:updater|check 调用（HTTP 打到本地 fake server）。
// 前置：以 WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 启动应用。
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

const messages = [];
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    const { resolve, reject } = pending.get(msg.id);
    pending.delete(msg.id);
    if (msg.error) reject(new Error(msg.error.message));
    else resolve(msg.result);
  } else if (msg.method === 'Runtime.consoleAPICalled') {
    const args = (msg.params.args ?? []).map((a) => a.value ?? a.description ?? '');
    messages.push(`[console.${msg.params.type}] ${args.join(' ')}`);
  } else if (msg.method === 'Runtime.exceptionThrown') {
    const d = msg.params.exceptionDetails;
    messages.push(`[exception] ${d.text} ${d.exception?.description ?? ''}`);
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

console.log('href:', await evalJs('window.location.href'));
console.log('version:', await evalJs('globalThis.__AURA_VERSION__'));
console.log('lastCheck:', await evalJs("localStorage.getItem('aura.update.lastCheckAt')"));
console.log('settings.autoCheck:', await evalJs(
  "window.__TAURI_INTERNALS__ ? 'internals-ok' : 'no-internals'",
));

console.log('--- manual plugin check（真实 HTTP 到本地 fake server）---');
console.log(
  await evalJs(
    "(async () => { try { const { check } = await import('@tauri-apps/plugin-updater'); const u = await check({ timeout: 8000 }); return u ? `available ${u.version}` : 'up-to-date/null'; } catch (e) { return 'CHECK-ERR: ' + (e?.message ?? String(e)); } })()",
  ),
);

await new Promise((r) => setTimeout(r, 500));
console.log('--- console/exceptions during session ---');
for (const m of messages.slice(-20)) console.log(m);

ws.close();
process.exit(0);
