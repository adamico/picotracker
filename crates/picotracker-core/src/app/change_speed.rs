//! Change Speed: a new Speed typed for the Phrase on show scales many Phrases by the same ratio.

use super::messages::Report;
use super::restructure::counted;
use super::selection::{Block, CellPosition};
use super::{Action, App, CHAIN_ENTRIES, Focus, NO_PHRASE_HERE};
use crate::project::{ChainId, PhraseId, SONG_ROWS, TRACKS};

/// The most digits a Speed has in hexadecimal.
const SPEED_DIGITS: usize = 2;
const MIN_SPEED: u32 = 1;
const MAX_SPEED: u32 = 255;
/// Arpeggios change rate between this Speed and the next one up.
const ARPEGGIO_SPEED: u8 = 8;
const BAD_SPEED: &str = "SPEED MUST BE 01-FF  ";

/// The new Speed being typed on the status line. The scope is settled when the question opens, so
/// that nothing the Playhead does while the user types can change it.
#[derive(Debug)]
pub(super) struct SpeedEntry {
    shown: PhraseId,
    shown_speed: u8,
    phrases: Vec<PhraseId>,
    used_elsewhere: usize,
    digits: Vec<u8>,
    refused: bool,
    text: String,
}

impl SpeedEntry {
    fn new(
        shown: PhraseId,
        shown_speed: u8,
        (phrases, used_elsewhere): (Vec<PhraseId>, usize),
    ) -> Self {
        let mut entry = Self {
            shown,
            shown_speed,
            phrases,
            used_elsewhere,
            digits: Vec::new(),
            refused: false,
            text: String::new(),
        };
        entry.write_question();
        entry
    }

    /// The question on the status line.
    pub(super) fn text(&self) -> &str {
        &self.text
    }

    fn write_question(&mut self) {
        let typed: String = self
            .digits
            .iter()
            .map(|digit| format!("{digit:X}"))
            .collect();
        self.text = format!(
            "{}NEW SPEED FOR PHRASE {:02X} (NOW {:02X}): {typed:_<width$}  ENTER APPLY  ESC CANCEL",
            if self.refused { BAD_SPEED } else { "" },
            self.shown.number(),
            self.shown_speed,
            width = SPEED_DIGITS,
        );
    }

    fn type_digit(&mut self, digit: u8) {
        if self.digits.len() < SPEED_DIGITS {
            self.digits.push(digit);
        }
        self.refused = false;
        self.write_question();
    }

    fn backspace(&mut self) {
        self.digits.pop();
        self.refused = false;
        self.write_question();
    }

    fn refuse(&mut self) {
        self.refused = true;
        self.write_question();
    }

    /// The Speed typed so far, if it is one a Phrase can have.
    fn typed_speed(&self) -> Option<u8> {
        let speed = self
            .digits
            .iter()
            .fold(0, |speed, digit| speed << 4 | digit);
        (speed > 0).then_some(speed)
    }
}

/// A Speed scaled by a ratio, rounded to the nearest whole Speed and kept inside 1 to 255, and
/// whether that is exactly what the ratio asked for.
fn scaled(speed: u8, new: u8, old: u8) -> (u8, bool) {
    let (new, old) = (u32::from(new), u32::from(old));
    let product = u32::from(speed) * new;
    let rounded = (2 * product + old) / (2 * old);
    let kept = rounded.clamp(MIN_SPEED, MAX_SPEED);
    (kept as u8, product % old == 0 && kept == rounded)
}

/// Whether arpeggios play at a different rate after this change of Speed.
const fn crossed_arpeggio_speed(before: u8, after: u8) -> bool {
    (before <= ARPEGGIO_SPEED) != (after <= ARPEGGIO_SPEED)
}

fn all_chains() -> impl Iterator<Item = ChainId> {
    (0..=u8::MAX).map(ChainId::new)
}

/// Every cell of the Song.
fn whole_song() -> Block {
    Block::between(
        CellPosition { column: 0, row: 0 },
        CellPosition {
            column: TRACKS - 1,
            row: SONG_ROWS - 1,
        },
    )
}

impl App {
    /// Asks for the new Speed of the Phrase on show.
    pub(super) fn ask_for_speed(&mut self) {
        let Some(shown) = self.phrase_under_cursor() else {
            self.status = Some(NO_PHRASE_HERE);
            return;
        };
        let shown_speed = self.project.phrase(shown).speed();
        self.speed_entry = Some(SpeedEntry::new(shown, shown_speed, self.speed_scope()));
    }

