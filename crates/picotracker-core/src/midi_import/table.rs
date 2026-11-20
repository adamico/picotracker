//! The program table and drum kit that decide what each MIDI note plays.

const TRIANGLE: u8 = 0;
const TILTED_SAW: u8 = 1;
const SAW: u8 = 2;
const SQUARE: u8 = 3;
const PULSE: u8 = 4;
const ORGAN: u8 = 5;
const NOISE: u8 = 6;
const PHASER: u8 = 7;

const DROP: u8 = 3;
const FADE_OUT: u8 = 5;

const PROGRAMS_PER_FAMILY: usize = 8;
pub const PROGRAMS: usize = 128;
pub const DRUM_ROWS: usize = 5;
const MAX_PITCH: u8 = 63;
const INSTRUMENTS: u8 = 8;
const EFFECTS: u8 = 8;
const LOWEST_TOM: u8 = 41;
const TOMS: usize = 4;

const FAMILIES: [(&str, u8); PROGRAMS / PROGRAMS_PER_FAMILY] = [
    ("PIANO", TILTED_SAW),
    ("CHROMATIC", TILTED_SAW),
    ("ORGAN", ORGAN),
    ("GUITAR", SAW),
    ("BASS", TRIANGLE),
    ("STRINGS", SAW),
    ("ENSEMBLE", PULSE),
    ("BRASS", SAW),
    ("REED", SQUARE),
    ("PIPE", TRIANGLE),
    ("LEAD", SQUARE),
    ("PAD", PULSE),
    ("SYNTH FX", PHASER),
    ("ETHNIC", TILTED_SAW),
    ("PERCUSSIVE", TILTED_SAW),
    ("SOUND FX", NOISE),
];

const DRUM_NOTES: [&[u8]; DRUM_ROWS] = [
    &[35, 36],
    &[38, 40],
    &[42, 44, 46],
    &[49, 51, 52, 53, 55, 57, 59],
    &[41, 43, 45, 47, 48, 50],
];

/// The name of a program's General MIDI family.
pub fn family_name(program: u8) -> &'static str {
    FAMILIES[usize::from(program) / PROGRAMS_PER_FAMILY % FAMILIES.len()].0
}

/// The kit row a General MIDI drum note belongs to, if any.
pub(super) fn drum_row(key: u8) -> Option<usize> {
    DRUM_NOTES.iter().position(|notes| notes.contains(&key))
}

/// What one hit of the drum kit plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrumHit {
    pub name: &'static str,
    pub instrument: u8,
    pub note: u8,
    pub effect: u8,
}

const fn hit(name: &'static str, instrument: u8, note: u8, effect: u8) -> DrumHit {
    DrumHit {
        name,
        instrument,
        note,
        effect,
    }
}

const FACTORY_KIT: [DrumHit; DRUM_ROWS] = [
    hit("KICK", TRIANGLE, 12, DROP),
    hit("SNARE", NOISE, 30, FADE_OUT),
    hit("HATS", NOISE, 55, FADE_OUT),
    hit("CYMBALS", NOISE, 50, FADE_OUT),
    hit("TOMS", TRIANGLE, 17, DROP),
];

/// How one Import maps MIDI programs and drum notes to Instruments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidiTable {
    programs: [u8; PROGRAMS],
    kit: [DrumHit; DRUM_ROWS],
}

impl Default for MidiTable {
    /// The factory table.
    fn default() -> Self {
        Self {
            programs: std::array::from_fn(|program| FAMILIES[program / PROGRAMS_PER_FAMILY].1),
            kit: FACTORY_KIT,
        }
    }
}

impl MidiTable {
    /// The Built-in Instrument a program plays.
    pub fn instrument(&self, program: u8) -> u8 {
        self.programs[usize::from(program) % PROGRAMS]
    }

    /// What a row of the drum kit plays.
    pub fn hit(&self, row: usize) -> DrumHit {
        self.kit[row]
    }

