//! Translates `winit` key events into the Keymap's own key presses.

use picotracker_core::keymap::{Key, KeyPress, Modifiers};
use winit::event::KeyEvent;
use winit::keyboard::{Key as WinitKey, ModifiersState, NamedKey};

/// The key press for a `winit` key event, if the Keymap knows the key.
pub fn key_press(event: &KeyEvent, modifiers: ModifiersState) -> Option<KeyPress> {
    let key = key(&event.logical_key)?;
    let modifiers = Modifiers {
        ctrl: modifiers.control_key(),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    };
    Some(if event.repeat {
        KeyPress::repeated(key, modifiers)
    } else {
        KeyPress::new(key, modifiers)
    })
}

fn key(key: &WinitKey) -> Option<Key> {
    match key {
        WinitKey::Named(named) => named_key(*named),
        WinitKey::Character(text) => match text.chars().next()? {
            ' ' => Some(Key::Space),
            ch => Some(Key::Char(ch)),
        },
        _ => None,
    }
}

fn named_key(key: NamedKey) -> Option<Key> {
    Some(match key {
        NamedKey::ArrowUp => Key::Up,
        NamedKey::ArrowDown => Key::Down,
        NamedKey::ArrowLeft => Key::Left,
        NamedKey::ArrowRight => Key::Right,
        NamedKey::Tab => Key::Tab,
        NamedKey::Space => Key::Space,
        NamedKey::Enter => Key::Enter,
        NamedKey::Escape => Key::Escape,
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Insert => Key::Insert,
        NamedKey::Delete => Key::Delete,
        NamedKey::F1 => Key::F(1),
        NamedKey::F2 => Key::F(2),
        NamedKey::F3 => Key::F(3),
        NamedKey::F4 => Key::F(4),
        NamedKey::F5 => Key::F(5),
        NamedKey::F6 => Key::F(6),
        NamedKey::F7 => Key::F(7),
        NamedKey::F8 => Key::F(8),
        NamedKey::F9 => Key::F(9),
        NamedKey::F10 => Key::F(10),
        _ => return None,
    })
}
