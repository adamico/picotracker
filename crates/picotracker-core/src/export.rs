//! Export: compiles a Project into new Cart bytes and a claimed-bytes hash, or the complete list
//! of failures.
//!
//! This covers simple Projects: each Chain row is timed by its left-most Exported entry, with no
//! Fill and no Custom Instruments. Anything else is refused as [`ExportFailure::NotSimple`] rather
//! than Exported so that it plays differently.

use crate::cart::{
    Cart, CartError, FILTERS_BYTE, LOOP_START_BYTE, SFX_BYTES, SLOTS, SPEED_BYTE, note_word,
};
use crate::project::{
    CHAIN_ENTRIES, ChainEntry, Claim, EXPORT_CHANNELS, FIRST_CUSTOM_INSTRUMENT, MAX_PITCH,
    MAX_STEPS, Phrase, Project, SONG_ROWS,
};
use std::collections::BTreeSet;
use std::fmt;

const FULL_LENGTH: usize = 32;

const TRACKER_MODE_BIT: u8 = 0x01;
const NOIZ_BIT: u8 = 0x02;
const BUZZ_BIT: u8 = 0x04;
const DETUNE_UNIT: u8 = 8;
const REVERB_UNIT: u8 = 24;
const DAMPEN_UNIT: u8 = 72;

const LOOP_START_CHANNEL: usize = 0;
const LOOP_BACK_CHANNEL: usize = 1;
const STOP_CHANNEL: usize = 2;
const FLAG: u8 = 0x80;
const DISABLED_CHANNEL: u8 = 0x41;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFailure {
    NoExportTracks,
    EmptyClaim,
    BadClaim,
    EmptySong,
    TooFewSfx {
        needed: usize,
        claimed: usize,
    },
    TooFewPatterns {
        needed: usize,
        claimed: usize,
    },
    NoteOutOfRange {
        phrase: u8,
        step: usize,
        track: usize,
        song_row: usize,
        chain_row: usize,
    },
    NotSimple {
        song_row: usize,
        chain_row: usize,
    },
    UnreadableCart(CartError),
}

impl fmt::Display for ExportFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoExportTracks => write!(formatter, "NO EXPORT TRACKS"),
            Self::EmptyClaim => write!(
                formatter,
                "THIS PROJECT CLAIMS NO SFX OR PATTERNS, SO THERE IS NOWHERE TO WRITE"
            ),
            Self::BadClaim => write!(formatter, "THE CLAIM NAMES SLOTS A CART DOES NOT HAVE"),
            Self::EmptySong => write!(formatter, "THE SONG IS EMPTY"),
            Self::TooFewSfx { needed, claimed } => {
                write!(formatter, "NEEDS {needed} SFX, THE CLAIM HAS {claimed}")
            }
            Self::TooFewPatterns { needed, claimed } => {
                write!(
                    formatter,
                    "NEEDS {needed} PATTERNS, THE CLAIM HAS {claimed}"
                )
            }
            Self::NoteOutOfRange {
                phrase,
                step,
                track,
                ..
            } => write!(
                formatter,
                "PHRASE {phrase:02X} STEP {step:02X} ON T{} IS OUTSIDE C-0 TO D#5",
                track + 1
            ),
            Self::NotSimple {
                song_row,
                chain_row,
            } => write!(
                formatter,
                "SONG ROW {song_row:02X} CHAIN ROW {chain_row:X} USES FILL, CUSTOM INSTRUMENTS \
                 OR MIXED LENGTHS, WHICH CANNOT EXPORT YET"
            ),
            Self::UnreadableCart(error) => write!(formatter, "CANNOT READ THE CART: {error}"),
        }
    }
}

/// A finished Export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exported {
    pub cart_bytes: Vec<u8>,
    pub claimed_bytes_hash: String,
    pub sfx_count: usize,
    pub pattern_count: usize,
}

/// One Chain row as a Pattern: the compiled SFX each channel plays.
type CompiledPattern = [Option<usize>; EXPORT_CHANNELS];

/// A Song compiled to Cart data that has not been given slots yet.
#[derive(Default)]
struct CompiledSong {
    sfx: Vec<[u8; SFX_BYTES]>,
    patterns: Vec<CompiledPattern>,
    loop_target: Option<usize>,
}

impl CompiledSong {
    /// The index of an SFX, shared with any byte-identical one.
    fn sfx_index(&mut self, sfx: [u8; SFX_BYTES]) -> usize {
        self.sfx
            .iter()
            .position(|known| *known == sfx)
            .unwrap_or_else(|| {
                self.sfx.push(sfx);
                self.sfx.len() - 1
            })
    }
}

/// Whether every slot a Claim names exists in a Cart.
fn claim_is_valid(claim: &Claim) -> bool {
    let exists = |slot: u8| usize::from(slot) < SLOTS;
    claim.sfx.iter().all(|slot| exists(*slot))
        && claim
            .patterns
            .is_none_or(|(first, last)| first <= last && exists(last))
}

