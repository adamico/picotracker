#!/usr/bin/env python3
"""Phase-independent comparison for expi, and Pattern lengths for expl."""
import cmath, math, os, struct, wave

S = os.path.dirname(os.path.abspath(__file__))
C = S + "/p8home/carts/"
TICK, NOTE, N = 183, 16 * 183, 2048


def load(nm):
    w = wave.open(C + nm + ".wav")
    d = w.readframes(w.getnframes())
    return [v / 32768 for v in struct.unpack("<%dh" % (len(d) // 2), d)]


def on(x, th=0.004):
    return next((i for i, v in enumerate(x) if abs(v) > th), 0)


def fft(a):
    n = len(a)
    if n == 1:
        return a
    e, o = fft(a[0::2]), fft(a[1::2])
    t = [cmath.exp(-2j * math.pi * k / n) * o[k] for k in range(n // 2)]
    return [e[k] + t[k] for k in range(n // 2)] + [e[k] - t[k] for k in range(n // 2)]


HANN = [0.5 - 0.5 * math.cos(2 * math.pi * i / N) for i in range(N)]


def spectra(x):
    o = on(x)
    out = []
    for k in range(8):  # eight pitches, two Steps each
        s = o + k * NOTE + 500
        out.append([abs(v) for v in fft([complex(x[s + i] * HANN[i]) for i in range(N)])[: N // 2]])
    return out


def dist(a, b):
    tot = 0
    for p, q in zip(a, b):
        tot += math.sqrt(sum((u - v) ** 2 for u, v in zip(p, q)) / sum(v * v for v in q))
    return tot / len(a)


def residual(a, b):
    """Best sample-aligned difference per note, as a fraction of the note's level."""
    oa, ob, tot = on(a), on(b), 0
    for k in range(8):
        sa, sb = oa + k * NOTE + 500, ob + k * NOTE + 500
        y = b[sb : sb + 1000]
        ref = math.sqrt(sum(v * v for v in y))
        tot += min(math.sqrt(sum((a[sa + l + i] - y[i]) ** 2 for i in range(0, 1000))) for l in range(-100, 101)) / ref
    return tot / 8


print("=== I: difference from the unfiltered recording (ctrl2 is the same notes again, so it is the noise floor)")
print(f"{'':7}{'':7}{'spectrum':>10}{'waveform':>10}")
for pre in ("wi", "wim", "si", "tri"):
    x0 = load(pre + "_ctrl")
    s0 = spectra(x0)
    for nm in ("ctrl2", "noiz", "buzz", "both"):
        x = load(f"{pre}_{nm}")
        print(f"{pre:7}{nm:7}{dist(spectra(x), s0):10.4f}{residual(x, x0):10.4f}")

print("\n=== L: Pattern length with looping channels at different Speeds")
rows = {}
for l in open(S + "/expl.log", errors="replace"):
    if l.startswith("L "):
        p = l.split()
        rows.setdefault(p[1], []).append([int(float(v)) for v in p[2:]])
for nm, r in rows.items():
    cur = [q for q in r if q[9] == r[0][9]]
    print(f"{nm:14} {'' if len(cur) < len(r) else '>='}{max(q[11] for q in cur):5d} ticks")
