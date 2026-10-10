#!/usr/bin/env bash
# Aura 远程音乐库：只读验收（不改任何配置）。本地或服务器执行都行。
#   BASE_URL=https://ifuyo.com/audio bash aura-server-verify.sh
# 退出码 = FAIL 条数。逐条打印 PASS/FAIL，把输出贴回来即可判断。
set -uo pipefail

BASE_URL="${BASE_URL:-https://ifuyo.com/audio}"
MANIFEST_NAME="${MANIFEST_NAME:-aura-manifest.json}"
PYTHON="${PYTHON:-python3}"
MANIFEST_URL="${BASE_URL%/}/$MANIFEST_NAME"
FAILS=0

say() { # $1=PASS|FAIL|INFO  $2=说明  $3=细节
  if [ "$1" = "FAIL" ]; then FAILS=$((FAILS + 1)); fi
  printf '%-4s %-34s %s\n' "$1" "$2" "$3"
}

# 用 GET 取头部而不是 HEAD：部分 CDN/站点会拒绝 HEAD，导致这里假 FAIL。
HDRS="$(curl -sS -D - -o /dev/null "$MANIFEST_URL" || true)"
CODE="$(printf '%s' "$HDRS" | awk 'NR==1{print $2}')"
if [ "$CODE" = "200" ]; then
  say PASS "清单可达" "$MANIFEST_URL"
else
  say FAIL "清单可达" "HTTP=${CODE:-无响应}（先跑 aura-manifest-cron.sh 生成，再确认 nginx 指向该目录）"
  echo; echo "FAILS=$FAILS"; exit "$FAILS"
fi

printf '%s' "$HDRS" | grep -qi '^content-type:.*application/json' \
  && say PASS "content-type" "application/json" \
  || say FAIL "content-type" "缺 application/json（客户端会按文本处理，务必修）"

# 注意 `: *` ——冒号后必然有一个空格。写成 `[^ ]*` 会永远匹不上，
# 于是"合规服务器也报 FAIL"（本地双情形实测抓到过这个假阴性）。
printf '%s' "$HDRS" | grep -qiE 'cache-control: *(no-cache|no-store)' \
  && say PASS "清单不被缓存" "Cache-Control: no-cache" \
  || say FAIL "清单不被缓存" "无 no-cache → 客户端会启发式缓存，新歌不更新（跑 setup 脚本）"

ETAG="$(printf '%s' "$HDRS" | awk 'tolower($1)=="etag:"{gsub(/\r/,"",$2);print $2}')"
if [ -n "$ETAG" ]; then
  C304="$(curl -sS -o /dev/null -w '%{http_code}' -H "If-None-Match: $ETAG" "$MANIFEST_URL")"
  [ "$C304" = "304" ] && say PASS "条件请求省流" "304（$ETAG）" \
    || say FAIL "条件请求省流" "期望 304，实际 $C304"
else
  say INFO "条件请求省流" "无 ETag（nginx 静态默认有；若上了 CDN 需单独确认）"
fi

curl -sS -D - -o /dev/null -H 'Accept-Encoding: gzip' "$MANIFEST_URL" | grep -qi 'content-encoding: *gzip' \
  && say PASS "gzip" "清单压缩传输（千首约 200KB→~40KB）" \
  || say INFO "gzip" "未压缩（能用，只是白跑带宽）"

# 清单结构与 base-url 一致性：--base-url 拼错是这里的头号事故。
FIRST_URL="$("$PYTHON" - "$MANIFEST_URL" "$BASE_URL" <<'PY'
import json, sys, urllib.request
m = json.load(urllib.request.urlopen(sys.argv[1], timeout=15))
assert m.get("version") == 1, f"version={m.get('version')}"
ts = m.get("tracks") or []
assert ts, "tracks 为空（目录里没音频，或扩展名不在白名单）"
base = sys.argv[2].rstrip("/")
bad = [t["url"] for t in ts if not t["url"].startswith(base + "/")]
assert not bad, f"{len(bad)} 条 url 不以 {base}/ 开头（--base-url 与实际地址不一致），例：{bad[0]}"
miss = [t for t in ts if not t.get("title")]
assert not miss, f"{len(miss)} 条缺 title"
print(f"ok {len(ts)} tracks baseUrl={m.get('baseUrl')}", file=sys.stderr)
print(ts[0]["url"])
PY
)" || { say FAIL "清单结构" "校验未通过（见上一行 python 报错）"; echo; echo "FAILS=$FAILS"; exit "$FAILS"; }
say PASS "清单结构" "version=1、tracks 非空、url 前缀与 BASE_URL 一致"

