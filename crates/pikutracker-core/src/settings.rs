//! Settings: reads the text of `config.ron`, which the user edits by hand, and reads and writes
//! the text of `state.ron`, everything the app remembers between sessions. The Window shell finds
//! the files and does the reading and writing.

use crate::keymap::Preset;
use crate::midi_import::{DrumHit, MidiTable};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const CONFIG_VERSION: u32 = 1;
pub const STATE_VERSION: u32 = 1;

pub const MAX_RECENT_PROJECTS: usize = 10;
pub const DEFAULT_OCTAVE: u8 = 2;
pub const MAX_OCTAVE: u8 = 4;

/// What the user set in `config.ron`: the key preset and the keys they changed, by action name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub preset: Preset,
    pub keys: BTreeMap<String, String>,
}

/// What the app remembers between sessions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppState {
    recent_projects: Vec<PathBuf>,
    pub follow: bool,
    pub octave: u8,
    pub midi_table: MidiTable,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            recent_projects: Vec::new(),
            follow: false,
            octave: DEFAULT_OCTAVE,
            midi_table: MidiTable::default(),
        }
    }
}

impl AppState {
    /// The Project files last opened or saved, the most recent first.
    pub fn recent_projects(&self) -> &[PathBuf] {
        &self.recent_projects
    }

    /// The Project file opened or saved most recently.
    pub fn last_project(&self) -> Option<&Path> {
        self.recent_projects.first().map(PathBuf::as_path)
    }

    /// Puts a Project file at the front of the recent Projects.
    pub fn remember_project(&mut self, path: PathBuf) {
        self.recent_projects.retain(|recent| *recent != path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsRead<T> {
    Loaded(T),
    Damaged,
    Newer { found: u32 },
}

impl<T> SettingsRead<T> {
    /// What the file holds, turned into something else if it was read.
    fn map<U>(self, change: impl FnOnce(T) -> Option<U>) -> SettingsRead<U> {
        match self {
            SettingsRead::Loaded(value) => {
                change(value).map_or(SettingsRead::Damaged, SettingsRead::Loaded)
            }
            SettingsRead::Damaged => SettingsRead::Damaged,
            SettingsRead::Newer { found } => SettingsRead::Newer { found },
        }
    }
}

/// `config.ron` as stored: the format version, then the settings.
#[derive(Deserialize)]
struct ConfigFile {
    #[serde(rename = "version")]
    _version: u32,
    #[serde(default)]
    preset: Preset,
    #[serde(default)]
    keys: BTreeMap<String, String>,
}

/// `state.ron` as stored: the format version, then the state. The MIDI table holds only the rows
/// that differ from the factory table.
#[derive(Serialize, Deserialize)]
struct StateFile {
    version: u32,
    #[serde(default)]
    follow: bool,
    #[serde(default = "default_octave")]
    octave: u8,
    #[serde(default)]
    midi_programs: BTreeMap<u8, u8>,
    #[serde(default)]
    drum_kit: BTreeMap<usize, DrumHitFile>,
    #[serde(default)]
    recent_projects: Vec<PathBuf>,
}

/// A drum kit row as stored.
#[derive(Serialize, Deserialize)]
struct DrumHitFile {
    instrument: u8,
    note: u8,
    effect: u8,
}

const fn default_octave() -> u8 {
    DEFAULT_OCTAVE
}

/// Only the format version, read before anything else in the file.
#[derive(Deserialize)]
struct VersionOnly {
    version: u32,
}

/// Reads a settings file whose format version is `current` or older.
fn read_versioned<T: DeserializeOwned>(text: &str, current: u32) -> SettingsRead<T> {
    let Ok(VersionOnly { version }) = ron::from_str(text) else {
        return SettingsRead::Damaged;
    };
    if version > current {
        return SettingsRead::Newer { found: version };
    }
    match ron::from_str(text) {
        Ok(file) => SettingsRead::Loaded(file),
        Err(_) => SettingsRead::Damaged,
    }
}

/// Reads the text of `config.ron`.
pub fn read_config(text: &str) -> SettingsRead<Config> {
    read_versioned::<ConfigFile>(text, CONFIG_VERSION).map(|file| {
        Some(Config {
            preset: file.preset,
            keys: file.keys,
        })
    })
}

/// Reads the text of `state.ron`. A value out of its range makes the file damaged.
pub fn read_state(text: &str) -> SettingsRead<AppState> {
    read_versioned::<StateFile>(text, STATE_VERSION).map(state_from)
}

fn state_from(file: StateFile) -> Option<AppState> {
    if file.octave > MAX_OCTAVE {
        return None;
    }
    let mut midi_table = MidiTable::default();
    for (program, instrument) in file.midi_programs {
        midi_table.set_program(program, instrument)?;
    }
    for (row, hit) in file.drum_kit {
        midi_table.set_hit(row, hit.instrument, hit.note, hit.effect)?;
    }
    Some(AppState {
        recent_projects: file.recent_projects,
        follow: file.follow,
        octave: file.octave,
        midi_table,
    })
}

/// Writes the text of `state.ron`.
pub fn write_state(state: &AppState) -> String {
    let table = &state.midi_table;
    let midi_programs = table
        .edited_programs()
        .map(|program| (program, table.instrument(program)))
        .collect();
    let drum_kit = table
        .edited_hits()
        .map(|row| {
            let DrumHit {
                instrument,
                note,
                effect,
                ..
            } = table.hit(row);
            (
                row,
                DrumHitFile {
                    instrument,
                    note,
                    effect,
                },
            )
        })
        .collect();
    let file = StateFile {
        version: STATE_VERSION,
        follow: state.follow,
        octave: state.octave,
        midi_programs,
        drum_kit,
        recent_projects: state.recent_projects.clone(),
    };
    ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::default())
        .expect("the state is plain data that RON can always write")
}
