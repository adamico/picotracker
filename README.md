# picotracker

A keyboard-driven desktop tracker for composing PICO-8 music, with a Song, Chain and Phrase workflow.

picotracker is an independent project. It is not affiliated with or endorsed by Lexaloffle or Dirtywave, and it includes no PICO-8 code.

## Install

Releases are for Linux x86_64. Download the archive from GitHub Releases, unpack it and run the binary:

```sh
tar -xzf picotracker-X.Y.Z-x86_64-linux.tar.gz
cd picotracker-X.Y.Z-x86_64-linux
./picotracker
```

ALSA (`libasound`) is the one runtime requirement. There is no installer and no desktop launcher entry.

## Settings

picotracker runs with no settings file. To switch to the M8-style key preset or rebind keys, copy `config.example.ron` to `$XDG_CONFIG_HOME/picotracker/config.ron` and edit the copy. picotracker reads it at launch and never writes it.

picotracker remembers Follow, the keyboard octave, MIDI table edits and the 10 most recent Projects in `$XDG_STATE_HOME/picotracker/state.ron`. A damaged settings file is renamed with a `.bad` suffix, and picotracker starts on defaults and says so.

## Reference manuals

- [PICO-8 manual](https://www.lexaloffle.com/dl/docs/pico-8_manual.html)
- [M8 operation manual](https://dirtywave.com/pages/resources-downloads)

## Licence

picotracker is licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

Third-party notices are in `THIRD_PARTY_LICENSES`, and `picotracker --licenses` prints them.
