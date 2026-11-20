//! Clone, Deep clone and new unused Chains and Phrases.

mod support;

use pikutracker_core::app::{Action, App, Direction};
use pikutracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use pikutracker_core::project::{ChainEntry, ChainId, PhraseId, Project, SONG_ROWS, Step, TRACKS};
use support::{apply, status_line};

const DOWN: Action = Action::Move(Direction::Down);
const RIGHT: Action = Action::Move(Direction::Right);
const TAB: Action = Action::FocusNext;
const SELECT: Action = Action::StartSelection;

fn song_cell(app: &App, row: usize, track: usize) -> Option<u8> {
    app.project().song_cell(row, track).map(ChainId::number)
}

fn entry(app: &App, chain: u8, index: usize) -> Option<ChainEntry> {
    app.project().chain(ChainId::new(chain)).entry(index)
}

fn entry_phrase(app: &App, chain: u8, index: usize) -> Option<u8> {
    entry(app, chain, index).map(|entry| entry.phrase().number())
}

fn put_entry(project: &mut Project, chain: u8, index: usize, phrase: u8) {
    *project.chain_mut(ChainId::new(chain)).entry_mut(index) =
        Some(ChainEntry::new(PhraseId::new(phrase)));
}

/// Chain 00 on the first Song row. Its entries play Phrases 00, 01 and 00 again, and Phrase 00
/// holds a note, a Speed, a LEN and a filter of its own.
fn project_with_a_song() -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    put_entry(&mut project, 0, 0, 0);
    put_entry(&mut project, 0, 1, 1);
    put_entry(&mut project, 0, 2, 0);
    let phrase = project.phrase_mut(PhraseId::new(0));
    *phrase.step_mut(0) = Step::new(30, 9, 5, 2).unwrap();
    phrase.set_speed(9);
    phrase.set_length(12);
    phrase.filters_mut().set_reverb(2);
    project.phrase_mut(PhraseId::new(1)).set_speed(20);
    project
}

#[test]
fn cloning_a_chain_points_the_song_cell_at_a_new_copy_that_shares_its_phrases() {
    let mut app = App::with_project(project_with_a_song());

    app.apply(Action::Clone);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
    assert_eq!(entry_phrase(&app, 1, 0), Some(0));
    assert_eq!(entry_phrase(&app, 1, 1), Some(1));
    assert_eq!(entry_phrase(&app, 1, 2), Some(0));
    assert_eq!(entry_phrase(&app, 0, 1), Some(1), "the original stays");
}

#[test]
fn cloning_a_phrase_copies_all_of_it_and_keeps_the_entrys_transpose_and_fill() {
    let mut project = project_with_a_song();
    let mut filled = ChainEntry::new(PhraseId::new(0));
    filled.set_transpose(5);
    filled.set_fill(true);
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(filled);
    let mut app = App::with_project(project);

    apply(&mut app, &[TAB, Action::Clone]);

    let cloned = entry(&app, 0, 0).unwrap();
    assert_eq!(cloned.phrase().number(), 2);
    assert_eq!(cloned.transpose(), 5);
    assert!(cloned.fill());
    assert!(app.project().phrase(PhraseId::new(2)) == app.project().phrase(PhraseId::new(0)));
    assert_eq!(entry_phrase(&app, 0, 2), Some(0), "other entries stay");
}

#[test]
fn a_clone_never_lands_on_a_loose_phrase_or_a_chain_nothing_places() {
    let mut project = project_with_a_song();
    // Phrase 02 is loose: no Chain plays it, but it holds a note.
    *project.phrase_mut(PhraseId::new(2)).step_mut(0) = Step::new(12, 0, 3, 0).unwrap();
    // Chain 01 is not on the Song, and it is the only thing that plays Phrase 03.
    put_entry(&mut project, 1, 0, 3);
    let mut app = App::with_project(project);

    app.apply(Action::Clone);
    assert_eq!(song_cell(&app, 0, 0), Some(2));

    apply(&mut app, &[TAB, Action::Clone]);
    assert_eq!(entry_phrase(&app, 2, 0), Some(4));
}

#[test]
fn deep_clone_gives_the_new_chain_one_copy_of_each_distinct_phrase() {
    let mut app = App::with_project(project_with_a_song());

    app.apply(Action::DeepClone);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
    let first = entry_phrase(&app, 1, 0).unwrap();
    let second = entry_phrase(&app, 1, 1).unwrap();
    assert_eq!((first, second), (2, 3));
    assert_eq!(entry_phrase(&app, 1, 2), Some(first), "still shared");
    assert!(app.project().phrase(PhraseId::new(2)) == app.project().phrase(PhraseId::new(0)));
    assert!(app.project().phrase(PhraseId::new(3)) == app.project().phrase(PhraseId::new(1)));
}

