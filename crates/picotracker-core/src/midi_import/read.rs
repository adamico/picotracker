//! Reads a Standard MIDI File into notes, one list for each source, and the file's timing.

use super::ImportError;
use midly::{Format, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use std::collections::{BTreeMap, BTreeSet};

/// The channel that plays drums, counted from 0.
pub(super) const DRUM_CHANNEL: u8 = 9;

const SUSTAIN_PEDAL: u8 = 64;
const PEDAL_DOWN: u8 = 64;
const VOLUME: u8 = 7;
const EXPRESSION: u8 = 11;
const DEFAULT_MICROS_PER_BEAT: u32 = 500_000;
const CHANNELS: usize = 16;

/// One note as the file plays it, in ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Note {
    pub(super) start: u32,
    pub(super) end: u32,
    pub(super) key: u8,
    pub(super) velocity: u8,
}

/// The notes of one (MIDI track, channel) pair. Channel 10 is one source across the whole file.
#[derive(Clone, Debug)]
pub(super) struct Source {
    pub(super) channel: u8,
    pub(super) notes: Vec<Note>,
}

impl Source {
    pub(super) fn is_drums(&self) -> bool {
        self.channel == DRUM_CHANNEL
    }
}

/// A time signature: beats in a bar, and the note value of a beat as a power of two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Metre {
    pub(super) numerator: u8,
    pub(super) denominator_power: u8,
}

impl Default for Metre {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator_power: 2,
        }
    }
}

/// Data the file holds that Import does not use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Unused {
    PitchBend,
    Aftertouch,
    Controller(u8),
}

/// Everything Import needs from the file.
#[derive(Clone, Debug)]
pub(super) struct Score {
    pub(super) ticks_per_beat: u32,
    pub(super) sources: Vec<Source>,
    /// Tempo changes in order: the tick and the microseconds per beat.
    pub(super) tempos: Vec<(u32, u32)>,
    pub(super) metres: Vec<(u32, Metre)>,
    /// Program changes in order, for each channel.
    pub(super) programs: [Vec<(u32, u8)>; CHANNELS],
    pub(super) volume_ignored: bool,
    pub(super) expression_ignored: bool,
    pub(super) unused: BTreeSet<Unused>,
}

impl Score {
    /// The program a channel has selected at a tick.
    pub(super) fn program_at(&self, channel: u8, tick: u32) -> u8 {
        self.programs[usize::from(channel)]
            .iter()
            .take_while(|(at, _)| *at <= tick)
            .last()
            .map_or(0, |(_, program)| *program)
    }

    /// The tempo the file starts with when it names none.
    pub(super) const fn default_micros_per_beat() -> u32 {
        DEFAULT_MICROS_PER_BEAT
    }
}

/// Where a note is kept: which source, and which of its notes.
type NoteRef = (usize, usize);

/// What one MIDI track is doing at the current tick.
#[derive(Default)]
struct TrackState {
    sounding: BTreeMap<(u8, u8), NoteRef>,
    /// Notes released while the pedal is down.
    pedalled: BTreeMap<(u8, u8), NoteRef>,
    pedal_down: [bool; CHANNELS],
}

/// Reads a file, refusing what Import cannot use.
pub(super) fn read(bytes: &[u8]) -> Result<Score, ImportError> {
    let smf = Smf::parse(bytes).map_err(|_| ImportError::NotMidi)?;
    if smf.header.format == Format::Sequential {
        return Err(ImportError::SequentialTracks);
    }
    let Timing::Metrical(ticks_per_beat) = smf.header.timing else {
        return Err(ImportError::TimecodeTiming);
    };
    let mut reader = Reader {
        score: Score {
            ticks_per_beat: u32::from(ticks_per_beat.as_int()).max(1),
            sources: Vec::new(),
            tempos: Vec::new(),
            metres: Vec::new(),
            programs: Default::default(),
            volume_ignored: false,
            expression_ignored: false,
            unused: BTreeSet::new(),
        },
        source_of: BTreeMap::new(),
    };
    for (index, track) in smf.tracks.iter().enumerate() {
        let mut state = TrackState::default();
        let mut tick = 0_u32;
        for event in track {
            tick = tick.saturating_add(event.delta.as_int());
            reader.event(index, tick, &event.kind, &mut state);
        }
        reader.finish_track(tick, state);
    }
    let mut score = reader.score;
    score.tempos.sort_by_key(|(tick, _)| *tick);
    score.metres.sort_by_key(|(tick, _)| *tick);
    for changes in &mut score.programs {
        changes.sort_by_key(|(tick, _)| *tick);
    }
    for source in &mut score.sources {
        source.notes.sort_by_key(|note| note.start);
    }
    if score.sources.is_empty() {
        return Err(ImportError::NoNotes);
    }
    Ok(score)
}

