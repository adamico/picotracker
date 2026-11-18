#!/usr/bin/env python3
"""Follow-up Cart: what a Pattern does when one of its channels holds an SFX with byte 66 bit 7 set."""
import json, os, sys

src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen.py")).read().split("META = {}")[0]
exec(src)

g = Cart("expg")
aud = g.add(line([n(C3, 3, 5)] * 16, spd=8, ls=16))
aud2 = g.add(line([n(G3, 1, 5)] * 16, spd=8, ls=16))
wf = g.add(line([n(E3, 2, 5)] * 16, spd=8, ls=0x80))
# SFX 0 is a real Waveform Instrument
g.sfx[0] = "01108000" + "".join(["20350"] * 32)


def seq(name, pats, frames):
    first = None
    for i, ch in enumerate(pats):
        p = g.pattern(ch, flags=4 if i == len(pats) - 1 else 0)
        first = p if first is None else first
    g.item(name, f"m({first})", frames, {"expect": "patterns from " + str(first)})


seq("g_left", [[wf, aud]], 60)
seq("g_right", [[aud, wf]], 60)
seq("g_left_wi0", [[0, aud]], 60)
seq("g_right_wi0", [[aud, 0]], 60)
seq("g_only", [[wf]], 30)
seq("g_middle", [[aud], [wf], [aud2]], 130)
seq("g_middle_mixed", [[aud], [wf, aud2], [aud]], 130)
seq("g_first", [[wf], [aud]], 80)

meta = json.load(open(os.path.join(OUT, "meta.json")))
meta.update(g.write())
json.dump(meta, open(os.path.join(OUT, "meta.json"), "w"), indent=0)
