//! Splits each source into voices that play one note at a time.

use super::Voice;
use super::read::{Note, Score, Source};
use super::table::{drum_row, family_name};

const LEGATO_BEAT_FRACTION: u32 = 8;

/// The voices of every source, in file order, with one source's voices next to each other.
pub(super) fn split(score: &Score) -> Vec<Voice> {
    score
        .sources
        .iter()
        .enumerate()
        .flat_map(|(index, source)| voices_of(score, index, source))
        .collect()
}

/// A note may overlap the next one by a fraction of a beat and still share its voice, because
/// played parts rarely let go exactly on time.
fn voices_of(score: &Score, index: usize, source: &Source) -> Vec<Voice> {
    let legato = score.ticks_per_beat / LEGATO_BEAT_FRACTION;
    let mut notes: Vec<Note> = source
        .notes
        .iter()
        .filter(|note| !source.is_drums() || drum_row(note.key).is_some())
        .copied()
        .collect();
    // Notes struck together go highest first, or in the order of the kit for drums.
    notes.sort_by_key(|note| {
        let order = if source.is_drums() {
            drum_row(note.key).unwrap_or(usize::MAX)
        } else {
            usize::from(u8::MAX - note.key)
        };
        (note.start, order)
    });

    let mut parts: Vec<Vec<Note>> = Vec::new();
    for note in notes {
        let free = parts.iter_mut().find(|part| {
            part.last().is_some_and(|last| {
                last.start < note.start && last.end <= note.start.saturating_add(legato)
            })
        });
        match free {
            Some(part) => part.push(note),
            None => parts.push(vec![note]),
        }
    }

    let count = parts.len();
    let program = source
        .notes
        .first()
        .map(|note| score.program_at(source.channel, note.start));
    parts
        .into_iter()
        .enumerate()
        .map(|(part, notes)| {
            let name = if source.is_drums() {
                format!("CH10 {}", drum_name(&notes))
            } else {
                let family = family_name(program.unwrap_or(0));
                if count > 1 {
                    format!("{family} {}", part + 1)
                } else {
                    family.to_owned()
                }
            };
            Voice {
                name,
                source: index,
                channel: source.channel + 1,
                program: (!source.is_drums()).then(|| program.unwrap_or(0)),
                notes,
            }
        })
        .collect()
}

/// The kit row a drum voice plays most, with the earlier row winning a tie.
fn drum_name(notes: &[Note]) -> &'static str {
    let mut counts = [0_usize; super::DRUM_ROWS];
    for row in notes.iter().filter_map(|note| drum_row(note.key)) {
        counts[row] += 1;
    }
    let most = counts.iter().copied().max().unwrap_or(0);
    let row = counts.iter().position(|count| *count == most).unwrap_or(0);
    super::MidiTable::default().hit(row).name
}
