//! Custom Instruments from a Cart: each SFX 0–7 that notes use as an instrument becomes one
//! (ADR 0004).

use super::rebuild::{built_in_step, filters_of, has_bit_7, stored_length};
use crate::cart::Cart;
use crate::project::{
    CustomInstrument, InstrumentSound, SfxInstrument, WAVEFORM_SAMPLES, WaveformInstrument,
};

const BASS_BIT: u8 = 0x01;

/// The Custom Instrument that notes marked as SFX Instruments play from this slot. Bit 7 of byte
/// 66 makes it a Waveform Instrument, whose header bits other than Bass and the filters cannot be
/// heard.
pub(super) fn imported_instrument(cart: &Cart, slot: u8) -> CustomInstrument {
    let sfx = cart.sfx(slot);
    let filters = filters_of(sfx);
    let sound = if has_bit_7(sfx) {
        let bytes = cart.sfx_bytes(slot);
        InstrumentSound::Waveform(WaveformInstrument {
            samples: std::array::from_fn::<_, WAVEFORM_SAMPLES, _>(|index| bytes[index] as i8),
            bass: sfx.speed() & BASS_BIT != 0,
            detune: filters.detune(),
            reverb: filters.reverb(),
            dampen: filters.dampen(),
        })
    } else {
        // PICO-8 plays a note marked as an SFX Instrument inside one as a Built-in Instrument.
        let steps = (0..stored_length(sfx))
            .map(|index| built_in_step(sfx.note(index)))
            .collect();
        let (loop_start, loop_end) = if sfx.loop_start() < sfx.loop_end() {
            (sfx.loop_start(), sfx.loop_end())
        } else {
            (0, 0)
        };
        InstrumentSound::Sfx(SfxInstrument {
            steps,
            speed: sfx.speed(),
            filters,
            loop_start,
            loop_end,
        })
    };
    CustomInstrument { name: None, sound }
}
