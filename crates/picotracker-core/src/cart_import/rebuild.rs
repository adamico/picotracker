//! PICO-8's Pattern timing: works out how long a Pattern plays and rebuilds the Steps each channel
//! plays in that time (ADR 0006).

use crate::cart::{Cart, Pattern, SFX_NOTES, Sfx};
use crate::project::{
    ARPEGGIO_GROUP, EXPORT_CHANNELS, Filters, MAX_STEPS, Phrase, Step, arpeggio_group_start,
};
use std::collections::BTreeSet;

const BIT_7: u8 = 0x80;
const NOIZ_BIT: u8 = 0x02;
const BUZZ_BIT: u8 = 0x04;
const DETUNE_DIVISOR: u8 = 8;
const REVERB_DIVISOR: u8 = 24;
const DAMPEN_DIVISOR: u8 = 72;
const FILTER_LEVELS: u8 = 3;
const LOOP_CYCLE_STEPS: usize = 512;

/// The Steps of one Chain entry.
pub(super) struct RebuiltEntry {
    pub(super) phrase: Phrase,
    pub(super) fill: bool,
    pub(super) instruments: BTreeSet<u8>,
    pub(super) arpeggio_changed: Option<usize>,
}

/// What one channel plays in one Chain row, and what could not be kept. A Phrase can only start
/// where its Chain row starts, so when the Chain row starts partway through one of the channel's
/// Steps, the following Steps play `early_ticks` early.
pub(super) struct RebuiltChannel {
    pub(super) entry: Option<RebuiltEntry>,
    pub(super) lost_step: bool,
    pub(super) unroll_needed: Option<usize>,
    pub(super) early_ticks: u32,
}

/// What a Pattern becomes: its Chain rows, each with one place per channel. A Pattern where
/// every channel that plays holds a bit 7 SFX stalls PICO-8.
pub(super) struct RebuiltPattern {
    pub(super) chain_rows: Vec<[Option<RebuiltChannel>; EXPORT_CHANNELS]>,
    pub(super) only_bit_7: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Playback {
    Once,
    Loop { start: usize, end: usize },
    Bit7,
}

impl Playback {
    /// A bit 7 SFX plays its 32 notes whatever its LEN and loop say, and never sets the Pattern's
    /// length.
    fn of(sfx: Sfx<'_>) -> Self {
        let (start, end) = (sfx.loop_start(), sfx.loop_end());
        if has_bit_7(sfx) {
            Self::Bit7
        } else if start < end {
            Self::Loop {
                start: usize::from(start),
                end: usize::from(end),
            }
        } else {
            Self::Once
        }
    }

