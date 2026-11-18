//! The music compiler seam: a Cart in, a Project and warnings out. Every Cart here is generated in
//! test code. None was saved by PICO-8.

mod support;

use picotracker_core::cart::Cart;
use picotracker_core::cart_import::import;
use picotracker_core::player::{AudioRequest, Player};
use picotracker_core::project::{ChainId, Project, TRACKS};
use picotracker_core::project_file;
use picotracker_core::synth::{Note, SAMPLES_PER_TICK, Sound, Voice, packed_filters};
use std::sync::Arc;
use support::{TestCart, TestPattern, TestSfx};

const CART_NAME: &str = "demo.p8";

fn imported(cart: &TestCart) -> (Project, Vec<String>) {
    let imported = import(&Cart::read(&cart.p8()).unwrap(), CART_NAME);
    (imported.project, imported.warnings)
}

fn pattern(channels: [Option<u8>; 4]) -> TestPattern {
    TestPattern::playing(channels)
}

/// Two different SFX at Speed 8.
fn two_sfx() -> Vec<TestSfx> {
    vec![
        TestSfx::with_pitches(8, &[12, 14]),
        TestSfx::with_pitches(8, &[19, 21]),
    ]
}

fn chain_on(project: &Project, row: usize, track: usize) -> ChainId {
    project.song_cell(row, track).unwrap()
}

#[test]
fn each_pattern_becomes_one_song_row_with_channel_n_on_track_n() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![
            pattern([Some(0), Some(1), Some(0), Some(1)]),
            pattern([Some(1), Some(0), Some(1), Some(0)]),
        ],
    };

    let (project, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
    for (row, sfx_on_tracks) in [[12, 19, 12, 19], [19, 12, 19, 12]].iter().enumerate() {
        for (track, pitch) in sfx_on_tracks.iter().enumerate() {
            let entry = project.chain_row_entry(row, track, 0).unwrap();
            assert_eq!(project.phrase(entry.phrase()).step(0).pitch(), *pitch);
        }
        for track in 4..TRACKS {
            assert_eq!(project.song_cell(row, track), None);
        }
    }
    assert!(project.song_row_is_empty(2));
}

#[test]
fn the_export_tracks_are_tracks_1_to_4() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    assert_eq!(
        project.export_tracks(),
        [Some(1), Some(2), Some(3), Some(4)]
    );
}

#[test]
fn a_phrase_keeps_every_value_of_its_sfx() {
    let mut sfx = TestSfx::with_pitches(23, &[12, 14]);
    // NOIZ, BUZZ, DETUNE 1, REVERB 2 and DAMPEN 1: 2 + 4 + 8 + 48 + 72, with the editor bit.
    sfx.editor_and_filters = 135;
    sfx.notes[1].instrument = 5;
    sfx.notes[1].volume = 6;
    sfx.notes[1].effect = 3;
    // A silent note keeps its pitch and Instrument.
    sfx.notes[2].pitch = 40;
    sfx.notes[2].instrument = 2;
    let cart = TestCart {
        sfx: vec![sfx],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    let entry = project.chain_row_entry(0, 0, 0).unwrap();
    let phrase = project.phrase(entry.phrase());
    assert_eq!(phrase.length(), 32);
    assert_eq!(phrase.speed(), 23);
    let filters = phrase.filters();
    assert!(filters.noiz() && filters.buzz());
    assert_eq!(
        (filters.detune(), filters.reverb(), filters.dampen()),
        (1, 2, 1)
    );
    let step = phrase.step(1);
    assert_eq!(
        (
            step.pitch(),
            step.instrument(),
            step.volume(),
            step.effect()
        ),
        (14, 5, 6, 3)
    );
    let silent = phrase.step(2);
    assert_eq!(
        (silent.pitch(), silent.instrument(), silent.volume()),
        (40, 2, 0)
    );
}

#[test]
fn a_disabled_channel_becomes_an_empty_chain_entry() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), None, Some(1), None])],
    };

    let (project, _) = imported(&cart);

    let chain = chain_on(&project, 0, 1);
    assert_eq!(project.chain(chain).entry(0), None);
    assert_eq!(project.chain_row_entry(0, 3, 0), None);
    assert!(project.chain_row_entry(0, 2, 0).is_some());
}

#[test]
fn identical_chains_and_exact_match_phrases_are_shared() {
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[12]),
            TestSfx::with_pitches(8, &[19]),
            // Slot 2 holds exactly what slot 0 holds.
            TestSfx::with_pitches(8, &[12]),
        ],
        patterns: vec![
            pattern([Some(0), Some(1), None, None]),
            pattern([Some(2), Some(0), None, None]),
        ],
    };

    let (project, _) = imported(&cart);

    let shared = chain_on(&project, 0, 0);
    assert_eq!(chain_on(&project, 1, 0), shared);
    assert_eq!(chain_on(&project, 1, 1), shared);
    assert_ne!(chain_on(&project, 0, 1), shared);
    // The two disabled channels share one Chain too.
    assert_eq!(chain_on(&project, 0, 2), chain_on(&project, 1, 3));
}

