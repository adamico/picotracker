//! The Export proof: an Exported Cart, played the way PICO-8's `music()` plays it, renders sample
//! for sample the same through our synth as the Project's Export Tracks through the Player.

mod support;

use picotracker_core::cart::Cart;
use picotracker_core::export::export;
use picotracker_core::project::{
    Chain, ChainEntry, ChainId, Claim, CustomInstrument, EXPORT_CHANNELS, InstrumentSound,
    PhraseId, Project, SfxInstrument, Step, WaveformInstrument,
};
use picotracker_core::synth::SAMPLES_PER_TICK;
use support::{HEADER, assert_same_samples, render_export_tracks, render_music};

const SFX_INSTRUMENT: u8 = 0x0C;
const WAVEFORM_INSTRUMENT: u8 = 0x20;
const SONG_TRACKS: usize = 5;
const FIRST_PATTERN: u8 = 12;
const TAIL_TICKS: usize = 40;

/// Writes a Phrase of quiet notes, one every other Step, on these instruments in turn. The Steps
/// between them are silent, on the triangle.
fn write_phrase(project: &mut Project, id: u8, length: i32, speed: i32, instruments: &[u8]) {
    let phrase = project.phrase_mut(PhraseId::new(id));
    phrase.set_length(length);
    phrase.set_speed(speed);
    for index in 0..32 {
        let pitch = 20 + (index * 5 % 17) as u8;
        let effect = if index % 6 == 4 { 2 } else { 0 };
        let (instrument, volume) = if index % 2 == 0 {
            (instruments[index / 2 % instruments.len()], 2)
        } else {
            (0, 0)
        };
        *phrase.step_mut(index) = Step::new(pitch, instrument, volume, effect).unwrap();
    }
}

fn plays(phrase: u8) -> Option<ChainEntry> {
    Some(ChainEntry::new(PhraseId::new(phrase)))
}

fn fills(phrase: u8) -> Option<ChainEntry> {
    let mut entry = ChainEntry::new(PhraseId::new(phrase));
    entry.set_fill(true);
    Some(entry)
}

fn transposed(phrase: u8, semitones: i32) -> Option<ChainEntry> {
    let mut entry = ChainEntry::new(PhraseId::new(phrase));
    entry.set_transpose(semitones);
    Some(entry)
}

/// Puts one Chain per Track on a Song row, its entries given Chain row by Chain row.
fn place_song_row(
    project: &mut Project,
    song_row: usize,
    chain_rows: &[[Option<ChainEntry>; SONG_TRACKS]],
) {
    for track in 0..SONG_TRACKS {
        let mut chain = Chain::default();
        for (chain_row, entries) in chain_rows.iter().enumerate() {
            *chain.entry_mut(chain_row) = entries[track];
        }
        let id = ChainId::new((song_row * SONG_TRACKS + track) as u8);
        *project.chain_mut(id) = chain;
        project.set_song_cell(song_row, track, Some(id));
    }
}

/// A Project with an SFX Instrument and a Waveform Instrument, whose first 4 Tracks are Exported.
fn instrumented_project() -> Project {
    let mut project = Project::new();
    let mut sfx = SfxInstrument {
        steps: vec![
            Step::new(24, 3, 5, 0).unwrap(),
            Step::new(31, 1, 4, 0).unwrap(),
            Step::new(36, 0, 6, 0).unwrap(),
            Step::new(28, 5, 3, 0).unwrap(),
        ],
        speed: 2,
        loop_start: 0,
        loop_end: 4,
        ..SfxInstrument::default()
    };
    sfx.filters.set_dampen(1);
    project.set_custom_instrument(
        SFX_INSTRUMENT,
        Some(CustomInstrument {
            name: None,
            sound: InstrumentSound::Sfx(sfx),
        }),
    );
    let wave = WaveformInstrument {
        samples: std::array::from_fn(|index| (index as i32 * 4 - 128) as i8),
        detune: 1,
        ..WaveformInstrument::default()
    };
    project.set_custom_instrument(
        WAVEFORM_INSTRUMENT,
        Some(CustomInstrument {
            name: None,
            sound: InstrumentSound::Waveform(wave),
        }),
    );
    for channel in 0..EXPORT_CHANNELS {
        project.set_export_track(channel, Some(channel as u8 + 1));
    }
    project.set_song_loop_row(None);
    project
}

