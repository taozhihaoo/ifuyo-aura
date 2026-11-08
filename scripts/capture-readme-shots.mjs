#!/usr/bin/env node
// 生成 README 用的英/中双语气泡截图。
//
// 做法：用真实构建产物（ui/dist）在无头 Edge 里渲染真实 React UI，
// 通过 scripts/lib/readme-mock.js 注入一份**虚构的演示数据集**（Tauri IPC
// 仿真），不启动桌面应用、不读写用户库、不联网。截图仅用于展示界面。
//
// 用法：node scripts/capture-readme-shots.mjs
// 前置：cd ui && npm run build（本脚本会检查 ui/dist 是否存在）
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { dirname, extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const distDir = join(root, 'ui', 'dist');
const outRoot = join(root, 'docs', 'screenshots');
const MOCK_PATH = join(root, 'scripts', 'lib', 'readme-mock.js');

const EDGE_CANDIDATES = [
  process.env.MSEDGE_PATH,
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
].filter(Boolean);

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.woff2': 'font/woff2',
};

function fail(msg) {
  console.error(`✗ ${msg}`);
  process.exit(1);
}

if (!existsSync(join(distDir, 'index.html'))) {
  fail('缺少 ui/dist/index.html —— 先运行 cd ui && npm run build');
}
const edge = EDGE_CANDIDATES.find((p) => existsSync(p));
if (!edge) fail('找不到 Microsoft Edge（可用 MSEDGE_PATH 指定）');

// ---- 静态服务器：/ 返回注入 mock 的 index.html ----
const indexHtml = readFileSync(join(distDir, 'index.html'), 'utf8');
const mockJs = readFileSync(MOCK_PATH, 'utf8');
const injected = indexHtml.replace(
  '</head>',
  '  <script src="/__mock.js"></script>\n  </head>',
);

const server = createServer((req, res) => {
  const url = new URL(req.url ?? '/', 'http://127.0.0.1');
  if (url.pathname === '/' || url.pathname === '/index.html') {
    res.writeHead(200, { 'Content-Type': MIME['.html'] });
    res.end(injected);
    return;
  }
  if (url.pathname === '/__mock.js') {
    res.writeHead(200, { 'Content-Type': MIME['.js'] });
    res.end(mockJs);
    return;
  }
  const file = join(distDir, decodeURIComponent(url.pathname));
  if (!file.startsWith(distDir) || !existsSync(file)) {
    res.writeHead(404).end('not found');
    return;
  }
  res.writeHead(200, { 'Content-Type': MIME[extname(file)] ?? 'application/octet-stream' });
  res.end(readFileSync(file));
});

await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;
const origin = `http://127.0.0.1:${port}`;

// ---- 无头 Edge + CDP ----
const cdpPort = 9411;
const profile = join(tmpdir(), `aura-readme-${process.pid}`);
const browser = spawn(
  edge,
  [
    '--headless=new',
    '--disable-gpu',
    '--hide-scrollbars',
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-extensions',
    '--force-color-profile=srgb',
    `--user-data-dir=${profile}`,
    `--remote-debugging-port=${cdpPort}`,
    'about:blank',
  ],
  { stdio: 'ignore' },
);

