#!/usr/bin/env python3
"""Edge-regularity click test for square and pulse, Pattern reverb against the spike, slide shape, bit-7 Patterns."""
import json, math, os, struct, subprocess, wave

S = os.path.dirname(os.path.abspath(__file__))
C = S + "/p8home/carts/"
Z = S + "/spike_out/"
BIN = S + "/spike/prototypes/playback-spike/target/release/playback-spike"
TICK = 183


def load(p):
    w = wave.open(p)
    d = w.readframes(w.getnframes())
    return [x / 32768 for x in struct.unpack("<%dh" % (len(d) // 2), d)]


def onset(x, th=0.004):
    return next((i for i, v in enumerate(x) if abs(v) > th), None)


def edges(x, a, b):
    return [i for i in range(a, b) if x[i - 1] <= 0 < x[i]]


def regular(x, o, n):
    e = edges(x, o + 100, o + n - 100)
    iv = [q - p for p, q in zip(e, e[1:])]
    return min(iv), max(iv), len(iv)


print("=== Rising-edge spacing across note boundaries (a phase reset would break the spacing)")
for w in (3, 4):
    x = load(C + f"b_rept{w}.wav")
    print(f"  repeat wave {w}: spacing {regular(x, onset(x), 8 * 1464)}")
    x = load(C + f"b_prept{w}.wav") if w == 3 else None
    if x:
        print(f"  two Patterns wave {w}: spacing {regular(x, onset(x), 16 * 1464)}")
    h = load(C + f"b_held{w}.wav")
    o = onset(h)
    y = list(h[:o])
    for k in range(8):
        y += h[o + 2000: o + 2000 + 1464]
    y += [0.0] * 500
    print(f"  CONTROL phase reset wave {w}: spacing {regular(y, o, 8 * 1464)}")

print("\n=== Reverb in a Pattern: sounding ticks, PICO-8 against the spike")
for nm, p in (("f_r1_silentloop", 26), ("f_r1_len_right", 28), ("f_r2_silentloop", 30), ("f_r2_len_right", 32)):
    subprocess.run([BIN, C + "expf.p8", "music", str(p), "--wav", Z + nm + ".wav", "--secs", "4"],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
    r = {}
    for k, path in (("p8", C + nm + ".wav"), ("spike", Z + nm + ".wav")):
        x = load(path)
        o = onset(x)
        r[k] = (max(i for i, v in enumerate(x) if abs(v) > 0.003) - o) / TICK
    print(f"  {nm:18} PICO-8 {r['p8']:6.1f}   spike {r['spike']:6.1f}")


def pitch(x, c, win=420):
    seg = x[c: c + win + 170]
    if len(seg) < win + 170 or sum(v * v for v in seg[:win]) / win < 2e-5:
        return None
    e = sum(v * v for v in seg[:win])
    cs = {l: sum(seg[i] * seg[i + l] for i in range(win)) / e for l in range(12, 169)}
    top = max(cs.values())
    for l in range(13, 168):
        if cs[l] >= 0.88 * top and cs[l] >= cs[l - 1] and cs[l] >= cs[l + 1]:
            a, b, d = cs[l - 1], cs[l], cs[l + 1]
            q = a - 2 * b + d
            return 33 + 12 * math.log2(22050 / (l + (0.5 * (a - d) / q if q else 0)) / 440)


print("\n=== Shape of the C-3 to C-4 slide (last note, 32 ticks), semitones above C-3")
print("  tick into note:   " + " ".join(f"{t:5d}" for t in range(2, 32, 4)))
for label, path in (("PICO-8", C), ("zepto8 port", Z)):
    x = load(path + "e_plain_slide.wav")
    o = onset(x)
    print(f"  {label:17} " + " ".join(f"{(pitch(x, o + (96 + t) * TICK - 210) or 0) - 36:5.1f}" for t in range(2, 32, 4)))
print("  linear in pitch   " + " ".join(f"{12 * t / 32:5.1f}" for t in range(2, 32, 4)))
print("  linear in freq    " + " ".join(f"{12 * math.log2(1 + t / 32):5.1f}" for t in range(2, 32, 4)))

print("\n=== Patterns holding an SFX with byte 66 bit 7 set")
rows = {}
for l in open(S + "/expg.log", errors="replace"):
    if l.startswith("L "):
        p = l.split()
        rows.setdefault(p[1], []).append([int(float(v)) for v in p[2:]])
for nm, r in rows.items():
    x = load(C + nm + ".wav")
    o = onset(x)
    snd = (max(i for i, v in enumerate(x) if abs(v) > 0.004) - o) / TICK if o is not None else 0
    seq = []
    for q in r:
        if not seq or seq[-1] != q[9]:
            seq.append(q[9])
    print(f"  {nm:16} patterns visited {str(seq):22} max ticks {max(q[11] for q in r):4d}  sounding {snd:6.1f} ticks  music on at end {r[-1][12]}")
