# PROTOTYPE — generates coverage.p8 (synth feature coverage) and ref_driver.p8
# (records PICO-8's own rendering of each item via extcmd audio_rec/audio_end).
#   python3 make_coverage.py
import os

HERE = os.path.dirname(os.path.abspath(__file__))
C3, E3, G3, C4 = 36, 40, 43, 48


def note(key=0, inst=0, vol=0, fx=0, custom=False):
    return f"{key:02x}{(inst + (8 if custom else 0)):x}{vol:x}{fx:x}"


def sfx(notes, speed=16, loop_start=0, loop_end=0, noiz=0, buzz=0, detune=0, reverb=0, dampen=0):
    notes = (notes + [note()] * 32)[:32]
    f = 1 + noiz * 2 + buzz * 4 + detune * 8 + reverb * 24 + dampen * 72
    return f"{f:02x}{speed:02x}{loop_start:02x}{loop_end:02x}" + "".join(notes)


S = {}
desc = {}

# 0: SFX instrument source (used as custom instrument 0): square blip + organ tail, loops 0-4
S[0] = sfx([note(C3, 3, 6), note(C3 + 12, 3, 4), note(C3, 5, 5), note(C3 + 7, 5, 3)], speed=4, loop_start=0, loop_end=4)
desc[0] = "SFX instrument source (square/organ, loop 0-4)"

# 8..15: one waveform each, through every effect (4 notes per effect)
fx_rows = []
for fx, keys in [(0, [C3] * 4), (1, [G3] * 4), (2, [C3] * 4), (3, [C4] * 4),
                 (4, [E3] * 4), (5, [E3] * 4), (6, [C3, E3, G3, C4]), (7, [C3, E3, G3, C4])]:
    fx_rows.append((fx, keys))
for w in range(8):
    notes = []
    for fx, keys in fx_rows:
        for k in keys:
            notes.append(note(k, w, 5, fx))
    S[8 + w] = sfx(notes, speed=10)
    desc[8 + w] = f"waveform {w} through effects 0..7 (4 notes each)"

# a short melody with mixed waveforms, used for the filter tests
mel = [C3, E3, G3, C4, G3, E3, C3, 0]
def melody(inst):
    out = []
    for i, k in enumerate(mel * 4):
        out.append(note(k, inst, 5 if k else 0))
    return out

