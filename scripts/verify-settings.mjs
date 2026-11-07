// M12.5 L2 verify-settings——设置改→重启→回读持久化回环（隔离资料目录）。
// 覆盖 crossfade / loudness / eqPreset / language。
// 运行：node scripts/verify-settings.mjs（自动启停应用两轮，勿手动开 9223）
//
// 契约：IPC/磁盘 JSON 为 camelCase（与 ui/src/ipc/types.ts 一致）。
// 旧版脚本直接用 snake_case 调 IPC，绕过了真实 UI 消费端，因此没能
// 发现 AppSettings 缺 serde(rename_all) 导致前端读到全 undefined 的问题。
// S00 固定该契约，S06 验证 M15b 之前写盘的 snake_case 仍可读。
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
    const shapeOk =
      original &&
      typeof original.crossfadeSeconds === 'number' &&
      typeof original.eqPreset === 'number' &&
      typeof original.loudnessEnabled === 'boolean' &&
      typeof original.updatesAutoCheck === 'boolean' &&
      typeof original.uiScale === 'number' &&
      !('crossfade_seconds' in original) &&
      !('eq_preset' in original);
    line('S00', shapeOk, `IPC 契约为 camelCase（前端消费端同名）：crossfade=${original?.crossfadeSeconds} uiScale=${original?.uiScale}`);
    line('S01', typeof original?.crossfadeSeconds === 'number', `读设置：crossfade=${original.crossfadeSeconds} lang=${original.language}`);

    // 写入一组非默认值（save_settings 全量回写：基于 original 改字段）
    const modified = {
      ...original,
      crossfadeSeconds: 2.5,
      loudnessEnabled: !original.loudnessEnabled,
      eqPreset: (original.eqPreset + 1) % 4,
    };
    await inv('save_settings', { settings: modified });
    const afterWrite = await inv('get_settings', {});
    const writeOk =
      afterWrite.crossfadeSeconds === 2.5 &&
      afterWrite.loudnessEnabled === modified.loudnessEnabled &&
      afterWrite.eqPreset === modified.eqPreset;
    line('S02', writeOk, `写后回读一致：crossfade=${afterWrite.crossfadeSeconds} loud=${afterWrite.loudnessEnabled} eq=${afterWrite.eqPreset}`);
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
    line('S03', s2.crossfadeSeconds === 2.5, `重启后 crossfade=${s2.crossfadeSeconds}（应 2.5）`);
    line('S04', s2.loudnessEnabled === !original.loudnessEnabled,
      `重启后 loudness=${s2.loudnessEnabled}（应 ${!original.loudnessEnabled}）`);
    line('S05', s2.eqPreset === (original.eqPreset + 1) % 4, `重启后 eqPreset=${s2.eqPreset}`);

    // S06：旧盘/旧客户端的 snake_case 载荷必须仍能读（serde alias 兼容），
    // 且回读统一为 camelCase——防止升级后用户偏好被静默重置。
    const { ws, inv } = await connectCdp(9223);
    const legacy = {
      version: 1,
      language: s2.language,
      updates_auto_check: s2.updatesAutoCheck,
      mojibake_fix: s2.mojibakeFix,
      analysis_auto: s2.analysisAuto,
      ui_scale: 0.9,
      reduced_motion: s2.reducedMotion,
      loudness_enabled: s2.loudnessEnabled,
      crossfade_seconds: 3.5,
      eq_enabled: s2.eqEnabled,
      eq_preset: 2,
      mica_enabled: s2.micaEnabled,
    };
    await inv('save_settings', { settings: legacy });
    const migrated = await inv('get_settings', {});
    ws.close();
    line('S06', migrated.crossfadeSeconds === 3.5 && migrated.uiScale === 0.9 && migrated.eqPreset === 2,
      `snake_case 旧载荷兼容读：crossfade=${migrated.crossfadeSeconds} uiScale=${migrated.uiScale} eq=${migrated.eqPreset}`);
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
