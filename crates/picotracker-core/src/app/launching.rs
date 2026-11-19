//! Launching with what `state.ron` held, and keeping the recent Projects in it up to date.

use super::App;
use super::files::{FileRequest, Waiting};
use super::messages::Report;
use crate::settings::{StateRead, read_state, write_state};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// What the Window shell found at launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    /// Where `state.ron` lives, whether or not it exists yet. `None` when the platform has no
    /// folder for it, and then nothing is remembered this session.
    pub state_file: Option<PathBuf>,
    /// What reading `state.ron` gave.
    pub state: StateContents,
    /// The folder the file list starts in until a Project is opened or saved.
    pub directory: PathBuf,
}

/// What reading `state.ron` at launch gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateContents {
    /// There is no file yet.
    Missing,
    /// The file's bytes. Bytes that are not text count as a damaged file.
    Bytes(Vec<u8>),
    /// The file is there but could not be read, so it is left alone and never written this
    /// session.
    Unreadable { reason: String },
}

const DAMAGED_STATE: &str = "STATE.RON WAS DAMAGED: RENAMED TO STATE.RON.BAD, USING DEFAULTS";
const NEWER_STATE: &str = "STATE.RON IS FROM A NEWER PICOTRACKER: USING DEFAULTS THIS SESSION";

impl App {
    /// Starts the app with what `state.ron` held, and opens the Project opened or saved last.
    /// A damaged file is renamed with a `.bad` suffix, and written again only once that rename
    /// has worked. A file from a newer version, or one that cannot be read, is left alone and
    /// never written this session.
    pub fn launch(&mut self, launch: Launch) {
        self.directory = launch.directory;
        let Some(state_file) = launch.state_file else {
            return;
        };
        let read = match launch.state {
            StateContents::Missing => StateRead::Loaded(Default::default()),
            StateContents::Bytes(bytes) => match String::from_utf8(bytes) {
                Ok(text) => read_state(&text),
                Err(_) => StateRead::Damaged,
            },
            StateContents::Unreadable { reason } => {
                let reason = reason.to_uppercase();
                self.report(Report::warning(format!(
                    "CANNOT READ STATE.RON: {reason}. USING DEFAULTS THIS SESSION"
                )));
                return;
            }
        };
        match read {
            StateRead::Loaded(state) => {
                self.app_state = state;
                self.state_file = Some(state_file);
                if let Some(last) = self.app_state.last_project() {
                    let path = last.to_path_buf();
                    self.file_requests
                        .push(FileRequest::Read { path: path.clone() });
                    self.waiting = Some(Waiting::Reading(path));
                }
            }
            StateRead::Damaged => {
                let mut bad: OsString = state_file.as_os_str().to_owned();
                bad.push(".bad");
                let bad = PathBuf::from(bad);
                self.file_requests.push(FileRequest::Rename {
                    from: state_file.clone(),
                    to: bad.clone(),
                });
                self.waiting = Some(Waiting::SettingAside {
                    state_file,
                    bad_file: bad,
                });
                self.report(Report::warning(DAMAGED_STATE));
            }
            StateRead::Newer { .. } => self.report(Report::warning(NEWER_STATE)),
        }
    }

    /// The damaged `state.ron` is now `state.ron.bad`, so a new one can be written.
    pub(super) fn finish_setting_aside(&mut self, renamed: &Path) {
        match self.waiting.take() {
            Some(Waiting::SettingAside {
                state_file,
                bad_file,
            }) if bad_file == renamed => self.state_file = Some(state_file),
            other => self.waiting = other,
        }
    }

    /// Puts a Project file at the front of the recent Projects, and saves them.
    pub(super) fn remember_project(&mut self, path: PathBuf) {
        let Some(state_file) = self.state_file.clone() else {
            return;
        };
        self.app_state.remember_project(path);
        self.file_requests.push(FileRequest::Write {
            path: state_file,
            contents: write_state(&self.app_state),
        });
    }
}
