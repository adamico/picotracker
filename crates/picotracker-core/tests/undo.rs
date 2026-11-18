//! One linear undo history for the whole Project, driven through named actions.

use picotracker_core::app::{Action, App, DirectoryEntry, Direction, FileRequest, FileResponse};
use picotracker_core::project::{ChainEntry, ChainId, PhraseId, Project};
use picotracker_core::project_file;
use std::path::PathBuf;

const DOWN: Action = Action::Move(Direction::Down);
const UP: Action = Action::Move(Direction::Up);
const NUDGE_UP: Action = Action::Nudge {
    up: true,
    coarse: false,
};
const NUDGE_DOWN: Action = Action::Nudge {
    up: false,
    coarse: false,
};
const NOTHING_TO_UNDO: &str = "NOTHING TO UNDO";
const NOTHING_TO_REDO: &str = "NOTHING TO REDO";

fn apply(app: &mut App, actions: &[Action]) {
    for action in actions {
        app.apply(*action);
    }
}

fn song_cell(app: &App, row: usize, track: usize) -> Option<u8> {
    app.project().song_cell(row, track).map(ChainId::number)
}

fn status_line(app: &App) -> String {
    app.grid().text_dump().lines().last().unwrap().to_owned()
}

/// A Project with Chain 00 on the first Song row, holding Phrase 00.
fn arranged() -> Project {
    let mut project = Project::new();
    let chain = ChainId::new(0);
    *project.chain_mut(chain).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    project.set_song_cell(0, 0, Some(chain));
    project
}

/// Saves the Project as `./demo.ptrk`, and gives back what was written.
fn save(app: &mut App) -> String {
    app.apply(Action::SaveAs);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: Vec::new(),
    });
    for ch in "demo.ptrk".chars() {
        app.apply(Action::TypeChar(ch));
    }
    app.apply(Action::Confirm);
    let mut requests = app.take_file_requests();
    let Some(FileRequest::Write { path, contents }) = requests.pop() else {
        panic!("expected a write");
    };
    app.receive_file(FileResponse::Written { path });
    contents
}

/// Opens a Project from a file through the file list.
fn open(app: &mut App, project: &Project) {
    app.apply(Action::OpenProject);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: "other.ptrk".to_owned(),
            is_directory: false,
        }],
    });
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let mut requests = app.take_file_requests();
    let Some(FileRequest::Read { path }) = requests.pop() else {
        panic!("expected a read");
    };
    app.receive_file(FileResponse::Read {
        path,
        contents: project_file::write(project),
    });
}

#[test]
fn undo_takes_back_a_song_edit_and_redo_puts_it_back() {
    let mut app = App::new();
    apply(&mut app, &[Action::HexDigit(3)]);
    let edited = app.project().clone();

    app.apply(Action::Undo);
    assert!(*app.project() == Project::new());

    app.apply(Action::Redo);
    assert!(*app.project() == edited);
}

#[test]
fn undo_moves_the_cursor_back_to_the_change() {
    let mut app = App::new();
    apply(&mut app, &[DOWN, Action::HexDigit(3), DOWN, DOWN]);

    app.apply(Action::Undo);
    app.apply(Action::HexDigit(7));

    assert_eq!(song_cell(&app, 1, 0), Some(7));
    assert_eq!(song_cell(&app, 3, 0), None);
}

#[test]
fn undo_brings_focus_back_to_the_pane_of_the_change() {
    let mut app = App::with_project(arranged());
    apply(&mut app, &[Action::HexDigit(5), Action::FocusNext]);

    app.apply(Action::Undo);
    app.apply(Action::HexDigit(6));

    assert_eq!(song_cell(&app, 0, 0), Some(6));
}

#[test]
fn edits_in_every_pane_are_undone_in_order() {
    let mut app = App::with_project(arranged());
    let start = app.project().clone();
    apply(
        &mut app,
        &[
            Action::SetSongLoopRow,
            Action::FocusNext,
            Action::ToggleFill,
            Action::FocusNext,
            Action::PianoKey(0),
            UP,
            UP,
            NUDGE_UP,
        ],
    );

    apply(&mut app, &[Action::Undo; 4]);

    assert!(*app.project() == start);
    app.apply(Action::Undo);
    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
}

#[test]
fn repeated_nudges_on_one_cell_are_one_step() {
    let mut app = App::with_project(arranged());
    let start = app.project().clone();
    apply(&mut app, &[NUDGE_UP, NUDGE_UP, NUDGE_UP]);

    app.apply(Action::Undo);

    assert!(*app.project() == start);
}

