// L2 多窗交互验证（W 系列）——Mini 窗与桌面歌词窗都是独立 webview（独立 JS
// realm），主窗与它们之间只靠事件桥，所以"桥断掉"在单窗测试里完全看不出来：
// Mini 的 ⏮/⏭ 就曾因为 emit 事件名写错（integration-command vs integration://command）
// 而静默失效，静态门禁全绿。本脚本把这条跨窗链路做成可重跑断言。
//
// 运行：node scripts/verify-windows.mjs   （自动启停，隔离资料目录）
import { mkdtempSync, readdirSync, rmSync, cpSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { connectCdp, connectCdpByUrl, startApp, stopApp, APP_EXE } from './lib/qa-app.mjs';

const results = [];
const line = (id, ok, desc) => {
  const verdict = ok === 'skip' ? 'SKIP' : ok ? 'PASS' : 'FAIL';
  results.push(verdict);
  console.log(`[${id}] ${verdict} ${desc}`);
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const click = (sess, selector, index = 0) =>
  sess.evalJs(
    `(()=>{const n=document.querySelectorAll('${selector}')[${index}];
       if(!n) return false; n.click(); return true})()`,
  ).then((r) => r.result?.value === true);

const dblclick = (sess, selector, index = 0) =>
  sess.evalJs(
    `(()=>{const n=document.querySelectorAll('${selector}')[${index}];
       if(!n) return false; n.dispatchEvent(new MouseEvent('dblclick',{bubbles:true})); return true})()`,
  ).then((r) => r.result?.value === true);

const text = (sess, selector) =>
  sess.evalJs(`document.querySelector('${selector}')?.textContent ?? null`).then(
    (r) => r.result?.value ?? null,
  );

// PlayerSnapshot 没有 trackId —— 曲目身份字段是 trackPath（两个 fixture
// 共用标题"Fixture曲目"，只比 title 会把换曲判成没换）。
const snap = async (sess) => {
  const s = await sess.inv('player_snapshot', {});
  return {
    path: s?.trackPath ?? null,
    title: s?.title ?? null,
    state: s?.state ?? null,
    pos: s?.positionSeconds ?? null,
  };
};
const base = (p) => (p ?? '').split(/[\/]/).pop();

const run = async () => {
  console.log(`QA exe: ${APP_EXE}`);
  const fixtures = join(process.cwd(), 'tests', 'fixtures');
  const musicDir = mkdtempSync(join(tmpdir(), 'aura-win-'));
  // 只放短小、可辨识的曲目，保证队列里有多条且标题互不相同
  const pick = ['fixture-id3v2.3.mp3', 'fixture-id3v2.4.mp3', 'fixture-alac.m4a', 'fixture-gbk.mp3'];
  for (const f of pick) cpSync(join(fixtures, f), join(musicDir, f));
  const all = readdirSync(fixtures).filter((f) => /\.(mp3|flac|ogg|m4a|aiff|lrc)$/i.test(f));
  for (const f of all) if (!pick.includes(f)) cpSync(join(fixtures, f), join(musicDir, f));

  const { child, dataDir } = await startApp();
  let main = null;
  try {
    main = await connectCdp();
    const { inv, evalJs } = main;

    await inv('scan_folder', { path: musicDir });
    const end = Date.now() + 40_000;
    let tracks = [];
    while (Date.now() < end) {
      await sleep(700);
      tracks = await inv('library_list', { sort: 'title', limit: 50, offset: 0 });
      if (tracks.length >= 4) break;
    }
    line('W00', tracks.length >= 4, `扫描 ${all.length} 个文件 → ${tracks.length} 首`);
    if (tracks.length < 4) return;

    const lrcId = tracks.find((t) => (t.path ?? '').endsWith('fixture-lrc.mp3'))?.id ?? null;
    // 用真实 UI 路径建队列（点曲库行 = playWithQueueContext + playTrack）
    await evalJs(`(()=>{const b=[...document.querySelectorAll('.nav-btn,.side-btn')]
        .find(x=>/音乐库|Library/.test(x.textContent)); b&&b.click(); return !!b})()`);
    await sleep(900);
    const rowOk = await dblclick(main, '.track-row', 0);
    await sleep(1200);
    const first = await snap(main);
    line('W01', rowOk && first.path !== null && first.state === 'playing',
      `双击曲库首行起播：${base(first.path)} state=${first.state}`);

    // ---- 打开 Mini 窗（Ctrl+M，与键盘图 D-078 同一路径） ----
    await evalJs(`document.body.dispatchEvent(new KeyboardEvent('keydown',
        {key:'m', ctrlKey:true, bubbles:true})); 1`);
    const mini = await connectCdpByUrl('view=mini');
    line('W02', !!mini, mini ? 'Mini 窗已开并可连接' : 'Mini 窗目标未出现（Ctrl+M 未开窗？）');
    if (!mini) return;

    // ---- Mini 窗反映主窗播放状态 ----
    await sleep(800);
    const miniTitle = await text(mini, '.mini-title');
    line('W03', miniTitle === (first.title ?? '') && miniTitle !== null,
      `Mini 标题与主窗一致：mini="${miniTitle}" main="${first.title}"`);

    // ---- 跨窗事件桥：Mini ⏭ → 主窗队列前进 ----
    const nextOk = await click(mini, '.mini-btn', 2);
    await sleep(1500);
    const afterNext = await snap(main);
    line('W04', nextOk && afterNext.path !== null && afterNext.path !== first.path,
      `Mini ⏭ 换曲：${base(first.path)} → ${base(afterNext.path)}`);

    // ---- Mini ⏮ 回到上一曲 ----
    const prevOk = await click(mini, '.mini-btn', 0);
    await sleep(1500);
    const afterPrev = await snap(main);
    line('W05', prevOk && afterPrev.path === first.path,
      `Mini ⏮ 回到上一曲：${base(afterPrev.path)}，期望 ${base(first.path)}`);

    // ---- Mini 播放/暂停键直接驱动引擎 ----
    const before = (await snap(main)).state;
    const playOk = await click(mini, '.mini-btn', 1);
    // 暂停含 1s 传输淡变：淡出期间引擎刻意保持 Playing（驱动淡完才翻状态），
    // 所以必须轮询而不是固定 sleep —— 第一版 900ms 等待把可用功能误判成 FAIL。
    let st = before;
    for (let i = 0; i < 14 && st === before; i += 1) {
      await sleep(250);
      st = (await snap(main)).state;
    }
    line('W06', playOk && st !== before && ['playing', 'paused'].includes(st),
      `Mini ▶/⏸ 翻转状态：${before} → ${st}（淡变后）`);

    // ---- 桌面歌词窗：开 → 锁定切换 → 关 ----
    // 先切到带 LRC 的曲目并保持播放（歌词窗要看得到词）
    const lrcRow = await main.evalJs(
      `(()=>{const rows=[...document.querySelectorAll('.track-row')];
         const i=rows.findIndex(r=>(r.dataset.trackId??'')===String(${lrcId ?? 'null'}));
         if(i<0) return -1; rows[i].dispatchEvent(new MouseEvent('dblclick',{bubbles:true})); return i})()`,
    ).then((r) => r.result?.value);
    await sleep(1200);
    if (lrcRow >= 0) {
      await main.inv('seek', { positionSeconds: 2.6 });
      await sleep(400);
    }
    await click(main, '.pb-lyrics');
    const dl = await connectCdpByUrl('view=lyrics');
    line('W07', !!dl, dl ? '桌面歌词窗已开并可连接' : '歌词窗目标未出现（播放条「词」未开窗？）');
    if (!dl) return;

    const locked0 = await dl
      .evalJs(`document.querySelector('.desktop-lyrics')?.dataset.locked ?? null`)
      .then((r) => r.result?.value);
    await click(dl, '.dl-pill');
    await sleep(600);
    const locked1 = await dl
      .evalJs(`document.querySelector('.desktop-lyrics')?.dataset.locked ?? null`)
      .then((r) => r.result?.value);
    await click(dl, '.dl-pill');
    await sleep(600);
    const locked2 = await dl
      .evalJs(`document.querySelector('.desktop-lyrics')?.dataset.locked ?? null`)
      .then((r) => r.result?.value);
    line('W08', locked0 === 'false' && locked1 === 'true' && locked2 === 'false',
      `歌词窗锁定可切换：${locked0} → ${locked1} → ${locked2}`);

    // 锁定态下解锁按钮必须仍可点：本应用不使用 setIgnoreCursorEvents（整窗
    // 穿透会让按钮也点不到），因此这一条在 webview 内可证——锁定后 pill 仍
    // 在 DOM 且未 disabled；真正的"透明区不挡下方窗口"只能目测。
    // 用户要求的是"锁定时解锁按钮必须能点"。未锁定且未悬停时 pill 是
    // display:none（设计如此：平时不挂灰块），所以必须在**锁定态**测可见性
    // 与命中——第一版在未锁定态测，得到 0×0 却把结论写成了通过。
    const lockState = await dl
      .evalJs(`document.querySelector('.desktop-lyrics')?.dataset.locked ?? null`)
      .then((r) => r.result?.value);
    if (lockState !== 'true') await click(dl, '.dl-pill');
    await sleep(400);
    const pillLive = await dl.evalJs(
      `(()=>{const b=document.querySelector('.dl-pill');
        if(!b) return null;
        const cs=getComputedStyle(b); const r=b.getBoundingClientRect();
        const el=document.elementFromPoint(r.left+r.width/2, r.top+r.height/2);
        return {display:cs.display, w:Math.round(r.width), h:Math.round(r.height),
                locked:document.querySelector('.desktop-lyrics').dataset.locked,
                hitSelf: el===b || b.contains(el)}})()`,
    ).then((r) => r.result?.value);
    const togglesBack = await click(dl, '.dl-pill');
    const unlocked = await dl
      .evalJs(`document.querySelector('.desktop-lyrics')?.dataset.locked ?? null`)
      .then((r) => r.result?.value);
    line('W09',
      !!pillLive && pillLive.locked === 'true' && pillLive.display !== 'none'
        && pillLive.w > 0 && pillLive.h > 0 && pillLive.hitSelf === true
        && togglesBack && unlocked === 'false',
      `锁定态解锁入口：display=${pillLive?.display} ${pillLive?.w}×${pillLive?.h} ` +
        `自身命中=${pillLive?.hitSelf} 点击后 locked=${unlocked}`);

    // 能力面（capabilities windows 列表）若不含 'desktop-lyrics'，这里的 invoke
    // 会被拒——而 UI 侧把错误 .catch 吞掉，用户看到的只是"永远不出词"。
    let ipcErr = null;
    try {
      await dl.inv('player_snapshot', {});
    } catch (e) {
      ipcErr = String(e?.message ?? e).slice(0, 160);
    }
    // 冻结在 2.6s 再读：暂停态下位置保持，歌词行因此是确定值
    await main.inv('pause', {});
    await sleep(1300); // 让 1s 传输淡变落地
    await main.inv('seek', { positionSeconds: 2.6 });
    await sleep(1500); // 悬浮窗 500ms 快照 + 100ms 位置轮询，留足两拍
    const dlNow = await text(dl, '.dl-now');
    const playing = (await snap(main));
    line('W10',
      ipcErr === null && lrcRow >= 0 && dlNow === '第三句',
      `歌词窗 IPC 与出词：invoke=${ipcErr ?? 'OK'} 冻结 2.6s 当前行="${dlNow}"` +
        `（期望 第三句；主窗 ${playing.state}/${(playing.pos ?? 0).toFixed(2)}s）`);

    line('W11', 'skip', '真正的鼠标穿透（锁定时点下方窗口）需肉眼确认——CDP 无法验证 OS 层命中测试');

    await click(main, '.pb-lyrics'); // 关闭歌词窗
    await sleep(800);
    const still = await connectCdpByUrl('view=lyrics', 9223, 1500);
    // 关闭=隐藏（不停播放），Chromium 会为隐藏窗口保留 target，所以这里不能
    // 断言"target 消失"——写成 SKIP 只报观测值，不做恒真断言。
    line('W13', 'skip', `再点「词」后歌词 target：${still ? '仍在（隐藏窗口保留 target，属预期）' : '已不可连接'}`);
  } catch (e) {
    line('W-ERR', false, `异常：${e.message}`);
  } finally {
    stopApp(child, dataDir);
    try {
      rmSync(musicDir, { recursive: true, force: true });
    } catch {
      /* WebView2 释放延迟 */
    }
    await sleep(500);
  }
};

run().then(() => {
  const fails = results.filter((r) => r === 'FAIL').length;
  console.log(`\nMATRIX: ${results.filter((r) => r === 'PASS').length}/${results.length} PASS, ${fails} FAIL`);
  process.exit(fails ? 1 : 0);
});
