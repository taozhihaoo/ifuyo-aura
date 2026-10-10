// M15b 诊断：桌面歌词开关往返回归（等 UI 就绪后执行）。
// 用法：node scripts/verify-desktop-lyrics.mjs [--cdp 9223]
const CDP = `http://127.0.0.1:${process.argv.includes('--cdp') ? Number(process.argv[process.argv.indexOf('--cdp') + 1]) : 9223}`;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const attach = async (isMain) => {
  const targets = await (await fetch(CDP + '/json')).json();
  const page = targets.find(
    (t) => t.type === 'page' && (isMain ? !t.url.includes('view=lyrics') : t.url.includes('view=lyrics')),
  );
  if (!page) throw new Error(`no target isMain=${isMain}`);
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let id = 0;
  const pend = new Map();
  const send = (m, p) =>
    new Promise((res) => {
      const i = ++id;
      pend.set(i, res);
      ws.send(JSON.stringify({ id: i, method: m, params: p }));
    });
  ws.onmessage = (ev) => {
    const m = JSON.parse(ev.data);
    if (m.id && pend.has(m.id)) {
      pend.get(m.id)(m.result);
      pend.delete(m.id);
    }
  };
  await new Promise((r) => (ws.onopen = r));
  return {
    ws,
    ev: async (expr) => {
      const r = await send('Runtime.evaluate', {
        expression: expr,
        returnByValue: true,
        awaitPromise: true,
      });
      return r.result.value;
    },
  };
};

const run = async () => {
  // 等 UI 就绪：.pb-lyrics 出现（新版启动含远程清单初始化，就绪较慢）
  let main = null;
  let ready = false;
  for (let i = 0; i < 40; i += 1) {
    try {
      main = await attach(true);
      // PlayerBar 在非正在播放视图渲染（设计）——先切曲库视图
      await main.ev(
        "[...document.querySelectorAll('.nav-btn,.side-btn')].find((b) => b.textContent.includes('音乐库') || b.textContent.includes('Library'))?.click();1",
      );
      await sleep(600);
      const ok = await main.ev("!!document.querySelector('.pb-lyrics')");
      if (ok) {
        ready = true;
        break;
      }
      main.ws.close();
    } catch {
      /* 未就绪 */
    }
    await sleep(500);
  }
  if (!ready) {
    console.log('FATAL .pb-lyrics 未就绪');
    process.exit(2);
  }
  console.log('READY');

  const vis = () =>
    main.ev(
      "window.__TAURI_INTERNALS__.invoke('plugin:window|is_visible', { label: 'desktop-lyrics' })",
    );
  // 视图切换会瞬时重挂载（M15b 远程清单加载后自动切视图）——重试点击
  const clickLyrics = async () => {
    for (let i = 0; i < 30; i += 1) {
      const r = await main.ev("document.querySelector('.pb-lyrics')?.click() ?? 'no-btn'");
      if (r !== 'no-btn') return;
      await sleep(500);
    }
    throw new Error('按钮消失（重试耗尽）');
  };

  await clickLyrics();
  await sleep(2500);
  console.log('1.OPEN visible:', await vis());

  await clickLyrics();
  await sleep(600);
  console.log('2.LOCK visible(应仍 true):', await vis());

  await clickLyrics();
  await sleep(600);
  console.log('3.UNLOCK visible:', await vis());

  await clickLyrics();
  await sleep(600);
  console.log('4.CLOSE visible(应 false):', await vis());

  await clickLyrics();
  await sleep(1500);
  console.log('5.REOPEN visible(应 true):', await vis());

  main.ws.close();
  setTimeout(() => process.exit(0), 100);
};
run().catch((e) => {
  console.log('FATAL', e.message);
  setTimeout(() => process.exit(2), 100);
});
