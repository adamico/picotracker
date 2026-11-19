//! What our synth plays for a driver Cart. A case played through `sfx()` goes straight to one
//! voice of the offline synth. A case played through `music()` goes the way a user hears a Cart:
//! Cart Import, then the Player.

use super::cases::Plays;
use picotracker_core::cart::Cart;
use picotracker_core::cart_import::import;
use picotracker_core::player::{AudioRequest, Player};
use picotracker_core::synth::{Note, Sound, Voice};
use std::sync::Arc;

const SFX_INSTRUMENTS: u8 = 8;

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
        filters: sfx.filters() & !1,
        loop_start: sfx.loop_start(),
        loop_end: sfx.loop_end(),
    }
}

/// The samples our synth plays for a Cart.
pub fn render(cart: &Cart, plays: Plays, samples: usize) -> Vec<f32> {
    match plays {
        Plays::Sfx(slot) => {
            let mut voice = Voice::new();
            for instrument in 0..SFX_INSTRUMENTS {
                voice.set_sfx_instrument(usize::from(instrument), sound(cart, instrument));
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
