//! The release archive built by `scripts/package-release.sh`.
#![cfg(target_os = "linux")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn package(version: &str, out_dir: &Path) -> Output {
    let _ = fs::remove_dir_all(out_dir);
    Command::new("bash")
        .arg(repo_root().join("scripts/package-release.sh"))
        .args([version, env!("CARGO_BIN_EXE_picotracker")])
        .arg(out_dir)
        .current_dir(repo_root())
        .output()
        .unwrap()
}

fn out_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(name)
}

#[test]
fn the_archive_holds_the_binary_and_every_release_file() {
    let out = out_dir("archive-contents");
    let packaged = package(VERSION, &out);
    assert!(packaged.status.success(), "{packaged:?}");

    let name = format!("picotracker-{VERSION}-x86_64-linux");
    let listing = Command::new("tar")
        .arg("-tzf")
        .arg(out.join(format!("{name}.tar.gz")))
        .output()
        .unwrap();
    let mut files: Vec<String> = String::from_utf8(listing.stdout)
        .unwrap()
        .lines()
        .filter(|line| !line.ends_with('/'))
        .map(str::to_owned)
        .collect();
    files.sort();

    let mut expected: Vec<String> = [
        "CHANGELOG.md",
        "LICENSE-APACHE",
        "LICENSE-MIT",
        "README.md",
        "THIRD_PARTY_LICENSES",
        "config.example.ron",
        "picotracker",
    ]
    .iter()
    .map(|file| format!("{name}/{file}"))
    .collect();
    expected.sort();
    assert_eq!(files, expected);
}

#[test]
fn sha256sums_verifies_the_archive() {
    let out = out_dir("archive-checksums");
    let packaged = package(VERSION, &out);
    assert!(packaged.status.success(), "{packaged:?}");

    let check = Command::new("sha256sum")
        .args(["--check", "SHA256SUMS"])
        .current_dir(&out)
        .output()
        .unwrap();
    assert!(check.status.success(), "{check:?}");
    assert_eq!(
        String::from_utf8(check.stdout).unwrap(),
        format!("picotracker-{VERSION}-x86_64-linux.tar.gz: OK\n")
    );
}

#[test]
fn release_notes_are_the_changelog_section_for_the_version() {
    let out = out_dir("archive-notes");
    let packaged = package(VERSION, &out);
    assert!(packaged.status.success(), "{packaged:?}");

    let notes = fs::read_to_string(out.join("RELEASE_NOTES.md")).unwrap();
    assert!(!notes.trim().is_empty());
    assert!(!notes.contains(&format!("## {VERSION}")));
    assert!(
        fs::read_to_string(repo_root().join("CHANGELOG.md"))
            .unwrap()
            .contains(notes.trim())
    );
}

#[test]
fn a_version_that_differs_from_the_binary_is_refused() {
    let out = out_dir("archive-wrong-version");
    let packaged = package("99.99.99", &out);

    assert!(!packaged.status.success());
    assert!(
        !out.join("picotracker-99.99.99-x86_64-linux.tar.gz")
            .exists()
    );
}
