//! The Export compiler seam: a Project, a Claim and a Cart's bytes in, new Cart bytes or the whole
//! list of failures out. Every Cart here is generated in test code, and the expected bytes are
//! worked out by hand from the spec.

mod support;

use picotracker_core::cart::Cart;
use picotracker_core::export::{ExportFailure, export};
use picotracker_core::project::{Chain, ChainEntry, ChainId, Claim, PhraseId, Project, Step};
use support::{HEADER, assert_same_samples, render_notes, render_song, sound_of};

const EMPTY_PATTERN: [u8; 4] = [0x41, 0x42, 0x43, 0x44];

fn blank_cart() -> Vec<u8> {
    format!("{HEADER}__lua__\nprint(1)\n").into_bytes()
}

fn claim(sfx: &[u8], patterns: (u8, u8)) -> Claim {
    Claim {
        sfx: sfx.to_vec(),
        patterns: Some(patterns),
    }
}

/// Writes a Phrase of `length` Steps whose first Step is pitch 12, volume 5.
fn write_phrase(project: &mut Project, id: u8, length: i32, speed: i32) {
    let phrase = project.phrase_mut(PhraseId::new(id));
    phrase.set_length(length);
    phrase.set_speed(speed);
    *phrase.step_mut(0) = Step::new(12, 3, 5, 1).unwrap();
}

/// Puts a one-entry Chain playing a Phrase on a Song row of a Track.
fn place(project: &mut Project, row: usize, track: usize, chain: u8, phrase: u8) {
    let mut entries = Chain::default();
    *entries.entry_mut(0) = Some(ChainEntry::new(PhraseId::new(phrase)));
    *project.chain_mut(ChainId::new(chain)) = entries;
    project.set_song_cell(row, track, Some(ChainId::new(chain)));
}

/// One Song row: Track 1 plays a 16-Step Phrase, and it is the only Export Track.
fn one_row_project() -> Project {
    let mut project = Project::new();
    write_phrase(&mut project, 0, 16, 20);
    place(&mut project, 0, 0, 0, 0);
    project.set_export_track(0, Some(1));
    project.set_song_loop_row(None);
    project
}

fn exported_cart(project: &Project, claim: &Claim) -> Cart {
    let exported = export(project, claim, &blank_cart()).unwrap();
    Cart::read(&exported.cart_bytes).unwrap()
}

#[test]
fn a_phrase_becomes_an_sfx_with_its_speed_len_and_notes() {
    let cart = exported_cart(&one_row_project(), &claim(&[8], (0, 0)));

    let sfx = cart.sfx_bytes(8);
    // Pitch 12, instrument 3, volume 5, effect 1: 12 | 3 << 6 | 5 << 9 | 1 << 12 = 0x1ACC.
    assert_eq!(sfx[..2], [0xCC, 0x1A]);
    // The editor-mode bit, Speed 20, and LEN 16 held as loop start 16 with loop end 0.
    assert_eq!(sfx[64..], [0x01, 20, 16, 0]);
}

#[test]
fn a_32_step_phrase_needs_no_len() {
    let mut project = one_row_project();
    project.phrase_mut(PhraseId::new(0)).set_length(32);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.sfx_bytes(8)[66..], [0, 0]);
}

#[test]
fn filters_come_from_the_phrase() {
    let mut project = one_row_project();
    let filters = project.phrase_mut(PhraseId::new(0)).filters_mut();
    filters.set_noiz(true);
    filters.set_buzz(true);
    filters.set_detune(1);
    filters.set_reverb(2);
    filters.set_dampen(1);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    // 1 editor + 2 NOIZ + 4 BUZZ + 1 x 8 DETUNE + 2 x 24 REVERB + 1 x 72 DAMPEN.
    assert_eq!(cart.sfx_bytes(8)[64], 135);
}

#[test]
fn a_transpose_is_baked_into_the_notes() {
    let mut project = one_row_project();
    let chain = project.chain_mut(ChainId::new(0));
    chain.entry_mut(0).as_mut().unwrap().set_transpose(7);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.sfx(8).note(0).pitch, 19);
}

