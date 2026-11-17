# Custom Instruments are unlimited in the Project and assigned to SFX 0–7 at Export

A Step's instrument is two hex digits: 00–07 are PICO-8's Built-in Instruments, and 08–FF are Custom Instruments (SFX Instruments or Waveform Instruments). A Custom Instrument has no fixed SFX number in the Project. Export gives each Custom Instrument used by the 4 Exported Tracks a free SFX in 0–7, within the Project's claimed SFX range, and rewrites the Steps' instrument digits to match. We chose this over mirroring PICO-8's 8 fixed slots because it follows ADR 0001: the Project holds more than a Cart can, just as 8 Tracks are Exported as 4 channels, and only what is Exported must fit.

## Consequences

- An Export that uses more than 8 Custom Instruments, or whose claim does not cover enough of SFX 0–7, fails. There is no manual pinning of an instrument to a slot in the MVP.
- Cart Import turns every SFX 0–7 that some note uses as an instrument into a Custom Instrument.
- Filters live on each Phrase, like Speed, because PICO-8 sets them per SFX. A Custom Instrument has its own filters, which get added for each note that uses it, as in PICO-8.
