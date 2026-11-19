//! Custom Instruments: designing them in the Instrument view and hearing them in Phrases.

mod support;

use picotracker_core::project::{
    ChainEntry, ChainId, CustomInstrument, Filters, InstrumentSound, PhraseId, Project,
    SfxInstrument, Step, WaveformInstrument,
};
use picotracker_core::synth::SAMPLE_RATE;
use support::render_song;

const C2: u8 = 24;
const A2: u8 = 33;
const C3: u8 = 36;
const TRIANGLE: u8 = 0;
const SETTLED: usize = 2_000;

fn step(pitch: u8, instrument: u8, volume: u8) -> Step {
    Step::new(pitch, instrument, volume, 0).unwrap()
}

/// A Project whose Song plays one Phrase holding one held note on this Instrument.
fn playing_one_note(pitch: u8, instrument: u8) -> Project {
    let mut project = Project::new();
    project.set_song_cell(0, 0, Some(ChainId::new(0)));
    *project.chain_mut(ChainId::new(0)).entry_mut(0) = Some(ChainEntry::new(PhraseId::new(0)));
    let phrase = project.phrase_mut(PhraseId::new(0));
    phrase.set_speed(64);
    phrase.set_length(1);
    *phrase.step_mut(0) = step(pitch, instrument, 5);
    project
}

fn waveform_instrument() -> CustomInstrument {
    CustomInstrument {
        name: None,
        sound: InstrumentSound::Waveform(WaveformInstrument {
            samples: std::array::from_fn(|index| if index < 32 { 100 } else { -100 }),
            bass: false,
            detune: 0,
            reverb: 0,
            dampen: 0,
        }),
    }
}

/// An SFX Instrument holding A-2 on the triangle.
fn sfx_instrument() -> CustomInstrument {
    CustomInstrument {
        name: None,
        sound: InstrumentSound::Sfx(SfxInstrument {
            steps: vec![step(A2, TRIANGLE, 7); 32],
            speed: 32,
            filters: Filters::default(),
            loop_start: 0,
            loop_end: 32,
        }),
    }
}

/// The frequency of a tone, counted from its rising zero crossings.
fn frequency(samples: &[f32]) -> f32 {
    let rising: Vec<usize> = samples
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(index, _)| index)
        .collect();
    assert!(rising.len() > 2, "no tone");
    let (first, last) = (rising[0], rising[rising.len() - 1]);
    (rising.len() - 1) as f32 * SAMPLE_RATE as f32 / (last - first) as f32
}

fn assert_near(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} within {tolerance}, got {actual}"
    );
}

#[test]
fn a_step_on_a_waveform_instrument_at_c2_plays_at_261_6_hz() {
    let mut project = playing_one_note(C2, 0x30);
    project.set_custom_instrument(0x30, Some(waveform_instrument()));

    let samples = render_song(project, 10_000);

    assert_near(frequency(&samples[SETTLED..]), 261.63, 1.0);
}

#[test]
fn an_sfx_instrument_plays_relative_to_c2() {
    let mut project = playing_one_note(C3, 0x08);
    project.set_custom_instrument(0x08, Some(sfx_instrument()));

    let samples = render_song(project, 10_000);

    assert_near(frequency(&samples[SETTLED..]), 880.0, 4.0);
}

#[test]
fn a_project_plays_more_than_8_custom_instruments() {
    let mut project = playing_one_note(C2, 0x10);
    for id in 0x08..0x10 {
        project.set_custom_instrument(id, Some(sfx_instrument()));
    }
    project.set_custom_instrument(0x10, Some(waveform_instrument()));

    let samples = render_song(project, 10_000);

    assert_near(frequency(&samples[SETTLED..]), 261.63, 1.0);
}

#[test]
fn a_step_on_an_undefined_custom_instrument_is_silent() {
    let project = playing_one_note(C2, 0x40);

    let samples = render_song(project, 10_000);

    assert!(samples.iter().all(|sample| *sample == 0.0));
}
