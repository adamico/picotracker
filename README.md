# pikutracker

A keyboard-driven desktop tracker for writing PICO-8 music. You compose in a Project with more room than a Cart has, then Export the music into a PICO-8 Cart.

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

## How a Song is built

![The pikutracker screen. On the left, the Song pane: in row 00, Track 1 plays Chain 00 and Track 2 plays Chain 02; in row 01 they play Chains 01 and 03. In the middle, the Chain pane shows Chain 00, which lists Phrases 00, 00 and 01, and under it the Chain rows of Song row 00. On the right, the Phrase pane shows the 8 Steps of Phrase 01, from A-2 down to D-2 and back up to G-2.](docs/images/song-chain-phrase.png)

The screen shows three panes side by side. From left to right:

- **Song**: the whole piece, read from top to bottom. Each Song row names one Chain for each Track, or none. A Song has up to 256 rows.
  - **Track**: one of the Song's 8 columns (T1 to T8), played by one voice. Export sends 4 of them to PICO-8's 4 channels, so you can try parts on the other Tracks before choosing.
- **Chain**: a list of up to 16 Phrases, played in order. The same Phrase can appear more than once, and each entry can transpose it.
  - **Chain row**: entry K of every Track's Chain in the same Song row. They start together and become one PICO-8 Pattern. The table under the Chain pane lists them.
- **Phrase**: 1 to 32 Steps (16 by default) with its own speed. This is where you write notes. A Phrase becomes roughly one PICO-8 SFX; when a Chain transposes it, the transposed copy is exported as its own SFX.
  - **Step**: one row of a Phrase: a note, its instrument, volume and effect. It is the same as one note of a PICO-8 SFX.

In the screenshot, Song row 00 plays Chain 00 on Track 1 and Chain 02 on Track 2. Chain 00 plays Phrase 00 twice, then Phrase 01. The cursor is on that Phrase 01, so the Phrase pane shows its 8 Steps.

The workflow and the optional M8-style keys come from the Dirtywave M8; its [operation manual](https://dirtywave.com/pages/resources-downloads) explains them further. Some things work differently from the M8:

- The Song, Chain and Phrase panes are all on one screen.
- Each Chain row starts on every Track at once, because PICO-8 plays its 4 channels that way.
- There is no note-off or hold marker. To hold a note, repeat it on the next Steps, as in PICO-8.

## Getting started

When nothing else is shown, the status line at the bottom lists the main keys: Ctrl+N new, Ctrl+O open, Ctrl+S save, Ctrl+I import, Ctrl+E export, Space play. Ctrl+S saves the Project as one `.ptrk` file. Ctrl+E Exports it into a Cart.

## Settings

No settings file is needed. To use the M8-style key preset or rebind keys, copy `config.example.ron` to `~/.config/pikutracker/config.ron` (or `$XDG_CONFIG_HOME`) and edit the copy. The file explains each option.

pikutracker keeps Follow, the keyboard octave, MIDI table edits and your recent Projects in `~/.local/state/pikutracker/state.ron` (or `$XDG_STATE_HOME`).

## Licence

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE); you may use it under either. Third-party notices are in `THIRD_PARTY_LICENSES`, and `pikutracker --licenses` prints them.

pikutracker is an independent project. It is not affiliated with or endorsed by Lexaloffle or Dirtywave, and it includes no PICO-8 code.
