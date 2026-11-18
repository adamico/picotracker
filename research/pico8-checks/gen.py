#!/usr/bin/env python3
"""Generates the driver Carts for 'Check open PICO-8 behaviours against real PICO-8'.
Each Cart records items to <name>.wav and logs per-frame stat() lines to stdout."""
import json, os, sys

OUT = sys.argv[1]
C2, E2, G2, C3, E3, G3, C4 = 24, 28, 31, 36, 40, 43, 48


def n(p=0, w=0, v=0, f=0, c=0):
    return f"{p:02x}{w + 8 * c:x}{v:x}{f:x}"


E = n()


def line(notes, spd=16, ls=0, le=0, flags=1):
    return f"{flags:02x}{spd:02x}{ls:02x}{le:02x}" + "".join((list(notes) + [E] * 32)[:32])


LUA_HEAD = r'''
function r(name,fn,frames)
 extcmd("set_filename",name) extcmd("audio_rec")
 fn()
 for f=0,frames do
  printh("L "..name.." "..f.." "..stat(46).." "..stat(47).." "..stat(48).." "..stat(49).." "..stat(50).." "..stat(51).." "..stat(52).." "..stat(53).." "..stat(54).." "..stat(55).." "..stat(56).." "..(stat(57) and 1 or 0))
  flip()
 end
 music(-1) sfx(-1,-1)
 extcmd("audio_end",1)
 for i=1,6 do flip() end
end
function m(p) return function() music(p) end end
function s(i) return function() sfx(i,0) end end
'''


class Cart:
    def __init__(self, name):
        self.name, self.sfx, self.pat, self.items, self.extra = name, {}, [], [], ""
        self.next = 8

    def add(self, l):
        for k, v in self.sfx.items():
            if v == l and k >= 8:
                return k
        k = self.next
        self.next += 1
        assert k < 64, "too many SFX in " + self.name
        self.sfx[k] = l
        return k

    def pattern(self, ch, flags=0):
        ch = list(ch) + [None] * 4
        b = [(0x40 | 0) if c is None else c for c in ch[:4]]
        self.pat.append(f"{flags:02x} " + "".join(f"{x:02x}" for x in b))
        return len(self.pat) - 1

    def item(self, name, start, frames, meta=None):
        self.items.append((name, start, frames, meta or {}))

    def write(self):
        blank = line([])
        sfx = "\n".join(self.sfx.get(i, blank) for i in range(64))
        music = "\n".join(self.pat) if self.pat else "00 40404040"
        calls = "\n".join(f'r("{a}",{b},{c})' for a, b, c, _ in self.items)
        body = (f"pico-8 cartridge // http://www.pico-8.com\nversion 43\n__lua__\n{LUA_HEAD}\n{self.extra}\n"
                f'{calls}\nprinth("ALLDONE")\nwhile true do flip() end\n__sfx__\n{sfx}\n__music__\n{music}\n\n')
        with open(os.path.join(OUT, self.name + ".p8"), "w") as f:
            f.write(body)
        return {a: {"frames": c, **d} for a, _, c, d in self.items}


META = {}

# ---------------------------------------------------------------- A: Waveform Instrument layout
a = Cart("expa")
a.extra = r'''
function wi(flt,spd,ls,le)
 local v={0,0x20,0x40,0x7f,0x80,0xa0,0xc0,0xff}
 for i=0,63 do poke(0x3200+i,v[i\8+1]) end
 poke(0x3240,flt) poke(0x3241,spd) poke(0x3242,ls) poke(0x3243,le)
end
function p(flt,spd,ls,le,i) return function() wi(flt,spd,ls,le) sfx(i,0) end end
'''
k24 = a.add(line([n(C2, 0, 7, 0, 1)], spd=64, ls=1))
k36 = a.add(line([n(C3, 0, 7, 0, 1)], spd=64, ls=1))
for nm, (flt, spd, ls, le, pl) in {
    "a_noflag": (1, 16, 0, 0, k24), "a_base": (1, 16, 0x80, 0, k24), "a_base36": (1, 16, 0x80, 0, k36),
    "a_bass": (1, 17, 0x80, 0, k24), "a_bass36": (1, 17, 0x80, 0, k36),
    "a_flag65": (1, 0x80, 0, 0, k24), "a_ls85": (1, 16, 0x85, 0, k24), "a_le55": (1, 16, 0x80, 0x55, k24),
    "a_spd00": (1, 0, 0x80, 0, k24), "a_spdfe": (1, 0xfe, 0x80, 0, k24), "a_spd02": (1, 2, 0x80, 0, k24),
    "a_ed0": (0, 16, 0x80, 0, k24), "a_noiz": (3, 16, 0x80, 0, k24), "a_buzz": (5, 16, 0x80, 0, k24),
    "a_det1": (9, 16, 0x80, 0, k24), "a_rev1": (25, 16, 0x80, 0, k24), "a_dam2": (145, 16, 0x80, 0, k24),
}.items():
    a.item(nm, f"p({flt},{spd},{ls},{le},{pl})", 24)
