//! Cart Import: turns a Cart and a chosen song into a Project and warnings.
//!
//! This covers the first song of the Cart. Each Pattern is rebuilt from what PICO-8 plays, not from
//! the stored SFX (ADR 0006).

mod rebuild;

use crate::cart::{Cart, SLOTS};
use crate::project::{
    Chain, ChainEntry, ChainId, Claim, EXPORT_CHANNELS, Phrase, PhraseId, Project, TargetCart,
};
use rebuild::{has_bit_7, rebuild_pattern};
use std::collections::BTreeSet;

const LAST_SLOT: u8 = SLOTS as u8 - 1;

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
    fn general(text: String) -> Self {
        Self { text, place: None }
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
            warnings.push(ImportWarning::general(
                "CART HOLDS MORE MUSIC: IMPORTED SONG 1 ONLY".to_owned(),
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

/// Says which Step was lost where a Pattern ends inside it.
fn lost_step_warning(
    pattern: u8,
    place: SongPlace,
    phrase: Option<(PhraseId, usize)>,
) -> ImportWarning {
    let track = place.track + 1;
    let (lost, step) = match phrase {
        Some((id, length)) => (
            format!("T{track} PHRASE {:02X} LOST ITS LAST STEP.", id.number()),
            length - 1,
        ),
        None => (format!("T{track} LOST ITS ONLY STEP."), 0),
    };
    ImportWarning {
        text: format!("PATTERN {pattern:02X} ENDS INSIDE A STEP: {lost}"),
        place: Some(SongPlace { step, ..place }),
    }
}

/// Writes a song into the Project, one Song row per Pattern, and says what it claims.
fn build_song(
    cart: &Cart,
    song: SongRange,
    project: &mut Project,
    warnings: &mut Vec<ImportWarning>,
) -> Claim {
    let mut shared = Shared::default();
    let mut claimed = BTreeSet::new();
    let mut with_instruments = BTreeSet::new();
    let mut spd_0_warned = BTreeSet::new();
    let mut bit_7_warned = BTreeSet::new();
    for (row, slot) in (song.first..=song.last).enumerate() {
        let pattern = cart.pattern(slot);
        claimed.extend((0..EXPORT_CHANNELS).filter_map(|channel| pattern.channel(channel)));
        let rebuilt = rebuild_pattern(cart, pattern);
        if rebuilt.only_bit_7 {
            let track = (0..EXPORT_CHANNELS).find(|&c| pattern.channel(c).is_some());
            warnings.push(ImportWarning {
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
        for channel in 0..EXPORT_CHANNELS {
            if let Some(played) = pattern.channel(channel) {
                let place = SongPlace {
                    song_row: row,
                    track: channel,
                    chain_row: 0,
                    step: 0,
                };
                let sfx = cart.sfx(played);
                if sfx.speed() == 0 && spd_0_warned.insert(played) {
                    warnings.push(ImportWarning {
                        text: format!(
                            "SFX {played:02X} HAS SPD 0: SET TO SPEED 1, TIMING FOLLOWS PICO-8."
                        ),
                        place: Some(place),
                    });
                }
                if has_bit_7(sfx) && bit_7_warned.insert(played) {
                    warnings.push(ImportWarning {
                        text: format!(
                            "SFX {played:02X} HAS BIT 7 SET: IMPORTED AS A PLAIN PHRASE THAT \
                             NEVER SETS THE PATTERN LENGTH."
                        ),
                        place: Some(place),
                    });
                }
            }
            let mut chain = Chain::default();
            for (k, chain_row) in rebuilt.chain_rows.iter().enumerate() {
                let (Some(played), Some(rebuilt)) = (pattern.channel(channel), &chain_row[channel])
                else {
                    continue;
                };
                let place = SongPlace {
                    song_row: row,
                    track: channel,
                    chain_row: k,
                    step: 0,
                };
                let mut phrase = None;
                if let Some(entry) = &rebuilt.entry {
                    let id = shared.phrase(project, entry.phrase);
                    let mut chain_entry = ChainEntry::new(id);
                    chain_entry.set_fill(entry.fill);
                    *chain.entry_mut(k) = Some(chain_entry);
                    phrase = Some((id, entry.phrase.length()));
                    if !entry.instruments.is_empty() {
                        with_instruments.insert(played);
                    }
                    claimed.extend(entry.instruments.iter().copied());
                }
                if rebuilt.lost_step {
                    warnings.push(lost_step_warning(slot, place, phrase));
                }
                if let Some(needed) = rebuilt.unroll_needed {
                    warnings.push(ImportWarning {
                        text: format!(
                            "SFX {played:02X} LEAD-IN NEEDS {needed} STEPS UNROLLED: KEPT AS FILL \
                             OVER THE LOOP BODY ONLY."
                        ),
                        place: Some(place),
                    });
                }
            }
            let chain = shared.chain(project, chain);
            project.set_song_cell(row, channel, Some(chain));
        }
    }
    project.set_song_loop_row(song.loop_row);
    warnings.extend(
        with_instruments
            .iter()
            .map(|slot| format!("SFX {slot:02X} PLAYS SFX INSTRUMENTS AS BUILT-IN ONES"))
            .map(ImportWarning::general),
    );
    Claim {
        sfx: claimed.into_iter().collect(),
        patterns: Some((song.first, song.last)),
    }
}
