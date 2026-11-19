//! The PICO-8 reference set: driver Carts recorded on real PICO-8 0.2.7, a measurement fingerprint
//! of each recording, and our synth compared with them. CI never runs PICO-8: it reads the
//! committed Carts and fingerprints. `scripts/record-reference.sh` re-records the whole set on a
//! machine with a licensed copy of PICO-8.

#[path = "../support/mod.rs"]
mod support;

mod cases;
mod fingerprint;
mod render;
mod wav;

use cases::Case;
use fingerprint::{Fingerprint, Tolerance};
use picotracker_core::cart::{Cart, SLOTS};
use picotracker_core::synth::SAMPLES_PER_TICK;
use std::path::{Path, PathBuf};

const FOLDER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/reference_set");
const RECORDINGS_VARIABLE: &str = "PICOTRACKER_RECORDINGS";
const RENDERED_SPARE_TICKS: u32 = 8;
const NOISE: u8 = 6;
const WAVEFORM_INSTRUMENT_FLAG: u8 = 0x80;
const SFX_INSTRUMENTS: u8 = 8;

/// The limits from the spec for matching a recording of real PICO-8. Loosening any of them needs
/// an A/B listening check by the owner, noted in the commit.
#[derive(Clone, Copy, Debug)]
struct Limits {
    duration_percent: f32,
    pitch_cents: f32,
    level_db: f32,
    noise_level_db: f32,
    noiz_buzz_and_detune_1_level_db: f32,
}

const SPEC_LIMITS: Limits = Limits {
    duration_percent: 0.5,
    pitch_cents: 5.0,
    level_db: 0.5,
    noise_level_db: 2.0,
    noiz_buzz_and_detune_1_level_db: 3.0,
};

const AWAITING_SYNTH_FIXES_39: &[&str] = &[
    "coverage_sfx08",
    "coverage_sfx09",
    "coverage_sfx10",
    "coverage_sfx11",
    "coverage_sfx12",
    "coverage_sfx13",
    "coverage_sfx14",
    "coverage_sfx15",
    "coverage_sfx21",
    "coverage_sfx22",
    "coverage_sfx25",
    "coverage_music0",
    "coverage_music1",
    "coverage_music4",
    "drop_on_an_sfx_instrument_keeps_the_pitch",
    "drop_on_an_sfx_instrument_retriggers_the_same_pitch",
    "slide_is_linear_in_pitch",
    "reverb_1_rings_through_silent_steps",
    "reverb_2_rings_through_silent_steps",
    "reverb_2_rings_after_len_inside_a_pattern",
    "arpeggio_inside_an_sfx_instrument_follows_the_played_pitch",
    "slow_arpeggio_inside_an_sfx_instrument_follows_the_played_pitch",
];

const AWAITING_CUSTOM_INSTRUMENTS_40: &[&str] = &[
    "coverage_music4",
    "waveform_instrument_at_c2",
    "waveform_instrument_at_c3",
    "waveform_instrument_bytes_without_the_flag",
    "waveform_instrument_ignores_its_own_noiz_and_buzz",
    "waveform_instrument_ignores_noiz_and_buzz_on_the_playing_sfx",
    "waveform_instrument_with_bass",
    "waveform_instrument_with_dampen",
    "waveform_instrument_with_detune",
    "waveform_instrument_with_reverb",
];

const AWAITING_SYNTH_DIFFERENCES_53: &[&str] = &[
    "arpeggio_inside_an_sfx_instrument_follows_the_played_pitch",
    "bit_7_sfx_beside_another_channel",
    "bit_7_sfx_does_not_set_the_length_beside_looping_channels",
    "bit_7_sfx_is_silent_after_note_31",
    "coverage_music0",
    "coverage_music1",
    "coverage_music2",
    "coverage_music3",
    "coverage_music4",
    "coverage_sfx08",
    "coverage_sfx09",
    "coverage_sfx10",
    "coverage_sfx11",
    "coverage_sfx12",
    "coverage_sfx13",
    "coverage_sfx14",
    "coverage_sfx15",
    "coverage_sfx16",
    "coverage_sfx17",
    "coverage_sfx18",
    "coverage_sfx19",
    "coverage_sfx20",
    "coverage_sfx21",
    "coverage_sfx22",
    "coverage_sfx23",
    "coverage_sfx24",
    "coverage_sfx25",
    "coverage_sfx26",
    "coverage_sfx27",
    "coverage_sfx28",
    "coverage_sfx29",
    "coverage_sfx30",
    "drop_on_an_sfx_instrument_keeps_the_pitch",
    "drop_on_an_sfx_instrument_retriggers_the_same_pitch",
    "len_40_sets_the_pattern_length",
    "len_64_sets_the_pattern_length",
    "len_on_another_channel_keeps_playing",
    "loop_past_note_31_does_not_set_the_length",
    "loop_start_past_loop_end_plays_every_note",
    "looping_channels_play_32_times_the_slowest_speed",
    "repeated_note_does_not_click",
    "repeated_note_does_not_click_across_patterns",
    "reverb_1_rings_through_silent_steps",
    "reverb_2_is_cut_when_sfx_ends_at_len",
    "reverb_2_rings_after_len_inside_a_pattern",
    "reverb_2_rings_through_silent_steps",
    "sfx_instrument_nested_one_level",
    "sfx_stops_at_len",
    "silent_looping_step_after_1_step",
    "silent_looping_step_after_31_steps",
    "silent_looping_step_after_8_steps",
    "silent_looping_step_at_a_faster_speed",
    "silent_looping_steps_on_two_channels",
    "slide_is_linear_in_pitch",
    "slow_arpeggio_inside_an_sfx_instrument_follows_the_played_pitch",
    "spd_0_in_a_pattern_plays_at_speed_1",
    "spd_0_plays_at_speed_1",
    "waveform_instrument_at_c2",
    "waveform_instrument_at_c3",
    "waveform_instrument_bytes_without_the_flag",
    "waveform_instrument_ignores_its_own_noiz_and_buzz",
    "waveform_instrument_ignores_noiz_and_buzz_on_the_playing_sfx",
    "waveform_instrument_with_bass",
    "waveform_instrument_with_dampen",
    "waveform_instrument_with_detune",
    "waveform_instrument_with_reverb",
];

