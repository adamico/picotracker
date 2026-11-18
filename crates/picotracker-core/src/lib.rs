//! The headless core of picotracker.
//!
//! Nothing in this crate may depend on a window or an audio device. The `picotracker` binary
//! crate is the Window shell that connects this core to `winit`, `pixels` and, later, `cpal`.
//!
//! Each module of the MVP spec has its own file here, declared up front so that tickets built in
//! parallel do not collide in this list.

pub mod app;
pub mod cart;
pub mod cart_import;
pub mod export;
pub mod keymap;
pub mod midi_import;
pub mod player;
pub mod project;
pub mod project_file;
pub mod screen;
pub mod settings;
pub mod synth;
