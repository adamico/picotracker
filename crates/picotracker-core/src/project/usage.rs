//! Which Chains and Phrases are in use, and which slots are free for new ones.

use super::{
    CHAIN_ENTRIES, Chain, ChainId, ID_COUNT, Phrase, PhraseId, Project, SONG_ROWS, Step, TRACKS,
};

/// The Chains and Phrases that nothing refers to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnusedItems {
    pub chains: Vec<ChainId>,
    pub phrases: Vec<PhraseId>,
    /// How many of the Phrases no Chain plays at all, not even a Chain that is going too.
    pub loose_phrases: usize,
}

impl UnusedItems {
    /// Whether there is nothing to remove.
    pub fn is_empty(&self) -> bool {
        self.chains.is_empty() && self.phrases.is_empty()
    }
}

impl Phrase {
    /// Opens an empty Step at an index inside LEN. The last Step of LEN falls off, and Steps past
    /// LEN stay where they are.
    pub fn insert_step(&mut self, index: usize) {
        if index < self.length {
            self.steps[index..self.length].rotate_right(1);
            self.steps[index] = Step::default();
        }
    }

    /// Closes the Step at an index inside LEN. An empty Step appears at the end of LEN, and Steps
    /// past LEN stay where they are.
    pub fn delete_step(&mut self, index: usize) {
        if index < self.length {
            self.steps[index..self.length].rotate_left(1);
            self.steps[self.length - 1] = Step::default();
        }
    }
}

impl Chain {
    /// Opens an empty entry at an index from 0 to 15. The last entry falls off.
    pub fn insert_entry(&mut self, index: usize) {
        self.entries[index..].rotate_right(1);
        self.entries[index] = None;
    }

    /// Closes the entry at an index from 0 to 15. An empty entry appears at the end.
    pub fn delete_entry(&mut self, index: usize) {
        self.entries[index..].rotate_left(1);
        self.entries[CHAIN_ENTRIES - 1] = None;
    }

    /// The Phrases this Chain plays, each once, in the order they first appear.
    pub fn phrases(&self) -> Vec<PhraseId> {
        let mut phrases = Vec::new();
        for entry in self.entries.iter().flatten() {
            if !phrases.contains(&entry.phrase()) {
                phrases.push(entry.phrase());
            }
        }
        phrases
    }

    fn plays(&self, phrase: PhraseId) -> bool {
        self.entries
            .iter()
            .flatten()
            .any(|entry| entry.phrase() == phrase)
    }
}

impl Project {
    fn chain_ids() -> impl Iterator<Item = ChainId> {
        (0..ID_COUNT).map(|id| ChainId::new(id as u8))
    }

    fn phrase_ids() -> impl Iterator<Item = PhraseId> {
        (0..ID_COUNT).map(|id| PhraseId::new(id as u8))
    }

    /// Whether any Song cell holds this Chain.
    fn chain_is_placed(&self, chain: ChainId) -> bool {
        self.song.iter().flatten().any(|cell| *cell == Some(chain))
    }

    /// Whether any Chain, on the Song or not, plays this Phrase.
    fn phrase_is_played(&self, phrase: PhraseId) -> bool {
        self.chains.iter().any(|chain| chain.plays(phrase))
    }

    /// The Chain slots that hold nothing and that nothing points to, lowest first.
    pub fn free_chains(&self) -> Vec<ChainId> {
        Self::chain_ids()
            .filter(|id| *self.chain(*id) == Chain::default() && !self.chain_is_placed(*id))
            .collect()
    }

    /// The Phrase slots that hold nothing and that nothing points to, lowest first. A loose Phrase
    /// holds something, so its slot is never free.
    pub fn free_phrases(&self) -> Vec<PhraseId> {
        Self::phrase_ids()
            .filter(|id| *self.phrase(*id) == Phrase::default() && !self.phrase_is_played(*id))
            .collect()
    }

    /// Opens an empty Song row across every Track. The last row falls off, and the Song loop row
    /// stays on the music it marked.
    ///
    /// # Panics
    ///
    /// Panics if the row is 256 or more.
    pub fn insert_song_row(&mut self, row: usize) {
        self.song[row..].rotate_right(1);
        self.song[row] = [None; TRACKS];
        self.song_loop_row = self.song_loop_row.map(|loop_row| {
            if loop_row >= row {
                (loop_row + 1).min(SONG_ROWS - 1)
            } else {
                loop_row
            }
        });
    }

    /// Closes a Song row across every Track. An empty row appears at the end, and the Song loop
    /// row stays on the music it marked, or on the row that takes the place of a deleted one.
    ///
    /// # Panics
    ///
    /// Panics if the row is 256 or more.
    pub fn delete_song_row(&mut self, row: usize) {
        self.song[row..].rotate_left(1);
        self.song[SONG_ROWS - 1] = [None; TRACKS];
        self.song_loop_row = self
            .song_loop_row
            .map(|loop_row| loop_row - usize::from(loop_row > row));
    }

    /// What Purge unused would remove: the Chains the Song does not hold, and the Phrases no
    /// remaining Chain plays. Empty slots hold nothing to remove, so they are not counted.
    pub fn unused_items(&self) -> UnusedItems {
        let chains: Vec<ChainId> = Self::chain_ids()
            .filter(|id| *self.chain(*id) != Chain::default() && !self.chain_is_placed(*id))
            .collect();
        let kept_chain_plays = |phrase: PhraseId| {
            Self::chain_ids()
                .filter(|id| !chains.contains(id))
                .any(|id| self.chain(id).plays(phrase))
        };
        let phrases: Vec<PhraseId> = Self::phrase_ids()
            .filter(|id| *self.phrase(*id) != Phrase::default() && !kept_chain_plays(*id))
            .collect();
        let loose_phrases = phrases
            .iter()
            .filter(|id| !self.phrase_is_played(**id))
            .count();
        UnusedItems {
            chains,
            phrases,
            loose_phrases,
        }
    }

    /// Empties the slots of these Chains and Phrases.
    pub fn purge(&mut self, items: &UnusedItems) {
        for chain in &items.chains {
            *self.chain_mut(*chain) = Chain::default();
        }
        for phrase in &items.phrases {
            *self.phrase_mut(*phrase) = Phrase::default();
        }
    }

    /// An empty Phrase at the Speed that new Phrases take.
    pub fn new_phrase(&self) -> Phrase {
        let mut phrase = Phrase::default();
        phrase.set_speed(i32::from(self.default_speed));
        phrase
    }
}