/// Exports a Project and checks that the Cart plays its Song, `song_ticks` long, and a silent tail
/// after it, exactly as the Player plays its Export Tracks.
fn assert_exports_sample_for_sample(project: &Project, song_ticks: usize) {
    let claim = Claim {
        sfx: (0..4).chain(10..30).collect(),
        patterns: Some((FIRST_PATTERN, 30)),
    };
    let original = format!("{HEADER}__lua__\nprint(1)\n");

    let exported = export(project, &claim, original.as_bytes()).unwrap();

    assert_eq!(exported.warnings, []);
    let cart = Cart::read(&exported.cart_bytes).unwrap();
    let samples = (song_ticks + TAIL_TICKS) * SAMPLES_PER_TICK as usize;
    assert_same_samples(
        &render_music(&cart, FIRST_PATTERN, samples),
        &render_export_tracks(project.clone(), samples),
    );
}

#[test]
fn every_kind_of_chain_row_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, 3]);
    write_phrase(&mut project, 1, 16, 6, &[2, 4, 1]);
    write_phrase(&mut project, 2, 4, 6, &[SFX_INSTRUMENT, 0x0E]);
    write_phrase(&mut project, 3, 16, 3, &[WAVEFORM_INSTRUMENT, 5]);
    write_phrase(&mut project, 4, 8, 8, &[1]);
    write_phrase(&mut project, 5, 6, 8, &[3, 0]);
    write_phrase(&mut project, 6, 16, 8, &[4]);
    write_phrase(&mut project, 7, 4, 8, &[2]);
    write_phrase(&mut project, 8, 32, 5, &[0, SFX_INSTRUMENT]);
    place_song_row(
        &mut project,
        0,
        &[
            // A shorter Phrase left of the longest, a Fill and a shorter one on the right.
            [plays(0), plays(1), fills(2), plays(3), None],
            // Every entry marked Fill.
            [fills(4), fills(5), None, None, None],
            // Timed by the unexported T5, with a shorter Phrase left of the free channel.
            [plays(7), None, fills(2), None, plays(6)],
            // No Exported entries at all.
            [None, None, None, None, plays(4)],
        ],
    );
    place_song_row(
        &mut project,
        1,
        &[[plays(0), transposed(8, 5), plays(8), None, None]],
    );

    // 96 + 64 + 128 + 64 Ticks on Song row 0, then 160 on Song row 1.
    assert_exports_sample_for_sample(&project, 96 + 64 + 128 + 64 + 160);
}

#[test]
fn phrase_filters_play_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, 6, 7]);
    write_phrase(&mut project, 1, 16, 6, &[2, SFX_INSTRUMENT, 7]);
    write_phrase(&mut project, 2, 4, 6, &[WAVEFORM_INSTRUMENT, 1]);
    write_phrase(&mut project, 3, 16, 6, &[5, 3]);
    project
        .phrase_mut(PhraseId::new(0))
        .filters_mut()
        .set_reverb(2);
    let filters = project.phrase_mut(PhraseId::new(1)).filters_mut();
    filters.set_detune(1);
    filters.set_reverb(1);
    let filters = project.phrase_mut(PhraseId::new(2)).filters_mut();
    filters.set_detune(2);
    filters.set_reverb(2);
    let filters = project.phrase_mut(PhraseId::new(3)).filters_mut();
    filters.set_dampen(2);
    filters.set_noiz(true);
    filters.set_buzz(true);
    filters.set_detune(2);
    place_song_row(
        &mut project,
        0,
        &[
            [plays(0), plays(1), fills(2), plays(3), None],
            [plays(3), plays(0), plays(1), fills(2), None],
        ],
    );

    assert_exports_sample_for_sample(&project, 96 + 96);
}

