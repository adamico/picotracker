//! The audio stream: the Player runs on its own thread and reaches the device through lock-free
//! queues, so the UI thread and the device callback never wait for each other.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use picotracker_core::player::{AudioRequest, Player, Resampler};
use rtrb::{Consumer, Producer, RingBuffer};
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const REQUEST_QUEUE: usize = 64;
/// Frames queued for the device: a stop is heard within this many seconds.
const QUEUED_SECONDS: f32 = 0.02;
const MIN_QUEUED_FRAMES: usize = 256;
const SYNTH_THREAD_NAP: Duration = Duration::from_millis(1);

/// A running audio stream. Dropping it stops the sound and the synth thread.
pub struct AudioOutput {
    requests: Producer<AudioRequest>,
    running: Arc<AtomicBool>,
    finished_play: Arc<AtomicU64>,
    synth_thread: Option<JoinHandle<()>>,
    _stream: Stream,
}

impl AudioOutput {
    /// Opens the default output device and starts the synth thread.
    pub fn start() -> Result<Self, Box<dyn Error>> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no audio output device")?;
        let supported = device.default_output_config()?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.into();

        let queued = ((config.sample_rate as f32 * QUEUED_SECONDS) as usize).max(MIN_QUEUED_FRAMES);
        let (frames, device_frames) = RingBuffer::new(queued);
        let (requests, player_requests) = RingBuffer::new(REQUEST_QUEUE);

        let stream = match format {
            SampleFormat::F32 => open_stream::<f32>(&device, &config, device_frames),
            SampleFormat::I16 => open_stream::<i16>(&device, &config, device_frames),
            SampleFormat::U16 => open_stream::<u16>(&device, &config, device_frames),
            SampleFormat::I32 => open_stream::<i32>(&device, &config, device_frames),
            other => Err(format!("unsupported audio sample format {other}").into()),
        }?;
        stream.play()?;

        let running = Arc::new(AtomicBool::new(true));
        let finished_play = Arc::new(AtomicU64::new(0));
        let synth_thread = thread::Builder::new().name("synth".into()).spawn({
            let running = Arc::clone(&running);
            let finished_play = Arc::clone(&finished_play);
            let device_rate = config.sample_rate;
            move || {
                run_synth(
                    &running,
                    &finished_play,
                    device_rate,
                    player_requests,
                    frames,
                );
            }
        })?;

        Ok(Self {
            requests,
            running,
            finished_play,
            synth_thread: Some(synth_thread),
            _stream: stream,
        })
    }

    /// The number of the latest play that ended by itself.
    pub fn finished_play(&self) -> u64 {
        self.finished_play.load(Ordering::Relaxed)
    }

    /// Passes a request to the synth thread.
    pub fn send(&mut self, request: AudioRequest) {
        if self.requests.push(request).is_err() {
            eprintln!("picotracker: the audio request queue is full");
        }
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(thread) = self.synth_thread.take() {
            let _ = thread.join();
        }
    }
}

/// Keeps the device's queue full until the output closes.
fn run_synth(
    running: &AtomicBool,
    finished_play: &AtomicU64,
    device_rate: u32,
    mut requests: Consumer<AudioRequest>,
    mut frames: Producer<f32>,
) {
    let mut player = Player::new();
    let mut resampler = Resampler::new(device_rate);
    while running.load(Ordering::Relaxed) {
        while let Ok(request) = requests.pop() {
            player.handle(request);
        }
        while !frames.is_full() {
            let frame = resampler.next_frame(|| player.next_sample());
            if frames.push(frame).is_err() {
                break;
            }
        }
        finished_play.store(player.finished_play(), Ordering::Relaxed);
        thread::sleep(SYNTH_THREAD_NAP);
    }
}

/// Builds a stream that copies queued frames to every channel, with silence if the queue is empty.
fn open_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    mut frames: Consumer<f32>,
) -> Result<Stream, Box<dyn Error>>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let stream = device.build_output_stream(
        *config,
        move |output: &mut [T], _| {
            for frame in output.chunks_mut(channels) {
                let sample = T::from_sample(frames.pop().unwrap_or(0.0));
                frame.fill(sample);
            }
        },
        |error| eprintln!("picotracker: audio stream error: {error}"),
        None,
    )?;
    Ok(stream)
}
