//! The Window shell: runs the App core in a native window and starts the audio stream.
//!
//! `pixels` scales the frame by the largest whole number that fits, never below ×1, and clears
//! the rest of the window to black. This module holds no application logic.

mod audio;
mod files;
mod keys;

use audio::AudioOutput;
use picotracker_core::app::{Action, App, Launch, StateContents};
use picotracker_core::keymap::Keymap;
use picotracker_core::screen::frame::{Frame, LOGICAL_HEIGHT, LOGICAL_WIDTH};
use picotracker_core::screen::viewport::integer_scale;
use pixels::{Pixels, SurfaceTexture};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};

/// How often the window checks where playback has got to.
const PLAYBACK_REFRESH: Duration = Duration::from_millis(16);

/// Opens the window and runs until it closes.
pub fn run() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut shell = Shell::default();
    shell.launch();
    event_loop.run_app(&mut shell)?;
    shell.failure.map_or(Ok(()), Err)
}

struct Surface {
    window: Arc<Window>,
    pixels: Pixels<'static>,
    /// The title the window shows, so it is only set when it changes.
    title: String,
}

impl Surface {
    fn open(event_loop: &ActiveEventLoop, title: String) -> Result<Self, Box<dyn Error>> {
        let attributes = Window::default_attributes()
            .with_title(title.clone())
            .with_inner_size(initial_size(event_loop));
        let window = Arc::new(event_loop.create_window(attributes)?);

        let size = window.inner_size();
        let texture = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        let pixels = Pixels::new(LOGICAL_WIDTH, LOGICAL_HEIGHT, texture)?;
        Ok(Self {
            window,
            pixels,
            title,
        })
    }

    fn draw(&mut self, app: &App) -> Result<(), Box<dyn Error>> {
        let title = app.window_title();
        if title != self.title {
            self.window.set_title(&title);
            self.title = title;
        }
        let frame = Frame::from_grid(&app.grid());
        self.pixels.frame_mut().copy_from_slice(frame.rgba());
        self.pixels.render()?;
        Ok(())
    }
}

/// The largest whole-number multiple of the logical screen that fits on the monitor.
fn initial_size(event_loop: &ActiveEventLoop) -> PhysicalSize<u32> {
    let scale = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next())
        .map_or(1, |monitor| {
            let size = monitor.size();
            integer_scale(size.width, size.height)
        });
    PhysicalSize::new(LOGICAL_WIDTH * scale, LOGICAL_HEIGHT * scale)
}

struct Shell {
    app: App,
    keymap: Keymap,
    modifiers: ModifiersState,
    surface: Option<Surface>,
    /// The sound output. The app still runs without one.
    audio: Option<AudioOutput>,
    /// When the app was last told how much time has passed.
    last_tick: Instant,
    failure: Option<Box<dyn Error>>,
}

impl Default for Shell {
    fn default() -> Self {
        Self {
            app: App::new(),
            keymap: Keymap::pc_tracker(),
            modifiers: ModifiersState::empty(),
            surface: None,
            audio: AudioOutput::start()
                .inspect_err(|error| eprintln!("picotracker: no sound: {error}"))
                .ok(),
            last_tick: Instant::now(),
            failure: None,
        }
    }
}

impl Shell {
    /// Gives the app what `state.ron` holds and starts in the folder the app was launched from.
    fn launch(&mut self) {
        let directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let state_file = files::state_file();
        let state = state_file
            .as_deref()
            .map_or(StateContents::Missing, files::read_state);
        self.app.launch(Launch {
            state_file,
            state,
            directory,
        });
        self.run_file_requests();
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: Box<dyn Error>) {
        self.failure = Some(error);
        event_loop.exit();
    }

    /// Applies one action, carries out what it asks for, and closes the window if it quits.
    fn apply(&mut self, event_loop: &ActiveEventLoop, action: Action) {
        self.app.apply(action);
        if self.app.wants_to_quit() {
            event_loop.exit();
            return;
        }
        self.send_audio_requests();
        self.run_file_requests();
        self.send_audio_requests();
        if let Some(surface) = self.surface.as_ref() {
            surface.window.request_redraw();
        }
    }

    /// Passes the app's audio requests to the synth thread.
    fn send_audio_requests(&mut self) {
        for request in self.app.take_audio_requests() {
            if let Some(audio) = self.audio.as_mut() {
                audio.send(request);
            }
        }
    }

    /// Carries out the app's file requests, including any that the results lead to.
    fn run_file_requests(&mut self) {
        loop {
            let requests = self.app.take_file_requests();
            if requests.is_empty() {
                return;
            }
            for request in requests {
                self.app.receive_file(files::carry_out(request));
            }
        }
    }
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.surface.is_some() {
            return;
        }
        match Surface::open(event_loop, self.app.window_title()) {
            Ok(surface) => {
                surface.window.request_redraw();
                self.surface = Some(surface);
            }
            Err(error) => self.fail(event_loop, error),
        }
    }

    /// Lets time pass, shows where playback has got to, and wakes again for whatever is next.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let mut changed = self.app.advance_time(now - self.last_tick);
        self.last_tick = now;
        if let Some(report) = self.audio.as_mut().and_then(AudioOutput::latest_report) {
            changed |= self.app.show_playback(&report);
        }
        if changed && let Some(surface) = self.surface.as_ref() {
            surface.window.request_redraw();
        }
        let status_change = self.app.time_until_status_change().map(|wait| now + wait);
        let next_report = self.audio.is_some().then_some(now + PLAYBACK_REFRESH);
        event_loop.set_control_flow(match status_change.into_iter().chain(next_report).min() {
            Some(wake) => ControlFlow::WaitUntil(wake),
            None => ControlFlow::Wait,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.surface.is_none() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => self.apply(event_loop, Action::Quit),
            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                let Some(surface) = self.surface.as_mut() else {
                    return;
                };
                match surface.pixels.resize_surface(size.width, size.height) {
                    Ok(()) => surface.window.request_redraw(),
                    Err(error) => self.fail(event_loop, error.into()),
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(surface) = self.surface.as_mut() else {
                    return;
                };
                if let Err(error) = surface.draw(&self.app) {
                    self.fail(event_loop, error);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let context = self.app.key_context();
                let action = keys::key_press(&event, self.modifiers)
                    .and_then(|press| self.keymap.action_in(press, context));
                if let Some(action) = action {
                    self.apply(event_loop, action);
                }
            }
            WindowEvent::KeyboardInput { .. } => {
                self.app.key_released();
                self.send_audio_requests();
            }
            _ => {}
        }
    }
}
