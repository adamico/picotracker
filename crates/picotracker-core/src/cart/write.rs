//! Puts a Cart's claimed SFX and Patterns back into the bytes of the file they came from, leaving
//! every other byte as it was.

use super::{
    BOM, Cart, CartError, MUSIC_SECTION, NOTE_DIGITS, PATTERN_BYTES, PATTERNS_AT, PNG_CHANNELS,
    PNG_HEIGHT, PNG_SIGNATURE, PNG_WIDTH, SFX_AT, SFX_BYTES, SFX_NOTES, SFX_SECTION, SLOTS,
    Section, TEXT_HEADER, is_section_mark, pixel_byte, png_pixels,
};
use crate::project::Claim;
use sha1::{Digest, Sha1};
use std::fmt::Write as _;
use std::ops::Range;

/// The SHA-1 of a `.p8.png` Cart covers the bytes before this address.
const SHA1_COVERS: usize = 0x8000;
const SHA1_AT: usize = 0x8006;
/// A PNG chunk's length and type before its data, and its checksum after.
const CHUNK_HEAD: usize = 8;
const CHUNK_TAIL: usize = 4;
const IMAGE_HEADER: &[u8] = b"IHDR";
const IMAGE_DATA: &[u8] = b"IDAT";
const IMAGE_END: &[u8] = b"IEND";

/// The sections that come before `__sfx__` in a Cart, nearest first.
const SECTIONS_BEFORE_SFX: [&[u8]; 5] =
    [b"__map__", b"__gff__", b"__label__", b"__gfx__", b"__lua__"];

/// A range of the original bytes and the text that takes its place.
type Edit = (Range<usize>, String);

impl Cart {
    /// The bytes of a Cart file with the SFX and Patterns of a Claim replaced by this Cart's.
    ///
    /// # Panics
    ///
    /// Panics if the Claim names a slot of 64 or more.
    pub fn write_claimed(&self, original: &[u8], claim: &Claim) -> Result<Vec<u8>, CartError> {
        if original.starts_with(&PNG_SIGNATURE) {
            self.write_png(original, claim)
        } else {
            self.write_text(original, claim)
        }
    }

    /// Rewrites the claimed bytes and the SHA-1 in the low bits of the original pixels. Code is
    /// not recompressed, and the picture the high bits hold stays as it was.
    fn write_png(&self, original: &[u8], claim: &Claim) -> Result<Vec<u8>, CartError> {
        let mut pixels = png_pixels(original).ok_or(CartError::NotACart)?;
        for &slot in &claim.sfx {
            let at = SFX_AT + usize::from(slot) * SFX_BYTES;
            put_bytes(&mut pixels, at, self.sfx_bytes(slot));
        }
        if let Some((first, last)) = claim.patterns {
            for slot in first..=last {
                let at = PATTERNS_AT + usize::from(slot) * PATTERN_BYTES;
                put_bytes(&mut pixels, at, &self.pattern_bytes(slot));
            }
        }
        let memory: Vec<u8> = pixels
            .as_chunks::<PNG_CHANNELS>()
            .0
            .iter()
            .take(SHA1_COVERS)
            .map(|pixel| pixel_byte(*pixel))
            .collect();
        put_bytes(&mut pixels, SHA1_AT, Sha1::digest(&memory).as_slice());
        let encoded = encode_png(&pixels).ok_or(CartError::NotACart)?;
        spliced_png(original, &encoded).ok_or(CartError::NotACart)
    }

    fn write_text(&self, original: &[u8], claim: &Claim) -> Result<Vec<u8>, CartError> {
        if !original
            .strip_prefix(BOM)
            .unwrap_or(original)
            .starts_with(TEXT_HEADER)
        {
            return Err(CartError::NotACart);
        }
        let layout = TextLayout::of(original);
        let mut edits = Vec::new();
        let new_sfx = claimed_rows(
            &layout.sfx.rows,
            &claim.sfx,
            &mut edits,
            |slot| sfx_line(self.sfx_bytes(slot)),
            &sfx_line(Cart::default().sfx_bytes(0)),
        );
        let patterns: Vec<u8> = claim
            .patterns
            .map_or(Vec::new(), |(first, last)| (first..=last).collect());
        let new_music = claimed_rows(
            &layout.music.rows,
            &patterns,
            &mut edits,
            |slot| music_line(self.pattern_bytes(slot)),
            &music_line(Cart::default().pattern_bytes(0)),
        );
        edits.extend(layout.insertion(&layout.sfx, SFX_SECTION, new_sfx));
        edits.extend(layout.insertion(&layout.music, MUSIC_SECTION, new_music));
        Ok(apply_edits(original, edits))
    }
}

/// Stores Cart bytes from an address on in the low 2 bits of each pixel's channels.
fn put_bytes(pixels: &mut [u8], at: usize, bytes: &[u8]) {
    for (index, byte) in bytes.iter().enumerate() {
        let pixel = &mut pixels[(at + index) * PNG_CHANNELS..][..PNG_CHANNELS];
        let bits = [byte >> 4, byte >> 2, *byte, byte >> 6];
        for (channel, bits) in pixel.iter_mut().zip(bits) {
            *channel = *channel & !3 | bits & 3;
        }
    }
}

