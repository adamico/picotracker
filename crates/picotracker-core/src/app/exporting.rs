//! Ctrl+E: reads the Target Cart, compiles the Project into it and writes it back. The claimed-bytes
//! hash is stored only once the Cart is safely written, and never becomes an undo step.

use super::App;
use super::files::{Waiting, shown_name};
use super::messages::{Location, LogItem, Report, Severity};
use crate::export::{self, ExportFailure, Exported};
use crate::project::EXPORT_CHANNELS;
use std::path::{Path, PathBuf};

const NO_TARGET_CART: &str = "EXPORT FAILED: NO TARGET CART";

/// An Export whose Cart is being written.
#[derive(Debug)]
pub(super) struct FinishedExport {
    claimed_bytes_hash: String,
    summary: String,
}

/// The Export Tracks as messages show them: `T1-T4` for a run, or else `T1 T3`.
fn tracks_text(tracks: [Option<u8>; EXPORT_CHANNELS]) -> String {
    let mut tracks: Vec<u8> = tracks.into_iter().flatten().collect();
    tracks.sort_unstable();
    let is_run = tracks.windows(2).all(|pair| pair[0] + 1 == pair[1]);
    match (tracks.first(), tracks.last()) {
        (Some(first), Some(last)) if is_run && first != last => format!("T{first}-T{last}"),
        _ => tracks
            .iter()
            .map(|track| format!("T{track}"))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Where a failure can be fixed, if it has one place.
fn failure_location(failure: ExportFailure) -> Option<Location> {
    match failure {
        ExportFailure::NoteOutOfRange {
            step,
            track,
            song_row,
            chain_row,
            ..
        } => Some(Location {
            song_row,
            track,
            chain_row,
            step,
        }),
        ExportFailure::NotSimple {
            song_row,
            chain_row,
        } => Some(Location {
            song_row,
            track: 0,
            chain_row,
            step: 0,
        }),
        _ => None,
    }
}

fn failure_item(failure: ExportFailure) -> LogItem {
    let item = LogItem::new(Severity::Failure, failure.to_string());
    match failure_location(failure) {
        Some(location) => item.at(location),
        None => item,
    }
}

/// The result of an Export that failed: every problem goes to the Log.
fn failure_report(cart_name: &str, failures: &[ExportFailure]) -> Report {
    let summary = if failures.contains(&ExportFailure::EmptyClaim) {
        format!("EXPORT FAILED: NO CLAIM IN {cart_name}")
    } else if failures == [ExportFailure::NoExportTracks] {
        "EXPORT FAILED: NO EXPORT TRACKS".to_owned()
    } else {
        let count = failures.len();
        let plural = if count == 1 { "" } else { "S" };
        format!("EXPORT FAILED: {count} PROBLEM{plural}")
    };
    let items = failures.iter().copied().map(failure_item).collect();
    Report::failure(summary).with_items(items)
}

impl App {
    /// Where the Target Cart is: its path is relative to the Project file.
    fn target_cart_path(&self, cart: &str) -> PathBuf {
        let folder = self
            .project_path
            .as_deref()
            .and_then(Path::parent)
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(&self.directory);
        folder.join(cart)
    }

    /// Starts an Export by asking for the Target Cart's bytes.
    pub(super) fn export(&mut self) {
        let Some(target) = self.project.target_cart() else {
            self.report(Report::failure(NO_TARGET_CART));
            return;
        };
        let path = self.target_cart_path(&target.path);
        self.request_bytes(path.clone());
        self.waiting = Some(Waiting::ExportReading(path));
    }

    /// Compiles the Project into the Target Cart's bytes, and writes them if nothing failed.
    pub(super) fn finish_reading_target(&mut self, path: PathBuf, original: &[u8]) {
        let cart_name = shown_name(&path);
        let Some(target) = self.project.target_cart() else {
            return;
        };
        match export::export(&self.project, &target.claim, original) {
            Ok(exported) => self.write_target(path, &cart_name, exported),
            Err(failures) => self.report(failure_report(&cart_name, &failures)),
        }
    }

    fn write_target(&mut self, path: PathBuf, cart_name: &str, exported: Exported) {
        let summary = format!(
            "EXPORTED TO {cart_name}: {} SFX, {} PATTERNS. {}",
            exported.sfx_count,
            exported.pattern_count,
            tracks_text(self.project.export_tracks()),
        );
        self.request_write_bytes(path.clone(), exported.cart_bytes);
        self.waiting = Some(Waiting::ExportWriting(
            path,
            FinishedExport {
                claimed_bytes_hash: exported.claimed_bytes_hash,
                summary,
            },
        ));
    }

    /// The Cart is written: remembers what it now holds. This is not an edit, so it happens
    /// outside the undo history.
    pub(super) fn finish_writing_target(&mut self, finished: FinishedExport) {
        if let Some(target) = self.project.target_cart() {
            let mut target = target.clone();
            target.claimed_bytes_hash = Some(finished.claimed_bytes_hash);
            self.project.set_target_cart(Some(target));
        }
        self.report(Report::info(finished.summary));
    }
}
