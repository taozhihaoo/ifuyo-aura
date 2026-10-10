#!/usr/bin/env bash
# Aura 远程音乐库：把清单缓存策略落到宝塔站点上（幂等，可反复执行）。
#
# 服务器上 root 执行：
#   MUSIC_DIR=/www/wwwroot/ifuyo.com/audio \
#   SITE_CONF=/www/server/panel/vhost/nginx/html_ifuyo.com.conf \
#   BASE_URL=https://ifuyo.com/audio \
#   bash aura-server-setup.sh
#
# 设计取舍（都是"改生产 nginx 配置"特有的坑）：
#   * 规则写在独立文件里，站点配置只加一行 include —— 不把你的几百行站点配置
#     交给 sed 改写。宝塔在改站点设置（SSL/伪静态）时会重新生成 vhost 文件，
#     那行 include 可能丢；本脚本幂等，重跑一次就补回来。
#   * **只在能确定插入位置时才动手**：文件里恰好一个顶层 server{} 才注入；
#     有多个 server{}（80 与 443 分块、或一站多域名）时拒绝猜测，改为打印
#     要手工粘的那一行。插错块 = 配置合法但规则没生效，`nginx -t` 也查不出来。
#   * nginx -t 通过才 reload；不通过自动撤销本脚本加的那一行。
#   * 不做 HTTP→HTTPS 跳转：请用宝塔「网站设置 → 强制 HTTPS」，它会为
#     /.well-known/acme-challenge/ 放行；手写 if 可能把证书续期打死。
set -euo pipefail

MUSIC_DIR="${MUSIC_DIR:?音频目录，如 /www/wwwroot/ifuyo.com/audio}"
SITE_CONF="${SITE_CONF:?站点配置文件，如 /www/server/panel/vhost/nginx/html_ifuyo.com.conf}"
BASE_URL="${BASE_URL:-https://ifuyo.com/audio}"
MANIFEST_NAME="${MANIFEST_NAME:-aura-manifest.json}"
PYTHON="${PYTHON:-python3}"
AURA_DIR="${AURA_DIR:-/www/server/panel/vhost/nginx/aura}"
NGINX="${NGINX:-$(command -v nginx 2>/dev/null || echo /www/server/nginx/sbin/nginx)}"

AURA_CONF="$AURA_DIR/manifest.conf"
INCLUDE_LINE="include $AURA_CONF;"
BACKUP=""   # 只有真注入过才有值；失败提示里据此决定是否报备份路径

# ---- 预检：任何一样不满足都在"动手之前"退出，不留半成品 ----
[ -d "$MUSIC_DIR" ] || { echo "FAIL 目录不存在: $MUSIC_DIR"; exit 1; }
[ -f "$SITE_CONF" ] || { echo "FAIL 站点配置不存在: $SITE_CONF"; exit 1; }
[ -x "$NGINX" ] || { echo "FAIL 找不到可执行的 nginx（设 NGINX=/路径/nginx 再来）"; exit 1; }
[ -w "$SITE_CONF" ] || echo "WARN $SITE_CONF 不可写（需 root），仅输出待粘贴的配置"
mkdir -p "$AURA_DIR"

# 清单的 URI 路径 = BASE_URL 的路径部分 + 文件名（避免手抄错）。
MANIFEST_URI="$("$PYTHON" - "$BASE_URL" "$MANIFEST_NAME" <<'PY'
import sys, urllib.parse
u = urllib.parse.urlsplit(sys.argv[1])
print(u.path.rstrip("/") + "/" + sys.argv[2])
PY
)"

# 可选：把音频目录挪出站点部署路径（推荐——站点"清空根目录再上传"这类操作
# 不会连带删掉曲库）。设了它，生成的配置改用 alias 映射，对外 URL 不变。
AUDIO_ALIAS="${AUDIO_ALIAS:-}"