S[16] = sfx([note(k, 6, 5) for k in [12, 24, 36, 48] * 8], speed=8, noiz=1)
desc[16] = "noise with NOIZ"
S[17] = sfx([note(k, 6, 5) for k in [12, 24, 36, 48] * 8], speed=8, buzz=1)
desc[17] = "noise with BUZZ (brown noise)"
S[18] = sfx([note(k, (i // 4) % 8, 5) for i, k in enumerate(mel * 4)], speed=8, buzz=1)
desc[18] = "all waveforms with BUZZ"
S[19] = sfx([note(k, (i // 4) % 8, 5) for i, k in enumerate(mel * 4)], speed=8, detune=1)
desc[19] = "all waveforms with DETUNE 1"
S[20] = sfx([note(k, (i // 4) % 8, 5) for i, k in enumerate(mel * 4)], speed=8, detune=2)
desc[20] = "all waveforms with DETUNE 2"
S[21] = sfx(melody(2), speed=8, reverb=1)
desc[21] = "saw melody with REVERB 1"
S[22] = sfx(melody(2), speed=8, reverb=2)
desc[22] = "saw melody with REVERB 2 (zepto8 bug, fake-08 fix)"
S[23] = sfx(melody(2), speed=8, dampen=1)
desc[23] = "saw melody with DAMPEN 1"
S[24] = sfx(melody(2), speed=8, dampen=2)
desc[24] = "saw melody with DAMPEN 2"

# 25: SFX instrument: same pitch repeated (no retrigger), a gap (retrigger), pitch change, slide, drop-effect
n25 = [note(C3, 0, 5, custom=True)] * 4 + [note()] + [note(C3, 0, 5, custom=True)] * 3 \
    + [note(E3, 0, 5, custom=True), note(G3, 0, 5, custom=True)] \
    + [note(C4, 0, 5, 1, custom=True)] * 2 + [note(C3, 0, 5, 3, custom=True)] * 4
S[25] = sfx(n25, speed=12)
desc[25] = "SFX instrument 0: repeats, gap, pitch changes, slide, drop (retrigger rule)"

# 26: arpeggio speed halving: speed 8 (halved) vs 27 at speed 9
S[26] = sfx([note(k, 4, 5, 6) for k in [C3, E3, G3, C4] * 4], speed=8)
desc[26] = "fast arp at speed 8 (arp rate halved)"
S[27] = sfx([note(k, 4, 5, 6) for k in [C3, E3, G3, C4] * 4], speed=9)
desc[27] = "fast arp at speed 9"

# 28/29/30: pattern-length rules
S[28] = sfx([note(C3 + i % 12, 1, 4) for i in range(32)], speed=12, loop_start=0, loop_end=8)
desc[28] = "looping SFX (loop 0-8, speed 12)"
S[29] = sfx([note(G3, 3, 4) if i % 4 == 0 else note() for i in range(32)], speed=12, loop_start=12)
desc[29] = "LEN 12 SFX (speed 12)"
S[30] = sfx([note(C4, 0, 4) for i in range(32)], speed=6, loop_start=0, loop_end=16)
desc[30] = "looping SFX (loop 0-16, speed 6)"

# music patterns
P = {
    0: (0x04, [8, 9, 10, 11]),    # stop after: 4 waveforms at once
    1: (0x04, [12, 13, 14, 15]),
    2: (0x04, [28, 29, 0x40 | 2, 0x40 | 3]),  # length = LEN channel (first non-looping)
    3: (0x04, [28, 30, 0x40 | 2, 0x40 | 3]),  # all looping: longest 32*speed
    4: (0x04, [25, 21, 26, 0x40 | 3]),
}
pdesc = {0: "waveforms 0-3 together", 1: "waveforms 4-7 together", 2: "pattern length: loop + LEN (LEN wins)",
         3: "pattern length: all looping (longest)", 4: "SFX instrument + reverb + arp together"}

sfx_lines = [S.get(i, sfx([])) for i in range(max(S) + 1)]
music_lines = []
for i in range(max(P) + 1):
    flags, ch = P[i]
    music_lines.append(f"{flags:02x} " + "".join(f"{c:02x}" for c in ch))

with open(os.path.join(HERE, "coverage.p8"), "w") as f:
    f.write("pico-8 cartridge // http://www.pico-8.com\nversion 43\n__lua__\n-- playback spike coverage cart (prototype)\n")
    f.write("__sfx__\n" + "\n".join(sfx_lines) + "\n__music__\n" + "\n".join(music_lines) + "\n\n")

# items: (name, kind, n)
items = [(f"sfx{i:02d}", "sfx", i) for i in sorted(S) if i != 0] + [(f"music{p}", "music", p) for p in sorted(P)]

lua = ["-- records PICO-8's rendering of each coverage item to <name>.wav in this folder",
       'cls() print("recording reference wavs...")']
for name, kind, n in items:
    lua.append(f'extcmd("set_filename","ref_{name}") extcmd("audio_rec")')
    if kind == "sfx":
        lua.append(f"sfx({n},0) flip() local t=0 while stat(46)>=0 and t<120 do flip() t+=1 end sfx(-1,0)")
    else:
        lua.append(f"music({n}) flip() local t=0 while stat(54)>=0 and t<600 do flip() t+=1 end music(-1)")
    lua.append('for i=1,15 do flip() end extcmd("audio_end",1)')
    lua.append(f'print("{name}")')
lua.append('print("done") stop()')
with open(os.path.join(HERE, "ref_driver.p8"), "w") as f:
    f.write("pico-8 cartridge // http://www.pico-8.com\nversion 43\n__lua__\n" + "\n".join(lua) + "\n")
    f.write("__sfx__\n" + "\n".join(sfx_lines) + "\n__music__\n" + "\n".join(music_lines) + "\n\n")

with open(os.path.join(HERE, "items.txt"), "w") as f:
    for name, kind, n in items:
        f.write(f"{name} {kind} {n} {desc[n] if kind == 'sfx' else pdesc[n]}\n")
print("wrote coverage.p8, ref_driver.p8, items.txt:", len(items), "items")