#[test]
fn phrases_that_differ_only_in_a_silent_step_are_not_shared() {
    let mut other = TestSfx::with_pitches(8, &[12]);
    other.notes[5].pitch = 33;
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(8, &[12]), other],
        patterns: vec![pattern([Some(0), Some(1), None, None])],
    };

    let (project, _) = imported(&cart);

    let first = project.chain_row_entry(0, 0, 0).unwrap().phrase();
    let second = project.chain_row_entry(0, 1, 0).unwrap().phrase();
    assert_ne!(first, second);
}

#[test]
fn the_claim_covers_exactly_the_songs_sfx_and_patterns() {
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(8, &[1]),
            TestSfx::with_pitches(8, &[12]),
            TestSfx::with_pitches(8, &[2]),
            TestSfx::with_pitches(8, &[19]),
        ],
        patterns: vec![
            pattern([Some(3), None, None, None]),
            pattern([Some(1), Some(3), None, None]),
        ],
    };

    let (project, _) = imported(&cart);

    let target = project.target_cart().unwrap();
    assert_eq!(target.path, CART_NAME);
    assert_eq!(target.claim.sfx, [1, 3]);
    assert_eq!(target.claim.patterns, Some((0, 1)));
}

#[test]
fn the_claim_includes_the_sfx_that_notes_use_as_instruments() {
    let mut sfx = TestSfx::with_pitches(8, &[12]);
    sfx.notes[0].custom = true;
    sfx.notes[0].instrument = 6;
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(8, &[1]), sfx],
        patterns: vec![pattern([Some(1), None, None, None])],
    };

    let (project, warnings) = imported(&cart);

    assert_eq!(project.target_cart().unwrap().claim.sfx, [1, 6]);
    assert_eq!(warnings, ["SFX 01 PLAYS SFX INSTRUMENTS AS BUILT-IN ONES"]);
}

#[test]
fn the_claimed_bytes_hash_is_stored_with_one_checksum_per_slot() {
    let cart = TestCart {
        sfx: vec![TestSfx::with_pitches(8, &[12, 14])],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    // FNV-1a over the 68 and the 4 claimed bytes, worked out outside this code base.
    assert_eq!(
        project.target_cart().unwrap().claimed_bytes_hash.as_deref(),
        Some("S00=6bd8a93f P00=d0cfb638")
    );
}

#[test]
fn a_song_ending_in_stop_does_not_loop_and_stops_the_claim() {
    let mut last = pattern([Some(1), None, None, None]);
    last.stop = true;
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![
            pattern([Some(0), None, None, None]),
            last,
            pattern([Some(0), None, None, None]),
        ],
    };

    let (project, warnings) = imported(&cart);

    assert_eq!(project.song_loop_row(), None);
    assert_eq!(project.target_cart().unwrap().claim.patterns, Some((0, 1)));
    assert!(project.song_row_is_empty(2));
    assert_eq!(warnings, ["CART HOLDS MORE MUSIC: IMPORTED SONG 1 ONLY"]);
}

#[test]
fn loop_back_jumps_to_the_nearest_loop_start_before_it() {
    let mut start = pattern([Some(1), None, None, None]);
    start.loop_start = true;
    let mut last = pattern([Some(0), None, None, None]);
    last.loop_back = true;
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), None, None, None]), start, last],
    };

    let (project, _) = imported(&cart);

    assert_eq!(project.song_loop_row(), Some(1));
}

#[test]
fn a_song_that_runs_into_an_empty_pattern_ends_there_without_a_loop() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (project, _) = imported(&cart);

    assert_eq!(project.song_loop_row(), None);
    assert_eq!(project.target_cart().unwrap().claim.patterns, Some((0, 0)));
}

#[test]
fn a_cart_with_no_music_gives_an_empty_song_and_an_empty_claim() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![],
    };

    let (project, _) = imported(&cart);

    assert!(project.song_row_is_empty(0));
    let claim = &project.target_cart().unwrap().claim;
    assert!(claim.sfx.is_empty());
    assert_eq!(claim.patterns, None);
}

#[test]
fn a_pattern_whose_timing_is_not_simple_says_so() {
    let mut looping = TestSfx::with_pitches(8, &[12]);
    looping.loop_start = 0;
    looping.loop_end = 8;
    let cart = TestCart {
        sfx: vec![looping],
        patterns: vec![pattern([Some(0), None, None, None])],
    };

    let (_, warnings) = imported(&cart);

    assert_eq!(warnings, ["PATTERN 00 MAY NOT PLAY AS IN PICO-8: TIMING"]);
}

#[test]
fn a_shorter_channel_right_of_the_longest_is_simple() {
    let cart = TestCart {
        sfx: vec![
            TestSfx::with_pitches(16, &[12]),
            TestSfx::with_pitches(8, &[19]),
        ],
        patterns: vec![pattern([Some(0), Some(1), None, None])],
    };

    let (_, warnings) = imported(&cart);

    assert_eq!(warnings, Vec::<String>::new());
}

