"""M12 诊断：WASAPI loopback 黑盒录制系统实际输出（v2：显式 loopback 设置）。

用法：
  python capture_loopback.py <seconds> <out.wav>
"""
import sys
import time

import numpy as np
import sounddevice as sd

seconds = float(sys.argv[1]) if len(sys.argv) > 1 else 20
out = sys.argv[2] if len(sys.argv) > 2 else "target/loopback.wav"

wasapi_hosts = [i for i, h in enumerate(sd.query_hostapis()) if "WASAPI" in h["name"]]
if not wasapi_hosts:
    print("NO-WASAPI-HOST")
    sys.exit(1)
host_idx = wasapi_hosts[0]
host = sd.query_hostapis(host_idx)
dev_idx = host["default_output_device"]
dev = sd.query_devices(dev_idx)
sr = int(dev["default_samplerate"])
ch = 2 if dev["max_output_channels"] >= 2 else int(dev["max_output_channels"])
print(f"capturing: host={host['name']} dev={dev_idx} '{dev['name']}' sr={sr} ch={ch} seconds={seconds}", flush=True)

extra = sd.WasapiSettings(loopback=True)
rec = sd.rec(
    int(seconds * sr),
    samplerate=sr,
    channels=ch,
    device=dev_idx,
    dtype="float32",
    extra_settings=extra,
    blocking=False,
)
sd.sleep(int(seconds * 1000))
sd.stop()
time.sleep(0.5)

data = rec
mono = data.mean(axis=1)
peak = float(np.max(np.abs(mono)))
rms = float(np.sqrt(np.mean(mono**2)))
print(f"captured {len(mono)} samples ({len(mono)/sr:.1f}s) peak={peak:.4f} rms={rms:.4f}")

import wave

with wave.open(out, "wb") as w:
    w.setnchannels(data.shape[1])
    w.setsampwidth(2)
    w.setframerate(sr)
    w.writeframes((np.clip(data, -1, 1) * 32767).astype("<i2").tobytes())

top = np.argsort(np.abs(mono))[-8:]
print("top-8 peaks at (s):", sorted(round(int(i) / sr, 3) for i in top)[-8:])
print("saved:", out)