/// A Claim that names each SFX slot once, in the order it first named them.
fn without_repeats(claim: &Claim) -> Claim {
    let mut seen = BTreeSet::new();
    Claim {
        sfx: claim
            .sfx
            .iter()
            .copied()
            .filter(|slot| seen.insert(*slot))
            .collect(),
        patterns: claim.patterns,
    }
}

/// Compiles a Project into the bytes of its Target Cart, changing nothing outside the Claim.
pub fn export(
    project: &Project,
    claim: &Claim,
    original: &[u8],
) -> Result<Exported, Vec<ExportFailure>> {
    let mut failures = Vec::new();
    let has_tracks = project.export_tracks().iter().any(Option::is_some);
    if !has_tracks {
        failures.push(ExportFailure::NoExportTracks);
    }
    let claim = &without_repeats(claim);
    let claim_is_empty = claim.sfx.is_empty() && claim.patterns.is_none();
    if claim_is_empty {
        failures.push(ExportFailure::EmptyClaim);
    }
    let claim_is_valid = claim_is_valid(claim);
    if !claim_is_valid {
        failures.push(ExportFailure::BadClaim);
    }
    let chain_rows = chain_rows(project);
    if chain_rows.is_empty() {
        failures.push(ExportFailure::EmptySong);
    }
    let song = compile_song(project, &chain_rows, has_tracks, &mut failures);

    let sfx_slots: Vec<u8> = claim
        .sfx
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let pattern_slots: Vec<u8> = claim
        .patterns
        .map_or(Vec::new(), |(first, last)| (first..=last).collect());
    if !claim_is_empty && claim_is_valid {
        check_room(&song, &sfx_slots, &pattern_slots, &mut failures);
    }

    let cart = Cart::read(original);
    if let Err(error) = cart {
        failures.push(ExportFailure::UnreadableCart(error));
    }
    let (Ok(mut cart), true) = (cart, failures.is_empty()) else {
        return Err(failures);
    };

    place_song(&song, &sfx_slots, &pattern_slots, &mut cart);
    let cart_bytes = cart
        .write_claimed(original, claim)
        .map_err(|error| vec![ExportFailure::UnreadableCart(error)])?;
    Ok(Exported {
        cart_bytes,
        claimed_bytes_hash: cart.claimed_bytes_hash(claim),
        sfx_count: song.sfx.len(),
        pattern_count: song.patterns.len(),
    })
}

/// The Song rows that play, in order: up to the first empty row, and then, if the Song loop row
/// lies past that gap, from the loop row up to the next empty row.
fn song_rows(project: &Project) -> Vec<usize> {
    let rows_from =
        |first: usize| (first..SONG_ROWS).take_while(|&row| !project.song_row_is_empty(row));
    let mut rows: Vec<usize> = rows_from(0).collect();
    let past_a_gap = project
        .song_loop_row()
        .filter(|row| !rows.is_empty() && !rows.contains(row));
    if let Some(loop_row) = past_a_gap {
        rows.extend(rows_from(loop_row));
    }
    rows
}

/// Every Chain row that plays, in Song order, as a Song row and an entry number.
fn chain_rows(project: &Project) -> Vec<(usize, usize)> {
    song_rows(project)
        .into_iter()
        .flat_map(|row| (0..CHAIN_ENTRIES).map(move |k| (row, k)))
        .filter(|&(row, k)| project.chain_row_ticks(row, k) > 0)
        .collect()
}

/// Compiles every Chain row. With no Export Tracks there is nothing to bake, but each Chain row
/// still counts as a Pattern, so that a Claim that is too small is reported as well.
fn compile_song(
    project: &Project,
    chain_rows: &[(usize, usize)],
    has_tracks: bool,
    failures: &mut Vec<ExportFailure>,
) -> CompiledSong {
    let mut song = CompiledSong::default();
    for &(song_row, chain_row) in chain_rows {
        if has_tracks && !is_simple(project, song_row, chain_row) {
            failures.push(ExportFailure::NotSimple {
                song_row,
                chain_row,
            });
        }
        let mut pattern = [None; EXPORT_CHANNELS];
        for (channel, track) in project.export_track_indices().into_iter().enumerate() {
            let Some(track) = track else {
                continue;
            };
            let Some(entry) = project.chain_row_entry(song_row, track, chain_row) else {
                continue;
            };
            let phrase = project.phrase(entry.phrase());
            for step in phrase.out_of_range_steps(entry.transpose()) {
                failures.push(ExportFailure::NoteOutOfRange {
                    phrase: entry.phrase().number(),
                    step,
                    track,
                    song_row,
                    chain_row,
                });
            }
            pattern[channel] = Some(song.sfx_index(bake_sfx(phrase, entry.transpose())));
        }
        song.patterns.push(pattern);
    }
    song.loop_target = project
        .song_loop_row()
        .and_then(|loop_row| chain_rows.iter().position(|(row, _)| *row == loop_row));
    song
}

/// The Exported entries of a Chain row, from channel 0 up.
fn exported_entries(project: &Project, song_row: usize, chain_row: usize) -> Vec<ChainEntry> {
    project
        .export_track_indices()
        .into_iter()
        .flatten()
        .filter_map(|track| project.chain_row_entry(song_row, track, chain_row))
        .collect()
}

