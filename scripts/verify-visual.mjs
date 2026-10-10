// L2 视觉/歌词运行时验证（Y 系列）——真实 release 产物 + CDP 驱动。
//
// 为什么存在：PROGRESS 里记过 "Y01–Y05 PASS"，但仓库里从来没有这些断言
// （全仓 grep "Y0" 零命中），也就是说光场/歌词舞台改成什么样都没有回归网。
// 本脚本把那些结论变成可重跑的断言。
//
// 运行：node scripts/verify-visual.mjs   （自动启停应用，隔离资料目录）
// 前置：与源码同代的 release 产物（npx tauri build；plain cargo build 走 devUrl）。
import { mkdtempSync, readdirSync, rmSync, cpSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { connectCdp, startApp, stopApp, APP_EXE } from './lib/qa-app.mjs';

const results = [];
const line = (id, ok, desc) => {
  const verdict = ok === 'skip' ? 'SKIP' : ok ? 'PASS' : 'FAIL';
  results.push(verdict);
  console.log(`[${id}] ${verdict} ${desc}`);
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** 点侧栏按钮（找不到返回 false，让断言自己判失败而不是抛异常）。 */
const goto = (sess, label) =>
  sess.evalJs(
    `(()=>{const b=[...document.querySelectorAll('.nav-btn,.side-btn')]
        .find(x=>x.textContent.includes('${label}'));
       if(!b) return false; b.click(); return true})()`,
  ).then((r) => r.result?.value === true);

/** 等歌词舞台出现（梦境视图是异步挂载的）。 */
const waitForStage = async (sess, ms = 8000) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const v = await sess.evalJs(`!!document.querySelector('.lyric-stage')`);
    if (v.result?.value) return true;
    await sleep(300);
  }
  return false;
};

const stageText = (sess) =>
  sess.evalJs(
    `JSON.stringify({now:document.querySelector('.lyric-now')?.textContent ?? null,
        next:document.querySelector('.lyric-next')?.textContent ?? null,
        empty:document.querySelector('.lyric-stage')?.dataset.empty ?? null})`,
  ).then((r) => JSON.parse(r.result.value));

const canvasDiag = (sess) =>
  sess.evalJs(
    `(()=>{const c=document.querySelector('.dreamscape-canvas');
      if(!c) return null;
      return {fps:Number(c.dataset.fps??-1), err:c.dataset.fieldError??null,
              tel:c.dataset.telemetry??null}})()`,
  ).then((r) => r.result?.value ?? null);

