//! Clone, Deep clone, new unused Chains and Phrases, and Insert and Delete row.

use super::cell_edits::NO_STEP_HERE;
use super::files::Question;
use super::messages::Report;
use super::selection::CellRange;
use super::{App, CHAIN_ENTRIES, Chain, Focus, NO_CHAIN_HERE, NO_PHRASE_HERE, Phrase, Project};
use crate::project::{ChainEntry, ChainId, PhraseId, UnusedItems};
use std::collections::HashMap;

const NO_FREE_CHAIN: &str = "NO UNUSED CHAIN LEFT";
const NO_FREE_PHRASE: &str = "NO UNUSED PHRASE LEFT";
const NEEDS_A_REFERENCE: &str = "USE THE SONG OR CHAIN PANE";
const NOTHING_TO_PURGE: &str = "NOTHING TO PURGE";

/// A number of things, in the singular or the plural.
pub(super) fn counted(count: usize, thing: &str) -> String {
    let plural = if count == 1 { "" } else { "S" };
    format!("{count} {thing}{plural}")
}

/// How many Chains and Phrases these are, as messages say it.
fn counted_items(items: &UnusedItems) -> String {
    format!(
        "{} AND {}",
        counted(items.chains.len(), "CHAIN"),
        counted(items.phrases.len(), "PHRASE"),
    )
}

impl App {
    /// Copies the Chains or Phrases under the cursor or the selection, and points the cells at the
    /// copies. Items shared inside the selection stay shared with each other.
    pub(super) fn clone_items(&mut self, deep: bool) {
        let Some(cells) = self.target_cells() else {
            return;
        };
        match self.focus {
            Focus::Song => self.clone_chains(cells, deep),
            Focus::Chain => self.clone_phrases(cells),
            Focus::Phrase => self.status = Some(NEEDS_A_REFERENCE),
            Focus::RightSlot => {}
        }
    }

    fn clone_chains(&mut self, cells: CellRange, deep: bool) {
        let chains = self.chains_in(cells);
        if chains.is_empty() {
            self.status = Some(NO_CHAIN_HERE);
            return;
        }
        let phrases = if deep {
            self.phrases_of(&chains)
        } else {
            Vec::new()
        };
        let Some(new_chains) = self.free_chain_slots(chains.len()) else {
            return;
        };
        let Some(new_phrases) = self.free_phrase_slots(phrases.len()) else {
            return;
        };
        let phrase_copies = self.copy_phrases(&phrases, &new_phrases);
        let mut chain_copies = HashMap::new();
        for (chain, copy) in chains.into_iter().zip(new_chains) {
            let mut copied = *self.project.chain(chain);
            for index in 0..CHAIN_ENTRIES {
                if let Some(entry) = copied.entry_mut(index) {
                    let phrase = entry.phrase();
                    entry.set_phrase(*phrase_copies.get(&phrase).unwrap_or(&phrase));
                }
            }
            *self.project.chain_mut(copy) = copied;
            chain_copies.insert(chain, copy);
        }
        for row in cells.rows() {
            for track in cells.columns() {
                if let Some(copy) = self
                    .project
                    .song_cell(row, track)
                    .and_then(|chain| chain_copies.get(&chain))
                {
                    self.project.set_song_cell(row, track, Some(*copy));
                }
            }
        }
        self.selection = None;
    }

    fn clone_phrases(&mut self, cells: CellRange) {
        let Some(chain) = self.chain_under_cursor() else {
            self.status = Some(NO_CHAIN_HERE);
            return;
        };
        let mut phrases = Vec::new();
        for row in cells.rows() {
            if let Some(entry) = self.project.chain(chain).entry(row)
                && !phrases.contains(&entry.phrase())
            {
                phrases.push(entry.phrase());
            }
        }
        if phrases.is_empty() {
            self.status = Some(NO_PHRASE_HERE);
            return;
        }
        let Some(new_phrases) = self.free_phrase_slots(phrases.len()) else {
            return;
        };
        let copies = self.copy_phrases(&phrases, &new_phrases);
        for row in cells.rows() {
            if let Some(entry) = self.project.chain_mut(chain).entry_mut(row) {
                entry.set_phrase(copies[&entry.phrase()]);
            }
        }
        self.selection = None;
    }

    /// The distinct Chains in a cells of Song cells, in the order they first appear.
    pub(super) fn chains_in(&self, cells: CellRange) -> Vec<ChainId> {
        let mut chains = Vec::new();
        for row in cells.rows() {
            for track in cells.columns() {
                if let Some(chain) = self.project.song_cell(row, track)
                    && !chains.contains(&chain)
                {
                    chains.push(chain);
                }
            }
        }
        chains
    }

