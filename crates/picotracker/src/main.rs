//! picotracker: a keyboard-driven tracker for composing PICO-8 music.

mod shell;

use std::process::ExitCode;

/// Printed by `--licenses` ahead of the notices. The README says the same.
const INDEPENDENCE_STATEMENT: &str = "picotracker is an independent project. It is not affiliated \
with or endorsed by Lexaloffle or Dirtywave, and it includes no PICO-8 code.";

/// The file shipped in each release, built into the binary.
const THIRD_PARTY_LICENSES: &str = include_str!("../../../THIRD_PARTY_LICENSES");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--version") {
        println!("picotracker {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|arg| arg == "--licenses") {
        println!("{INDEPENDENCE_STATEMENT}\n\n{THIRD_PARTY_LICENSES}");
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
