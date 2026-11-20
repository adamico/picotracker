//! What the app tells the user: results for the status line and items for the Log.

use crate::project::SongPlace;
use std::time::Duration;

const INFO_LIFETIME: Duration = Duration::from_secs(3);
const INFO_BRIGHT: Duration = Duration::from_secs(2);
const LOG_POINTER: &str = "  ^L LOG";
const ATTENTION_MARK: &str = "! ";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Failure,
}

/// One line of work in the Log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogItem {
    pub(super) severity: Severity,
    pub(super) text: String,
    pub(super) hint: Option<String>,
    pub(super) location: Option<SongPlace>,
}

impl LogItem {
    pub fn new(severity: Severity, text: impl Into<String>) -> Self {
        Self {
            severity,
            text: text.into(),
            hint: None,
            location: None,
        }
    }

    /// Adds what the user can do about this item, such as `^T EDIT CLAIM`.
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Adds the place that Enter jumps to.
    #[must_use]
    pub fn at(mut self, location: SongPlace) -> Self {
        self.location = Some(location);
        self
    }
}

/// The outcome of something the user asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub(super) severity: Severity,
    pub(super) summary: String,
    pub(super) items: Vec<LogItem>,
}

impl Report {
    fn new(severity: Severity, summary: impl Into<String>) -> Self {
        Self {
            severity,
            summary: summary.into(),
            items: Vec::new(),
        }
    }

    /// A short result that fades from the status line.
    pub fn info(summary: impl Into<String>) -> Self {
        Self::new(Severity::Info, summary)
    }

    /// A result that stays on the status line in orange until Esc.
    pub fn warning(summary: impl Into<String>) -> Self {
        Self::new(Severity::Warning, summary)
    }

    /// A result that stays on the status line in red until Esc.
    pub fn failure(summary: impl Into<String>) -> Self {
        Self::new(Severity::Failure, summary)
    }

    /// Adds the items to work through in the Log, which opens to show them.
    #[must_use]
    pub fn with_items(mut self, items: Vec<LogItem>) -> Self {
        self.items = items;
        self
    }

    /// The text for the status line.
    pub(super) fn status_text(&self) -> String {
        let mark = if self.severity == Severity::Info {
            ""
        } else {
            ATTENTION_MARK
        };
        let pointer = if self.items.is_empty() {
            ""
        } else {
            LOG_POINTER
        };
        format!("{mark}{}{pointer}", self.summary)
    }

    /// What the Log keeps of this result: its items, or else its summary.
    pub(super) fn into_log_items(self) -> Vec<LogItem> {
        if self.items.is_empty() {
            vec![LogItem::new(self.severity, self.summary)]
        } else {
            self.items
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Look {
    Info,
    FadingInfo,
    Warning,
    Failure,
}

/// The result showing on the status line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Notice {
    severity: Severity,
    text: String,
    age: Duration,
}

impl Notice {
    pub(super) fn new(severity: Severity, text: String) -> Self {
        Self {
            severity,
            text,
            age: Duration::ZERO,
        }
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn look(&self) -> Look {
        match self.severity {
            Severity::Info if self.age < INFO_BRIGHT => Look::Info,
            Severity::Info => Look::FadingInfo,
            Severity::Warning => Look::Warning,
            Severity::Failure => Look::Failure,
        }
    }

    /// Whether the notice stays until the user dismisses it.
    pub(super) fn is_persistent(&self) -> bool {
        self.severity != Severity::Info
    }

    /// Lets time pass, and says whether the notice has now gone.
    pub(super) fn grow_older(&mut self, elapsed: Duration) -> bool {
        self.age += elapsed;
        !self.is_persistent() && self.age >= INFO_LIFETIME
    }

    /// How long until the notice next looks different, if it ever will by itself.
    pub(super) fn time_until_change(&self) -> Option<Duration> {
        if self.is_persistent() {
            None
        } else if self.age < INFO_BRIGHT {
            Some(INFO_BRIGHT - self.age)
        } else {
            Some(INFO_LIFETIME.saturating_sub(self.age))
        }
    }
}