#[test]
fn each_chain_row_becomes_one_pattern_in_song_order() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 16, 10);
    let mut chain = *project.chain(ChainId::new(0));
    *chain.entry_mut(1) = Some(ChainEntry::new(PhraseId::new(1)));
    *project.chain_mut(ChainId::new(0)) = chain;
    place(&mut project, 1, 0, 1, 1);

    let cart = exported_cart(&project, &claim(&[8, 9], (4, 7)));

    // Song row 0 holds two Chain rows and Song row 1 holds one. The last one stops the Song.
    assert_eq!(cart.pattern_bytes(4), [8, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(5), [9, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(6), [9, 0x42, 0x43 | 0x80, 0x44]);
}

#[test]
fn export_tracks_choose_the_channels() {
    let mut project = one_row_project();
    place(&mut project, 0, 4, 1, 0);
    project.set_export_track(0, None);
    project.set_export_track(2, Some(5));

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.pattern_bytes(0), [0x41, 0x42, 8 | 0x80, 0x44]);
}

#[test]
fn a_looping_song_marks_its_loop_target_and_its_last_pattern() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    project.set_song_loop_row(Some(1));

    let cart = exported_cart(&project, &claim(&[8], (2, 3)));

    assert_eq!(cart.pattern_bytes(2), [8, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(3), [8 | 0x80, 0x42 | 0x80, 0x43, 0x44]);
}

#[test]
fn identical_sfx_share_a_slot_and_slots_fill_in_ascending_order() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    write_phrase(&mut project, 2, 8, 9);
    place(&mut project, 2, 0, 2, 2);

    let exported = export(&project, &claim(&[30, 10], (0, 2)), &blank_cart()).unwrap();

    let cart = Cart::read(&exported.cart_bytes).unwrap();
    assert_eq!(exported.sfx_count, 2);
    assert_eq!(exported.pattern_count, 3);
    assert_eq!(cart.pattern(0).channel(0), Some(10));
    assert_eq!(cart.pattern(1).channel(0), Some(10));
    assert_eq!(cart.pattern(2).channel(0), Some(30));
}

#[test]
fn unused_claimed_slots_are_reset_to_empty() {
    let sfx = format!("01100000{}", "18350".repeat(32));
    let original = format!("{HEADER}__sfx__\n{sfx}\n{sfx}\n__music__\n00 00014344\n00 00014344\n");

    let exported = export(
        &one_row_project(),
        &claim(&[0, 1], (0, 1)),
        original.as_bytes(),
    );

    let cart = Cart::read(&exported.unwrap().cart_bytes).unwrap();
    let mut blank = [0; 68];
    blank[65] = 16;
    assert_eq!(cart.sfx_bytes(1), &blank);
    assert_eq!(cart.pattern_bytes(1), EMPTY_PATTERN);
}

#[test]
fn the_new_hash_describes_the_exported_claimed_bytes() {
    let claim = claim(&[8], (0, 1));

    let exported = export(&one_row_project(), &claim, &blank_cart()).unwrap();

    let cart = Cart::read(&exported.cart_bytes).unwrap();
    assert_eq!(exported.claimed_bytes_hash, cart.claimed_bytes_hash(&claim));
}

#[test]
fn every_failure_is_reported_at_once() {
    let mut project = one_row_project();
    project.set_export_track(0, None);

    let failures = export(&project, &Claim::default(), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NoExportTracks, ExportFailure::EmptyClaim]
    );
}

#[test]
fn missing_export_tracks_do_not_hide_a_claim_that_is_too_small() {
    let mut project = one_row_project();
    place(&mut project, 1, 0, 0, 0);
    project.set_export_track(0, None);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [
            ExportFailure::NoExportTracks,
            ExportFailure::TooFewPatterns {
                needed: 2,
                claimed: 1
            },
        ]
    );
}

