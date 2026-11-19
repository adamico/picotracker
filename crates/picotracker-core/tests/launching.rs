//! Launching the headless App core with what `state.ron` held, and keeping the recent Projects in
//! it up to date. File contents go in and out through the App core's interface, so these tests
//! touch no disk.

mod support;

use picotracker_core::app::{Action, App, DirectoryEntry, FileRequest, FileResponse, Launch};
use picotracker_core::project::Project;
use picotracker_core::project_file;
use picotracker_core::settings::{AppState, StateRead, read_state, write_state};
use std::path::PathBuf;
use support::status_line;

const STATE_FILE: &str = "/state/picotracker/state.ron";
const HOME: &str = "/home/user";

fn launch(state: Option<String>) -> App {
    let mut app = App::new();
    app.launch(Launch {
        state_file: PathBuf::from(STATE_FILE),
        state,
        directory: PathBuf::from(HOME),
    });
    app
}

fn state_remembering(paths: &[&str]) -> String {
    let mut state = AppState::default();
    for path in paths.iter().rev() {
        state.remember_project(PathBuf::from(path));
    }
    write_state(&state)
}

/// The recent Projects in the `state.ron` the app has just asked to write, if it asked.
fn state_written(requests: &[FileRequest]) -> Option<Vec<PathBuf>> {
    requests.iter().find_map(|request| match request {
        FileRequest::Write { path, contents }
            if path.as_path() == std::path::Path::new(STATE_FILE) =>
        {
            match read_state(contents) {
                StateRead::Loaded(state) => Some(state.recent_projects().to_vec()),
                other => panic!("the app wrote an unreadable state: {other:?}"),
            }
        }
        _ => None,
    })
}

fn project_text() -> String {
    project_file::write(&Project::new())
}

#[test]
fn with_no_state_file_the_app_starts_empty_in_the_launch_folder() {
    let mut app = launch(None);

    assert!(app.take_file_requests().is_empty());
    app.apply(Action::OpenProject);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(HOME)
        }]
    );
}

#[test]
fn the_last_opened_project_is_opened_at_launch() {
    let mut app = launch(Some(state_remembering(&["/music/b.ptrk", "/music/a.ptrk"])));

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::Read {
            path: PathBuf::from("/music/b.ptrk")
        }]
    );
    app.receive_file(FileResponse::Read {
        path: PathBuf::from("/music/b.ptrk"),
        contents: project_text(),
    });

    assert!(status_line(&app).starts_with("OPENED B.PTRK"));
    app.take_file_requests();
    app.apply(Action::Save);
    assert_eq!(
        app.take_file_requests()[0],
        FileRequest::Write {
            path: PathBuf::from("/music/b.ptrk"),
            contents: project_text(),
        }
    );
}

#[test]
fn a_last_project_that_is_gone_leaves_an_empty_project_and_says_why() {
    let mut app = launch(Some(state_remembering(&["/music/gone.ptrk"])));
    app.take_file_requests();

    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("/music/gone.ptrk"),
        reason: "No such file or directory".to_owned(),
    });

    assert!(status_line(&app).starts_with("! CANNOT OPEN GONE.PTRK: NO SUCH FILE OR DIRECTORY"));
    assert!(!app.has_unsaved_changes());
}

#[test]
fn opening_a_project_remembers_it_first() {
    let mut app = launch(Some(state_remembering(&["/music/a.ptrk"])));
    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("/music/a.ptrk"),
        reason: "busy".to_owned(),
    });
    app.take_file_requests();

    app.apply(Action::OpenProject);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME),
        entries: vec![DirectoryEntry {
            name: "new.ptrk".to_owned(),
            is_directory: false,
        }],
    });
    app.apply(Action::Move(picotracker_core::app::Direction::Down));
    app.apply(Action::Confirm);
    let requests = app.take_file_requests();
    let opened = PathBuf::from(HOME).join("new.ptrk");
    assert_eq!(
        requests,
        [FileRequest::Read {
            path: opened.clone()
        }]
    );
    app.receive_file(FileResponse::Read {
        path: opened.clone(),
        contents: project_text(),
    });

    assert_eq!(
        state_written(&app.take_file_requests()),
        Some(vec![opened, PathBuf::from("/music/a.ptrk")])
    );
}

#[test]
fn saving_a_project_remembers_it_first() {
    let mut app = launch(None);
    app.apply(Action::HexDigit(1));
    app.apply(Action::SaveAs);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME),
        entries: vec![],
    });
    for ch in "song".chars() {
        app.apply(Action::TypeChar(ch));
    }
    app.apply(Action::Confirm);
    let saved = PathBuf::from(HOME).join("song.ptrk");
    assert!(matches!(
        &app.take_file_requests()[..],
        [FileRequest::Write { path, .. }] if *path == saved
    ));

    app.receive_file(FileResponse::Written {
        path: saved.clone(),
    });

    assert_eq!(state_written(&app.take_file_requests()), Some(vec![saved]));
}

#[test]
fn a_damaged_state_file_is_renamed_and_the_app_says_so() {
    let mut app = launch(Some("(version: 1, recent_projects: [".to_owned()));

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::Rename {
            from: PathBuf::from(STATE_FILE),
            to: PathBuf::from(format!("{STATE_FILE}.bad")),
        }]
    );
    assert!(
        status_line(&app)
            .starts_with("! STATE.RON WAS DAMAGED: RENAMED TO STATE.RON.BAD, USING DEFAULTS")
    );
}

#[test]
fn a_newer_state_file_is_left_alone_for_the_whole_session() {
    let mut app = launch(Some("(version: 9)".to_owned()));

    assert!(app.take_file_requests().is_empty());
    assert!(
        status_line(&app)
            .starts_with("! STATE.RON IS FROM A NEWER PICOTRACKER: USING DEFAULTS THIS SESSION")
    );

    app.apply(Action::HexDigit(1));
    app.apply(Action::SaveAs);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from(HOME),
        entries: vec![],
    });
    app.apply(Action::TypeChar('s'));
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Written {
        path: PathBuf::from(HOME).join("s.ptrk"),
    });

    assert_eq!(state_written(&app.take_file_requests()), None);
}

#[test]
fn an_app_that_was_not_launched_with_a_state_file_never_writes_one() {
    let mut app = App::new();
    app.apply(Action::HexDigit(1));
    app.apply(Action::SaveAs);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![],
    });
    app.apply(Action::TypeChar('s'));
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Written {
        path: PathBuf::from("./s.ptrk"),
    });

    assert!(app.take_file_requests().is_empty());
}
