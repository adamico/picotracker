//! Settings: reads and writes the text of `state.ron`, everything the app remembers between
//! sessions. The Window shell finds the file and does the reading and writing.
//!
//! So far it holds the recent Projects. Follow, the octave and changed MIDI table rows join it
//! later, as fields that read as their defaults when a file lacks them.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const STATE_VERSION: u32 = 1;

pub const MAX_RECENT_PROJECTS: usize = 10;

/// What the app remembers between sessions.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppState {
    recent_projects: Vec<PathBuf>,
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
pub enum StateRead {
    Loaded(AppState),
    Damaged,
    Newer { found: u32 },
}

/// The file as stored: the format version, then the state.
#[derive(Serialize, Deserialize)]
struct StateFile {
    version: u32,
    #[serde(default)]
    recent_projects: Vec<PathBuf>,
}

/// Only the format version, read before anything else in the file.
#[derive(Deserialize)]
struct VersionOnly {
    version: u32,
}

/// Reads the text of `state.ron`.
pub fn read_state(text: &str) -> StateRead {
    let Ok(VersionOnly { version }) = ron::from_str(text) else {
        return StateRead::Damaged;
    };
    if version > STATE_VERSION {
        return StateRead::Newer { found: version };
    }
    match ron::from_str::<StateFile>(text) {
        Ok(file) => StateRead::Loaded(AppState {
            recent_projects: file.recent_projects,
        }),
        Err(_) => StateRead::Damaged,
    }
}

/// Writes the text of `state.ron`.
pub fn write_state(state: &AppState) -> String {
    let file = StateFile {
        version: STATE_VERSION,
        recent_projects: state.recent_projects.clone(),
    };
    ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::default())
        .expect("the state is plain data that RON can always write")
}
