# The PICO-8 reference set is recorded once and checked in CI

Playback is proven against recordings of real PICO-8 0.2.7. `scripts/record-reference.sh` records the cases on a machine with a licensed PICO-8, and the Carts and their fingerprints are committed. CI never runs PICO-8: it renders each Cart through our synth and compares with the stored fingerprints. We chose this because PICO-8 is proprietary and cannot run in CI, while the stored measurements still make real PICO-8 the source of truth.

## Consequences

- PICO-8 crashes on long recording names, so every driver Cart records to the same name and the record command renames the file.
- A case waiting on unbuilt work must still differ from PICO-8, so the work that makes it match also takes it off every waiting list.
- Exact-output snapshots of our synth run only on Linux x86_64, because floating point can round differently elsewhere.
- Level limits are wider for noise, and wider again for NOIZ, BUZZ and DETUNE 1. Waveform Instruments are left out of level checks because their bytes are samples, not notes.
- Our noise sequence is not PICO-8's. After the owner's A/B listening checks for #54, each noise note, with whatever plays beside it, is held to the level limit on its power-averaged level, and the pitch window of the Tick that crossfades out of noise is not compared. Over many Ticks plain noise's mean level must still match as closely as a tone.
- The Carts in the test support are generated in code from the documented formats. They stand in until Carts re-saved through real PICO-8 exist.