async function cdpTarget() {
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${cdpPort}/json`)).json();
      const page = list.find((t) => t.type === 'page');
      if (page) return page;
    } catch {
      /* 未就绪 */
    }
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error('CDP 未就绪');
}

let ws;
let msgId = 0;
const pending = new Map();

function send(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++msgId;
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
}

async function evaluate(expression) {
  const r = await send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (r.exceptionDetails) {
    throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
  }
  return r.result.value;
}

async function waitFor(expression, timeoutMs = 12_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await evaluate(`Boolean(${expression})`)) return true;
    await new Promise((r) => setTimeout(r, 120));
  }
  return false;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const page = await cdpTarget();
ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (!msg.id) return;
  const entry = pending.get(msg.id);
  if (!entry) return;
  pending.delete(msg.id);
  if (msg.error) entry.reject(new Error(JSON.stringify(msg.error)));
  else entry.resolve(msg.result);
};

await send('Page.enable');
await send('Runtime.enable');
await send('Page.addScriptToEvaluateOnNewDocument', {
  source:
    "window.__AURA_ERRORS__=[];" +
    "addEventListener('error',e=>window.__AURA_ERRORS__.push(String(e.message||e.error)));" +
    "addEventListener('unhandledrejection',e=>window.__AURA_ERRORS__.push(String(e.reason)));",
});
await send('Emulation.setDeviceMetricsOverride', {
  width: 1180,
  height: 760,
  deviceScaleFactor: 1.5,
  mobile: false,
});

/** 每张图的动作脚本。返回前必须让目标元素真实出现在 DOM 里。 */
const SHOTS = [
  {
    file: '01-now-playing',
    async prepare() {
      await waitFor("document.querySelector('.np-artwork')");
      await waitFor("document.querySelector('.np-lyrics .np-lyric-line.active')");
    },
  },
  {
    file: '02-library',
    async prepare() {
      await evaluate("document.querySelectorAll('.titlebar-nav .nav-btn')[1].click()");
      await waitFor("document.querySelectorAll('.track-row').length > 6");
    },
  },
  {
    file: '03-command-palette',
    async prepare() {
      await evaluate("document.querySelectorAll('.titlebar-actions .icon-btn')[0].click()");
      await waitFor("document.querySelector('.palette')");
    },
  },
  {
    file: '04-queue',
    async prepare() {
      await evaluate("document.querySelectorAll('.titlebar-nav .nav-btn')[1].click()");
      await waitFor("document.querySelectorAll('.track-row').length > 6");
      await evaluate(`(() => {
        const row = document.querySelectorAll('.track-row')[2];
        row.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
        return true;
      })()`);
      await sleep(350);
      await evaluate("document.querySelectorAll('.sidebar .side-btn')[2].click()");
      await waitFor("document.querySelectorAll('.queue-item').length > 6");
    },
  },
];

const ALL_LANGS = [
  { id: 'en', label: 'English' },
  { id: 'zh-CN', label: '简体中文' },
];
const only = (process.env.SHOT_LANGS ?? '').split(',').filter(Boolean);
const LANGS = only.length > 0 ? ALL_LANGS.filter((l) => only.includes(l.id)) : ALL_LANGS;

let count = 0;
for (const lang of LANGS) {
  const dir = join(outRoot, lang.id);
  mkdirSync(dir, { recursive: true });
  for (const shot of SHOTS) {
    await send('Page.navigate', { url: `${origin}/?lang=${encodeURIComponent(lang.id)}` });
    if (!(await waitFor("document.querySelector('.app-root')", 15_000))) {
      const diag = await evaluate(
        "({ href: location.href, ready: document.readyState, errors: window.__AURA_ERRORS__, mock: Boolean(window.__TAURI_INTERNALS__), body: document.body?.innerHTML.slice(0, 200) })",
      ).catch((e) => ({ error: String(e) }));
      throw new Error(`${lang.id}/${shot.file}: 应用未渲染 ${JSON.stringify(diag)}`);
    }
    await sleep(500);
    await shot.prepare();
    await sleep(650); // 等入场动画与封面加载稳定
    const png = await send('Page.captureScreenshot', { format: 'png', fromSurface: true });
    const out = join(dir, `${shot.file}.png`);
    writeFileSync(out, Buffer.from(png.data, 'base64'));
    count += 1;
    console.log(`✓ ${lang.label}  docs/screenshots/${lang.id}/${shot.file}.png`);
  }
}

console.log(`\n完成：${count} 张截图 → docs/screenshots/{en,zh-CN}/`);

ws.close();
server.close();
browser.kill();
// WebView 释放 profile 有延迟；失败不影响截图。
setTimeout(() => {
  try {
    rmSync(profile, { recursive: true, force: true });
  } catch {
    /* ignore */
  }
}, 1200);
