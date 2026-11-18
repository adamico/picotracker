#!/usr/bin/env python3
"""LEN on a channel that does not set the Pattern's length, with audible notes after LEN."""
import json, os, sys
src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "gen.py")).read().split("META = {}")[0]
exec(src)
h = Cart("exph")
marker = h.add(line([], spd=8, ls=8))
AUD = lambda k, c: [n(k, 3, 5)] * c
gov = h.add(line([n(C3, 3, 0)] * 32, spd=8))
def t(name, ch, fr=100):
    p = h.pattern(ch); h.pattern([marker], flags=4); h.item(name, f"m({p})", fr)
t("h_right_len8", [gov, h.add(line(AUD(G3, 32), spd=8, ls=8))])
t("h_right_len20", [gov, h.add(line(AUD(G3, 32), spd=8, ls=20))])
t("h_right_len8_spd4", [gov, h.add(line(AUD(G3, 32), spd=4, ls=8))])
t("h_right_len8_spd16", [gov, h.add(line(AUD(G3, 32), spd=16, ls=8))])
t("h_third_len8", [gov, None, h.add(line(AUD(G3, 32), spd=8, ls=8))])
t("h_left_len8_ctrl", [h.add(line(AUD(G3, 32), spd=8, ls=8)), gov], 60)
t("h_noLEN_ctrl", [gov, h.add(line(AUD(G3, 32), spd=8))])
h.item("h_sfx_len8", f"s({h.add(line(AUD(G3, 32), spd=8, ls=8))})", 90)
m = json.load(open(os.path.join(OUT, "meta.json"))); m.update(h.write()); json.dump(m, open(os.path.join(OUT, "meta.json"), "w"))