#[test]
fn typed_digits_on_one_cell_are_one_step() {
    let mut app = App::new();
    apply(&mut app, &[Action::HexDigit(1), Action::HexDigit(2)]);
    assert_eq!(song_cell(&app, 0, 0), Some(0x12));

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), None);
}

#[test]
fn moving_the_cursor_ends_a_merge() {
    let mut app = App::with_project(arranged());
    apply(&mut app, &[NUDGE_UP, DOWN, UP, NUDGE_UP]);

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
}

#[test]
fn another_action_ends_a_merge() {
    let mut app = App::with_project(arranged());
    apply(&mut app, &[NUDGE_UP, Action::OctaveUp, NUDGE_UP]);

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
}

#[test]
fn a_nudge_and_a_typed_digit_are_separate_steps() {
    let mut app = App::with_project(arranged());
    apply(&mut app, &[NUDGE_UP, Action::HexDigit(9)]);

    app.apply(Action::Undo);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
}

#[test]
fn nudges_that_cancel_out_leave_no_step() {
    let mut app = App::with_project(arranged());
    apply(&mut app, &[NUDGE_UP, NUDGE_DOWN]);

    app.apply(Action::Undo);

    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
}

#[test]
fn a_new_edit_after_undo_drops_what_could_be_redone() {
    let mut app = App::new();
    apply(&mut app, &[Action::HexDigit(1), Action::Undo, DOWN]);
    app.apply(Action::HexDigit(2));

    app.apply(Action::Redo);

    assert_eq!(status_line(&app), NOTHING_TO_REDO);
    assert_eq!(song_cell(&app, 0, 0), None);
}

#[test]
fn actions_that_change_no_music_are_not_steps() {
    let mut app = App::with_project(arranged());
    apply(
        &mut app,
        &[
            DOWN,
            Action::FocusNext,
            Action::OctaveUp,
            Action::Mute(1),
            Action::Solo(2),
            Action::ToggleFollow,
        ],
    );

    app.apply(Action::Undo);

    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
}

#[test]
fn undo_leaves_mute_and_solo_alone() {
    let mut app = App::with_project(arranged());
    apply(&mut app, &[Action::HexDigit(1), Action::Mute(1)]);
    let muted = app.grid().text_dump().lines().nth(1).unwrap().to_owned();

    app.apply(Action::Undo);

    let after = app.grid().text_dump().lines().nth(1).unwrap().to_owned();
    assert_eq!(after[..28], muted[..28]);
}

#[test]
fn undo_turns_follow_off_because_it_moves_the_cursor() {
    let mut app = App::with_project(arranged());
    apply(&mut app, &[Action::HexDigit(1), Action::ToggleFollow]);

    app.apply(Action::Undo);

    assert_eq!(status_line(&app), "FOLLOW OFF");
}

#[test]
fn the_history_keeps_the_last_thousand_steps() {
    let mut app = App::new();
    apply(&mut app, &[Action::SetSongLoopRow; 1001]);
    // An odd number of flips leaves the loop off, and the oldest flip can no longer be undone.
    assert_eq!(app.project().song_loop_row(), None);

    apply(&mut app, &[Action::Undo; 1000]);
    assert_eq!(app.project().song_loop_row(), None);

    app.apply(Action::Undo);
    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
    assert_eq!(app.project().song_loop_row(), None);
}

#[test]
fn the_history_survives_save_and_stays_out_of_the_file() {
    let mut app = App::new();
    app.apply(Action::HexDigit(4));

    let contents = save(&mut app);

    assert_eq!(contents, project_file::write(app.project()));
    app.apply(Action::Undo);
    assert_eq!(song_cell(&app, 0, 0), None);
}

#[test]
fn undoing_back_to_the_saved_music_clears_the_unsaved_mark() {
    let mut app = App::new();
    save(&mut app);
    app.apply(Action::HexDigit(4));
    assert!(app.has_unsaved_changes());

    app.apply(Action::Undo);

    assert!(!app.has_unsaved_changes());
}

#[test]
fn the_history_starts_empty_when_another_project_opens() {
    let mut app = App::new();
    app.apply(Action::HexDigit(4));
    save(&mut app);

    open(&mut app, &arranged());
    app.apply(Action::Undo);

    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
    assert!(*app.project() == arranged());
}

#[test]
fn the_history_starts_empty_for_a_new_project() {
    let mut app = App::new();
    app.apply(Action::HexDigit(4));
    save(&mut app);

    app.apply(Action::NewProject);
    app.apply(Action::Undo);

    assert_eq!(status_line(&app), NOTHING_TO_UNDO);
}
