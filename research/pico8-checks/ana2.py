#!/usr/bin/env python3
"""Click metric with a positive control, slide curves, resampling fit and the follow-up Cart."""
import json, math, os, struct, sys, wave

S = os.path.dirname(os.path.abspath(__file__))
C = S + "/p8home/carts/"
Z = S + "/spike_out/"
TICK = 183


def load(p):
    w = wave.open(p)
    d = w.readframes(w.getnframes())
    return [x / 32768 for x in struct.unpack("<%dh" % (len(d) // 2), d)]


def onset(x, th=0.004):
    return next((i for i, v in enumerate(x) if abs(v) > th), None)


def rms(x):
    return math.sqrt(sum(v * v for v in x) / max(len(x), 1))


LAG = 295  # 7 periods of C-3 (42.14 samples) is 294.99 samples


def click(x, o, step, count):
    """largest periodicity error within 40 samples of a note boundary / the same measure mid-note"""
    r = lambda c, h=40: max(abs(x[i] - x[i - LAG]) for i in range(c - h, c + h))
    edge = [r(o + k * step) for k in range(1, count)]
    mid = [r(o + k * step + step // 2) for k in range(1, count)]
    return max(edge), sorted(mid)[len(mid) // 2]


def section_click():
    print("=== Click test: periodicity error at note boundaries against mid-note (1.0 = no click)")
    print(f"{'case':24} {'boundary':>9} {'mid-note':>9} {'ratio':>6}")
    for w in (0, 1, 2, 3, 4, 5, 7):
        x = load(C + f"b_rept{w}.wav")
        e, m = click(x, onset(x), 1464, 8)
        print(f"{'repeat, wave ' + str(w):24} {e:9.4f} {m:9.4f} {e / max(m, 1e-4):6.2f}")
    for w in (0, 2, 3):
        x = load(C + f"b_prept{w}.wav")
        o = onset(x)
        e, m = click(x, o, 1464, 16)
        p = max(abs(x[i] - x[i - LAG]) for i in range(o + 11712 - 40, o + 11712 + 40))
        print(f"{'2 Patterns, wave ' + str(w):24} {e:9.4f} {m:9.4f} {e / max(m, 1e-4):6.2f}   at the Pattern boundary {p:.4f}")
    # positive control: restart the phase at each boundary of the held recording
    for w in (0, 3):
        x = load(C + f"b_held{w}.wav")
        o = onset(x)
        y = list(x[:o])
        for k in range(8):
            y += x[o + 2000: o + 2000 + 1464]
        y += [0.0] * 2000
        e, m = click(y, o, 1464, 8)
        print(f"{'CONTROL phase reset w' + str(w):24} {e:9.4f} {m:9.4f} {e / max(m, 1e-4):6.2f}")


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


def section_slide():
    print("\n=== Slide curves, PICO-8 against the zepto8 port (semitones, best time shift applied)")
    for nm in ("e_plain_slide", "e_i1_slide"):
        a, b = load(C + nm + ".wav"), load(Z + nm + ".wav")
        oa, ob = onset(a), onset(b)
        ta = [pitch(a, oa + t * 92) for t in range(0, 250)]
        tb = [pitch(b, ob + t * 92) for t in range(0, 250)]
        best = None
        for sh in range(-12, 13):
            d = [abs(p - q) for p, q in zip(ta[12:238], tb[12 + sh:238 + sh]) if p is not None and q is not None]
            if len(d) > 100:
                v = sum(d) / len(d)
                if best is None or v < best[0]:
                    best = (v, sh, max(d))
        print(f"  {nm:14} mean error {best[0]:.2f} st, worst {best[2]:.2f} st, shift {best[1] * 0.5:+.1f} ticks")
    a, b = load(C + "e_plain_slide.wav"), load(C + "e_i1_slide.wav")
    oa, ob = onset(a), onset(b)
    ta = [pitch(a, oa + t * 92) for t in range(0, 250)]
    tb = [pitch(b, ob + t * 92) for t in range(0, 250)]
    d = [abs(p - q) for p, q in zip(ta, tb) if p is not None and q is not None]
    print(f"  PICO-8 plain note against SFX Instrument note: mean {sum(d) / len(d):.2f} st, worst {max(d):.2f} st")


def section_resample():
    print("\n=== Waveform Instrument resampling (period and phase fitted)")
    x = load(C + "a_base.wav")
    o = onset(x)
    w = x[o + 1500: o + 1500 + 2500]
    v = [0, 0x20, 0x40, 0x7f, -0x80, -0x60, -0x40, -1]
    s = [v[i // 8] / 128 for i in range(64)]
    for kind in ("nearest", "linear"):
        best = None
        for pp in [84.20 + i * 0.01 for i in range(25)]:
            for k in range(0, 128):
                ph = k / 2
                m = []
                for i in range(0, len(w), 2):
                    t = (ph + i * 64 / pp) % 64
                    a = int(t)
                    m.append(s[a] if kind == "nearest" else s[a] + (s[(a + 1) % 64] - s[a]) * (t - a))
                ww = w[::2]
                g = sum(p * q for p, q in zip(ww, m)) / sum(q * q for q in m)
                r = rms([p - g * q for p, q in zip(ww, m)])
                if best is None or r < best[0]:
                    best = (r, g, pp, ph)
        print(f"  {kind:8} residual rms {best[0]:.4f}  gain {best[1]:.3f}  period {best[2]:.2f}")


def section_f():
    print("\n=== Follow-up Cart")
    meta = json.load(open(C + "meta.json"))
    rows = {}
    other = []
    for l in open(S + "/expf.log", errors="replace"):
        if l.startswith("L "):
            p = l.split()
            rows.setdefault(p[1], []).append([int(float(v)) for v in p[2:]])
        elif l.startswith("W"):
            other.append(l.strip())
    print(" ", other)
    print(f"{'item':22} {'music ticks':>11} {'ch0 notes seen':>16} {'sounding ticks':>14}  expected")
    for nm, r in rows.items():
        x = load(C + nm + ".wav")
        o = onset(x)
        snd = (max(i for i, v in enumerate(x) if abs(v) > 0.004) - o) / TICK if o is not None else 0
        first = r[0][9]
        cur = [q for q in r if q[9] == first]
        ticks = max(q[11] for q in cur)
        notes = max(q[5] for q in r)
        print(f"{nm:22} {ticks:11d} {notes:16d} {snd:14.1f}  {meta[nm].get('expect') or ''}")


for sec in sys.argv[1:] or ["click", "slide", "f"]:
    {"click": section_click, "slide": section_slide, "resample": section_resample, "f": section_f}[sec]()