#[test]
fn deep_clone_keeps_each_entrys_transpose_and_fill() {
    let mut project = project_with_a_song();
    let mut filled = ChainEntry::new(PhraseId::new(1));
    filled.set_transpose(-3);
    filled.set_fill(true);
    *project.chain_mut(ChainId::new(0)).entry_mut(1) = Some(filled);
    let mut app = App::with_project(project);

    app.apply(Action::DeepClone);

    let copy = entry(&app, 1, 1).unwrap();
    assert_eq!((copy.transpose(), copy.fill()), (-3, true));
}

#[test]
fn deep_clone_in_the_chain_pane_copies_the_phrase_under_the_cursor() {
    let mut app = App::with_project(project_with_a_song());

    apply(&mut app, &[TAB, Action::DeepClone]);

    assert_eq!(entry_phrase(&app, 0, 0), Some(2));
    assert_eq!(entry_phrase(&app, 0, 2), Some(0));
}

#[test]
fn cloning_a_song_selection_makes_one_copy_per_distinct_chain() {
    let mut project = project_with_a_song();
    for (row, track, chain) in [(1, 0, 0), (0, 1, 5), (1, 1, 5)] {
        project.set_song_cell(row, track, Some(ChainId::new(chain)));
    }
    put_entry(&mut project, 5, 0, 1);
    let mut app = App::with_project(project);

    apply(&mut app, &[SELECT, DOWN, RIGHT, Action::Clone]);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
    assert_eq!(song_cell(&app, 1, 0), Some(1));
    assert_eq!(song_cell(&app, 0, 1), Some(2));
    assert_eq!(song_cell(&app, 1, 1), Some(2));
    assert_eq!(entry_phrase(&app, 2, 0), Some(1));
}

#[test]
fn deep_cloning_a_song_selection_keeps_phrases_shared_between_the_new_chains() {
    let mut project = project_with_a_song();
    project.set_song_cell(0, 1, Some(ChainId::new(5)));
    put_entry(&mut project, 5, 0, 1);
    let mut app = App::with_project(project);

    apply(&mut app, &[SELECT, RIGHT, Action::DeepClone]);

    let (first, second) = (
        song_cell(&app, 0, 0).unwrap(),
        song_cell(&app, 0, 1).unwrap(),
    );
    assert_ne!(first, second);
    let shared = entry_phrase(&app, first, 1).unwrap();
    assert_ne!(shared, 1, "the new Chains have their own copy");
    assert_eq!(entry_phrase(&app, second, 0), Some(shared));
}

#[test]
fn cloning_a_chain_selection_makes_one_copy_per_distinct_phrase() {
    let mut app = App::with_project(project_with_a_song());

    apply(&mut app, &[TAB, SELECT, DOWN, DOWN, Action::Clone]);

    assert_eq!(entry_phrase(&app, 0, 0), Some(2));
    assert_eq!(entry_phrase(&app, 0, 1), Some(3));
    assert_eq!(entry_phrase(&app, 0, 2), Some(2));
}

#[test]
fn a_chain_selection_only_clones_the_entries_inside_it() {
    let mut app = App::with_project(project_with_a_song());

    apply(&mut app, &[TAB, SELECT, DOWN, Action::Clone]);

    assert_eq!(entry_phrase(&app, 0, 2), Some(0));
}

#[test]
fn cloning_an_empty_cell_says_there_is_nothing_to_clone() {
    let mut app = App::with_project(project_with_a_song());

    apply(&mut app, &[DOWN, Action::Clone]);
    assert!(status_line(&app).contains("NO CHAIN HERE"));
    assert_eq!(song_cell(&app, 1, 0), None);

    apply(&mut app, &[Action::Move(Direction::Up), TAB]);
    apply(&mut app, &[DOWN, DOWN, DOWN, Action::Clone]);
    assert!(status_line(&app).contains("NO PHRASE HERE"));
    assert_eq!(entry(&app, 0, 3), None);
}

#[test]
fn the_phrase_pane_has_no_cell_to_point_at_a_clone() {
    let mut app = App::with_project(project_with_a_song());
    let before = app.project().clone();

    apply(&mut app, &[TAB, TAB, Action::Clone]);

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("SONG OR CHAIN PANE"));
}

