mod support;

use pikutracker_core::cart::{Cart, CartError};
use support::{HEADER, TestCart, TestPattern, TestSfx, png_of};

const HEADER_FILTERS_0A_SPEED_20_LOOP_2_TO_8: &str = "0a200208";
const PITCH_18_INSTRUMENT_3_VOLUME_5_EFFECT_0: &str = "18350";
const SAME_WITH_CUSTOM_BIT_AND_EFFECT_2: &str = "18b52";

fn sfx_line() -> String {
    format!(
        "{HEADER_FILTERS_0A_SPEED_20_LOOP_2_TO_8}{PITCH_18_INSTRUMENT_3_VOLUME_5_EFFECT_0}\
         {SAME_WITH_CUSTOM_BIT_AND_EFFECT_2}{}",
        "00000".repeat(30)
    )
}

fn p8_with(body: &str) -> Vec<u8> {
    format!("{HEADER}{body}").into_bytes()
}

#[test]
fn a_p8_sfx_line_becomes_the_bytes_of_cart_memory() {
    let cart = Cart::read(&p8_with(&format!("__sfx__\n{}\n", sfx_line()))).unwrap();

    let sfx = cart.sfx_bytes(0);
    assert_eq!(sfx[..4], [0xD8, 0x0A, 0xD8, 0xAA]);
    assert_eq!(sfx[4..64], [0; 60]);
    assert_eq!(sfx[64..], [0x0A, 0x20, 0x02, 0x08]);
}

#[test]
fn a_p8_music_line_moves_its_flags_into_bit_7_of_each_channel() {
    let cart = Cart::read(&p8_with("__music__\n05 01024344\n02 05060708\n")).unwrap();

    assert_eq!(cart.pattern_bytes(0), [0x81, 0x02, 0xC3, 0x44]);
    assert_eq!(cart.pattern_bytes(1), [0x05, 0x86, 0x07, 0x08]);
}

#[test]
fn omitted_trailing_rows_read_as_the_default_rows() {
    let cart = Cart::read(&p8_with(&format!(
        "__sfx__\n{}\n__music__\n00 01020304\n",
        sfx_line()
    )))
    .unwrap();

    let mut blank = [0; 68];
    blank[65] = 16;
    assert_eq!(cart.sfx_bytes(1), &blank);
    assert_eq!(cart.sfx_bytes(63), &blank);
    assert_eq!(cart.pattern_bytes(1), [0x41, 0x42, 0x43, 0x44]);
    assert_eq!(cart.pattern_bytes(63), [0x41, 0x42, 0x43, 0x44]);
}

#[test]
fn a_cart_without_music_sections_reads_as_all_default_rows() {
    let cart = Cart::read(&p8_with("__lua__\nprint(1)\n")).unwrap();

    assert_eq!(cart.pattern_bytes(0), [0x41, 0x42, 0x43, 0x44]);
    assert_eq!(cart.sfx_bytes(0)[65], 16);
}

#[test]
fn windows_line_endings_a_bom_and_upper_case_hex_are_accepted() {
    let text = format!(
        "\u{feff}{}__sfx__\r\n{}\r\n__music__\r\n05 0102434A\r\n",
        HEADER.replace('\n', "\r\n"),
        sfx_line().to_uppercase()
    );

    let cart = Cart::read(text.as_bytes()).unwrap();

    assert_eq!(cart.sfx_bytes(0)[..2], [0xD8, 0x0A]);
    assert_eq!(cart.pattern_bytes(0), [0x81, 0x02, 0xC3, 0x4A]);
}

#[test]
fn unknown_sections_are_ignored() {
    let text = "__meta:title__\nnot hex at all\n__music__\n00 01020304\n";

    let cart = Cart::read(&p8_with(text)).unwrap();

    assert_eq!(cart.pattern_bytes(0), [0x01, 0x02, 0x03, 0x04]);
}

#[test]
fn a_file_that_is_not_a_cart_is_refused() {
    assert_eq!(
        Cart::read(b"just some text\n").unwrap_err(),
        CartError::NotACart
    );
}

#[test]
fn a_damaged_music_line_is_refused_with_its_line_number() {
    let error = Cart::read(&p8_with("__music__\n00 0102\n")).unwrap_err();

    assert_eq!(error, CartError::BadLine { line: 4 });
}

#[test]
fn a_damaged_sfx_line_is_refused_with_its_line_number() {
    let error = Cart::read(&p8_with("__sfx__\n0a20zz08\n")).unwrap_err();

    assert_eq!(error, CartError::BadLine { line: 4 });
}

fn generated_cart() -> TestCart {
    let mut sfx = TestSfx::with_pitches(9, &[12, 14, 16]);
    sfx.editor_and_filters = 0x1a;
    sfx.loop_start = 4;
    sfx.loop_end = 12;
    sfx.notes[2].custom = true;
    sfx.notes[2].instrument = 5;
    TestCart {
        sfx: vec![TestSfx::with_pitches(16, &[1]), sfx],
        patterns: vec![
            TestPattern {
                channels: [Some(1), None, Some(0), None],
                loop_start: true,
                loop_back: false,
                stop: false,
            },
            TestPattern {
                channels: [Some(0), Some(1), None, None],
                loop_start: false,
                loop_back: true,
                stop: true,
            },
        ],
    }
}

#[test]
fn a_png_cart_holds_one_byte_in_the_low_bits_of_each_pixel() {
    let mut memory = vec![0; 0x8020];
    memory[0x3100..0x3104].copy_from_slice(&[0x81, 0x42, 0xC3, 0x1B]);
    memory[0x3200] = 0xD8;
    memory[0x3200 + 67] = 0xE4;

    let cart = Cart::read(&png_of(&memory, 160, 205)).unwrap();

    assert_eq!(cart.pattern_bytes(0), [0x81, 0x42, 0xC3, 0x1B]);
    assert_eq!(cart.sfx_bytes(0)[0], 0xD8);
    assert_eq!(cart.sfx_bytes(0)[67], 0xE4);
}

#[test]
fn both_file_formats_read_to_the_same_cart() {
    let generated = generated_cart();

    let from_text = Cart::read(&generated.p8()).unwrap();
    let from_pixels = Cart::read(&generated.png()).unwrap();

    assert_eq!(from_text, from_pixels);
}

#[test]
fn an_image_of_any_other_size_is_refused() {
    let error = Cart::read(&png_of(&[0; 16], 4, 4)).unwrap_err();

    assert_eq!(error, CartError::NotACart);
}

#[test]
fn a_truncated_image_is_refused() {
    let file = generated_cart().png();

    let error = Cart::read(&file[..file.len() / 2]).unwrap_err();

    assert_eq!(error, CartError::NotACart);
}
