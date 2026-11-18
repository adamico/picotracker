//! Player: turns a Project and a play request into the Playhead and samples. The Window shell
//! owns the audio stream that carries them to a device.

use crate::project::{MAX_STEPS, Phrase};
use crate::synth::{Note, SAMPLE_RATE, Sound, Voice, packed_filters};

/// What the App core asks the Player to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioRequest {
    /// Loops one Phrase over its LEN, starting from one of its Steps.
    PlayPhrase { phrase: Phrase, from_step: usize },
    /// Cuts the sound at once.
    Stop,
}

/// Renders what the App core asks for, one sample at a time.
#[derive(Clone, Debug, Default)]
pub struct Player {
    voice: Voice,
}

impl Player {
    /// A silent Player.
    pub fn new() -> Self {
        Self::default()
    }

    /// Carries out one request.
    pub fn handle(&mut self, request: AudioRequest) {
        match request {
            AudioRequest::PlayPhrase { phrase, from_step } => {
                self.voice.stop();
                self.voice.play(looping_sound(&phrase), from_step);
            }
            AudioRequest::Stop => self.voice.stop(),
        }
    }

    /// The next sample at 22,050 Hz.
    pub fn next_sample(&mut self) -> f32 {
        self.voice.next_sample()
    }

    /// Renders a run of samples at 22,050 Hz.
    pub fn render(&mut self, samples: usize) -> Vec<f32> {
        (0..samples).map(|_| self.next_sample()).collect()
    }
}

/// The sound of a Phrase that loops from its first Step when it reaches LEN.
fn looping_sound(phrase: &Phrase) -> Sound {
    let mut notes = [Note::default(); MAX_STEPS];
    for (index, note) in notes.iter_mut().enumerate() {
        let step = phrase.step(index);
        *note = Note {
            pitch: i16::from(step.pitch()),
            instrument: step.instrument(),
            custom: false,
            volume: step.volume(),
            effect: step.effect(),
        };
    }
    let filters = phrase.filters();
    Sound {
        notes,
        speed: phrase.speed(),
        filters: packed_filters(
            filters.noiz(),
            filters.buzz(),
            filters.detune(),
            filters.reverb(),
            filters.dampen(),
        ),
        loop_start: 0,
        loop_end: phrase.length() as u8,
    }
}

/// Converts samples at 22,050 Hz to the device's rate by linear interpolation.
#[derive(Clone, Copy, Debug)]
pub struct Resampler {
    step: f64,
    position: f64,
    previous: f32,
    next: f32,
}

impl Resampler {
    /// A resampler for a device running at this many frames per second.
    pub fn new(device_rate: u32) -> Self {
        Self {
            step: f64::from(SAMPLE_RATE) / f64::from(device_rate),
            position: 1.0,
            previous: 0.0,
            next: 0.0,
        }
    }

    /// The next frame at the device's rate, pulling samples from the source as needed.
    pub fn next_frame(&mut self, mut source: impl FnMut() -> f32) -> f32 {
        self.position += self.step;
        while self.position >= 1.0 {
            self.position -= 1.0;
            self.previous = self.next;
            self.next = source();
        }
        self.previous + (self.next - self.previous) * self.position as f32
    }
}
