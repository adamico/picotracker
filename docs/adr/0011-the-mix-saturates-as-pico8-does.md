# The mix saturates as PICO-8 does, for any number of Tracks

The Player sums its Tracks and saturates the result the way real PICO-8 0.2.7 does. Up to 0.75 of full scale the mix passes through unchanged. Past that it rises at a twenty-fifth of the rate, so four loud channels never go above about 0.77. This replaces the limiter in [Playback controls while editing](https://github.com/adamico/picotracker/issues/21), which acted only when the 8-Track mix would clip and kept the waveform's shape. We chose PICO-8's curve so that 4 Tracks sound in the tracker exactly as they will in the Exported Cart, including how loud passages squash. A limiter that acts only above full scale leaves loud 4-Track passages up to 0.9 dB away from PICO-8. The cost: PICO-8 has only 4 channels, so nothing measures how it would treat 8. With 8 loud Tracks the same curve presses the mix into about 0.75–0.84, which sounds close to a hard clip.

## Consequences

- The curve applies to everything the Player plays: Song, Chain and Phrase playback, previews and auditions.
- The knee and slope come from the PICO-8 reference set (`coverage_music0`), not from listening.
