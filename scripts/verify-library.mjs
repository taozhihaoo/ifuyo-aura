// M12.5 L2 verify-library（Phase 3 消费端）——真实格式 fixture 的
// 扫描→元数据→封面→搜索→播放→重扫幂等 全旅程（隔离资料目录）。
//
// 运行：node scripts/verify-library.mjs（自动启停应用，勿手动开 9223）
import { cpSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
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
  const files = readdirSync(fixtures).filter((f) => !f.startsWith('_'));
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
    line('L13', after.tracks === 7 && after.artists === 3 && after.albums === 1,
      `stats 精确值：tracks=${after.tracks}(7) artists=${after.artists}(3) albums=${after.albums}(1)`);
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
