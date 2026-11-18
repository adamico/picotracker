//! New, Open, Save and Save As. The App core never touches the disk: it asks for files with
//! `FileRequest`s and is given the results as `FileResponse`s.

use super::messages::{Location, LogItem, Report, Severity};
use super::panes::file_list::{Choice, FileList, Purpose};
use super::panes::midi_dialog::{MidiDialog, Outcome};
use super::{Action, App, Focus};
use crate::cart::Cart;
use crate::cart_import;
use crate::midi_import::{self, Warning};
use crate::player::{AudioRequest, SCOPE_COLUMNS, TrackMix};
use crate::project::Project;
use crate::project_file;
use std::path::{Path, PathBuf};

const UNTITLED: &str = "UNTITLED";
const UNSAVED_QUESTION: &str = "UNSAVED CHANGES WILL BE LOST  ENTER DISCARD  ESC CANCEL";

/// Something the app needs done to the file system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileRequest {
    /// Say what a folder holds.
    List { directory: PathBuf },
    /// Read a whole text file.
    Read { path: PathBuf },
    /// Read a whole file that may not be text.
    ReadBytes { path: PathBuf },
    /// Replace a file with these contents.
    Write { path: PathBuf, contents: String },
}

/// One thing a folder holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub is_directory: bool,
}

/// The result of a `FileRequest`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileResponse {
    Listed {
        directory: PathBuf,
        entries: Vec<DirectoryEntry>,
    },
    Read {
        path: PathBuf,
        contents: String,
    },
    BytesRead {
        path: PathBuf,
        contents: Vec<u8>,
    },
    Written {
        path: PathBuf,
    },
    /// A request for this path could not be carried out.
    Failed {
        path: PathBuf,
        reason: String,
    },
}

/// What a yes to the question on the status line does.
#[derive(Debug)]
pub(super) enum Accepted {
    NewProject,
    BrowseToOpen,
    BrowseToImport,
    SaveOver(PathBuf),
    PurgeUnused,
}

/// A question on the status line, waiting for Enter or Esc.
#[derive(Debug)]
pub(super) struct Question {
    pub(super) text: String,
    accepted: Accepted,
}

impl Question {
    /// Asks whether to go ahead with Purge unused.
    pub(super) fn purge_unused(text: String) -> Self {
        Self {
            text,
            accepted: Accepted::PurgeUnused,
        }
    }
}

/// A file request still being carried out.
#[derive(Debug)]
pub(super) enum Waiting {
    Listing(PathBuf),
    Reading(PathBuf),
    Importing(PathBuf),
    /// The Project as it was when the save was asked for.
    Writing(PathBuf, Box<Project>),
}

/// What a file can be Imported as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ImportKind {
    Cart,
    Midi,
}

/// Decides how a file is Imported, from its name. Files of no kind are not listed for Import.
pub(super) fn import_kind(file_name: &str) -> Option<ImportKind> {
    let name = file_name.to_lowercase();
    if name.ends_with(".p8") || name.ends_with(".p8.png") {
        Some(ImportKind::Cart)
    } else if name.ends_with(".mid") || name.ends_with(".midi") {
        Some(ImportKind::Midi)
    } else {
        None
    }
}

/// The file name of a path in capitals, as messages show it.
fn shown_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .to_uppercase()
}

fn folder_of(path: &Path) -> PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// The result of an Import: its warnings, if any, go to the Log.
fn import_report(source: &str, warnings: Vec<LogItem>) -> Report {
    if warnings.is_empty() {
        return Report::info(format!("IMPORTED {source}"));
    }
    let count = warnings.len();
    let plural = if count == 1 { "" } else { "S" };
    Report::warning(format!("IMPORTED {source} WITH {count} WARNING{plural}")).with_items(warnings)
}

/// A MIDI Import warning as a Log item, which jumps to the place it is about.
fn midi_log_item(warning: Warning) -> LogItem {
    let item = LogItem::new(Severity::Warning, warning.text);
    match warning.spot {
        Some(spot) => item.at(Location {
            song_row: spot.song_row,
            track: spot.track,
            chain_row: spot.chain_row,
            step: spot.step,
        }),
        None => item,
    }
}

impl App {
    /// Whether the music differs from what was last saved or opened.
    pub fn has_unsaved_changes(&self) -> bool {
        self.project != self.saved_project
    }

    /// Hands over the file requests made since the last call.
    pub fn take_file_requests(&mut self) -> Vec<FileRequest> {
        std::mem::take(&mut self.file_requests)
    }

    /// Takes in the result of a file request.
    pub fn receive_file(&mut self, response: FileResponse) {
        match response {
            FileResponse::Listed { directory, entries } => {
                if matches!(&self.waiting, Some(Waiting::Listing(listed)) if *listed == directory) {
                    self.waiting = None;
                    if let Some(list) = self.file_list.as_mut() {
                        list.show(directory, entries);
                    }
                }
            }
            FileResponse::Read { path, contents } => self.finish_reading(path, &contents),
            FileResponse::BytesRead { path, contents } => self.finish_importing(path, &contents),
            FileResponse::Written { path } => self.finish_writing(&path),
            FileResponse::Failed { path, reason } => self.file_failed(&path, &reason),
        }
    }

