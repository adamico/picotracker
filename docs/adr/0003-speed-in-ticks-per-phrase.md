# Tempo is a Speed in Ticks, held by each Phrase

Each Phrase stores a Speed: how many Ticks each of its Steps lasts (1–255), exactly like a PICO-8 SFX's SPD. New Phrases take the Project's default Speed. The editor shows an approximate BPM, assuming 4 Steps per beat (BPM ≈ 1807 / Speed), but never stores one.

We chose this over storing a BPM, or a single Speed for the whole Song, a Track or a Chain. PICO-8 can play only these Speeds, and the gaps between them are large (Speed 15 ≈ 120 BPM, 16 ≈ 113 BPM). Storing a BPM would mean rounding on every Export, and the Project would hold tempos that no Cart can play. A single Speed for the Song cannot hold what real Carts do: double-time drums at one Speed against a melody at another, or changes of Speed between Patterns. Cart Import must handle any Cart without loss.

## Consequences

- Every tempo in a Project plays exactly in PICO-8. Only MIDI Import rounds, picking the nearest Speed.
- Arpeggio rate follows each Phrase's Speed, as in PICO-8 (twice as fast at Speed ≤ 8).
- A Song row ends when its longest Track finishes, counted in Ticks. Shorter Tracks fall silent for the rest of the row.
- Phrases have no loop points. Cart Import unrolls a looping SFX into repeats of its Phrase in the Chain.
- Changing the tempo of a whole Song means changing many Phrases, so the editor needs a command that does it.
