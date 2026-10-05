# M12.5 L3 真实格式 fixture 生成器（Phase 3）。
#
# 生成含 ID3v2.3/2.4、内嵌封面、Unicode 文件名、超长标签的 mp3/flac/ogg
# 到 tests/fixtures/（TEST ONLY：ffmpeg lavfi 合成正弦，无版权问题）。
# 依赖：ffmpeg（winget Gyan.FFmpeg 全量构建，含 libmp3lame/libvorbis）。
#
# 用法：python scripts/gen-fixtures.py
# 消费：scripts/verify-library.mjs（复制到临时目录 → scan_folder → 断言）。

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tests" / "fixtures"
FFMPEG = "ffmpeg"  # PATH 中可发现（winget 安装）

COVER = OUT / "_cover.png"


def run(*args: str) -> None:
    r = subprocess.run([FFMPEG, "-hide_banner", "-loglevel", "error", "-y", *args])
    if r.returncode != 0:
        sys.exit(f"ffmpeg 失败：{' '.join(args)}")


def audio_args(seconds: int) -> list[str]:
    # 440Hz 正弦，确定性合成（lavfi 无随机性）；采样率内联在 lavfi 参数里
    return [
        "-f", "lavfi", "-i", f"sine=frequency=440:sample_rate=44100:duration={seconds}",
    ]


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)

    # 封面：64x64 纯色 PNG（ffmpeg 生成，无外部资源）
    run("-f", "lavfi", "-i", "color=c=steelblue:s=64x64", "-frames:v", "1", str(COVER))

    cover_map = ["-i", str(COVER), "-map", "0:a", "-map", "1:v",
                 "-c:v", "copy", "-disposition:v", "attached_pic"]
    out_opts = ["-ar", "44100", "-ac", "2"]
    meta = ["-metadata", "title=Fixture曲目", "-metadata", "artist=测试艺术家",
            "-metadata", "album=加固计划合集", "-metadata", "date=2026"]

    # 1) ID3v2.3 + 内嵌封面
    run(*audio_args(3), *cover_map, *out_opts, *meta,
        "-c:a", "libmp3lame", "-b:a", "128k", "-id3v2_version", "3", "-write_id3v1", "0",
        str(OUT / "fixture-id3v2.3.mp3"))

    # 2) ID3v2.4 + 内嵌封面
    run(*audio_args(3), *cover_map, *out_opts, *meta,
        "-c:a", "libmp3lame", "-b:a", "128k", "-id3v2_version", "4", "-write_id3v1", "0",
        str(OUT / "fixture-id3v2.4.mp3"))

    # 3) 无标签裸流
    run(*audio_args(2), "-map_metadata", "-1", *out_opts,
        "-c:a", "libmp3lame", "-b:a", "128k", "-id3v2_version", "0",
        str(OUT / "fixture-notags.mp3"))

    # 4) 超长标签（8KB 标题，测试标签解析健壮性）
    long_title = "长" * 4096
    run(*audio_args(2), *cover_map, *out_opts,
        "-metadata", f"title={long_title}", "-metadata", "artist=长标签",
        "-c:a", "libmp3lame", "-b:a", "128k", "-id3v2_version", "3",
        str(OUT / "fixture-longtag.mp3"))

    # 5) Unicode 文件名 + 中文元数据
    run(*audio_args(3), *cover_map, *out_opts,
        "-metadata", "title=文件名测试曲目🎵", "-metadata", "artist=王奕承测试",
        "-c:a", "libmp3lame", "-b:a", "128k", "-id3v2_version", "3",
        str(OUT / "文件名测试🎵.mp3"))

    # 6) FLAC（Vorbis comment + PICTURE 块）
    run(*audio_args(3), *cover_map, *out_opts, *meta,
        "-c:a", "flac",
        str(OUT / "fixture.flac"))

    # 7) OGG Vorbis（有标签无封面）
    run(*audio_args(2), *out_opts, *meta,
        "-c:a", "libvorbis", "-q:a", "4",
        str(OUT / "fixture.ogg"))

    COVER.unlink(missing_ok=True)
    print(f"fixtures → {OUT}")
    for f in sorted(OUT.iterdir()):
        print(f"  {f.name}  {f.stat().st_size} bytes")


if __name__ == "__main__":
    main()
