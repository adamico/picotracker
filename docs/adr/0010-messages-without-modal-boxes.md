# Messages go on the status line and in the Log, never in a modal box

Every message goes on the status line, the bottom row of the grid. A result with items also opens the Log in the right-hand slot, without taking focus. Questions sit on the status line with their answers inline, such as `ENTER OVERWRITE  ESC CANCEL`. We prototyped three layouts: the status line with the Log, a modal box, and a full-screen report with markers on the grid. We chose the first because the panes stay usable while results are showing, so the user can fix one problem while reading the next.

## Consequences

- Export reports every failure at once, each as a Log item that jumps to where it is.
- Short questions go on the status line. Longer content, such as the MIDI Import dialog, uses the right-hand slot or a full-screen view.
