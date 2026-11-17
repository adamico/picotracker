# PROTOTYPE: playback spike (adamico/picotracker#15). Never merged.

Is a faithful-enough emulation achievable with a Rust port of zepto8's synth
(+ fake-08's REVERB-2 fix) played through cpal 0.18?

    cargo run -- ~/pico-advent-sound/cart/advent26.p8 music 0     # live, prints each note change
    cargo run -- some.p8.png info                                  # list SFX + Patterns
    cargo run -- some.p8 sfx 8 --wav out.wav                       # offline render at 22050 Hz
    cargo run -- some.p8 music 0 --mute 1,2                        # mute channels while listening

A/B against real PICO-8:

    python3 carts/make_coverage.py                 # writes coverage.p8 + ref_driver.p8
    ~/pico-8/pico8 -run carts/ref_driver.p8        # PICO-8 plays + records ref_*.wav (audible)
    ./ab.sh                                        # spike renders + compare table

`src/synth.rs` is a near line-by-line port of zepto8 @ b2929a4 (WTFPL v2,
Sam Hocevar) with fake-08's REVERB-2 fix (MIT, Jon Bell). Waveform
instruments are not implemented.
