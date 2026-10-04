// M11 §77/§78：生成 release 构建 overlay 配置（target/tauri.release.conf.json）。
// Fail-closed：TAURI_UPDATER_PUBKEY 缺失 → 非零退出（§78 不伪装生产）。
// 端点：GITHUB_REPOSITORY 存在 → GitHub Releases latest.json（§54 明确
// artifact mapping，不用 latest/download 猜测文件名——latest.json 即
// 官方 updater manifest，指向精确版本 artifact URL）。
// 本地 dry-run：设置 TAURI_UPDATER_PUBKEY（dev 公钥即可）+ 可选
// TAURI_UPDATER_ENDPOINT（本地 fake server，§59）。
import { writeFileSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(path.dirname(process.argv[1]), '..');
const outPath = path.join(root, 'target', 'tauri.release.conf.json');

const pubkey = process.env.TAURI_UPDATER_PUBKEY;
if (!pubkey) {
  console.error('prepare-release-config FAIL — TAURI_UPDATER_PUBKEY 未设置（§78 fail-closed）');
  process.exit(1);
}

let endpoint;
if (process.env.TAURI_UPDATER_ENDPOINT) {
  endpoint = process.env.TAURI_UPDATER_ENDPOINT; // 本地 fake server（仅 dry-run/测试）
} else if (process.env.GITHUB_REPOSITORY) {
  // §54：latest.json 为官方 updater manifest（内含精确版本 artifact URL）。
  endpoint = `https://github.com/${process.env.GITHUB_REPOSITORY}/releases/latest/download/latest.json`;
} else {
  console.error(
    'prepare-release-config FAIL — GITHUB_REPOSITORY 或 TAURI_UPDATER_ENDPOINT 必须其一（端点不可猜测 §54）',
  );
  process.exit(1);
}

if (!endpoint.startsWith('https://') && !endpoint.startsWith('http://127.0.0.1')) {
  console.error(`prepare-release-config FAIL — 端点非 HTTPS 且非本地：${endpoint}（§9）`);
  process.exit(1);
}

// 与 tauri.conf.json 合并的 overlay（tauri build --config 深合并语义）。
const overlay = {
  bundle: {
    createUpdaterArtifacts: true,
  },
  plugins: {
    updater: {
      pubkey,
      endpoints: [endpoint],
    },
  },
};

mkdirSync(path.dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify(overlay, null, 2));
console.log(`prepare-release-config OK — ${outPath}`);
console.log(`  endpoint: ${endpoint}`);
console.log(`  createUpdaterArtifacts: true`);
