# PROTOTYPE — rough objective A/B between the spike and PICO-8 recordings.
# Per item: sounding length, RMS level ratio, and the correlation of the two
# averaged log spectra (1.0 = same timbre balance). Not a pass/fail test: it
# points at items worth listening to.
import sys, wave, struct, math, os

items, out, ref = sys.argv[1], sys.argv[2], sys.argv[3]


def load(p):
    w = wave.open(p)
    rate, n, ch, sw = w.getframerate(), w.getnframes(), w.getnchannels(), w.getsampwidth()
    d = w.readframes(n)
    s = struct.unpack("<%dh" % (len(d) // 2), d)[::ch]
    s = [x / 32768 for x in s]
    if rate != 22050:  # crude decimation if needed
        step = rate / 22050
        s = [s[int(i * step)] for i in range(int(len(s) / step))]
    return s


def trim(s, thr=0.002):
    idx = [i for i, x in enumerate(s) if abs(x) > thr]
    return s[idx[0]:idx[-1] + 1] if idx else []


def rms(s):
    return math.sqrt(sum(x * x for x in s) / len(s)) if s else 0.0


def spectrum(s, n=1024):
    # average magnitude spectrum over frames (naive DFT on a few bins, log-spaced)
    bins = [int(2 ** (k / 4)) for k in range(12, 36)]  # ~ 8..256 of 512 bins
    bins = sorted(set(b for b in bins if b < n // 2))
    acc = [0.0] * len(bins)
    frames = 0
    for start in range(0, max(0, len(s) - n), n * 4):
        f = s[start:start + n]
        for j, b in enumerate(bins):
            re = sum(f[t] * math.cos(2 * math.pi * b * t / n) for t in range(0, n, 2))
            im = sum(f[t] * math.sin(2 * math.pi * b * t / n) for t in range(0, n, 2))
            acc[j] += math.hypot(re, im)
        frames += 1
    return [math.log(a / max(frames, 1) + 1e-6) for a in acc]


def corr(a, b):
    ma, mb = sum(a) / len(a), sum(b) / len(b)
    num = sum((x - ma) * (y - mb) for x, y in zip(a, b))
    den = math.sqrt(sum((x - ma) ** 2 for x in a) * sum((y - mb) ** 2 for y in b))
    return num / den if den else 0.0


print(f"{'item':8} {'spike s':>8} {'p8 s':>8} {'len Δ%':>7} {'rms ratio':>9} {'spec corr':>9}  description")
for line in open(items):
    name, kind, n, desc = line.split(" ", 3)
    sp = trim(load(os.path.join(out, f"spike_{name}.wav")))
    rp = os.path.join(ref, f"ref_{name}.wav")
    if not os.path.exists(rp):
        print(f"{name:8} {len(sp)/22050:8.2f} {'-':>8} {'':>7} {'':>9} {'':>9}  {desc.strip()}  (no reference)")
        continue
    rf = trim(load(rp))
    sl, rl = len(sp) / 22050, len(rf) / 22050
    # sfx items are capped at 4 s (looping SFX), so compare length only when both are shorter
    lend = f"{100 * (sl - rl) / rl:+7.1f}" if rl and not (kind == "sfx" and rl > 3.9) else "   cap"
    rr = rms(sp) / rms(rf) if rms(rf) else float("nan")
    sc = corr(spectrum(sp), spectrum(rf)) if sp and rf else float("nan")
    flag = "  <-- listen" if (abs(math.log(rr)) > math.log(1.5) if rr == rr and rr > 0 else True) or sc < 0.9 else ""
    print(f"{name:8} {sl:8.2f} {rl:8.2f} {lend:>7} {rr:9.2f} {sc:9.3f}  {desc.strip()}{flag}")
