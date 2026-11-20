use super::files::{folder_of, shown_name};
use super::focus::Focus;
use super::history::CursorPlace;
use super::messages::{LogItem, Report, Severity};
use super::panes::midi_dialog::{MidiDialog, Outcome};
use super::panes::song_picker::SongPicker;
use super::{Action, App};
use crate::cart::Cart;
use crate::cart_import::{self, ImportedCart};
use crate::midi_import;
use crate::player::AudioRequest;
use crate::project::SongPlace;
use crate::wording::counted;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ImportKind {
    Cart,
    Midi,
}

#[derive(Debug)]
pub(super) struct PickingSong {
    pub(super) picker: SongPicker,
    pub(super) cursor_before: CursorPlace,
}

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

fn warning_item(text: String, place: Option<SongPlace>) -> LogItem {
    let item = LogItem::new(Severity::Warning, text);
    match place {
        Some(place) => item.at(place),
        None => item,
    }
}

fn song_source(number: usize, cart_name: &str) -> String {
    format!("SONG {number} OF {cart_name}")
}

fn import_report(source: &str, warnings: Vec<LogItem>) -> Report {
    if warnings.is_empty() {
        return Report::info(format!("IMPORTED {source}"));
    }
    let count = counted(warnings.len(), "WARNING");
    Report::warning(format!("IMPORTED {source} WITH {count}")).with_items(warnings)
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

    pub fn start_midi_import(&mut self, path: impl AsRef<Path>, contents: &[u8]) {
        let path = path.as_ref();
        let name = shown_name(path);
        match midi_import::analyse(contents) {
            Ok(analysis) => {
                let table = self.midi_table.clone();
                let dialog = MidiDialog::new(name, folder_of(path), analysis, table);
                self.midi_dialog = Some(Box::new(dialog));
            }
            Err(error) => self.report(Report::failure(format!("CANNOT IMPORT {name}: {error}"))),
        }
    }

    pub(super) fn use_midi_dialog(&mut self, action: Action) {
        let Some(dialog) = self.midi_dialog.as_mut() else {
            return;
        };
        match dialog.apply(action) {
            Outcome::Stay => {}
            Outcome::Cancel => self.midi_dialog = None,
            Outcome::Import => {
                if let Some(dialog) = self.midi_dialog.take() {
                    let (name, folder, imported, table) = dialog.finish();
                    self.midi_table = table;
                    self.directory = folder;
                    self.replace_project(imported.project, None);
                    let warnings = imported.warnings.into_iter();
                    let items = warnings.map(|w| warning_item(w.text, w.place)).collect();
                    self.report(import_report(&name, items));
                }
            }
        }
    }

    fn import_cart(&mut self, path: PathBuf, file_name: &str, contents: &[u8]) {
        let name = shown_name(&path);
        let cart = match Cart::read(contents) {
            Ok(cart) => cart,
            Err(error) => {
                self.report(Report::failure(format!("CANNOT IMPORT {name}: {error}")));
                return;
            }
        };
        let songs = cart_import::songs(&cart);
        let folder = folder_of(&path);
        if songs.len() > 1 {
            let picker = SongPicker::new(name, folder, &cart, file_name, songs);
            self.open_song_picker(picker);
            return;
        }
        let song = songs.first().copied();
        let source = match song {
            Some(_) => song_source(1, &name),
            None => name,
        };
        let imported = cart_import::import_song(&cart, file_name, song);
        self.finish_cart_import(folder, &source, imported);
    }

    fn finish_cart_import(&mut self, folder: PathBuf, source: &str, imported: ImportedCart) {
        self.directory = folder;
        self.replace_project(imported.project, None);
        let warnings = imported.warnings.into_iter();
        let items = warnings.map(|w| warning_item(w.text, w.place)).collect();
        self.report(import_report(source, items));
    }

    fn open_song_picker(&mut self, picker: SongPicker) {
        let cursor_before = self.cursor_place();
        self.selection_start = None;
        self.song = Default::default();
        self.chain = Default::default();
        self.phrase.return_to_top();
        self.focus = Focus::Song;
        self.played_pane = Focus::Song;
        self.picking_song = Some(Box::new(PickingSong {
            picker,
            cursor_before,
        }));
    }

    pub(super) fn use_song_picker(&mut self, action: Action) {
        let Some(picking) = self.picking_song.as_mut() else {
            return;
        };
        match action {
            Action::Move(direction) => picking.picker.move_cursor(direction),
            Action::PlayFocusedPane if self.playing => self.play_or_stop(),
            Action::PlayFocusedPane => {
                let project = Arc::new(picking.picker.project().clone());
                self.start_playing(AudioRequest::PlaySong {
                    project,
                    from_row: 0,
                });
            }
            Action::Confirm => {
                if let Some(picking) = self.picking_song.take() {
                    let (name, folder, number, imported) = picking.picker.finish();
                    let source = song_source(number, &name);
                    self.finish_cart_import(folder, &source, imported);
                }
            }
            Action::Cancel => {
                if self.playing {
                    self.play_or_stop();
                }
                if let Some(picking) = self.picking_song.take() {
                    self.return_to(picking.cursor_before);
                }
            }
            _ => {}
        }
    }
}
