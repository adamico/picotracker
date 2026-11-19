//! Launching with what `state.ron` held, and keeping the recent Projects in it up to date.

use super::App;
use super::files::{FileRequest, Waiting};
use super::messages::Report;
use crate::settings::{StateRead, read_state, write_state};
use std::ffi::OsString;
use std::path::PathBuf;

/// What the Window shell found at launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    /// Where `state.ron` lives, whether or not it exists yet.
    pub state_file: PathBuf,
    /// The text of `state.ron`, if it exists.
    pub state: Option<String>,
    /// The folder the file list starts in until a Project is opened or saved.
    pub directory: PathBuf,
}

const DAMAGED_STATE: &str = "STATE.RON WAS DAMAGED: RENAMED TO STATE.RON.BAD, USING DEFAULTS";
const NEWER_STATE: &str = "STATE.RON IS FROM A NEWER PICOTRACKER: USING DEFAULTS THIS SESSION";

impl App {
    /// Starts the app with what `state.ron` held, and opens the Project opened or saved last.
    /// A damaged file is renamed with a `.bad` suffix. A file from a newer version is left alone
    /// and never written this session.
    pub fn launch(&mut self, launch: Launch) {
        self.directory = launch.directory;
        let read = match launch.state {
            Some(text) => read_state(&text),
            None => StateRead::Loaded(Default::default()),
        };
        match read {
            StateRead::Loaded(state) => {
                self.app_state = state;
                self.state_file = Some(launch.state_file);
                if let Some(last) = self.app_state.last_project() {
                    let path = last.to_path_buf();
                    self.file_requests
                        .push(FileRequest::Read { path: path.clone() });
                    self.waiting = Some(Waiting::Reading(path));
                }
            }
            StateRead::Damaged => {
                let mut bad: OsString = launch.state_file.as_os_str().to_owned();
                bad.push(".bad");
                self.file_requests.push(FileRequest::Rename {
                    from: launch.state_file.clone(),
                    to: PathBuf::from(bad),
                });
                self.state_file = Some(launch.state_file);
                self.report(Report::warning(DAMAGED_STATE));
            }
            StateRead::Newer { .. } => self.report(Report::warning(NEWER_STATE)),
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
