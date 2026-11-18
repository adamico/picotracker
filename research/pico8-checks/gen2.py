#!/usr/bin/env python3
"""Follow-up Cart: byte 66 bit 7 outside waveforms, loops past note 31, reverb under the silent loop, waveform from text."""
import json, os, sys

src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen.py")).read().split("META = {}")[0]
exec(src)

f = Cart("expf")
marker = f.add(line([], spd=8, ls=8))
AUD = lambda k, cnt: [n(k, 3, 5)] * cnt
SIL = lambda cnt: [n(C3, 3, 0)] * cnt
gov32 = f.add(line(SIL(32), spd=8))


def ptest(name, ch, frames=100, expect=None):
    p0 = f.pattern(ch)
    f.pattern([marker], flags=4)
    f.item(name, f"m({p0})", frames, {"expect": expect})


# can a full 32-Step Phrase on the left stop governing, using a loop that sits past note 31?
ptest("f_32_ctrl", [f.add(line(AUD(C3, 32), spd=4)), gov32], 80, "128: left channel governs")
ptest("f_32_ls32le33", [f.add(line(AUD(C3, 32), spd=4, ls=32, le=33)), gov32], 110, "256 if the loop counts")
ptest("f_32_ls33le34", [f.add(line(AUD(C3, 32), spd=4, ls=33, le=34)), gov32], 110, "256 if the loop counts")
ptest("f_32_ls40le41", [f.add(line(AUD(C3, 32), spd=4, ls=40, le=41)), gov32], 110, "256 if the loop counts")
ptest("f_31_ls31le32", [f.add(line(AUD(C3, 31), spd=4, ls=31, le=32)), gov32], 110, "256: the documented trick")

# byte 66 bit 7 on SFX 8 and up
for v in (127, 128, 129, 200, 255):
    i = f.add(line(AUD(C3, 32), spd=2, ls=v))
    ptest(f"f_len{v}_pat", [i], 90, "?")
    f.item(f"f_len{v}_sfx", f"s({i})", 90)
for ls, le in ((128, 140), (130, 131), (0x90, 0x88)):
    i = f.add(line(AUD(C3, 32), spd=2, ls=ls, le=le))
    ptest(f"f_loop{ls}_{le}_pat", [i], 90, "?")
    f.item(f"f_loop{ls}_{le}_sfx", f"s({i})", 90)

# reverb behind Export's silent loop, against the same Phrase cut by LEN
for nm, flt in (("r1", 25), ("r2", 49)):
    k = [n(p, 2, 5) for p in (C3, E3, G3, C4)]
    sl = f.add(line(k + [n(0, 0, 0)], spd=16, ls=4, le=5, flags=flt))
    ln = f.add(line(k, spd=16, ls=4, flags=flt))
    gov = f.add(line(SIL(16), spd=16, ls=16))
    ptest(f"f_{nm}_silentloop", [sl, gov], 90, "tail rings?")
    ptest(f"f_{nm}_len_right", [gov, ln], 90, "tail cut?")

# Waveform Instrument stored as text in SFX 0, with SFX 1 the same bytes without the flag
samples = [0, 0x20, 0x40, 0x7f, 0x80, 0xa0, 0xc0, 0xff]
bytes_ = [samples[i // 8] for i in range(64)]
words = [bytes_[i] | bytes_[i + 1] << 8 for i in range(0, 64, 2)]
txt = []
for w in words:
    ins = (w >> 6 & 7) | (w >> 15) << 3
    txt.append(f"{w & 63:02x}{ins:x}{w >> 9 & 7:x}{w >> 12 & 7:x}")
f.sfx[0] = "01108000" + "".join(txt)
f.extra = 'function pk() local t="" for i=64,67 do t=t..tostr(peek(0x3200+i),true).." " end printh("WH "..t) local c=0 for i=0,63 do if peek(0x3200+i)!=' \
          + "({0,0x20,0x40,0x7f,0x80,0xa0,0xc0,0xff})[i\\8+1] then c+=1 end end printh(\"WMISMATCH \"..c) end\npk()\n"
f.item("f_wi_text", f"s({f.add(line([n(C2, 0, 7, 0, 1)], spd=64, ls=1))})", 24)
f.item("f_wi_direct_sfx", "s(0)", 40)
ptest("f_wi_direct_pat", [0], 60, "Waveform Instrument placed straight in a Pattern")

meta = json.load(open(os.path.join(OUT, "meta.json")))
meta.update(f.write())
json.dump(meta, open(os.path.join(OUT, "meta.json"), "w"), indent=0)
print("items", len(f.items), "patterns", len(f.pat), "sfx", f.next)
