use super::{
    CHAIN_ENTRIES, Chain, ChainId, ID_COUNT, Phrase, PhraseId, Project, SONG_ROWS, Step, TRACKS,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnusedItems {
    pub chains: Vec<ChainId>,
    pub phrases: Vec<PhraseId>,
    pub loose_phrases: usize,
}

impl UnusedItems {
    pub fn is_empty(&self) -> bool {
        self.chains.is_empty() && self.phrases.is_empty()
    }
}

impl Phrase {
    pub fn insert_step(&mut self, index: usize) {
        if index < self.length {
            self.steps[index..self.length].rotate_right(1);
            self.steps[index] = Step::default();
        }
    }

    pub fn delete_step(&mut self, index: usize) {
        if index < self.length {
            self.steps[index..self.length].rotate_left(1);
            self.steps[self.length - 1] = Step::default();
        }
    }
}

impl Chain {
    pub fn insert_entry(&mut self, index: usize) {
        self.entries[index..].rotate_right(1);
        self.entries[index] = None;
    }

    pub fn delete_entry(&mut self, index: usize) {
        self.entries[index..].rotate_left(1);
        self.entries[CHAIN_ENTRIES - 1] = None;
    }

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

    fn chain_is_placed(&self, chain: ChainId) -> bool {
        self.song.iter().flatten().any(|cell| *cell == Some(chain))
    }

    fn phrase_is_played(&self, phrase: PhraseId) -> bool {
        self.chains.iter().any(|chain| chain.plays(phrase))
    }

    pub fn free_chains(&self) -> Vec<ChainId> {
        Self::chain_ids()
            .filter(|id| *self.chain(*id) == Chain::default() && !self.chain_is_placed(*id))
            .collect()
    }

    pub fn free_phrases(&self) -> Vec<PhraseId> {
        Self::phrase_ids()
            .filter(|id| *self.phrase(*id) == Phrase::default() && !self.phrase_is_played(*id))
            .collect()
    }

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

    pub fn delete_song_row(&mut self, row: usize) {
        self.song[row..].rotate_left(1);
        self.song[SONG_ROWS - 1] = [None; TRACKS];
        self.song_loop_row = self
            .song_loop_row
            .map(|loop_row| loop_row - usize::from(loop_row > row));
    }

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

    pub fn purge(&mut self, items: &UnusedItems) {
        for chain in &items.chains {
            *self.chain_mut(*chain) = Chain::default();
        }
        for phrase in &items.phrases {
            *self.phrase_mut(*phrase) = Phrase::default();
        }
    }

    pub fn new_phrase(&self) -> Phrase {
        let mut phrase = Phrase::default();
        phrase.set_speed(i32::from(self.default_speed));
        phrase
    }
}
