#!/usr/bin/env python3
"""M3 测试 fixture 生成脚本。

生成 4 个微小的合成音频文件（0.25s 单声道 8kHz 正弦波），无版权内容。
标签（title/artist 等）不在生成时写入 —— 测试用 lofty 在运行时附加，
以便覆盖完整/缺失/Unicode/多封面等场景。

工具：python soundfile（libsndfile 绑定）。仅本地生成工具；
产物为合成正弦波，不包含任何第三方版权材料。

用法：python generate_fixtures.py
"""

import math
import struct

import soundfile as sf

SR = 8000
DUR = 0.25
N = int(SR * DUR)


def sine(freq: float) -> list[float]:
    return [0.5 * math.sin(2 * math.pi * freq * i / SR) for i in range(N)]


def write(name: str, fmt: str) -> None:
    data = sine(440.0)
    sf.write(name, data, SR, format=fmt)
    print(f"{name}: {len(open(name, 'rb').read())} bytes")


if __name__ == "__main__":
    write("sine_8k.flac", "FLAC")
    write("sine_8k.mp3", "MP3")
    write("sine_8k.ogg", "OGG")
    write("sine_8k.wav", "WAV")
