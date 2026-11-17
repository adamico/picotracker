# Cart Import rebuilds what plays, not the stored SFX

Cart Import works out how long PICO-8 plays each Pattern, in Ticks and by PICO-8's own rules, and turns each channel into a Phrase holding the Steps that channel plays in that time. An SFX does not map one-to-one to a Phrase. We chose this over copying each SFX into a Phrase because PICO-8's Pattern timing (the left-most non-looping channel ends the Pattern) differs from the Project's (the longest Phrase not marked Fill ends the Chain row, per ADR 0005). Copied SFX would make an imported Cart play differently in the tracker and after Export.

## Consequences

- A longer channel cut off by the left-most one gets a shortened Phrase. A Pattern that ends partway through a Step drops that Step.
- A loop over the whole SFX becomes a Phrase marked Fill. A loop with a lead-in is unrolled. If unrolling needs more than 32 Steps, it becomes Fill over the loop body only, and Import warns. Notes after a loop's end are dropped, since music never plays them.
- LEN only cuts playback on the channel that sets the Pattern's length. On other channels, the notes after LEN are kept, because PICO-8 plays them.
- When every channel loops, PICO-8 plays 32 × the slowest Speed, so Import unrolls that channel without Fill.
- SPD 0 becomes Speed 1, and the timing follows PICO-8's quirk. Import warns.
- A Cart's SFX can turn into several Phrases, so an Import → Export round trip may use different SFX slots.
- Only the chosen song's SFX are claimed, and the SFX part of a Claim is a set of slots, not a range. A contiguous range would sweep in a game's sound effects lying between the song's instruments and its music, and Export would reset them. Every other non-empty SFX becomes a loose, unclaimed Phrase, with its loop dropped.
- The timing above comes from zepto8 and fake-08, not from real PICO-8. It must be checked against WAV exports from PICO-8 0.2.7.
