//! Cart Import: turns a Cart and a chosen song into a Project and warnings.
//!
//! This covers simple music: the first song of the Cart, whose Patterns play their SFX straight
//! through. A Pattern whose timing PICO-8 would play differently is imported as stored and warned
//! about.

use crate::cart::{Cart, SFX_NOTES, SLOTS, Sfx};
use crate::project::{
    Chain, ChainEntry, ChainId, Claim, EXPORT_CHANNELS, Filters, Phrase, PhraseId, Project, Step,
    TargetCart,
};
use std::collections::BTreeSet;

const LAST_SLOT: u8 = SLOTS as u8 - 1;
/// Bit 7 of byte 66 marks an SFX as a Waveform Instrument.
const WAVEFORM_FLAG: u8 = 0x80;
const NOIZ_BIT: u8 = 0x02;
const BUZZ_BIT: u8 = 0x04;
const DETUNE_DIVISOR: u8 = 8;
const REVERB_DIVISOR: u8 = 24;
const DAMPEN_DIVISOR: u8 = 72;
const FILTER_LEVELS: u8 = 3;

/// A Project rebuilt from a Cart, and everything about it that differs from the Cart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedCart {
    pub project: Project,
    pub warnings: Vec<String>,
}

/// The Patterns of one song.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SongRange {
    first: u8,
    last: u8,
    /// The Song row that LOOP BACK jumps to.
    loop_row: Option<usize>,
}

/// Finds the first song: it starts at the first non-empty Pattern and runs up to a STOP, a LOOP
/// BACK or an empty Pattern.
fn first_song(cart: &Cart) -> Option<SongRange> {
    let first = (0..=LAST_SLOT).find(|&slot| !cart.pattern(slot).is_empty())?;
    let mut last = first;
    while last < LAST_SLOT
        && !cart.pattern(last).stop()
        && !cart.pattern(last).loop_back()
        && !cart.pattern(last + 1).is_empty()
    {
        last += 1;
    }
    let ending = cart.pattern(last);
    let loop_row = (ending.loop_back() && !ending.stop())
        .then(|| {
            let target = (0..=last)
                .rev()
                .find(|&slot| cart.pattern(slot).loop_start())
                .unwrap_or(0);
            target.checked_sub(first).map(usize::from)
        })
        .flatten();
    Some(SongRange {
        first,
        last,
        loop_row,
    })
}

/// How many notes of an SFX play: its LEN if it has one, or else all of them.
fn played_notes(sfx: Sfx<'_>) -> usize {
    let start = usize::from(sfx.loop_start());
    if sfx.loop_end() == 0 && (1..=SFX_NOTES).contains(&start) {
        start
    } else {
        SFX_NOTES
    }
}

/// Whether an SFX plays once from its first note at a Speed the Project can hold.
fn plays_straight_through(sfx: Sfx<'_>) -> bool {
    let has_loop = sfx.loop_start() < sfx.loop_end();
    let long_len = sfx.loop_end() == 0 && usize::from(sfx.loop_start()) > SFX_NOTES;
    sfx.speed() > 0 && !has_loop && !long_len && sfx.loop_start() & WAVEFORM_FLAG == 0
}

