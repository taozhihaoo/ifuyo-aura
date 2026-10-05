// M11 §59/§384：CDP 真实 GUI 自动化——键盘/鼠标事件经 WebView2 输入域
// 注入（用户路径），驱动 Settings → 检查更新 → 下载并安装 全回环。
import { spawn, execSync } from 'node:child_process';
import path from 'node:path';

const CDP = 'http://127.0.0.1:9223';
const EXE = path.join(process.env.LOCALAPPDATA, 'ifuyo Aura', 'aura-shell.exe');

function launchApp() {
  try {
    execSync('taskkill /IM aura-shell.exe /F', { stdio: 'ignore', shell: 'cmd.exe' });
  } catch {
    /* 无残留进程 */
  }
  const child = spawn(EXE, [], {
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9223',
    },
    stdio: 'ignore',
    detached: false,
  });
  return child;
}

let app = launchApp();
await new Promise((r) => setTimeout(r, 7000));

let targets = [];
for (let i = 0; i < 10; i++) {
  try {
    targets = await (await fetch(`${CDP}/json`)).json();
    break;
  } catch {
    await new Promise((r) => setTimeout(r, 1000));
  }
}
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

async function evalJs(expr) {
  const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
  return r.exceptionDetails ? `THREW:${r.exceptionDetails.text}` : JSON.stringify(r.result.value);
}
async function key(code, keyName, down = true, modifiers = 0) {
  await send('Input.dispatchKeyEvent', { type: down ? 'keyDown' : 'keyUp', windowsVirtualKeyCode: code, code: keyName, key: keyName, modifiers });
}
async function text(t) {
  await send('Input.insertText', { text: t });
}
async function click(x, y) {
  await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
  await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
}
async function shot(name) {
  const r = await send('Page.captureScreenshot', { format: 'png' });
  const { writeFileSync } = await import('node:fs');
  const { fileURLToPath } = await import('node:url');
  const out = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'target', `${name}.png`);
  writeFileSync(out, Buffer.from(r.data, 'base64'));
}

// 打开 Settings：Esc 复位 → Ctrl+K → 输入 'eq' → Enter。
;
await key(17, 'ControlLeft', true, 2);
await key(75, 'KeyK', true, 2);
await key(75, 'KeyK', false, 2);
await key(17, 'ControlLeft', false, 2);
await new Promise((r) => setTimeout(r, 700));
await text('eq');
await new Promise((r) => setTimeout(r, 600));
await key(13, 'Enter', true);
await key(13, 'Enter', false);
await new Promise((r) => setTimeout(r, 1000));
await shot('m11-settings-open');
console.log('settings opened');

// 定位「检查更新」按钮（真实 DOM 查询 + 中心坐标点击）。
const rect = JSON.parse(await evalJs(
  "(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent.includes('检查更新')); if (!b) return 'null'; const r = b.getBoundingClientRect(); return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 }); })()",
));
console.log('check-button:', rect);
if (rect !== 'null') {
  await click(rect.x, rect.y);
  await new Promise((r) => setTimeout(r, 2500));
  await shot('m11-update-checked');
  console.log('check clicked');

  // 状态文本回读（真实 DOM）。
  console.log('status:', await evalJs(
    "(() => { const el = document.querySelector('.settings-update-status'); return el ? el.textContent : 'no-status'; })()",
  ));

  // 若出现「下载并安装」→ 点击（真实安装回环）。
  const inst = JSON.parse(await evalJs(
    "(() => { const b = [...document.querySelectorAll('button')].find((x) => x.textContent.includes('下载并安装')); if (!b) return 'null'; const r = b.getBoundingClientRect(); return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 }); })()",
  ));
  console.log('install-button:', inst);
  if (inst !== 'null') {
    await click(inst.x, inst.y);
    await new Promise((r) => setTimeout(r, 8000));
    await shot('m11-update-installed');
    console.log('install clicked (app may exit for relaunch)');
  }
}

ws.close();
process.exit(0);