    fn finish_reading(&mut self, path: PathBuf, contents: &str) {
        if !matches!(&self.waiting, Some(Waiting::Reading(read)) if *read == path) {
            return;
        }
        self.waiting = None;
        self.file_list = None;
        let name = shown_name(&path);
        match project_file::parse(contents) {
            Ok(project) => {
                self.directory = folder_of(&path);
                self.replace_project(project, Some(path));
                self.report(Report::info(format!("OPENED {name}")));
            }
            Err(error) => self.report(Report::failure(format!(
                "CANNOT OPEN {name}: {}",
                error.to_string().to_uppercase()
            ))),
        }
    }

    fn finish_importing(&mut self, path: PathBuf, contents: &[u8]) {
        if !matches!(&self.waiting, Some(Waiting::Importing(read)) if *read == path) {
            return;
        }
        self.waiting = None;
        self.file_list = None;
        let file_name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        match import_kind(&file_name) {
            Some(ImportKind::Cart) => self.import_cart(path, &file_name, contents),
            Some(ImportKind::Midi) => self.start_midi_import(path, contents),
            None => {}
        }
    }

    /// Opens the MIDI Import dialog on a file's bytes, or says why the file cannot be Imported.
    pub fn start_midi_import(&mut self, path: impl AsRef<Path>, contents: &[u8]) {
        let path = path.as_ref();
        let name = shown_name(path);
        match midi_import::analyse(contents) {
            Ok(analysis) => {
                let dialog = MidiDialog::new(name, folder_of(path), analysis);
                self.midi_dialog = Some(Box::new(dialog));
            }
            Err(error) => self.report(Report::failure(format!("CANNOT IMPORT {name}: {error}"))),
        }
    }

    /// The MIDI Import dialog takes the keys while it is open.
    pub(super) fn use_midi_dialog(&mut self, action: Action) {
        let Some(dialog) = self.midi_dialog.as_mut() else {
            return;
        };
        match dialog.apply(action) {
            Outcome::Stay => {}
            Outcome::Cancel => self.midi_dialog = None,
            Outcome::Import => {
                if let Some(dialog) = self.midi_dialog.take() {
                    let (name, folder, imported) = dialog.finish();
                    self.directory = folder;
                    self.replace_project(imported.project, None);
                    let items = imported.warnings.into_iter().map(midi_log_item).collect();
                    self.report(import_report(&name, items));
                }
            }
        }
    }

    /// Opens a Cart's first song as a new Project, which has no file until it is saved.
    fn import_cart(&mut self, path: PathBuf, file_name: &str, contents: &[u8]) {
        let name = shown_name(&path);
        let cart = match Cart::read(contents) {
            Ok(cart) => cart,
            Err(error) => {
                self.report(Report::failure(format!("CANNOT IMPORT {name}: {error}")));
                return;
            }
        };
        let imported = cart_import::import(&cart, file_name);
        let has_song = imported
            .project
            .target_cart()
            .is_some_and(|target| target.claim.patterns.is_some());
        let source = if has_song {
            format!("SONG 1 OF {name}")
        } else {
            name
        };
        self.directory = folder_of(&path);
        self.replace_project(imported.project, None);
        let items = imported
            .warnings
            .into_iter()
            .map(|warning| LogItem::new(Severity::Warning, warning))
            .collect();
        self.report(import_report(&source, items));
    }

    fn finish_writing(&mut self, path: &Path) {
        match self.waiting.take() {
            Some(Waiting::Writing(written, project)) if written == path => {
                self.saved_project = *project;
                self.directory = folder_of(&written);
                self.report(Report::info(format!("SAVED {}", shown_name(&written))));
                self.project_path = Some(written);
            }
            other => self.waiting = other,
        }
    }

    fn file_failed(&mut self, path: &Path, reason: &str) {
        let reason = reason.to_uppercase();
        let name = shown_name(path);
        let summary = match self.waiting.take() {
            Some(Waiting::Listing(listed)) if listed == path => {
                format!("CANNOT LIST {}: {reason}", path.display())
            }
            Some(Waiting::Reading(read)) if read == path => {
                self.file_list = None;
                format!("CANNOT OPEN {name}: {reason}")
            }
            Some(Waiting::Importing(read)) if read == path => {
                self.file_list = None;
                format!("CANNOT IMPORT {name}: {reason}")
            }
            Some(Waiting::Writing(written, _)) if written == path => {
                format!("CANNOT SAVE {name}: {reason}")
            }
            other => {
                self.waiting = other;
                return;
            }
        };
        self.report(Report::failure(summary));
    }

