// M12.5 L2 verify-library（Phase 3 消费端）——真实格式 fixture 的
// 扫描→元数据→封面→搜索→播放→重扫幂等 全旅程（隔离资料目录）。
//
// 运行：node scripts/verify-library.mjs（自动启停应用，勿手动开 9223）
import { cpSync, existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { connectCdp, startApp, stopApp } from './lib/qa-app.mjs';

const results = [];
const line = (id, ok, desc) => {
  const verdict = ok === 'skip' ? 'SKIP' : ok ? 'PASS' : 'FAIL';
  results.push(verdict);
  console.log(`[${id}] ${verdict} ${desc}`);
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const run = async () => {
  const fixtures = join(process.cwd(), 'tests', 'fixtures');
  const files = readdirSync(fixtures).filter(
    (f) => !f.startsWith('_') && /\.(mp3|flac|ogg|m4a|aiff)$/i.test(f),
  );
  const musicDir = mkdtempSync(join(tmpdir(), 'aura-lib-'));
  cpSync(fixtures, musicDir, { recursive: true });

  const { child, dataDir } = await startApp();
  let ws = null;
  try {
    const sess = await connectCdp();
    ws = sess.ws;
    const { inv } = sess;

    // L-01 扫描真实 fixture 目录
    await inv('scan_folder', { path: musicDir });
    let stats = null;
    const deadline = Date.now() + 30_000;
    while (Date.now() < deadline) {
      await sleep(700);
      stats = await inv('library_stats', {});
      if (stats.tracks >= files.length) break;
    }
    line('L01', stats && stats.tracks >= files.length, `扫描 ${musicDir} → ${stats?.tracks}/${files.length} 首`);

    // 全部 fixture 按 path 可查
    const ids = {};
    for (const f of files) {
      const id = await inv('track_id_by_path', { path: join(musicDir, f) });
      ids[f] = id;
    }
    const found = Object.values(ids).filter(Boolean).length;
    line('L02', found === files.length, `按路径可查 ${found}/${files.length}`);

    // L-03 ID3v2.3 元数据
    let s = await inv('library_list', { sort: 'title', limit: 50, offset: 0 });
    const byFile = (f) => s.find((t) => t.path && t.path.endsWith(f));
    const v23 = byFile('fixture-id3v2.3.mp3');
    line('L03', v23 && v23.title === 'Fixture曲目' && v23.artist === '测试艺术家',
      `ID3v2.3 元数据：title=${v23?.title} artist=${v23?.artist}`);

    // L-04 ID3v2.4 元数据
    const v24 = byFile('fixture-id3v2.4.mp3');
    line('L04', v24 && v24.title === 'Fixture曲目', `ID3v2.4 元数据：title=${v24?.title}`);

    // L-05 Unicode 文件名 + 中文元数据
    const uni = s.find((t) => t.title === '文件名测试曲目🎵');
    line('L05', !!uni, `Unicode 元数据/文件名：${uni ? uni.title : '未找到'}`);

    // L-06 超长标签（8KB title）不破坏解析——存在且库可用即可
    const long = byFile('fixture-longtag.mp3');
    line('L06', !!long, `超长标签曲目入库：${long ? `title 长度 ${long.title?.length ?? 0}` : '未找到'}`);

    // L-07 无标签：title=None 是设计（UI 以 `title ?? path` 回退显示）
    const noTags = byFile('fixture-notags.mp3');
    line('L07', !!noTags && noTags.title === null && !!noTags.path,
      `无标签设计（title=null + UI 路径回退）：title=${noTags?.title}`);

    // L-08 时长解析（3s fixture）
    const durOk = [v23, v24, uni].filter(Boolean).every((t) => Math.abs(t.durationSeconds - 3) < 1);
    line('L08', durOk, `时长解析 ≈3s：${[v23, v24, uni].filter(Boolean).map((t) => t.durationSeconds?.toFixed(1)).join('/')}`);

    // L-09 内嵌封面（ID3v2.3 / v2.4 / FLAC PICTURE）
    const flac = byFile('fixture.flac');
    const artFor = async (t) => (t ? await inv('track_artwork', { trackId: t.id }) : null);
    const a23 = await artFor(v23);
    const aFlac = await artFor(flac);
    line('L09', !!(a23 && aFlac), `内嵌封面提取：mp3=${a23 ? `data ${String(a23).length}B` : '无'} flac=${aFlac ? `data ${String(aFlac).length}B` : '无'}`);

    // L-10 搜索
    const hits = await inv('library_search', { query: 'Fixture', limit: 10, offset: 0 });
    line('L10', hits.length >= 2, `搜索 "Fixture" → ${hits.length} 条`);

    // L-11 播放 fixture（真实解码路径：MP3 44.1k）
    if (v23) {
      await inv('play_track', { trackId: v23.id });
      await sleep(1500);
      const snap = await inv('player_snapshot', {});
      line('L11', snap.state === 'playing' && snap.positionSeconds > 0,
        `fixture 播放：state=${snap.state} pos=${snap.positionSeconds.toFixed(2)} dur=${snap.durationSeconds?.toFixed(2)}`);
      await inv('stop', {});
    } else {
      line('L11', 'skip', '无 v23 fixture');
    }

    // L-12 重扫幂等（同目录再扫不产生重复）
    const before = await inv('library_stats', {});
    await inv('scan_folder', { path: musicDir });
    await sleep(3000);
    const after = await inv('library_stats', {});
    line('L12', after.tracks === before.tracks, `重扫幂等：${before.tracks} → ${after.tracks}`);

    // L-13 库统计精确值（library_stats 列名 bug 回归：曾恒 0）
    // 期望值由 fixture 集合推导，不写死数字：加一条 fixture 就改一次常数，
    // 迟早会变成"改测试迁就实现"（本次新增 fixture-dyn.mp3 正撞在这上面）。
    // 曲目数 = 音频文件数 + CUE 子轨数（整轨主行在子轨展开后被停用，不计数）。
    const cuePath = join(fixtures, 'fixture-cue.cue');
    const cueTracks = existsSync(cuePath)
      ? readFileSync(cuePath, 'utf-8')
          .split(/\r?\n/)
          .filter((l) => /^\s*TRACK\s+\d+\s+AUDIO/i.test(l)).length
      : 0;
    const expectTracks = files.length + cueTracks;
    line('L13', after.tracks === expectTracks && after.artists === 3 && after.albums === 3,
      `stats 精确值：tracks=${after.tracks}(${expectTracks}=${files.length}+cue${cueTracks}) ` +
      `artists=${after.artists}(3) albums=${after.albums}(3)`);

    // L-14 封面缩略图列（M13.2）：5 个带封面 fixture → 曲库可视行渲染 <img>
    await inv('library_list', { sort: 'title', limit: 50, offset: 0 });
    const { ws: ws2, evalJs } = await (async () => connectCdp(9223))();
    try {
      await evalJs("[...document.querySelectorAll('.nav-btn,.side-btn')].find(b=>b.textContent.includes('音乐库')||b.textContent.includes('Library'))?.click()");
      await sleep(1200);
      const thumbs = await evalJs("document.querySelectorAll('.track-list .tr-art-img').length");
      const thumbCount = Number(thumbs?.result?.value ?? 0);
      line('L14', thumbCount >= 4, `封面缩略图列：${thumbCount} 个 <img>（≥4，5 带封面 fixture）`);

      // L-15 沉浸模糊背景（M13.2）：播放带封面曲目 → 正在播放页 .np-ambient
      if (v23) {
        await inv('play_track', { trackId: v23.id });
        await sleep(1800);
        await evalJs("[...document.querySelectorAll('.nav-btn,.side-btn')].find(b=>b.textContent.includes('正在播放')||b.textContent.includes('Now Playing'))?.click()");
        await sleep(600);
        const ambient = await evalJs("!!document.querySelector('.np-ambient img')");
        line('L15', ambient?.result?.value === true, `沉浸封面背景 .np-ambient：${ambient?.result?.value}`);
        await inv('stop', {});
      } else {
        line('L15', 'skip', '无 v23 fixture');
      }

      // L16/L17/L18 歌词（M13.3）：.lrc 优先→USLT 标签→无
      const lrcTrack = s.find((t) => t.path && t.path.endsWith('fixture-lrc.mp3'));
      const l16 = lrcTrack ? await inv('lyrics_for_track', { trackId: lrcTrack.id }) : null;
      line('L16', !!l16 && l16.synced === true && l16.lines.length === 4 && l16.lines[0].text === '第一句',
        `LRC 歌词：synced=${l16?.synced} lines=${l16?.lines?.length}（应为 4 行时间戳）`);
      const v23b = s.find((t) => t.path && t.path.endsWith('fixture-id3v2.3.mp3'));
      const l17 = v23b ? await inv('lyrics_for_track', { trackId: v23b.id }) : null;
      line('L17', !!l17 && l17.synced === false && l17.lines.length === 3 && l17.lines[0].text === '第一句歌词',
        `USLT 标签歌词：synced=${l17?.synced} lines=${l17?.lines?.length}（应 3 行纯文本）`);
      const noneTrack = s.find((t) => t.path && t.path.endsWith('fixture-notags.mp3'));
      const l18 = noneTrack ? await inv('lyrics_for_track', { trackId: noneTrack.id }) : null;
      line('L18', l18 === null, `无歌词回退 null：${JSON.stringify(l18)}`);

      // L19 M15a 新格式播放矩阵：AAC(M4A)/ALAC(M4A)/AIFF 全部真实解码
      const fmtFiles = ['fixture.m4a', 'fixture-alac.m4a', 'fixture.aiff'];
      const fmtResults = [];
      for (const f of fmtFiles) {
        const t = s.find((x) => x.path && x.path.endsWith(f));
        if (!t) {
          fmtResults.push(`${f}:MISSING`);
          continue;
        }
        await inv('play_track', { trackId: t.id });
        // 轮询等待 playing（首次播放含编解码器初始化，容 3s 延迟）
        let ok = false;
        let at = '';
        for (let i = 0; i < 10; i += 1) {
          await sleep(300);
          const st = await inv('player_snapshot', {});
          if (st.state === 'playing') {
            ok = true;
            at = `${((i + 1) * 0.3).toFixed(1)}s`;
            break;
          }
        }
        fmtResults.push(`${f}:${ok ? 'OK@' + at : 'FAIL'}`);
        await inv('stop', {});
        await sleep(400);
      }
      line('L19', fmtResults.every((r) => r.includes('OK@')), `新格式播放矩阵 ${fmtResults.join(' | ')}`);

      const s2fresh = await inv('library_list', { sort: 'title', limit: 50, offset: 0 });
      // L21/L22 CUE 整轨（M15a）：虚拟子轨入库 + 子轨播放坐标
      const cueRows = s2fresh.filter((x) => x.path && x.path.includes('fixture-cue.flac#'));
      line('L21', cueRows.length === 6 && cueRows.some((r) => r.title === '轨道三'),
        `CUE 虚拟子轨：${cueRows.length} 条（应 6）｜全部路径尾部=${s2fresh.map((x) => x.path?.split(/[\/]/).pop()).join(', ')}`);
      const cue3 = cueRows.find((r) => r.title === '轨道三');
      if (cue3) {
        await inv('play_track', { trackId: cue3.id });
        await sleep(1200);
        const st = await inv('player_snapshot', {});
        const posOk = st.positionSeconds >= 0.9 && st.positionSeconds <= 2.0;
        const durOk = Math.abs(st.durationSeconds - 2) < 0.1;
        line('L22', st.state === 'playing' && posOk && durOk,
          `CUE 子轨播放：state=${st.state} pos=${st.positionSeconds.toFixed(2)}（子轨内 0.9-2.0）dur=${st.durationSeconds?.toFixed(2)}（应 ≈2.0）`);
        await inv('stop', {});
      } else {
        line('L22', 'skip', '无 轨道三');
      }

      // L20 GBK 乱码标题修复（M15a）：扫描时 Latin-1→GBK 启发式恢复
      const gbk = s.find((x) => x.path && x.path.endsWith('fixture-gbk.mp3'));
      const gbkFresh = s2fresh.find((x) => x.path && x.path.endsWith('fixture-gbk.mp3'));
      void gbk;
      line('L20', !!gbkFresh && gbkFresh.title === '周杰伦',
        `GBK 乱码修复：title=${gbkFresh?.title}（应为 周杰伦；修复前=${gbk?.title?.slice(0, 8)}…）`);
    } finally {
      try { ws2?.close(); } catch { /* 已断开 */ }
    }

    // L23 M8 分析接线（本批次新增）：后台 worker 自动跑完 fixture →
    // 覆盖度归零，且 LUFS 真实入库（响度归一与 Music Map 的数据源）。
    let astat = null;
    const aDeadline = Date.now() + 90_000;
    while (Date.now() < aDeadline) {
      // 轮询本身可能瞬态失败（DB 被 worker 写锁占用/IPC 无回包）：
      // 一次抛错不该让 L23 整段中止，吞掉并等下一轮。
      try {
        astat = await inv('analysis_status', {});
        if (astat && astat.pending === 0 && astat.analyzed > 0) break;
      } catch {
        /* 重试 */
      }
      await sleep(1000);
    }
    const flacId = ids['fixture.flac'];
    let sum = null;
    let astate = null;
    try {
      sum = flacId ? await inv('track_analysis_summary', { trackId: flacId }) : null;
    } catch (e) {
      sum = { error: JSON.stringify(e) };
    }
    try {
      astate = flacId ? await inv('track_analysis_state', { trackId: flacId }) : null;
    } catch (e) {
      astate = `err:${JSON.stringify(e)}`;
    }
    line('L23',
      !!astat && astat.pending === 0 && astat.analyzed > 0
        && !!sum && sum.status === 'completed' && typeof sum.integratedLufs === 'number',
      `分析接线：analyzed=${astat?.analyzed} pending=${astat?.pending} `
      + `progress=${astat?.progress?.processed}/${astat?.progress?.total} failed=${astat?.progress?.failed}`
      + `｜id=${flacId} state=${astate}｜summary=${sum ? JSON.stringify(sum).slice(0, 220) : 'null'}`);

    // L24 分析 → Music Map：已分析曲目必须真能建出图（回答"分析有什么用"）。
    try {
      await inv('request_map_build', {});
    } catch (e) {
      console.log('  map build request failed:', JSON.stringify(e));
    }
    let mstat = null;
    const mDeadline = Date.now() + 60_000;
    while (Date.now() < mDeadline) {
      mstat = await inv('get_map_status', {});
      if (mstat && mstat.status !== 'pending' && mstat.status !== 'building') break;
      await sleep(1000);
    }
    const ov = mstat?.status === 'ready' ? await inv('get_map_overview', {}) : null;
    line('L24', mstat?.status === 'ready' && !!ov && ov.trackCount > 0,
      `分析→地图：status=${mstat?.status} msg=${mstat?.message ?? ''} `
      + `nodes=${ov?.trackCount ?? 0} clusters=${ov?.clusterCount ?? 0} `
      + `mapped=${ov?.coverage?.mappedTracks ?? 0}/${ov?.coverage?.totalTracks ?? 0}`);
  } finally {
    try {
      ws?.close();
    } catch {
      /* 已断开 */
    }
    stopApp(child, dataDir);
    setTimeout(() => rmSync(musicDir, { recursive: true, force: true }), 2000);
  }

  const fail = results.filter((r) => r === 'FAIL').length;
  const pass = results.filter((r) => r === 'PASS').length;
  console.log(`MATRIX: ${pass}/${results.length} PASS, ${fail} FAIL`);
  setTimeout(() => process.exit(fail > 0 ? 1 : 0), 100);
};
run().catch((e) => {
  console.log('FATAL', e.message);
  setTimeout(() => process.exit(2), 100);
});
