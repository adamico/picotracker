//! What one edit did to a Project, kept small enough that a long undo history stays cheap.

use super::{Chain, ChainId, CustomInstrument, EXPORT_CHANNELS, Phrase, Project, TRACKS, TargetCart};

/// A value before and after an edit.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Swap<T> {
    before: T,
    after: T,
}

impl<T: Clone + PartialEq> Swap<T> {
    /// The two values, if they differ.
    fn between(before: &T, after: &T) -> Option<Self> {
        (before != after).then(|| Self {
            before: before.clone(),
            after: after.clone(),
        })
    }

    fn value(&self, way: Way) -> &T {
        match way {
            Way::Back => &self.before,
            Way::Forward => &self.after,
        }
    }
}

/// Which way a change is being applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Way {
    Back,
    Forward,
}

/// Every difference between two versions of a Project.
///
/// The claimed-bytes hash is left out: Export updates it, and Export is not an edit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ProjectChange {
    phrases: Vec<(usize, Swap<Phrase>)>,
    chains: Vec<(usize, Swap<Chain>)>,
    song_rows: Vec<(usize, Swap<[Option<ChainId>; TRACKS]>)>,
    song_loop_row: Option<Swap<Option<usize>>>,
    default_speed: Option<Swap<u8>>,
    export_tracks: Option<Swap<[Option<u8>; EXPORT_CHANNELS]>>,
    target_cart: Option<Swap<Option<TargetCart>>>,
    custom_instruments: Vec<(u8, Swap<Option<CustomInstrument>>)>,
}

/// The items that differ between two equally long lists, with their positions.
fn changed_items<T: Clone + PartialEq>(before: &[T], after: &[T]) -> Vec<(usize, Swap<T>)> {
    before
        .iter()
        .zip(after)
        .enumerate()
        .filter_map(|(index, (before, after))| Some((index, Swap::between(before, after)?)))
        .collect()
}

/// A Target Cart with the claimed-bytes hash taken out.
fn without_hash(target_cart: Option<&TargetCart>) -> Option<TargetCart> {
    target_cart.cloned().map(|mut target_cart| {
        target_cart.claimed_bytes_hash = None;
        target_cart
    })
}

impl ProjectChange {
    /// What it takes to turn one version of a Project into another.
    pub(crate) fn between(before: &Project, after: &Project) -> Self {
        let target_cart = Swap::between(
            &without_hash(before.target_cart.as_ref()),
            &without_hash(after.target_cart.as_ref()),
        )
        .map(|_| Swap {
            before: before.target_cart.clone(),
            after: after.target_cart.clone(),
        });
        let instrument_ids = before
            .custom_instruments
            .keys()
            .chain(after.custom_instruments.keys());
        let mut custom_instruments: Vec<_> = instrument_ids
            .filter_map(|id| {
                let swap = Swap::between(
                    &before.custom_instruments.get(id).cloned(),
                    &after.custom_instruments.get(id).cloned(),
                )?;
                Some((*id, swap))
            })
            .collect();
        custom_instruments.dedup_by_key(|(id, _)| *id);
        Self {
            phrases: changed_items(&before.phrases, &after.phrases),
            chains: changed_items(&before.chains, &after.chains),
            song_rows: changed_items(&before.song, &after.song),
            song_loop_row: Swap::between(&before.song_loop_row, &after.song_loop_row),
            default_speed: Swap::between(&before.default_speed, &after.default_speed),
            export_tracks: Swap::between(&before.export_tracks, &after.export_tracks),
            target_cart,
            custom_instruments,
        }
    }

    /// Whether the two versions hold the same music.
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Takes the change out of a Project.
    pub(crate) fn undo(&self, project: &mut Project) {
        self.apply(project, Way::Back);
    }

    /// Makes the change to a Project again.
    pub(crate) fn redo(&self, project: &mut Project) {
        self.apply(project, Way::Forward);
    }