    /// Puts another Project in place, opening at the top of the Song with the Song pane focused.
    /// Playback stops, and Mute and Solo do not carry over to the new Project.
    fn replace_project(&mut self, project: Project, path: Option<PathBuf>) {
        if self.playing || self.previewing {
            self.playing = false;
            self.previewing = false;
            self.audio_requests.push(AudioRequest::Stop);
        }
        self.playhead = None;
        self.scope = [0; SCOPE_COLUMNS];
        if self.track_mix != TrackMix::default() {
            self.track_mix = TrackMix::default();
            self.audio_requests
                .push(AudioRequest::SetTrackMix(self.track_mix));
        }
        self.saved_project = project.clone();
        self.project = project;
        self.history.clear();
        self.speed_entry = None;
        self.selection = None;
        self.clipboard = None;
        self.project_path = path;
        self.song = Default::default();
        self.chain = Default::default();
        self.phrase.return_to_top();
        self.overview = Default::default();
        self.focus = Focus::Song;
        self.played_pane = Focus::Song;
        self.log_return = Focus::Song;
    }

    pub(super) fn new_project(&mut self) {
        if self.has_unsaved_changes() {
            self.ask_about_unsaved_changes(Accepted::NewProject);
        } else {
            self.replace_project(Project::new(), None);
        }
    }

    pub(super) fn open_project(&mut self) {
        if self.has_unsaved_changes() {
            self.ask_about_unsaved_changes(Accepted::BrowseToOpen);
        } else {
            self.browse(Purpose::Open);
        }
    }

    pub(super) fn import(&mut self) {
        if self.has_unsaved_changes() {
            self.ask_about_unsaved_changes(Accepted::BrowseToImport);
        } else {
            self.browse(Purpose::Import);
        }
    }

    fn ask_about_unsaved_changes(&mut self, accepted: Accepted) {
        self.question = Some(Question {
            text: UNSAVED_QUESTION.to_owned(),
            accepted,
        });
    }

    pub(super) fn save(&mut self) {
        match self.project_path.clone() {
            Some(path) => self.write_project(path),
            None => self.save_as(),
        }
    }

    pub(super) fn save_as(&mut self) {
        self.browse(Purpose::SaveAs);
    }

    fn browse(&mut self, purpose: Purpose) {
        let name = self
            .project_path
            .as_deref()
            .and_then(Path::file_name)
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let directory = self.directory.clone();
        self.file_list = Some(FileList::new(purpose, directory.clone(), name));
        self.list_folder(directory);
    }

    fn list_folder(&mut self, directory: PathBuf) {
        self.file_requests.push(FileRequest::List {
            directory: directory.clone(),
        });
        self.waiting = Some(Waiting::Listing(directory));
    }

    fn write_project(&mut self, path: PathBuf) {
        self.file_list = None;
        self.file_requests.push(FileRequest::Write {
            path: path.clone(),
            contents: project_file::write(&self.project),
        });
        self.waiting = Some(Waiting::Writing(path, Box::new(self.project.clone())));
    }

    /// Enter says yes to the question and Esc says no. Other keys wait for an answer.
    pub(super) fn answer_question(&mut self, action: Action) {
        match action {
            Action::Cancel => self.question = None,
            Action::Confirm => {
                if let Some(question) = self.question.take() {
                    match question.accepted {
                        Accepted::NewProject => self.replace_project(Project::new(), None),
                        Accepted::BrowseToOpen => self.browse(Purpose::Open),
                        Accepted::BrowseToImport => self.browse(Purpose::Import),
                        Accepted::SaveOver(path) => self.write_project(path),
                        Accepted::PurgeUnused => self.purge_unused(),
                    }
                }
            }
            _ => {}
        }
    }

    /// The file list takes the keys while it is open.
    pub(super) fn use_file_list(&mut self, action: Action) {
        let Some(list) = self.file_list.as_mut() else {
            return;
        };
        match action {
            Action::Move(direction) => list.move_cursor(direction),
            Action::TypeChar(ch) => list.type_char(ch),
            Action::Backspace => list.backspace(),
            Action::Cancel => self.file_list = None,
            Action::Confirm => {
                let choice = list.choose();
                self.follow_choice(choice);
            }
            _ => {}
        }
    }

    fn follow_choice(&mut self, choice: Choice) {
        match choice {
            Choice::Stay => {}
            Choice::Folder(directory) => self.list_folder(directory),
            Choice::Open(path) => {
                self.file_requests
                    .push(FileRequest::Read { path: path.clone() });
                self.waiting = Some(Waiting::Reading(path));
            }
            Choice::Import(path) => {
                self.file_requests
                    .push(FileRequest::ReadBytes { path: path.clone() });
                self.waiting = Some(Waiting::Importing(path));
            }
            Choice::Save { path, exists: true } => {
                self.question = Some(Question {
                    text: format!("{} EXISTS  ENTER OVERWRITE  ESC CANCEL", shown_name(&path)),
                    accepted: Accepted::SaveOver(path),
                });
            }
            Choice::Save {
                path,
                exists: false,
            } => self.write_project(path),
        }
    }

    /// The name of the Project on the status line.
    pub(super) fn project_name(&self) -> String {
        self.project_path
            .as_deref()
            .map_or_else(|| UNTITLED.to_owned(), shown_name)
    }
}
