use super::{EMPTY, HEADING, VALUE, draw_title, stepped};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::cart::Cart;
use crate::cart_import::{self, CartSong, ImportedCart};
use crate::project::{
    CHAIN_ENTRIES, FIRST_CUSTOM_INSTRUMENT, MAX_STEPS, Project, SONG_ROWS, TRACKS,
};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;
use crate::synth::{SAMPLE_RATE, SAMPLES_PER_TICK};
use std::collections::BTreeSet;
use std::path::PathBuf;

const TITLE: &str = "IMPORT SONG";
const FIRST_SONG_ROW: usize = 2;
const ROWS_PER_SONG: usize = 4;
const INSTRUMENTS_ON_FIRST_ROW: usize = 3;
const SECONDS_PER_MINUTE: u32 = 60;
pub const SONG_PICKER_KEYS: &str = "UP/DOWN PICK  P LISTEN  ENTER IMPORT  ESC BACK";

const POINTER: Style = Style::fg(Colour::Yellow);

#[derive(Debug)]
struct Choice {
    song: CartSong,
    imported: ImportedCart,
}

#[derive(Debug)]
pub struct SongPicker {
    name: String,
    folder: PathBuf,
    choices: Vec<Choice>,
    picked: usize,
}

impl SongPicker {
    pub fn new(
        name: String,
        folder: PathBuf,
        cart: &Cart,
        cart_path: &str,
        songs: Vec<CartSong>,
    ) -> Self {
        let choices = songs
            .into_iter()
            .map(|song| Choice {
                song,
                imported: cart_import::import_song(cart, cart_path, Some(song)),
            })
            .collect();
        Self {
            name,
            folder,
            choices,
            picked: 0,
        }
    }

    pub fn project(&self) -> &Project {
        &self.choices[self.picked].imported.project
    }

    pub fn move_cursor(&mut self, direction: Direction) {
        let (_, rows) = direction.delta();
        self.picked = stepped(self.picked, rows, self.choices.len());
    }

    pub fn finish(mut self) -> (String, PathBuf, usize, ImportedCart) {
        let number = self.picked + 1;
        let imported = self.choices.swap_remove(self.picked).imported;
        (self.name, self.folder, number, imported)
    }

    pub fn draw(&self, grid: &mut Grid, area: Area) {
        draw_title(grid, area.column, 0, TITLE, true);
        grid.put(area.column, 1, &self.name, HEADING);
        let visible = (area.rows - FIRST_SONG_ROW) / ROWS_PER_SONG;
        let first = (self.picked + 1).saturating_sub(visible);
        for (offset, index) in (first..self.choices.len()).take(visible).enumerate() {
            let row = FIRST_SONG_ROW + offset * ROWS_PER_SONG;
            self.draw_song(grid, area.column, row, index);
        }
    }

    fn draw_song(&self, grid: &mut Grid, column: usize, row: usize, index: usize) {
        let choice = &self.choices[index];
        if index == self.picked {
            grid.put(column, row, ">", POINTER);
        }
        let (first, last) = choice.song.patterns;
        let patterns = if first == last {
            format!("PATTERN {first:02X}")
        } else {
            format!("PATTERNS {first:02X}-{last:02X}")
        };
        grid.put(
            column + 1,
            row,
            &format!("SONG {} {patterns}", index + 1),
            VALUE,
        );
        let project = &choice.imported.project;
        let sfx = project
            .target_cart()
            .map_or(0, |target| target.claim.sfx.len());
        let ending = if choice.song.loop_row.is_some() {
            "LOOP"
        } else {
            "STOP"
        };
        let summary = format!("  {sfx} SFX  {}  {ending}", length(song_ticks(project)));
        grid.put(column, row + 1, &summary, VALUE);
        let instruments: Vec<String> = played_instruments(project)
            .into_iter()
            .map(|instrument| format!("{instrument:02X}"))
            .collect();
        if instruments.is_empty() {
            grid.put(column, row + 2, "  NO INSTRUMENTS", EMPTY);
            return;
        }
        let split = instruments.len().min(INSTRUMENTS_ON_FIRST_ROW);
        let first_row = format!("  INSTRUMENTS {}", instruments[..split].join(" "));
        grid.put(column, row + 2, &first_row, VALUE);
        if split < instruments.len() {
            let second_row = format!("  {}", instruments[split..].join(" "));
            grid.put(column, row + 3, &second_row, VALUE);
        }
    }
}

fn song_ticks(project: &Project) -> u32 {
    (0..SONG_ROWS)
        .take_while(|&row| !project.song_row_is_empty(row))
        .flat_map(|row| (0..CHAIN_ENTRIES).map(move |k| (row, k)))
        .map(|(row, k)| project.chain_row_ticks(row, k))
        .sum()
}

fn length(ticks: u32) -> String {
    let samples = u64::from(ticks) * u64::from(SAMPLES_PER_TICK);
    let seconds = (samples as f64 / f64::from(SAMPLE_RATE)).round() as u32;
    format!(
        "{}:{:02}",
        seconds / SECONDS_PER_MINUTE,
        seconds % SECONDS_PER_MINUTE
    )
}

fn played_instruments(project: &Project) -> BTreeSet<u8> {
    let mut played = BTreeSet::new();
    for row in (0..SONG_ROWS).take_while(|&row| !project.song_row_is_empty(row)) {
        for track in 0..TRACKS {
            for k in 0..CHAIN_ENTRIES {
                let Some(entry) = project.chain_row_entry(row, track, k) else {
                    continue;
                };
                let phrase = project.phrase(entry.phrase());
                let steps = (0..MAX_STEPS).map(|index| phrase.step(index));
                played.extend(
                    steps
                        .take(phrase.length())
                        .filter(|step| !step.is_silent())
                        .map(|step| step.instrument())
                        .filter(|instrument| *instrument >= FIRST_CUSTOM_INSTRUMENT),
                );
            }
        }
    }
    played
}
