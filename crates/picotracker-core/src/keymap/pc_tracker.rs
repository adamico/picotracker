//! The PC tracker preset, the first-launch default.

use super::key::{Key, Modifiers};
use crate::app::{Action, Direction, KeyContext};

const NONE: Modifiers = Modifiers::NONE;
const CTRL: Modifiers = Modifiers::CTRL;
const SHIFT: Modifiers = Modifiers::SHIFT;
const CTRL_SHIFT: Modifiers = Modifiers::CTRL_SHIFT;

const LOWER_OCTAVE_KEYS: &str = "zsxdcvgbhnjm";
const UPPER_OCTAVE_KEYS: &str = "q2w3er5t6y7u";
const HEX_DIGITS: &str = "0123456789abcdef";

/// Every binding of the preset.
pub(super) fn bindings() -> Vec<(Key, Modifiers, Action)> {
    let mut bindings = vec![
        (Key::Up, NONE, Action::Move(Direction::Up)),
        (Key::Down, NONE, Action::Move(Direction::Down)),
        (Key::Left, NONE, Action::Move(Direction::Left)),
        (Key::Right, NONE, Action::Move(Direction::Right)),
        (Key::Tab, NONE, Action::FocusNext),
        (Key::Tab, SHIFT, Action::FocusPrevious),
        (Key::Enter, NONE, Action::Confirm),
        (Key::Escape, NONE, Action::Cancel),
        (Key::Char('n'), CTRL, Action::NewProject),
        (Key::Char('o'), CTRL, Action::OpenProject),
        (Key::Char('s'), CTRL, Action::Save),
        (Key::Char('s'), CTRL_SHIFT, Action::SaveAs),
        (Key::Char('i'), CTRL, Action::Import),
        (Key::Char('e'), CTRL, Action::Export),
        (Key::Char('q'), CTRL, Action::Quit),
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
        (Key::Delete, NONE, Action::Clear),
        (Key::Char('f'), CTRL, Action::ToggleFill),
        (Key::Char('r'), CTRL, Action::ToggleSongLoopRow),
        (Key::Char('['), NONE, Action::OctaveDown),
        (Key::Char(']'), NONE, Action::OctaveUp),
        (Key::Char('r'), SHIFT, Action::ResetMidiTable),
        (Key::Up, SHIFT, nudge(true, false)),
        (Key::Down, SHIFT, nudge(false, false)),
        (Key::Up, CTRL, nudge(true, true)),
        (Key::Down, CTRL, nudge(false, true)),
    ];
    for track_number in 1..=8 {
        let key = Key::F(track_number);
        bindings.push((key, NONE, Action::Mute { track_number }));
        bindings.push((key, SHIFT, Action::Solo { track_number }));
    }
    bindings
}

/// The keys that type characters, which depend on what the cursor is on.
pub(super) fn context_bindings() -> Vec<(KeyContext, Key, Action)> {
    let piano = LOWER_OCTAVE_KEYS.chars().chain(UPPER_OCTAVE_KEYS.chars());
    let piano = (0..).zip(piano).map(|(semitone, ch)| {
        (
            KeyContext::PhrasePitch,
            Key::Char(ch),
            Action::PianoKey(semitone),
        )
    });
    let digits = [
        KeyContext::PhraseValue,
        KeyContext::ReferenceValue,
        KeyContext::SpeedEntry,
    ]
    .into_iter()
    .flat_map(|context| {
        (0..)
            .zip(HEX_DIGITS.chars())
            .map(move |(digit, ch)| (context, Key::Char(ch), Action::HexDigit(digit)))
    });
    let backspace = (KeyContext::SpeedEntry, Key::Backspace, Action::Backspace);
    let midi_import = [
        (Key::Space, Action::ToggleVoice),
        (Key::Char('s'), Action::ToggleSource),
        (Key::Tab, Action::SwitchMidiTable),
        (Key::Char('['), Action::MidiDrumNote { up: false }),
        (Key::Char(']'), Action::MidiDrumNote { up: true }),
        (Key::Char('f'), Action::CycleDrumEffect),
        (Key::Char('r'), Action::ResetMidiRow),
    ]
    .map(|(key, action)| (KeyContext::MidiImport, key, action));
    let song_picker = (
        KeyContext::SongPicker,
        Key::Char('p'),
        Action::PlayFocusedPane,
    );
    piano
        .chain(digits)
        .chain([backspace, song_picker])
        .chain(midi_import)
        .collect()
}

const fn nudge(up: bool, coarse: bool) -> Action {
    Action::Nudge { up, coarse }
}
