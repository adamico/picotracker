# pikutracker

A keyboard-driven desktop tracker for writing PICO-8 music. You compose in a Project with more room than a Cart has, then Export the music into a PICO-8 Cart.

The Song → Chain → Phrase workflow and the optional M8-style keys come from the Dirtywave M8, so the [M8 operation manual](https://dirtywave.com/pages/resources-downloads) is a good introduction. Some things work differently from the M8:

- The Song, Chain and Phrase panes are all on one screen.
- In each Song row, the Chains on every Track start together, because PICO-8 plays its 4 channels that way.
- There is no note-off or hold marker. To hold a note, repeat it on the next Steps, as in PICO-8.

What pikutracker does:

- Song, Chain and Phrase panes on one screen, with 8 Tracks to try parts in before you choose the 4 that go to PICO-8's channels.
- Export writes only the SFX and Patterns the Project claims, so the Cart's code, graphics, map and other sounds stay as they are.
- Import the music of a `.p8` or `.p8.png` Cart, or a Standard MIDI File.
- Playback emulates PICO-8's sound chip and is checked against recordings of real PICO-8.

## Install

Releases are for Linux x86_64 and need ALSA (`libasound`). Download the archive from [GitHub Releases](https://github.com/adamico/pikutracker/releases), then:

```sh
tar -xzf pikutracker-X.Y.Z-x86_64-linux.tar.gz
./pikutracker-X.Y.Z-x86_64-linux/pikutracker
```

To build from source, run `cargo run --release -p pikutracker`.

## Getting started

When nothing else is shown, the status line at the bottom lists the main keys: Ctrl+N new, Ctrl+O open, Ctrl+S save, Ctrl+I import, Ctrl+E export, Space play. Ctrl+S saves the Project as one `.ptrk` file. Ctrl+E Exports it into a Cart.

## Settings

No settings file is needed. To use the M8-style key preset or rebind keys, copy `config.example.ron` to `~/.config/pikutracker/config.ron` (or `$XDG_CONFIG_HOME`) and edit the copy. The file explains each option.

pikutracker keeps Follow, the keyboard octave, MIDI table edits and your recent Projects in `~/.local/state/pikutracker/state.ron` (or `$XDG_STATE_HOME`).

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE); you may use it under either. Third-party notices are in `THIRD_PARTY_LICENSES`, and `pikutracker --licenses` prints them.

pikutracker is an independent project, not affiliated with or endorsed by Lexaloffle or Dirtywave, and it contains no PICO-8 code.