#[test]
fn a_song_loop_row_past_a_gap_is_compiled_as_the_player_plays_it() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 8, 9);
    place(&mut project, 2, 0, 1, 1);
    project.set_song_loop_row(Some(2));

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 1)));

    // Row 0 plays, the gap at row 1 jumps to row 2, and row 2 then loops for ever.
    assert_eq!(cart.pattern_bytes(0), [8, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(1), [9 | 0x80, 0x42 | 0x80, 0x43, 0x44]);
}

#[test]
fn a_song_loop_row_on_an_empty_row_stops_the_song() {
    let mut project = one_row_project();
    project.set_song_loop_row(Some(5));

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.pattern_bytes(0), [8, 0x42, 0x43 | 0x80, 0x44]);
}

#[test]
fn a_claim_naming_a_slot_twice_exports_into_that_slot_once() {
    let cart = exported_cart(&one_row_project(), &claim(&[8, 8], (0, 0)));

    assert_eq!(cart.pattern(0).channel(0), Some(8));
}

#[test]
fn a_claim_naming_slots_a_cart_does_not_have_fails_the_export() {
    for bad in [
        claim(&[64], (0, 0)),
        claim(&[8], (0, 64)),
        claim(&[8], (5, 2)),
    ] {
        let failures = export(&one_row_project(), &bad, &blank_cart()).unwrap_err();

        assert!(failures.contains(&ExportFailure::BadClaim), "{failures:?}");
    }
}

#[test]
fn too_few_claimed_slots_say_how_many_are_needed() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 8, 9);
    place(&mut project, 1, 0, 1, 1);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [
            ExportFailure::TooFewSfx {
                needed: 2,
                claimed: 1
            },
            ExportFailure::TooFewPatterns {
                needed: 2,
                claimed: 1
            },
        ]
    );
}

#[test]
fn an_audible_note_pushed_out_of_range_is_named() {
    let mut project = one_row_project();
    let chain = project.chain_mut(ChainId::new(0));
    chain.entry_mut(0).as_mut().unwrap().set_transpose(60);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NoteOutOfRange {
            phrase: 0,
            step: 0,
            track: 0,
            song_row: 0,
            chain_row: 0,
        }]
    );
}

#[test]
fn a_silent_step_pushed_out_of_range_is_clamped() {
    let mut project = one_row_project();
    *project.phrase_mut(PhraseId::new(0)).step_mut(1) = Step::new(60, 0, 0, 0).unwrap();
    project
        .chain_mut(ChainId::new(0))
        .entry_mut(0)
        .as_mut()
        .unwrap()
        .set_transpose(10);

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    assert_eq!(cart.sfx(8).note(1).pitch, 63);
}

#[test]
fn a_silent_step_an_arpeggio_reads_pushed_out_of_range_is_named() {
    let mut project = one_row_project();
    let phrase = project.phrase_mut(PhraseId::new(0));
    *phrase.step_mut(0) = Step::new(24, 0, 5, 6).unwrap();
    *phrase.step_mut(1) = Step::new(63, 0, 0, 0).unwrap();
    project
        .chain_mut(ChainId::new(0))
        .entry_mut(0)
        .as_mut()
        .unwrap()
        .set_transpose(1);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NoteOutOfRange {
            phrase: 0,
            step: 1,
            track: 0,
            song_row: 0,
            chain_row: 0,
        }]
    );
}

#[test]
fn an_empty_song_has_nothing_to_export() {
    let mut project = Project::new();
    project.set_export_track(0, Some(1));

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(failures, [ExportFailure::EmptySong]);
}

#[test]
fn a_cart_that_cannot_be_read_fails_the_export() {
    let failures = export(&one_row_project(), &claim(&[8], (0, 0)), b"not a cart").unwrap_err();

    assert!(matches!(failures[..], [ExportFailure::UnreadableCart(_)]));
}

