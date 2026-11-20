//! What our synth plays for a driver Cart. A case played through `sfx()` goes straight to one
//! voice of the offline synth. A case played through `music()` goes the way a user hears a Cart:
//! Cart Import, then the Player.

use super::cases::Plays;
use crate::support::{stored_instrument, stored_sound};
use pikutracker_core::cart::Cart;
use pikutracker_core::cart_import::import;
use pikutracker_core::player::{AudioRequest, Player};
use pikutracker_core::synth::Voice;
use std::sync::Arc;

const SFX_INSTRUMENTS: u8 = 8;
const CHANNEL_0_PHASE_AT_BOOT: u16 = 26_056;

/// The samples our synth plays for a Cart. `sfx()` keeps the phase its channel holds, so a case
/// played through it starts at the phase, in 65,536ths of a cycle, that PICO-8 0.2.7 measures on
/// channel 0 when a Cart starts.
pub fn render(cart: &Cart, plays: Plays, samples: usize) -> Vec<f32> {
    match plays {
        Plays::Sfx(slot) => {
            let mut voice = Voice::with_phase(CHANNEL_0_PHASE_AT_BOOT);
            for instrument in 0..SFX_INSTRUMENTS {
                voice.set_instrument(instrument, stored_instrument(cart, instrument));
            }
            voice.play(stored_sound(cart, slot), 0);
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
