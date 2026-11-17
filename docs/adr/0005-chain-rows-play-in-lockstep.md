# Chain rows play in lockstep, one Pattern each

Within a Song row, the k-th entry of every Track's Chain plays together as a **Chain row**, and Export compiles each Chain row into exactly one Pattern. A Chain entry can be marked **Fill**, which repeats its Phrase until the Chain row ends and compiles to a looping SFX. A Chain row lasts as long as its longest Phrase that is not marked Fill. This is PICO-8's own song structure: a flat list of Patterns, each holding 4 SFX that play together, where a looping SFX repeats until the Pattern ends. We chose it over M8-style free-running Chains, which Export would have to split into SFX wherever any Track's Phrase starts or ends. Free-running uses more SFX in ways that are hard to predict, can't Export cuts that land mid-Step or mid-arpeggio-group, and doesn't round-trip through Cart Import. The cost: Chains in parallel Tracks don't run independently. A [16, 16] Chain beside a [32] Chain lasts 48 Steps, not 32.

## Consequences

- This replaces part of ADR 0003. "A Song row ends when its longest Track finishes" now applies to each Chain row. Cart Import no longer unrolls a looping SFX into repeats. A loop that covers the whole SFX becomes a Phrase marked Fill. Phrases still have no loop points; the loop belongs to the Chain entry.
- Every Phrase starts at note 0 of its own SFX, so arpeggio groups of 4 always line up with PICO-8's.
- PICO-8 ends a Pattern when its *left-most* non-looping channel ends. When a shorter Phrase without Fill sits to the left of the longest one, Export gives its SFX one extra silent Step and loops that Step, so the channel no longer decides the length. When the Phrase already has 32 Steps there's no room, and Export fails. This trick still has to be checked against real PICO-8.
- If every entry in a Chain row is marked Fill, the longest plays once and sets the length.
