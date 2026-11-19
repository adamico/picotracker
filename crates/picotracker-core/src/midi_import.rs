//! MIDI Import: analyses a Standard MIDI File and turns the user's choices into a Project and
//! warnings.

mod build;
mod read;
mod table;
mod voices;

pub use build::{GridOption, Imported, Warning, build, compare_grids};
pub use table::{DRUM_ROWS, DrumHit, MidiTable, PROGRAMS, family_name};

use read::{Note, Score};
use std::fmt;

pub const STEPS_PER_BEAT: [u8; 5] = [2, 3, 4, 6, 8];
pub const DEFAULT_STEPS_PER_BEAT: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportError {
    NotMidi,
    SequentialTracks,
    TimecodeTiming,
    NoNotes,
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotMidi => "NOT A STANDARD MIDI FILE",
            Self::SequentialTracks => "FORMAT 2 MIDI FILES ARE NOT SUPPORTED",
            Self::TimecodeTiming => "SMPTE TIMING IS NOT SUPPORTED",
            Self::NoNotes => "THE FILE HAS NO NOTES",
        })
    }
}

impl std::error::Error for ImportError {}

/// One line of notes that a Track can play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice {
    name: String,
    source: usize,
    channel: u8,
    program: Option<u8>,
    notes: Vec<Note>,
}

impl Voice {
    /// The name the dialog and the warnings use.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Which source the voice came from, counted from 0.
    pub const fn source(&self) -> usize {
        self.source
    }

    /// The MIDI channel, numbered from 1.
    pub const fn channel(&self) -> u8 {
        self.channel
    }

    /// The program the voice starts on, or none for drums.
    pub const fn program(&self) -> Option<u8> {
        self.program
    }

    pub fn note_count(&self) -> usize {
        self.notes.len()
    }
}

/// What a file holds, ready for the dialog to show and for `build` to use.
#[derive(Clone, Debug)]
pub struct Analysis {
    score: Score,
    voices: Vec<Voice>,
    unmapped_drums: Vec<u8>,
    unmapped_drum_count: usize,
}

impl Analysis {
    /// Every voice in file order.
    pub fn voices(&self) -> &[Voice] {
        &self.voices
    }

    /// The programs the file's voices play, in order of number.
    pub fn programs(&self) -> Vec<u8> {
        let mut programs: Vec<u8> = self
            .score
            .sources
            .iter()
            .filter(|source| !source.is_drums())
            .flat_map(|source| {
                source
                    .notes
                    .iter()
                    .map(|note| self.score.program_at(source.channel, note.start))
            })
            .collect();
        programs.sort_unstable();
        programs.dedup();
        programs
    }
}

/// Reads a file and works out its voices.
///
/// # Errors
///
/// Fails if the bytes are not a Standard MIDI File that Import can use.
pub fn analyse(bytes: &[u8]) -> Result<Analysis, ImportError> {
    let score = read::read(bytes)?;
    let mut unmapped: Vec<u8> = score
        .sources
        .iter()
        .filter(|source| source.is_drums())
        .flat_map(|source| &source.notes)
        .map(|note| note.key)
        .filter(|key| table::drum_row(*key).is_none())
        .collect();
    let unmapped_drum_count = unmapped.len();
    unmapped.sort_unstable();
    unmapped.dedup();
    let voices = voices::split(&score);
    if voices.is_empty() {
        return Err(ImportError::NoNotes);
    }
    Ok(Analysis {
        score,
        voices,
        unmapped_drums: unmapped,
        unmapped_drum_count,
    })
}

/// What the user decides in the dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choices {
    pub ticked: Vec<bool>,
    pub steps_per_beat: u8,
    pub table: MidiTable,
}

impl Choices {
    /// Every voice ticked, the default Steps per beat and the factory table.
    pub fn defaults_for(analysis: &Analysis) -> Self {
        Self {
            ticked: vec![true; analysis.voices.len()],
            steps_per_beat: DEFAULT_STEPS_PER_BEAT,
            table: MidiTable::default(),
        }
    }

    /// The Track each voice fills, counted from 0: none if unticked or past the 8 Tracks.
    pub fn tracks(&self) -> Vec<Option<usize>> {
        let mut next = 0;
        self.ticked
            .iter()
            .map(|ticked| {
                ticked
                    .then(|| {
                        next += 1;
                        next - 1
                    })
                    .filter(|track| *track < crate::project::TRACKS)
            })
            .collect()
    }
}