fn cart_path(case: &Case) -> PathBuf {
    Path::new(FOLDER)
        .join("carts")
        .join(format!("{}.p8", case.name))
}

fn fingerprint_path(case: &Case) -> PathBuf {
    Path::new(FOLDER)
        .join("fingerprints")
        .join(format!("{}.ron", case.name))
}

fn synth_hashes_path() -> PathBuf {
    Path::new(FOLDER).join("synth_hashes.txt")
}

fn committed_cart(case: &Case) -> Cart {
    let path = cart_path(case);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    Cart::read(&bytes).unwrap()
}

fn committed_fingerprint(case: &Case) -> Fingerprint {
    let path = fingerprint_path(case);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    ron::from_str(&text).unwrap()
}

/// Our synth's samples for a case: a few Ticks more than PICO-8's recording is measured over.
fn rendered(case: &Case) -> Vec<f32> {
    let samples = (case.ticks + RENDERED_SPARE_TICKS) * SAMPLES_PER_TICK;
    render::render(&committed_cart(case), case.plays, samples as usize)
}

/// The level limit for what a Cart plays: wider for noise, and wider again for NOIZ, BUZZ and
/// DETUNE 1. Waveform Instruments are left out, because their bytes are samples, not notes.
fn level_limit_db(cart: &Cart) -> f32 {
    let mut limit = SPEC_LIMITS.level_db;
    for slot in 0..SLOTS as u8 {
        let sfx = cart.sfx(slot);
        if slot < SFX_INSTRUMENTS && sfx.loop_start() & WAVEFORM_INSTRUMENT_FLAG != 0 {
            continue;
        }
        let notes: Vec<_> = (0..32)
            .map(|index| sfx.note(index))
            .filter(|note| note.volume > 0)
            .collect();
        if notes.is_empty() {
            continue;
        }
        if notes
            .iter()
            .any(|note| !note.custom && note.instrument == NOISE)
        {
            limit = limit.max(SPEC_LIMITS.noise_level_db);
        }
        let filters = sfx.filters();
        let (noiz, buzz, detune) = (filters & 2 != 0, filters & 4 != 0, (filters / 8) % 3);
        if noiz || buzz || detune == 1 {
            limit = limit.max(SPEC_LIMITS.noiz_buzz_and_detune_1_level_db);
        }
    }
    limit
}

fn tolerance(cart: &Cart) -> Tolerance {
    Tolerance {
        duration_percent: SPEC_LIMITS.duration_percent,
        pitch_cents: SPEC_LIMITS.pitch_cents,
        level_db: level_limit_db(cart),
    }
}

const AWAITED_WORK: [(&str, &[&str]); 3] = [
    ("#39 synth fixes", AWAITING_SYNTH_FIXES_39),
    ("#40 Custom Instruments", AWAITING_CUSTOM_INSTRUMENTS_40),
    ("#53 synth differences", AWAITING_SYNTH_DIFFERENCES_53),
];

/// The work a case still waits on before it can match PICO-8: every piece of work whose list
/// names it. A case that no list names must match.
fn awaited_work(name: &str) -> Vec<&'static str> {
    AWAITED_WORK
        .iter()
        .filter(|(_, cases)| cases.contains(&name))
        .map(|(work, _)| *work)
        .collect()
}

#[test]
fn every_case_has_a_unique_name() {
    let cases = cases::all();
    let mut names: Vec<_> = cases.iter().map(|case| case.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), cases.len());
}

#[test]
fn the_committed_driver_carts_are_the_generated_ones() {
    for case in cases::all() {
        let committed = std::fs::read_to_string(cart_path(&case)).unwrap_or_default();
        assert!(
            committed == case.p8(),
            "{} is not the generated driver Cart: run scripts/record-reference.sh",
            case.name
        );
    }
}

#[test]
fn the_expected_failures_name_real_cases() {
    let names: Vec<_> = cases::all().iter().map(|case| case.name).collect();
    for name in AWAITED_WORK.iter().flat_map(|(_, cases)| cases.iter()) {
        assert!(names.contains(name), "{name} is not a case");
    }
}