# text round trip: pseudo-random samples through cstore
a.extra += r'''
function rt()
 local x=1
 for i=0,63 do x=(x*73+41)%256 poke(0x3200+i,x) end
 poke(0x3240,145) poke(0x3241,17) poke(0x3242,0x80) poke(0x3243,0)
 local t="" for i=0,67 do t=t..tostr(peek(0x3200+i),true) end
 printh("RTBYTES "..t)
 cstore(0x3200,0x3200,68,"wavout.p8")
 printh("RTSTORED")
end
'''
a.item("a_rt", "rt", 2)
META.update(a.write())

# ---------------------------------------------------------------- B: repeated notes
b = Cart("expb")
for w in range(8):
    h = b.add(line([n(C3, w, 5)], spd=64, ls=1))
    t = b.add(line([n(C3, w, 5)] * 8, spd=8, ls=8))
    b.item(f"b_held{w}", f"s({h})", 28)
    b.item(f"b_rept{w}", f"s({t})", 28)
for w in (0, 2, 3):
    h = b.add(line([n(C3, w, 5)], spd=128, ls=1))
    t = b.add(line([n(C3, w, 5)] * 8, spd=8, ls=8))
    p0 = b.pattern([t, None, None, None])
    b.pattern([t, None, None, None], flags=4)
    b.item(f"b_pheld{w}", f"s({h})", 46)
    b.item(f"b_prept{w}", f"m({p0})", 46)
META.update(b.write())

# ---------------------------------------------------------------- C/D: Pattern length rules
c = Cart("expc")
marker = c.add(line([], spd=8, ls=8))
AUD = lambda k, cnt: [n(k, 3, 5)] * cnt
SIL = lambda cnt: [n(C3, 3, 0)] * cnt


def silent_loop(cnt, spd=8, k=C3):
    return c.add(line(AUD(k, cnt) + [n(0, 0, 0)], spd=spd, ls=cnt, le=cnt + 1))


def ptest(name, ch, frames=100, expect=None):
    p0 = c.pattern(ch)
    c.pattern([marker], flags=4)
    c.item(name, f"m({p0})", frames, {"expect": expect})


gov16 = c.add(line(SIL(16), spd=8, ls=16))
gov32 = c.add(line(SIL(32), spd=8))
aud16 = c.add(line(AUD(G3, 16), spd=8, ls=16))
ptest("c_ctrl_len8", [c.add(line(AUD(C3, 8), spd=8, ls=8)), gov16], 60, "64 (LEN 8 on the left ends it)")
ptest("c_sl8", [silent_loop(8), gov16], 80, "128")
ptest("c_sl1", [silent_loop(1), gov16], 80, "128")
ptest("c_sl15", [silent_loop(15), gov16], 80, "128")
ptest("c_sl31", [silent_loop(31), gov32], 120, "256")
ptest("c_sl8_fastspd", [silent_loop(8, spd=4), gov16], 80, "128")
ptest("c_sl8_slowspd", [silent_loop(8, spd=16), gov16], 80, "128")
ptest("c_sl_two", [silent_loop(8), silent_loop(4), aud16], 80, "128")
ptest("c_sl_three", [silent_loop(8), silent_loop(4), silent_loop(2), aud16], 80, "128")
ptest("c_sl_gapchan", [silent_loop(8), None, gov16], 80, "128")
ptest("c_sl_only", [silent_loop(8)], 150, "all looping: 32 x speed = 256")
# LEN and loop edge cases
aud8len = c.add(line(AUD(G3, 8), spd=8, ls=8))
ptest("d_right_len8", [gov32, aud8len], 120, "256; right channel silent after 8 steps")
ptest("d_right_len8_spd4", [gov32, c.add(line(AUD(G3, 8), spd=4, ls=8))], 120, "256")
ptest("d_left_len40", [c.add(line(AUD(C3, 32), spd=8, ls=40))], 150, "256 or 320?")
ptest("d_left_len32", [c.add(line(AUD(C3, 32), spd=8, ls=32))], 120, "256")
ptest("d_left_len64", [c.add(line(AUD(C3, 32), spd=8, ls=64))], 200, "256 or 512?")
ptest("d_left_len255", [c.add(line(AUD(C3, 32), spd=2, ls=255))], 200, "64 or 510?")
ptest("d_right_len40", [gov32, c.add(line(AUD(G3, 32), spd=8, ls=40))], 120, "256")
ptest("d_ls8_le4", [c.add(line(AUD(C3, 32), spd=8, ls=8, le=4))], 120, "?")
ptest("d_ls8_le8", [c.add(line(AUD(C3, 32), spd=8, ls=8, le=8))], 120, "?")
ptest("d_ls0_le8", [c.add(line(AUD(C3, 32), spd=8, ls=0, le=8))], 120, "all looping: 256")
ptest("d_le40", [c.add(line(AUD(C3, 32), spd=8, ls=0, le=40))], 120, "loop end past 32")
ptest("d_loop_then_len", [c.add(line(AUD(C3, 8), spd=8, ls=0, le=8)), aud8len], 60, "64")
c.write and META.update(c.write())

