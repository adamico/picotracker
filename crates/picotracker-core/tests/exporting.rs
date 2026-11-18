//! Choosing Export Tracks and Exporting through the headless App core. File contents go in and
//! out through the App core's interface, so these tests touch no disk. Every Cart here is
//! generated in test code.

mod support;

use picotracker_core::app::{Action, App, Direction, DirectoryEntry, FileRequest, FileResponse};
use picotracker_core::cart::Cart;
use picotracker_core::keymap::{Key, KeyPress, Keymap, Modifiers};
use picotracker_core::player::AudioRequest;
use picotracker_core::project::{Claim, Project, TargetCart};
use picotracker_core::project_file;
use std::path::{Path, PathBuf};
use support::status_line;
use support::{HEADER, TestCart, TestPattern, TestSfx};

const DOWN: Action = Action::Move(Direction::Down);
const CART_NAME: &str = "game.p8";

fn line(app: &App, index: usize) -> String {
    app.grid()
        .text_dump()
        .lines()
        .nth(index)
        .unwrap()
        .to_owned()
}

fn right_slot(app: &App, index: usize) -> String {
    let text: String = line(app, index).chars().skip(83).collect();
    text.trim_end().to_owned()
}

fn cart_path() -> PathBuf {
    Path::new(".").join(CART_NAME)
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

/// An app holding the Project Imported from a Cart with these bytes.
fn imported(contents: Vec<u8>) -> App {
    let mut app = App::new();
    app.apply(Action::Import);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: CART_NAME.to_owned(),
            is_directory: false,
        }],
    });
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: cart_path(),
        contents,
    });
    app
}

/// Presses Ctrl+E and hands the app the Target Cart, returning what the app asks for next.
fn export_with(app: &mut App, contents: Vec<u8>) -> Vec<FileRequest> {
    app.apply(Action::Export);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes { path: cart_path() }]
    );
    app.receive_file(FileResponse::BytesRead {
        path: cart_path(),
        contents,
    });
    app.take_file_requests()
}

/// Runs a whole Export, returning the bytes written to the Target Cart.
fn export_ok(app: &mut App, contents: Vec<u8>) -> Vec<u8> {
    let requests = export_with(app, contents);
    let [FileRequest::WriteBytes { path, contents }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    assert_eq!(*path, cart_path());
    app.receive_file(FileResponse::Written { path: cart_path() });
    contents.clone()
}

fn stored_hash(app: &App) -> Option<String> {
    app.project()
        .target_cart()
        .unwrap()
        .claimed_bytes_hash
        .clone()
}

fn focus_overview(app: &mut App) {
    app.apply(Action::FocusPrevious);
}

#[test]
fn overview_shows_each_channels_track() {
    let mut project = Project::new();
    project.set_export_track(0, Some(3));
    project.set_export_track(2, Some(8));

    let app = App::with_project(project);

    assert_eq!(right_slot(&app, 2), "CH0  T3");
    assert_eq!(right_slot(&app, 3), "CH1  --");
    assert_eq!(right_slot(&app, 4), "CH2  T8");
    assert_eq!(right_slot(&app, 5), "CH3  --");
}

#[test]
fn typing_a_digit_in_overview_sets_the_channels_track() {
    let mut app = App::new();
    focus_overview(&mut app);

    app.apply(DOWN);
    app.apply(Action::HexDigit(6));

    assert_eq!(app.project().export_tracks(), [None, Some(6), None, None]);
}

#[test]
fn the_keys_of_the_overview_reach_its_actions() {
    let keymap = Keymap::pc_tracker();
    let mut app = App::new();
    focus_overview(&mut app);
    let context = app.key_context();
    let action = |key, modifiers| keymap.action_in(KeyPress::new(key, modifiers), context);

    assert_eq!(
        action(Key::Char('3'), Modifiers::NONE),
        Some(Action::HexDigit(3))
    );
    assert_eq!(action(Key::Delete, Modifiers::NONE), Some(Action::Clear));
    assert_eq!(
        action(Key::Char('e'), Modifiers::CTRL),
        Some(Action::Export)
    );
}

#[test]
fn digits_that_name_no_track_are_ignored() {
    let mut app = App::new();
    focus_overview(&mut app);

    for digit in [0, 9, 10, 15] {
        app.apply(Action::HexDigit(digit));
    }

    assert_eq!(app.project().export_tracks(), [None; 4]);
}

#[test]
fn clear_takes_the_track_off_a_channel() {
    let mut project = Project::new();
    project.set_export_track(0, Some(2));
    let mut app = App::with_project(project);
    focus_overview(&mut app);

    app.apply(Action::Clear);

    assert_eq!(app.project().export_tracks(), [None; 4]);
}

#[test]
fn choosing_a_track_used_elsewhere_swaps_the_two() {
    let mut project = Project::new();
    project.set_export_track(0, Some(1));
    project.set_export_track(1, Some(2));
    let mut app = App::with_project(project);
    focus_overview(&mut app);

    app.apply(Action::HexDigit(2));

    assert_eq!(
        app.project().export_tracks(),
        [Some(2), Some(1), None, None]
    );
}

#[test]
fn choosing_a_track_from_an_empty_channel_moves_it() {
    let mut project = Project::new();
    project.set_export_track(1, Some(2));
    let mut app = App::with_project(project);
    focus_overview(&mut app);

    app.apply(Action::HexDigit(2));

    assert_eq!(app.project().export_tracks(), [Some(2), None, None, None]);
}

#[test]
fn each_channel_edit_is_its_own_undo_step() {
    let mut app = App::new();
    focus_overview(&mut app);
    app.apply(Action::HexDigit(1));
    app.apply(DOWN);
    app.apply(Action::HexDigit(2));

    app.apply(Action::Undo);

    assert_eq!(app.project().export_tracks(), [Some(1), None, None, None]);
}

#[test]
fn escape_in_overview_moves_focus_to_the_phrase_pane() {
    let mut app = App::new();
    focus_overview(&mut app);

    app.apply(Action::Cancel);

    assert_eq!(
        app.key_context(),
        picotracker_core::app::KeyContext::PhrasePitch
    );
}

#[test]
fn the_song_headers_show_each_exported_tracks_channel() {
    let mut project = Project::new();
    project.set_export_track(0, Some(3));
    project.set_export_track(3, Some(1));

    let app = App::with_project(project);

    assert_eq!(&line(&app, 1)[..28], "    T13T2 T30T4 T5 T6 T7 T8 ");
}

#[test]
fn export_writes_the_cart_and_says_what_it_wrote() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());

    let written = export_ok(&mut app, original);

    let cart = Cart::read(&written).unwrap();
    assert_eq!(cart.pattern(0).channel(0), Some(0));
    assert_eq!(cart.pattern(0).channel(2), Some(1));
    assert!(cart.pattern(1).stop());
    assert!(status_line(&app).starts_with("EXPORTED TO GAME.P8: 2 SFX, 2 PATTERNS. T1-T4"));
}

