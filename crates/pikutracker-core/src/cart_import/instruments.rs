use super::rebuild::{built_in_step, filters_of, has_bit_7, stored_length};
use crate::cart::Cart;
use crate::project::{
    CustomInstrument, InstrumentSound, SfxInstrument, WAVEFORM_SAMPLES, WaveformInstrument,
};

const BASS_BIT: u8 = 0x01;

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
