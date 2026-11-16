# picotracker

A music tracker for composing PICO-8 music with a Song → Chain → Phrase workflow modelled on the Dirtywave M8, exporting to PICO-8 carts.

## Language

### Our side

**Project**:
The tracker's own saved work: the full Song, its Chains, Phrases and instruments. Richer than what a Cart can hold.
_Avoid_: Song file, module

**Song**:
The top-level arrangement of a Project: rows of Chains, one per Track.
_Avoid_: Arrangement, sequence

**Track**:
One of the Song's 8 vertical lanes, played by one voice. Export picks 4 of them for PICO-8's channels.
_Avoid_: Channel (that is PICO-8's word)

**Chain**:
An ordered list of Phrases, each with an optional transpose.
_Avoid_: Block, sequence

**Phrase**:
A run of 1 to 32 Steps (16 by default); the unit where notes are written.
_Avoid_: Pattern (that is PICO-8's word), SFX

**Step**:
One row of a Phrase, equivalent to one PICO-8 note: a pitch with its instrument, volume and effect. A Step at volume 0 is silent and is shown as empty; a note is held by repeating it in the following Steps.
_Avoid_: Row, note (when meaning the whole row)

### PICO-8 side

**Cart**:
A PICO-8 cartridge file, either `.p8` text or `.p8.png` image.
_Avoid_: Cartridge file, ROM

**SFX**:
One of the Cart's 64 sound slots: up to 32 notes with a speed, loop/length and filter switches. Used for both sound effects and music.
_Avoid_: Sample, sound

**Pattern**:
One of the Cart's 64 music entries: up to 4 SFX played together on PICO-8's channels, plus loop-start, loop-back and stop flags.
_Avoid_: Music (ambiguous), frame

### Between them

**Export**:
Compiling a Project into a Cart's SFX and Patterns. May lose information the Cart cannot hold.
_Avoid_: Save (saving writes the Project)

**Import**:
Reconstructing a Project from outside material: a Cart's SFX and Patterns (Cart Import) or a Standard MIDI File (MIDI Import).
_Avoid_: Load (loading reads a Project)
