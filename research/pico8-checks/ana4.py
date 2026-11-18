#!/usr/bin/env python3
"""LEN on channels that don't set the Pattern's length (exph), and whether a bit-7 SFX sounds beside other channels (expg)."""
import math, os, struct, wave

S = os.path.dirname(os.path.abspath(__file__))
C = S + "/p8home/carts/"


def load(p):
    w = wave.open(p)
    d = w.readframes(w.getnframes())
    return [v / 32768 for v in struct.unpack("<%dh" % (len(d) // 2), d)]


def on(x):
    return next(i for i, v in enumerate(x) if abs(v) > 0.004)


def rms(x):
    return math.sqrt(sum(v * v for v in x) / max(len(x), 1))


rows = {}
for l in open(S + "/exph.log", errors="replace"):
    if l.startswith("L "):
        p = l.split()
        rows.setdefault(p[1], []).append([int(float(v)) for v in p[2:]])
print("=== LEN with audible notes after the LEN point")
for nm, r in rows.items():
    x = load(C + nm + ".wav")
    o = on(x)
    last = max(i for i, v in enumerate(x) if abs(v) > 0.004)
    cur = [q for q in r if q[9] == r[0][9]]
    print(f"{nm:20} Pattern ticks {max(q[11] for q in cur):4d}  sounding {(last - o) / 183:6.1f} ticks")

print("\n=== Bit-7 SFX beside another channel (a large difference means it is audible)")
ref = load(C + "g_middle.wav")
r0 = on(ref)
N = 16 * 8 * 183 - 400
for nm in ("g_left", "g_right", "g_left_wi0", "g_right_wi0"):
    x = load(C + nm + ".wav")
    o = on(x)
    best = min(rms([x[o + l + i] - ref[r0 + i] for i in range(0, N, 3)]) for l in range(-30, 31))
    print(f"{nm:12} rms {rms(x[o:o + N]):.4f}  difference from the other channel alone {best:.4f}")