    /// The distinct Phrases to scale, and how many of them are also used outside the scope.
    fn speed_scope(&self) -> (Vec<PhraseId>, usize) {
        match (self.focus, self.selected_block()) {
            (Focus::Song, Some(block)) => self.song_scope(block),
            (Focus::Chain, Some(block)) => self.chain_scope(block),
            _ => self.song_scope(whole_song()),
        }
    }

    /// The Phrases of the Chains in a block of Song cells. A Phrase is used elsewhere when a Chain
    /// outside the block plays it, or when one of the block's Chains is also placed outside it.
    fn song_scope(&self, block: Block) -> (Vec<PhraseId>, usize) {
        let chains = self.chains_in(block);
        let phrases = self.phrases_of(&chains);
        let outside: Vec<ChainId> = all_chains()
            .filter(|chain| !chains.contains(chain) || self.placed_outside(*chain, block))
            .collect();
        let played_outside = self.phrases_of(&outside);
        let used_elsewhere = phrases
            .iter()
            .filter(|phrase| played_outside.contains(phrase))
            .count();
        (phrases, used_elsewhere)
    }

    fn placed_outside(&self, chain: ChainId, block: Block) -> bool {
        (0..SONG_ROWS).any(|row| {
            (0..TRACKS).any(|track| {
                let inside = block.rows().contains(&row) && block.columns().contains(&track);
                !inside && self.project.song_cell(row, track) == Some(chain)
            })
        })
    }

    /// The Phrases of the selected entries of the Chain on show. A Phrase is used elsewhere when
    /// another entry of this Chain or any other Chain plays it.
    fn chain_scope(&self, block: Block) -> (Vec<PhraseId>, usize) {
        let Some(id) = self.chain_under_cursor() else {
            return (Vec::new(), 0);
        };
        let chain = self.project.chain(id);
        let phrases_at = |rows: &mut dyn Iterator<Item = usize>| -> Vec<PhraseId> {
            rows.filter_map(|row| chain.entry(row).map(|entry| entry.phrase()))
                .collect()
        };
        let mut phrases = phrases_at(&mut block.rows());
        let mut seen = Vec::new();
        phrases.retain(|phrase| {
            let first = !seen.contains(phrase);
            seen.push(*phrase);
            first
        });
        let others: Vec<ChainId> = all_chains().filter(|other| *other != id).collect();
        let mut played_outside = self.phrases_of(&others);
        played_outside.extend(phrases_at(
            &mut (0..CHAIN_ENTRIES).filter(|row| !block.rows().contains(row)),
        ));
        let used_elsewhere = phrases
            .iter()
            .filter(|phrase| played_outside.contains(phrase))
            .count();
        (phrases, used_elsewhere)
    }

    /// The question on the status line takes the keys while it is open.
    pub(super) fn use_speed_entry(&mut self, action: Action) {
        let Some(entry) = self.speed_entry.as_mut() else {
            return;
        };
        match action {
            Action::HexDigit(digit) => entry.type_digit(digit),
            Action::Backspace => entry.backspace(),
            Action::Confirm => match entry.typed_speed() {
                Some(speed) => self.apply_speed(speed),
                None => entry.refuse(),
            },
            Action::Cancel => {
                self.speed_entry = None;
                self.selection = None;
            }
            _ => {}
        }
    }

    /// Scales every Phrase in scope, and reports what that did.
    fn apply_speed(&mut self, new: u8) {
        let Some(entry) = self.speed_entry.take() else {
            return;
        };
        let (mut changed, mut inexact, mut crossed) = (0, 0, 0);
        for id in &entry.phrases {
            let before = self.project.phrase(*id).speed();
            let (after, exact) = scaled(before, new, entry.shown_speed);
            self.project.phrase_mut(*id).set_speed(i32::from(after));
            changed += usize::from(after != before);
            inexact += usize::from(!exact);
            crossed += usize::from(crossed_arpeggio_speed(before, after));
        }
        self.selection = None;
        let summary = format!(
            "{} CHANGED  {inexact} INEXACT  {} USED ELSEWHERE  {crossed} CROSSED SPEED 8",
            counted(changed, "PHRASE"),
            entry.used_elsewhere,
        );
        self.report(if inexact + entry.used_elsewhere + crossed > 0 {
            Report::warning(summary)
        } else {
            Report::info(summary)
        });
    }
}
