# Changelog

Each release has a `## X.Y.Z` section. The release workflow uses that section as the release notes.

## 0.1.0

The first release of pikutracker, a keyboard-driven desktop tracker for composing PICO-8 music.

- Compose in Song, Chain and Phrase panes shown together, with 8 Tracks to try parts before choosing up to 4 Export Tracks.
- Write Steps with PICO-8's 8 Built-in Instruments, or design SFX Instruments and Waveform Instruments in a full-screen Instrument view.
- Hear the Project through an emulation of PICO-8's sound, checked against recordings of real PICO-8, with Follow, Mute and Solo.
- Import the SFX and Patterns of a `.p8` or `.p8.png` Cart, or a Standard MIDI File, into a Project.
- Save the Project as one standalone `.ptrk` file.
- Export into the Claim of a Target Cart. Export writes only inside the Claim and leaves the Cart's code, graphics, map and other SFX and Patterns untouched; if anything does not fit, it changes nothing and lists every problem.
- Undo and redo every change, Clone and Deep clone Chains and Phrases, and copy, cut, paste and nudge selections.
- Switch to the M8-style key preset or rebind keys in a hand-edited `config.ron`.

### Known limits

- Linux x86_64 only, and ALSA (`libasound`) is required.
- No installer and no desktop launcher entry: unpack the archive and run the binary.
- A Cart does not record which Projects claim it. When two Projects Export into one Cart, give them separate Claims by hand: a slot one Project claims but leaves empty looks free to the other. If they overlap, the next Export asks before overwriting the changed bytes.
