# Key rebinding and chords

A key in `config.ron` replaces the action's preset keys in the widest scope the preset binds it in, and takes the key from any action that had it there. Narrower bindings stay, such as P in the song picker beside Space. A key held first in a chord acts on key-up, not key-down, unless another key went down in between, so holding it for a chord never also triggers its own action.

## Consequences

- The X+arrow jump copies the M8's OPTION+arrows: 16 Song rows in the Song pane, and in the Chain and Phrase panes a Track sideways or the next Chain or Phrase up or down.
