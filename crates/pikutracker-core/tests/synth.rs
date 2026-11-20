use pikutracker_core::synth::{
    Instrument, Note, SAMPLE_RATE, SAMPLES_PER_TICK, Sound, Voice, Waveform, packed_filters,
};

const TRIANGLE: u8 = 0;
const NOISE: u8 = 6;
const A_440: i16 = 33;
const A_880: i16 = 45;
const LOUDEST: u8 = 7;

fn note(pitch: i16) -> Note {
    Note {
        pitch,
        instrument: TRIANGLE,
        custom: false,
        volume: LOUDEST,
        effect: 0,
    }
}

fn held(pitch: i16, speed: u8, filters: u8) -> Sound {
    Sound {
        notes: [note(pitch); 32],
        speed,
        filters,
        loop_start: 0,
        loop_end: 0,
    }
}

fn render(voice: &mut Voice, samples: usize) -> Vec<f32> {
    (0..samples).map(|_| voice.next_sample()).collect()
}

fn rendered(sound: Sound, samples: usize) -> Vec<f32> {
    let mut voice = Voice::new();
    voice.play(sound, 0);
    render(&mut voice, samples)
}

fn frequency(samples: &[f32]) -> f32 {
    let rising: Vec<usize> = samples
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[0] < 0.0 && pair[1] >= 0.0)
        .map(|(index, _)| index)
        .collect();
    let (first, last) = (rising[0], rising[rising.len() - 1]);
    (rising.len() - 1) as f32 * SAMPLE_RATE as f32 / (last - first) as f32
}

fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

fn assert_near(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected} within {tolerance}, got {actual}"
    );
}

#[test]
fn one_tick_is_183_samples_at_22050_hz() {
    assert_eq!(SAMPLE_RATE, 22_050);
    assert_eq!(SAMPLES_PER_TICK, 183);
}

#[test]
fn pitch_33_sounds_at_440_hz() {
    let samples = rendered(held(A_440, 32, 0), 12_000);

    assert_near(frequency(&samples[1_000..]), 440.0, 2.0);
}

#[test]
fn a_full_volume_triangle_peaks_at_half_scale() {
    let samples = rendered(held(A_440, 32, 0), 12_000);

    assert_near(peak(&samples[1_000..]), 0.5, 0.03);
}

#[test]
fn each_note_lasts_speed_ticks() {
    let mut sound = held(A_440, 4, 0);
    sound.notes[1] = note(A_880);
    let note_length = 4 * SAMPLES_PER_TICK as usize;

    let samples = rendered(sound, note_length * 2);

    assert_near(frequency(&samples[200..note_length]), 440.0, 8.0);
    assert_near(frequency(&samples[note_length + 200..]), 880.0, 12.0);
}

#[test]
fn a_sound_without_a_loop_ends_after_its_last_audible_note() {
    let mut sound = held(A_440, 1, 0);
    for silent in &mut sound.notes[2..] {
        silent.volume = 0;
    }
    let mut voice = Voice::new();
    voice.play(sound, 0);

    render(&mut voice, 2 * SAMPLES_PER_TICK as usize + 1);

    assert!(!voice.is_playing());
}

#[test]
fn a_loop_returns_to_its_start_note() {
    let mut sound = held(A_440, 8, 0);
    sound.notes[1] = note(A_880);
    sound.loop_end = 2;
    let note_length = 8 * SAMPLES_PER_TICK as usize;

    let samples = rendered(sound, note_length * 3);

    let third_note = &samples[note_length * 2 + 200..];
    assert_near(frequency(third_note), 440.0, 6.0);
}

#[test]
fn play_can_start_from_a_later_note() {
    let mut sound = held(A_440, 8, 0);
    sound.notes[3] = note(A_880);
    let note_length = 8 * SAMPLES_PER_TICK as usize;

    let mut voice = Voice::new();
    voice.play(sound, 3);
    let samples = render(&mut voice, note_length);

    assert_near(frequency(&samples[200..]), 880.0, 12.0);
}