struct Reader {
    score: Score,
    /// The source of each (MIDI track, channel) pair. Drums share one, under track 0.
    source_of: BTreeMap<(usize, u8), usize>,
}

impl Reader {
    fn event(&mut self, track: usize, tick: u32, kind: &TrackEventKind, state: &mut TrackState) {
        match kind {
            TrackEventKind::Meta(MetaMessage::Tempo(micros)) => {
                self.score.tempos.push((tick, micros.as_int()));
            }
            TrackEventKind::Meta(MetaMessage::TimeSignature(numerator, power, ..)) => {
                let metre = Metre {
                    numerator: (*numerator).max(1),
                    denominator_power: *power,
                };
                self.score.metres.push((tick, metre));
            }
            TrackEventKind::Midi { channel, message } => {
                self.message(track, tick, channel.as_int(), *message, state);
            }
            _ => {}
        }
    }

    fn message(
        &mut self,
        track: usize,
        tick: u32,
        channel: u8,
        message: MidiMessage,
        state: &mut TrackState,
    ) {
        match message {
            MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                self.start_note(track, tick, channel, (key.as_int(), vel.as_int()), state);
            }
            MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                let held = (channel, key.as_int());
                if let Some(note) = state.sounding.remove(&held) {
                    if state.pedal_down[usize::from(channel)] {
                        state.pedalled.insert(held, note);
                    } else {
                        self.end_note(note, tick);
                    }
                }
            }
            MidiMessage::Controller { controller, value } => {
                self.controller(tick, channel, controller.as_int(), value.as_int(), state);
            }
            MidiMessage::ProgramChange { program } => {
                self.score.programs[usize::from(channel)].push((tick, program.as_int()));
            }
            MidiMessage::PitchBend { .. } => {
                self.score.unused.insert(Unused::PitchBend);
            }
            MidiMessage::Aftertouch { .. } | MidiMessage::ChannelAftertouch { .. } => {
                self.score.unused.insert(Unused::Aftertouch);
            }
        }
    }

    fn controller(
        &mut self,
        tick: u32,
        channel: u8,
        controller: u8,
        value: u8,
        state: &mut TrackState,
    ) {
        match controller {
            SUSTAIN_PEDAL => {
                let down = value >= PEDAL_DOWN;
                state.pedal_down[usize::from(channel)] = down;
                if !down {
                    let released: Vec<(u8, u8)> = state
                        .pedalled
                        .keys()
                        .filter(|(held, _)| *held == channel)
                        .copied()
                        .collect();
                    for held in released {
                        if let Some(note) = state.pedalled.remove(&held) {
                            self.end_note(note, tick);
                        }
                    }
                }
            }
            VOLUME => self.score.volume_ignored = true,
            EXPRESSION => self.score.expression_ignored = true,
            other => {
                self.score.unused.insert(Unused::Controller(other));
            }
        }
    }

    fn start_note(
        &mut self,
        track: usize,
        tick: u32,
        channel: u8,
        (key, velocity): (u8, u8),
        state: &mut TrackState,
    ) {
        // A key struck again ends the note it was still playing.
        let held = (channel, key);
        for note in [state.sounding.remove(&held), state.pedalled.remove(&held)]
            .into_iter()
            .flatten()
        {
            self.end_note(note, tick);
        }
        let source = self.source_for(track, channel);
        let notes = &mut self.score.sources[source].notes;
        notes.push(Note {
            start: tick,
            end: tick,
            key,
            velocity,
        });
        state.sounding.insert(held, (source, notes.len() - 1));
    }

    fn source_for(&mut self, track: usize, channel: u8) -> usize {
        let track = if channel == DRUM_CHANNEL { 0 } else { track };
        *self.source_of.entry((track, channel)).or_insert_with(|| {
            self.score.sources.push(Source {
                channel,
                notes: Vec::new(),
            });
            self.score.sources.len() - 1
        })
    }

    fn end_note(&mut self, (source, note): NoteRef, tick: u32) {
        self.score.sources[source].notes[note].end = tick;
    }

    /// Notes still sounding when a track ends stop there.
    fn finish_track(&mut self, tick: u32, state: TrackState) {
        for note in state
            .sounding
            .into_values()
            .chain(state.pedalled.into_values())
        {
            self.end_note(note, tick);
        }
    }
}