#[test]
fn a_fill_entry_waits_for_the_full_compile_rules() {
    let mut project = one_row_project();
    let chain = project.chain_mut(ChainId::new(0));
    chain.entry_mut(0).as_mut().unwrap().set_fill(true);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NotSimple {
            song_row: 0,
            chain_row: 0,
        }]
    );
}

#[test]
fn a_shorter_phrase_on_the_left_waits_for_the_full_compile_rules() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 32, 20);
    place(&mut project, 0, 1, 1, 1);
    project.set_export_track(1, Some(2));

    let failures = export(&project, &claim(&[8, 9], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NotSimple {
            song_row: 0,
            chain_row: 0,
        }]
    );
}

#[test]
fn a_shorter_phrase_on_the_right_is_simple() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 8, 20);
    place(&mut project, 0, 1, 1, 1);
    project.set_export_track(1, Some(2));

    let cart = exported_cart(&project, &claim(&[8, 9], (0, 0)));

    assert_eq!(cart.pattern_bytes(0), [8, 9, 0x43 | 0x80, 0x44]);
}

#[test]
fn a_chain_row_timed_by_an_unexported_track_waits_for_the_full_compile_rules() {
    let mut project = one_row_project();
    write_phrase(&mut project, 1, 32, 20);
    place(&mut project, 0, 5, 1, 1);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NotSimple {
            song_row: 0,
            chain_row: 0,
        }]
    );
}

/// One Song row playing a 6-Step Phrase whose last two Steps are a fast arpeggio. Its group runs
/// on to Steps 6 and 7, which lie past LEN.
fn arpeggio_past_len() -> Project {
    let mut project = one_row_project();
    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_length(6);
    phrase.set_speed(8);
    for index in 0..4 {
        *phrase.step_mut(index) = Step::new(24, 0, 5, 0).unwrap();
    }
    *phrase.step_mut(4) = Step::new(30, 0, 5, 6).unwrap();
    *phrase.step_mut(5) = Step::new(34, 0, 5, 6).unwrap();
    *phrase.step_mut(6) = Step::new(40, 0, 0, 0).unwrap();
    *phrase.step_mut(7) = Step::new(44, 0, 0, 0).unwrap();
    project
}

#[test]
fn an_arpeggio_reading_steps_past_len_sounds_the_same_after_export() {
    let project = arpeggio_past_len();

    let cart = exported_cart(&project, &claim(&[8], (0, 0)));

    let rendered = render_notes(sound_of(&cart, 8), &[0, 1, 2, 3, 4, 5]);
    assert_same_samples(&rendered, &render_song(project, 6 * 8 * 183));
}

#[test]
fn the_steps_an_arpeggio_reads_past_len_stay_silent_in_the_cart() {
    let cart = exported_cart(&arpeggio_past_len(), &claim(&[8], (0, 0)));

    // A channel that does not set the Pattern's length plays on past LEN, so they must not sound.
    assert_eq!(cart.sfx(8).note(6).volume, 0);
    assert_eq!(cart.sfx(8).note(7).volume, 0);
}

#[test]
fn a_pitch_an_arpeggio_reads_past_the_top_of_the_range_is_named() {
    let mut project = arpeggio_past_len();
    *project.phrase_mut(PhraseId::new(0)).step_mut(7) = Step::new(60, 0, 0, 0).unwrap();
    project
        .chain_mut(ChainId::new(0))
        .entry_mut(0)
        .as_mut()
        .unwrap()
        .set_transpose(10);

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NoteOutOfRange {
            phrase: 0,
            step: 7,
            track: 0,
            song_row: 0,
            chain_row: 0,
        }]
    );
}

#[test]
fn a_custom_instrument_waits_for_the_full_compile_rules() {
    let mut project = one_row_project();
    *project.phrase_mut(PhraseId::new(0)).step_mut(2) = Step::new(12, 8, 5, 0).unwrap();

    let failures = export(&project, &claim(&[8], (0, 0)), &blank_cart()).unwrap_err();

    assert_eq!(
        failures,
        [ExportFailure::NotSimple {
            song_row: 0,
            chain_row: 0,
        }]
    );
}
