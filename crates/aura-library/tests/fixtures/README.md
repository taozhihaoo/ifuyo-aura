# M3 测试 Fixtures

| 文件 | 格式 | 来源 | 许可 | 预期 metadata |
|---|---|---|---|---|
| `sine_8k.flac` | FLAC, 8kHz mono 0.25s | `generate_fixtures.py`（合成 440Hz 正弦波） | 项目自产，无版权材料 | 无标签 |
| `sine_8k.mp3` | MP3, 8kHz mono 0.25s | 同上（libsndfile 1.2 MPEG） | 同上 | 无标签 |
| `sine_8k.ogg` | OGG/Vorbis, 8kHz mono 0.25s | 同上 | 同上 | 无标签 |
| `sine_8k.wav` | WAV PCM16, 8kHz mono 0.25s | 同上 | 同上 | 无标签 |

- 生成工具：python `soundfile` 0.14（libsndfile 绑定），仅作本地生成工具，非项目依赖。
- 标签由测试运行时用 `lofty` 附加到临时副本（完整/缺失/Unicode/空/多封面场景）。
- 畸形文件（截断、伪扩展名）由测试在运行时以字节操作构造，不提交。
- `flac_real_v1.flac`（aura-core M1）为同一生成管线的 48kHz 变体。
