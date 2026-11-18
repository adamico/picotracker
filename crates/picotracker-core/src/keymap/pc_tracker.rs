//! The PC tracker preset, the first-launch default.

use super::key::{Key, Modifiers};
use crate::app::{Action, Direction};

const NONE: Modifiers = Modifiers::NONE;
const CTRL: Modifiers = Modifiers::CTRL;
const SHIFT: Modifiers = Modifiers::SHIFT;
const CTRL_SHIFT: Modifiers = Modifiers::CTRL_SHIFT;

/// Every binding of the preset.
pub(super) fn bindings() -> Vec<(Key, Modifiers, Action)> {
    let mut bindings = vec![
        (Key::Up, NONE, Action::Move(Direction::Up)),
        (Key::Down, NONE, Action::Move(Direction::Down)),
        (Key::Left, NONE, Action::Move(Direction::Left)),
        (Key::Right, NONE, Action::Move(Direction::Right)),
        (Key::Tab, NONE, Action::FocusNext),
        (Key::Tab, SHIFT, Action::FocusPrevious),
        (Key::Char('n'), CTRL, Action::NewProject),
        (Key::Char('o'), CTRL, Action::OpenProject),
        (Key::Char('s'), CTRL, Action::Save),
        (Key::Char('s'), CTRL_SHIFT, Action::SaveAs),
        (Key::Char('i'), CTRL, Action::Import),
        (Key::Char('e'), CTRL, Action::Export),
        (Key::Char('n'), CTRL_SHIFT, Action::NewUnused),
        (Key::Char('b'), CTRL, Action::StartSelection),
        (Key::Char('c'), CTRL, Action::Copy),
        (Key::Char('x'), CTRL, Action::Cut),
        (Key::Char('v'), CTRL, Action::Paste),
        (Key::Char('d'), CTRL, Action::Clone),
        (Key::Char('d'), CTRL_SHIFT, Action::DeepClone),
        (Key::Char('z'), CTRL, Action::Undo),
        (Key::Char('y'), CTRL, Action::Redo),
        (Key::Insert, NONE, Action::InsertRow),
        (Key::Delete, CTRL, Action::DeleteRow),
        (Key::Char('t'), CTRL_SHIFT, Action::ChangeSpeed),
        (Key::Char('p'), CTRL_SHIFT, Action::PurgeUnused),
        (Key::Char('t'), CTRL, Action::ToggleCart),
        (Key::Char('l'), CTRL, Action::ToggleLog),
        (Key::Space, NONE, Action::PlayFocusedPane),
        (Key::Space, SHIFT, Action::PlaySongFromCursor),
        (Key::F(9), NONE, Action::ClearMutesAndSolos),
        (Key::F(10), NONE, Action::ToggleFollow),
    ];
    for track in 1..=8 {
        bindings.push((Key::F(track), NONE, Action::Mute(track)));
        bindings.push((Key::F(track), SHIFT, Action::Solo(track)));
    }
    bindings
}
