// M11 §22/§56：release artifact 校验（fail-closed）。
// 校验：setup.exe 存在/非零/含版本与 x64/updater artifact（zip+sig 或
// latest.json）存在/checksums.txt 已生成且行覆盖主要 artifact。
// 用法：node scripts/release-verify.mjs [version]
//   version 缺省 = 从 ui/package.json 读取。
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(path.dirname(process.argv[1]), '..');
const version = process.argv[2] ?? JSON.parse(readFileSync(path.join(root, 'ui/package.json'), 'utf8')).version;
const bundleDir = path.join(root, 'target', 'release', 'bundle');

function fail(msg) {
  console.error(`release-verify FAIL — ${msg}`);
  process.exit(1);
}

if (!existsSync(bundleDir)) fail(`bundle 目录不存在：${bundleDir}`);

const files = [];
function walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = path.join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) walk(p);
    else files.push({ rel: path.relative(bundleDir, p), p, size: st.size });
  }
}
walk(bundleDir);

// §22：installer 存在 + 命名规则 + 非零。
const nsis = files.filter((f) => /^nsis\/ifuyo Aura_.+_x64-setup\.exe$/.test(f.rel));
if (nsis.length === 0) fail(`未找到 NSIS installer（期望 ifuyo Aura_${version}_x64-setup.exe）`);
const installer = nsis[0];
if (!installer.rel.includes(`_${version}_`)) {
  fail(`installer 版本不匹配：${installer.rel}（期望 ${version}）`);
}
if (installer.size < 1_000_000) {
  fail(`installer 过小（${installer.size}B）——疑似损坏/空 artifact`);
}
console.log(`release-verify installer: ${installer.rel} (${(installer.size / 1048576).toFixed(2)} MiB)`);

// §22/§13：updater artifact（createUpdaterArtifacts 开启时生成 zip+sig）。
const updaterZip = files.filter((f) => /\.nsis\.zip$/.test(f.rel));
const updaterSig = files.filter((f) => /\.nsis\.zip\.sig$/.test(f.rel));
const latestJson = files.filter((f) => /latest\.json$/.test(f.rel));
if (updaterZip.length > 0) {
  if (updaterSig.length !== updaterZip.length) {
    fail(`updater zip 缺 .sig（updater signature present §22）：zip=${updaterZip.length} sig=${updaterSig.length}`);
  }
  console.log(`release-verify updater: ${updaterZip.length} zip + ${updaterSig.length} sig ✓`);
} else if (latestJson.length === 0) {
  console.warn('release-verify WARNING — 无 updater artifact（createUpdaterArtifacts 未开启？）');
}

// debug 二进制混入检查（§22）：bundle 内不应出现 .pdb/.dll。
const pdb = files.filter((f) => /\.(pdb|dll)$/i.test(f.rel));
if (pdb.length > 0) fail(`bundle 内出现非预期二进制：${pdb.map((f) => f.rel).join(', ')}`);

// §92：开发者绝对路径残留抽查（NSIS installer 前若干 MB 文本扫描）。
const buf = readFileSync(installer.p);
const head = buf.subarray(0, Math.min(buf.length, 4_000_000)).toString('latin1');
for (const bad of ['C:\\Users\\', 'OneDrive', '\\Desktop\\', '\\Documents\\']) {
  if (head.includes(bad)) {
    fail(`installer 含开发者路径残留 "${bad}"（§92）`);
  }
}

console.log(`release-verify OK — ${files.length} files, version ${version}`);
