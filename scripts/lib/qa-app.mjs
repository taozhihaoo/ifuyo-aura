// M12.5 QA 共用：以隔离资料目录启动真实应用 + CDP 连接。
//
// AURA_DATA_DIR 指向临时目录（lib.rs 钩子），扫描/设置改动零用户污染；
// WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS 开调试端口。用完 kill 进程。
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

export const APP_EXE = process.env.AURA_QA_EXE ?? 'target/release/aura-shell.exe';

export async function startApp({ cdpPort = 9223, dataDir } = {}) {
  const dir = dataDir ?? mkdtempSync(join(tmpdir(), 'aura-qa-'));
  const child = spawn(APP_EXE, [], {
    cwd: process.cwd(),
    detached: false,
    stdio: ['ignore', 'ignore', 'pipe'],
    env: {
      ...process.env,
      AURA_DATA_DIR: dir,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${cdpPort}`,
    },
  });
  child.stderr.on('data', () => {}); // 探针日志不刷屏；需要时手动跑可见
  // 等 CDP 就绪（最多 30s）
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const t = await (await fetch(`http://127.0.0.1:${cdpPort}/json`)).json();
      if (t.some((x) => x.type === 'page')) break;
    } catch {
      /* 未就绪 */
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  return { child, dataDir: dir };
}

export function stopApp(child, dataDir, { clean = true } = {}) {
  try {
    child.kill();
  } catch {
    /* 已退出 */
  }
  // webview 子进程可能持有目录；由调用方决定是否清理（失败不致命）
  if (clean && dataDir) {
    setTimeout(() => {
      try {
        rmSync(dataDir, { recursive: true, force: true });
      } catch {
        /* WebView2 释放延迟 */
      }
    }, 1500);
  }
}

/** CDP 会话：Runtime.evaluate + Tauri invoke 封装。
 *  启动竞态下可能存在 about:blank 过渡页（origin=null → invoke 报
 *  "Origin header is not a valid URL"）——优先选应用页面并探测可用性。 */
export async function connectCdp(cdpPort = 9223) {
  const deadline = Date.now() + 20_000;
  let lastErr = null;
  while (Date.now() < deadline) {
    try {
      const targets = await (await fetch(`http://127.0.0.1:${cdpPort}/json`)).json();
      const pages = targets.filter((t) => t.type === 'page');
      const ordered = [
        ...pages.filter((p) => /tauri\.localhost|localhost:\d+/.test(p.url)),
        ...pages,
      ];
      for (const page of ordered) {
        try {
          const sess = await attach(page);
          await sess.inv('app_info', {});
          return sess;
        } catch (e) {
          lastErr = e;
        }
      }
    } catch (e) {
      lastErr = e;
    }
    await new Promise((r) => setTimeout(r, 600));
  }
  throw new Error(`CDP attach 失败：${lastErr?.message ?? '超时'}`);
}

async function attach(page) {
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
    if (r.exceptionDetails) {
      const d = r.exceptionDetails;
      const msg =
        d.exception?.description ?? d.exception?.value ?? d.text ?? JSON.stringify(d);
      throw new Error(`${c}: ${msg}`);
    }
    return r.result.value;
  };
  return { ws, inv, evalJs: (expr) => send('Runtime.evaluate', { expression: expr, returnByValue: true }) };
}