if [ -n "${FIRST_URL:-}" ]; then
  FULL_LEN="$(curl -sS -o /dev/null -w '%{size_download} %{http_code}' "$FIRST_URL")"
  set -- $FULL_LEN
  LEN="$1"; C1="$2"
  [ "$C1" = "200" ] && say PASS "整取（无 Range）" "$FIRST_URL → 200 $LEN bytes" \
    || say FAIL "整取（无 Range）" "HTTP=${C1:-无响应}"

  HEAD_BYTES="$(curl -sS -o /dev/null -w '%{size_download}' -r 0-99 "$FIRST_URL")"
  # 同样用 GET 取状态码：部分站点/CDN 拒绝 HEAD，会把 Range 检查变成假阴性。
  R206="$(curl -sS -D - -o /dev/null -r 0-99 "$FIRST_URL" | awk 'NR==1{print $2}')"
  if [ "$R206" = "206" ] && [ "$HEAD_BYTES" = "100" ]; then
    say PASS "Range 头部" "206 + 恰好 100 字节"
  else
    say FAIL "Range 头部" "HTTP=$R206 下载=${HEAD_BYTES:-?} 字节（期望 206/100；返回 200 说明 Range 被忽略）"
  fi

  if [ "${LEN:-0}" -gt 1 ]; then
    LAST=$((LEN - 1))
    TAIL="$(curl -sS -o /dev/null -w '%{http_code}\t%{size_download}' -r "${LAST}-${LAST}" "$FIRST_URL")"
    set -- $TAIL
    [ "$1" = "206" ] && [ "$2" = "1" ] \
      && say PASS "Range 尾部（拖动进度条用）" "206 + 1 字节" \
      || say FAIL "Range 尾部（拖动进度条用）" "HTTP=${1:-?} 下载=${2:-?}（期望 206/1）"
  fi

  # 防盗链探测：桌面客户端不发 Referer，宝塔开了防盗链就会在这里 403。
  NOREF="$(curl -sS -o /dev/null -w '%{http_code}' -r 0-9 "$FIRST_URL")"
  case "$NOREF" in
    200 | 206) say PASS "无 Referer 可取" "HTTP=$NOREF（防盗链未拦截桌面客户端）" ;;
    *) say FAIL "无 Referer 可取" "HTTP=$NOREF → 宝塔「防盗链」或 WAF 在拦；Aura 不发 Referer，必须关闭" ;;
  esac
fi

# 目录列表不应暴露（nginx 的标题是 "Index of"，一并匹其他常见实现）
IDX="$(curl -sS -r 0-2048 "${BASE_URL%/}/" 2>/dev/null | grep -Ec 'Index of|Directory listing' || true)"
[ "${IDX:-0}" = "0" ] && say PASS "目录列表未暴露" "${BASE_URL%/}/ 无索引页" \
  || say FAIL "目录列表未暴露" "autoindex 开着，全部文件名可见"

# 明文 80：只提示（Aura 允许 http，见 crates/aura-sources/src/url_policy.rs:10）
H1="$(curl -sS -o /dev/null -w '%{http_code}' "http://${BASE_URL#*://}/${MANIFEST_NAME}" 2>/dev/null || echo '?')"
case "$H1" in
  301 | 308) say PASS "HTTP 强制跳转" "$H1" ;;
  *) say INFO "HTTP 强制跳转" "明文 80 返回 $H1（不影响 Aura 播放；建议开宝塔「强制 HTTPS」，避免误用 http 裸奔）" ;;
esac

echo
echo "FAILS=$FAILS"
exit "$FAILS"
