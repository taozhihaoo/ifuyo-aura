// L2 远程曲库验证（R 系列）——打真实服务器 https://ifuyo.com/audio 走完
// 「拉清单 → 校验 → 列表页 → 流式播放」，并验证策略面：
// 远程清单的 URL 必须走 deny_private，而 "Open URL" 仍保留 D-070 的私网确认。
//
// 运行：node scripts/verify-remote.mjs
// 前置：与源码同代的 release 产物；网络可达 ifuyo.com（不可达时整批 SKIP）。
import { connectCdp, startApp, stopApp, APP_EXE } from './lib/qa-app.mjs';

const results = [];
const line = (id, ok, desc) => {
  const verdict = ok === 'skip' ? 'SKIP' : ok ? 'PASS' : 'FAIL';
  results.push(verdict);
  console.log(`[${id}] ${verdict} ${desc}`);
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** 在页面里 invoke 并把 AppError 的 code 取回来。
 *  不走 sess.inv：它把异常统一包成 Error(字符串)，code 会丢。 */
async function probeCode(sess, cmd, args) {
  const r = await sess.awaitJs(
    `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args ?? {})})
       .then(() => 'RESOLVED')
       .catch((e) => (typeof e === 'object' && e !== null && 'code' in e) ? e.code : 'ERR:' + String((e && e.message) ?? e).slice(0, 120))`,
  );
  const v = r.result?.value;
  if (v !== undefined) return v;
  // 取不到值时如实暴露 CDP 侧原因，别让"我读不到"伪装成"行为异常"。
  const d = r.exceptionDetails;
  return `NO-VALUE(${d ? String(d.exception?.description ?? d.text ?? d.message).slice(0, 140) : JSON.stringify(r).slice(0, 140)})`;
}

const run = async () => {
  console.log(`QA exe: ${APP_EXE}`);
  const { child, dataDir } = await startApp();
  try {
    const sess = await connectCdp();
    const { inv, evalJs } = sess;

    // ---- R01 拉取真实清单 ----
    let list = null;
    let loadErr = null;
    try {
      list = await inv('ifuyo_list', {});
    } catch (e) {
      loadErr = codeOf(e) ?? String(e?.message ?? e).slice(0, 80);
    }
    if (list === null) {
      line('R01', 'skip', `远程清单不可达（${loadErr}）——网络/服务器问题，跳过整批`);
      return;
    }
    line('R01', Array.isArray(list.tracks) && list.tracks.length > 0 &&
        list.tracks.every((t) => typeof t.url === 'string' && t.url.startsWith('https://')),
      `ifuyo_list → ${list.tracks.length} 首，generatedAt=${list.generatedAt} skipped=${list.skipped}`);

    // ---- R02 列表页渲染（导航 + 标题 + null 时长显示 --:--） ----
    await evalJs(`(()=>{const b=[...document.querySelectorAll('.nav-btn')].find(x=>/ifuyo/i.test(x.textContent));
      if(!b) return false; b.click(); return true})()`);
    await sleep(1200);
    const rows = await evalJs(`document.querySelectorAll('.ifuyo-row').length`).then((r) => r.result?.value ?? 0);
    const hasDash = await evalJs(
      `[...document.querySelectorAll('.ifuyo-time')].some(e=>e.textContent==='--:--')`,
    ).then((r) => r.result?.value === true);
    const anyZero = await evalJs(
      `[...document.querySelectorAll('.ifuyo-time')].some(e=>e.textContent==='0:00')`,
    ).then((r) => r.result?.value === true);
    const firstTitle = await evalJs(
      `document.querySelector('.ifuyo-row .ifuyo-title')?.textContent ?? null`,
    ).then((r) => r.result?.value);
    line('R02', rows === list.tracks.length && firstTitle !== null && !anyZero &&
        (list.tracks.some((t) => t.durationSeconds === null) ? hasDash : true),
      `列表页渲染 ${rows}/${list.tracks.length} 行，首行="${firstTitle}"，无伪造 0:00`);

    // ---- R03 点击真实曲目 → 流式播放并推进 ----
    // 必须挑**有 duration 的那条**：清单里第一行是占位的 test.mp3（1MB 无标签
    // 假数据，本来就解不开），拿它断言"播放失败"是把测试数据问题当成产品缺陷。
    const playIdx = list.tracks.findIndex((t) => t.durationSeconds !== null);
    await evalJs(
      `document.querySelectorAll('.ifuyo-row')[${playIdx}]?.click(); 1`,
    );
    let st = null;
    const end = Date.now() + 20_000;
    while (Date.now() < end) {
      await sleep(700);
      const s = await inv('player_snapshot', {});
      st = s;
      if (s?.state === 'playing') break;
    }
    const p1 = st?.positionSeconds ?? 0;
    await sleep(1500);
    const p2 = (await inv('player_snapshot', {})).positionSeconds ?? 0;
    line('R03', playIdx >= 0 && st?.state === 'playing' && p2 - p1 >= 0.5,
      `远程流播放（第 ${playIdx} 行）：state=${st?.state} pos ${p1.toFixed(2)}→${p2.toFixed(2)}s`);

    // ---- R03b 播放条标题必须是清单里的可读曲名 ----
    // 此前远程播放只显示 "%E7%8E%B0…"：清单里的 title/artist 没传下去，
    // UI 只能拿 percent-encoded 的 URL 末段回退。
    const want = list.tracks[playIdx];
    line('R03b', typeof st?.title === 'string' && st.title.includes('%') === false &&
        (want?.title === undefined || st?.title === want.title),
      `远程标题：snapshot.title="${st?.title}"（清单="${want?.title}"）`);
    await inv('stop', {});

    // ---- R04/R05 策略面：远程清单入口必须拒绝私网地址 ----
    const loopback = await probeCode(sess, 'ifuyo_play', { url: 'http://127.0.0.1:9/x.mp3' });
    line('R04', loopback === 'SOURCE_BLOCKED', `ifuyo_play 私网回环 → ${loopback}（期望 SOURCE_BLOCKED）`);
    const lan = await probeCode(sess, 'ifuyo_play', { url: 'http://192.168.1.7/a.mp3' });
    line('R05', lan === 'SOURCE_BLOCKED', `ifuyo_play 局域网 → ${lan}`);
    const meta = await probeCode(sess, 'ifuyo_play', { url: 'http://169.254.169.254/latest/meta-data' });
    line('R05b', meta === 'SOURCE_BLOCKED', `ifuyo_play 云元数据端点 → ${meta}`);
    const cred = await probeCode(sess, 'ifuyo_play', { url: 'https://u:p@ifuyo.com/a.mp3' });
    line('R05c', cred === 'SOURCE_BLOCKED', `ifuyo_play 带凭据 URL → ${cred}`);

    // ---- R06 对照：Open URL 仍保留 D-070（用户输入即确认私网可达）----
    const openUrl = await probeCode(sess, 'play_input', { input: 'http://127.0.0.1:9/x.mp3' });
    line('R06', !String(openUrl).startsWith('NO-VALUE') && openUrl !== 'SOURCE_BLOCKED',
      `play_input 私网回环 → ${openUrl}（不应被策略拒绝；连接失败属预期）`);
  } catch (e) {
    line('R-ERR', false, `异常：${String(e?.message ?? e).slice(0, 200)}`);
  } finally {
    stopApp(child, dataDir);
    await sleep(500);
  }
};

run().then(() => {
  const fails = results.filter((r) => r === 'FAIL').length;
  console.log(`\nMATRIX: ${results.filter((r) => r === 'PASS').length} PASS / ${fails} FAIL / ${results.filter((r) => r === 'skip' || r === 'SKIP').length} SKIP`);
  process.exit(fails ? 1 : 0);
});
