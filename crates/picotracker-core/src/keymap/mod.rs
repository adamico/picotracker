//! The Keymap: key presses in, named actions out.

mod key;
mod pc_tracker;

pub use key::{Key, KeyPress, Modifiers};

use crate::app::Action;
use std::collections::HashMap;

/// A set of key bindings.
#[derive(Clone, Debug)]
pub struct Keymap {
    bindings: HashMap<(Key, Modifiers), Action>,
}

impl Keymap {
    /// The PC tracker preset.
    pub fn pc_tracker() -> Self {
        let bindings = pc_tracker::bindings()
            .into_iter()
            .map(|(key, modifiers, action)| ((key, modifiers), action))
            .collect();
        Self { bindings }
    }

    /// The action a key press triggers, if any. Only cursor movement repeats while a key is held.
    pub fn action_for(&self, press: KeyPress) -> Option<Action> {
        let action = *self
            .bindings
            .get(&(press.key.normalised(), press.modifiers))?;
        (!press.repeat || matches!(action, Action::Move(_))).then_some(action)
    }
}
