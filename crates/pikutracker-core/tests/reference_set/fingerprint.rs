//! The measurement fingerprint of one recording, and the comparison of our synth's samples with
//! it. Both sides are measured the same way, from the first audible sample.

use pikutracker_core::synth::{SAMPLE_RATE, SAMPLES_PER_TICK};
use serde::{Deserialize, Serialize};
use std::ops::Range;

const AUDIBLE: f32 = 0.002;
const LEVEL_FLOOR_DB: f32 = -50.0;
const PITCH_WINDOW_TICKS: usize = 4;
const TONAL_CLARITY: f32 = 0.9;
const SHORTEST_PERIOD: usize = 4;
const A_440_PITCH: f32 = 33.0;
const CENTS_PER_PITCH: f32 = 100.0;
const LISTED_DIFFERENCES: usize = 4;

/// What a recording measures, from its first audible sample up to `ticks` Ticks later: how long
/// it sounds in samples, its RMS level in dBFS for each Tick (never below the floor), and its
/// pitch in PICO-8 pitch numbers (33 is A at 440 Hz) for each window of four Ticks that holds a
/// clear tone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub duration_samples: usize,
    pub tick_levels_db: Vec<f32>,
    pub window_pitches: Vec<Option<f32>>,
}

/// How closely our synth must match a fingerprint. Each span of Ticks in `level_averaged_tick_spans`
/// is held to the level limit on its average level, and every other Tick on its own level. A pitch
/// window holding any Tick in `pitch_skipped_ticks` is not compared.
#[derive(Clone, Debug)]
pub struct Tolerance {
    pub duration_percent: f32,
    pub pitch_cents: f32,
    pub level_db: f32,
    pub level_averaged_tick_spans: Vec<Range<usize>>,
    pub pitch_skipped_ticks: Vec<usize>,
}

/// The samples from the first audible one, `ticks` Ticks long, padded with silence.
fn window(samples: &[f32], ticks: usize) -> Vec<f32> {
    let length = ticks * SAMPLES_PER_TICK as usize;
    let onset = samples
        .iter()
        .position(|sample| sample.abs() >= AUDIBLE)
        .unwrap_or(samples.len());
    let mut window: Vec<f32> = samples[onset..].iter().copied().take(length).collect();
    window.resize(length, 0.0);
    window
}

fn level_db(samples: &[f32]) -> f32 {
    let power = samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32;
    (10.0 * power.log10()).max(LEVEL_FLOOR_DB)
}

/// The pitch of a window, and how clearly the window repeats at its period, from 0 to 1. The
/// period is the shortest that repeats clearly enough to count as a tone, or else the one that
/// repeats best.
fn pitch_and_clarity(samples: &[f32]) -> Option<(f32, f32)> {
    let longest = samples.len() / 2;
    let correlation = |lag: usize| {
        let (early, late) = (&samples[..samples.len() - lag], &samples[lag..]);
        let product: f32 = early.iter().zip(late).map(|(a, b)| a * b).sum();
        let energy =
            early.iter().map(|a| a * a).sum::<f32>() * late.iter().map(|b| b * b).sum::<f32>();
        if energy > 0.0 {
            product / energy.sqrt()
        } else {
            0.0
        }
    };
    let correlations: Vec<f32> = (0..=longest + 1).map(correlation).collect();
    let past_first_lobe = (SHORTEST_PERIOD..=longest).find(|&lag| correlations[lag] <= 0.0)?;
    let peaks: Vec<usize> = (past_first_lobe..=longest)
        .filter(|&lag| {
            correlations[lag] >= correlations[lag - 1] && correlations[lag] > correlations[lag + 1]
        })
        .collect();
    let strongest = peaks
        .iter()
        .copied()
        .max_by(|a, b| correlations[*a].total_cmp(&correlations[*b]))?;
    let lag = peaks
        .iter()
        .copied()
        .find(|&lag| correlations[lag] >= TONAL_CLARITY)
        .unwrap_or(strongest);
    let (before, at, after) = (
        correlations[lag - 1],
        correlations[lag],
        correlations[lag + 1],
    );
    let curve = before - 2.0 * at + after;
    let shift = if curve == 0.0 {
        0.0
    } else {
        0.5 * (before - after) / curve
    };
    let frequency = SAMPLE_RATE as f32 / (lag as f32 + shift);
    let pitch = A_440_PITCH + 12.0 * (frequency / 440.0).log2();
    Some((pitch, at))
}

fn pitch_windows(window: &[f32]) -> impl Iterator<Item = &[f32]> {
    window
        .as_chunks::<{ PITCH_WINDOW_TICKS * SAMPLES_PER_TICK as usize }>()
        .0
        .iter()
        .map(|chunk| chunk.as_slice())
}

