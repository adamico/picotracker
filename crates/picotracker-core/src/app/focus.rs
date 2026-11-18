//! Which pane has focus. Tab cycles four stops.

/// A stop in the focus cycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Song,
    Chain,
    Phrase,
    /// Whichever view the right-hand slot is showing.
    RightSlot,
}

impl Focus {
    pub const fn next(self) -> Self {
        match self {
            Focus::Song => Focus::Chain,
            Focus::Chain => Focus::Phrase,
            Focus::Phrase => Focus::RightSlot,
            Focus::RightSlot => Focus::Song,
        }
    }

    pub const fn previous(self) -> Self {
        match self {
            Focus::Song => Focus::RightSlot,
            Focus::Chain => Focus::Song,
            Focus::Phrase => Focus::Chain,
            Focus::RightSlot => Focus::Phrase,
        }
    }
}