/// What is wrong with a case's comparison with PICO-8, if anything.
fn problem(case: &Case) -> Option<String> {
    let cart = committed_cart(case);
    let differences = fingerprint::differences(
        &rendered(case),
        &committed_fingerprint(case),
        tolerance(&cart),
    );
    let awaited = awaited_work(case.name);
    if awaited.is_empty() && !differences.is_empty() {
        Some(format!(
            "{} differs from PICO-8:\n    {}",
            case.name,
            differences.join("\n    ")
        ))
    } else if !awaited.is_empty() && differences.is_empty() {
        Some(format!(
            "{} now matches PICO-8: take it off the cases awaiting {}",
            case.name,
            awaited.join(" and ")
        ))
    } else {
        None
    }
}

/// Every case matches its recording of real PICO-8 within the spec's limits, except the cases
/// that wait on unbuilt synth work. Those must still differ, so that the work that makes a case
/// match also takes it off every list that names it.
#[test]
fn every_case_matches_real_pico8_within_the_spec_limits() {
    let cases = cases::all();
    let problems: Vec<String> = std::thread::scope(|scope| {
        let checks: Vec<_> = cases
            .iter()
            .map(|case| scope.spawn(move || problem(case)))
            .collect();
        checks
            .into_iter()
            .filter_map(|check| check.join().unwrap())
            .collect()
    });
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

fn synth_hash(samples: &[f32]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    for sample in samples {
        hasher.update(wav::pcm(*sample).to_le_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn synth_hashes() -> String {
    cases::all()
        .iter()
        .map(|case| format!("{} {}\n", case.name, synth_hash(&rendered(case))))
        .collect()
}

/// Our synth's exact output for each case, as 16-bit samples rendered with the synth's fixed
/// noise seed. Floating point can round differently elsewhere, so only Linux x86_64 checks them.
/// `UPDATE_SNAPSHOTS=1` rewrites the hashes; a commit that changes one must also pass
/// `every_case_matches_real_pico8_within_the_spec_limits`.
#[test]
#[cfg_attr(
    not(all(target_os = "linux", target_arch = "x86_64")),
    ignore = "synth hashes are checked on Linux x86_64 only"
)]
fn the_synth_renders_each_case_to_its_snapshot_hash() {
    let hashes = synth_hashes();
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(synth_hashes_path(), &hashes).unwrap();
    }
    let committed = std::fs::read_to_string(synth_hashes_path()).unwrap_or_default();
    let changed: Vec<_> = hashes
        .lines()
        .filter(|line| !committed.lines().any(|committed| committed == *line))
        .collect();
    assert!(
        changed.is_empty(),
        "synth output changed for:\n{}",
        changed.join("\n")
    );
    assert_eq!(committed.lines().count(), hashes.lines().count());
}

/// Writes the driver Carts. `scripts/record-reference.sh` runs this before recording.
#[test]
#[ignore = "run by scripts/record-reference.sh"]
fn write_driver_carts() {
    let folder = Path::new(FOLDER).join("carts");
    std::fs::create_dir_all(&folder).unwrap();
    for entry in std::fs::read_dir(&folder).unwrap() {
        std::fs::remove_file(entry.unwrap().path()).unwrap();
    }
    for case in cases::all() {
        std::fs::write(cart_path(&case), case.p8()).unwrap();
    }
}

/// Writes our synth's samples for each case as `<case>.synth.wav` beside the recordings in the
/// folder `PICOTRACKER_RECORDINGS` names, for listening to the two side by side.
#[test]
#[ignore = "run by scripts/record-reference.sh"]
fn write_synth_recordings() {
    let recordings = recordings_folder();
    for case in cases::all() {
        let path = recordings.join(format!("{}.synth.wav", case.name));
        std::fs::write(path, wav::bytes(&rendered(&case))).unwrap();
    }
}

fn recordings_folder() -> PathBuf {
    PathBuf::from(
        std::env::var_os(RECORDINGS_VARIABLE)
            .unwrap_or_else(|| panic!("{RECORDINGS_VARIABLE} names no folder")),
    )
}

/// Measures each recording in the folder `PICOTRACKER_RECORDINGS` names and writes its
/// fingerprint. `scripts/record-reference.sh` runs this after recording.
#[test]
#[ignore = "run by scripts/record-reference.sh"]
fn write_fingerprints() {
    let recordings = recordings_folder();
    let folder = Path::new(FOLDER).join("fingerprints");
    std::fs::create_dir_all(&folder).unwrap();
    let pretty = ron::ser::PrettyConfig::new().compact_arrays(true);
    for case in cases::all() {
        let path = recordings.join(format!("{}.wav", case.name));
        let bytes =
            std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let samples =
            wav::samples(&bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let measured = fingerprint::measure(&samples, case.ticks as usize);
        let text = ron::ser::to_string_pretty(&measured, pretty.clone()).unwrap();
        std::fs::write(fingerprint_path(&case), text + "\n").unwrap();
    }
}
