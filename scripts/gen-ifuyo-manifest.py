#!/usr/bin/env python3
"""生成 ifuyo Aura 远程音乐库清单（aura-manifest.json）。

用法（首次）：
    python3 gen-ifuyo-manifest.py /www/wwwroot/ifuyo.com/audio \\
        --base-url https://ifuyo.com/audio

定时增量更新（服务器 cron 用；也可直接调用本脚本）：
    python3 gen-ifuyo-manifest.py <目录> --base-url <https 前缀>

清单是**静态文件**，服务器只需能按 Range 提供它（nginx/Caddy 默认都行）。
元数据优先用 mutagen，其次 ffprobe，都没有则退化为「文件名 + 大小 + 扩展名」
（此时 title=文件名，artist/album 为 null——客户端会如实显示，不猜）。

兼容 Python 3.6（CentOS 7 / 宝塔自带 python3 就是 3.6.8）：不要用
`from __future__ import annotations`（3.7+）与 `list[str]` 内建泛型标注
（3.9+），否则服务器上直接 SyntaxError。用 typing.List/Dict/Tuple。

增量：默认读已有的 `--out`，对 (大小, mtime) 未变的路径**直接复用**旧元数据，
不再逐首探测——千首库定时重扫的开销因此接近于零。`--no-reuse` 可强制全量。

字段即本文件 track 对象的键（version=1）；客户端 ifuyo 库模块落地时按这里的
形状实现（当前客户端尚未实现，勿引用不存在的文件）。
"""

import argparse
import datetime as dt
import json
import mimetypes
import os
import shutil
import subprocess
import sys
import urllib.parse
from typing import Dict, List, Tuple

AUDIO_EXT = {".mp3", ".flac", ".ogg", ".oga", ".m4a", ".aac", ".wav", ".aiff", ".aif", ".opus"}

# 混进音乐目录的系统/回收目录（Windows 卷标目录、宝塔备份/缩略图缓存等）。
SKIP_DIRS = {"$RECYCLE.BIN", "System Volume Information", "node_modules", "__pycache__"}


def probe_mutagen(path: str) -> dict:
    from mutagen import File  # type: ignore[import-not-found]

    f = File(path, easy=True)
    if f is None:
        return {}
    tags = f.tags or {}

    def first(key: str):
        v = tags.get(key)
        return (v[0] if isinstance(v, (list, tuple)) and v else v) if v else None

    out = {}
    title, artist, album = first("title"), first("artist"), first("album")
    if title:
        out["title"] = str(title)
    if artist:
        out["artist"] = str(artist)
    if album:
        out["album"] = str(album)
    if getattr(f, "info", None) and f.info.length:
        out["durationSeconds"] = round(float(f.info.length), 2)
    return out


def probe_ffprobe(path: str) -> dict:
    exe = shutil.which("ffprobe")
    if not exe:
        return {}
    cmd = [
        exe, "-v", "quiet", "-print_format", "json",
        "-show_format", "-show_streams", path,
    ]
    try:
        # 显式 utf-8：Windows 默认按本地码（gbk）解码 ffprobe 输出会抛 UnicodeDecodeError
        raw = subprocess.run(
            cmd, capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=20
        ).stdout
        data = json.loads(raw or "{}")
    except Exception:  # noqa: BLE001 - 探测失败只是少元数据，不影响出清单
        return {}
    tags = (data.get("format") or {}).get("tags") or {}
    out = {}
    for src, dst in (("title", "title"), ("artist", "artist"), ("album", "album")):
        if tags.get(src):
            out[dst] = str(tags[src])
    dur = (data.get("format") or {}).get("duration")
    if dur:
        try:
            out["durationSeconds"] = round(float(dur), 2)
        except ValueError:
            pass
    return out


def audio_files(root: str) -> List[str]:
    """音频文件绝对路径；跳过隐藏/系统目录与点开头文件（回收站、缩略图缓存、
    Windows 卷标目录混进音乐目录时不该变成"曲目"）。"""
    found: List[str] = []
    for dirpath, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if not d.startswith(".") and d not in SKIP_DIRS]
        for name in sorted(files):
            if name.startswith("."):
                continue
            if os.path.splitext(name)[1].lower() in AUDIO_EXT:
                found.append(os.path.join(dirpath, name))
    return found


