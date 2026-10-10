#!/usr/bin/env bash
# ifuyo Aura 远程音乐库清单定时更新（服务器侧，Linux）。
#
# 放在服务器上执行，例：
#   mkdir -p /opt/aura && cp gen-ifuyo-manifest.py aura-manifest-cron.sh /opt/aura/
#   chmod +x /opt/aura/aura-manifest-cron.sh
#   MUSIC_DIR=/www/wwwroot/ifuyo.com/audio \
#   BASE_URL=https://ifuyo.com/audio /opt/aura/aura-manifest-cron.sh
#
# crontab（每 30 分钟；宝塔「计划任务」选 Shell 脚本填同样一行）：
#   */30 * * * * MUSIC_DIR=/www/wwwroot/ifuyo.com/audio BASE_URL=https://ifuyo.com/audio /opt/aura/aura-manifest-cron.sh >> /var/log/aura-manifest.log 2>&1
#
# 设计要点（都是"定时任务特有的坑"）：
#   1. 先生成到 staging，校验通过才 mv 上线——校验失败时线上仍是上一份好清单。
#   2. cp 旧清单进 staging 再跑生成器：生成器据 staging 复用未变曲目的元数据，
#      所以增量跑完仍只碰新/改过的文件。
#   3. flock -n：上一轮没跑完就直接跳过本轮，不并发改同一份清单。
set -euo pipefail

MUSIC_DIR="${MUSIC_DIR:?请设置 MUSIC_DIR（音频目录，即站点里的 audio 目录）}"
BASE_URL="${BASE_URL:?请设置 BASE_URL（该目录对外的 https 前缀，末尾不带斜杠）}"
MANIFEST_NAME="${MANIFEST_NAME:-aura-manifest.json}"
PYTHON="${PYTHON:-python3}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GEN="${GEN:-$HERE/gen-ifuyo-manifest.py}"
LOCK="${LOCK:-/tmp/aura-manifest.lock}"

log() { printf '%s %s\n' "$(date '+%F %T')" "$*"; }

[ -d "$MUSIC_DIR" ] || { log "FAIL 目录不存在: $MUSIC_DIR"; exit 1; }
[ -f "$GEN" ] || { log "FAIL 找不到生成器: $GEN"; exit 1; }
# 版本预检：CentOS 7 / 宝塔自带 python3 常是 3.6.8。不预检的话，3.7+ 语法
# 会以一句 SyntaxError 冒出来（实际踩过），运维完全看不出是版本问题。
if ! "$PYTHON" -c 'import sys; sys.exit(0 if sys.version_info >= (3, 6) else 1)' 2>/dev/null; then
  ver="$("$PYTHON" -V 2>&1 || echo 未知)"
  log "FAIL 需要 Python 3.6+，当前 $ver。装 3.8：宝塔「软件商店 → Python 项目」，"
  log "     或临时指定解释器 PYTHON=/usr/bin/python3.8 bash $0"
  exit 1
fi

LIVE="$MUSIC_DIR/$MANIFEST_NAME"
STAGING="$MUSIC_DIR/.$MANIFEST_NAME.new.$$"
trap 'rm -f "$STAGING"' EXIT

if command -v flock >/dev/null 2>&1; then
  # 锁住 fd9 指向的空文件；-n = 抢不到锁立刻退出。子命令结束后内核自动放锁。
  exec 9>"$LOCK"
  if ! flock -n 9; then
    log "SKIP 上一轮仍在运行（$LOCK）"
    exit 0
  fi
fi

# 旧清单作为元数据缓存的种子；首轮没有旧文件则全量探测。
: > "$STAGING"
[ -f "$LIVE" ] && cp "$LIVE" "$STAGING"

if ! "$PYTHON" "$GEN" "$MUSIC_DIR" --base-url "$BASE_URL" --out "$STAGING"; then
  log "FAIL 生成器退出码非 0"
  exit 1
fi

# 上线前校验：必须是合法 JSON、version=1、tracks 为数组。宁可不更新，
# 也不能把截断/畸形的清单挂给客户端。
if ! "$PYTHON" - "$STAGING" <<'PY'
import json, sys
m = json.load(open(sys.argv[1], encoding="utf-8"))
assert m.get("version") == 1, f"version={m.get('version')}"
assert isinstance(m.get("tracks"), list), "tracks 不是数组"
assert all(t.get("url") for t in m["tracks"]), "存在缺 url 的条目"
print(f"ok {len(m['tracks'])} tracks")
PY
then
  log "FAIL 清单校验未通过，保持线上旧版"
  exit 1
fi

# 同目录 rename = 原子替换；客户端要么拿到旧的完整清单，要么新的完整清单。
mv -f "$STAGING" "$LIVE"
trap - EXIT
log "DONE $(wc -c <"$LIVE") bytes → $LIVE"
