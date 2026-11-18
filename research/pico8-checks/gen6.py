#!/usr/bin/env python3
"""expl: bit 7 SFX beside looping channels whose Speeds differ, so each candidate rule predicts a different length."""
import json, os, sys

src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen.py")).read().split("META = {}")[0]
exec(src)

c = Cart("expl")
marker = c.add(line([], spd=8, ls=8))
L = lambda spd: c.add(line([n(G3, 1, 4)] * 32, spd=spd, ls=0, le=8))
B = lambda spd: c.add(line([n(C3, 3, 5)] * 32, spd=spd, ls=0x80))


def t(name, ch, frames=300):
    p = c.pattern(ch)
    c.pattern([marker], flags=4)
    c.item(name, f"m({p})", frames)


# Controls: looping channels only
t("l_c_L4", [L(4)])
t("l_c_L16", [L(16)])
t("l_c_L4_L8", [L(4), L(8)])
t("l_c_L8_L4", [L(8), L(4)])
t("l_c_L16_L4", [L(16), L(4)])
# The same with a bit 7 SFX whose own 32 notes last a different time
t("l_b8_L4", [B(8), L(4)])
t("l_L4_b8", [L(4), B(8)])
t("l_L8_b4", [L(8), B(4)])
t("l_L8_b16", [L(8), B(16)])
t("l_b4_L16", [B(4), L(16)])
t("l_b16_L4_L8", [B(16), L(4), L(8)])
t("l_b16_L8_L4", [B(16), L(8), L(4)])
t("l_b4_L16_L4", [B(4), L(16), L(4)])

m = json.load(open(os.path.join(OUT, "meta.json")))
m.update(c.write())
json.dump(m, open(os.path.join(OUT, "meta.json"), "w"), indent=0)
