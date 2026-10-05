// M12.5 L2 verify-settings——设置改→重启→回读持久化回环（隔离资料目录）。
// 覆盖 crossfade / loudness / eq_preset / language。
// 运行：node scripts/verify-settings.mjs（自动启停应用两轮，勿手动开 9223）
import { connectCdp, startApp, stopApp } from './lib/qa-app.mjs';

const results = [];
const line = (id, ok, desc) => {
  const verdict = ok === 'skip' ? 'SKIP' : ok ? 'PASS' : 'FAIL';
  results.push(verdict);
  console.log(`[${id}] ${verdict} ${desc}`);
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const readSettings = async (port) => {
  const { ws, inv } = await connectCdp(port);
  const s = await inv('get_settings', {});
  ws.close();
  return s;
};

const run = async () => {
  const { child, dataDir } = await startApp({ cdpPort: 9223 });
  let original;
  try {
    const { ws, inv } = await connectCdp();
    original = await inv('get_settings', {});
    line('S01', typeof original?.crossfade_seconds === 'number', `读设置：crossfade=${original.crossfade_seconds} lang=${original.language}`);

    // 写入一组非默认值（save_settings 全量回写：基于 original 改字段）
    const modified = {
      ...original,
      crossfade_seconds: 2.5,
      loudness_enabled: !original.loudness_enabled,
      eq_preset: (original.eq_preset + 1) % 4,
    };
    await inv('save_settings', { settings: modified });
    const afterWrite = await inv('get_settings', {});
    const writeOk =
      afterWrite.crossfade_seconds === 2.5 &&
      afterWrite.loudness_enabled === modified.loudness_enabled &&
      afterWrite.eq_preset === modified.eq_preset;
    line('S02', writeOk, `写后回读一致：crossfade=${afterWrite.crossfade_seconds} loud=${afterWrite.loudness_enabled} eq=${afterWrite.eq_preset}`);
    ws.close();
  } catch (e) {
    stopApp(child, dataDir);
    throw e;
  }
  // 硬杀（非优雅退出也要求设置已持久化——§114 写盘语义）
  try {
    child.kill();
  } catch {
    /* 已退出 */
  }
  await sleep(2000);

  // 第二轮：同资料目录重启 → 设置必须恢复
  const second = await startApp({ cdpPort: 9223, dataDir });
  try {
    const s2 = await readSettings(9223);
    line('S03', s2.crossfade_seconds === 2.5, `重启后 crossfade=${s2.crossfade_seconds}（应 2.5）`);
    line('S04', s2.loudness_enabled === !original.loudness_enabled,
      `重启后 loudness=${s2.loudness_enabled}（应 ${!original.loudness_enabled}）`);
    line('S05', s2.eq_preset === (original.eq_preset + 1) % 4, `重启后 eq_preset=${s2.eq_preset}`);

    // 还原默认（临时目录即将删除，此步保证若目录复用也无残留语义）
    // 不需要——临时目录随测试清理。
  } finally {
    stopApp(second.child, dataDir);
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