def load_cache(path: str, base_url: str) -> Dict[str, dict]:
    """已有清单 → {相对路径: 元数据+etag}；损坏/缺字段一律忽略（退化为全量探测）。"""
    try:
        with open(path, encoding="utf-8") as fh:
            old = json.load(fh)
        prefix = base_url.rstrip("/") + "/"
    except Exception:  # noqa: BLE001 - 没有可用旧清单不是错误
        return {}
    cache: Dict[str, dict] = {}
    for t in old.get("tracks", []):
        url = t.get("url") or ""
        if not url.startswith(prefix):
            continue
        rel = urllib.parse.unquote(url[len(prefix):])
        etag = t.get("etag")
        if not rel or not etag:
            continue
        cache[rel] = {
            "etag": etag,
            "title": t.get("title"),
            "artist": t.get("artist"),
            "album": t.get("album"),
            "durationSeconds": t.get("durationSeconds"),
        }
    return cache


def probe(full: str, cache: Dict[str, dict], rel: str, etag: str) -> Tuple[dict, str]:
    """(元数据, 来源)。来源 ∈ cache/mutagen/ffprobe/filename——缓存命中即不碰解码器。"""
    hit = cache.get(rel)
    if hit is not None and hit.get("etag") == etag:
        return {k: hit[k] for k in ("title", "artist", "album", "durationSeconds") if hit.get(k) is not None}, "cache"
    try:
        meta = probe_mutagen(full)
        mode = "mutagen"
    except Exception:  # noqa: BLE001 - 探测失败只是少元数据，不影响出清单
        meta = {}
        mode = "filename"
    if not meta:
        meta = probe_ffprobe(full)
        mode = "ffprobe" if meta else "filename"
    return meta, mode


def scan(root: str, base_url: str, cache: Dict[str, dict]) -> Tuple[List[dict], dict]:
    tracks: List[dict] = []
    modes: Dict[str, int] = {}
    for full in audio_files(root):
        rel = os.path.relpath(full, root).replace(os.sep, "/")
        st = os.stat(full)
        etag = f"{st.st_size:x}-{int(st.st_mtime):x}"
        ext = os.path.splitext(full)[1].lower()
        # 同名 .lrc 才算歌词：远程清单没有"目录里顺手找一下"这回事。
        lrc_rel = os.path.splitext(rel)[0] + ".lrc"
        lrc_url = (
            base_url.rstrip("/") + "/" + urllib.parse.quote(lrc_rel, safe="/-_.~")
            if os.path.isfile(os.path.splitext(full)[0] + ".lrc")
            else None
        )
        meta, mode = probe(full, cache, rel, etag)
        modes[mode] = modes.get(mode, 0) + 1
        stem = os.path.splitext(os.path.basename(full))[0]
        tracks.append({
            # id 用相对路径的稳定 hash 前缀：改名即换 id（可接受——
            # 远程库无本地身份体系，客户端按 url 去重）。
            "id": urllib.parse.quote(rel, safe="/-_.")[:120],
            "title": meta.get("title") or stem,
            "artist": meta.get("artist"),
            "album": meta.get("album"),
            "url": base_url.rstrip("/") + "/" + urllib.parse.quote(rel, safe="/-_.~"),
            "durationSeconds": meta.get("durationSeconds"),
            "sizeBytes": st.st_size,
            "format": ext.lstrip("."),
            "mimeType": mimetypes.guess_type(full)[0] or "application/octet-stream",
            "etag": etag,
            # 无同名 .lrc 时为 null；客户端据此决定要不要再去取歌词。
            "lyricsUrl": lrc_url,
        })
    return tracks, modes


def write_atomic(out: str, manifest: dict) -> None:
    """先写同目录临时文件再 rename：读者永远只会看到完整清单（rename 在同一
    文件系统上是原子的）。定时任务尤其需要——否则 cron 写一半时客户端正好
    拉清单，会拿到截断的 JSON。"""
    tmp = f"{out}.tmp-{os.getpid()}"
    with open(tmp, "w", encoding="utf-8") as fh:
        json.dump(manifest, fh, ensure_ascii=False, indent=2)
    os.replace(tmp, out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("root", help="服务器上存放音频的目录")
    ap.add_argument("--base-url", required=True, help="该目录对外的 https 前缀")
    ap.add_argument("--out", default=None, help="输出路径（默认 <root>/aura-manifest.json）")
    ap.add_argument("--no-reuse", action="store_true", help="忽略已有清单，全量重新探测元数据")
    args = ap.parse_args()

    out = args.out or os.path.join(args.root, "aura-manifest.json")
    cache = {} if args.no_reuse else load_cache(out, args.base_url)
    tracks, modes = scan(args.root, args.base_url, cache)
    manifest = {
        "version": 1,
        "generatedAt": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "baseUrl": args.base_url,
        "tracks": tracks,
    }
    write_atomic(out, manifest)
    print(f"wrote {out}: {len(tracks)} tracks; 元数据来源 {modes or '无'}")
    if modes.get("filename"):
        print(f"  注意：{modes['filename']} 首仅有文件名（未装 mutagen/ffprobe）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
