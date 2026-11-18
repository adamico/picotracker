//! Cart Import: turns a Cart and a chosen song into a Project and warnings.
//!
//! This covers the first song of the Cart. Each Pattern is rebuilt from what PICO-8 plays, not from
//! the stored SFX (ADR 0006).

mod rebuild;

use crate::cart::{Cart, SLOTS};
use crate::project::{
    Chain, ChainEntry, ChainId, Claim, EXPORT_CHANNELS, Phrase, PhraseId, Project, TargetCart,
};
use rebuild::{RebuiltChannel, RebuiltPattern, has_bit_7, rebuild_pattern};
use std::collections::BTreeSet;

const LAST_SLOT: u8 = SLOTS as u8 - 1;
/// The Phrases a Project can name.
const MAX_PHRASES: usize = 256;

/// A Project rebuilt from a Cart, and everything about it that differs from the Cart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedCart {
    pub project: Project,
    pub warnings: Vec<ImportWarning>,
}

/// Something about an Import that the user should know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportWarning {
    pub text: String,
    /// The place in the Song it is about, if it is about one place.
    pub place: Option<SongPlace>,
}

impl ImportWarning {
    fn general(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            place: None,
        }
    }

    fn at(place: SongPlace, text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            place: Some(place),
        }
    }
}

/// A place in the imported Song.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SongPlace {
    pub song_row: usize,
    pub track: usize,
    pub chain_row: usize,
    pub step: usize,
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

/// Gives equal Phrases and equal Chains one id each.
#[derive(Default)]
struct Shared {
    phrases: Vec<Phrase>,
    chains: Vec<Chain>,
}

