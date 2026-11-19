//! Importing a Cart through the headless App core. File contents go in and out through the App
//! core's interface, so these tests touch no disk. Every Cart here is generated in test code.

mod support;

use picotracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use picotracker_core::player::AudioRequest;
use std::path::{Path, PathBuf};
use support::status_line;
use support::{TestCart, TestPattern, TestSfx};

const DOWN: Action = Action::Move(Direction::Down);
const UNSAVED_QUESTION: &str = "UNSAVED CHANGES WILL BE LOST  ENTER DISCARD  ESC CANCEL";

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

fn file(name: &str) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_owned(),
        is_directory: false,
    }
}

fn folder(name: &str) -> DirectoryEntry {
    DirectoryEntry {
        name: name.to_owned(),
        is_directory: true,
    }
}

fn answer_listing(app: &mut App, entries: Vec<DirectoryEntry>) {
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: PathBuf::from(".")
        }]
    );
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries,
    });
}

/// A one-song Cart with two SFX and a disabled channel.
fn simple_cart() -> TestCart {
    let mut last = TestPattern::playing([Some(1), Some(0), None, None]);
    last.stop = true;
    TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[12, 14]),
            TestSfx::with_pitches(8, &[19, 21]),
        ],
        patterns: vec![TestPattern::playing([Some(0), None, Some(1), None]), last],
    }
}

/// Picks the only file on the list and hands the app its contents.
fn import_file(app: &mut App, name: &str, contents: Vec<u8>) {
    app.apply(Action::Import);
    answer_listing(app, vec![file(name)]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let path = Path::new(".").join(name);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes { path: path.clone() }]
    );
    app.receive_file(FileResponse::BytesRead { path, contents });
}

/// Makes a change to the music: places Chain 01 on the first Song row.
fn edit(app: &mut App) {
    app.apply(Action::HexDigit(1));
}

#[test]
fn import_opens_a_file_list_of_folders_and_carts() {
    let mut app = App::new();

    app.apply(Action::Import);
    answer_listing(
        &mut app,
        vec![
            file("song.ptrk"),
            file("demo.p8"),
            file("notes.txt"),
            file("picture.png"),
            file("LABEL.P8.PNG"),
            folder("carts"),
        ],
    );

    assert_eq!(right_slot(&app, 0), "IMPORT FILE");
    assert_eq!(right_slot(&app, 2), ">../");
    assert_eq!(right_slot(&app, 3), " carts/");
    assert_eq!(right_slot(&app, 4), " demo.p8");
    assert_eq!(right_slot(&app, 5), " LABEL.P8.PNG");
    assert_eq!(right_slot(&app, 6), "");
}

#[test]
fn selecting_a_p8_cart_opens_it_as_a_new_project() {
    let mut app = App::new();

    import_file(&mut app, "demo.p8", simple_cart().p8());

    let project = app.project();
    assert!(project.chain_row_entry(0, 0, 0).is_some());
    assert!(project.chain_row_entry(1, 1, 0).is_some());
    assert_eq!(project.target_cart().unwrap().path, "demo.p8");
    assert_eq!(status_line(&app), "IMPORTED SONG 1 OF DEMO.P8");
    assert_eq!(right_slot(&app, 0), "OVERVIEW");
}

#[test]
fn selecting_a_png_cart_gives_the_same_project_as_its_text_twin() {
    let mut from_text = App::new();
    let mut from_pixels = App::new();

    import_file(&mut from_text, "demo.p8", simple_cart().p8());
    import_file(&mut from_pixels, "demo.p8.png", simple_cart().png());

    let mut expected = from_text.project().clone();
    let mut target = expected.target_cart().unwrap().clone();
    target.path = "demo.p8.png".to_owned();
    expected.set_target_cart(Some(target));
    assert_eq!(from_pixels.project(), &expected);
    assert_eq!(status_line(&from_pixels), "IMPORTED SONG 1 OF DEMO.P8.PNG");
}

