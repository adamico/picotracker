#!/usr/bin/env python3
"""Analysis for gen5.py: expi, expj, expk."""
import math, os, struct, wave

S = os.path.dirname(os.path.abspath(__file__))
C = S + "/p8home/carts/"
TICK = 183


def load(nm):
    w = wave.open(C + nm + ".wav")
    d = w.readframes(w.getnframes())
    return [v / 32768 for v in struct.unpack("<%dh" % (len(d) // 2), d)]


def on(x, th=0.004):
    return next((i for i, v in enumerate(x) if abs(v) > th), 0)


def rms(x):
    return math.sqrt(sum(v * v for v in x) / max(len(x), 1))


def rows(log):
    r = {}
    for l in open(S + "/" + log + ".log", errors="replace"):
        if l.startswith("L "):
            p = l.split()
            r.setdefault(p[1], []).append([int(float(v)) for v in p[2:]])
    return r


def diff(a, b):
    oa, ob = on(a), on(b)
    n = min(len(a) - oa, len(b) - ob, 16 * 8 * TICK) - 40
    return min(rms([a[oa + l + i] - b[ob + i] for i in range(0, n, 2)]) for l in range(-20, 21))


def hf(x):
    o = on(x)
    y = x[o : o + 16 * 8 * TICK]
    return rms([y[i + 1] - y[i] for i in range(len(y) - 1)]) / max(rms(y), 1e-9)


print("=== I: filters on the Phrase. diff = RMS difference from the same notes with no filter")
for pre in ("wi", "wim", "si", "tri", "noise"):
    ref = load(pre + "_ctrl")
    print(f"{pre:6} level {rms(ref[on(ref):on(ref) + 16 * 8 * TICK]):.4f}  brightness {hf(ref):.3f}")
    for nm in ("ctrl2", "noiz", "buzz", "both"):
        x = load(f"{pre}_{nm}")
        print(f"   {nm:6} diff {diff(x, ref):.4f}  brightness {hf(x):.3f}")


def profile(x, o, ticks, step=32):
    return " ".join(f"{rms(x[o + t * TICK:o + (t + step) * TICK]) * 100:4.1f}" for t in range(0, ticks, step))


def report(log, title, width):
    print("\n=== " + title)
    for nm, r in rows(log).items():
        x = load(nm)
        o = on(x)
        last = max((i for i, v in enumerate(x) if abs(v) > 0.004), default=o)
        first = r[0][9]
        cur = [q for q in r if q[9] == first]
        moved = len(cur) < len(r)
        ticks = max(q[11] for q in cur)
        print(f"{nm:14} Pattern {'' if moved else '>='}{ticks:5d} ticks  sound ends at {(last - o) / TICK:7.1f}  moved on: {moved}")
        print(f"{'':14} level per 32 ticks: {profile(x, o, width)}")


report("expj", "J: bit 7 SFX in a Pattern longer than its 32 notes", 576)
report("expk", "K: bit 7 SFX beside channels that all loop", 576)
