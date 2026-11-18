# A Step is exactly one PICO-8 note

A Step holds a pitch, an instrument, a volume and one PICO-8 effect (0–7), just like a PICO-8 note. An empty Step is a Step at volume 0, which is silent, and it keeps its pitch and instrument. There is no M8-style hold marker and no OFF note: a note is held by repeating it in the following Steps, as in PICO-8.

We chose this over M8 semantics, where a blank note keeps the previous note sounding, because PICO-8 has no such marker. PICO-8 holds a repeated note smoothly, since SFX instruments retrigger only on a pitch change or after a silent note. A silent note's pitch still matters: effect 1 slides from the previous note's pitch and volume. Sustain semantics would have made Cart Import guess, and dropping the pitch of silent notes would change how Carts sound after a round trip.

## Consequences

- Cart Import and Export map notes one-to-one.
- The editor shows a volume-0 Step as empty (`...`). Faster ways to enter held notes are an editor feature, not part of the model.
- M8-style effect commands that need compiling are left out of the MVP.
- A `.ptrk` file writes a silent Step that holds any data in full, for example `C#2 08 0 3`, so that saving and reloading is exact. The token `--- -- - -` is written only for a Step whose pitch, Instrument, volume and effect are all 0, and it reads back as exactly that Step.
