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

    # 1) ID3v2.3 + 内嵌封面（USLT 歌词由 lofty 工具后写——ffmpeg lyrics
    #    元数据实测不落 USLT 帧）
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

    # 7) LRC 对：mp3（无歌词标签）+ 同名 .lrc（时间戳行）
    run(*audio_args(4), *cover_map, *out_opts, *meta,
        "-c:a", "libmp3lame", "-b:a", "128k", "-id3v2_version", "3",
        str(OUT / "fixture-lrc.mp3"))
    lrc_lines = ["[ti:Fixture曲目]", "[00:00.50]第一句", "[00:01.50]第二句", "[00:02.50]第三句", "[00:03.50]第四句"]
    (OUT / "fixture-lrc.lrc").write_text(chr(10).join(lrc_lines) + chr(10), encoding="utf-8")

    # 7.4) CUE 整轨对：12s FLAC + 6 子轨 ×2s 的同名 .cue
    run(*audio_args(12), *out_opts,
        "-metadata", "album=CUE合集",
        "-c:a", "flac",
        str(OUT / "fixture-cue.flac"))
    cue_lines = ["PERFORMER \"测试艺术家\"", "TITLE \"CUE合集\"",
                 "FILE \"fixture-cue.flac\" WAVE"]
    titles = ["轨道一", "轨道二", "轨道三", "轨道四", "轨道五", "轨道六"]
    for i, name in enumerate(titles):
        mm, ss = divmod(i * 2, 60)
        cue_lines.append(f"  TRACK {i + 1:02d} AUDIO")
        cue_lines.append(f"    TITLE \"{name}\"")
        cue_lines.append(f"    INDEX 01 {mm:02d}:{ss:02d}:00")
    (OUT / "fixture-cue.cue").write_text(chr(10).join(cue_lines) + chr(10), encoding="utf-8")

    # 7.5) GBK 乱码标题载体（有封面；标题随后由 set_tag_latin1 覆盖）
    run(*audio_args(3), *cover_map, *out_opts, *meta,
        "-c:a", "libmp3lame", "-b:a", "128k", "-id3v2_version", "3",
        str(OUT / "fixture-gbk.mp3"))

    # 8) OGG Vorbis（有标签无封面）
    run(*audio_args(2), *out_opts, *meta,
        "-c:a", "libvorbis", "-q:a", "4",
        str(OUT / "fixture.ogg"))

    # GBK 乱码标题 fixture：Latin-1 形态写入 GBK 原字节（老 MP3 场景），
    # 扫描 → mojibake 启发式 → 标题恢复"周杰伦"
    r = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "aura-library",
         "--example", "set_tag_latin1", "--",
         str(OUT / "fixture-gbk.mp3"), "周杰伦"],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        sys.exit(f"set_tag_latin1 失败：{r.stderr[-400:]}")

    # USLT：应用同款 lofty 写入（与读取路径往返一致，D-016 工具口径）
    r = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "aura-library",
         "--example", "set_uslt", "--",
         str(OUT / "fixture-id3v2.3.mp3"),
         chr(10).join(["第一句歌词", "第二句歌词", "第三句歌词"])],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        sys.exit(f"set_uslt 失败：{r.stderr[-400:]}")

    # 9) M15a 新格式：AAC(M4A) / ALAC(M4A) / AIFF
    run(*audio_args(3), *out_opts, *meta,
        "-c:a", "aac", "-b:a", "128k",
        str(OUT / "fixture.m4a"))
    run(*audio_args(3), *out_opts, *meta,
        "-c:a", "alac",
        str(OUT / "fixture-alac.m4a"))
    run(*audio_args(3), *out_opts, *meta,
        "-c:a", "pcm_s16be",
        str(OUT / "fixture.aiff"))

    COVER.unlink(missing_ok=True)
    print(f"fixtures → {OUT}")
    for f in sorted(OUT.iterdir()):
        print(f"  {f.name}  {f.stat().st_size} bytes")


if __name__ == "__main__":
    main()