/// Whether a Chain row plays the same in PICO-8 with no further compile rules: PICO-8 ends a
/// Pattern with its left-most channel, so that channel has to last as long as the Chain row.
fn is_simple(project: &Project, song_row: usize, chain_row: usize) -> bool {
    let entries = exported_entries(project, song_row, chain_row);
    let Some(left_most) = entries.first() else {
        return false;
    };
    let ticks = project.phrase(left_most.phrase()).ticks();
    ticks == project.chain_row_ticks(song_row, chain_row)
        && entries
            .iter()
            .all(|entry| !entry.fill() && !plays_custom_instrument(project.phrase(entry.phrase())))
}

fn plays_custom_instrument(phrase: &Phrase) -> bool {
    (0..phrase.length())
        .map(|index| phrase.step(index))
        .any(|step| !step.is_silent() && step.instrument() >= FIRST_CUSTOM_INSTRUMENT)
}

/// The 68 bytes of the SFX that plays a Phrase at a transpose. Steps past LEN are never heard, so
/// they are left blank, which lets more SFX share a slot. The exception is a pitch that an arpeggio
/// reads, which is kept on a silent note.
fn bake_sfx(phrase: &Phrase, transpose: i8) -> [u8; SFX_BYTES] {
    let mut sfx = [0; SFX_BYTES];
    let read = phrase.steps_read_by_arpeggios();
    for index in 0..MAX_STEPS {
        let step = phrase.step(index);
        let pitch = (i32::from(step.pitch()) + i32::from(transpose)).clamp(0, MAX_PITCH) as u16;
        let word = if index < phrase.length() {
            note_word(
                pitch,
                u16::from(step.instrument() & 7),
                u16::from(step.volume()),
                u16::from(step.effect()),
                0,
            )
        } else if read[index] {
            pitch
        } else {
            0
        };
        sfx[index * 2..index * 2 + 2].copy_from_slice(&word.to_le_bytes());
    }
    sfx[FILTERS_BYTE] = filters_byte(phrase);
    sfx[SPEED_BYTE] = phrase.speed();
    if phrase.length() < FULL_LENGTH {
        sfx[LOOP_START_BYTE] = phrase.length() as u8;
    }
    sfx
}

fn filters_byte(phrase: &Phrase) -> u8 {
    let filters = phrase.filters();
    let switch = |on: bool, bit: u8| if on { bit } else { 0 };
    TRACKER_MODE_BIT
        + switch(filters.noiz(), NOIZ_BIT)
        + switch(filters.buzz(), BUZZ_BIT)
        + filters.detune() * DETUNE_UNIT
        + filters.reverb() * REVERB_UNIT
        + filters.dampen() * DAMPEN_UNIT
}

fn check_room(
    song: &CompiledSong,
    sfx_slots: &[u8],
    pattern_slots: &[u8],
    failures: &mut Vec<ExportFailure>,
) {
    if song.sfx.len() > sfx_slots.len() {
        failures.push(ExportFailure::TooFewSfx {
            needed: song.sfx.len(),
            claimed: sfx_slots.len(),
        });
    }
    if song.patterns.len() > pattern_slots.len() {
        failures.push(ExportFailure::TooFewPatterns {
            needed: song.patterns.len(),
            claimed: pattern_slots.len(),
        });
    }
}

/// Puts a compiled Song into the claimed slots, and resets the claimed slots it does not use.
fn place_song(song: &CompiledSong, sfx_slots: &[u8], pattern_slots: &[u8], cart: &mut Cart) {
    let blank = Cart::default();
    for (index, &slot) in sfx_slots.iter().enumerate() {
        let sfx = song.sfx.get(index).unwrap_or(blank.sfx_bytes(slot));
        cart.set_sfx_bytes(slot, *sfx);
    }
    for (index, &slot) in pattern_slots.iter().enumerate() {
        let bytes = song.patterns.get(index).map_or_else(
            || blank.pattern_bytes(slot),
            |pattern| pattern_bytes(song, index, pattern, sfx_slots),
        );
        cart.set_pattern_bytes(slot, bytes);
    }
}

/// The 4 bytes of a Pattern, with the flags that its place in the Song calls for. A disabled
/// channel holds the disabling bit and its number from 1.
fn pattern_bytes(
    song: &CompiledSong,
    index: usize,
    pattern: &CompiledPattern,
    sfx_slots: &[u8],
) -> [u8; EXPORT_CHANNELS] {
    let mut bytes = [0; EXPORT_CHANNELS];
    for (channel, byte) in bytes.iter_mut().enumerate() {
        *byte = pattern[channel].map_or(DISABLED_CHANNEL + channel as u8, |sfx| sfx_slots[sfx]);
    }
    if song.loop_target == Some(index) {
        bytes[LOOP_START_CHANNEL] |= FLAG;
    }
    if index + 1 == song.patterns.len() {
        let ending = if song.loop_target.is_some() {
            LOOP_BACK_CHANNEL
        } else {
            STOP_CHANNEL
        };
        bytes[ending] |= FLAG;
    }
    bytes
}