#[test]
fn an_imported_project_starts_with_an_empty_undo_history() {
    let mut app = App::new();
    edit(&mut app);
    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let path = PathBuf::from("./demo.p8");
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path,
        contents: simple_cart().p8(),
    });
    let imported = app.project().clone();

    app.apply(Action::Undo);

    assert_eq!(status_line(&app), "NOTHING TO UNDO");
    assert_eq!(app.project(), &imported);
}

#[test]
fn import_asks_first_when_there_are_unsaved_changes() {
    let mut app = App::new();
    edit(&mut app);
    let edited = app.project().clone();

    app.apply(Action::Import);

    assert_eq!(status_line(&app), UNSAVED_QUESTION);
    assert_eq!(app.take_file_requests(), []);

    app.apply(Action::Cancel);
    assert_eq!(app.take_file_requests(), []);
    assert_eq!(app.project(), &edited);

    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    assert_eq!(right_slot(&app, 0), "IMPORT FILE");
}

#[test]
fn leaving_the_file_list_changes_nothing() {
    let mut app = App::new();
    edit(&mut app);
    let edited = app.project().clone();

    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(Action::Cancel);

    assert_eq!(right_slot(&app, 0), "OVERVIEW");
    assert_eq!(app.project(), &edited);
}

#[test]
fn import_stops_playback() {
    let mut app = App::new();
    import_file(&mut app, "first.p8", simple_cart().p8());
    app.apply(Action::PlayFocusedPane);
    app.take_audio_requests();

    import_file(&mut app, "second.p8", simple_cart().p8());

    assert!(app.take_audio_requests().contains(&AudioRequest::Stop));
}

#[test]
fn warnings_are_counted_on_the_status_line_and_listed_in_the_log() {
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(0, &[12])],
        patterns: vec![TestPattern::playing([Some(0), None, None, None])],
    };
    let mut app = App::new();

    import_file(&mut app, "demo.p8", cart.p8());

    assert_eq!(
        status_line(&app),
        "! IMPORTED SONG 1 OF DEMO.P8 WITH 1 WARNING  ^L LOG"
    );
    let dump = app.grid().text_dump();
    assert!(dump.contains("SFX 00 HAS SPD 0"), "{dump}");
}

#[test]
fn enter_on_a_timing_warning_jumps_to_the_step_it_is_about() {
    // On the second Pattern, 5 Steps at Speed 8 end inside the 14th Step at Speed 3.
    let mut setter = TestSfx::with_pitches(8, &[12]);
    setter.loop_start = 5;
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[20]),
            setter,
            TestSfx::with_pitches(3, &[24]),
        ],
        patterns: vec![
            TestPattern::playing([Some(0), None, None, None]),
            TestPattern::playing([Some(1), Some(2), None, None]),
        ],
    };
    let mut app = App::new();
    import_file(&mut app, "demo.p8", cart.p8());
    let cut_off = app.project().chain_row_entry(1, 1, 0).unwrap().phrase();

    app.apply(Action::FocusPrevious);
    app.apply(Action::Confirm);

    assert_eq!(app.phrase_under_cursor(), Some(cut_off));
    let dump = app.grid().text_dump();
    assert!(dump.contains("0C[...]"), "{dump}");
}

#[test]
fn a_damaged_cart_is_refused_and_the_project_stays() {
    let mut app = App::new();
    edit(&mut app);
    let edited = app.project().clone();

    app.apply(Action::Import);
    app.apply(Action::Confirm);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: PathBuf::from("./demo.p8"),
        contents: b"not a cart".to_vec(),
    });

    assert_eq!(
        status_line(&app),
        "! CANNOT IMPORT DEMO.P8: NOT A PICO-8 CART"
    );
    assert_eq!(app.project(), &edited);
    assert_eq!(right_slot(&app, 0), "OVERVIEW");
}

#[test]
fn a_cart_that_cannot_be_read_says_why() {
    let mut app = App::new();

    app.apply(Action::Import);
    answer_listing(&mut app, vec![file("demo.p8")]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Failed {
        path: PathBuf::from("./demo.p8"),
        reason: "permission denied".to_owned(),
    });

    assert_eq!(
        status_line(&app),
        "! CANNOT IMPORT DEMO.P8: PERMISSION DENIED"
    );
}