const run = async () => {
  console.log(`QA exe: ${APP_EXE}`);
  const fixtures = join(process.cwd(), 'tests', 'fixtures');
  const musicDir = mkdtempSync(join(tmpdir(), 'aura-vis-'));
  // 只拷播放需要的三份：带 LRC / 无歌词 / 短音效
  for (const f of ['fixture-lrc.mp3', 'fixture-lrc.lrc', 'fixture-notags.mp3', 'fixture.flac']) {
    cpSync(join(fixtures, f), join(musicDir, f));
  }
  const all = readdirSync(musicDir);

  const { child, dataDir } = await startApp();
  let sess = null;
  try {
    sess = await connectCdp();
    const { inv } = sess;

    await inv('scan_folder', { path: musicDir });
    const end = Date.now() + 30_000;
    let tracks = [];
    while (Date.now() < end) {
      await sleep(700);
      tracks = await inv('library_list', { sort: 'title', limit: 50, offset: 0 });
      if (tracks.length >= 3) break;
    }
    const idOf = (f) => tracks.find((t) => t.path && t.path.endsWith(f))?.id ?? null;
    const lrcId = idOf('fixture-lrc.mp3');
    const noTagId = idOf('fixture-notags.mp3');
    line('Y00', tracks.length >= 3 && lrcId !== null && noTagId !== null,
      `扫描 ${all.length} 个文件 → ${tracks.length} 首（lrc=${lrcId} notags=${noTagId}）`);
    if (lrcId === null || noTagId === null) return;

    // 进梦境视图（歌词舞台与光场都在这里）
    const clicked = await goto(sess, '梦境') || (await goto(sess, 'Dreamscape'));
    line('Y01', clicked && (await waitForStage(sess)), `进入梦境视图并等到歌词舞台（点击=${clicked}）`);
    if (!clicked) return;

    // ---- Y02/Y03：歌词当前行/下一行随播放位置切换 ----
    // fixture-lrc.lrc: 0.50 第一句 / 1.50 第二句 / 2.50 第三句 / 3.50 第四句
    await inv('play_track', { trackId: lrcId });
    await sleep(600);
    await inv('seek', { positionSeconds: 0.6 });
    await sleep(700);
    const a = await stageText(sess);
    line('Y02', a.now === '第一句' && a.next === '第二句',
      `seek 0.6s → now="${a.now}" next="${a.next}"（应 第一句/第二句）`);

    await inv('seek', { positionSeconds: 2.6 });
    await sleep(700);
    const b = await stageText(sess);
    line('Y03', b.now === '第三句' && b.next === '第四句',
      `seek 2.6s → now="${b.now}" next="${b.next}"（应 第三句/第四句）`);

    // ---- Y04：无歌词曲目必须给提示，不能留白 ----
    await inv('play_track', { trackId: noTagId });
    await sleep(900);
    const c = await stageText(sess);
    const hinted = /没有歌词|No lyrics/.test(c.now ?? '');
    line('Y04', hinted, `无歌词曲目显示提示：now="${c.now}" empty=${c.empty}`);

    // ---- Y05：光场真的在画（帧率>0 且无 GL 错误） ----
    await inv('play_track', { trackId: lrcId });
    await sleep(2500);
    const d1 = await canvasDiag(sess);
    await sleep(1200);
    const d2 = await canvasDiag(sess);
    line('Y05', !!d1 && !d1.err && d1.fps > 0,
      `光场渲染：fps=${d1?.fps} fieldError=${d1?.err ?? 'null'}`);

    // ---- Y06：播放中电平确实在变（不是常数） ----
    // 必须"起播后立刻"采样：fixture 全部只有 ~3s，若在 Y05 的等待之后才取样，
    // 测的是曲终静音（第一版就是这样把 18/24 个 0 误当成引擎缺陷的）。
    // 每条读数都同时记位置，用来证明采样窗口确实落在播放区间内。
    await inv('play_track', { trackId: lrcId });
    await sleep(400);
    const raw = [];
    for (let i = 0; i < 16; i += 1) {
      const [lv, pos] = await Promise.all([inv('player_level', {}), inv('player_position', {})]);
      raw.push({ ...lv, pos });
      await sleep(80);
    }
    const rms = raw.map((r) => r.rms);
    const posFirst = raw[0].pos;
    const posLast = raw[raw.length - 1].pos;
    const mn = Math.min(...rms);
    const mx = Math.max(...rms);
    const nonZero = rms.filter((v) => v > 0).length;
    line('Y06', mx - mn > 1e-4 && nonZero >= rms.length * 0.9 && posLast < 3.0,
      `播放中 pos=${posFirst.toFixed(2)}→${posLast.toFixed(2)} 24k… ` +
        `rms ∈ [${mn.toFixed(5)}, ${mx.toFixed(5)}] 非零=${nonZero}/${rms.length}`);

    // ---- Y07：静音时电平归零（零门控；不是按音量缩放） ----
    // 重新起播并确认位置仍在曲目内：否则"读数为 0"可能只是曲终静音，
    // 那就成了为错误原因通过的假测试。
    await inv('set_volume', { volume: 0 });
    await inv('play_track', { trackId: lrcId });
    await sleep(500);
    const muted = [];
    for (let i = 0; i < 8; i += 1) {
      const [lv, pos] = await Promise.all([inv('player_level', {}), inv('player_position', {})]);
      muted.push({ ...lv, pos });
      await sleep(80);
    }
    const maxMuted = Math.max(...muted.map((m) => m.rms));
    const stillPlaying = muted[0].pos < 3.0 && muted[muted.length - 1].pos > muted[0].pos;
    line('Y07', maxMuted === 0 && stillPlaying,
      `音量 0 且 pos=${muted[0].pos.toFixed(2)}→${muted[muted.length - 1].pos.toFixed(2)} 播放中 → 电平最大=${maxMuted}`);
    await inv('set_volume', { volume: 1 });

    // ---- Y08：平滑跟随器既收敛又不放大抖动 ----
    // 前两版都在这里翻车：1) 拿"还在从 0 爬升的跟随器"当抖动（收敛≠抖动）；
    // 2) 用未映射的 rms 抖动去比已映射的 telemetry（量纲不同）。正确做法：
    // 先等跟随器收敛，再把原始读数过同一个 mapLevel 映射后逐拍对比。
    // fixture 是稳态正弦 ⇒ 原始抖动本身≈0，此时"平滑更有价值"不可证，
    // 如实 SKIP 并打印数字，而不是让断言恒真。
    const clamp01 = (v) => Math.min(1, Math.max(0, v));
    await inv('set_volume', { volume: 1 });
    await inv('play_track', { trackId: lrcId });
    await sleep(900); // 收敛期（攻击 τ=0.14s，0.9s ≈ 6τ）；fixture 只有 3s，窗口必须省着用
    const pairs = [];
    for (let i = 0; i < 10; i += 1) {
      const pos = await inv('player_position', {});
      if (pos > 2.8) break; // 逼近曲终就停手，别把静音尾段算进抖动
      const [lv, telRaw] = await Promise.all([
        inv('player_level', {}),
        canvasDiag(sess).then((d) => d?.tel ?? null),
      ]);
      if (telRaw) pairs.push({ mapped: clamp01(lv.rms * 2.6), sm: JSON.parse(telRaw).level, pos });
      await sleep(120);
    }
    const jitter = (arr) =>
      arr.length < 2 ? null : arr.slice(1).reduce((s, v, k) => s + Math.abs(v - arr[k]), 0) / (arr.length - 1);
    const rawJ = jitter(pairs.map((p) => p.mapped));
    const smJ = jitter(pairs.map((p) => p.sm));
    const converged =
      pairs.length > 0 && Math.abs(pairs[pairs.length - 1].mapped - pairs[pairs.length - 1].sm) < 0.05;
    const lastPos = pairs.at(-1)?.pos ?? null;
    if (pairs.length < 3) {
      line('Y08', 'skip', `播放窗口内只取到 ${pairs.length} 组配对读数（3s fixture 太短，需带动态的 fixture）`);
    } else if (lastPos !== null && lastPos >= 2.9) {
      line('Y08', false, `采样窗口逼近曲终（末位置=${lastPos.toFixed(2)}），读数不可信`);
    } else if (rawJ !== null && rawJ < 1e-3) {
      line('Y08', 'skip',
        `稳态正弦原始抖动≈0（mapped=${rawJ.toExponential(1)} vs 平滑=${smJ?.toExponential(1)}）——` +
          `收敛性可证（${converged}），抖动抑制需要带动态的 fixture 才能证`);
    } else {
      line('Y08', converged && smJ !== null && smJ <= rawJ,
        `收敛=${converged} 抖动 raw=${rawJ?.toExponential(2)} smooth=${smJ?.toExponential(2)}`);
    }

    // ---- Y09：WebGL 上下文丢失 → fallback；恢复 → 自动续画 ----
    // 必须在 loseContext 之前就把 WEBGL_lose_context 句柄存下来：上下文丢失
    // 之后再 getContext('webgl2').getExtension(...) 可能拿不到句柄，restore
    // 调用就永远不会发生 —— 第一版 Y09 正是这样把"环境不支持"误判成代码 bug。
    await sess.evalJs(`(()=>{const c=document.querySelector('.dreamscape-canvas');
      window.__ext2 = c.getContext('webgl2').getExtension('WEBGL_lose_context'); return 1})()`);
    const lost = await sess.evalJs(
      `(()=>{const c=document.querySelector('.dreamscape-canvas');
        if(!window.__ext2) return 'no-ext';
        window.__ext2.loseContext(); return 'lost'})()`,
    );
    await sleep(700);
    const during = await canvasDiag(sess);
    await sess.evalJs('window.__ext2 && window.__ext2.restoreContext(); 1');
    const after = await (async () => {
      const end2 = Date.now() + 8000;
      while (Date.now() < end2) {
        await sleep(400);
        const s = await canvasDiag(sess);
        if (s && !s.err && s.fps > 0) return s;
      }
      return await canvasDiag(sess);
    })();
    line('Y09',
      lost.result?.value === 'lost' && during?.err === 'context-lost' && !!after && !after.err && after.fps > 0,
      `上下文丢失→${during?.err}，恢复后 fps=${after?.fps} err=${after?.err ?? 'null'}`);
  } catch (e) {
    line('Y-ERR', false, `异常：${e.message}`);
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