/// A whole PNG file holding these pixels.
fn encode_png(pixels: &[u8]) -> Option<Vec<u8>> {
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, PNG_WIDTH, PNG_HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(pixels).ok()?;
    writer.finish().ok()?;
    Some(file)
}

/// The chunks of a PNG file, each with its length, type, data and checksum.
fn png_chunks(file: &[u8]) -> Option<Vec<&[u8]>> {
    let mut chunks = Vec::new();
    let mut rest = file.get(PNG_SIGNATURE.len()..)?;
    while !rest.is_empty() {
        let length = u32::from_be_bytes(rest.get(..4)?.try_into().ok()?) as usize;
        let (chunk, after) = rest.split_at_checked(CHUNK_HEAD + length + CHUNK_TAIL)?;
        chunks.push(chunk);
        rest = after;
        if chunk_type(chunk) == IMAGE_END {
            break;
        }
    }
    Some(chunks)
}

fn chunk_type(chunk: &[u8]) -> &[u8] {
    &chunk[4..CHUNK_HEAD]
}

/// The original file with its image swapped for a newly encoded one. Every other chunk, and
/// anything after the end of the image, stays byte for byte.
fn spliced_png(original: &[u8], encoded: &[u8]) -> Option<Vec<u8>> {
    let new_chunks = png_chunks(encoded)?;
    let new_of_type = |kind: &[u8]| -> Vec<u8> {
        new_chunks
            .iter()
            .filter(|chunk| chunk_type(chunk) == kind)
            .flat_map(|chunk| chunk.iter().copied())
            .collect()
    };
    let mut file = PNG_SIGNATURE.to_vec();
    let mut image_written = false;
    let mut kept = PNG_SIGNATURE.len();
    for chunk in png_chunks(original)? {
        kept += chunk.len();
        match chunk_type(chunk) {
            IMAGE_HEADER => file.extend(new_of_type(IMAGE_HEADER)),
            IMAGE_DATA if image_written => {}
            IMAGE_DATA => {
                file.extend(new_of_type(IMAGE_DATA));
                image_written = true;
            }
            _ => file.extend_from_slice(chunk),
        }
    }
    file.extend_from_slice(&original[kept..]);
    Some(file)
}

/// Replaces the claimed rows a section already has, and returns the rows it has to gain: default
/// rows up to the last claimed row that holds anything. A row PICO-8 would leave out stays out.
fn claimed_rows(
    rows: &[Range<usize>],
    claimed: &[u8],
    edits: &mut Vec<Edit>,
    line_of: impl Fn(u8) -> String,
    default_line: &str,
) -> Vec<String> {
    let is_past_end = |slot: u8| usize::from(slot) >= rows.len();
    for &slot in claimed.iter().filter(|slot| !is_past_end(**slot)) {
        edits.push((rows[usize::from(slot)].clone(), line_of(slot)));
    }
    let last_new = claimed
        .iter()
        .copied()
        .filter(|&slot| is_past_end(slot) && line_of(slot) != default_line)
        .max();
    let Some(last_new) = last_new else {
        return Vec::new();
    };
    (rows.len() as u8..=last_new)
        .map(|slot| {
            if claimed.contains(&slot) {
                line_of(slot)
            } else {
                default_line.to_owned()
            }
        })
        .collect()
}

/// The rows of a `__sfx__` or `__music__` section, as the reader counts them.
#[derive(Default)]
struct SectionRows {
    rows: Vec<Range<usize>>,
    /// Just past the section's last row, or its mark if it has no rows.
    append_at: Option<usize>,
}

/// One section of a `.p8` file, from its mark up to the next mark.
struct Span<'a> {
    name: &'a [u8],
    start: usize,
    end: usize,
}

/// Where everything is in a `.p8` file.
struct TextLayout<'a> {
    bytes: &'a [u8],
    sfx: SectionRows,
    music: SectionRows,
    spans: Vec<Span<'a>>,
}

impl<'a> TextLayout<'a> {
    fn of(bytes: &'a [u8]) -> Self {
        let mut layout = Self {
            bytes,
            sfx: SectionRows::default(),
            music: SectionRows::default(),
            spans: Vec::new(),
        };
        let mut section = Section::Other;
        let mut start = 0;
        for line in bytes.split_inclusive(|byte| *byte == b'\n') {
            let content = line.strip_suffix(b"\n").unwrap_or(line);
            let content = content.strip_suffix(b"\r").unwrap_or(content);
            let range = start..start + content.len();
            let next = start + line.len();
            if is_section_mark(content) {
                section = match content {
                    SFX_SECTION => Section::Sfx,
                    MUSIC_SECTION => Section::Music,
                    _ => Section::Other,
                };
                layout.open_span(content, start);
                if let Some(rows) = layout.rows_of(section) {
                    rows.append_at = Some(next);
                }
            } else if !content.is_empty()
                && let Some(rows) = layout.rows_of(section)
                && rows.rows.len() < SLOTS
            {
                rows.rows.push(range);
                rows.append_at = Some(next);
            }
            start = next;
        }
        if let Some(last) = layout.spans.last_mut() {
            last.end = bytes.len();
        }
        layout
    }

