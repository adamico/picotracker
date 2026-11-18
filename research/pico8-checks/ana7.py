#!/usr/bin/env python3
"""Is a filtered Waveform Instrument take further from the unfiltered takes than unfiltered takes are from each other?"""
import importlib.util, itertools, os, statistics as st, sys, io, contextlib

S = os.path.dirname(os.path.abspath(__file__))
src = open(S + "/ana6.py").read().split('print("=== I:')[0]
exec(src)

T = range(1, 7)
ctrl = {t: load(f"m{t}_ctrl") for t in T}
sc = {t: spectra(x) for t, x in ctrl.items()}
pairs = list(itertools.combinations(T, 2))
fw = [residual(ctrl[a], ctrl[b]) for a, b in pairs]
fs = [dist(sc[a], sc[b]) for a, b in pairs]
print(f"unfiltered vs unfiltered ({len(pairs)} pairs)  waveform mean {st.mean(fw):.4f} max {max(fw):.4f}   spectrum mean {st.mean(fs):.4f} max {max(fs):.4f}")
for nm in ("noiz", "buzz", "both"):
    w, s = [], []
    for t in T:
        x = load(f"m{t}_{nm}")
        sx = spectra(x)
        for u in T:
            w.append(residual(x, ctrl[u]))
            s.append(dist(sx, sc[u]))
    print(f"{nm} vs unfiltered ({len(w)} pairs)  waveform mean {st.mean(w):.4f} max {max(w):.4f}   spectrum mean {st.mean(s):.4f} max {max(s):.4f}")
