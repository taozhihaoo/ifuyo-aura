"""M11 §59 updater integration fake server（本地 127.0.0.1:19988）。

Phase A：manifest version=当前版本（0.1.0）→ §47 降级/同版本拒绝
         （预期：仅 manifest GET，无 artifact GET）。
Phase B：manifest version=0.2.0（签名仍为真实 0.1.0 setup.exe 的
         minisign 签名）→ 完整回环（artifact GET → 下载 → 安装 →
         重启；§59 Case1）。
所有请求记录到 target/fake-server.log。
"""
import http.server
import json
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NSIS = ROOT / 'target/release/bundle/nsis'
SETUP = NSIS / 'ifuyo Aura_0.1.0_x64-setup.exe'
SIG = NSIS / 'ifuyo Aura_0.1.0_x64-setup.exe.sig'

log_lines = []


class Handler(http.server.BaseHTTPRequestHandler):
    manifest_version = '0.2.0'  # Phase B：新版本（artifact 仍为 0.1.0 签名包）

    def do_GET(self):  # noqa: N802
        log_lines.append(f'GET {self.path}')
        print(f'[srv] GET {self.path}', flush=True)
        if self.path.startswith('/updates/'):
            sig = SIG.read_text(encoding='utf-8')
            body = json.dumps({
                'version': self.manifest_version,
                'pub_date': '2026-10-05T00:00:00Z',
                'platforms': {
                    'windows-x86_64': {
                        'signature': sig,
                        'url': 'http://127.0.0.1:19988/setup.exe',
                    }
                },
            }).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path.startswith('/setup.exe'):
            data = SETUP.read_bytes()
            self.send_response(200)
            self.send_header('Content-Type', 'application/octet-stream')
            self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, *_a):
        pass


if __name__ == '__main__':
    assert SETUP.exists(), SETUP
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 19988), Handler)
    print('[srv] listening 19988', flush=True)
    server.serve_forever()
