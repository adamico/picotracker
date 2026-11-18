//! The undo history: one linear list of steps for the whole Project.
//!
//! `App::apply` brackets every action with `begin` and `finish`, so any action that changes the
//! Project becomes one step without having to ask for it.

use super::focus::Focus;
use super::panes::chain::ChainSpot;
use super::panes::phrase::PhraseSpot;
use super::panes::song::SongSpot;
use crate::project::{Project, ProjectChange};
use std::collections::VecDeque;

/// The most steps the history keeps.
const MAX_STEPS: usize = 1000;

/// Where the cursor was when a change was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Spot {
    pub(super) focus: Focus,
    pub(super) song: SongSpot,
    pub(super) chain: ChainSpot,
    pub(super) phrase: PhraseSpot,
}

/// The repeated actions that merge into one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Merge {
    Nudge,
    Digit,
}

#[derive(Debug)]
struct Step {
    change: ProjectChange,
    spot: Spot,
}

/// A run of repeated actions that is still merging.
#[derive(Debug)]
struct Run {
    merge: Merge,
    /// The Project as it was before the run began.
    before: Project,
    spot: Spot,
    /// Where the cursor was left by the latest action of the run. The run ends if it moves.
    left_at: Spot,
    /// Whether the run has left a step on the history.
    has_step: bool,
}

/// Every step that can be undone or redone.
#[derive(Debug, Default)]
pub(super) struct History {
    undo: VecDeque<Step>,
    redo: Vec<Step>,
    run: Option<Run>,
    /// The Project and the cursor from before the action being applied.
    pending: Option<(Project, Spot)>,
}

impl History {
    /// Forgets everything, because another Project has opened.
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Notes the Project and the cursor as an action starts.
    pub(super) fn begin(&mut self, project: &Project, spot: Spot) {
        self.pending = Some((project.clone(), spot));
    }

    /// Records what the action did, if anything. A repeat of the action that came just before it
    /// joins that action's step, as long as the cursor has stayed where that action left it.
    pub(super) fn finish(&mut self, project: &Project, left_at: Spot, merge: Option<Merge>) {
        let Some((before, spot)) = self.pending.take() else {
            return;
        };
        let run = self
            .run
            .take()
            .filter(|run| Some(run.merge) == merge && run.left_at == spot);
        match run {
            Some(run) => self.continue_run(run, project, left_at),
            None => self.start_run(before, (spot, left_at), project, merge),
        }
    }

    fn continue_run(&mut self, mut run: Run, project: &Project, left_at: Spot) {
        run.left_at = left_at;
        if run.has_step {
            self.undo.pop_back();
        }
        let change = ProjectChange::between(&run.before, project);
        run.has_step = !change.is_empty();
        if run.has_step {
            self.push(change, run.spot);
        }
        self.run = Some(run);
    }

    fn start_run(
        &mut self,
        before: Project,
        (spot, left_at): (Spot, Spot),
        project: &Project,
        merge: Option<Merge>,
    ) {
        let change = ProjectChange::between(&before, project);
        let has_step = !change.is_empty();
        if has_step {
            self.push(change, spot);
        }
        self.run = merge.map(|merge| Run {
            merge,
            before,
            spot,
            left_at,
            has_step,
        });
    }

    fn push(&mut self, change: ProjectChange, spot: Spot) {
        self.redo.clear();
        self.undo.push_back(Step { change, spot });
        if self.undo.len() > MAX_STEPS {
            self.undo.pop_front();
        }
    }

    /// Takes back the latest step, and says where it was made.
    pub(super) fn undo(&mut self, project: &mut Project) -> Option<Spot> {
        self.pending = None;
        self.run = None;
        let step = self.undo.pop_back()?;
        step.change.undo(project);
        let spot = step.spot;
        self.redo.push(step);
        Some(spot)
    }

    /// Makes the latest undone step again, and says where it was made.
    pub(super) fn redo(&mut self, project: &mut Project) -> Option<Spot> {
        self.pending = None;
        self.run = None;
        let step = self.redo.pop()?;
        step.change.redo(project);
        let spot = step.spot;
        self.undo.push_back(step);
        Some(spot)
    }
}