if [ -n "$AUDIO_ALIAS" ]; then
  case "$AUDIO_ALIAS" in
    /*) ;;
    *) echo "FAIL AUDIO_ALIAS 必须是绝对路径（如 /www/ifuyo-audio）"; exit 1 ;;
  esac
  # 若 BASE_URL 不带路径（https://ifuyo.com），alias 就会落到 location /，
  # 那等于把整个站点根交给音频目录 —— 主站立刻被打穿。拒绝这种组合。
  if [ "$MANIFEST_URI" = "/$MANIFEST_NAME" ]; then
    echo "FAIL 用 AUDIO_ALIAS 时 BASE_URL 必须带子路径（如 https://ifuyo.com/audio）"
    echo "     否则要 alias 的就是站点根 /；请改用独立子域名 + 独立 server 块。"
    exit 1
  fi
  AUDIO_ALIAS="${AUDIO_ALIAS%/}/"
  URI_PREFIX="${MANIFEST_URI%/$MANIFEST_NAME}/"
  LOCATIONS="# 音频目录已隔离在 $AUDIO_ALIAS（不随站点部署变动）。
# 两个 location 都得显式 alias：清单那条是精确匹配（= 的优先级高于前缀匹配），
# 若只写前缀 alias，清单请求会走精确块、按站点 root 找文件 → 404。
location = $MANIFEST_URI {
    alias $AUDIO_ALIAS$MANIFEST_NAME;
    gzip on;
    gzip_types application/json;
    gzip_min_length 1k;
    add_header Cache-Control \"no-cache\" always;
}
location $URI_PREFIX {
    alias $AUDIO_ALIAS;
    # 音频已压缩，gzip 只会白烧 CPU，这里不开。
}"
else
  LOCATIONS="location = $MANIFEST_URI {
    gzip on;
    gzip_types application/json;
    gzip_min_length 1k;
    # 注意：本块一旦出现 add_header，就不再继承 server{} 层的 add_header。
    # 若站点开过 HSTS / X-Frame-Options 等安全头，请在这里重抄一份。
    add_header Cache-Control \"no-cache\" always;
}"
fi

umask 022
cat >"$AURA_CONF" <<EOF
# 由 aura-server-setup.sh 生成，勿手改（改脚本后重跑）。
# 清单必须每次回源校验：nginx 对静态文件默认不发 Cache-Control，客户端会按
# 启发式缓存 → 加了新歌但列表一直不更新。no-cache（而不是 no-store）允许留
# 副本但强制校验，配合 nginx 自带的 ETag 命中 304，代价只有几个字节。
$LOCATIONS
EOF
# 目录必须可执行（755）才能被 worker 穿越；只给文件 644。给目录 644 会让
# 清单直接 403——这是"配置看起来没错但取不到"的经典成因。
chmod 755 "$AURA_DIR"
chmod 644 "$AURA_CONF"
echo "wrote $AURA_CONF"
printf '%s\n' "$LOCATIONS" | grep -E '^location' | sed 's/^/  /'

SERVER_BLOCKS="$("$PYTHON" - "$SITE_CONF" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8", errors="surrogateescape").read()
# 顶层 server 块：行首缩进后紧跟 server{ 。注释行先去掉，避免注释里的
# "server {" 被当成块头（宝塔模板常带示例注释）。
body = "\n".join(l for l in text.splitlines(True) if not l.lstrip().startswith("#"))
print(len(re.findall(r"^[ \t]*server[ \t]*\{", body, re.M)))
PY
)"

if grep -qF "$INCLUDE_LINE" "$SITE_CONF"; then
  echo "include 已存在，无需改动"
elif [ "$SERVER_BLOCKS" = "1" ]; then
  BACKUP="$SITE_CONF.bak.$(date +%Y%m%d%H%M%S)"
  cp -a "$SITE_CONF" "$BACKUP"
  echo "backed up → $BACKUP"
  "$PYTHON" - "$SITE_CONF" "$INCLUDE_LINE" <<'PY'
import sys
path, line = sys.argv[1], sys.argv[2]
# newline="" 关键：否则 Windows/文本模式会把整个文件的换行重写一遍，
# 回滚后与原始配置逐字节不等（本地实测过：12 行 → 多出 12 字节 \r）。
text = open(path, encoding="utf-8", errors="surrogateescape", newline="").read()
depth, start = 0, None
nl = "\r\n" if "\r\n" in text[:2000] else "\n"   # 跟随原文件的换行风格
for i, ch in enumerate(text):
    if ch == "{":
        depth += 1
        if depth == 1:
            start = i
    elif ch == "}":
        depth -= 1
        if depth == 0:
            if start is None:
                sys.exit("no top-level block")
            # 本脚本只在"恰好一个 server 块"时被调用：该块收尾即插入点。
            out = text[:i] + "    " + line + nl + text[i:]
            open(path, "w", encoding="utf-8", errors="surrogateescape", newline="").write(out)
            break
else:
    sys.exit("unbalanced braces in " + path)
PY
  echo "已注入 include（位于唯一顶层 server{} 末尾）"
else
  echo "WARN 站点配置里有 $SERVER_BLOCKS 个顶层 server 块，不猜测插入位置。"
  echo "     请在宝塔「网站设置 → 配置文件」中，把下面这一行粘进 **443 那个 server{}**"
  echo "     的末尾（与 location 同级），保存后执行：$NGINX -t && $NGINX -s reload"
  echo ""
  echo "     $INCLUDE_LINE"
  exit 0
fi

if ! "$NGINX" -t; then
  echo "FAIL nginx 配置校验未通过，撤销刚注入的 include"
  if [ -n "$BACKUP" ]; then
    # 有备份就直接逐字节还原，而不是"再改一遍文件"。
    cp -a "$BACKUP" "$SITE_CONF"
    echo "     已从备份逐字节还原：$BACKUP"
  else
    "$PYTHON" - "$SITE_CONF" "$INCLUDE_LINE" <<'PY'
import sys
path, line = sys.argv[1], sys.argv[2]
text = open(path, encoding="utf-8", errors="surrogateescape", newline="").read()
kept = "".join(l for l in text.splitlines(True) if l.strip() != line)
open(path, "w", encoding="utf-8", errors="surrogateescape", newline="").write(kept)
PY
    echo "     已删除注入行（无备份可还原）"
  fi
  exit 1
fi

"$NGINX" -s reload
echo "DONE 已 reload。下一步："
echo "  1) 宝塔「网站设置 → 强制 HTTPS」打开（这里不手写 if，避免打死证书续期）"
echo "  2) 确认「防盗链」关闭 —— 桌面客户端不发 Referer，开了音频会全 403"
echo "  3) 跑 aura-manifest-cron.sh 生成清单，再跑 aura-server-verify.sh 验收"
