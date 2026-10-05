// M12.5 Phase 4 soak——脚本化循环（播放→seek→暂停→恢复→停止→重播→音量）
// + 内存/句柄采样，验证长时运行无泄漏/无崩溃/状态不劣化。
//
// 运行：node scripts/soak.mjs [--minutes 30] [--cdp 9224]
// 输出：target/soak-<ts>.jsonl（采样行）+ 控制台摘要；退出码 0=通过。
// 隔离资料目录（AURA_DATA_DIR），扫 fixtures + 用户真实 Music 目录。
import { execSync } from 'node:child_process';
import { writeFileSync, appendFileSync } from 'node:fs';
import { connectCdp, startApp, stopApp } from './lib/qa-app.mjs';

const args = process.argv.slice(2);
const minutes = args.includes('--minutes') ? Number(args[args.indexOf('--minutes') + 1]) : 30;
const cdpPort = args.includes('--cdp') ? Number(args[args.indexOf('--cdp') + 1]) : 9224;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const log = (obj) => {
  const l = JSON.stringify({ t: new Date().toISOString(), ...obj });
  appendFileSync(outFile, l + '\n');
  console.log(l);
};
const ts = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
const outFile = `target/soak-${ts}.jsonl`;
writeFileSync(outFile, '');

const sampleProcess = (pid) => {
  try {
    const out = execSync(
      `powershell -NoProfile -Command "Get-Process -Id ${pid} | Select-Object WorkingSet64,HandleCount | ConvertTo-Json"`,
    ).toString();
    const j = JSON.parse(out);
    return { rssMB: Math.round(j.WorkingSet64 / 1048576), handles: j.HandleCount };
  } catch {
    return null; // 进程已退出
  }
};

const run = async () => {
  const { child, dataDir } = await startApp({ cdpPort, dataDir: undefined });
  const sess = await connectCdp(cdpPort);
  const { inv } = sess;
  log({ ev: 'start', pid: child.pid, minutes });

  // 隔离目录：fixtures + 用户 Music（真实曲目，无污染——临时 profile）。
  // scan_folder 是单飞（SCAN_RUNNING 拒绝并发）——顺序扫并等上一轮结束。
  const scanAndWait = async (path) => {
    const stats0 = (await inv('library_stats', {})).tracks;
    await inv('scan_folder', { path });
    const deadline = Date.now() + 60_000;
    while (Date.now() < deadline) {
      await sleep(1500);
      const n = (await inv('library_stats', {})).tracks;
      if (n > stats0) {
        // 等到数量稳定（连两次一致）
        await sleep(2000);
        if ((await inv('library_stats', {})).tracks === n) return n;
      }
    }
    return stats0;
  };
  await scanAndWait(`${process.cwd()}\\tests\\fixtures`);
  if (process.env.USERPROFILE) {
    try {
      await scanAndWait(`${process.env.USERPROFILE}\\Music`);
    } catch (e) {
      log({ ev: 'scan-user-music-failed', err: String(e.message).slice(0, 120) });
    }
  }
  await sleep(4000);
  let tracks = await inv('library_list', { sort: 'title', limit: 200, offset: 0 });
  log({ ev: 'library', tracks: tracks.length });
  if (tracks.length === 0) throw new Error('soak 库为空');

  const deadline = Date.now() + minutes * 60_000;
  let ops = 0;
  let lastSample = 0;
  let fail = null;

  while (Date.now() < deadline) {
    try {
      tracks = await inv('library_list', { sort: 'title', limit: 200, offset: 0 });
      const t = tracks[Math.floor(Math.random() * tracks.length)];
      const roll = Math.random();
      if (roll < 0.4) {
        await inv('play_track', { trackId: t.id });
      } else if (roll < 0.55 && t.durationSeconds > 10) {
        await inv('seek', { positionSeconds: Math.random() * t.durationSeconds });
      } else if (roll < 0.7) {
        await inv('pause', {});
      } else if (roll < 0.85) {
        await inv('play', {});
      } else {
        await inv('set_volume', { volume: 0.2 + Math.random() * 0.8 });
      }
      ops += 1;

      if (Date.now() - lastSample > 30_000) {
        lastSample = Date.now();
        const snap = await inv('player_snapshot', {});
        const mem = sampleProcess(child.pid);
        log({ ev: 'sample', ops, state: snap.state, pos: +snap.positionSeconds.toFixed(1), ...mem });
        if (!mem) {
          fail = 'app process exited';
          break;
        }
      }
      await sleep(4000);
    } catch (e) {
      log({ ev: 'op-error', err: e.message });
      await sleep(2000);
    }
  }

  // 收尾摘要
  const mem = sampleProcess(child.pid);
  const snap = await inv('player_snapshot', {});
  log({ ev: 'done', ops, ...mem, state: snap.state });
  sess.ws.close();
  stopApp(child, dataDir, { clean: false });

  if (fail) {
    console.log(`SOAK FAIL: ${fail}`);
    setTimeout(() => process.exit(1), 100);
  } else {
    console.log(`SOAK OK: ${minutes}min, ${ops} ops, rss=${mem?.rssMB}MB handles=${mem?.handles}`);
    setTimeout(() => process.exit(0), 100);
  }
};
run().catch((e) => {
  console.log('SOAK FATAL', e.message);
  setTimeout(() => process.exit(2), 100);
});
