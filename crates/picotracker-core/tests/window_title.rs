//! The window title names the Project and marks unsaved changes, as the status line does.

use picotracker_core::app::{Action, App, FileResponse, Launch};
use picotracker_core::project::Project;
use picotracker_core::project_file;
use picotracker_core::settings::{AppState, write_state};
use std::path::PathBuf;

/// Opens a Project the way launch reopens the last one.
fn open(app: &mut App, path: &str) {
    let mut state = AppState::default();
    state.remember_project(PathBuf::from(path));
    app.launch(Launch {
        state_file: PathBuf::from("/state/state.ron"),
        state: Some(write_state(&state)),
        directory: PathBuf::from("/"),
    });
    app.take_file_requests();
    app.receive_file(FileResponse::Read {
        path: PathBuf::from(path),
        contents: project_file::write(&Project::new()),
    });
}

#[test]
fn a_new_project_is_untitled() {
    assert_eq!(App::new().window_title(), "Untitled - picotracker");
}

#[test]
fn an_opened_project_shows_its_file_name_as_spelt() {
    let mut app = App::new();
    open(&mut app, "/music/Demo Song.ptrk");

    assert_eq!(app.window_title(), "Demo Song.ptrk - picotracker");
}

#[test]
fn unsaved_changes_are_marked_with_a_star() {
    let mut app = App::new();
    open(&mut app, "/music/demo.ptrk");

    app.apply(Action::HexDigit(1));
    assert_eq!(app.window_title(), "demo.ptrk* - picotracker");

    app.apply(Action::Undo);
    assert_eq!(app.window_title(), "demo.ptrk - picotracker");
}
