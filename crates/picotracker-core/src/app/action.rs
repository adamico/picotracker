//! The named actions that drive the App core. Every key press becomes one of these.

/// A direction the cursor can move in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// The change in column and row.
    pub(super) const fn delta(self) -> (isize, isize) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

/// A named, rebindable action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Move(Direction),
    FocusNext,
    FocusPrevious,
    NewProject,
    OpenProject,
    Save,
    SaveAs,
    Import,
    Export,
    NewUnused,
    StartSelection,
    Copy,
    Cut,
    Paste,
    Clone,
    DeepClone,
    Undo,
    Redo,
    InsertRow,
    DeleteRow,
    ChangeSpeed,
    PurgeUnused,
    ToggleCart,
    ToggleLog,
    PlayFocusedPane,
    PlaySongFromCursor,
    /// Mutes a Track, numbered 1 to 8.
    Mute(u8),
    /// Solos a Track, numbered 1 to 8.
    Solo(u8),
    ClearMutesAndSolos,
    ToggleFollow,
    /// Enters a note: semitones above C of the keyboard's lower octave.
    PianoKey(u8),
    /// Types one hexadecimal digit into the cell under the cursor.
    HexDigit(u8),
    OctaveUp,
    OctaveDown,
    /// Removes the reference or value under the cursor.
    Clear,
    /// Marks the Chain entry under the cursor as Fill, or takes the mark away.
    ToggleFill,
    /// Makes the Song row under the cursor the Song loop row, or turns the loop off if it already
    /// is.
    SetSongLoopRow,
    /// Raises or lowers the cell under the cursor, by a small or a large amount.
    Nudge {
        up: bool,
        coarse: bool,
    },
}

/// What the cursor is on, which decides what the keys that type characters do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyContext {
    /// Nothing that takes typed characters.
    Navigation,
    /// A Step's pitch: letter and digit keys are a piano.
    PhrasePitch,
    /// Any other value in the Phrase pane: hexadecimal digits are typed directly.
    PhraseValue,
    /// A Chain or Phrase reference, or a transpose: hexadecimal digits are typed directly.
    ArrangeValue,
}
