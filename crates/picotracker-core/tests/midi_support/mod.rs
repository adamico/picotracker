//! Small Standard MIDI Files built in test code.
#![allow(dead_code)]

use midly::num::{u4, u7, u15, u24, u28};
use midly::{
    Format, Header, MetaMessage, MidiMessage, PitchBend, Smf, Timing, TrackEvent, TrackEventKind,
};

pub const TICKS_PER_BEAT: u32 = 96;
pub const TICKS_PER_STEP: u32 = TICKS_PER_BEAT / 4;

/// A file under construction: events at absolute ticks, track by track.
pub struct Song {
    tracks: Vec<Vec<(u32, TrackEventKind<'static>)>>,
}

impl Default for Song {
    fn default() -> Self {
        Self {
            tracks: vec![Vec::new()],
        }
    }
}

impl Song {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the next MIDI track.
    pub fn track(mut self) -> Self {
        self.tracks.push(Vec::new());
        self
    }

    fn event(mut self, tick: u32, kind: TrackEventKind<'static>) -> Self {
        self.tracks.last_mut().unwrap().push((tick, kind));
        self
    }

    fn midi(self, tick: u32, channel: u8, message: MidiMessage) -> Self {
        self.event(
            tick,
            TrackEventKind::Midi {
                channel: u4::new(channel - 1),
                message,
            },
        )
    }

    pub fn note_on(self, tick: u32, channel: u8, key: u8, velocity: u8) -> Self {
        self.midi(
            tick,
            channel,
            MidiMessage::NoteOn {
                key: u7::new(key),
                vel: u7::new(velocity),
            },
        )
    }

    pub fn note_off(self, tick: u32, channel: u8, key: u8) -> Self {
        self.midi(
            tick,
            channel,
            MidiMessage::NoteOff {
                key: u7::new(key),
                vel: u7::new(0),
            },
        )
    }

    /// A note on a channel numbered from 1.
    pub fn note(self, start: u32, length: u32, channel: u8, key: u8, velocity: u8) -> Self {
        self.note_on(start, channel, key, velocity)
            .note_off(start + length, channel, key)
    }

    pub fn program(self, tick: u32, channel: u8, program: u8) -> Self {
        self.midi(
            tick,
            channel,
            MidiMessage::ProgramChange {
                program: u7::new(program),
            },
        )
    }

    pub fn controller(self, tick: u32, channel: u8, controller: u8, value: u8) -> Self {
        self.midi(
            tick,
            channel,
            MidiMessage::Controller {
                controller: u7::new(controller),
                value: u7::new(value),
            },
        )
    }

    pub fn pitch_bend(self, tick: u32, channel: u8) -> Self {
        self.midi(
            tick,
            channel,
            MidiMessage::PitchBend {
                bend: PitchBend::from_int(100),
            },
        )
    }

    pub fn tempo(self, tick: u32, bpm: u32) -> Self {
        self.event(
            tick,
            TrackEventKind::Meta(MetaMessage::Tempo(u24::new(60_000_000 / bpm))),
        )
    }

    /// A time signature, with the denominator written as a power of two.
    pub fn metre(self, tick: u32, numerator: u8, denominator_power: u8) -> Self {
        self.event(
            tick,
            TrackEventKind::Meta(MetaMessage::TimeSignature(
                numerator,
                denominator_power,
                24,
                8,
            )),
        )
    }

    /// The finished file.
    pub fn bytes(&self) -> Vec<u8> {
        let tracks = self.tracks.iter().map(|events| {
            let mut events = events.clone();
            events.sort_by_key(|(tick, _)| *tick);
            let mut last = 0;
            let mut track: Vec<TrackEvent<'static>> = events
                .into_iter()
                .map(|(tick, kind)| {
                    let delta = u28::new(tick - last);
                    last = tick;
                    TrackEvent { delta, kind }
                })
                .collect();
            track.push(TrackEvent {
                delta: u28::new(0),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            });
            track
        });
        let smf = Smf {
            header: Header::new(
                Format::Parallel,
                Timing::Metrical(u15::new(TICKS_PER_BEAT as u16)),
            ),
            tracks: tracks.collect(),
        };
        let mut bytes = Vec::new();
        smf.write_std(&mut bytes).unwrap();
        bytes
    }
}
