//! The Cart codec's write side: claimed SFX and Patterns go back into the original bytes. Every
//! Cart here is generated in test code, and the expected bytes are worked out by hand from the
//! formats in the spec.

mod support;

use pikutracker_core::cart::Cart;
use pikutracker_core::project::Claim;
use support::HEADER;

const WRITTEN_SFX_LINE_START: &str = "0a20020818350";

/// Filters 0x0a, Speed 0x20, loop 2 to 8, and a first note `18350`: pitch 0x18, instrument 3,
/// volume 5, effect 0, which is the word 0x0AD8.
fn sfx_bytes() -> [u8; 68] {
    let mut sfx = [0; 68];
    sfx[..2].copy_from_slice(&[0xD8, 0x0A]);
    sfx[64..].copy_from_slice(&[0x0A, 0x20, 0x02, 0x08]);
    sfx
}

fn written_sfx_line() -> String {
    format!("{WRITTEN_SFX_LINE_START}{}", "00000".repeat(31))
}

fn old_sfx_line(digit: char) -> String {
    format!("00100000{}", digit.to_string().repeat(160))
}

fn claim(sfx: &[u8], patterns: (u8, u8)) -> Claim {
    Claim {
        sfx: sfx.to_vec(),
        patterns: Some(patterns),
    }
}

/// Reads a Cart, puts the claimed values in it and writes it back into the same bytes.
fn write_into(original: &str, claim: &Claim) -> String {
    let mut cart = Cart::read(original.as_bytes()).unwrap();
    for &slot in &claim.sfx {
        cart.set_sfx_bytes(slot, sfx_bytes());
    }
    if let Some((first, last)) = claim.patterns {
        for slot in first..=last {
            cart.set_pattern_bytes(slot, [0x81, 0x02, 0xC3, 0x44]);
        }
    }
    String::from_utf8(cart.write_claimed(original.as_bytes(), claim).unwrap()).unwrap()
}

/// The row PICO-8 leaves out: no notes, Speed 16.
fn default_sfx_line() -> String {
    format!("00100000{}", "00000".repeat(32))
}

#[test]
fn a_claimed_line_past_the_end_of_its_section_is_created_after_default_rows() {
    let first = old_sfx_line('1');
    let original = format!("{HEADER}__sfx__\n{first}\n\n__music__\n00 01424344\n");

    let written = write_into(&original, &claim(&[2], (0, 0)));

    let expected = format!(
        "{HEADER}__sfx__\n{first}\n{}\n{}\n\n__music__\n05 01024344\n",
        default_sfx_line(),
        written_sfx_line()
    );
    assert_eq!(written, expected);
}

#[test]
fn missing_sections_are_created_in_canonical_order() {
    let original = format!("{HEADER}__lua__\nx=1\n__gfx__\n00\n__map__\n00\n__meta:a__\nb\n");

    let written = write_into(&original, &claim(&[0], (0, 0)));

    let expected = format!(
        "{HEADER}__lua__\nx=1\n__gfx__\n00\n__map__\n00\n__sfx__\n{}\n\
         __music__\n05 01024344\n__meta:a__\nb\n",
        written_sfx_line()
    );
    assert_eq!(written, expected);
}

#[test]
fn a_missing_music_section_follows_the_sfx_section() {
    let first = old_sfx_line('1');
    let original = format!("{HEADER}__sfx__\n{first}\n\n__meta:a__\nb\n");

    let written = write_into(&original, &claim(&[0], (0, 0)));

    let expected = format!(
        "{HEADER}__sfx__\n{}\n\n__music__\n05 01024344\n__meta:a__\nb\n",
        written_sfx_line()
    );
    assert_eq!(written, expected);
}

#[test]
fn a_missing_sfx_section_goes_before_an_existing_music_section() {
    let original = format!("{HEADER}__map__\n00\n__music__\n00 01424344\n");

    let written = write_into(&original, &claim(&[0], (0, 0)));

    let expected = format!(
        "{HEADER}__map__\n00\n__sfx__\n{}\n__music__\n05 01024344\n",
        written_sfx_line()
    );
    assert_eq!(written, expected);
}

#[test]
fn a_cart_with_only_a_header_gains_both_sections() {
    let written = write_into(HEADER, &claim(&[0], (0, 0)));

    let expected = format!(
        "{HEADER}__sfx__\n{}\n__music__\n05 01024344\n",
        written_sfx_line()
    );
    assert_eq!(written, expected);
}

#[test]
fn windows_line_endings_are_kept_and_used_for_new_lines() {
    let header = HEADER.replace('\n', "\r\n");
    let original = format!("{header}__music__\r\n00 01424344\r\n__meta:a__\r\nb\r\n");
    let only_patterns = Claim {
        sfx: Vec::new(),
        patterns: Some((0, 1)),
    };

    let written = write_into(&original, &only_patterns);

    let expected =
        format!("{header}__music__\r\n05 01024344\r\n05 01024344\r\n__meta:a__\r\nb\r\n");
    assert_eq!(written, expected);
}

#[test]
fn a_file_without_a_final_newline_stays_without_one() {
    let original = format!("{HEADER}__music__\n00 01424344");
    let only_patterns = Claim {
        sfx: Vec::new(),
        patterns: Some((0, 1)),
    };

    let written = write_into(&original, &only_patterns);

    assert_eq!(
        written,
        format!("{HEADER}__music__\n05 01024344\n05 01024344")
    );
}

