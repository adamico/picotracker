//! The file list, drawn in the right-hand slot while a file is opened, saved or Imported.

use super::{HEADING, VALUE, draw_title, stepped};
use crate::app::Direction;
use crate::app::files::DirectoryEntry;
use crate::app::importing::import_kind;
use crate::app::layout::Area;
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;
use std::path::{Component, Path, PathBuf};

pub const PROJECT_EXTENSION: &str = ".ptrk";
const PARENT: &str = "..";
const MAX_NAME: usize = 32;
const SLOT_WIDTH: usize = 22;
const DIRECTORY_ROW: usize = 1;
const FIRST_ROW: usize = 2;
const NAME_LABEL: &str = "NAME ";

const FOLDER: Style = Style::fg(Colour::Blue);
const POINTER: Style = Style::fg(Colour::Yellow);

/// What the file list is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Open,
    SaveAs,
    Import,
}

impl Purpose {
    /// Whether the list shows a file with this name.
    fn lists(self, name: &str) -> bool {
        match self {
            Self::Open | Self::SaveAs => is_project_file(name),
            Self::Import => import_kind(name).is_some(),
        }
    }
}

/// What Enter leads to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Nothing further to do.
    Stay,
    /// Show this folder.
    Folder(PathBuf),
    /// Open this file.
    Open(PathBuf),
    /// Import this file.
    Import(PathBuf),
    /// Save to this file, which may already exist.
    Save { path: PathBuf, exists: bool },
}

#[derive(Debug)]
struct Row {
    name: String,
    is_directory: bool,
}

/// A folder's contents, with a name field when saving.
#[derive(Debug)]
pub struct FileList {
    purpose: Purpose,
    directory: PathBuf,
    rows: Vec<Row>,
    /// The picked row. When saving, row 0 is the name field.
    picked: usize,
    name: String,
}

impl FileList {
    pub fn new(purpose: Purpose, directory: PathBuf, name: String) -> Self {
        Self {
            purpose,
            directory,
            rows: vec![parent_row()],
            picked: 0,
            name,
        }
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    /// Shows what a folder holds: its folders, then the files this list is for, each in name
    /// order.
    pub fn show(&mut self, directory: PathBuf, entries: Vec<DirectoryEntry>) {
        let mut rows: Vec<Row> = entries
            .into_iter()
            .filter(|entry| entry.is_directory || self.purpose.lists(&entry.name))
            .map(|entry| Row {
                name: entry.name,
                is_directory: entry.is_directory,
            })
            .collect();
        rows.sort_by_key(|row| (!row.is_directory, row.name.to_lowercase()));
        rows.insert(0, parent_row());
        self.directory = directory;
        self.rows = rows;
        self.picked = 0;
    }

    /// Rows above the folder's contents: the name field when saving.
    fn name_rows(&self) -> usize {
        usize::from(self.purpose == Purpose::SaveAs)
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (_, rows) = direction.delta();
        self.picked = stepped(self.picked, rows, self.rows.len() + self.name_rows());
    }

    /// Types one character of the file name, if a file name can hold it.
    pub fn type_char(&mut self, ch: char) {
        let allowed = ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.');
        if self.purpose == Purpose::SaveAs && allowed && self.name.len() < MAX_NAME {
            self.name.push(ch);
            self.picked = 0;
        }
    }

    pub fn backspace(&mut self) {
        self.name.pop();
        self.picked = 0;
    }

    /// What Enter does on the picked row.
    pub fn choose(&mut self) -> Choice {
        let Some(index) = self.picked.checked_sub(self.name_rows()) else {
            return self.save_choice();
        };
        let row = &self.rows[index];
        if row.name == PARENT {
            Choice::Folder(parent_of(&self.directory))
        } else if row.is_directory {
            Choice::Folder(self.directory.join(&row.name))
        } else {
            match self.purpose {
                Purpose::Open => Choice::Open(self.directory.join(&row.name)),
                Purpose::Import => Choice::Import(self.directory.join(&row.name)),
                Purpose::SaveAs => {
                    self.name.clone_from(&row.name);
                    self.picked = 0;
                    Choice::Stay
                }
            }
        }
    }

    fn save_choice(&self) -> Choice {
        if self.name.is_empty() {
            return Choice::Stay;
        }
        let file_name = if is_project_file(&self.name) {
            self.name.clone()
        } else {
            format!("{}{PROJECT_EXTENSION}", self.name)
        };
        let exists = self.rows.iter().any(|row| row.name == file_name);
        Choice::Save {
            path: self.directory.join(file_name),
            exists,
        }
    }

    pub fn draw(&self, grid: &mut Grid, area: Area) {
        let title = match self.purpose {
            Purpose::Open => "OPEN PROJECT",
            Purpose::SaveAs => "SAVE PROJECT AS",
            Purpose::Import => "IMPORT FILE",
        };
        draw_title(grid, area.column, 0, title, true);
        let directory = self.directory.to_string_lossy();
        grid.put(
            area.column,
            DIRECTORY_ROW,
            tail(&directory, SLOT_WIDTH + 1),
            HEADING,
        );

        let mut row = FIRST_ROW;
        if self.purpose == Purpose::SaveAs {
            self.draw_pointer(grid, area, row, 0);
            let name = format!("{NAME_LABEL}{}_", self.name);
            grid.put(area.column + 1, row, tail(&name, SLOT_WIDTH), VALUE);
            row += 1;
        }

        let visible = area.rows - row;
        let picked_row = self.picked.saturating_sub(self.name_rows());
        let first = (picked_row + 1).saturating_sub(visible);
        for (index, entry) in self.rows.iter().enumerate().skip(first).take(visible) {
            self.draw_pointer(grid, area, row, index + self.name_rows());
            let (text, style) = if entry.is_directory {
                (format!("{}/", entry.name), FOLDER)
            } else {
                (entry.name.clone(), VALUE)
            };
            grid.put(area.column + 1, row, &text, style);
            row += 1;
        }
    }

    fn draw_pointer(&self, grid: &mut Grid, area: Area, row: usize, index: usize) {
        if self.picked == index {
            grid.put(area.column, row, ">", POINTER);
        }
    }
}

fn parent_row() -> Row {
    Row {
        name: PARENT.to_owned(),
        is_directory: true,
    }
}

fn is_project_file(name: &str) -> bool {
    name.to_lowercase().ends_with(PROJECT_EXTENSION)
}

/// The folder above this one, worked out from the path alone.
fn parent_of(directory: &Path) -> PathBuf {
    match directory.components().next_back() {
        Some(Component::Normal(_)) => directory
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf),
        _ => directory.join(PARENT),
    }
}

/// The end of a text that fits a width, so that the part being typed stays in view.
fn tail(text: &str, width: usize) -> &str {
    let extra = text.chars().count().saturating_sub(width);
    text.char_indices()
        .nth(extra)
        .map_or("", |(at, _)| &text[at..])
}