/// The sound a channel plays for an SFX of a generated Cart.
fn cart_sound(sfx: &TestSfx) -> Sound {
    let mut notes = [Note::default(); 32];
    for (note, written) in notes.iter_mut().zip(&sfx.notes) {
        *note = Note {
            pitch: i16::from(written.pitch),
            instrument: written.instrument,
            custom: false,
            volume: written.volume,
            effect: written.effect,
        };
    }
    let byte = sfx.editor_and_filters;
    Sound {
        notes,
        speed: sfx.speed,
        filters: packed_filters(
            byte & 2 != 0,
            byte & 4 != 0,
            (byte / 8) % 3,
            (byte / 24) % 3,
            (byte / 72) % 3,
        ),
        loop_start: 0,
        loop_end: 32,
    }
}

/// Plays a Cart's Patterns straight through the synth, stepping each channel's notes the way
/// PICO-8 does, without going through a Project.
fn render_cart(cart: &TestCart) -> Vec<f32> {
    let mut voices = vec![Voice::new(); 4];
    let mut samples = Vec::new();
    for pattern in &cart.patterns {
        let sounds = pattern
            .channels
            .map(|channel| channel.map(|slot| cart_sound(&cart.sfx[usize::from(slot)])));
        let length = sounds
            .iter()
            .flatten()
            .next()
            .map_or(0, |sound| 32 * usize::from(sound.speed));
        for (voice, sound) in voices.iter_mut().zip(&sounds) {
            voice.play(sound.unwrap_or_default(), 0);
        }
        for tick in 0..length {
            for _ in 0..SAMPLES_PER_TICK {
                samples.push(voices.iter_mut().map(Voice::next_sample).sum());
            }
            for (voice, sound) in voices.iter_mut().zip(&sounds) {
                let Some(sound) = sound else { continue };
                let speed = usize::from(sound.speed);
                let next_tick = tick + 1;
                if next_tick < length && next_tick % speed == 0 {
                    if next_tick / speed < 32 {
                        voice.continue_at(*sound, next_tick / speed);
                    } else {
                        voice.play(Sound::default(), 0);
                    }
                }
            }
        }
    }
    samples
}

fn render_project(project: Project, samples: usize) -> Vec<f32> {
    let mut player = Player::new();
    player.handle(AudioRequest::PlaySong {
        project: Arc::new(project),
        from_row: 0,
    });
    player.render(samples)
}

#[test]
fn an_import_with_no_warnings_renders_the_same_samples_as_the_cart() {
    let mut lead = TestSfx::with_pitches(6, &[24, 27, 31, 36, 31, 27, 24, 19]);
    lead.editor_and_filters = 2 * 24;
    lead.notes[3].effect = 2;
    let mut bass = TestSfx::with_pitches(12, &[12, 12, 7, 7]);
    bass.notes[1].instrument = 3;
    let mut slide = TestSfx::with_pitches(6, &[40, 44]);
    slide.notes[1].effect = 1;
    slide.notes[2].pitch = 48;
    slide.notes[2].volume = 2;
    slide.notes[2].instrument = 2;
    let mut stop = pattern([Some(1), Some(2), Some(0), None]);
    stop.stop = true;
    let cart = TestCart {
        sfx: vec![lead, bass, slide],
        patterns: vec![
            pattern([Some(1), Some(0), None, None]),
            pattern([Some(1), None, Some(2), Some(0)]),
            stop,
        ],
    };

    let (project, warnings) = imported(&cart);
    assert_eq!(warnings, Vec::<String>::new());

    let expected = render_cart(&cart);
    let rendered = render_project(project, expected.len());

    assert!(expected.iter().any(|sample| sample.abs() > 0.01));
    let first_difference = expected
        .iter()
        .zip(&rendered)
        .position(|(expected, rendered)| expected != rendered);
    assert_eq!(first_difference, None);
}

/// A small song: a loop over two Patterns, a shared Phrase and a disabled channel.
fn snapshot_cart() -> TestCart {
    let mut first = pattern([Some(0), Some(1), None, None]);
    first.loop_start = true;
    let mut last = pattern([Some(1), None, Some(0), None]);
    last.loop_back = true;
    TestCart {
        sfx: vec![
            TestSfx::with_pitches(16, &[12, 14]),
            TestSfx::with_pitches(16, &[19]),
        ],
        patterns: vec![first, last],
    }
}

#[test]
fn an_imported_project_saves_as_the_expected_ptrk_text() {
    let (project, warnings) = imported(&snapshot_cart());

    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(
        project_file::write(&project),
        include_str!("snapshots/imported_cart.ptrk")
    );
}

#[test]
fn a_png_cart_imports_to_the_same_project_as_its_text_twin() {
    let cart = TestCart {
        sfx: two_sfx(),
        patterns: vec![pattern([Some(0), Some(1), None, None])],
    };

    let from_text = import(&Cart::read(&cart.p8()).unwrap(), CART_NAME);
    let from_pixels = import(&Cart::read(&cart.png()).unwrap(), CART_NAME);

    assert_eq!(from_text.project, from_pixels.project);
}
