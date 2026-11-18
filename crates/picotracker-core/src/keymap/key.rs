//! Key presses, independent of any windowing library.

/// A key on the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Tab,
    Space,
    Enter,
    Escape,
    Backspace,
    Insert,
    Delete,
    /// A function key, numbered from 1.
    F(u8),
    /// A key that types a character.
    Char(char),
}

impl Key {
    /// Letters are bound by their lowercase form, whatever Shift does to them.
    pub(super) fn normalised(self) -> Self {
        match self {
            Key::Char(ch) => Key::Char(ch.to_ascii_lowercase()),
            key => key,
        }
    }
}

/// The modifier keys held during a key press.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        ..Self::NONE
    };
    pub const SHIFT: Self = Self {
        shift: true,
        ..Self::NONE
    };
    pub const ALT: Self = Self {
        alt: true,
        ..Self::NONE
    };
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        shift: true,
        alt: false,
    };
}

/// One key going down, or repeating while it is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPress {
    pub key: Key,
    pub modifiers: Modifiers,
    pub repeat: bool,
}

impl KeyPress {
    /// A key going down.
    pub const fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            repeat: false,
        }
    }

    /// A key repeating while it is held.
    pub const fn repeated(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            repeat: true,
        }
    }
}
