//! The undo history: one linear list of entries for the whole Project.
//!
//! `App::apply` brackets every action with `begin` and `finish`, so any action that changes the
//! Project becomes one entry without having to ask for it.

use super::focus::Focus;
use super::panes::chain::ChainCursorPlace;
use super::panes::phrase::PhraseCursorPlace;
use super::panes::song::SongCursorPlace;
use crate::project::{Project, ProjectChange};
use std::collections::VecDeque;

/// The most entries the history keeps.
const MAX_UNDO_ENTRIES: usize = 1000;

/// Where the cursor was when a change was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CursorPlace {
    pub(super) focus: Focus,
    pub(super) song: SongCursorPlace,
    pub(super) chain: ChainCursorPlace,
    pub(super) phrase: PhraseCursorPlace,
    /// The channel under the Overview's cursor.
    pub(super) overview: usize,
}

/// The repeated actions that merge into one entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Merge {
    Nudge,
    Digit,
}

#[derive(Debug)]
struct UndoEntry {
    change: ProjectChange,
    place: CursorPlace,
}

/// A run of repeated actions that is still merging.
#[derive(Debug)]
struct Run {
    merge: Merge,
    /// The Project as it was before the run began.
    before: Project,
    place: CursorPlace,
    /// Where the cursor was left by the latest action of the run. The run ends if it moves.
    left_at: CursorPlace,
    /// Whether the run has left an entry on the history.
    has_entry: bool,
}

/// Every entry that can be undone or redone.
#[derive(Debug, Default)]
pub(super) struct History {
    undo: VecDeque<UndoEntry>,
    redo: Vec<UndoEntry>,
    run: Option<Run>,
    /// The Project and the cursor from before the action being applied.
    pending: Option<(Project, CursorPlace)>,
}

impl History {
    /// Forgets everything, because another Project has opened.
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Notes the Project and the cursor as an action starts.
    pub(super) fn begin(&mut self, project: &Project, place: CursorPlace) {
        self.pending = Some((project.clone(), place));
    }

    /// Records what the action did, if anything. A repeat of the action that came just before it
    /// joins that action's entry, as long as the cursor has stayed where that action left it.
    pub(super) fn finish(&mut self, project: &Project, left_at: CursorPlace, merge: Option<Merge>) {
        let Some((before, place)) = self.pending.take() else {
            return;
        };
        let run = self
            .run
            .take()
            .filter(|run| Some(run.merge) == merge && run.left_at == place);
        match run {
            Some(run) => self.continue_run(run, project, left_at),
            None => self.start_run(before, (place, left_at), project, merge),
        }
    }

    fn continue_run(&mut self, mut run: Run, project: &Project, left_at: CursorPlace) {
        run.left_at = left_at;
        if run.has_entry {
            self.undo.pop_back();
        }
        let change = ProjectChange::between(&run.before, project);
        run.has_entry = !change.is_empty();
        if run.has_entry {
            self.push(change, run.place);
        }
        self.run = Some(run);
    }

    fn start_run(
        &mut self,
        before: Project,
        (place, left_at): (CursorPlace, CursorPlace),
        project: &Project,
        merge: Option<Merge>,
    ) {
        let change = ProjectChange::between(&before, project);
        let has_entry = !change.is_empty();
        if has_entry {
            self.push(change, place);
        }
        self.run = merge.map(|merge| Run {
            merge,
            before,
            place,
            left_at,
            has_entry,
        });
    }

    fn push(&mut self, change: ProjectChange, place: CursorPlace) {
        self.redo.clear();
        self.undo.push_back(UndoEntry { change, place });
        if self.undo.len() > MAX_UNDO_ENTRIES {
            self.undo.pop_front();
        }
    }

    /// Takes back the latest entry, and says where it was made.
    pub(super) fn undo(&mut self, project: &mut Project) -> Option<CursorPlace> {
        self.pending = None;
        self.run = None;
        let entry = self.undo.pop_back()?;
        entry.change.undo(project);
        let place = entry.place;
        self.redo.push(entry);
        Some(place)
    }

    /// Makes the latest undone entry again, and says where it was made.
    pub(super) fn redo(&mut self, project: &mut Project) -> Option<CursorPlace> {
        self.pending = None;
        self.run = None;
        let entry = self.redo.pop()?;
        entry.change.redo(project);
        let place = entry.place;
        self.undo.push_back(entry);
        Some(place)
    }
}
