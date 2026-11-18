//! Carts generated in test code. None of these were saved by PICO-8: they follow the formats the
//! spec describes, and only stand in until Carts re-saved through real PICO-8 exist.
#![allow(dead_code)]

use std::fmt::Write;

pub const HEADER: &str = "pico-8 cartridge // http://www.pico-8.com\nversion 43\n";

/// The byte offsets of the music data in a `.p8.png` Cart's memory.
pub const PATTERNS_AT: usize = 0x3100;
pub const SFX_AT: usize = 0x3200;
const CART_BYTES: usize = 0x8020;
const PNG_WIDTH: u32 = 160;
const PNG_HEIGHT: u32 = 205;

/// One note written the way a `__sfx__` line holds it.
#[derive(Clone, Copy, Debug)]
pub struct TestNote {
    pub pitch: u8,
    pub instrument: u8,
    pub volume: u8,
    pub effect: u8,
    pub custom: bool,
}

/// An audible note on the triangle at a moderate volume.
pub const fn note(pitch: u8) -> TestNote {
    TestNote {
        pitch,
        instrument: 0,
        volume: 3,
        effect: 0,
        custom: false,
    }
}

const REST: TestNote = TestNote {
    pitch: 0,
    instrument: 0,
    volume: 0,
    effect: 0,
    custom: false,
};

/// One SFX of a generated Cart.
#[derive(Clone, Copy, Debug)]
pub struct TestSfx {
    pub editor_and_filters: u8,
    pub speed: u8,
    pub loop_start: u8,
    pub loop_end: u8,
    pub notes: [TestNote; 32],
}

impl TestSfx {
    /// An SFX at this Speed whose first notes are these pitches, and silent after them.
    pub fn with_pitches(speed: u8, pitches: &[u8]) -> Self {
        let mut notes = [REST; 32];
        for (slot, pitch) in notes.iter_mut().zip(pitches) {
            *slot = note(*pitch);
        }
        Self {
            editor_and_filters: 0,
            speed,
            loop_start: 0,
            loop_end: 0,
            notes,
        }
    }

    /// The SFX's 168-digit `__sfx__` line.
    pub fn line(&self) -> String {
        let mut line = format!(
            "{:02x}{:02x}{:02x}{:02x}",
            self.editor_and_filters, self.speed, self.loop_start, self.loop_end
        );
        for note in &self.notes {
            let instrument = note.instrument + if note.custom { 8 } else { 0 };
            write!(
                line,
                "{:02x}{:x}{:x}{:x}",
                note.pitch, instrument, note.volume, note.effect
            )
            .unwrap();
        }
        line
    }

    /// The SFX's 68 bytes in Cart memory.
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(68);
        for note in &self.notes {
            let word = u16::from(note.pitch)
                | u16::from(note.instrument) << 6
                | u16::from(note.volume) << 9
                | u16::from(note.effect) << 12
                | u16::from(note.custom) << 15;
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend([
            self.editor_and_filters,
            self.speed,
            self.loop_start,
            self.loop_end,
        ]);
        bytes
    }
}

/// One Pattern of a generated Cart.
#[derive(Clone, Copy, Debug, Default)]
pub struct TestPattern {
    pub channels: [Option<u8>; 4],
    pub loop_start: bool,
    pub loop_back: bool,
    pub stop: bool,
}

impl TestPattern {
    pub const fn playing(channels: [Option<u8>; 4]) -> Self {
        Self {
            channels,
            loop_start: false,
            loop_back: false,
            stop: false,
        }
    }

    /// The Pattern's four bytes in Cart memory.
    pub fn bytes(&self) -> [u8; 4] {
        let flags = [self.loop_start, self.loop_back, self.stop, false];
        let mut bytes = [0; 4];
        for (channel, byte) in bytes.iter_mut().enumerate() {
            let sfx = self.channels[channel].unwrap_or(0x41 + channel as u8);
            *byte = sfx | u8::from(flags[channel]) << 7;
        }
        bytes
    }

    /// The Pattern's `__music__` line.
    pub fn line(&self) -> String {
        let flags = u8::from(self.loop_start)
            | u8::from(self.loop_back) << 1
            | u8::from(self.stop) << 2;
        let [a, b, c, d] = self.bytes().map(|byte| byte & 0x7f);
        format!("{flags:02x} {a:02x}{b:02x}{c:02x}{d:02x}")
    }
}

/// A generated Cart: SFX and Patterns from slot 0, with the rest left out.
#[derive(Clone, Debug, Default)]
pub struct TestCart {
    pub sfx: Vec<TestSfx>,
    pub patterns: Vec<TestPattern>,
}

impl TestCart {
    /// The Cart as `.p8` text, with the trailing default rows left out.
    pub fn p8(&self) -> Vec<u8> {
        let mut text = format!("{HEADER}__lua__\nprint(\"hi\")\n");
        if !self.sfx.is_empty() {
            text.push_str("__sfx__\n");
            for sfx in &self.sfx {
                text.push_str(&sfx.line());
                text.push('\n');
            }
        }
        if !self.patterns.is_empty() {
            text.push_str("__music__\n");
            for pattern in &self.patterns {
                text.push_str(&pattern.line());
                text.push('\n');
            }
        }
        text.into_bytes()
    }

    /// The Cart's whole memory, with default rows where nothing was written.
    pub fn memory(&self) -> Vec<u8> {
        let mut memory = vec![0; CART_BYTES];
        let blank = TestSfx::with_pitches(16, &[]);
        for slot in 0..64 {
            let sfx = self.sfx.get(slot).unwrap_or(&blank);
            memory[SFX_AT + slot * 68..SFX_AT + (slot + 1) * 68].copy_from_slice(&sfx.bytes());
            let pattern = self.patterns.get(slot).copied().unwrap_or_default();
            memory[PATTERNS_AT + slot * 4..PATTERNS_AT + (slot + 1) * 4]
                .copy_from_slice(&pattern.bytes());
        }
        memory
    }

    /// The Cart as a `.p8.png` image.
    pub fn png(&self) -> Vec<u8> {
        png_of(&self.memory(), PNG_WIDTH, PNG_HEIGHT)
    }
}

/// Packs bytes into the low 2 bits of each pixel's channels, over a mid-grey picture.
pub fn png_of(memory: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for index in 0..(width * height) as usize {
        let byte = memory.get(index).copied().unwrap_or(0);
        pixels.extend([
            0x80 | (byte >> 4 & 3),
            0x40 | (byte >> 2 & 3),
            0xc0 | (byte & 3),
            0xfc | (byte >> 6),
        ]);
    }
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    file
}
