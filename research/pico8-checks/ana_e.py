#!/usr/bin/env python3
"""Pitch traces of PICO-8 recordings against the zepto8-port spike, and reverb tail lengths."""
import json, math, os, struct, subprocess, sys, wave

S = os.path.dirname(os.path.abspath(__file__))
C = S + "/p8home/carts/"
BIN = S + "/spike/prototypes/playback-spike/target/release/playback-spike"
OUT = S + "/spike_out/"
os.makedirs(OUT, exist_ok=True)
TICK = 183
NAMES = "C-,C#,D-,D#,E-,F-,F#,G-,G#,A-,A#,B-".split(",")


def load(p):
    w = wave.open(p)
    d = w.readframes(w.getnframes())
    return [x / 32768 for x in struct.unpack("<%dh" % (len(d) // 2), d)]


def onset(x, th=0.004):
    return next((i for i, v in enumerate(x) if abs(v) > th), None)


def pitch_at(x, c, win=640):
    seg = x[c: c + win + 200]
    if len(seg) < win + 200:
        return None
    e = sum(v * v for v in seg[:win])
    if e / win < 2e-5:
        return None
    cs = {}
    for l in range(10, 200):
        cs[l] = sum(seg[i] * seg[i + l] for i in range(0, win, 2)) * 2 / e
    top = max(cs.values())
    if top < 0.45:
        return None
    for l in range(11, 199):
        if cs[l] >= 0.9 * top and cs[l] >= cs[l - 1] and cs[l] >= cs[l + 1]:
            a, b, d = cs[l - 1], cs[l], cs[l + 1]
            q = a - 2 * b + d
            lag = l + (0.5 * (a - d) / q if q else 0)
            return 33 + 12 * math.log2(22050 / lag / 440)
    return None


def trace(x, ticks, hop=4):
    o = onset(x) or 0
    out = []
    for t in range(0, ticks, hop):
        out.append(pitch_at(x, o + t * TICK))
    return out


def name(k):
    if k is None:
        return "..."
    r = round(k)
    return f"{NAMES[r % 12]}{r // 12}"


def rle(tr, hop=4):
    res, cur, cnt = [], None, 0
    for k in tr:
        n = name(k)
        if n == cur:
            cnt += hop
        else:
            if cur is not None:
                res.append(f"{cur}x{cnt}")
            cur, cnt = n, hop
    res.append(f"{cur}x{cnt}")
    return " ".join(res)


def tail(x, th=0.003):
    o = onset(x)
    last = max(i for i, v in enumerate(x) if abs(v) > th)
    return (last - o) / TICK


meta = json.load(open(C + "meta.json"))
items = [k for k in meta if k.startswith("e_")]
for nm in items:
    idx, fr = meta[nm]["sfx"], meta[nm]["frames"]
    subprocess.run([BIN, C + "expe.p8", "sfx", str(idx), "--wav", OUT + nm + ".wav", "--secs", str(fr / 30 + 0.5)],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)

print("=== Pitch traces (note x ticks; ... = no clear pitch). P8 = real PICO-8, Z8 = zepto8 port")
for nm in items:
    if nm.startswith("e_r"):
        continue
    ticks = int(meta[nm]["frames"] * 4)
    a, b = trace(load(C + nm + ".wav"), ticks), trace(load(OUT + nm + ".wav"), ticks)
    same = sum(1 for p, q in zip(a, b) if name(p) == name(q)) / max(len(a), 1)
    print(f"\n{nm}  (same note in {same * 100:.0f}% of windows)")
    print("  P8:", rle(a))
    print("  Z8:", rle(b))

print("\n=== Reverb tails: sounding length in ticks after the first note starts")
print(f"{'item':12} {'PICO-8':>8} {'spike':>8} {'diff':>7}")
for nm in items:
    if nm.startswith("e_r"):
        p, q = tail(load(C + nm + ".wav")), tail(load(OUT + nm + ".wav"))
        print(f"{nm:12} {p:8.1f} {q:8.1f} {q - p:+7.1f}")