    /// The distinct Phrases these Chains play, in the order they first appear.
    pub(super) fn phrases_of(&self, chains: &[ChainId]) -> Vec<PhraseId> {
        let mut phrases = Vec::new();
        for chain in chains {
            for phrase in self.project.chain(*chain).phrases() {
                if !phrases.contains(&phrase) {
                    phrases.push(phrase);
                }
            }
        }
        phrases
    }

    fn copy_phrases(
        &mut self,
        phrases: &[PhraseId],
        new_phrases: &[PhraseId],
    ) -> HashMap<PhraseId, PhraseId> {
        let mut copies = HashMap::new();
        for (phrase, copy) in phrases.iter().zip(new_phrases) {
            *self.project.phrase_mut(*copy) = *self.project.phrase(*phrase);
            copies.insert(*phrase, *copy);
        }
        copies
    }

    /// This many free Chain slots, or a failure on the status line if there are too few.
    fn free_chain_slots(&mut self, count: usize) -> Option<Vec<ChainId>> {
        let free = self.project.free_chains();
        if free.len() < count {
            self.report(Report::failure(NO_FREE_CHAIN));
            return None;
        }
        Some(free[..count].to_vec())
    }

    /// This many free Phrase slots, or a failure on the status line if there are too few.
    fn free_phrase_slots(&mut self, count: usize) -> Option<Vec<PhraseId>> {
        let free = self.project.free_phrases();
        if free.len() < count {
            self.report(Report::failure(NO_FREE_PHRASE));
            return None;
        }
        Some(free[..count].to_vec())
    }

    /// Opens an empty row at the cursor, shifting later rows down.
    pub(super) fn insert_row(&mut self) {
        self.shift_rows(
            Project::insert_song_row,
            Chain::insert_entry,
            Phrase::insert_step,
        );
    }

    /// Closes the row at the cursor, shifting later rows up.
    pub(super) fn delete_row(&mut self) {
        self.shift_rows(
            Project::delete_song_row,
            Chain::delete_entry,
            Phrase::delete_step,
        );
    }

    fn shift_rows(
        &mut self,
        shift_song: fn(&mut Project, usize),
        shift_chain: fn(&mut Chain, usize),
        shift_phrase: fn(&mut Phrase, usize),
    ) {
        match self.focus {
            Focus::Song => shift_song(&mut self.project, self.song.row()),
            Focus::Chain => {
                let Some(chain) = self.chain_under_cursor() else {
                    self.status = Some(NO_CHAIN_HERE);
                    return;
                };
                shift_chain(self.project.chain_mut(chain), self.chain.entry());
            }
            Focus::Phrase => {
                let Some(phrase) = self.phrase_under_cursor() else {
                    self.status = Some(NO_PHRASE_HERE);
                    return;
                };
                let Some(cell) = self.phrase.cell() else {
                    self.status = Some(NO_STEP_HERE);
                    return;
                };
                shift_phrase(self.project.phrase_mut(phrase), cell.row);
            }
            Focus::RightSlot => return,
        }
        self.selection = None;
    }

    /// Asks whether to remove what nothing refers to, saying how much that is.
    pub(super) fn ask_to_purge(&mut self) {
        let unused = self.project.unused_items();
        if unused.is_empty() {
            self.status = Some(NOTHING_TO_PURGE);
            return;
        }
        let loose = match unused.loose_phrases {
            0 => String::new(),
            count => format!(" ({count} LOOSE)"),
        };
        self.selection = None;
        self.question = Some(Question::purge_unused(format!(
            "PURGE {}{loose}  ENTER PURGE  ESC CANCEL",
            counted_items(&unused),
        )));
    }

    /// Removes what nothing refers to.
    pub(super) fn purge_unused(&mut self) {
        let unused = self.project.unused_items();
        self.project.purge(&unused);
        self.report(Report::info(format!("PURGED {}", counted_items(&unused))));
    }

    /// Points the cell under the cursor at a new, empty Chain or Phrase.
    pub(super) fn new_unused(&mut self) {
        match self.focus {
            Focus::Song => {
                if let Some(slots) = self.free_chain_slots(1) {
                    let (row, track) = (self.song.row(), self.song.track());
                    self.project.set_song_cell(row, track, Some(slots[0]));
                }
            }
            Focus::Chain => self.new_unused_phrase(),
            Focus::Phrase => self.status = Some(NEEDS_A_REFERENCE),
            Focus::RightSlot => {}
        }
    }

    fn new_unused_phrase(&mut self) {
        let Some(chain) = self.chain_under_cursor() else {
            self.status = Some(NO_CHAIN_HERE);
            return;
        };
        let Some(slots) = self.free_phrase_slots(1) else {
            return;
        };
        *self.project.phrase_mut(slots[0]) = self.project.new_phrase();
        let index = self.chain.entry();
        let entry = self.project.chain_mut(chain).entry_mut(index);
        match entry {
            Some(entry) => entry.set_phrase(slots[0]),
            None => *entry = Some(ChainEntry::new(slots[0])),
        }
    }
}
