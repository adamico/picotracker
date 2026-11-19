# Export refuses every out-of-range pitch that would be heard

A Step's pitch plus its Chain entry's transpose can fall outside PICO-8's 0–63. Export fails on such a Step when the pitch would be heard: the Step is audible and inside LEN, or an arpeggio reads it. An arpeggio (effect 6 or 7) plays the pitches of its group of four Steps, whatever their volume, and the group can reach past LEN. Every other out-of-range Step is silent and unread, so Export clamps its pitch into range. The Phrase and Chain panes flag exactly the Steps Export would refuse, so the user can fix them before Export.

We chose this over clamping every silent Step, the obvious reading of "silent Steps are clamped". Clamping a pitch an arpeggio reads would make the Exported Cart sound different from the Project, which breaks sample-for-sample Export.

## Consequences

- A silent Step can block Export, and its `!` flag shows on a Step that displays as empty (`...`).
- Export keeps the pitches an arpeggio reads past LEN in the SFX, rather than blanking every Step past LEN.
- Playback is more permissive than Export: it plays out-of-range Steps at their true pitch.
