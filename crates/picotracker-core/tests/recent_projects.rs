//! The recent Projects at the top of the Open file list.

use picotracker_core::app::{
    Action, App, Direction, DirectoryEntry, FileRequest, FileResponse, Launch, StateContents,
};
use picotracker_core::project::Project;
use picotracker_core::project_file;
use picotracker_core::settings::{AppState, write_state};
use std::path::PathBuf;

const DOWN: Action = Action::Move(Direction::Down);
const HOME: &str = "/home/user";

fn right_slot(app: &App, line: usize) -> String {
    let text: String = app
        .grid()
        .text_dump()
        .lines()
        .nth(line)
        .unwrap()
        .chars()
        .skip(83)
        .collect();
    text.trim_end().to_owned()
}

/// Launches with these recent Projects, the most recent first. The last one fails to open, so
/// the app starts with an untitled Project.
fn launch_remembering(paths: &[&str]) -> App {
    let mut state = AppState::default();
    for path in paths.iter().rev() {
        state.remember_project(PathBuf::from(path));
    }
    let mut app = App::new();
    app.launch(Launch {
        state_file: Some(PathBuf::from("/state/state.ron")),
        state: StateContents::Bytes(write_state(&state).into_bytes()),
        directory: PathBuf::from(HOME),
    });
    app.take_file_requests();
    if let Some(first) = paths.first() {
        app.receive_file(FileResponse::Failed {
            path: PathBuf::from(first),
            reason: "gone".to_owned(),
        });
    }
    app
}

fn open_list(app: &mut App, entries: Vec<DirectoryEntry>) {
    app.apply(Action::OpenProject);
    let requests = app.take_file_requests();
    let [FileRequest::List { directory }] = &requests[..] else {
        panic!("expected one listing, got {requests:?}");
    };
    app.receive_file(FileResponse::Listed {
        directory: directory.clone(),
        entries,
    });
}

fn folder(name: &str) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_owned(),
        is_directory: true,
    }
}

fn file(name: &str) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_owned(),
        is_directory: false,
    }
}

#[test]
fn recent_projects_are_listed_above_the_folder_with_the_cursor_on_the_newest() {
    let mut app = launch_remembering(&["/music/b.ptrk", "/old/a.ptrk"]);

    open_list(&mut app, vec![folder("songs"), file("c.ptrk")]);

    assert_eq!(right_slot(&app, 0), "OPEN PROJECT");
    assert_eq!(right_slot(&app, 1), "RECENT");
    assert_eq!(right_slot(&app, 2), ">b.ptrk");
    assert_eq!(right_slot(&app, 3), " a.ptrk");
    assert_eq!(right_slot(&app, 4), HOME);
    assert_eq!(right_slot(&app, 5), " ../");
    assert_eq!(right_slot(&app, 6), " songs/");
    assert_eq!(right_slot(&app, 7), " c.ptrk");
}

#[test]
fn enter_on_a_recent_project_opens_it() {
    let mut app = launch_remembering(&["/music/b.ptrk", "/old/a.ptrk"]);
    open_list(&mut app, vec![]);

    app.apply(DOWN);
    app.apply(Action::Confirm);

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::Read {
            path: PathBuf::from("/old/a.ptrk")
        }]
    );
}

#[test]
fn the_cursor_moves_from_the_recent_projects_into_the_folder() {
    let mut app = launch_remembering(&["/music/b.ptrk"]);
    open_list(&mut app, vec![folder("songs")]);

    app.apply(DOWN);
    app.apply(DOWN);
    app.apply(Action::Confirm);

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(HOME).join("songs")
        }]
    );
}

#[test]
fn after_moving_to_another_folder_the_cursor_is_on_its_first_row() {
    let mut app = launch_remembering(&["/music/b.ptrk"]);
    open_list(&mut app, vec![folder("songs")]);
    app.apply(DOWN);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();

    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME).join("songs"),
        entries: vec![],
    });

    assert_eq!(right_slot(&app, 2), " b.ptrk");
    assert_eq!(right_slot(&app, 4), ">../");
}

#[test]
fn the_open_project_is_not_listed_as_recent() {
    let mut state = AppState::default();
    state.remember_project(PathBuf::from("/old/a.ptrk"));
    state.remember_project(PathBuf::from("/music/b.ptrk"));
    let mut app = App::new();
    app.launch(Launch {
        state_file: Some(PathBuf::from("/state/state.ron")),
        state: StateContents::Bytes(write_state(&state).into_bytes()),
        directory: PathBuf::from(HOME),
    });
    app.take_file_requests();
    app.receive_file(FileResponse::Read {
        path: PathBuf::from("/music/b.ptrk"),
        contents: project_file::write(&Project::new()),
    });
    app.take_file_requests();

    open_list(&mut app, vec![]);

    assert_eq!(right_slot(&app, 1), "RECENT");
    assert_eq!(right_slot(&app, 2), ">a.ptrk");
    assert_eq!(right_slot(&app, 3), "/music");
}

#[test]
fn save_as_and_import_list_no_recent_projects() {
    let mut app = launch_remembering(&["/music/b.ptrk"]);

    app.apply(Action::SaveAs);
    assert_ne!(right_slot(&app, 1), "RECENT");
    app.apply(Action::Cancel);
    app.take_file_requests();

    app.apply(Action::Import);
    assert_ne!(right_slot(&app, 1), "RECENT");
}

#[test]
fn with_no_recent_projects_the_list_starts_with_the_folder() {
    let mut app = launch_remembering(&[]);

    open_list(&mut app, vec![]);

    assert_eq!(right_slot(&app, 1), HOME);
    assert_eq!(right_slot(&app, 2), ">../");
}
