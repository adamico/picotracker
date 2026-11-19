//! Quitting through the headless App core. The Window shell closes once the App core says to,
//! both for Ctrl+Q and for the window's close button.

mod support;

use picotracker_core::app::{Action, App, KeyContext};
use picotracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use support::status_line;

const UNSAVED_QUESTION: &str = "UNSAVED CHANGES WILL BE LOST  ENTER DISCARD  ESC CANCEL";

fn edit(app: &mut App) {
    app.apply(Action::HexDigit(1));
}

#[test]
fn ctrl_q_quits() {
    let keymap = Keymap::pc_tracker();
    let press = KeyPress::new(Key::Char('q'), Modifiers::CTRL);

    for context in [
        KeyContext::Navigation,
        KeyContext::PhrasePitch,
        KeyContext::FileName,
        KeyContext::MidiImport,
    ] {
        assert_eq!(keymap.action_in(press, context), Some(Action::Quit));
    }
}

#[test]
fn a_project_with_no_unsaved_changes_quits_at_once() {
    let mut app = App::new();
    assert!(!app.wants_to_quit());

    app.apply(Action::Quit);

    assert!(app.wants_to_quit());
}

#[test]
fn quit_asks_first_when_there_are_unsaved_changes() {
    let mut app = App::new();
    edit(&mut app);

    app.apply(Action::Quit);

    assert!(!app.wants_to_quit());
    assert_eq!(status_line(&app).trim_end(), UNSAVED_QUESTION);
}

#[test]
fn enter_discards_the_changes_and_quits() {
    let mut app = App::new();
    edit(&mut app);
    app.apply(Action::Quit);

    app.apply(Action::Confirm);

    assert!(app.wants_to_quit());
}

#[test]
fn esc_keeps_the_project_open() {
    let mut app = App::new();
    edit(&mut app);
    app.apply(Action::Quit);

    app.apply(Action::Cancel);

    assert!(!app.wants_to_quit());
    assert!(app.has_unsaved_changes());
}

#[test]
fn quit_works_from_inside_the_file_list() {
    let mut app = App::new();
    app.apply(Action::OpenProject);

    app.apply(Action::Quit);

    assert!(app.wants_to_quit());
}

#[test]
fn quit_while_another_question_is_open_asks_about_quitting() {
    let mut app = App::new();
    edit(&mut app);
    app.apply(Action::NewProject);

    app.apply(Action::Quit);
    app.apply(Action::Confirm);

    assert!(app.wants_to_quit());
}