impl Shared {
    /// The id of this Phrase, unless the Project has no id left for it.
    fn phrase(&mut self, project: &mut Project, phrase: Phrase) -> Option<PhraseId> {
        let known = self.phrases.iter().position(|known| *known == phrase);
        let index = match known {
            Some(index) => index,
            None if self.phrases.len() < MAX_PHRASES => {
                self.phrases.push(phrase);
                self.phrases.len() - 1
            }
            None => return None,
        };
        let id = PhraseId::new(index as u8);
        *project.phrase_mut(id) = phrase;
        Some(id)
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
        claim = SongBuilder::new(cart, &mut project, &mut warnings).build(song);
        let more_music = (song.last..LAST_SLOT).any(|slot| !cart.pattern(slot + 1).is_empty());
        if more_music {
            warnings.push(ImportWarning::general(
                "CART HOLDS MORE MUSIC: IMPORTED SONG 1 ONLY",
            ));
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

/// Says which Step was lost where a Chain row ends inside it.
fn lost_step_warning(
    pattern: u8,
    is_last_row: bool,
    place: SongPlace,
    phrase: Option<(PhraseId, usize)>,
) -> ImportWarning {
    let track = place.track + 1;
    let (lost, step) = match phrase {
        Some((id, length)) => (
            format!("T{track} PHRASE {:02X} LOST ITS LAST STEP.", id.number()),
            length - 1,
        ),
        None => (format!("T{track} LOST A STEP."), 0),
    };
    let ends = if is_last_row { "ENDS" } else { "SPLITS" };
    ImportWarning::at(
        SongPlace { step, ..place },
        format!("PATTERN {pattern:02X} {ends} INSIDE A STEP: {lost}"),
    )
}

/// Writes a song into a Project, one Song row per Pattern, gathering what it claims.
struct SongBuilder<'a> {
    cart: &'a Cart,
    project: &'a mut Project,
    warnings: &'a mut Vec<ImportWarning>,
    shared: Shared,
    /// The SFX the Patterns play directly.
    played: BTreeSet<u8>,
    /// The SFX that played notes use as instruments.
    instruments: BTreeSet<u8>,
    with_instruments: BTreeSet<u8>,
    spd_0_warned: BTreeSet<u8>,
    bit_7_warned: BTreeSet<u8>,
    chain_rows: usize,
    out_of_phrases: bool,
}

impl<'a> SongBuilder<'a> {
    fn new(cart: &'a Cart, project: &'a mut Project, warnings: &'a mut Vec<ImportWarning>) -> Self {
        Self {
            cart,
            project,
            warnings,
            shared: Shared::default(),
            played: BTreeSet::new(),
            instruments: BTreeSet::new(),
            with_instruments: BTreeSet::new(),
            spd_0_warned: BTreeSet::new(),
            bit_7_warned: BTreeSet::new(),
            chain_rows: 0,
            out_of_phrases: false,
        }
    }

    /// Writes the song and says what it claims.
    fn build(mut self, song: SongRange) -> Claim {
        for (row, slot) in (song.first..=song.last).enumerate() {
            self.add_pattern(row, slot);
        }
        self.project.set_song_loop_row(song.loop_row);
        self.warn_about_the_whole_song(song);
        let mut claimed = self.played;
        claimed.extend(self.instruments);
        Claim {
            sfx: claimed.into_iter().collect(),
            patterns: Some((song.first, song.last)),
        }
    }

    fn add_pattern(&mut self, row: usize, slot: u8) {
        let pattern = self.cart.pattern(slot);
        let rebuilt = rebuild_pattern(self.cart, pattern);
        self.chain_rows += rebuilt.chain_rows.len();
        if rebuilt.only_bit_7 {
            let track = (0..EXPORT_CHANNELS).find(|&track| pattern.channel(track).is_some());
            self.warnings.push(ImportWarning {
                text: format!(
                    "PATTERN {slot:02X} HOLDS ONLY BIT 7 SFX: THE CART STALLS HERE, THE EXPORTED \
                     SONG PLAYS THROUGH."
                ),
                place: track.map(|track| SongPlace {
                    song_row: row,
                    track,
                    chain_row: 0,
                    step: 0,
                }),
            });
        }
        for track in 0..EXPORT_CHANNELS {
            let mut chain = Chain::default();
            if let Some(played) = pattern.channel(track) {
                self.played.insert(played);
                self.warn_about_sfx(played, row, track);
                chain = self.chain_of(slot, played, row, track, &rebuilt);
            }
            let chain = self.shared.chain(self.project, chain);
            self.project.set_song_cell(row, track, Some(chain));
        }
    }

    /// Warns once per SFX about what its own bytes say.
    fn warn_about_sfx(&mut self, played: u8, song_row: usize, track: usize) {
        let place = SongPlace {
            song_row,
            track,
            chain_row: 0,
            step: 0,
        };
        let sfx = self.cart.sfx(played);
        if sfx.speed() == 0 && self.spd_0_warned.insert(played) {
            self.warnings.push(ImportWarning::at(
                place,
                format!("SFX {played:02X} HAS SPD 0: SET TO SPEED 1, TIMING FOLLOWS PICO-8."),
            ));
        }
        if has_bit_7(sfx) && self.bit_7_warned.insert(played) {
            self.warnings.push(ImportWarning::at(
                place,
                format!(
                    "SFX {played:02X} HAS BIT 7 SET: IMPORTED AS A PLAIN PHRASE THAT NEVER SETS \
                     THE PATTERN LENGTH."
                ),
            ));
        }
    }

    /// The Chain of one channel of a Pattern, one entry per Chain row.
    fn chain_of(
        &mut self,
        slot: u8,
        played: u8,
        song_row: usize,
        track: usize,
        rebuilt: &RebuiltPattern,
    ) -> Chain {
        let mut chain = Chain::default();
        let last_row = rebuilt.chain_rows.len() - 1;
        for (k, chain_row) in rebuilt.chain_rows.iter().enumerate() {
            let Some(channel) = &chain_row[track] else {
                continue;
            };
            let place = SongPlace {
                song_row,
                track,
                chain_row: k,
                step: 0,
            };
            let phrase = self.place_entry(&mut chain, k, played, channel);
            let arpeggio_changed = channel.entry.as_ref().and_then(|e| e.arpeggio_changed);
            if let Some(step) = arpeggio_changed {
                self.warnings.push(ImportWarning::at(
                    SongPlace { step, ..place },
                    format!(
                        "PATTERN {slot:02X} T{} STEP {step:02X} PLAYS A DIFFERENT ARPEGGIO AFTER \
                         IMPORT.",
                        track + 1
                    ),
                ));
            }
            if channel.lost_step {
                let warning = lost_step_warning(slot, k == last_row, place, phrase);
                self.warnings.push(warning);
            }
            let sounds = channel
                .entry
                .as_ref()
                .is_some_and(|e| !e.phrase.is_silent());
            if channel.early_ticks > 0 && sounds {
                let ticks = channel.early_ticks;
                let plural = if ticks == 1 { "" } else { "S" };
                self.warnings.push(ImportWarning::at(
                    place,
                    format!(
                        "PATTERN {slot:02X} SPLITS INSIDE A STEP: T{} PLAYS ITS LATER STEPS \
                         {ticks} TICK{plural} EARLY.",
                        track + 1
                    ),
                ));
            }
            if let Some(needed) = channel.unroll_needed {
                self.warnings.push(ImportWarning::at(
                    place,
                    format!(
                        "SFX {played:02X} LEAD-IN NEEDS {needed} STEPS UNROLLED: KEPT AS FILL \
                         OVER THE LOOP BODY ONLY."
                    ),
                ));
            }
        }
        chain
    }

    /// Puts a channel's Phrase on a Chain row, and says which Phrase it is and how long.
    fn place_entry(
        &mut self,
        chain: &mut Chain,
        k: usize,
        played: u8,
        channel: &RebuiltChannel,
    ) -> Option<(PhraseId, usize)> {
        let entry = channel.entry.as_ref()?;
        if !entry.instruments.is_empty() {
            self.with_instruments.insert(played);
        }
        self.instruments.extend(entry.instruments.iter().copied());
        let Some(id) = self.shared.phrase(self.project, entry.phrase) else {
            self.out_of_phrases = true;
            return None;
        };
        let mut placed = ChainEntry::new(id);
        placed.set_fill(entry.fill);
        *chain.entry_mut(k) = Some(placed);
        Some((id, entry.phrase.length()))
    }

    fn warn_about_the_whole_song(&mut self, song: SongRange) {
        let instrument_warnings = self
            .with_instruments
            .iter()
            .map(|slot| format!("SFX {slot:02X} PLAYS SFX INSTRUMENTS AS BUILT-IN ONES"))
            .map(ImportWarning::general);
        self.warnings.extend(instrument_warnings);
        if self.out_of_phrases {
            self.warnings.push(ImportWarning::general(format!(
                "SONG NEEDS MORE THAN {MAX_PHRASES} PHRASES: SOME ENTRIES STAY EMPTY."
            )));
        }
        let claimed_patterns = usize::from(song.last - song.first) + 1;
        let claimed_sfx = self.played.len();
        let needed_sfx = self.shared.phrases.len();
        if self.chain_rows > claimed_patterns || needed_sfx > claimed_sfx {
            self.warnings.push(ImportWarning::general(format!(
                "SONG NEEDS {} PATTERNS AND {needed_sfx} SFX BUT THE CLAIM HOLDS \
                 {claimed_patterns} AND {claimed_sfx}: WIDEN THE CLAIM BEFORE EXPORT.",
                self.chain_rows
            )));
        }
    }
}
