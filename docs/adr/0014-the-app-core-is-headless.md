# The App core is headless and the Window shell does all input and output

`pikutracker-core` holds every decision and depends on no window, audio device or disk. It asks for files with `FileRequest`s and receives `FileResponse`s, and it asks for sound with `AudioRequest`s. The `pikutracker` binary is the Window shell: it carries those requests out with winit, pixels and cpal and turns keys into named Actions. We chose this so that the whole app is testable headless through named Actions and file contents, which is the main test seam.

## Consequences

- CI fails if the core gains a winit, pixels, softbuffer, cpal or wgpu dependency.
- The Player runs on its own thread and reaches the audio device through lock-free queues, so the UI thread and the device callback never wait for each other. A playback report dropped on a full queue is harmless, because the next report says the same and more.
- The shell scales the frame by the largest whole number that fits the window, never below ×1, and clears the rest to black.
