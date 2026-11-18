//! The command line of the `picotracker` binary.

use std::process::Command;

#[test]
fn version_flag_prints_the_app_version_without_opening_a_window() {
    let output = Command::new(env!("CARGO_BIN_EXE_picotracker"))
        .arg("--version")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("picotracker {}\n", env!("CARGO_PKG_VERSION"))
    );
}