/// Whether a Pattern lasts as long in the Project as in PICO-8: PICO-8 ends it with the left-most
/// channel, and the Project with the longest.
fn timing_is_simple(cart: &Cart, slot: u8) -> bool {
    let pattern = cart.pattern(slot);
    let sfx: Vec<Sfx<'_>> = (0..EXPORT_CHANNELS)
        .filter_map(|channel| pattern.channel(channel))
        .map(|played| cart.sfx(played))
        .collect();
    let ticks = |sfx: &Sfx<'_>| played_notes(*sfx) * usize::from(sfx.speed());
    let left_most = sfx.first().map_or(0, ticks);
    sfx.iter()
        .all(|sfx| plays_straight_through(*sfx) && ticks(sfx) <= left_most)
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

/// The Phrase holding every stored note of an SFX, one Step per note.
fn phrase_of(sfx: Sfx<'_>) -> Phrase {
    let mut phrase = Phrase::default();
    for index in 0..SFX_NOTES {
        let note = sfx.note(index);
        // The Cart codec hands out values that are always inside a Step's limits.
        *phrase.step_mut(index) =
            Step::new(note.pitch, note.instrument, note.volume, note.effect).unwrap_or_default();
    }
    phrase.set_length(played_notes(sfx) as i32);
    phrase.set_speed(i32::from(sfx.speed()));
    *phrase.filters_mut() = filters_of(sfx);
    phrase
}

/// The SFX slots that the audible notes of an SFX play as instruments.
fn instruments_of(sfx: Sfx<'_>) -> BTreeSet<u8> {
    (0..played_notes(sfx))
        .map(|index| sfx.note(index))
        .filter(|note| note.custom && note.volume > 0)
        .map(|note| note.instrument)
        .collect()
}

/// Gives equal Phrases and equal Chains one id each.
#[derive(Default)]
struct Shared {
    phrases: Vec<Phrase>,
    chains: Vec<Chain>,
}

impl Shared {
    fn phrase(&mut self, project: &mut Project, phrase: Phrase) -> PhraseId {
        let index = self.phrases.iter().position(|known| *known == phrase);
        let index = index.unwrap_or_else(|| {
            self.phrases.push(phrase);
            self.phrases.len() - 1
        });
        let id = PhraseId::new(index as u8);
        *project.phrase_mut(id) = phrase;
        id
    }

    fn chain(&mut self, project: &mut Project, chain: Chain) -> ChainId {
        let index = self.chains.iter().position(|known| *known == chain);
        let index = index.unwrap_or_else(|| {
            self.chains.push(chain);
            self.chains.len() - 1
        });
        let id = ChainId::new(index as u8);
        *project.chain_mut(id) = chain;
        id
    }
}

/// Rebuilds the first song of a Cart as a new Project whose Target Cart is at `cart_path`.
pub fn import(cart: &Cart, cart_path: &str) -> ImportedCart {
    let mut project = Project::new();
    for channel in 0..EXPORT_CHANNELS {
        project.set_export_track(channel, Some(channel as u8 + 1));
    }
    project.set_song_loop_row(None);

    let mut warnings = Vec::new();
    let mut claim = Claim::default();
    if let Some(song) = first_song(cart) {
        claim = build_song(cart, song, &mut project, &mut warnings);
        let more_music = (song.last..LAST_SLOT).any(|slot| !cart.pattern(slot + 1).is_empty());
        if more_music {
            warnings.push("CART HOLDS MORE MUSIC: IMPORTED SONG 1 ONLY".to_owned());
        }
    }

    let claimed_bytes_hash = Some(cart.claimed_bytes_hash(&claim));
    project.set_target_cart(Some(TargetCart {
        path: cart_path.to_owned(),
        claim,
        claimed_bytes_hash,
    }));
    ImportedCart { project, warnings }
}

/// Writes a song into the Project, one Song row per Pattern, and says what it claims.
fn build_song(
    cart: &Cart,
    song: SongRange,
    project: &mut Project,
    warnings: &mut Vec<String>,
) -> Claim {
    let mut shared = Shared::default();
    let mut claimed = BTreeSet::new();
    let mut with_instruments = BTreeSet::new();
    for (row, slot) in (song.first..=song.last).enumerate() {
        let pattern = cart.pattern(slot);
        if !timing_is_simple(cart, slot) {
            warnings.push(format!(
                "PATTERN {slot:02X} MAY NOT PLAY AS IN PICO-8: TIMING"
            ));
        }
        for channel in 0..EXPORT_CHANNELS {
            let mut chain = Chain::default();
            if let Some(played) = pattern.channel(channel) {
                let sfx = cart.sfx(played);
                let phrase = shared.phrase(project, phrase_of(sfx));
                *chain.entry_mut(0) = Some(ChainEntry::new(phrase));
                claimed.insert(played);
                let instruments = instruments_of(sfx);
                if !instruments.is_empty() {
                    with_instruments.insert(played);
                }
                claimed.extend(instruments);
            }
            let chain = shared.chain(project, chain);
            project.set_song_cell(row, channel, Some(chain));
        }
    }
    project.set_song_loop_row(song.loop_row);
    warnings.extend(
        with_instruments
            .iter()
            .map(|slot| format!("SFX {slot:02X} PLAYS SFX INSTRUMENTS AS BUILT-IN ONES")),
    );
    Claim {
        sfx: claimed.into_iter().collect(),
        patterns: Some((song.first, song.last)),
    }
}