#[test]
fn export_stores_the_new_claimed_bytes_hash() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let claim = app.project().target_cart().unwrap().claim.clone();

    let written = export_ok(&mut app, original);

    let expected = Cart::read(&written).unwrap().claimed_bytes_hash(&claim);
    assert_eq!(stored_hash(&app), Some(expected));
}

#[test]
fn export_does_not_touch_the_undo_history() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    app.apply(Action::HexDigit(7));
    let edited = app.project().song_cell(0, 0);

    export_ok(&mut app, original);
    let hash = stored_hash(&app);
    app.apply(Action::Undo);

    assert_ne!(app.project().song_cell(0, 0), edited);
    assert_eq!(stored_hash(&app), hash);
    app.apply(Action::Undo);
    assert!(status_line(&app).contains("NOTHING TO UNDO"));
}

#[test]
fn export_does_not_stop_playback() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    app.apply(Action::PlaySongFromCursor);
    app.take_audio_requests();

    export_ok(&mut app, original);

    assert!(app.take_audio_requests().is_empty());
    app.apply(Action::PlayFocusedPane);
    assert!(matches!(
        app.take_audio_requests()[..],
        [AudioRequest::Stop]
    ));
}

#[test]
fn an_export_leaves_the_project_to_be_saved_so_the_hash_is_kept() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());

    export_ok(&mut app, original);

    assert!(app.has_unsaved_changes());
}

#[test]
fn a_failed_export_writes_nothing_and_lists_every_problem() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let hash = stored_hash(&app);
    focus_overview(&mut app);
    for _ in 0..4 {
        app.apply(Action::Clear);
        app.apply(DOWN);
    }

    let requests = export_with(&mut app, original);

    assert!(requests.is_empty());
    assert_eq!(stored_hash(&app), hash);
    assert!(status_line(&app).contains("EXPORT FAILED: NO EXPORT TRACKS"));
    assert_eq!(right_slot(&app, 0), "LOG");
}

#[test]
fn an_empty_claim_fails_with_the_no_claim_message() {
    let mut project = Project::new();
    project.set_target_cart(Some(TargetCart {
        path: CART_NAME.to_owned(),
        claim: Claim::default(),
        claimed_bytes_hash: None,
    }));
    project.set_export_track(0, Some(1));
    let mut app = App::with_project(project);

    let requests = export_with(&mut app, HEADER.as_bytes().to_vec());

    assert!(requests.is_empty());
    assert!(status_line(&app).contains("EXPORT FAILED: NO CLAIM IN GAME.P8"));
}

#[test]
fn a_project_file_claiming_a_slot_twice_exports_without_crashing() {
    let original = simple_cart().p8();
    let app = imported(original.clone());
    let mut project = app.project().clone();
    let mut target = project.target_cart().unwrap().clone();
    target.claim.sfx = vec![0, 0, 1];
    project.set_target_cart(Some(target));
    let reopened = project_file::parse(&project_file::write(&project)).unwrap();
    let mut app = App::with_project(reopened);

    let written = export_ok(&mut app, original);

    assert_eq!(Cart::read(&written).unwrap().pattern(0).channel(0), Some(0));
}

