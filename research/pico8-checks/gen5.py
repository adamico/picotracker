#!/usr/bin/env python3
"""Last unchecked behaviours:
expi: NOIZ/BUZZ on the Phrase playing a Waveform Instrument note.
expj: a bit 7 SFX in a Pattern longer than its 32 notes.
expk: a bit 7 SFX beside channels that all loop."""
import json, os, sys

src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen.py")).read().split("META = {}")[0]
exec(src)

NOIZ, BUZZ = 2, 4
meta = {}

# ---------------------------------------------------------------- I: NOIZ / BUZZ on the playing Phrase
i = Cart("expi")
i.extra = r'''
local v={0,0x20,0x40,0x7f,0x80,0xa0,0xc0,0xff}
for k=0,63 do poke(0x3200+k,v[k\8+1]) end
poke(0x3240,1) poke(0x3241,16) poke(0x3242,0x80) poke(0x3243,0)
'''
# SFX 1 is an ordinary SFX Instrument, as a comparison for the Waveform Instrument in SFX 0
i.sfx[1] = line([n(C3, 0, 5)] * 32, spd=16)
PITCH = [C2, E2, G2, C3, E3, G3, C4, C3]


def notes(w, c):
    return [n(p, w, 5, 0, c) for p in PITCH for _ in range(2)]


def filt(prefix, w, c):
    for nm, fl in (("ctrl", 0), ("ctrl2", 0), ("noiz", NOIZ), ("buzz", BUZZ), ("both", NOIZ | BUZZ)):
        k = len(i.sfx) + 8  # never reuse: ctrl and ctrl2 must be separate recordings of separate SFX
        i.next = max(i.next, 8)
        s = i.next
        i.next += 1
        i.sfx[s] = line(notes(w, c), spd=8, ls=16, flags=1 | fl)
        i.item(f"{prefix}_{nm}", f"s({s})", 45)
        if prefix == "wi":
            p = i.pattern([s], flags=4)
            i.item(f"wim_{nm}", f"m({p})", 45)


filt("wi", 0, 1)  # Waveform Instrument note
filt("si", 1, 1)  # ordinary SFX Instrument note
filt("tri", 0, 0)  # plain triangle: BUZZ must be audible here
filt("noise", 6, 0)  # plain noise: NOIZ must be audible here
meta.update(i.write())

# ---------------------------------------------------------------- J: bit 7 SFX in a long Pattern
j = Cart("expj")
marker = j.add(line([], spd=8, ls=8))
AUD = [n(C3, 3, 5)] * 32
gov = lambda spd, ln: j.add(line([n(C3, 3, 0)] * 32, spd=spd, ls=ln))


def jt(name, ch, frames=170):
    p = j.pattern(ch)
    j.pattern([marker], flags=4)
    j.item(name, f"m({p})", frames)


b = lambda spd=8, ls=0x80, le=0: j.add(line(AUD, spd=spd, ls=ls, le=le))
jt("j_right", [gov(8, 64), b()])
jt("j_left", [b(), gov(8, 64)])
jt("j_fast", [gov(8, 64), b(spd=4)])
jt("j_slow_gov32", [gov(8, 32), b(spd=4)])
jt("j_le8", [gov(8, 64), b(le=8)])
jt("j_ls88_le90", [gov(8, 64), b(ls=0x88, le=0x90)])
jt("j_ls88_le10", [gov(8, 64), b(ls=0x88, le=0x10)])
jt("j_lsff", [gov(8, 64), b(ls=0xff)])
# A real Waveform Instrument in SFX 0, played directly as a Pattern channel
j.sfx[0] = "01108000" + "".join(["20350"] * 32)
jt("j_wi0", [gov(16, 64), 0], 300)
# Controls: a plain SFX stops at note 31, and a looping one keeps sounding
jt("j_ctrl_plain", [gov(8, 64), j.add(line(AUD, spd=8))])
jt("j_ctrl_loop", [gov(8, 64), j.add(line(AUD, spd=8, ls=0, le=8))])
meta.update(j.write())

# ---------------------------------------------------------------- K: bit 7 SFX beside channels that all loop
k = Cart("expk")
marker = k.add(line([], spd=8, ls=8))
L = lambda le, w=1: k.add(line([n(G3, w, 4)] * 32, spd=8, ls=0, le=le))
B = lambda spd=8, ls=0x80, le=0: k.add(line([n(C3, 3, 5)] * 32, spd=spd, ls=ls, le=le))


def kt(name, ch, frames=300):
    p = k.pattern(ch)
    k.pattern([marker], flags=4)
    k.item(name, f"m({p})", frames)


# Controls: the same looping channels with no bit 7 SFX
kt("k_c_L8", [L(8)])
kt("k_c_L8_L16", [L(8), L(16)])
kt("k_c_L16_L8", [L(16), L(8)])
kt("k_c_x_L8", [None, L(8)])
# The same layouts with a bit 7 SFX added
kt("k_b_L8", [B(), L(8)])
kt("k_L8_b", [L(8), B()])
kt("k_b_L8_L16", [B(), L(8), L(16)])
kt("k_b_L16_L8", [B(), L(16), L(8)])
kt("k_L16_b_L8", [L(16), B(), L(8)])
# Does the bit 7 SFX's own Speed or header change the answer?
kt("k_b4_L8", [B(spd=4), L(8)])
kt("k_b16_L8", [B(spd=16), L(8)])
kt("k_ble8_L8", [B(le=8), L(8)])
kt("k_b88_90_L8", [B(ls=0x88, le=0x90), L(8)])
k.sfx[0] = "01108000" + "".join(["20350"] * 32)
kt("k_wi0_L8", [0, L(8)])
meta.update(k.write())

m = json.load(open(os.path.join(OUT, "meta.json")))
m.update(meta)
json.dump(m, open(os.path.join(OUT, "meta.json"), "w"), indent=0)
