// M11 §A1/§2：版本权威校验（version:check）。
// 单一 ApplicationVersion 规则：以下来源必须一致（MAJOR.MINOR.PATCH）：
//   ui/package.json ↔ src-tauri/tauri.conf.json ↔ src-tauri/Cargo.toml
//   ↔ 最新 app-v* Git tag（存在时）
// 附加：CHANGELOG 含当前版本段（warning——release pipeline 强制）。
// 退出码非 0 = 失败（§80 可重复执行的发布命令语义）。
import { readFileSync, existsSync } from 'node:fs';
import { execSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(path.dirname(process.argv[1]), '..');
const SEMVER = /^\d+\.\d+\.\d+$/;

function fail(msg) {
  console.error(`version:check FAIL — ${msg}`);
  process.exit(1);
}

function readJson(p) {
  return JSON.parse(readFileSync(path.join(root, p), 'utf8'));
}

function cargoVersion(p) {
  const m = readFileSync(path.join(root, p), 'utf8').match(/\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  return m?.[1];
}

function gitTags() {
  if (!existsSync(path.join(root, '.git'))) return [];
  try {
    return execSync('git tag --list "app-v*" --merged HEAD', { cwd: root, encoding: 'utf8' })
      .split('\n')
      .map((t) => t.trim())
      .filter(Boolean)
      .map((t) => t.replace(/^app-v/, ''));
  } catch {
    return [];
  }
}

const sources = [
  ['ui/package.json', readJson('ui/package.json').version],
  ['brand.json', readJson('brand.json').version],
  ['src-tauri/tauri.conf.json', readJson('src-tauri/tauri.conf.json').version],
  ['src-tauri/Cargo.toml', cargoVersion('src-tauri/Cargo.toml')],
];

for (const [name, v] of sources) {
  if (typeof v !== 'string' || !SEMVER.test(v)) {
    fail(`${name} 版本非法："${v}"（需 MAJOR.MINOR.PATCH）`);
  }
}

const versions = sources.map(([, v]) => v);
if (new Set(versions).size !== 1) {
  fail(`版本漂移：${sources.map(([n, v]) => `${n}=${v}`).join(' ')}（§A1 不允许漂移进入 release pipeline）`);
}

// Git tag（存在时必须等于当前版本；允许多个旧 tag，取最高比较）。
const tags = gitTags();
const semverTags = tags.filter((t) => SEMVER.test(t));
if (semverTags.length > 0) {
  const highest = semverTags.sort((a, b) => (a < b ? -1 : 1)).at(-1);
  const v0 = versions[0];
  if (highest !== v0) {
    fail(`Git tag app-v${highest} 与当前版本 ${v0} 不一致（tag 必须匹配已提交版本 §20）`);
  }
  console.log(`version:check tag app-v${highest} = ${v0}`);
} else {
  console.log(`version:check 无 app-v* tag（开发态，跳过 tag 比对）：${versions[0]}`);
}

// §82：CHANGELOG 版本段（warning——release pipeline 强制）。
const changelog = readFileSync(path.join(root, 'CHANGELOG.md'), 'utf8');
if (!changelog.includes(versions[0])) {
  console.warn(`version:check WARNING — CHANGELOG 未包含版本 ${versions[0]}（release pipeline 将强制）`);
}

console.log(`version:check OK — ${versions[0]} (全部来源一致${semverTags.length ? '，tag 匹配' : ''})`);