#[test]
fn export_without_a_target_cart_asks_for_nothing() {
    let mut app = App::new();

    app.apply(Action::Export);

    assert!(app.take_file_requests().is_empty());
    assert!(status_line(&app).contains("NO TARGET CART"));
}

#[test]
fn a_target_cart_that_cannot_be_read_is_reported() {
    let mut app = imported(simple_cart().p8());
    let hash = stored_hash(&app);

    app.apply(Action::Export);
    app.take_file_requests();
    app.receive_file(FileResponse::Failed {
        path: cart_path(),
        reason: "gone".to_owned(),
    });

    assert!(status_line(&app).contains("CANNOT EXPORT TO GAME.P8: GONE"));
    assert_eq!(stored_hash(&app), hash);
}

#[test]
fn a_failed_write_keeps_the_old_hash() {
    let original = simple_cart().p8();
    let mut app = imported(original.clone());
    let hash = stored_hash(&app);

    export_with(&mut app, original);
    app.receive_file(FileResponse::Failed {
        path: cart_path(),
        reason: "disk full".to_owned(),
    });

    assert!(status_line(&app).contains("CANNOT EXPORT TO GAME.P8: DISK FULL"));
    assert_eq!(stored_hash(&app), hash);
}

/// Answers the folder listing the app has just asked for.
fn answer_listing(app: &mut App, directory: &Path, entries: Vec<DirectoryEntry>) {
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::List {
            directory: directory.to_path_buf()
        }]
    );
    app.receive_file(FileResponse::Listed {
        directory: directory.to_path_buf(),
        entries,
    });
}

/// An app holding the Project Imported from `./carts/game.p8`.
fn imported_from_a_subfolder(contents: Vec<u8>) -> App {
    let mut app = App::new();
    app.apply(Action::Import);
    let carts = DirectoryEntry {
        name: "carts".to_owned(),
        is_directory: true,
    };
    answer_listing(&mut app, Path::new("."), vec![carts]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    let cart = DirectoryEntry {
        name: CART_NAME.to_owned(),
        is_directory: false,
    };
    answer_listing(&mut app, &Path::new(".").join("carts"), vec![cart]);
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::BytesRead {
        path: Path::new(".").join("carts").join(CART_NAME),
        contents,
    });
    app
}

/// Saves the Project as `s.ptrk` one folder up from `./carts`, returning the text written.
fn save_as_one_folder_up(app: &mut App) -> String {
    app.apply(Action::SaveAs);
    answer_listing(app, &Path::new(".").join("carts"), Vec::new());
    app.apply(DOWN);
    app.apply(Action::Confirm);
    answer_listing(app, Path::new("."), Vec::new());
    app.apply(Action::TypeChar('s'));
    app.apply(Action::Confirm);
    let requests = app.take_file_requests();
    let [FileRequest::Write { path, contents }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    let (path, contents) = (path.clone(), contents.clone());
    app.receive_file(FileResponse::Written { path });
    contents
}

#[test]
fn saving_into_another_folder_keeps_the_target_cart_pointing_at_the_same_cart() {
    let mut app = imported_from_a_subfolder(simple_cart().p8());

    let saved = save_as_one_folder_up(&mut app);

    let target = project_file::parse(&saved).unwrap();
    assert_eq!(target.target_cart().unwrap().path, "carts/game.p8");
    app.apply(Action::Export);
    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes {
            path: Path::new(".").join("carts").join(CART_NAME)
        }]
    );
}

#[test]
fn saving_into_another_folder_leaves_nothing_unsaved() {
    let mut app = imported_from_a_subfolder(simple_cart().p8());

    save_as_one_folder_up(&mut app);

    assert!(!app.has_unsaved_changes());
}

#[test]
fn saving_again_in_the_same_folder_keeps_the_path_as_it_was() {
    let mut app = imported_from_a_subfolder(simple_cart().p8());
    save_as_one_folder_up(&mut app);

    app.apply(Action::Save);

    let requests = app.take_file_requests();
    let [FileRequest::Write { contents, .. }] = &requests[..] else {
        panic!("expected one write, got {requests:?}");
    };
    let project = project_file::parse(contents).unwrap();
    assert_eq!(project.target_cart().unwrap().path, "carts/game.p8");
}

#[test]
fn the_target_cart_path_is_relative_to_the_project_file() {
    let mut project = Project::new();
    project.set_target_cart(Some(TargetCart {
        path: "carts/game.p8".to_owned(),
        claim: Claim::default(),
        claimed_bytes_hash: None,
    }));
    let mut app = App::new();
    app.apply(Action::OpenProject);
    app.take_file_requests();
    app.receive_file(FileResponse::Listed {
        directory: PathBuf::from("."),
        entries: vec![DirectoryEntry {
            name: "song.ptrk".to_owned(),
            is_directory: false,
        }],
    });
    app.apply(DOWN);
    app.apply(Action::Confirm);
    app.take_file_requests();
    app.receive_file(FileResponse::Read {
        path: Path::new(".").join("song.ptrk"),
        contents: project_file::write(&project),
    });

    app.apply(Action::Export);

    assert_eq!(
        app.take_file_requests(),
        [FileRequest::ReadBytes {
            path: Path::new(".").join("carts/game.p8")
        }]
    );
}
