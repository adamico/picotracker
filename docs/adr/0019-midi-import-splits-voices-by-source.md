# MIDI Import splits voices by source and tolerates legato

MIDI Import groups notes by source: channel 10 (drums) is one source across the whole file whatever MIDI track it is on, and every other channel is split by (MIDI track, channel). Within a source, a note may overlap the next one by 1/8 of a beat and still stay in the same voice. We chose the overlap because played parts rarely let go exactly on time, and a stricter rule would scatter one melody over several Tracks.
