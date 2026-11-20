//! Ctrl+E: reads the Target Cart, compiles the Project into it and writes it back. Claimed bytes
//! changed outside the tracker are overwritten only once the user says so, and a Target Cart that
//! has moved can be located. The claimed-bytes hash is stored only once the Cart is safely written,
//! and never becomes an undo entry.

use super::App;
use super::claim_text::named_slots;
use super::files::{Accepted, OVERWRITE_ANSWERS, Waiting, shown_name};
use super::messages::{LogItem, Report, Severity};
use crate::cart::{Cart, Slots};
use crate::export::{self, ExportFailure, ExportWarning, Exported};
use crate::project::{Claim, EXPORT_CHANNELS, SongPlace};
use crate::wording::counted;
use std::path::{Path, PathBuf};

const EDIT_CLAIM_HINT: &str = "^T EDIT CLAIM";
const LOCATE_ANSWERS: &str = "ENTER LOCATE  ESC CANCEL";

/// An Export whose Cart is being written.
#[derive(Debug)]
pub(super) struct FinishedExport {
    claimed_bytes_hash: String,
    report: Report,
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
fn failure_location(failure: ExportFailure) -> Option<SongPlace> {
    match failure {
        ExportFailure::NoteOutOfRange {
            step,
            track,
            song_row,
            chain_row,
            ..
        } => Some(SongPlace {
            song_row,
            track,
            chain_row,
            step,
        }),
        _ => None,
    }
}

/// Whether a failure is fixed by changing the Claim.
const fn is_about_the_claim(failure: ExportFailure) -> bool {
    matches!(
        failure,
        ExportFailure::EmptyClaim
            | ExportFailure::BadClaim
            | ExportFailure::TooFewSfx { .. }
            | ExportFailure::TooFewPatterns { .. }
            | ExportFailure::TooFewInstrumentSlots { .. }
    )
}

fn failure_item(failure: ExportFailure) -> LogItem {
    let mut item = LogItem::new(Severity::Failure, failure.to_string());
    if is_about_the_claim(failure) {
        item = item.with_hint(EDIT_CLAIM_HINT);
    }
    match failure_location(failure) {
        Some(location) => item.at(location),
        None => item,
    }
}

/// The slots that a Claim still holds.
fn still_claimed(slots: Slots, claim: &Claim) -> Slots {
    let in_pattern_range = |slot: &u8| {
        claim
            .patterns
            .is_some_and(|(first, last)| (first..=last).contains(slot))
    };
    Slots {
        sfx: slots
            .sfx
            .into_iter()
            .filter(|slot| claim.sfx.contains(slot))
            .collect(),
        patterns: slots
            .patterns
            .into_iter()
            .filter(in_pattern_range)
            .collect(),
    }
}

/// Changed slots as the overwrite question names them, such as `SFX 03 04 AND PATTERN 18`.
fn changed_slots_text(changed: &Slots) -> String {
    [
        named_slots(&changed.sfx, "SFX", "SFX"),
        named_slots(&changed.patterns, "PATTERN", "PATTERNS"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" AND ")
}

/// An Export warning as a Log item, which jumps to the place it is about.
fn warning_item(warning: ExportWarning) -> LogItem {
    let item = LogItem::new(Severity::Warning, warning.to_string());
    match warning {
        ExportWarning::UnexportedTrackSetsLength {
            song_row,
            chain_row,
            track,
        } => item.at(SongPlace {
            song_row,
            track,
            chain_row,
            step: 0,
        }),
    }
}

/// The result of an Export that wrote the Cart: its warnings, if any, go to the Log.
fn written_report(summary: String, warnings: &[ExportWarning]) -> Report {
    if warnings.is_empty() {
        return Report::info(summary);
    }
    let count = counted(warnings.len(), "WARNING");
    let items = warnings.iter().copied().map(warning_item).collect();
    Report::warning(format!("{summary} WITH {count}")).with_items(items)
}

/// The result of an Export that failed: every problem goes to the Log.
fn failure_report(cart_name: &str, failures: &[ExportFailure]) -> Report {
    let summary = if failures.contains(&ExportFailure::EmptyClaim) {
        format!("EXPORT FAILED: NO CLAIM IN {cart_name}")
    } else if failures == [ExportFailure::NoExportTracks] {
        "EXPORT FAILED: NO EXPORT TRACKS".to_owned()
    } else {
        format!("EXPORT FAILED: {}", counted(failures.len(), "PROBLEM"))
    };
    let items = failures.iter().copied().map(failure_item).collect();
    Report::failure(summary).with_items(items)
}

impl App {
    /// The folder the Target Cart's path starts from.
    pub(super) fn target_cart_folder(&self) -> &Path {
        self.project_path
            .as_deref()
            .and_then(Path::parent)
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(&self.directory)
    }

    /// Where the Target Cart is: its path is relative to the Project file.
    pub(super) fn target_cart_path(&self, cart: &str) -> PathBuf {
        self.target_cart_folder().join(cart)
    }

    /// Starts an Export by asking for the Target Cart's bytes. With no Target Cart, the Cart view
    /// opens so that the user can pick one.
    pub(super) fn export(&mut self) {
        self.close_cart_view();
        let Some(target) = self.project.target_cart() else {
            self.open_cart_view();
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
        let changed = target
            .claimed_bytes_hash
            .as_deref()
            .zip(Cart::read(original).ok())
            .map(|(hash, cart)| still_claimed(cart.changed_since(hash), &target.claim))
            .unwrap_or_default();
        match export::export(&self.project, &target.claim, original) {
            Ok(exported) if changed == Slots::default() => self.write_target(path, exported),
            Ok(exported) => {
                let text = format!(
                    "{} CHANGED IN {cart_name}  {OVERWRITE_ANSWERS}",
                    changed_slots_text(&changed)
                );
                self.ask(text, Accepted::OverwriteCart(path, Box::new(exported)));
            }
            Err(failures) => self.report(failure_report(&cart_name, &failures)),
        }
    }

    /// Asks whether to look for a Target Cart that is no longer where its path says.
    pub(super) fn ask_to_locate(&mut self, path: &Path) {
        let text = format!("{} NOT FOUND  {LOCATE_ANSWERS}", shown_name(path));
        self.ask(text, Accepted::LocateCart);
    }

    /// Points the Target Cart at the file the user located, keeping the Claim and the
    /// claimed-bytes hash, and Exports to it.
    pub(super) fn locate_target_cart(&mut self, path: &Path) {
        self.file_list = None;
        let Some(mut target) = self.project.target_cart().cloned() else {
            return;
        };
        target.path = self.path_from_project(path);
        self.project.set_target_cart(Some(target));
        self.export();
    }

    pub(super) fn write_target(&mut self, path: PathBuf, exported: Exported) {
        let cart_name = shown_name(&path);
        let summary = format!(
            "EXPORTED TO {cart_name}: {} SFX, {} PATTERNS. {}",
            exported.sfx_count,
            exported.pattern_count,
            tracks_text(self.project.export_tracks()),
        );
        let report = written_report(summary, &exported.warnings);
        self.request_write_bytes(path.clone(), exported.cart_bytes);
        self.waiting = Some(Waiting::ExportWriting(
            path,
            FinishedExport {
                claimed_bytes_hash: exported.claimed_bytes_hash,
                report,
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
        self.report(finished.report);
    }
}