    fn rows_of(&mut self, section: Section) -> Option<&mut SectionRows> {
        match section {
            Section::Sfx => Some(&mut self.sfx),
            Section::Music => Some(&mut self.music),
            Section::Other => None,
        }
    }

    fn open_span(&mut self, name: &'a [u8], start: usize) {
        if let Some(last) = self.spans.last_mut() {
            last.end = start;
        }
        self.spans.push(Span {
            name,
            start,
            end: start,
        });
    }

    /// The end of the last section with this name.
    fn end_of(&self, name: &[u8]) -> Option<usize> {
        let span = self.spans.iter().rev().find(|span| span.name == name)?;
        Some(span.end)
    }

    /// Where a new `__sfx__` section goes: after the sections that come before it in a Cart.
    fn canonical_sfx_position(&self) -> usize {
        SECTIONS_BEFORE_SFX
            .iter()
            .find_map(|name| self.end_of(name))
            .or_else(|| self.spans.first().map(|span| span.start))
            .unwrap_or(self.bytes.len())
    }

    /// The edit that adds rows to a section, creating the section if the file lacks it.
    fn insertion(&self, section: &SectionRows, name: &[u8], rows: Vec<String>) -> Option<Edit> {
        if rows.is_empty() {
            return None;
        }
        if let Some(at) = section.append_at {
            return Some(self.inserted_lines(at, rows));
        }
        let at = if name == MUSIC_SECTION {
            self.end_of(SFX_SECTION)
                .unwrap_or_else(|| self.canonical_sfx_position())
        } else {
            self.canonical_sfx_position()
        };
        let mark = String::from_utf8_lossy(name).into_owned();
        let lines = std::iter::once(mark).chain(rows).collect();
        Some(self.inserted_lines(at, lines))
    }

    /// New lines at a position, ending as the file's other lines end. A file that stops without a
    /// final newline goes on stopping without one.
    fn inserted_lines(&self, at: usize, lines: Vec<String>) -> Edit {
        let line_ending = self.line_ending();
        let open_ended = at == self.bytes.len() && !self.bytes.ends_with(b"\n");
        let text = lines
            .iter()
            .map(|line| {
                if open_ended {
                    format!("{line_ending}{line}")
                } else {
                    format!("{line}{line_ending}")
                }
            })
            .collect();
        (at..at, text)
    }

    /// The line ending of the file's first line.
    fn line_ending(&self) -> &'static str {
        let first = self.bytes.iter().position(|byte| *byte == b'\n');
        match first {
            Some(index) if index > 0 && self.bytes[index - 1] == b'\r' => "\r\n",
            _ => "\n",
        }
    }
}

/// Replaces ranges of the original bytes, which must not overlap. Edits at the same position keep
/// their order.
fn apply_edits(original: &[u8], mut edits: Vec<Edit>) -> Vec<u8> {
    edits.sort_by_key(|(range, _)| range.start);
    let mut written = Vec::with_capacity(original.len());
    let mut copied = 0;
    for (range, text) in edits {
        written.extend_from_slice(&original[copied..range.start]);
        written.extend_from_slice(text.as_bytes());
        copied = range.end;
    }
    written.extend_from_slice(&original[copied..]);
    written
}

/// A `__sfx__` line: the 4 header bytes, then 32 notes of 5 digits `PPIVE`, where the custom bit
/// adds 8 to the instrument digit.
fn sfx_line(sfx: &[u8; SFX_BYTES]) -> String {
    let mut line = String::with_capacity(8 + SFX_NOTES * NOTE_DIGITS);
    for byte in &sfx[SFX_NOTES * 2..] {
        push_hex(&mut line, *byte);
    }
    for note in sfx[..SFX_NOTES * 2].as_chunks::<2>().0 {
        let word = u16::from_le_bytes(*note);
        let instrument = (word >> 6 & 7) | (word >> 15) << 3;
        let (volume, effect) = (word >> 9 & 7, word >> 12 & 7);
        // Writing to a String cannot fail.
        let _ = write!(
            line,
            "{:02x}{instrument:x}{volume:x}{effect:x}",
            word & 0x3f
        );
    }
    line
}

/// A `__music__` line, `FF CCCCCCCC`: bit 7 of channel byte n becomes bit n of the flags.
fn music_line(pattern: [u8; PATTERN_BYTES]) -> String {
    let flags = (0..PATTERN_BYTES).fold(0, |flags, channel| {
        flags | (pattern[channel] >> 7) << channel
    });
    let mut line = String::with_capacity(11);
    push_hex(&mut line, flags);
    line.push(' ');
    for byte in pattern {
        push_hex(&mut line, byte & 0x7f);
    }
    line
}

fn push_hex(line: &mut String, byte: u8) {
    // Writing to a String cannot fail.
    let _ = write!(line, "{byte:02x}");
}
