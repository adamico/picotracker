#!/usr/bin/env python3
"""Waveform Instrument analysis: pitch, per-segment output levels, text round trip."""
import math, os, re, struct, sys, wave

S = os.path.dirname(os.path.abspath(__file__))
C = S + "/p8home/carts/"


def load(p):
    w = wave.open(p)
    d = w.readframes(w.getnframes())
    return [x / 32768 for x in struct.unpack("<%dh" % (len(d) // 2), d)]


def onset(x, th=0.01):
    for i, v in enumerate(x):
        if abs(v) > th:
            return i
    return None


def period(x, lo=20, hi=800):
    """best period (samples) by normalised autocorrelation, refined by parabola"""
    n = len(x) - hi
    best, bl, cs = -9, 0, {}
    e0 = sum(v * v for v in x[:n]) or 1
    for l in range(lo, hi):
        c = sum(x[i] * x[i + l] for i in range(0, n, 2)) * 2 / e0
        cs[l] = c
    # first lag whose correlation is close to the global maximum (avoids octave errors)
    top = max(cs.values())
    for l in range(lo + 1, hi - 1):
        if cs[l] >= 0.93 * top and cs[l] >= cs[l - 1] and cs[l] >= cs[l + 1]:
            bl = l
            break
    a, b, c = cs[bl - 1], cs[bl], cs[bl + 1]
    d = a - 2 * b + c
    return bl + (0.5 * (a - c) / d if d else 0), cs[bl]


def fold(x, p, bins=64):
    acc, cnt = [0.0] * bins, [0] * bins
    for i, v in enumerate(x):
        b = int((i % p) / p * bins) % bins
        acc[b] += v
        cnt[b] += 1
    return [a / c if c else 0 for a, c in zip(acc, cnt)]


def rms(x):
    return math.sqrt(sum(v * v for v in x) / max(len(x), 1))


names = ["a_noflag", "a_base", "a_base36", "a_bass", "a_bass36", "a_flag65", "a_ls85", "a_le55", "a_spd00", "a_spdfe",
         "a_spd02", "a_ed0", "a_noiz", "a_buzz", "a_det1", "a_rev1", "a_dam2"]
base = None
print(f"{'item':10} {'rms':>6} {'period':>8} {'Hz':>7} {'corr':>5} {'diff vs a_base':>14}")
seg = {}
for nm in names:
    x = load(C + nm + ".wav")
    o = onset(x)
    if o is None:
        print(nm, "SILENT")
        continue
    w = x[o + 1500:o + 1500 + 6000]
    p, c = period(w)
    if nm == "a_base":
        base = w
        seg[nm] = fold(w, p)
    if nm in ("a_base36", "a_bass", "a_bass36"):
        seg[nm] = fold(w, p)
    d = ""
    if base is not None:
        # best-lag difference against the base recording
        bestd = min(rms([a - b for a, b in zip(w[l:l + 3000], base[:3000])]) for l in range(0, 200))
        d = f"{bestd:.4f}"
    print(f"{nm:10} {rms(w):6.3f} {p:8.2f} {22050 / p:7.1f} {c:5.2f} {d:>14}")

print("\nExpected Hz: key 24 = %.1f, key 36 = %.1f" % (440 * 2 ** ((24 - 33) / 12), 440 * 2 ** ((36 - 33) / 12)))
print("\nFolded output per 8-sample segment, a_base (bytes 00 20 40 7f 80 a0 c0 ff):")
f = seg["a_base"]
# find rotation: the largest jump is between segment 7 (ff) and 0 (00) or 3 (7f) and 4 (80)
for rot in range(64):
    pass
levels = [sum(f[i * 8 + 2:i * 8 + 6]) / 4 for i in range(8)]
print("  raw (unrotated, phase unknown):", " ".join(f"{v:+.3f}" for v in levels))
print("  all 64 bins:", " ".join(f"{v:+.2f}" for v in f))

# text round trip
log = open(S + "/expa.log").read()
hexes = re.search(r"RTBYTES (\S+)", log).group(1)
mem = [int(h, 16) for h in re.findall(r"0x00([0-9a-f]{2})\.0000", hexes)]
txt = [l for l in open(C + "wavout.p8").read().split("__sfx__\n")[1].splitlines()][0]
hdr = [int(txt[i:i + 2], 16) for i in range(0, 8, 2)]
notes = [txt[8 + i * 5:13 + i * 5] for i in range(32)]
dec = []
for t in notes:
    p, ins, v, e = int(t[:2], 16), int(t[2], 16), int(t[3], 16), int(t[4], 16)
    word = p | (ins & 7) << 6 | v << 9 | e << 12 | (ins >> 3) << 15
    dec += [word & 255, word >> 8]
print("\nText round trip: %d memory bytes" % len(mem))
print("  header text", [hex(v) for v in hdr], "memory", [hex(v) for v in mem[64:68]], "match", hdr == mem[64:68])
print("  samples decoded with the normal note packing match memory:", dec == mem[:64])
print("  line:", txt[:48], "...")
