//! What sets off a binding, and where it applies. A trigger is a key pressed with modifiers held
//! (`Ctrl+S`), a hold-chord of a key pressed while another is still held (`Z+X`), or a double-tap
//! of the same key twice in quick succession with nothing between (`Double-tap Z`). A binding for
//! the cursor's context comes before one for an active selection, which comes before one that
//! applies anywhere. Also reads triggers written as text, as `config.ron` holds them.

use super::key::{Key, Modifiers};
use crate::app::KeyContext;

const DOUBLE_TAP_PREFIX: &str = "Double-tap ";
const MAX_FUNCTION_KEY: u8 = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Trigger {
    Press(Key, Modifiers),
    Chord { held: Key, key: Key },
    DoubleTap(Key),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    Anywhere,
    Selecting,
    Context(KeyContext),
}

impl Scope {
    /// How widely the scope applies: anywhere is widest.
    pub(super) const fn width(self) -> u8 {
        match self {
            Scope::Context(_) => 0,
            Scope::Selecting => 1,
            Scope::Anywhere => 2,
        }
    }
}

impl Trigger {
    /// Reads a trigger written as `config.ron` holds it: `Ctrl+Shift+S`, `Z+X` or `Double-tap Z`.
    /// Names ignore case.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if let Some(rest) = text
            .get(..DOUBLE_TAP_PREFIX.len())
            .filter(|prefix| prefix.eq_ignore_ascii_case(DOUBLE_TAP_PREFIX))
            .map(|_| &text[DOUBLE_TAP_PREFIX.len()..])
        {
            return key_named(rest).map(Trigger::DoubleTap);
        }
        let mut modifiers = Modifiers::NONE;
        let mut keys = Vec::new();
        for part in text.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" => modifiers.ctrl = true,
                "shift" => modifiers.shift = true,
                "alt" => modifiers.alt = true,
                _ => keys.push(key_named(part)?),
            }
        }
        match keys[..] {
            [key] => Some(Trigger::Press(key, modifiers)),
            [held, key] if modifiers == Modifiers::NONE && held != key => {
                Some(Trigger::Chord { held, key })
            }
            _ => None,
        }
    }
}

/// A key by its name, such as `Up`, `F9`, `Space` or `Z`.
fn key_named(name: &str) -> Option<Key> {
    let name = name.trim();
    let mut chars = name.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        return Some(Key::Char(ch).normalised());
    }
    let lower = name.to_ascii_lowercase();
    Some(match lower.as_str() {
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "enter" => Key::Enter,
        "esc" | "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "ins" | "insert" => Key::Insert,
        "del" | "delete" => Key::Delete,
        _ => {
            let number: u8 = lower.strip_prefix('f')?.parse().ok()?;
            (1..=MAX_FUNCTION_KEY)
                .contains(&number)
                .then_some(Key::F(number))?
        }
    })
}