#[test]
fn a_shorter_32_step_phrase_left_of_a_longer_one_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 32, 3, &[0, 4]);
    write_phrase(&mut project, 1, 32, 5, &[2, 3]);
    project
        .phrase_mut(PhraseId::new(0))
        .filters_mut()
        .set_reverb(2);
    place_song_row(
        &mut project,
        0,
        &[
            [plays(0), plays(1), None, None, None],
            [plays(1), plays(0), None, None, None],
        ],
    );

    assert_exports_sample_for_sample(&project, 160 + 160);
}

#[test]
fn a_fill_longer_than_its_chain_row_is_cut_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 32, 8, &[0, 5]);
    write_phrase(&mut project, 1, 8, 6, &[2]);
    place_song_row(
        &mut project,
        0,
        &[
            [fills(0), plays(1), None, None, None],
            [plays(1), fills(0), None, None, None],
        ],
    );

    assert_exports_sample_for_sample(&project, 48 + 48);
}

#[test]
fn an_all_fill_chain_row_timed_by_an_unexported_track_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, 3]);
    write_phrase(&mut project, 1, 16, 8, &[1]);
    place_song_row(
        &mut project,
        0,
        &[[fills(0), fills(0), None, None, fills(1)]],
    );

    assert_exports_sample_for_sample(&project, 128);
}

#[test]
fn a_looping_song_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, SFX_INSTRUMENT]);
    write_phrase(&mut project, 1, 16, 6, &[2, 4]);
    write_phrase(&mut project, 2, 4, 6, &[WAVEFORM_INSTRUMENT]);
    place_song_row(&mut project, 0, &[[plays(1), None, None, None, None]]);
    place_song_row(
        &mut project,
        1,
        &[
            [plays(0), plays(1), None, None, None],
            [fills(2), plays(0), None, None, None],
        ],
    );
    project.set_song_loop_row(Some(1));

    // Song row 0, then Song row 1 three times over.
    assert_exports_sample_for_sample(&project, 96 + 3 * (96 + 48));
}

#[test]
fn a_shorter_phrase_under_reverb_and_dampen_left_of_a_longer_one_plays_the_same_after_export() {
    let mut project = instrumented_project();
    write_phrase(&mut project, 0, 8, 6, &[0, 3]);
    write_phrase(&mut project, 1, 16, 6, &[2, 4]);
    let filters = project.phrase_mut(PhraseId::new(0)).filters_mut();
    filters.set_reverb(2);
    filters.set_dampen(2);
    place_song_row(&mut project, 0, &[[plays(0), plays(1), None, None, None]]);

    assert_exports_sample_for_sample(&project, 96);
}

/// Writes a Phrase under DETUNE 1 and REVERB 1 whose silent Steps name this instrument.
fn write_silent_steps_on(project: &mut Project, id: u8, audible: &[u8], silent_instrument: u8) {
    write_phrase(project, id, 16, 6, audible);
    let phrase = project.phrase_mut(PhraseId::new(id));
    for index in (1..16).step_by(2) {
        *phrase.step_mut(index) = Step::new(30, silent_instrument, 0, 0).unwrap();
    }
    let filters = phrase.filters_mut();
    filters.set_detune(1);
    filters.set_reverb(1);
}

#[test]
fn silent_steps_on_custom_instruments_under_detune_and_reverb_play_the_same_after_export() {
    let mut project = instrumented_project();
    write_silent_steps_on(&mut project, 0, &[3, SFX_INSTRUMENT], SFX_INSTRUMENT);
    write_silent_steps_on(&mut project, 1, &[2], WAVEFORM_INSTRUMENT);
    write_silent_steps_on(&mut project, 2, &[4], 0x0E);
    place_song_row(
        &mut project,
        0,
        &[[plays(0), plays(1), plays(2), None, None]],
    );

    assert_exports_sample_for_sample(&project, 96);
}
