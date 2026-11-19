//! The pixels the character grid turns into, and how they scale to fit a window.

use picotracker_core::screen::frame::{Frame, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use picotracker_core::screen::grid::{GRID_COLUMNS, GRID_ROWS, Grid, Style};
use picotracker_core::screen::palette::Colour;
use picotracker_core::screen::viewport::integer_scale;

const SNAPSHOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/snapshots/font_and_palette.png"
);

const PICO8_PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],
    [29, 43, 83],
    [126, 37, 83],
    [0, 135, 81],
    [171, 82, 54],
    [95, 87, 79],
    [194, 195, 199],
    [255, 241, 232],
    [255, 0, 77],
    [255, 163, 0],
    [255, 236, 39],
    [0, 228, 54],
    [41, 173, 255],
    [131, 118, 156],
    [255, 119, 168],
    [255, 204, 170],
];

fn pixel(frame: &Frame, x: usize, y: usize) -> [u8; 4] {
    let offset = (y * LOGICAL_WIDTH as usize + x) * 4;
    frame.rgba()[offset..offset + 4].try_into().unwrap()
}

/// Every glyph in every foreground colour, and every colour as a background.
fn font_and_palette_grid() -> Grid {
    let mut grid = Grid::new();
    let glyphs: Vec<char> = (' '..='_').chain(['|', '~', '│']).collect();
    for (index, colour) in Colour::ALL.into_iter().enumerate() {
        let line: String = glyphs.iter().collect();
        grid.put(0, index, &line, Style::fg(colour));
        let background = Style {
            fg: Colour::Black,
            bg: colour,
        };
        grid.put(0, 20 + index, "PICOTRACKER 0123456789", background);
    }
    grid
}

#[test]
fn the_palette_is_pico8s() {
    let colours: Vec<[u8; 4]> = Colour::ALL.into_iter().map(Colour::rgba).collect();
    let expected: Vec<[u8; 4]> = PICO8_PALETTE
        .into_iter()
        .map(|[r, g, b]| [r, g, b, 255])
        .collect();

    assert_eq!(colours, expected);
}

#[test]
fn the_frame_is_640_by_360_and_holds_106_by_45_cells_of_6_by_8() {
    assert_eq!((LOGICAL_WIDTH, LOGICAL_HEIGHT), (640, 360));
    assert_eq!((GRID_COLUMNS, GRID_ROWS), (106, 45));
    assert_eq!(Frame::from_grid(&Grid::new()).rgba().len(), 640 * 360 * 4);
}

#[test]
fn a_glyph_is_drawn_5_by_7_from_the_top_left_of_its_cell() {
    let mut grid = Grid::new();
    grid.put(2, 1, "A", Style::fg(Colour::White));
    let frame = Frame::from_grid(&grid);

    // The cell at column 2, row 1 starts at pixel (12, 8).
    let expected = [
        ".###..", "#...#.", "#...#.", "#####.", "#...#.", "#...#.", "#...#.", "......",
    ];
    for (y, row) in expected.iter().enumerate() {
        for (x, mark) in row.chars().enumerate() {
            let want = if mark == '#' {
                Colour::White.rgba()
            } else {
                Colour::Black.rgba()
            };
            assert_eq!(pixel(&frame, 12 + x, 8 + y), want, "pixel {x},{y}");
        }
    }
}

#[test]
fn a_background_fills_the_whole_cell() {
    let mut grid = Grid::new();
    let style = Style {
        fg: Colour::White,
        bg: Colour::Red,
    };
    grid.put(0, 0, " ", style);
    let frame = Frame::from_grid(&grid);

    assert_eq!(pixel(&frame, 5, 7), Colour::Red.rgba());
    assert_eq!(pixel(&frame, 6, 7), Colour::Black.rgba());
    assert_eq!(pixel(&frame, 5, 8), Colour::Black.rgba());
}

#[test]
fn a_separator_is_an_unbroken_line_down_the_cell() {
    let mut grid = Grid::new();
    grid.put(0, 0, "│", Style::fg(Colour::White));
    let frame = Frame::from_grid(&grid);

    for y in 0..8 {
        assert_eq!(pixel(&frame, 2, y), Colour::White.rgba(), "row {y}");
    }
}

#[test]
fn the_font_and_palette_match_the_pixel_snapshot() {
    let frame = Frame::from_grid(&font_and_palette_grid());

    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let file = std::fs::File::create(SNAPSHOT).unwrap();
        let mut encoder = png::Encoder::new(file, LOGICAL_WIDTH, LOGICAL_HEIGHT);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(frame.rgba()).unwrap();
    }

    let file = std::io::BufReader::new(std::fs::File::open(SNAPSHOT).unwrap());
    let mut reader = png::Decoder::new(file).read_info().unwrap();
    let mut snapshot = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut snapshot).unwrap();

    assert!(snapshot == frame.rgba(), "pixels differ from {SNAPSHOT}");
}

#[test]
fn the_window_scales_by_the_largest_whole_number_that_fits() {
    let decided = [
        (1366, 768, 2),
        (1920, 1080, 3),
        (2560, 1440, 4),
        (3840, 2160, 6),
    ];

    for (width, height, scale) in decided {
        assert_eq!(integer_scale(width, height), scale, "{width}x{height}");
    }
}

#[test]
fn a_window_too_small_for_times_two_falls_back_to_times_one() {
    for (width, height) in [(960, 1050), (1279, 720), (640, 360), (320, 200), (0, 0)] {
        assert_eq!(integer_scale(width, height), 1, "{width}x{height}");
    }
}
