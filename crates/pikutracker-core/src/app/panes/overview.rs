use super::{EMPTY, HEADING, VALUE, draw_styled_value, draw_title, stepped};
use crate::app::Direction;
use crate::app::layout::Area;
use crate::player::{SCOPE_COLUMNS, SCOPE_RANGE};
use crate::project::{EXPORT_CHANNELS, Project, TRACKS};
use crate::screen::grid::{Grid, Style};
use crate::screen::palette::Colour;

const TRACE: Style = Style::fg(Colour::Green);

const CHANNELS: usize = EXPORT_CHANNELS;
const FIRST_CHANNEL_ROW: usize = 2;
const CHANNEL_VALUE_COLUMN: usize = 5;
const SCOPE_ROW: usize = 7;

#[derive(Debug, Default)]
pub struct OverviewPane {
    channel: usize,
}

impl OverviewPane {
    pub fn move_cursor(&mut self, direction: Direction) {
        let (_, rows) = direction.column_and_row_delta();
        self.channel = stepped(self.channel, rows, CHANNELS);
    }

    pub fn channel(&self) -> usize {
        self.channel
    }

    pub fn return_to(&mut self, channel: usize) {
        self.channel = channel;
    }

    pub fn type_digit(&self, project: &mut Project, digit: u8) {
        if !(1..=TRACKS as u8).contains(&digit) {
            return;
        }
        let tracks = project.export_track_numbers();
        if let Some(other) = tracks.iter().position(|track| *track == Some(digit)) {
            project.set_export_track(other, tracks[self.channel]);
        }
        project.set_export_track(self.channel, Some(digit));
    }

    pub fn clear(&self, project: &mut Project) {
        project.set_export_track(self.channel, None);
    }

    pub fn draw(
        &self,
        grid: &mut Grid,
        area: Area,
        focused: bool,
        export_tracks: [Option<u8>; EXPORT_CHANNELS],
        scope: &[i8; SCOPE_COLUMNS],
    ) {
        draw_title(grid, area.column, 0, "OVERVIEW", focused);
        grid.put(area.column, 1, "EXPORT TRACKS", HEADING);

        for (channel, track) in export_tracks.into_iter().enumerate() {
            let row = FIRST_CHANNEL_ROW + channel;
            grid.put(area.column, row, &format!("CH{channel}"), HEADING);
            let is_cursor = focused && channel == self.channel;
            let (text, style) = track.map_or(("--".to_owned(), EMPTY), |track| {
                (format!("T{track}"), VALUE)
            });
            draw_styled_value(
                grid,
                area.column + CHANNEL_VALUE_COLUMN,
                row,
                &text,
                is_cursor,
                style,
            );
        }

        draw_title(grid, area.column, SCOPE_ROW, "SCOPE", false);
        draw_scope(grid, area, scope);
    }
}

fn draw_scope(grid: &mut Grid, area: Area, scope: &[i8; SCOPE_COLUMNS]) {
    if scope.iter().all(|value| *value == 0) {
        return;
    }
    let first_row = SCOPE_ROW + 1;
    let half_height = (area.rows - first_row - 1) as i32 / 2;
    let middle = first_row as i32 + half_height;
    for (column, value) in scope.iter().enumerate() {
        let offset = i32::from(*value) * half_height / i32::from(SCOPE_RANGE);
        grid.put(area.column + column, (middle - offset) as usize, "*", TRACE);
    }
}
