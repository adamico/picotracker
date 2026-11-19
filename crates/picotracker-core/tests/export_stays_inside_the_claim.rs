//! Property tests over random Carts, Claims and Projects: Export changes nothing outside the
//! Claim. The Carts are generated here from a fixed seed, so every run checks the same cases, and
//! a failure names the case that broke. None of them were saved by PICO-8.
//!
//! The checks read the files with their own small parser instead of the Cart codec, so that a
//! mistake in the codec cannot hide itself.

use picotracker_core::cart::Cart;
use picotracker_core::export::export;
use picotracker_core::project::{Chain, ChainEntry, ChainId, Claim, PhraseId, Project, Step};
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;

const TEXT_CASES: u64 = 400;
const PNG_CASES: u64 = 40;

const PATTERNS_AT: usize = 0x3100;
const SFX_AT: usize = 0x3200;
const SHA1_AT: usize = 0x8006;
const PNG_WIDTH: u32 = 160;
const PNG_HEIGHT: u32 = 205;

/// A small generator with a fixed sequence for each seed.
struct Rng(u64);

impl Rng {
    fn new(case: u64) -> Self {
        Self(case.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number from 0 up to but not including `limit`.
    fn below(&mut self, limit: usize) -> usize {
        ((self.next() >> 33) as usize) % limit
    }

    /// A number from `low` to `high`, both included.
    fn between(&mut self, low: usize, high: usize) -> usize {
        low + self.below(high - low + 1)
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn byte(&mut self) -> u8 {
        self.below(256) as u8
    }

    fn shuffled(&mut self, mut items: Vec<u8>) -> Vec<u8> {
        for index in (1..items.len()).rev() {
            items.swap(index, self.below(index + 1));
        }
        items
    }
}

/// A random simple Project, and how many SFX and Patterns it can need at most.
fn random_project(rng: &mut Rng) -> (Project, usize, usize) {
    let mut project = Project::new();
    let tracks = rng.shuffled((1..=8).collect());
    let track_count = rng.between(1, 4);
    let channels = rng.shuffled(vec![0, 1, 2, 3]);
    for (channel, track) in channels.iter().zip(&tracks[..track_count]) {
        project.set_export_track(usize::from(*channel), Some(*track));
    }

    let song_rows = rng.between(1, 4);
    let (mut next_phrase, mut next_chain, mut patterns) = (0, 0, 0);
    for row in 0..song_rows {
        let chain_rows = rng.between(1, 3);
        patterns += chain_rows;
        let shapes: Vec<(i32, i32)> = (0..chain_rows)
            .map(|_| (rng.between(1, 32) as i32, rng.between(1, 255) as i32))
            .collect();
        for track in &tracks[..track_count] {
            let mut chain = Chain::default();
            for (k, (length, speed)) in shapes.iter().enumerate() {
                let id = PhraseId::new(next_phrase);
                next_phrase += 1;
                let phrase = project.phrase_mut(id);
                phrase.set_length(*length);
                phrase.set_speed(*speed);
                for step in 0..32 {
                    *phrase.step_mut(step) = Step::new(
                        rng.between(12, 50) as u8,
                        rng.below(8) as u8,
                        rng.below(8) as u8,
                        rng.below(8) as u8,
                    )
                    .unwrap();
                }
                let mut entry = ChainEntry::new(id);
                entry.set_transpose(rng.between(0, 24) as i32 - 12);
                *chain.entry_mut(k) = Some(entry);
            }
            let id = ChainId::new(next_chain);
            next_chain += 1;
            *project.chain_mut(id) = chain;
            project.set_song_cell(row, usize::from(*track) - 1, Some(id));
        }
    }
    let loop_row = rng.chance(50).then(|| rng.below(song_rows));
    project.set_song_loop_row(loop_row);
    (project, usize::from(next_phrase), patterns)
}

/// A random Claim with room for what a Project needs.
fn random_claim(rng: &mut Rng, sfx: usize, patterns: usize) -> Claim {
    let slots = rng.shuffled((0..64).collect());
    let count = rng.between(sfx, 64);
    let first = rng.between(0, 64 - patterns);
    let last = rng.between(first + patterns - 1, 63);
    Claim {
        sfx: slots[..count].to_vec(),
        patterns: Some((first as u8, last as u8)),
    }
}

fn hex_text(rng: &mut Rng, digits: usize, upper: bool) -> String {
    let alphabet: &[u8] = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    (0..digits)
        .map(|_| char::from(alphabet[rng.below(16)]))
        .collect()
}

/// A random `.p8` file: any line ending, any set of sections, and trailing rows left out.
fn random_p8(rng: &mut Rng) -> Vec<u8> {
    let eol = if rng.chance(30) { "\r\n" } else { "\n" };
    let upper = rng.chance(20);
    let mut lines: Vec<String> = vec![
        "pico-8 cartridge // http://www.pico-8.com".to_owned(),
        format!("version {}", rng.between(8, 43)),
    ];
    for name in ["__lua__", "__gfx__", "__label__", "__gff__", "__map__"] {
        if rng.chance(60) {
            lines.push(name.to_owned());
            for _ in 0..rng.below(4) {
                lines.push(format!("x{} = {}", rng.below(9), hex_text(rng, 6, false)));
            }
            if rng.chance(30) {
                lines.push(String::new());
            }
        }
    }
    if rng.chance(20) {
        lines.push("__meta:early__".to_owned());
        lines.push("before the music".to_owned());
    }
    if rng.chance(70) {
        lines.push("__sfx__".to_owned());
        for _ in 0..rng.below(65) {
            lines.push(hex_text(rng, 168, upper));
            if rng.chance(5) {
                lines.push(String::new());
            }
        }
    }
    if rng.chance(70) {
        lines.push("__music__".to_owned());
        for _ in 0..rng.below(65) {
            let flags = hex_text(rng, 2, upper);
            lines.push(format!("{flags} {}", hex_text(rng, 8, upper)));
        }
    }
    for index in 0..rng.below(3) {
        lines.push(format!("__meta:extra{index}__"));
        lines.push(format!("data = {}", hex_text(rng, 12, false)));
    }
    let mut text = lines.join(eol);
    if rng.chance(70) {
        text.push_str(eol);
    }
    if rng.chance(10) {
        text.insert(0, '\u{feff}');
    }
    text.into_bytes()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Mark,
    SfxRow(usize),
    MusicRow(usize),
    Other,
}

struct Line<'a> {
    whole: &'a [u8],
    body: &'a [u8],
    kind: Kind,
}

fn parse_lines(bytes: &[u8]) -> Vec<Line<'_>> {
    let (mut in_sfx, mut in_music) = (false, false);
    let (mut sfx_rows, mut music_rows) = (0, 0);
    bytes
        .split_inclusive(|byte| *byte == b'\n')
        .map(|whole| {
            let body = whole.strip_suffix(b"\n").unwrap_or(whole);
            let body = body.strip_suffix(b"\r").unwrap_or(body);
            let kind = if body.len() > 4 && body.starts_with(b"__") && body.ends_with(b"__") {
                in_sfx = body == b"__sfx__";
                in_music = body == b"__music__";
                Kind::Mark
            } else if body.is_empty() {
                Kind::Other
            } else if in_sfx {
                sfx_rows += 1;
                Kind::SfxRow(sfx_rows - 1)
            } else if in_music {
                music_rows += 1;
                Kind::MusicRow(music_rows - 1)
            } else {
                Kind::Other
            };
            Line { whole, body, kind }
        })
        .collect()
}

fn line_ending(line: &[u8]) -> &[u8] {
    if line.ends_with(b"\r\n") {
        b"\r\n"
    } else if line.ends_with(b"\n") {
        b"\n"
    } else {
        b""
    }
}

fn count_rows(lines: &[Line<'_>], row: fn(Kind) -> bool) -> usize {
    lines.iter().filter(|line| row(line.kind)).count()
}

fn has_mark(lines: &[Line<'_>], name: &[u8]) -> bool {
    lines.iter().any(|line| line.body == name)
}

fn mark_position(lines: &[Line<'_>], name: &[u8]) -> Option<usize> {
    lines.iter().position(|line| line.body == name)
}

/// Checks that an Exported `.p8` file differs from the original only inside the Claim.
fn assert_only_the_claim_changed(case: u64, before: &[u8], after: &[u8], claim: &Claim) {
    let claimed_sfx: BTreeSet<usize> = claim.sfx.iter().map(|slot| usize::from(*slot)).collect();
    let (first, last) = claim.patterns.unwrap();
    let claimed_music = usize::from(first)..=usize::from(last);
    let is_claimed = |kind: Kind| match kind {
        Kind::SfxRow(row) => claimed_sfx.contains(&row),
        Kind::MusicRow(row) => claimed_music.contains(&row),
        _ => false,
    };

    let old = parse_lines(before);
    let new = parse_lines(after);
    let old_sfx = count_rows(&old, |kind| matches!(kind, Kind::SfxRow(_)));
    let old_music = count_rows(&old, |kind| matches!(kind, Kind::MusicRow(_)));
    let had_sfx = has_mark(&old, b"__sfx__");
    let had_music = has_mark(&old, b"__music__");
    let is_new = |line: &Line<'_>| match line.kind {
        Kind::SfxRow(row) => row >= old_sfx,
        Kind::MusicRow(row) => row >= old_music,
        Kind::Mark => {
            (line.body == b"__sfx__" && !had_sfx) || (line.body == b"__music__" && !had_music)
        }
        Kind::Other => false,
    };

    let kept_old: Vec<&Line<'_>> = old.iter().filter(|line| !is_claimed(line.kind)).collect();
    let kept_new: Vec<&Line<'_>> = new
        .iter()
        .filter(|line| !is_claimed(line.kind) && !is_new(line))
        .collect();
    assert_eq!(kept_old.len(), kept_new.len(), "case {case}: kept lines");
    for (index, (old_line, new_line)) in kept_old.iter().zip(&kept_new).enumerate() {
        // Only a last line with no line ending may gain one, when lines are added after it.
        let was_open_ended = line_ending(old_line.whole).is_empty();
        if was_open_ended {
            assert_eq!(old_line.body, new_line.body, "case {case}: line {index}");
        } else {
            assert_eq!(old_line.whole, new_line.whole, "case {case}: line {index}");
        }
    }

    let file_ending = line_ending(old[0].whole);
    for line in new.iter().filter(|line| is_new(line)) {
        let ending = line_ending(line.whole);
        assert!(
            ending == file_ending || ending.is_empty(),
            "case {case}: line ending of a new line"
        );
    }

    for line in new
        .iter()
        .filter(|line| is_new(line) && !is_claimed(line.kind))
    {
        match line.kind {
            Kind::SfxRow(_) => {
                let default = format!("00100000{}", "0".repeat(160));
                assert_eq!(line.body, default.as_bytes(), "case {case}: gap row");
            }
            Kind::MusicRow(_) => {
                assert_eq!(line.body, b"00 41424344", "case {case}: gap row");
            }
            _ => {}
        }
    }

    for old_line in old.iter().filter(|line| is_claimed(line.kind)) {
        let new_line = new.iter().find(|line| line.kind == old_line.kind).unwrap();
        if !line_ending(old_line.whole).is_empty() {
            assert_eq!(
                line_ending(old_line.whole),
                line_ending(new_line.whole),
                "case {case}: line ending of a claimed row"
            );
        }
    }

    assert_new_sections_are_in_canonical_order(case, &new, had_sfx, had_music);
}

fn assert_new_sections_are_in_canonical_order(
    case: u64,
    new: &[Line<'_>],
    had_sfx: bool,
    had_music: bool,
) {
    let earlier: [&[u8]; 5] = [b"__lua__", b"__gfx__", b"__label__", b"__gff__", b"__map__"];
    if let (false, Some(sfx)) = (had_sfx, mark_position(new, b"__sfx__")) {
        for name in earlier {
            if let Some(position) = mark_position(new, name) {
                assert!(position < sfx, "case {case}: __sfx__ comes too early");
            }
        }
    }
    if let (false, Some(music), Some(sfx)) = (
        had_music,
        mark_position(new, b"__music__"),
        mark_position(new, b"__sfx__"),
    ) {
        assert!(sfx < music, "case {case}: __music__ comes before __sfx__");
    }
}

/// Checks what the file now holds, slot by slot.
fn assert_slots_changed_only_inside_the_claim(
    case: u64,
    before: &[u8],
    after: &[u8],
    claim: &Claim,
) {
    let old = Cart::read(before).unwrap();
    let new = Cart::read(after).unwrap();
    let (first, last) = claim.patterns.unwrap();
    for slot in 0..64 {
        if !claim.sfx.contains(&slot) {
            assert_eq!(
                old.sfx_bytes(slot),
                new.sfx_bytes(slot),
                "case {case}: SFX {slot}"
            );
        }
        if !(first..=last).contains(&slot) {
            assert_eq!(
                old.pattern_bytes(slot),
                new.pattern_bytes(slot),
                "case {case}: Pattern {slot}"
            );
        }
    }
}

#[test]
fn a_p8_export_changes_no_byte_outside_the_claim() {
    for case in 0..TEXT_CASES {
        let mut rng = Rng::new(case + 1);
        let before = random_p8(&mut rng);
        let (project, sfx, patterns) = random_project(&mut rng);
        let claim = random_claim(&mut rng, sfx, patterns);

        let exported = export(&project, &claim, &before)
            .unwrap_or_else(|failures| panic!("case {case}: {failures:?}"));

        assert_only_the_claim_changed(case, &before, &exported.cart_bytes, &claim);
        assert_slots_changed_only_inside_the_claim(case, &before, &exported.cart_bytes, &claim);
        let written = Cart::read(&exported.cart_bytes).unwrap();
        assert_eq!(
            written.claimed_bytes_hash(&claim),
            exported.claimed_bytes_hash,
            "case {case}: hash"
        );
    }
}

/// A random image: every channel of every pixel is random, so the picture is noise.
fn random_pixels(rng: &mut Rng) -> Vec<u8> {
    (0..PNG_WIDTH * PNG_HEIGHT * 4)
        .map(|_| rng.byte())
        .collect()
}

fn encode(pixels: &[u8]) -> Vec<u8> {
    let mut file = Vec::new();
    let mut encoder = png::Encoder::new(&mut file, PNG_WIDTH, PNG_HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(pixels).unwrap();
    writer.finish().unwrap();
    file
}

fn decode(file: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(file))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    pixels
}

fn memory_byte(pixel: &[u8; 4]) -> u8 {
    (pixel[3] & 3) << 6 | (pixel[0] & 3) << 4 | (pixel[1] & 3) << 2 | (pixel[2] & 3)
}

/// The Cart addresses an Export may change.
fn claimed_addresses(claim: &Claim) -> BTreeSet<usize> {
    let mut addresses: BTreeSet<usize> = (SHA1_AT..SHA1_AT + 20).collect();
    for slot in &claim.sfx {
        let at = SFX_AT + usize::from(*slot) * 68;
        addresses.extend(at..at + 68);
    }
    let (first, last) = claim.patterns.unwrap();
    for slot in first..=last {
        let at = PATTERNS_AT + usize::from(slot) * 4;
        addresses.extend(at..at + 4);
    }
    addresses
}

#[test]
fn a_png_export_changes_no_pixel_outside_the_claim_and_the_sha1() {
    for case in 0..PNG_CASES {
        let mut rng = Rng::new(case + 1);
        let pixels = random_pixels(&mut rng);
        let before = encode(&pixels);
        let (project, sfx, patterns) = random_project(&mut rng);
        let claim = random_claim(&mut rng, sfx, patterns);

        let exported = export(&project, &claim, &before)
            .unwrap_or_else(|failures| panic!("case {case}: {failures:?}"));

        let after = decode(&exported.cart_bytes);
        assert_eq!(pixels.len(), after.len(), "case {case}: image size");
        let may_change = claimed_addresses(&claim);
        let old_pixels = pixels.as_chunks::<4>().0.iter();
        for (address, (old, new)) in old_pixels.zip(after.as_chunks::<4>().0).enumerate() {
            if may_change.contains(&address) {
                for (old_channel, new_channel) in old.iter().zip(new) {
                    assert_eq!(
                        old_channel & 0xFC,
                        new_channel & 0xFC,
                        "case {case}: picture at {address:#x}"
                    );
                }
            } else {
                assert_eq!(old, new, "case {case}: pixel {address:#x}");
            }
        }

        let memory: Vec<u8> = after.as_chunks::<4>().0.iter().map(memory_byte).collect();
        let digest = Sha1::digest(&memory[..0x8000]);
        assert_eq!(
            memory[SHA1_AT..SHA1_AT + 20],
            *digest.as_slice(),
            "case {case}: SHA-1"
        );
        let written = Cart::read(&exported.cart_bytes).unwrap();
        assert_eq!(
            written.claimed_bytes_hash(&claim),
            exported.claimed_bytes_hash,
            "case {case}: hash"
        );
    }
}
