//! The Keymap: key presses in, named actions out.

mod key;
mod pc_tracker;

pub use key::{Key, KeyPress, Modifiers};

use crate::app::{Action, KeyContext};
use std::collections::HashMap;

/// A set of key bindings.
#[derive(Clone, Debug)]
pub struct Keymap {
    bindings: HashMap<(Key, Modifiers), Action>,
    context_bindings: HashMap<(KeyContext, Key), Action>,
}

impl Keymap {
    /// The PC tracker preset.
    pub fn pc_tracker() -> Self {
        let bindings = pc_tracker::bindings()
            .into_iter()
            .map(|(key, modifiers, action)| ((key, modifiers), action))
            .collect();
        let context_bindings = pc_tracker::context_bindings()
            .into_iter()
            .map(|(context, key, action)| ((context, key), action))
            .collect();
        Self {
            bindings,
            context_bindings,
        }
    }

    /// The action a key press triggers wherever the cursor is, if any. Only cursor movement and
    /// nudges repeat while a key is held.
    pub fn action_for(&self, press: KeyPress) -> Option<Action> {
        let action = *self
            .bindings
            .get(&(press.key.normalised(), press.modifiers))?;
        (!press.repeat || matches!(action, Action::Move(_) | Action::Nudge { .. }))
            .then_some(action)
    }

    /// The action a key press triggers with the cursor in this context, if any. Keys that type
    /// characters depend on the context, and they do not repeat while held.
    pub fn action_in(&self, press: KeyPress, context: KeyContext) -> Option<Action> {
        if matches!(context, KeyContext::FileName | KeyContext::InstrumentName)
            && let Some(action) = file_name_action(press)
        {
            return Some(action);
        }
        let typed = (press.modifiers == Modifiers::NONE && !press.repeat)
            .then(|| {
                self.context_bindings
                    .get(&(context, press.key.normalised()))
            })
            .flatten();
        typed.copied().or_else(|| self.action_for(press))
    }
}

/// What a key does while a file name or an Instrument name is being typed. Characters keep their case, and they repeat
/// while held, as they do in any text field.
fn file_name_action(press: KeyPress) -> Option<Action> {
    if press.modifiers.ctrl || press.modifiers.alt {
        return None;
    }
    match press.key {
        Key::Char(ch) => Some(Action::TypeChar(ch)),
        Key::Backspace => Some(Action::Backspace),
        _ => None,
    }
}
