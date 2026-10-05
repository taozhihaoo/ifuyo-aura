// M12.5 诊断：停止语义验证（1s 淡出版）——
// play → 前进 → stop → 淡出期间 state=playing、位置前进 → ~1.2s 后
// state=stopped、位置归零（apply_stop seek 解码器归零）。
const CDP = 'http://127.0.0.1:9223';
const run = async () => {
  const targets = await (await fetch(CDP + '/json')).json();
  const page = targets.find((t) => t.type === 'page');
  if (!page) process.exit(1);
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let id = 0;
  const pending = new Map();
  const send = (method, params) => {
    return new Promise((resolve, reject) => {
      const mid = ++id;
      pending.set(mid, { resolve, reject });
      ws.send(JSON.stringify({ id: mid, method, params }));
    });
  };
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
    }
  };
  await new Promise((r) => (ws.onopen = r));

  const invoke = async (cmd, args) => {
    const r = await send('Runtime.evaluate', {
      expression: `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args ?? {})})`,
      awaitPromise: true,
      returnByValue: true,
    });
    return r.exceptionDetails
      ? { threw: r.exceptionDetails.exception?.description ?? r.exceptionDetails.text }
      : r.result.value;
  };
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  const tracks = await invoke('library_list', { sort: 'title', limit: 1, offset: 0 });
  if (!Array.isArray(tracks) || tracks.length === 0) {
    console.log('NO TRACKS — library empty');
    process.exit(1);
  }
  await invoke('play_track', { trackId: tracks[0].id });
  await sleep(1500);
  const s1 = await invoke('player_snapshot');
  console.log(`PLAYING state=${s1.state} pos=${s1.positionSeconds.toFixed(2)}`);

  await invoke('stop');
  // 淡出期间（~1s）state 仍 playing、位置前进（听得见=真实时间）。
  await sleep(500);
  const s2 = await invoke('player_snapshot');
  console.log(`FADING state=${s2.state} pos=${s2.positionSeconds.toFixed(2)} (应仍 playing 且 > ${s1.positionSeconds.toFixed(2)})`);
  const fadingOk = s2.state === 'playing' && s2.positionSeconds > s1.positionSeconds;

  await sleep(1000);
  const s3 = await invoke('player_snapshot');
  console.log(`STOPPED state=${s3.state} pos=${s3.positionSeconds.toFixed(2)} (应 stopped 且归零)`);
  const stoppedOk = s3.state === 'stopped' && s3.positionSeconds < 0.05;

  // 重播：应从 0 重新开始（解码器已 seek 归零）+ 1s 淡入。
  await invoke('play');
  await sleep(600);
  const s4 = await invoke('player_snapshot');
  console.log(`REPLAY state=${s4.state} pos=${s4.positionSeconds.toFixed(2)} (应 playing，从头起)`);
  const replayOk = s4.state === 'playing' && s4.positionSeconds < 1.0;

  console.log(`VERDICT fading=${fadingOk ? 'OK' : 'FAIL'} stopped=${stoppedOk ? 'OK' : 'FAIL'} replay=${replayOk ? 'OK' : 'FAIL'}`);
  ws.close();
  process.exit(fadingOk && stoppedOk && replayOk ? 0 : 1);
};
run().catch((e) => {
  console.log('ERR', e.message);
  process.exit(1);
});
