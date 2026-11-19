# Spec

The rules the app follows, from [MVP spec: PICO-8 tracker](https://github.com/adamico/picotracker/issues/31) as built. The [Wayfinder map](https://github.com/adamico/picotracker/issues/1) and its closed tickets hold the reasoning. `GLOSSARY.md` and `docs/adr/` are binding. When a decision changes, change it here.

## Modules

The application is built from these deep modules. Each one hides its detail behind a small interface.

| Module | Interface in outline | Hides |
|---|---|---|
| **Project model** | The Song, Chains, Phrases, Steps, Instruments, Export Tracks, Target Cart and Claim, plus validated edits | Sharing rules and limits |
| **Project file** | Parse `.ptrk` text into a Project; write a Project as `.ptrk` text | RON layout, Step tokens, version migration |
| **Cart codec** | Read `.p8` or `.p8.png` bytes into Cart data; write claimed SFX and Patterns back into the original bytes | Text sections, PNG bit packing, SHA-1, byte preservation |
| **Cart Import** | Cart and a chosen song → Project and warnings; list a Cart's songs | PICO-8's Pattern timing rules |
| **MIDI Import** | MIDI file → analysis for the dialog; analysis and choices → Project and warnings | Voices, quantising, program and drum tables |
| **Export** | Project and Cart → new Cart bytes and a new claimed-bytes hash, or the complete list of failures | SFX baking, slot assignment, flow flags |
| **Synth** | Cart-level sound data → samples at 22,050 Hz | The zepto8 port and its fixes |
| **Player** | Project and a play request → the Playhead and audio, with live edits, Mute and Solo | Lockstep sequencing, the audio thread, the limiter |
| **App core** | Named actions and file contents in; Project, character grid, status line, Log, audio requests and file writes out | Focus, selection, clipboard, undo, dialogs |
| **Keymap** | Key events → named actions, for both presets and user rebinds | Hold-chords, double-taps, key repeat |
| **Settings** | Load `config.ron`; load and save `state.ron` | Versioning, damaged-file handling, XDG paths |
| **Window shell** | Runs the App core in a window and starts the audio stream | winit, pixels, cpal |

The App core has no dependency on a window or audio device. The Window shell stays thin enough that it needs no logic tests.

## Dependencies

- `winit` and `pixels` for the window, with `softbuffer` as the fallback.
- `cpal` 0.18 for audio, with the synth on its own thread and lock-free queues to the UI thread.
- `midly` 0.5 for MIDI files.
- `serde` and `ron` for files, plus `png`, `sha1` and `directories`.
- Cart reading and writing is our own code, because no crate covers it.

## Domain rules

**Step and Phrase**
- A Step is exactly one PICO-8 note: pitch 0–63 (C-0 to D#-5), Instrument, volume 0–7, and effect 0–7.
- Volume 0 is silent and keeps its pitch and Instrument. There is no hold marker or OFF note.
- A Phrase has 1–32 Steps, a Speed of 1–255 Ticks, and its own filters: NOIZ and BUZZ on or off; DETUNE, REVERB and DAMPEN 0–2.
- Phrases have no loop points. New Phrases take the Project's default Speed.
- BPM is only a readout: approximately 1807 / Speed, assuming 4 Steps per beat.
- One Tick is 183 samples at 22,050 Hz.

**Song structure**
- Within a Song row, the k-th entries of all Track Chains form one Chain row, which plays in lockstep.
- A Chain row lasts as long as its longest Phrase not marked Fill. If every entry is Fill, the longest plays once and sets the length.
- The Song ends at its first empty row, then jumps to the Song loop row or stops.

**Instruments**
- Instrument `00`–`07` are Built-in Instruments. `08`–`FF` are Custom Instruments, up to 248 per Project.
- An SFX Instrument has 1–32 Steps, Speed, filters and a loop.
- A Waveform Instrument holds 64 signed 8-bit samples, Bass, DETUNE, REVERB and DAMPEN. Its view does not show NOIZ, BUZZ, Speed or loop, because they have no effect.
- Only Custom Instruments have names: optional, up to 12 characters.

## Project file

- One standalone `.ptrk` file in RON, with an integer format version at the top.
- Older versions migrate forward one step at a time. Newer versions are refused with a clear message. Saves always use the current version.
- It holds the Song, Chains, Phrases, Custom Instruments and their names, loose Phrases, Export Tracks, the Song loop row, the Target Cart link, the Claim and the claimed-bytes hash.
- Steps are tracker-style tokens: `C#2 08 5 3` for a note and `--- -- - -` for a silent Step.
- The Target Cart is a path relative to the Project file. The Claim is stored with it: a list of SFX slots plus a first and last Pattern, inclusive.
- Mute, Solo, cursor, focus, undo history and window state are never stored.

## Cart reading and writing

Memory layout:
- `0x3100`: 64 Patterns of 4 bytes. Bits 0–5 are the SFX, bit 6 disables the channel, and bit 7 of bytes 0, 1 and 2 means loop start, loop back and stop.
- `0x3200`: 64 SFX of 68 bytes: 32 little-endian note words, then the editor and filter byte, Speed, loop start and loop end.
- Note word: bits 0–5 pitch, 6–8 instrument, 9–11 volume, 12–14 effect, 15 custom.
- Byte 64: bit 0 editor mode, bit 1 NOIZ, bit 2 BUZZ. DETUNE, REVERB and DAMPEN are base-3 digits at divisors 8, 24 and 72.
- The loop is off when start ≥ end. When loop end is 0 and start is above 0, start is LEN.
- A Waveform Instrument is flagged by bit 7 of byte 66, with Bass in bit 0 of byte 65.

`.p8` text:
- Each `__sfx__` line is 8 header hex digits followed by 32 five-digit `PPIVE` notes. Each `__music__` line is `FF CCCCCCCC`.
- Trailing default rows may be absent, so the writer must handle claimed lines that do not exist yet.
- Export changes only the claimed lines. If a section is missing, it is inserted in canonical order.
- The header, `__lua__`, `__gfx__`, `__label__`, `__gff__`, `__map__`, all `__meta:*__` sections, other SFX and Pattern lines, and line endings are preserved byte for byte.

`.p8.png`:
- A 160×205 RGBA image holds one Cart byte per pixel in the low 2 bits of each channel (A high, then R, G, B).
- Export rewrites only the claimed bytes in the original pixels and recomputes the SHA-1 at `0x8006`. Code is never recompressed, and the visible label is unchanged.

## Cart Import

- Import follows ADR 0006: it rebuilds what each Pattern plays, not the stored SFX.
- A Pattern's length is the left-most non-looping channel's notes × Speed. If every channel loops, it is 32 × the slowest Speed.
- Each Pattern becomes one Song row, and channel *n* goes to Track *n*. Tracks 5–8 start empty, and the Export Tracks are 1–4.
- A disabled channel becomes an empty Chain entry. Identical Chains and exact-match Phrases are shared. Transposes are not detected.
- A channel cut off by the Pattern's end becomes a shortened Phrase, with no warning because the sound is the same. A Pattern ending partway through a Step drops that Step, with a warning.
- A loop over the whole SFX becomes Fill. A lead-in loop is unrolled. If unrolling needs more than 32 Steps, it becomes Fill over the loop body, with a warning. When unrolling moves notes into a different arpeggio group and the sound cannot be kept, Import warns. Notes after a loop's end are dropped with no warning.
- LEN on a channel that does not set the Pattern's length does not stop that channel.
- SPD 0 becomes Speed 1, with a warning.
- LEN above 32 on the length-setting channel splits the Pattern into several Chain rows of up to 32 Steps in one Song row. A Step straddling a split is dropped with a warning. A split inside another channel's Step makes that channel's later Steps play a few Ticks early, with a warning naming the Pattern, the Track and the Ticks (ADR 0006). The Claim does not grow, so Import warns when the song no longer fits.
- A bit 7 SFX played directly becomes an ordinary Phrase of the Steps it plays. It never sets the Pattern's length, and Import warns. A Pattern holding only such SFX is kept as a 32-Step Chain row with a warning.
- A song starts at Pattern 0, or at the first non-empty Pattern after STOP, LOOP BACK or an empty Pattern. The Song loop row comes from where LOOP BACK jumps. A jump outside the song means no loop.
- Each SFX in 0–7 used as an instrument becomes a Custom Instrument. If a Pattern also plays it directly, it also becomes a Phrase.
- The Claim is exactly the chosen song's SFX, including its Instruments, plus its Pattern range.
- Every other non-empty SFX becomes a loose, unclaimed Phrase with its loop dropped, and Import warns.
- A Cart with no music gives a Project with no Song and an empty Claim.

## MIDI Import

- A source is a (MIDI track, channel) pair. Chords split into stacked voices, highest note first. Channel 10 is one source.
- Voices fill Tracks 1–8 in file order, keeping one source's voices adjacent. Voices past 8 are dropped with a warning.
- Steps per beat is 2, 3, 4, 6 or 8, with 4 as the default. Note starts snap to the nearest Step and lengths have a minimum of one Step. On a collision within a voice the later note wins, with a warning. CC64 is honoured.
- Volume is ceil(velocity × 7 / 127). CC7, CC11, pitch bend and other CCs are dropped with warnings.
- Only Built-in Instruments are used. Default program table: bass → triangle; piano and chromatic percussion → tilted saw; organ → organ; guitar and strings → saw; lead → square; pad and ensemble → pulse; synth FX → phaser.
- Default drum kit: kick → low triangle with drop; snare → mid noise with fade out; hats and cymbals → high noise with fade out; toms → triangle with drop at matching pitches. The kit is editable.
- Speed is round(7230 / (BPM × Steps per beat)), clamped to 1–255. A tempo or time signature change starts a new Chain row.
- Phrases hold the largest whole number of bars that fits in 32 Steps, or 32 Steps if one bar does not fit. Each Song row holds a Chain of 4 Chain rows.
- All-silent Phrases become empty Chain entries. Identical Phrases and Chains are shared. Fill is never created.
- Each voice's Phrases stay within 0–63, with the octave offset carried in the Chain transpose. A voice spanning more than 64 semitones is folded by octave with a warning.
- The Project starts with an empty Claim and no Target Cart.

The dialog is full-screen with four pages: Voices, Steps per beat, Instruments, Review. Enter moves forward and Imports on Review. Esc moves back and cancels on Voices. Table edits become app defaults only when the user presses Import.

## Export

Order of checks: Export gathers every failure before writing anything.

Failures:
- no Export Tracks picked (`NO EXPORT TRACKS`);
- empty Claim;
- too few claimed SFX or Patterns;
- more than 8 Custom Instruments used, or too few claimed slots in 0–7 for them;
- a pitch outside 0–63 after transpose on an audible Step inside LEN, or on any Step an arpeggio reads, named by Phrase, Step and Track. Other silent Steps are clamped instead. The Phrase and Chain panes flag exactly the Steps Export refuses.

Compilation:
- Each Chain row becomes exactly one Pattern.
- Each (Phrase, transpose) pair is baked to its own SFX. Byte-identical SFX share a slot, including Instruments and Phrases. Patterns are never reused.
- Speed and filters come from the Phrase. Phrases under 32 Steps use LEN. The editor-mode bit is set to tracker.
- Fill compiles to a loop from 0 to the Phrase's length. In an all-Fill Chain row, the longest entry is written without a loop, using LEN.
- A shorter non-Fill Phrase left of a longer one gets loop start = its length and loop end = length + 1, including 32 to 33.
- When a Chain row's length comes from an unexported Track, Export pads with a silent SFX on a free channel, or warns if none is free.
- A Chain row with no Exported entries becomes one silent SFX when at least one Export Track is picked.
- Custom Instruments take the lowest free claimed slots in 0–7, and Steps' instrument digits are rewritten to match. Phrase SFX then fill the remaining claimed slots in ascending order of first use.
- A Waveform Instrument writes a fixed header: byte 65 is 16, or 17 with Bass; byte 66 is `80`; byte 67 is `00`.
- Patterns fill the claimed range in Song order from its first Pattern. Unused claimed slots are reset to empty.
- LOOP START is always written on the loop target. The last Pattern gets LOOP BACK, or STOP when the Song does not loop.

Around Export:
- With no Target Cart, Export opens the Cart view so the user can pick a Cart and Claim.
- With a Target Cart but an empty Claim, Export fails with the no-Claim message.
- If the path no longer resolves, the status line offers `ENTER LOCATE / ESC CANCEL`, and the Claim and hash carry over.
- If the claimed bytes no longer match the stored hash, the status line names the SFX and Patterns and offers `ENTER OVERWRITE / ESC CANCEL`.
- A failed Export leaves the Cart byte for byte unchanged.
- Export does not stop playback or touch the undo history.

## Synth

- Port zepto8's synth under its WTFPL licence, with fake-08's MIT REVERB 2 fix. Keep both notices.
- Required fixes to the port:
  - effect 3 on an SFX Instrument note keeps the pitch, and the same pitch retriggers while a new pitch does not;
  - slide is linear in pitch, not in frequency;
  - reverb tails ring through silent Steps and after a LEN inside a Pattern, for about 12 Ticks at REVERB 1 and 24 at REVERB 2, and are cut when `sfx()` ends at LEN;
  - arpeggios inside an SFX Instrument follow the played pitch.
- Waveform Instruments are new work: linear interpolation, with C-2 playing one 64-sample cycle at 261.6 Hz. DETUNE, REVERB and DAMPEN apply. NOIZ and BUZZ on the playing Phrase have no effect.
- The output mixes at 22,050 Hz and is resampled to the device rate.

## Playback

| Key and focus | Plays | Timing | Mute and Solo |
|---|---|---|---|
| Space, Song pane | The Song from the cursor's Song row | As in the Song | Respected |
| Space, Chain pane | This Chain alone, looping, from the cursor's entry | Each entry lasts as long as its Chain row | Ignored |
| Space, Phrase pane | This Phrase alone, looping, from the cursor's Step, with its transpose | The Phrase's own LEN | Ignored |
| Shift+Space, any pane | The Song from the cursor's Song row | As in the Song | Respected |

- Space always stops when something is playing, and stopping cuts the sound at once.
- With focus in the right-hand area, Space uses the last focused Song, Chain or Phrase pane.
- Mute and Solo never change timing, are not undoable and are not saved. Solo overrides Mute.
- Each Track plays at PICO-8's per-channel level. A limiter acts only when the 8-Track mix would clip.
- Edits are heard the next time the Playhead reaches them. A sounding Step is not retriggered, and a Speed change applies from the next Step.
- An edit never stops playback. A Phrase shortened past the Playhead ends its entry, a cleared entry stays silent until the next Chain row, and an emptied Song row acts like reaching an empty row.
- Opening or Importing a Project stops playback. Save and Export do not.
- Playback is more permissive than Export: out-of-range Steps play at their true pitch, and more than 8 Custom Instruments all play.

## Editing

- Clone, Deep clone, selection, clipboard, insert, delete, Clear, Purge unused, undo and Change Speed behave as decided in [Editing operations](https://github.com/adamico/picotracker/issues/20).
- Change Speed takes a typed new Speed for the Phrase in the Phrase pane, and scales the others by the same ratio.
- One linear undo history covers the whole Project, keeps 1,000 undo entries, lasts for the session and survives Save. Import starts an empty history.
- Each keymap action is one undo entry. Repeated nudges or typed digits on one cell merge until the cursor moves or another action runs.
- Each visit to the Cart view is one undo entry for the Claim.

## UI

- 640×360 logical pixels, 6×8 cells and 106×45 characters, using our own 5×7 font and the 16-colour PICO-8 palette.
- Whole-number scaling only, with black borders. Windows too small for ×2 use ×1.
- Column budget: Song 28, Chain and Chain row strip 33, Phrase 19, right-hand slot 23, with separators. The bottom row is the status line.
- The Phrase pane header shows SPD, BPM, LEN and the 5 filters.
- The right-hand slot shows one of Overview, Cart or Log. Tab cycles Song, Chain, Phrase and the visible right-hand view.
- Overview shows the 4 Export Tracks (`CH0`–`CH3`) and a read-only scope. Typing 1–8 sets a channel's Track, Del clears it, and choosing a Track already on another channel swaps the two.
- The Cart view shows the path, the SFX and Pattern fields, two 16×4 slot maps (`.` free, `O` used, `#` claimed, `!` claimed and overwritten) and live `ERR`, `WRN` and `OK` checks.
- SFX Claims are typed as hex slots and ranges such as `03 04 32-3D`, and Patterns as one range such as `18-1D`. The Claim applies as soon as the text is valid, and Esc restores the Claim from when the view opened.
- Status line colours: green info that fades after about 3 seconds, red failures until Esc, orange warnings, and blue questions with inline answers.
- The Log keeps a short history, and each item has a severity, wrapped text and an action hint. A result with items opens the Log without taking focus.
- The idle status line shows the main keys: `^N NEW  ^O OPEN  ^I IMPORT  ^S SAVE  ^L LOG  SPACE PLAY  ^Q QUIT`.
- The window title is the Project's file name as spelt, or `Untitled`, with `*` for unsaved changes, then `- picotracker`.
- There are no modal boxes.

## Keys

Every key goes through a named, rebindable action. The M8-style and PC tracker presets ship, and PC tracker is the first-launch default. Any bindings not listed in the closed tickets, including M8-style bindings for dialogs, are drafted during the build under the same scheme.

| Action | PC tracker | M8-style |
|---|---|---|
| New Project | Ctrl+N | Ctrl+N |
| Open Project | Ctrl+O | Ctrl+O |
| Save / Save As | Ctrl+S / Ctrl+Shift+S | Ctrl+S / Ctrl+Shift+S |
| Import (Cart or MIDI file) | Ctrl+I | Ctrl+I |
| Export | Ctrl+E | Ctrl+E |
| Quit | Ctrl+Q | Ctrl+Q |
| New unused Chain or Phrase | Ctrl+Shift+N | Double-tap Z |
| Start selection | Ctrl+B | Shift+X |
| Copy | Ctrl+C | X, while selecting |
| Cut | Ctrl+X | Z+X |
| Paste | Ctrl+V | Shift+Z |
| Clone | Ctrl+D | Shift+X then Z |
| Deep clone | Ctrl+Shift+D | Ctrl+Shift+D |
| Undo / Redo | Ctrl+Z / Ctrl+Y | Ctrl+Z / Ctrl+Y |
| Insert / Delete row | Ins / Ctrl+Del | Ins / Ctrl+Del |
| Change Speed | Ctrl+Shift+T | Ctrl+Shift+T |
| Purge unused | Ctrl+Shift+P | Ctrl+Shift+P |
| Toggle Cart | Ctrl+T | X+T |
| Toggle Log | Ctrl+L | X+L |
| Play focused pane, or stop | Space | Space |
| Play Song from cursor | Shift+Space | Shift+Space |
| Mute Track 1–8 | F1–F8 | F1–F8 |
| Solo Track 1–8 | Shift+F1–F8 | Shift+F1–F8 |
| Clear Mutes and Solos | F9 | F9 |
| Follow | F10 | F10 |

Other preset behaviour:
- **PC tracker:** Tab, Enter or Esc moves focus; hex digits are typed directly; piano keys enter notes; `[` and `]` change the octave; Shift or Ctrl+↑/↓ nudges; Del clears.
- **M8-style:** Z is EDIT and X is OPTION; Shift+arrows changes the focused pane; Z+arrows edits; X+arrows jumps Track, Chain or Phrase.
- Esc cancels an active selection before anything else.

Import, Open and Save As use a file list on the grid, following the Cart view's file list. It starts in the folder the app was launched from, then in the last Project's folder. Open lists up to 10 recent Projects under RECENT above the folder, leaving out the open Project, with the cursor on the newest. Import chooses Cart Import or MIDI Import from the selected file.

Quit, from Ctrl+Q or the window's close button, works from anywhere. With unsaved changes it asks the same question as New and Open.

## App files

- `.ptrk`: the music and optional Custom Instrument names.
- `$XDG_CONFIG_HOME/picotracker/config.ron`: the active preset and only the keys the user changed. The user edits it by hand, the app reads it at launch and never writes it.
- `$XDG_STATE_HOME/picotracker/state.ron`: Follow, octave, changed MIDI table rows and the 10 most recent Projects, opened or saved, newest first. The `directories` crate finds the folder; macOS and Windows use the local data folder.
- At launch the app opens the most recent Project. If that file is gone, it says so and starts on an empty Project.
- Both files are RON with an integer format version.
- A damaged file, including one that is not valid text, is renamed with a `.bad` suffix, and the app starts on defaults and says so. `state.ron` is written again only once that rename has worked.
- A newer file, or one that cannot be read, is left untouched, defaults apply for the session, and the app never writes it that session.

## Licence and release

- The project is licensed `MIT OR Apache-2.0`.
- Each release includes a generated `THIRD_PARTY_LICENSES` file, with a hand-written section for the ported synth. `--licenses` prints the same text and an independence statement.
- `cargo-deny` enforces the licence allowlist decided in [Choose the project's licence](https://github.com/adamico/picotracker/issues/26).
- The MVP is `0.1.0`, released as one Linux x86_64 `.tar.gz` on GitHub Releases with `SHA256SUMS`. It contains the binary, `README.md`, `CHANGELOG.md`, both licence files, `THIRD_PARTY_LICENSES` and a commented `config.example.ron`.
- glibc and `libasound` are linked dynamically and not bundled. Releases are built on the oldest Ubuntu LTS that GitHub's runners support.
- A version tag runs the full CI suite and creates a draft release, which the owner publishes by hand.

## Out of scope

- M8 tables, grooves and live mode.
- M8-style effect commands that need compiling.
- Live MIDI input and output.
- A web or WASM build.
- Live sync with a running PICO-8.
- Several Songs in one Project.
- Embedding the Project, or a link to it, in a Cart's `__meta:*__` section.
- Manual pinning of a Custom Instrument to an SFX slot.
- Folding repeated Phrases into loops at Export.
- Transpose detection during Import.
- A settings screen, UI colour themes, and remembering the cursor per Project.
- AppImage and Arch packages, publishing to crates.io, and releases for Windows, macOS or Linux on ARM.

## Owner-gated steps and known limits

- Renaming the app to `pikutracker` and purging the two reference manuals from the repo and its history happen together, only on the owner's explicit go-ahead ([#51](https://github.com/adamico/picotracker/issues/51)). The purge rewrites `main` and every pushed branch. The rename also moves the config and state folders to `pikutracker`. The release workflow skips every job until the purge is done.
- The PICO-8 checks did not test SFX Instruments nested deeper than one level, and could not measure clicks on the noise waveform.
- The licence conclusions are an engineering reading of the licence files, not legal advice. No trademark search was run for the new name.
