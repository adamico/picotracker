//! Reads the WAV files PICO-8 records with `extcmd("audio_rec")`: 16-bit mono PCM at 22,050 Hz.

use picotracker_core::synth::SAMPLE_RATE;

const PCM: u16 = 1;
const MONO: u16 = 1;
const BITS: u16 = 16;

/// Samples from -1 to 1 as a WAV file in the same format.
pub fn bytes(samples: &[f32]) -> Vec<u8> {
    let data = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&PCM.to_le_bytes());
    bytes.extend_from_slice(&MONO.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&BITS.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&pcm(*sample).to_le_bytes());
    }
    bytes
}

/// A sample from -1 to 1 as a 16-bit sample.
pub fn pcm(sample: f32) -> i16 {
    (sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16
}

/// The samples of a recording, scaled to -1 to 1.
pub fn samples(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".to_owned());
    }
    let mut at = 12;
    let mut format_checked = false;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let body = &bytes[at + 8..(at + 8 + size).min(bytes.len())];
        match id {
            b"fmt " => {
                let word = |offset: usize| u16::from_le_bytes([body[offset], body[offset + 1]]);
                let rate = u32::from_le_bytes(body[4..8].try_into().unwrap());
                if word(0) != PCM || word(2) != MONO || word(14) != BITS || rate != SAMPLE_RATE {
                    return Err(format!(
                        "expected 16-bit mono PCM at {SAMPLE_RATE} Hz, got format {} with {} \
                         channels, {} bits at {rate} Hz",
                        word(0),
                        word(2),
                        word(14)
                    ));
                }
                format_checked = true;
            }
            b"data" if format_checked => {
                return Ok(body
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32768.0)
                    .collect());
            }
            _ => {}
        }
        at += 8 + size + size % 2;
    }
    Err("no format and data chunks".to_owned())
}