/// A Project in which every Chain holds an entry, so none is free.
fn no_free_chains() -> Project {
    let mut project = project_with_a_song();
    for chain in 1..=255 {
        put_entry(&mut project, chain, 0, 1);
    }
    project
}

#[test]
fn a_clone_with_no_free_chain_fails_and_changes_nothing() {
    let mut app = App::with_project(no_free_chains());
    let before = app.project().clone();

    app.apply(Action::Clone);

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("NO UNUSED CHAIN LEFT"));
}

#[test]
fn a_selection_too_big_for_the_free_slots_clones_nothing() {
    let mut project = no_free_chains();
    *project.chain_mut(ChainId::new(9)) = Default::default();
    project.set_song_cell(0, 1, Some(ChainId::new(5)));
    let mut app = App::with_project(project);
    let before = app.project().clone();

    apply(&mut app, &[SELECT, RIGHT, Action::Clone]);

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("NO UNUSED CHAIN LEFT"));
}

#[test]
fn a_deep_clone_with_no_free_phrase_fails_and_changes_nothing() {
    let mut project = project_with_a_song();
    for phrase in 2..=255 {
        project.phrase_mut(PhraseId::new(phrase)).set_speed(1);
    }
    let mut app = App::with_project(project);
    let before = app.project().clone();

    app.apply(Action::DeepClone);

    assert!(app.project() == &before);
    assert!(status_line(&app).contains("NO UNUSED PHRASE LEFT"));
}

#[test]
fn new_unused_puts_an_empty_chain_on_the_song_cell() {
    let mut app = App::with_project(project_with_a_song());

    app.apply(Action::NewUnused);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
    assert!(*app.project().chain(ChainId::new(1)) == Default::default());
}

#[test]
fn new_unused_puts_a_new_phrase_at_the_projects_default_speed_on_the_chain_entry() {
    let mut project = project_with_a_song();
    project.set_default_speed(7);
    let mut app = App::with_project(project);

    apply(&mut app, &[TAB, DOWN, DOWN, DOWN, Action::NewUnused]);

    assert_eq!(entry_phrase(&app, 0, 3), Some(2));
    assert_eq!(app.project().phrase(PhraseId::new(2)).speed(), 7);
}

#[test]
fn new_unused_over_a_filled_entry_keeps_its_transpose_and_fill() {
    let mut project = project_with_a_song();
    let mut filled = ChainEntry::new(PhraseId::new(1));
    filled.set_transpose(2);
    filled.set_fill(true);
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(filled);
    let mut app = App::with_project(project);

    apply(&mut app, &[TAB, Action::NewUnused]);

    let entry = entry(&app, 0, 0).unwrap();
    assert_eq!(entry.phrase().number(), 2);
    assert_eq!((entry.transpose(), entry.fill()), (2, true));
}

#[test]
fn new_unused_works_from_its_key() {
    let keymap = Keymap::pc_tracker();
    let mut app = App::with_project(project_with_a_song());
    let press = KeyPress::new(Key::Char('n'), Modifiers::CTRL_SHIFT);

    let action = keymap.action_in(press, app.key_context()).unwrap();
    app.apply(action);

    assert_eq!(song_cell(&app, 0, 0), Some(1));
}

#[test]
fn each_clone_or_new_item_is_one_undo_step() {
    for action in [Action::Clone, Action::DeepClone, Action::NewUnused] {
        let mut app = App::with_project(project_with_a_song());
        let before = app.project().clone();

        app.apply(action);
        let after = app.project().clone();
        assert!(after != before);

        app.apply(Action::Undo);
        assert!(app.project() == &before, "{action:?} undoes in one step");
        app.apply(Action::Redo);
        assert!(app.project() == &after, "{action:?} redoes in one step");
    }
}

#[test]
fn a_clone_of_a_whole_selection_is_one_undo_step() {
    let mut project = project_with_a_song();
    project.set_song_cell(0, 1, Some(ChainId::new(5)));
    let mut app = App::with_project(project);
    let before = app.project().clone();

    apply(&mut app, &[SELECT, RIGHT, Action::DeepClone, Action::Undo]);

    assert!(app.project() == &before);
}

#[test]
fn the_song_is_as_wide_and_as_long_as_the_cursor_can_reach() {
    let mut app = App::with_project(project_with_a_song());
    apply(&mut app, &[RIGHT; TRACKS - 1]);
    apply(&mut app, &[DOWN; SONG_ROWS - 1]);
    app.apply(Action::NewUnused);

    assert_eq!(song_cell(&app, SONG_ROWS - 1, TRACKS - 1), Some(1));
}
