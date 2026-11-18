//! picotracker: a keyboard-driven tracker for composing PICO-8 music.

mod shell;

use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().skip(1).any(|arg| arg == "--version") {
        println!("picotracker {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }

    match shell::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("picotracker: {error}");
            ExitCode::FAILURE
        }
    }
}
