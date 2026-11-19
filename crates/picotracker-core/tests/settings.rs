//! Reading and writing `state.ron`, the file of everything the app remembers between sessions.

use picotracker_core::settings::{
    AppState, MAX_RECENT_PROJECTS, StateRead, read_state, write_state,
};
use std::path::PathBuf;

fn path(name: &str) -> PathBuf {
    PathBuf::from(format!("/music/{name}.ptrk"))
}

#[test]
fn a_written_state_reads_back_the_same() {
    let mut state = AppState::default();
    state.remember_project(path("a"));
    state.remember_project(path("b"));

    assert_eq!(read_state(&write_state(&state)), StateRead::Loaded(state));
}

#[test]
fn the_file_starts_with_its_format_version() {
    let text = write_state(&AppState::default());

    assert!(text.contains("version: 1"), "{text}");
}

#[test]
fn the_most_recent_project_comes_first() {
    let mut state = AppState::default();
    state.remember_project(path("a"));
    state.remember_project(path("b"));

    assert_eq!(state.recent_projects(), [path("b"), path("a")]);
    assert_eq!(state.last_project(), Some(path("b").as_path()));
}

#[test]
fn reopening_a_project_moves_it_to_the_front_without_a_duplicate() {
    let mut state = AppState::default();
    state.remember_project(path("a"));
    state.remember_project(path("b"));
    state.remember_project(path("a"));

    assert_eq!(state.recent_projects(), [path("a"), path("b")]);
}

#[test]
fn only_the_ten_most_recent_projects_are_kept() {
    let mut state = AppState::default();
    for n in 0..12 {
        state.remember_project(path(&n.to_string()));
    }

    assert_eq!(MAX_RECENT_PROJECTS, 10);
    assert_eq!(state.recent_projects().len(), 10);
    assert_eq!(state.recent_projects()[0], path("11"));
    assert_eq!(state.recent_projects()[9], path("2"));
}

#[test]
fn a_file_with_missing_fields_reads_them_as_defaults() {
    assert_eq!(
        read_state("(version: 1)"),
        StateRead::Loaded(AppState::default())
    );
}

#[test]
fn a_damaged_file_is_reported_as_damaged() {
    assert!(matches!(
        read_state("(version: 1, recent_projects: ["),
        StateRead::Damaged
    ));
    assert!(matches!(read_state("not ron at all"), StateRead::Damaged));
    assert!(matches!(
        read_state("(recent_projects: [])"),
        StateRead::Damaged
    ));
}

#[test]
fn a_file_from_a_newer_version_is_reported_as_newer() {
    assert_eq!(
        read_state("(version: 2, something_new: true)"),
        StateRead::Newer { found: 2 }
    );
}
