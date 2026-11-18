#!/usr/bin/env python3
"""expm: repeat takes of the Waveform Instrument notes, to measure the take-to-take noise floor."""
import json, os, sys

src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen.py")).read().split("META = {}")[0]
exec(src)

c = Cart("expm")
c.extra = r'''
local v={0,0x20,0x40,0x7f,0x80,0xa0,0xc0,0xff}
for k=0,63 do poke(0x3200+k,v[k\8+1]) end
poke(0x3240,1) poke(0x3241,16) poke(0x3242,0x80) poke(0x3243,0)
'''
PITCH = [C2, E2, G2, C3, E3, G3, C4, C3]
notes = [n(p, 0, 5, 0, 1) for p in PITCH for _ in range(2)]
FL = {"ctrl": 0, "noiz": 2, "buzz": 4, "both": 6}
for take in range(1, 7):
    for nm, fl in FL.items():
        s = c.next
        c.next += 1
        c.sfx[s] = line(notes, spd=8, ls=16, flags=1 | fl)
        c.item(f"m{take}_{nm}", f"s({s})", 45)

m = json.load(open(os.path.join(OUT, "meta.json")))
m.update(c.write())
json.dump(m, open(os.path.join(OUT, "meta.json"), "w"), indent=0)