#[test]
fn stopping_cuts_the_sound_at_once() {
    let mut voice = Voice::new();
    voice.play(held(A_440, 32, packed_filters(false, false, 0, 2, 0)), 0);
    render(&mut voice, 2_000);

    voice.stop();

    assert!(!voice.is_playing());
    assert!(
        render(&mut voice, 2_000)
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn reverb_1_echoes_after_two_ticks() {
    let dry = rendered(held(A_440, 32, 0), 3_000);
    let wet = rendered(
        held(A_440, 32, packed_filters(false, false, 0, 1, 0)),
        3_000,
    );

    assert_eq!(dry[..360], wet[..360]);
    assert_ne!(dry[370..800], wet[370..800]);
}

#[test]
fn reverb_2_echoes_after_four_ticks() {
    let dry = rendered(held(A_440, 32, 0), 3_000);
    let wet = rendered(
        held(A_440, 32, packed_filters(false, false, 0, 2, 0)),
        3_000,
    );

    assert_eq!(dry[..730], wet[..730]);
    assert_ne!(dry[740..1_500], wet[740..1_500]);
}

#[test]
fn each_filter_changes_the_sound() {
    let plain = rendered(held(A_440, 32, 0), 4_000);
    let filters = [
        packed_filters(false, true, 0, 0, 0),
        packed_filters(false, false, 1, 0, 0),
        packed_filters(false, false, 2, 0, 0),
        packed_filters(false, false, 0, 0, 1),
        packed_filters(false, false, 0, 0, 2),
    ];

    for filter in filters {
        assert_ne!(plain, rendered(held(A_440, 32, filter), 4_000), "{filter}");
    }
}

#[test]
fn noiz_changes_the_noise_instrument() {
    let mut sound = held(A_440, 32, 0);
    for note in &mut sound.notes {
        note.instrument = NOISE;
    }
    let mut noiz = sound;
    noiz.filters = packed_filters(true, false, 0, 0, 0);

    assert_ne!(rendered(sound, 4_000), rendered(noiz, 4_000));
}

#[test]
fn noise_renders_the_same_every_time() {
    let mut sound = held(A_440, 32, 0);
    for note in &mut sound.notes {
        note.instrument = NOISE;
    }

    assert_eq!(rendered(sound, 4_000), rendered(sound, 4_000));
}

#[test]
fn fade_out_ends_each_note_near_silence() {
    let mut sound = held(A_440, 32, 0);
    sound.notes[0].effect = 5;
    let note_length = 32 * SAMPLES_PER_TICK as usize;

    let samples = rendered(sound, note_length);

    assert!(peak(&samples[note_length - 400..]) < 0.05);
    assert!(peak(&samples[1_000..2_000]) > 0.3);
}

#[test]
fn a_fast_arpeggio_walks_the_four_notes_of_its_group() {
    let mut sound = held(A_440, 32, 0);
    for (index, pitch) in [A_440, A_880, A_440, A_880].into_iter().enumerate() {
        sound.notes[index] = Note {
            effect: 6,
            ..note(pitch)
        };
    }
    let arpeggio_note_at_speed_32 = 4 * SAMPLES_PER_TICK as usize;

    let samples = rendered(sound, arpeggio_note_at_speed_32 * 2);

    assert_near(
        frequency(&samples[200..arpeggio_note_at_speed_32]),
        440.0,
        12.0,
    );
    assert_near(
        frequency(&samples[arpeggio_note_at_speed_32 + 200..]),
        880.0,
        16.0,
    );
}

#[test]
fn a_note_on_an_sfx_instrument_plays_that_sound_at_the_note_pitch() {
    let c2 = 24;
    let c3 = 36;
    let mut played = held(c3, 32, 0);
    for note in &mut played.notes {
        note.custom = true;
    }

    let mut voice = Voice::new();
    voice.set_sfx_instrument(0, held(c2, 32, 0));
    voice.play(played, 0);
    let samples = render(&mut voice, 8_000);

    assert_near(frequency(&samples[1_000..]), 523.25, 3.0);
}

fn half_high_half_low() -> [i8; 64] {
    std::array::from_fn(|index| if index < 32 { 100 } else { -100 })
}

fn held_on_instrument_8(pitch: i16) -> Sound {
    let mut sound = held(pitch, 32, 0);
    for note in &mut sound.notes {
        note.instrument = 8;
        note.custom = true;
    }
    sound
}

fn on_waveform(waveform: Waveform, pitch: i16, samples: usize) -> Vec<f32> {
    let mut voice = Voice::new();
    voice.set_instrument(8, Instrument::Waveform(waveform));
    voice.play(held_on_instrument_8(pitch), 0);
    render(&mut voice, samples)
}

#[test]
fn a_waveform_instrument_at_c2_plays_one_64_sample_cycle_at_261_6_hz() {
    let c2 = 24;
    let waveform = Waveform {
        samples: half_high_half_low(),
        bass: false,
        filters: 0,
    };

    let samples = on_waveform(waveform, c2, 12_000);

    assert_near(frequency(&samples[1_000..]), 261.63, 1.0);
}

#[test]
fn bass_plays_a_waveform_instrument_one_octave_lower() {
    let c3 = 36;
    let waveform = Waveform {
        samples: half_high_half_low(),
        bass: true,
        filters: 0,
    };

    let samples = on_waveform(waveform, c3, 12_000);

    assert_near(frequency(&samples[1_000..]), 261.63, 1.0);
}

#[test]
fn a_waveform_instrument_moves_between_its_samples_by_linear_interpolation() {
    let c0 = 0;
    let waveform = Waveform {
        samples: half_high_half_low(),
        bass: false,
        filters: 0,
    };

    let samples = on_waveform(waveform, c0, 12_000);

    let swing = peak(&samples[1_000..]) * 2.0;
    let largest_jump = samples[1_000..]
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max);
    assert!(swing > 0.1, "the waveform is silent");
    assert!(
        largest_jump < swing * 0.3,
        "jumped {largest_jump} of a {swing} swing"
    );
}

#[test]
fn a_waveform_instrument_ignores_noiz_and_buzz() {
    let c2 = 24;
    let plain = Waveform {
        samples: half_high_half_low(),
        bass: false,
        filters: 0,
    };
    let filtered = Waveform {
        filters: packed_filters(true, true, 0, 0, 0),
        ..plain
    };

    assert_eq!(
        on_waveform(filtered, c2, 4_000),
        on_waveform(plain, c2, 4_000)
    );
}
