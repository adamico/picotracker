//! Launching with what `config.ron` and `state.ron` held, and keeping `state.ron` up to date.

use super::App;
use super::files::{FileRequest, Waiting};
use super::messages::{LogItem, Report, Severity};
use crate::keymap::{Keyboard, Keymap, RebindProblem};
use crate::settings::{AppState, Config, SettingsRead, read_config, read_state, write_state};
use crate::wording::counted;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// What the Window shell found at launch. A file is `None` when the platform has no folder for
/// it: then defaults apply, and nothing is remembered this session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    pub config_file: Option<PathBuf>,
    pub config: SettingsContents,
    pub state_file: Option<PathBuf>,
    pub state: SettingsContents,
    pub directory: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsContents {
    Missing,
    Bytes(Vec<u8>),
    Unreadable { reason: String },
}

const CONFIG_RON: &str = "CONFIG.RON";
const STATE_RON: &str = "STATE.RON";

/// The name a damaged settings file is set aside under.
fn bad_file(path: &Path) -> PathBuf {
    let mut bad: OsString = path.as_os_str().to_owned();
    bad.push(".bad");
    PathBuf::from(bad)
}

/// Reads a settings file's contents. A missing file reads as the defaults, and bytes that are not
/// text as a damaged file. A file that cannot be read gives the reason.
fn read_contents<T: Default>(
    contents: SettingsContents,
    read: impl FnOnce(&str) -> SettingsRead<T>,
) -> Result<SettingsRead<T>, String> {
    match contents {
        SettingsContents::Missing => Ok(SettingsRead::Loaded(T::default())),
        SettingsContents::Bytes(bytes) => Ok(match String::from_utf8(bytes) {
            Ok(text) => read(&text),
            Err(_) => SettingsRead::Damaged,
        }),
        SettingsContents::Unreadable { reason } => Err(reason),
    }
}

fn cannot_read(name: &str, reason: &str) -> Report {
    let reason = reason.to_uppercase();
    Report::warning(format!(
        "CANNOT READ {name}: {reason}. USING DEFAULTS THIS SESSION"
    ))
}

fn damaged(name: &str) -> Report {
    Report::warning(format!(
        "{name} WAS DAMAGED: RENAMED TO {name}.BAD, USING DEFAULTS"
    ))
}

fn newer(name: &str) -> Report {
    Report::warning(format!(
        "{name} IS FROM A NEWER PIKUTRACKER: USING DEFAULTS THIS SESSION"
    ))
}

fn ignored_keys(problems: &[RebindProblem]) -> Report {
    let items = problems
        .iter()
        .map(|problem| {
            let text = match problem {
                RebindProblem::UnknownAction(name) => {
                    format!("NO ACTION IS CALLED {}", name.to_uppercase())
                }
                RebindProblem::UnreadableKey { action, key } => format!(
                    "CANNOT READ THE KEY {} FOR {}",
                    key.to_uppercase(),
                    action.to_uppercase()
                ),
            };
            LogItem::new(Severity::Warning, text)
        })
        .collect();
    let count = counted(problems.len(), "KEY");
    Report::warning(format!("{CONFIG_RON}: IGNORED {count}")).with_items(items)
}

impl App {
    /// Starts the app with what `config.ron` and `state.ron` held, and opens the Project opened
    /// or saved last. A damaged file is renamed with a `.bad` suffix, and `state.ron` is written
    /// again only once that rename has worked. A file from a newer version, or one that cannot be
    /// read, is left alone and `state.ron` is never written this session. `config.ron` is never
    /// written.
    pub fn launch(&mut self, launch: Launch) {
        self.directory = launch.directory;
        if let Some(config_file) = launch.config_file {
            self.launch_config(config_file, launch.config);
        }
        if let Some(state_file) = launch.state_file {
            self.launch_state(state_file, launch.state);
        }
    }