# ---------------------------------------------------------------- S: SPD 0
d = Cart("expd")
marker = d.add(line([], spd=8, ls=8))
ramp = [n(24 + i, 3, 5) for i in range(32)]
sp0 = d.add(line(ramp, spd=0))
sp1 = d.add(line(ramp, spd=1))
d.item("s_sfx_spd0", f"s({sp0})", 90)
d.item("s_sfx_spd1", f"s({sp1})", 30)
d.item("s_sfx_spd0_len8", f"s({d.add(line(ramp, spd=0, ls=8))})", 60)
for nm, ch, fr in [
    ("s_pat_spd0", [sp0], 90),
    ("s_pat_spd1", [sp1], 30),
    ("s_pat_spd0_len8", [d.add(line(ramp, spd=0, ls=8))], 60),
    ("s_right_spd0", [d.add(line(SIL(32), spd=8)), sp0], 120),
    ("s_loop_spd0_vs8", [d.add(line(ramp, spd=0, ls=0, le=8)), d.add(line(ramp, spd=8, ls=0, le=8))], 150),
    ("s_loop_spd0_only", [d.add(line(ramp, spd=0, ls=0, le=8))], 150),
]:
    p0 = d.pattern(ch)
    d.pattern([marker], flags=4)
    d.item(nm, f"m({p0})", fr)
META.update(d.write())

# ---------------------------------------------------------------- E: SFX Instrument and reverb differences
e = Cart("expe")
# instrument 0: distinct stepped pitches, looping, so a retrigger is visible as a restart
e.sfx[0] = line([n(C2, 1, 6), n(E2, 1, 6), n(G2, 1, 6), n(C3, 1, 6)] * 2, spd=6, ls=0, le=8)
# instrument 1: steady triangle at C-2 (pitch tracking of the played note)
e.sfx[1] = line([n(C2, 0, 6)], spd=8, ls=0, le=1)
# instrument 2: fast arpeggio inside the instrument
e.sfx[2] = line([n(k, 1, 6, 6) for k in (C2, E2, G2, C3)] * 2, spd=16, ls=0, le=8)
# instrument 3: nests instrument 1, then a plain note
e.sfx[3] = line([n(C2, 1, 6, 0, 1), n(G2, 1, 6, 0, 1), n(C3, 1, 6), n(E3, 1, 6)], spd=16, ls=0, le=4)
# instrument 4: slow arpeggio
e.sfx[4] = line([n(k, 1, 6, 7) for k in (C2, E2, G2, C3)] * 2, spd=16, ls=0, le=8)
I = lambda k, i, fx=0, v=5: n(k, i, v, fx, 1)
items = {
    "e_i0_plain": (line([I(C3, 0)] * 6, spd=32, ls=6), 60),
    "e_i0_fx3": (line([I(C3, 0, 3)] * 6, spd=32, ls=6), 60),
    "e_i0_mix": (line([I(C3, 0), I(C3, 0), I(C3, 0, 3), I(C3, 0, 3), I(C3, 0), I(C3, 0)], spd=32, ls=6), 60),
    "e_i0_fx3_pitch": (line([I(C3, 0, 3), I(E3, 0, 3), I(G3, 0, 3), I(C4, 0, 3)], spd=32, ls=4), 44),
    "e_i1_fx3_pitch": (line([I(C3, 1, 3), I(E3, 1, 3), I(G3, 1, 3), I(C4, 1, 3)], spd=32, ls=4), 44),
    "e_i1_slide": (line([I(C3, 1), I(G3, 1, 1), I(C3, 1, 1), I(C4, 1, 1)], spd=32, ls=4), 44),
    "e_plain_slide": (line([n(C3, 0, 5), n(G3, 0, 5, 1), n(C3, 0, 5, 1), n(C4, 0, 5, 1)], spd=32, ls=4), 44),
    "e_i2_arp_c2": (line([I(C2, 2)], spd=96, ls=1), 34),
    "e_i2_arp_c3": (line([I(C3, 2)], spd=96, ls=1), 34),
    "e_i2_arp_g3": (line([I(G3, 2)], spd=96, ls=1), 34),
    "e_i4_arp_c2": (line([I(C2, 4)], spd=96, ls=1), 34),
    "e_i4_arp_c3": (line([I(C3, 4)], spd=96, ls=1), 34),
    "e_i3_nest_c2": (line([I(C2, 3)], spd=96, ls=1), 34),
    "e_i3_nest_c3": (line([I(C3, 3)], spd=96, ls=1), 34),
}
rv = [n(k, 2, 5) for k in (C3, E3, G3, C4)]
for nm, flags in [("r0", 1), ("r1", 25), ("r2", 49)]:
    items[f"e_{nm}_len4"] = (line(rv, spd=16, ls=4, flags=flags), 45)
    items[f"e_{nm}_tail"] = (line(rv + [E] * 12, spd=16, ls=16, flags=flags), 75)
for nm, (l, fr) in items.items():
    idx = e.add(l)
    e.item(nm, f"s({idx})", fr, {"sfx": idx})
META.update(e.write())

with open(os.path.join(OUT, "meta.json"), "w") as f:
    json.dump(META, f, indent=0)
print("items:", len(META), "est seconds:", sum(v["frames"] + 6 for v in META.values()) // 30)