#[test]
fn blank_lines_inside_a_section_do_not_shift_the_claimed_row() {
    let (first, second) = (old_sfx_line('1'), old_sfx_line('2'));
    let original = format!("{HEADER}__sfx__\n{first}\n\n{second}\n");
    let only_sfx = Claim {
        sfx: vec![1],
        patterns: None,
    };

    let written = write_into(&original, &only_sfx);

    let expected = format!("{HEADER}__sfx__\n{first}\n\n{}\n", written_sfx_line());
    assert_eq!(written, expected);
}

#[test]
fn a_claimed_default_row_past_the_end_adds_nothing() {
    let original = format!("{HEADER}__lua__\nx=1\n");
    let cart = Cart::read(original.as_bytes()).unwrap();

    let written = cart
        .write_claimed(original.as_bytes(), &claim(&[5], (7, 9)))
        .unwrap();

    assert_eq!(String::from_utf8(written).unwrap(), original);
}

/// The decoded RGBA pixels of an image.
fn pixels_of(file: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(file))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    pixels
}

/// The Cart bytes an image's pixels hold.
fn memory_of(pixels: &[u8]) -> Vec<u8> {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| {
            (pixel[3] & 3) << 6 | (pixel[0] & 3) << 4 | (pixel[1] & 3) << 2 | (pixel[2] & 3)
        })
        .collect()
}

/// An all-zero Cart image, apart from a version block that has to survive.
fn blank_png() -> Vec<u8> {
    let mut memory = vec![0; 0x8020];
    memory[0x8000..0x8006].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
    support::png_of(&memory, 160, 205)
}

fn write_into_png(original: &[u8], claim: &Claim) -> Vec<u8> {
    let mut cart = Cart::read(original).unwrap();
    cart.set_sfx_bytes(0, sfx_bytes());
    cart.set_pattern_bytes(0, [0x81, 0x02, 0xC3, 0x44]);
    cart.write_claimed(original, claim).unwrap()
}

#[test]
fn a_png_cart_gets_the_claimed_bytes_and_a_new_sha1() {
    let written = write_into_png(&blank_png(), &claim(&[0], (0, 0)));

    let memory = memory_of(&pixels_of(&written));
    assert_eq!(memory[0x3100..0x3104], [0x81, 0x02, 0xC3, 0x44]);
    assert_eq!(memory[0x3200..0x3200 + 68], sfx_bytes());
    assert_eq!(memory[0x8000..0x8006], [1, 2, 3, 4, 5, 6]);
    // Worked out with Python's hashlib from the same memory.
    assert_eq!(
        memory[0x8006..0x801A],
        [
            0x4e, 0xad, 0x1c, 0xb9, 0xfd, 0x3c, 0x48, 0x2e, 0x39, 0x1a, 0x7f, 0x3c, 0x18, 0x1f,
            0x60, 0x6b, 0x2f, 0x50, 0x9b, 0xb3
        ]
    );
}

#[test]
fn a_png_cart_keeps_every_other_pixel_and_the_high_bits_of_the_claimed_ones() {
    let original = blank_png();

    let written = write_into_png(&original, &claim(&[0], (0, 0)));

    let (before, after) = (pixels_of(&original), pixels_of(&written));
    assert_eq!(before.len(), after.len());
    let claimed = [0x3100..0x3104, 0x3200..0x3244, 0x8006..0x801A];
    let old_pixels = before.as_chunks::<4>().0.iter();
    for (index, (old, new)) in old_pixels.zip(after.as_chunks::<4>().0).enumerate() {
        if claimed.iter().any(|range| range.contains(&index)) {
            let high_bits = |pixel: &[u8; 4]| pixel.map(|bits| bits & 0xFC);
            assert_eq!(high_bits(old), high_bits(new), "pixel {index}");
        } else {
            assert_eq!(old, new, "pixel {index}");
        }
    }
}

#[test]
fn a_png_cart_keeps_its_other_chunks() {
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, 160, 205);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .add_text_chunk("Comment".to_owned(), "keep me".to_owned())
        .unwrap();
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&vec![0; 160 * 205 * 4]).unwrap();
    writer.finish().unwrap();

    let written = write_into_png(&file, &claim(&[0], (0, 0)));

    let holds = |needle: &[u8]| written.windows(needle.len()).any(|bytes| bytes == needle);
    assert!(holds(b"tEXtComment\0keep me"));
}

#[test]
fn only_the_claimed_lines_of_a_p8_cart_change() {
    let (first, second) = (old_sfx_line('1'), old_sfx_line('2'));
    let original = format!(
        "{HEADER}__lua__\nprint(1)\n__sfx__\n{first}\n{second}\n\
         __music__\n00 01424344\n00 02424344\n__meta:notes__\nkeep me\n"
    );

    let written = write_into(&original, &claim(&[1], (1, 1)));

    let expected = format!(
        "{HEADER}__lua__\nprint(1)\n__sfx__\n{first}\n{}\n\
         __music__\n00 01424344\n05 01024344\n__meta:notes__\nkeep me\n",
        written_sfx_line()
    );
    assert_eq!(written, expected);
}