    fn apply(&self, project: &mut Project, way: Way) {
        for (index, swap) in &self.phrases {
            project.phrases[*index] = *swap.value(way);
        }
        for (index, swap) in &self.chains {
            project.chains[*index] = *swap.value(way);
        }
        for (index, swap) in &self.song_rows {
            project.song[*index] = *swap.value(way);
        }
        if let Some(swap) = &self.song_loop_row {
            project.song_loop_row = *swap.value(way);
        }
        if let Some(swap) = &self.default_speed {
            project.default_speed = *swap.value(way);
        }
        if let Some(swap) = &self.export_tracks {
            project.export_tracks = *swap.value(way);
        }
        if let Some(swap) = &self.target_cart {
            project.target_cart = keeping_hash(swap.value(way), project.target_cart.as_ref());
        }
        for (id, swap) in &self.custom_instruments {
            project.set_custom_instrument(*id, swap.value(way).clone());
        }
    }
}

/// A Target Cart being put back, holding the hash Export last stored for that same Cart.
fn keeping_hash(restored: &Option<TargetCart>, current: Option<&TargetCart>) -> Option<TargetCart> {
    let mut restored = restored.clone();
    if let (Some(restored), Some(current)) = (restored.as_mut(), current)
        && restored.path == current.path
    {
        restored.claimed_bytes_hash = current.claimed_bytes_hash.clone();
    }
    restored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Claim, InstrumentSound, WAVEFORM_SAMPLES, WaveformInstrument};

    fn cart(sfx: Vec<u8>, hash: &str) -> TargetCart {
        TargetCart {
            path: "game.p8".to_owned(),
            claim: Claim {
                sfx,
                patterns: Some((0, 3)),
            },
            claimed_bytes_hash: Some(hash.to_owned()),
        }
    }

    fn instrument(name: &str) -> CustomInstrument {
        CustomInstrument {
            name: Some(name.to_owned()),
            sound: InstrumentSound::Waveform(WaveformInstrument {
                samples: [0; WAVEFORM_SAMPLES],
                bass: false,
                detune: 0,
                reverb: 0,
                dampen: 0,
            }),
        }
    }

    #[test]
    fn a_claim_edit_and_an_instrument_edit_can_be_undone_and_redone() {
        let mut before = Project::new();
        before.set_target_cart(Some(cart(vec![1], "aa")));
        before.set_custom_instrument(8, Some(instrument("BASS")));
        before.set_custom_instrument(9, Some(instrument("LEAD")));
        let mut after = before.clone();
        after.set_target_cart(Some(cart(vec![1, 2], "aa")));
        after.set_custom_instrument(8, Some(instrument("KICK")));
        after.set_custom_instrument(9, None);
        after.set_custom_instrument(10, Some(instrument("PAD")));

        let change = ProjectChange::between(&before, &after);
        let mut project = after.clone();

        change.undo(&mut project);
        assert!(project == before);
        change.redo(&mut project);
        assert!(project == after);
    }

    #[test]
    fn a_new_claimed_bytes_hash_alone_is_not_a_change() {
        let mut before = Project::new();
        before.set_target_cart(Some(cart(vec![1], "aa")));
        let mut after = before.clone();
        after.set_target_cart(Some(cart(vec![1], "bb")));

        assert!(ProjectChange::between(&before, &after).is_empty());
    }

    #[test]
    fn undoing_a_claim_edit_keeps_the_hash_of_a_later_export() {
        let mut before = Project::new();
        before.set_target_cart(Some(cart(vec![1], "aa")));
        let mut after = before.clone();
        after.set_target_cart(Some(cart(vec![1, 2], "aa")));
        let change = ProjectChange::between(&before, &after);

        let mut exported = after.clone();
        exported.set_target_cart(Some(cart(vec![1, 2], "bb")));
        change.undo(&mut exported);

        assert!(exported.target_cart() == Some(&cart(vec![1], "bb")));
    }
}