/// Measures samples at 22,050 Hz over `ticks` Ticks from their first audible sample.
pub fn measure(samples: &[f32], ticks: usize) -> Fingerprint {
    let window = window(samples, ticks);
    let duration_samples = window
        .iter()
        .rposition(|sample| sample.abs() >= AUDIBLE)
        .map_or(0, |last| last + 1);
    let tick_levels_db = window
        .as_chunks::<{ SAMPLES_PER_TICK as usize }>()
        .0
        .iter()
        .map(|tick| (100.0 * level_db(tick)).round() / 100.0)
        .collect();
    let window_pitches = pitch_windows(&window)
        .map(|samples| {
            pitch_and_clarity(samples)
                .filter(|(_, clarity)| *clarity >= TONAL_CLARITY)
                .map(|(pitch, _)| (1000.0 * pitch).round() / 1000.0)
        })
        .collect();
    Fingerprint {
        duration_samples,
        tick_levels_db,
        window_pitches,
    }
}

/// Where these samples differ from a fingerprint by more than the tolerance, in words. An empty
/// list means they match.
pub fn differences(samples: &[f32], reference: &Fingerprint, tolerance: &Tolerance) -> Vec<String> {
    let ticks = reference.tick_levels_db.len();
    let ours = measure(samples, ticks);
    let mut differences = Vec::new();

    let expected = reference.duration_samples as f32;
    let actual = ours.duration_samples as f32;
    if (actual - expected).abs() > expected * tolerance.duration_percent / 100.0 {
        differences.push(format!(
            "duration {actual} samples, PICO-8 {expected} (limit ±{}%)",
            tolerance.duration_percent
        ));
    }

    let spans = tolerance.level_averaged_tick_spans.iter().cloned();
    let single_ticks = (0..ticks)
        .filter(|tick| {
            !tolerance
                .level_averaged_tick_spans
                .iter()
                .any(|span| span.contains(tick))
        })
        .map(|tick| tick..tick + 1);
    let mut compared: Vec<Range<usize>> = spans.chain(single_ticks).collect();
    compared.sort_by_key(|span| span.start);
    let levels: Vec<String> = compared
        .into_iter()
        .filter_map(|span| {
            let span = span.start.min(ticks)..span.end.min(ticks);
            if span.is_empty() {
                return None;
            }
            let expected = average_level_db(&reference.tick_levels_db[span.clone()]);
            let actual = average_level_db(&ours.tick_levels_db[span.clone()]);
            let place = if span.len() == 1 {
                format!("tick {}", span.start)
            } else {
                format!("ticks {}-{} on average", span.start, span.end)
            };
            ((actual - expected).abs() > tolerance.level_db)
                .then(|| format!("{place}: {actual:.2} dB, PICO-8 {expected:.2}"))
        })
        .collect();
    if !levels.is_empty() {
        differences.push(format!(
            "level differs by more than ±{} dB in {} places, first {}",
            tolerance.level_db,
            levels.len(),
            listed(&levels)
        ));
    }

    let window = window(samples, ticks);
    let pitches: Vec<String> = reference
        .window_pitches
        .iter()
        .zip(pitch_windows(&window))
        .enumerate()
        .filter(|(index, _)| {
            let ticks = index * PITCH_WINDOW_TICKS..(index + 1) * PITCH_WINDOW_TICKS;
            !tolerance
                .pitch_skipped_ticks
                .iter()
                .any(|tick| ticks.contains(tick))
        })
        .filter_map(|(index, (expected, samples))| {
            let expected = (*expected)?;
            let actual = pitch_and_clarity(samples).map(|(pitch, _)| pitch);
            let cents = actual.map(|actual| CENTS_PER_PITCH * (actual - expected));
            match cents {
                Some(cents) if cents.abs() <= tolerance.pitch_cents => None,
                _ => Some(format!(
                    "ticks {}-{}: {} cents off pitch {expected}",
                    index * PITCH_WINDOW_TICKS,
                    (index + 1) * PITCH_WINDOW_TICKS,
                    cents.map_or("no tone".to_owned(), |cents| format!("{cents:+.1}"))
                )),
            }
        })
        .collect();
    if !pitches.is_empty() {
        differences.push(format!(
            "pitch differs by more than ±{} cents in {} of {} tonal windows, first {}",
            tolerance.pitch_cents,
            pitches.len(),
            reference.window_pitches.iter().flatten().count(),
            listed(&pitches)
        ));
    }
    differences
}

/// The level of Ticks taken together: the power average of their levels in dB.
fn average_level_db(tick_levels_db: &[f32]) -> f32 {
    let power = tick_levels_db
        .iter()
        .map(|level| 10f32.powf(level / 10.0))
        .sum::<f32>()
        / tick_levels_db.len() as f32;
    10.0 * power.log10()
}

fn listed(items: &[String]) -> String {
    items
        .iter()
        .take(LISTED_DIFFERENCES)
        .cloned()
        .collect::<Vec<_>>()
        .join("; ")
}
