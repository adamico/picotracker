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
}
