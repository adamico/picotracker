//! One linear undo history for the whole Project, driven through named actions.

use picotracker_core::app::{Action, App, Direction};
use picotracker_core::project::{ChainId, Project};

const DOWN: Action = Action::Move(Direction::Down);

fn apply(app: &mut App, actions: &[Action]) {
    for action in actions {
        app.apply(*action);
    }
}

fn song_cell(app: &App, row: usize, track: usize) -> Option<u8> {
    app.project().song_cell(row, track).map(ChainId::number)
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
