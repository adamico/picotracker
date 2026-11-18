#!/usr/bin/env python3
"""Analysis for repeated notes (b), Pattern length (c/d), SPD 0 (s) and interpolation of Waveform Instruments."""
import json, math, os, struct, sys, wave

S = os.path.dirname(os.path.abspath(__file__))
C = S + "/p8home/carts/"
TICK = 183


def load(p):
    w = wave.open(p)
    d = w.readframes(w.getnframes())
    return [x / 32768 for x in struct.unpack("<%dh" % (len(d) // 2), d)]


def onset(x, th=0.004):
    for i, v in enumerate(x):
        if abs(v) > th:
            return i
    return None


def rms(x):
    return math.sqrt(sum(v * v for v in x) / max(len(x), 1))


def logs(name):
    out = {}
    for l in open(f"{S}/{name}.log", errors="replace"):
        if l.startswith("L "):
            p = l.split()
            out.setdefault(p[1], []).append([int(float(v)) if v not in ("true", "false") else int(v == "true") for v in p[2:]])
    return out


def section_b():
    print("=== B: repeated note vs one held note (same pitch, same total length)")
    print(f"{'case':10} {'signal rms':>10} {'max |diff|':>10} {'rms diff':>9}  verdict")

    def cmp(a, b, n, label, bounds):
        x, y = load(C + a + ".wav"), load(C + b + ".wav")
        ox, oy = onset(x), onset(y)
        best = None
        for lag in range(-40, 41):
            d = [x[ox + i] - y[oy + lag + i] for i in range(0, 4000)]
            r = rms(d)
            if best is None or r < best[0]:
                best = (r, lag)
        lag = best[1]
        L = min(n, len(x) - ox - 1, len(y) - oy - lag - 1)
        d = [x[ox + i] - y[oy + lag + i] for i in range(L)]
        m = max(abs(v) for v in d)
        at = max(range(L), key=lambda i: abs(d[i]))
        near = min(abs(at - k * bounds) for k in range(0, L // bounds + 2))
        print(f"{label:10} {rms(x[ox:ox + L]):10.4f} {m:10.4f} {rms(d):9.5f}  worst diff at sample {at} ({near} from a note boundary)")

    for w in range(8):
        if w == 6:
            x = load(C + f"b_rept{w}.wav")
            o = onset(x)
            env = [rms(x[o + i * 1464: o + (i + 1) * 1464]) for i in range(8)]
            edge = [rms(x[o + i * 1464 - 60: o + i * 1464 + 60]) for i in range(1, 8)]
            print(f"wave 6     noise: step rms {min(env):.3f}..{max(env):.3f}, rms at boundaries {min(edge):.3f}..{max(edge):.3f}")
            continue
        cmp(f"b_held{w}", f"b_rept{w}", 8 * 1464, f"wave {w}", 1464)
    for w in (0, 2, 3):
        cmp(f"b_pheld{w}", f"b_prept{w}", 16 * 1464, f"2 Pat w{w}", 1464)


def steps(x, o, step, cnt):
    th = 0.01
    return "".join("#" if rms(x[o + i * step: o + (i + 1) * step]) > th else "." for i in range(cnt))


def section_c(logname, title):
    print(f"\n=== {title}")
    meta = json.load(open(C + "meta.json"))
    L = logs(logname)
    print(f"{'item':18} {'ticks':>6} {'frames':>6}  {'expected':30} audible steps of 8 ticks")
    for nm, rows in L.items():
        first = rows[0][9]
        cur = [r for r in rows if r[9] == first]
        ended = len(cur) < len(rows)
        ticks = max(r[11] for r in cur)
        fr = len(cur)
        x = load(C + nm + ".wav")
        o = onset(x)
        vis = steps(x, o, 8 * TICK, 34) if o is not None else "(silent)"
        tag = "" if ended else " (never left the Pattern)"
        print(f"{nm:18} {ticks:6d} {fr:6d}  {str(meta[nm].get('expect') or ''):30} {vis}{tag}")


def section_s():
    print("\n=== S: SPD 0")
    L = logs("expd")
    for nm, rows in L.items():
        x = load(C + nm + ".wav")
        o = onset(x)
        last = max((i for i, v in enumerate(x) if abs(v) > 0.004), default=0)
        dur = (last - o) / TICK if o is not None else 0
        first = rows[0][9]
        cur = [r for r in rows if r[9] == first]
        trace = [r[5] for r in rows[:14]]
        print(f"{nm:18} sounding {dur:7.1f} ticks | Pattern ticks max {max(r[11] for r in cur):5d} over {len(cur):3d} frames | ch0 note index by frame {trace}")


def section_interp():
    print("\n=== Waveform Instrument: resampling model against a_base (period 84.31 samples)")
    x = load(C + "a_base.wav")
    o = onset(x)
    w = x[o + 1500: o + 1500 + 4000]
    v = [0, 0x20, 0x40, 0x7f, -0x80, -0x60, -0x40, -1]
    s = [v[i // 8] / 128 for i in range(64)]
    p = 22050 / (440 * 2 ** ((24 - 33) / 12))
    res = {}
    for kind in ("nearest", "linear"):
        best = None
        for k in range(0, 340):
            ph = k / 340 * 64
            m = []
            for i in range(len(w)):
                t = (ph + i * 64 / p) % 64
                a = int(t)
                m.append(s[a] if kind == "nearest" else s[a] + (s[(a + 1) % 64] - s[a]) * (t - a))
            g = sum(a * b for a, b in zip(w, m)) / sum(b * b for b in m)
            r = rms([a - g * b for a, b in zip(w, m)])
            if best is None or r < best[0]:
                best = (r, g, ph)
        res[kind] = best
        print(f"  {kind:8} residual rms {best[0]:.4f}  gain {best[1]:.3f}  phase {best[2]:.1f}")


which = sys.argv[1:] or ["b", "c", "s", "i"]
if "i" in which:
    section_interp()
if "b" in which:
    section_b()
if "c" in which:
    section_c("expc", "C/D: Pattern length (ticks = highest stat(56) seen in the Pattern)")
if "s" in which:
    section_s()
