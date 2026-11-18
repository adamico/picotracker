//! Cart Import and MIDI Import: turns the bytes of a chosen file into a new Project and reports
//! what could not be kept.

use super::files::{folder_of, shown_name};
use super::messages::{LogItem, Report, Severity};
use super::panes::midi_dialog::{MidiDialog, Outcome};
use super::{Action, App};
use crate::cart::Cart;
use crate::cart_import;
use crate::midi_import;
use crate::project::SongPlace;
use std::path::{Path, PathBuf};

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

/// An Import warning as a Log item, which jumps to the place it is about.
fn warning_item(text: String, place: Option<SongPlace>) -> LogItem {
    let item = LogItem::new(Severity::Warning, text);
    match place {
        Some(place) => item.at(place),
        None => item,
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

impl App {
    pub(super) fn finish_importing(&mut self, path: PathBuf, contents: &[u8]) {
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
                    let warnings = imported.warnings.into_iter();
                    let items = warnings.map(|w| warning_item(w.text, w.place)).collect();
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
        let warnings = imported.warnings.into_iter();
        let items = warnings.map(|w| warning_item(w.text, w.place)).collect();
        self.report(import_report(&source, items));
    }
}
