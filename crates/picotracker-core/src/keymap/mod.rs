//! The Keymap: key presses in, named actions out. A binding's trigger is a single key press, a
//! hold-chord or a double-tap, so the Keyboard remembers which keys are held and the last tap.

mod key;
mod m8_style;
mod names;
mod pc_tracker;
mod shared;
mod trigger;

pub use key::{Key, KeyPress, Modifiers};
pub use names::named_actions;
pub use trigger::{Scope, Trigger};

use crate::app::{Action, KeyContext};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

pub const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(300);

/// One key binding of a Keymap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub scope: Scope,
    pub trigger: Trigger,
    pub action: Action,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub enum Preset {
    #[default]
    PcTracker,
    M8Style,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RebindProblem {
    UnknownAction(String),
    UnreadableKey { action: String, key: String },
}

/// A set of key bindings.
#[derive(Clone, Debug)]
pub struct Keymap {
    bindings: HashMap<(Scope, Trigger), Action>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::preset(Preset::default())
    }
}

impl Keymap {
    /// A shipped preset as it is.
    pub fn preset(preset: Preset) -> Self {
        let bindings = match preset {
            Preset::PcTracker => pc_tracker::bindings(),
            Preset::M8Style => m8_style::bindings(),
        };
        Self {
            bindings: bindings
                .into_iter()
                .map(|binding| ((binding.scope, binding.trigger), binding.action))
                .collect(),
        }
    }

    /// The PC tracker preset.
    pub fn pc_tracker() -> Self {
        Self::preset(Preset::PcTracker)
    }

    /// The M8-style preset.
    pub fn m8_style() -> Self {
        Self::preset(Preset::M8Style)
    }

    /// A preset with the user's changed keys, by action name, and what could not be used. A
    /// changed key replaces the action's preset keys in the widest scope the preset binds it in,
    /// and takes the key from any action that had it there. Narrower keys stay, such as P in the
    /// song picker beside Space.
    pub fn rebound(preset: Preset, keys: &BTreeMap<String, String>) -> (Self, Vec<RebindProblem>) {
        let mut keymap = Self::preset(preset);
        let mut problems = Vec::new();
        for (name, text) in keys {
            let Some(action) = names::action_named(name) else {
                problems.push(RebindProblem::UnknownAction(name.clone()));
                continue;
            };
            let Some(trigger) = Trigger::parse(text) else {
                problems.push(RebindProblem::UnreadableKey {
                    action: name.clone(),
                    key: text.clone(),
                });
                continue;
            };
            keymap.rebind(action, trigger);
        }
        (keymap, problems)
    }

    fn rebind(&mut self, action: Action, trigger: Trigger) {
        let scopes: Vec<Scope> = self
            .bindings
            .iter()
            .filter(|(_, bound)| **bound == action)
            .map(|((scope, _), _)| *scope)
            .collect();
        let widest = scopes.iter().map(|scope| scope.width()).max();
        let replaced: Vec<Scope> = match widest {
            Some(widest) => scopes
                .into_iter()
                .filter(|scope| scope.width() == widest)
                .collect(),
            None => vec![Scope::Anywhere],
        };
        self.bindings
            .retain(|(scope, _), bound| !(*bound == action && replaced.contains(scope)));
        for scope in replaced {
            self.bindings.insert((scope, trigger), action);
        }
    }

    /// Every binding.
    pub fn bindings(&self) -> Vec<Binding> {
        self.bindings
            .iter()
            .map(|(&(scope, trigger), &action)| Binding {
                scope,
                trigger,
                action,
            })
            .collect()
    }

    /// The action a single key press triggers wherever the cursor is, if any. Only cursor
    /// movement and nudges repeat while a key is held.
    pub fn action_for(&self, press: KeyPress) -> Option<Action> {
        let (_, action) = self.lookup(press, &[Scope::Anywhere], &[], false)?;
        repeats_or_new(press, action)
    }

    /// The action a single key press triggers with the cursor in this context, if any. Keys that
    /// type characters depend on the context, and they do not repeat while held.
    pub fn action_in(&self, press: KeyPress, context: KeyContext) -> Option<Action> {
        if typing_a_name(context)
            && let Some(action) = name_action(press)
        {
            return Some(action);
        }
        let (_, action) = self.lookup(press, &scopes(context, false), &[], false)?;
        repeats_or_new(press, action)
    }

