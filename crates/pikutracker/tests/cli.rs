//! The command line of the `pikutracker` binary.

use std::process::Command;

#[test]
fn version_flag_prints_the_app_version_without_opening_a_window() {
    let output = Command::new(env!("CARGO_BIN_EXE_pikutracker"))
        .arg("--version")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("pikutracker {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn licenses_flag_prints_the_independence_statement_and_every_notice_without_opening_a_window() {
    let output = Command::new(env!("CARGO_BIN_EXE_pikutracker"))
        .arg("--licenses")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();

    assert!(output.status.success());
    let printed = String::from_utf8(output.stdout).unwrap();
    assert!(printed.contains(
        "pikutracker is an independent project. It is not affiliated with or endorsed by \
         Lexaloffle or Dirtywave, and it includes no PICO-8 code."
    ));
    assert!(printed.contains(include_str!("../../../THIRD_PARTY_LICENSES")));
}

#[test]
fn licenses_flag_prints_the_notices_of_the_ported_synth() {
    let output = Command::new(env!("CARGO_BIN_EXE_pikutracker"))
        .arg("--licenses")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .unwrap();

    let printed = String::from_utf8(output.stdout).unwrap();
    for notice in [
        "Copyright © 2017—2020 Sam Hocevar <sam@hocevar.net>",
        "Copyright © 2017–2024 Sam Hocevar <sam@hocevar.net>",
        "Copyright © 2016–2024 Sam Hocevar <sam@hocevar.net>",
        "and/or modify it under the terms of the Do What the Fuck You Want",
        "to Public License, Version 2, as published by the WTFPL Task Force.",
        "https://github.com/jtothebell/fake-08",
        "Permission is hereby granted, free of charge, to any person obtaining a copy",
        "The above copyright notice and this permission notice shall be included in all",
        "THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR",
    ] {
        assert!(printed.contains(notice), "missing notice line: {notice}");
    }
}

#[test]
fn readme_carries_the_independence_statement() {
    assert!(include_str!("../../../README.md").contains(
        "pikutracker is an independent project. It is not affiliated with or endorsed by \
         Lexaloffle or Dirtywave, and it includes no PICO-8 code."
    ));
}

#[test]
fn third_party_licenses_carries_the_ported_synth_notices_and_the_crate_notices() {
    let notices = include_str!("../../../THIRD_PARTY_LICENSES");

    assert!(notices.contains("Copyright © 2016–2024 Sam Hocevar <sam@hocevar.net>"));
    assert!(notices.contains("Do What the Fuck You Want"));
    assert!(notices.contains("fake-08"));
    assert!(notices.contains("Permission is hereby granted, free of charge"));
    assert!(notices.contains("winit"));
}