    /// The stored note a channel plays as its Step at this index. Past note 31 it plays none.
    fn note_at(self, step: usize) -> Option<usize> {
        let note = match self {
            Self::Loop { start, end } if step >= end => start + (step - start) % (end - start),
            _ => step,
        };
        (note < SFX_NOTES).then_some(note)
    }
}

/// Whether bit 7 of byte 66 is set, which also means a LEN or loop start of 128 or more.
pub(super) const fn has_bit_7(sfx: Sfx<'_>) -> bool {
    sfx.loop_start() & BIT_7 != 0
}

/// How many Ticks each note of an SFX lasts. SPD 0 plays as Speed 1.
fn speed_of(sfx: Sfx<'_>) -> u32 {
    u32::from(sfx.speed().max(1))
}

/// How many notes an SFX plays when it sets the Pattern's length: its LEN, or else all of them.
fn notes_when_setting_length(sfx: Sfx<'_>) -> u32 {
    if sfx.loop_end() == 0 && sfx.loop_start() > 0 {
        u32::from(sfx.loop_start())
    } else {
        SFX_NOTES as u32
    }
}

fn filters_of(sfx: Sfx<'_>) -> Filters {
    let byte = sfx.filters();
    let level = |divisor: u8| i32::from(byte / divisor % FILTER_LEVELS);
    let mut filters = Filters::default();
    filters.set_noiz(byte & NOIZ_BIT != 0);
    filters.set_buzz(byte & BUZZ_BIT != 0);
    filters.set_detune(level(DETUNE_DIVISOR));
    filters.set_reverb(level(REVERB_DIVISOR));
    filters.set_dampen(level(DAMPEN_DIVISOR));
    filters
}

/// Gives a Phrase the pitches its arpeggios read, on silent Steps wherever no note is played, and
/// says which Step is the first whose arpeggio cannot sound as it does in the Cart. An arpeggio's
/// timing follows its place in the SFX, so one that has moved keeps its sound only if all 4 of its
/// pitches are the same.
fn carry_arpeggio_pitches(
    sfx: Sfx<'_>,
    played: &[Option<usize>],
    phrase: &mut Phrase,
) -> Option<usize> {
    let mut pitches = [None; MAX_STEPS];
    for (index, note) in played.iter().enumerate() {
        pitches[index] = note.map(|note| sfx.note(note).pitch);
    }
    let mut changed = None;
    for (index, note) in played.iter().enumerate() {
        let Some(note) = *note else { continue };
        if !phrase.step(index).plays_arpeggio() {
            continue;
        }
        let wanted: [u8; ARPEGGIO_GROUP] =
            std::array::from_fn(|offset| sfx.note(arpeggio_group_start(note) + offset).pitch);
        let mut keeps_sound = index == note || wanted.iter().all(|pitch| *pitch == wanted[0]);
        for (offset, pitch) in wanted.into_iter().enumerate() {
            let slot = &mut pitches[arpeggio_group_start(index) + offset];
            keeps_sound &= *slot.get_or_insert(pitch) == pitch;
        }
        if !keeps_sound {
            changed.get_or_insert(index);
        }
    }
    for (index, pitch) in pitches.into_iter().enumerate() {
        let plays_a_note = played.get(index).is_some_and(Option::is_some);
        if let Some(pitch) = pitch.filter(|_| !plays_a_note) {
            *phrase.step_mut(index) = Step::new(pitch, 0, 0, 0).unwrap_or_default();
        }
    }
    changed
}

/// The entry that plays these stored notes, with silence for `None`.
fn entry_of(sfx: Sfx<'_>, notes: impl Iterator<Item = Option<usize>>, fill: bool) -> RebuiltEntry {
    let mut phrase = Phrase::default();
    let mut instruments = BTreeSet::new();
    let mut length = 0;
    let notes: Vec<Option<usize>> = notes.collect();
    for (index, played) in notes.iter().copied().enumerate() {
        length = index + 1;
        let Some(played) = played else { continue };
        let note = sfx.note(played);
        // The Cart codec hands out values that are always inside a Step's limits.
        *phrase.step_mut(index) =
            Step::new(note.pitch, note.instrument, note.volume, note.effect).unwrap_or_default();
        if note.custom && note.volume > 0 {
            instruments.insert(note.instrument);
        }
    }
    phrase.set_length(length as i32);
    phrase.set_speed(speed_of(sfx) as i32);
    *phrase.filters_mut() = filters_of(sfx);
    let arpeggio_changed = carry_arpeggio_pitches(sfx, &notes, &mut phrase);
    RebuiltEntry {
        phrase,
        fill,
        instruments,
        arpeggio_changed,
    }
}

/// A Phrase marked Fill over these stored notes.
fn fill_over(
    sfx: Sfx<'_>,
    start: usize,
    end: usize,
    unroll_needed: Option<usize>,
) -> RebuiltChannel {
    RebuiltChannel {
        entry: Some(entry_of(sfx, (start..end.min(SFX_NOTES)).map(Some), true)),
        lost_step: false,
        unroll_needed,
        early_ticks: 0,
    }
}

/// Whether nothing can be heard from this Step on. Past `LOOP_CYCLE_STEPS`, every note a loop can
/// reach has come round at least once.
fn silent_from(sfx: Sfx<'_>, playback: Playback, first: usize, past: usize) -> bool {
    (first..past.min(first + LOOP_CYCLE_STEPS))
        .filter_map(|step| playback.note_at(step))
        .all(|note| sfx.note(note).volume == 0)
}

/// Rebuilds what a channel plays between two Ticks of its Pattern.
fn rebuild_channel(sfx: Sfx<'_>, from: u32, to: u32, may_fill: bool) -> RebuiltChannel {
    let playback = Playback::of(sfx);
    if let Playback::Loop { start: 0, end } = playback
        && may_fill
        && end <= SFX_NOTES
    {
        return fill_over(sfx, 0, end, None);
    }
    let speed = speed_of(sfx);
    let first = from.div_ceil(speed) as usize;
    let early_ticks = first as u32 * speed - from;
    let past = (to / speed) as usize;
    let needed = past.saturating_sub(first);
    let fits = needed <= MAX_STEPS;
    if let Playback::Loop { start, end } = playback
        && !fits
        && !silent_from(sfx, playback, first + MAX_STEPS, past)
    {
        return RebuiltChannel {
            early_ticks,
            ..fill_over(sfx, start, end, Some(needed))
        };
    }
    let steps = needed.min(MAX_STEPS);
    let notes = (first..first + steps).map(|step| playback.note_at(step));
    RebuiltChannel {
        entry: (steps > 0).then(|| entry_of(sfx, notes, false)),
        lost_step: fits && !to.is_multiple_of(speed) && playback.note_at(past).is_some(),
        unroll_needed: None,
        early_ticks,
    }
}

/// The channel whose timing a Pattern follows, and how many Ticks the Pattern lasts: the left-most
/// channel that does not loop or, when every channel loops, the slowest. Bit 7 SFX count only when
/// nothing else plays.
fn timing_of(sfx: &[Option<Sfx<'_>>; EXPORT_CHANNELS]) -> Option<(usize, u32)> {
    let playing = || (0..EXPORT_CHANNELS).filter_map(|c| sfx[c].map(|sfx| (c, sfx)));
    if let Some((channel, sfx)) = playing().find(|(_, sfx)| Playback::of(*sfx) == Playback::Once) {
        return Some((channel, notes_when_setting_length(sfx) * speed_of(sfx)));
    }
    // Reversed, so that the left-most of equally slow channels wins.
    let (channel, sfx) = playing()
        .rev()
        .filter(|(_, sfx)| !has_bit_7(*sfx))
        .max_by_key(|(_, sfx)| speed_of(*sfx))
        .or_else(|| playing().rev().max_by_key(|(_, sfx)| speed_of(*sfx)))?;
    Some((channel, SFX_NOTES as u32 * speed_of(sfx)))
}

/// Rebuilds what a Pattern plays.
pub(super) fn rebuild_pattern(cart: &Cart, pattern: Pattern) -> RebuiltPattern {
    let sfx = [0, 1, 2, 3].map(|channel| pattern.channel(channel).map(|slot| cart.sfx(slot)));
    let Some((timing_channel, ticks)) = timing_of(&sfx) else {
        return RebuiltPattern {
            chain_rows: Vec::new(),
            only_bit_7: false,
        };
    };
    // Only a LEN above 32 lasts longer than the 32 Steps a Phrase of that channel can hold.
    let row_ticks = sfx[timing_channel].map_or(1, speed_of) * SFX_NOTES as u32;
    let rows = ticks.div_ceil(row_ticks);
    let chain_rows = (0..rows)
        .map(|row| {
            let from = row * row_ticks;
            let to = ticks.min(from + row_ticks);
            [0, 1, 2, 3].map(|channel| {
                let sets_length = channel == timing_channel;
                sfx[channel].map(|sfx| {
                    let may_fill = rows == 1 && !sets_length;
                    let mut rebuilt = rebuild_channel(sfx, from, to, may_fill);
                    let finished = rebuilt
                        .entry
                        .as_ref()
                        .is_some_and(|entry| entry.phrase.is_silent());
                    if row > 0 && !sets_length && finished {
                        rebuilt.entry = None;
                    }
                    rebuilt
                })
            })
        })
        .collect();
    RebuiltPattern {
        chain_rows,
        only_bit_7: sfx.iter().flatten().all(|sfx| has_bit_7(*sfx)),
    }
}