    /// The binding a key press sets off: a double-tap first, then a chord with a held key, then
    /// the press itself, each looked for in the scopes in order.
    fn lookup(
        &self,
        press: KeyPress,
        scopes: &[Scope],
        held: &[Key],
        double: bool,
    ) -> Option<(Trigger, Action)> {
        let key = press.key.normalised();
        let mut triggers = Vec::new();
        if double {
            triggers.push(Trigger::DoubleTap(key));
        }
        if press.modifiers == Modifiers::NONE {
            triggers.extend(held.iter().rev().map(|&held| Trigger::Chord { held, key }));
        }
        triggers.push(Trigger::Press(key, press.modifiers));
        triggers.into_iter().find_map(|trigger| {
            scopes
                .iter()
                .find_map(|&scope| self.bindings.get(&(scope, trigger)))
                .map(|&action| (trigger, action))
        })
    }

    /// Whether the key is held first in any chord of these scopes.
    fn starts_chords(&self, key: Key, scopes: &[Scope]) -> bool {
        self.bindings.keys().any(|(scope, trigger)| {
            scopes.contains(scope) && matches!(trigger, Trigger::Chord { held, .. } if *held == key)
        })
    }
}

/// The Keymap with the keys held now and the last tap, which chords and double-taps need.
#[derive(Clone, Debug, Default)]
pub struct Keyboard {
    keymap: Keymap,
    held: Vec<Key>,
    last_tap: Option<(Key, Duration)>,
    on_release: Option<(Key, Action)>,
}

impl Keyboard {
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            ..Self::default()
        }
    }

    /// The action a key going down triggers, if any, with the cursor in this context and a
    /// selection active or not. `at` is the time of the press, from any fixed start. A key that is
    /// held first in a chord acts when it goes up instead, unless another key went down meanwhile,
    /// so that holding it for a chord does not also act.
    pub fn key_down(
        &mut self,
        press: KeyPress,
        context: KeyContext,
        selecting: bool,
        at: Duration,
    ) -> Option<Action> {
        if typing_a_name(context)
            && let Some(action) = name_action(press)
        {
            return Some(action);
        }
        let key = press.key.normalised();
        let mut double = false;
        if !press.repeat {
            let alone = press.modifiers == Modifiers::NONE;
            double = alone
                && self.last_tap.is_some_and(|(tapped, when)| {
                    tapped == key && at.saturating_sub(when) <= DOUBLE_TAP_WINDOW
                });
            self.last_tap = (alone && !double).then_some((key, at));
            self.on_release = None;
            if !self.held.contains(&key) {
                self.held.push(key);
            }
        }
        let scopes = scopes(context, selecting);
        let others: Vec<Key> = self.held.iter().copied().filter(|&k| k != key).collect();
        let (trigger, action) = self.keymap.lookup(press, &scopes, &others, double)?;
        let waits = !press.repeat
            && matches!(trigger, Trigger::Press(_, Modifiers::NONE))
            && self.keymap.starts_chords(key, &scopes);
        if waits {
            self.on_release = Some((key, action));
            return None;
        }
        repeats_or_new(press, action)
    }

    /// The action a key going up triggers: the action it held back for a chord that never came.
    pub fn key_up(&mut self, key: Key) -> Option<Action> {
        let key = key.normalised();
        self.held.retain(|&held| held != key);
        self.on_release
            .take_if(|(waiting, _)| *waiting == key)
            .map(|(_, action)| action)
    }
}

/// The scopes a key press looks in, in order.
fn scopes(context: KeyContext, selecting: bool) -> Vec<Scope> {
    let mut scopes = vec![Scope::Context(context)];
    if selecting {
        scopes.push(Scope::Selecting);
    }
    scopes.push(Scope::Anywhere);
    scopes
}

/// A held key repeats only cursor movement and nudges.
fn repeats_or_new(press: KeyPress, action: Action) -> Option<Action> {
    (!press.repeat || matches!(action, Action::Move(_) | Action::Nudge { .. })).then_some(action)
}

const fn typing_a_name(context: KeyContext) -> bool {
    matches!(context, KeyContext::FileName | KeyContext::InstrumentName)
}

/// What a key does while a file name or an Instrument name is being typed. Characters keep their
/// case, and they repeat while held, as they do in any text field.
fn name_action(press: KeyPress) -> Option<Action> {
    if press.modifiers.ctrl || press.modifiers.alt {
        return None;
    }
    match press.key {
        Key::Char(ch) => Some(Action::TypeChar(ch)),
        Key::Backspace => Some(Action::Backspace),
        _ => None,
    }
}