    /// The pitch a General MIDI drum note plays. Toms keep their distance from the lowest tom.
    pub(super) fn drum_pitch(&self, row: usize, key: u8) -> u8 {
        let note = self.kit[row].note;
        if row == TOMS {
            note.saturating_add(key.saturating_sub(LOWEST_TOM))
                .min(MAX_PITCH)
        } else {
            note
        }
    }

    /// Changes a program's Instrument, going round the 8 Built-in Instruments.
    pub fn change_program(&mut self, program: u8, up: bool) {
        let instrument = &mut self.programs[usize::from(program) % PROGRAMS];
        *instrument = stepped_round(*instrument, up, INSTRUMENTS);
    }

    /// Changes a drum hit's Instrument, going round the 8 Built-in Instruments.
    pub fn change_hit_instrument(&mut self, row: usize, up: bool) {
        let instrument = &mut self.kit[row].instrument;
        *instrument = stepped_round(*instrument, up, INSTRUMENTS);
    }

    /// Raises or lowers a drum hit's note, stopping at 0 and 63.
    pub fn change_hit_note(&mut self, row: usize, up: bool) {
        let note = &mut self.kit[row].note;
        *note = if up {
            (*note + 1).min(MAX_PITCH)
        } else {
            note.saturating_sub(1)
        };
    }

    /// Moves a drum hit on to the next effect.
    pub fn cycle_hit_effect(&mut self, row: usize) {
        let effect = &mut self.kit[row].effect;
        *effect = stepped_round(*effect, true, EFFECTS);
    }

    /// Whether a program differs from the factory table.
    pub fn program_is_edited(&self, program: u8) -> bool {
        self.instrument(program) != Self::default().instrument(program)
    }

    /// Whether a drum hit differs from the factory kit.
    pub fn hit_is_edited(&self, row: usize) -> bool {
        self.kit[row] != FACTORY_KIT[row]
    }

    /// Whether anything differs from the factory table.
    pub fn is_edited(&self) -> bool {
        *self != Self::default()
    }

    /// Sets a program's Instrument, if both are in range.
    pub fn set_program(&mut self, program: u8, instrument: u8) -> Option<()> {
        let slot = self.programs.get_mut(usize::from(program))?;
        *slot = (instrument < INSTRUMENTS).then_some(instrument)?;
        Some(())
    }

    /// Sets what a drum kit row plays, if the row and every value are in range.
    pub fn set_hit(&mut self, row: usize, instrument: u8, note: u8, effect: u8) -> Option<()> {
        let in_range = instrument < INSTRUMENTS && note <= MAX_PITCH && effect < EFFECTS;
        let hit = self.kit.get_mut(row).filter(|_| in_range)?;
        *hit = DrumHit {
            instrument,
            note,
            effect,
            ..*hit
        };
        Some(())
    }

    /// The programs that differ from the factory table.
    pub fn edited_programs(&self) -> impl Iterator<Item = u8> + '_ {
        (0..=u8::MAX)
            .take(PROGRAMS)
            .filter(|&program| self.program_is_edited(program))
    }

    /// The drum kit rows that differ from the factory kit.
    pub fn edited_hits(&self) -> impl Iterator<Item = usize> + '_ {
        (0..DRUM_ROWS).filter(|&row| self.hit_is_edited(row))
    }

    /// Puts a program back to the factory table.
    pub fn reset_program(&mut self, program: u8) {
        let index = usize::from(program) % PROGRAMS;
        self.programs[index] = Self::default().programs[index];
    }

    /// Puts a drum hit back to the factory kit.
    pub fn reset_hit(&mut self, row: usize) {
        self.kit[row] = FACTORY_KIT[row];
    }
}

const fn stepped_round(value: u8, up: bool, count: u8) -> u8 {
    if up {
        (value + 1) % count
    } else {
        (value + count - 1) % count
    }
}
