//! What our synth plays for a driver Cart. A case played through `sfx()` goes straight to one
//! voice of the offline synth. A case played through `music()` goes the way a user hears a Cart:
//! Cart Import, then the Player.

use super::cases::Plays;
use picotracker_core::cart::Cart;
use picotracker_core::cart_import::import;
use picotracker_core::player::{AudioRequest, Player};
use picotracker_core::synth::{Instrument, Note, Sound, Voice, WAVEFORM_SAMPLES, Waveform};
use std::sync::Arc;

const SFX_INSTRUMENTS: u8 = 8;
const WAVEFORM_INSTRUMENT_FLAG: u8 = 0x80;
const BASS: u8 = 1;
const EDITOR_MODE: u8 = 1;
const CHANNEL_0_PHASE_AT_BOOT: u16 = 26_056;

/// One SFX of a Cart exactly as its bytes hold it, loop bytes included.
fn sound(cart: &Cart, slot: u8) -> Sound {
    let sfx = cart.sfx(slot);
    Sound {
        notes: std::array::from_fn(|index| {
            let stored = sfx.note(index);
            Note {
                pitch: i16::from(stored.pitch),
                instrument: stored.instrument,
                custom: stored.custom,
                volume: stored.volume,
                effect: stored.effect,
            }
        }),
        speed: sfx.speed(),
        filters: sfx.filters() & !EDITOR_MODE,
        loop_start: sfx.loop_start(),
        loop_end: sfx.loop_end(),
    }
}

/// What a note marked `custom` plays from one SFX 0–7: its 64 note bytes as signed samples when
/// bit 7 of byte 66 flags a Waveform Instrument, with Bass in bit 0 of byte 65, or else the SFX.
fn instrument(cart: &Cart, slot: u8) -> Instrument {
    let sfx = cart.sfx(slot);
    if sfx.loop_start() & WAVEFORM_INSTRUMENT_FLAG == 0 {
        return Instrument::Sfx(sound(cart, slot));
    }
    let bytes = cart.sfx_bytes(slot);
    Instrument::Waveform(Waveform {
        samples: std::array::from_fn::<_, WAVEFORM_SAMPLES, _>(|index| bytes[index] as i8),
        bass: sfx.speed() & BASS != 0,
        filters: sfx.filters() & !EDITOR_MODE,
    })
}

/// The samples our synth plays for a Cart. `sfx()` keeps the phase its channel holds, so a case
/// played through it starts at the phase, in 65,536ths of a cycle, that PICO-8 0.2.7 measures on
/// channel 0 when a Cart starts.
pub fn render(cart: &Cart, plays: Plays, samples: usize) -> Vec<f32> {
    match plays {
        Plays::Sfx(slot) => {
            let mut voice = Voice::with_phase(CHANNEL_0_PHASE_AT_BOOT);
            for instrument in 0..SFX_INSTRUMENTS {
                voice.set_instrument(instrument, self::instrument(cart, instrument));
            }
            voice.play(sound(cart, slot), 0);
            (0..samples).map(|_| voice.next_sample()).collect()
        }
        Plays::Music => {
            let mut player = Player::new();
            player.handle(AudioRequest::PlaySong {
                project: Arc::new(import(cart, "reference.p8").project),
                from_row: 0,
            });
            player.render(samples)
        }
    }
}
