// M12.5 诊断：暂停语义验证（1s 淡出版）——
// play → 采样（前进）→ pause → 淡出期间继续出声（state=playing，
// 位置前进）→ ~1.2s 后 state=paused → 位置冻结（§183）。
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
  await invoke('play_track', { trackId: tracks[0].id });
  await sleep(1200);
  const s1 = await invoke('player_snapshot');
  console.log(`PLAYING state=${s1.state} pos=${s1.positionSeconds.toFixed(2)}`);
  await sleep(800);
  const s2 = await invoke('player_snapshot');
  console.log(`PLAYING state=${s2.state} pos=${s2.positionSeconds.toFixed(2)} (Δ=${(s2.positionSeconds - s1.positionSeconds).toFixed(2)})`);

  await invoke('pause');
  // 淡出期间（~1s）state 仍 playing、位置前进（听得见=真实时间）
  await sleep(1400);
  const s3 = await invoke('player_snapshot');
  console.log(`FADED-OUT state=${s3.state} pos=${s3.positionSeconds.toFixed(2)}`);
  const p0 = s3.positionSeconds;
  for (let i = 1; i <= 4; i++) {
    await sleep(700);
    const s = await invoke('player_snapshot');
    const drift = (s.positionSeconds - p0).toFixed(3);
    console.log(`PAUSED +${i * 0.7}s state=${s.state} pos=${s.positionSeconds.toFixed(2)} drift=${drift} ${Math.abs(s.positionSeconds - p0) < 0.05 ? 'OK' : 'FAIL'}`);
  }
  ws.close();
  process.exit(0);
};
run().catch((e) => {
  console.log('ERR', e.message);
  process.exit(1);
});