    fn launch_config(&mut self, config_file: PathBuf, contents: SettingsContents) {
        match read_contents(contents, read_config) {
            Err(reason) => self.report(cannot_read(CONFIG_RON, &reason)),
            Ok(SettingsRead::Loaded(config)) => self.use_config(&config),
            Ok(SettingsRead::Damaged) => {
                self.file_requests.push(FileRequest::Rename {
                    from: config_file.clone(),
                    to: bad_file(&config_file),
                });
                self.config_set_aside = Some(config_file);
                self.report(damaged(CONFIG_RON));
            }
            Ok(SettingsRead::Newer { .. }) => self.report(newer(CONFIG_RON)),
        }
    }

    fn use_config(&mut self, config: &Config) {
        let (keymap, problems) = Keymap::rebound(config.preset, &config.keys);
        self.keyboard = Keyboard::new(keymap);
        if !problems.is_empty() {
            self.report(ignored_keys(&problems));
        }
    }

    fn launch_state(&mut self, state_file: PathBuf, contents: SettingsContents) {
        match read_contents(contents, read_state) {
            Err(reason) => self.report(cannot_read(STATE_RON, &reason)),
            Ok(SettingsRead::Loaded(state)) => {
                self.follow = state.follow;
                self.phrase.set_octave(state.octave);
                self.midi_table = state.midi_table.clone();
                self.state_file = Some(state_file);
                if let Some(last) = state.last_project() {
                    let path = last.to_path_buf();
                    self.file_requests
                        .push(FileRequest::Read { path: path.clone() });
                    self.waiting = Some(Waiting::Reading(path));
                }
                self.app_state = state;
            }
            Ok(SettingsRead::Damaged) => {
                let bad = bad_file(&state_file);
                self.file_requests.push(FileRequest::Rename {
                    from: state_file.clone(),
                    to: bad.clone(),
                });
                self.waiting = Some(Waiting::SettingAside {
                    state_file,
                    bad_file: bad,
                });
                self.report(damaged(STATE_RON));
            }
            Ok(SettingsRead::Newer { .. }) => self.report(newer(STATE_RON)),
        }
    }

    /// A damaged settings file is now set aside. For `state.ron`, a new one can be written.
    pub(super) fn finish_setting_aside(&mut self, renamed: &Path) {
        if self
            .config_set_aside
            .take_if(|config| bad_file(config) == renamed)
            .is_some()
        {
            return;
        }
        match self.waiting.take() {
            Some(Waiting::SettingAside {
                state_file,
                bad_file,
            }) if bad_file == renamed => self.state_file = Some(state_file),
            other => self.waiting = other,
        }
    }

    /// Says that a damaged `config.ron` could not be set aside, if that is what failed.
    pub(super) fn config_not_set_aside(&mut self, path: &Path, reason: &str) -> bool {
        if self
            .config_set_aside
            .take_if(|config| config == path)
            .is_none()
        {
            return false;
        }
        let reason = reason.to_uppercase();
        self.report(Report::failure(format!(
            "CANNOT RENAME {CONFIG_RON} TO {CONFIG_RON}.BAD: {reason}"
        )));
        true
    }

    /// What `state.ron` should hold now.
    fn remembered(&self) -> AppState {
        let mut state = self.app_state.clone();
        state.follow = self.follow;
        state.octave = self.phrase.octave();
        state.midi_table = self.midi_table.clone();
        state
    }

    /// Writes `state.ron` if what it should hold has changed.
    pub(super) fn save_changed_state(&mut self) {
        if self.state_file.is_some() && self.remembered() != self.app_state {
            self.write_state();
        }
    }

    /// Puts a Project file at the front of the recent Projects, and saves them.
    pub(super) fn remember_project(&mut self, path: PathBuf) {
        if self.state_file.is_none() {
            return;
        }
        self.app_state.remember_project(path);
        self.write_state();
    }

    fn write_state(&mut self) {
        let Some(state_file) = self.state_file.clone() else {
            return;
        };
        self.app_state = self.remembered();
        self.file_requests.push(FileRequest::Write {
            path: state_file,
            contents: write_state(&self.app_state),
        });
    }
}
