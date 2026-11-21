//! The cull state machine: one photo, one decision, nothing moved until Apply.
//!
//! The transition table is normative in `docs/BEHAVIOUR.md` §2. This crate owns the session — the
//! queue, the cursor, the staged decisions and the summary — but never touches the filesystem or
//! the catalog, so a session is testable without either.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod gesture;
pub mod trash;

use serde::{Deserialize, Serialize};
use sortlens_core::{Flag, PhotoId};
use std::collections::BTreeMap;

/// What the photographer decided about one photo in this session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    /// Not judged yet.
    #[default]
    Unjudged,
    /// Keep it.
    Pick,
    /// Cut it. Staged only: the file stays on disk until Apply.
    Reject,
    /// Look at it again later. Leaves the photo's flag alone.
    Deferred,
}

impl Decision {
    /// Every decision, in keyboard order.
    pub const ALL: [Self; 4] = [Self::Unjudged, Self::Pick, Self::Reject, Self::Deferred];

    /// Stable name for the command layer.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Unjudged => "unjudged",
            Self::Pick => "pick",
            Self::Reject => "reject",
            Self::Deferred => "deferred",
        }
    }

    /// Parse a name from the command layer.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|decision| decision.id() == value)
    }

    /// The flag this decision sets, if it sets one.
    ///
    /// `Deferred` deliberately returns `None`: skipping a photo must not clear a pick it already
    /// had, and `Unjudged` clears the flag explicitly.
    pub const fn flag(self) -> Option<Flag> {
        match self {
            Self::Unjudged => Some(Flag::None),
            Self::Pick => Some(Flag::Pick),
            Self::Reject => Some(Flag::Reject),
            Self::Deferred => None,
        }
    }
}

/// What changed when a decision was staged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staged {
    /// The decision is a real change and will be applied.
    Changed,
    /// The photo already had that flag: nothing will be written.
    Unchanged,
    /// Deferred: recorded in the session, but it changes no flag.
    Deferred,
}

/// Counters for the action bar and the summary sheet.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// Photos in the session.
    pub total: usize,
    /// Photos carrying a pick decision.
    pub picks: usize,
    /// Photos carrying a reject decision; these are what Apply sends to the bin.
    pub rejects: usize,
    /// Photos deferred to a later pass.
    pub deferred: usize,
    /// Photos not judged yet.
    pub unjudged: usize,
}

impl Summary {
    /// Whether every photo has a decision.
    pub const fn is_complete(&self) -> bool {
        self.unjudged == 0
    }

    /// Photos that Apply would act on.
    pub const fn changed(&self) -> usize {
        self.picks + self.rejects
    }

    /// One line for the summary sheet.
    pub fn headline(&self) -> String {
        if self.total == 0 {
            return "Nothing to cull".to_string();
        }
        if self.is_complete() {
            format!("Judged all {} photos", self.total)
        } else {
            format!("{} of {} judged", self.total - self.unjudged, self.total)
        }
    }
}

/// A saved session. The version lets a future format change be rejected instead of misread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Format version.
    pub version: u32,
    /// The queue in the order the user will see it.
    pub queue: Vec<PhotoId>,
    /// Where the cursor was.
    pub cursor: usize,
    /// Decisions staged so far.
    pub decisions: Vec<(PhotoId, Decision)>,
    /// The flags each photo had when the session began.
    pub original: Vec<(PhotoId, Flag)>,
}

/// Current snapshot version.
pub const SNAPSHOT_VERSION: u32 = 1;

/// A cull session over a fixed queue of photos.
#[derive(Debug, Clone)]
pub struct Session {
    queue: Vec<PhotoId>,
    cursor: usize,
    decisions: BTreeMap<PhotoId, Decision>,
    original: BTreeMap<PhotoId, Flag>,
}

impl Session {
    /// Start a session over a queue, with every photo in its pre-existing flag state.
    ///
    /// The queue order is the order the user will see; nothing here reorders or filters it.
    pub fn new(queue: Vec<PhotoId>, flags: impl IntoIterator<Item = (PhotoId, Flag)>) -> Self {
        let original: BTreeMap<PhotoId, Flag> = flags.into_iter().collect();
        Self {
            queue,
            cursor: 0,
            decisions: BTreeMap::new(),
            original,
        }
    }

    /// A serialisable copy of the session, for crash resume.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            version: SNAPSHOT_VERSION,
            queue: self.queue.clone(),
            cursor: self.cursor,
            decisions: self.decisions.iter().map(|(id, decision)| (*id, *decision)).collect(),
            original: self.original.iter().map(|(id, flag)| (*id, *flag)).collect(),
        }
    }

    /// Rebuild a session from a snapshot. `None` when the snapshot is from another format version or
    /// has no photos, so a half-written or future file cannot start a bogus session.
    pub fn restore(snapshot: Snapshot) -> Option<Self> {
        if snapshot.version != SNAPSHOT_VERSION || snapshot.queue.is_empty() {
            return None;
        }
        let cursor = snapshot.cursor.min(snapshot.queue.len().saturating_sub(1));
        Some(Self {
            queue: snapshot.queue,
            cursor,
            decisions: snapshot.decisions.into_iter().collect(),
            original: snapshot.original.into_iter().collect(),
        })
    }

    /// Drop queued photos that no longer exist, and clamp the cursor.
    pub fn retain(&mut self, keep: impl Fn(PhotoId) -> bool) {
        let anchor = self.current();
        self.queue.retain(|id| keep(*id));
        let queue = self.queue.clone();
        self.decisions.retain(|id, _| queue.contains(id));
        // Stay on the same photo when it survived; otherwise clamp.
        self.cursor = anchor
            .and_then(|id| queue.iter().position(|queued| *queued == id))
            .unwrap_or_else(|| self.cursor.min(self.queue.len().saturating_sub(1)));
    }

    /// The photo under the cursor.
    pub fn current(&self) -> Option<PhotoId> {
        self.queue.get(self.cursor).copied()
    }

    /// Position in the queue, zero-based.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Queue length.
    pub fn total(&self) -> usize {
        self.queue.len()
    }

    /// The queue, in order.
    pub fn queue(&self) -> &[PhotoId] {
        &self.queue
    }

    /// The decision stored for a photo, defaulting to `Unjudged`.
    pub fn decision(&self, id: PhotoId) -> Decision {
        self.decisions.get(&id).copied().unwrap_or_default()
    }

    /// The photo's flag before the session started.
    pub fn original_flag(&self, id: PhotoId) -> Flag {
        self.original.get(&id).copied().unwrap_or_default()
    }

    /// Stage a decision for one photo without moving the cursor.
    pub fn stage(&mut self, id: PhotoId, decision: Decision) -> Staged {
        // Not part of this session: remember the flag lazily so the comparison stays honest.
        self.original.entry(id).or_insert(Flag::None);
        let previous = self.decision(id);
        self.decisions.insert(id, decision);
        match decision.flag() {
            // A decision that sets a flag is a change exactly when the flag differs from the one the
            // photo arrived with. Re-picking a pick writes nothing.
            Some(flag) if flag == self.original_flag(id) => Staged::Unchanged,
            Some(_) => Staged::Changed,
            // Deferring touches no flag: it is only remembered as a visit.
            None if decision == previous => Staged::Unchanged,
            None => Staged::Deferred,
        }
    }

    /// Stage a decision for the current photo and move on.
    ///
    /// Returns the new current photo, or `None` at the end of the queue. Pressing the same key twice
    /// is allowed: the first press advances past the photo, so the second applies to the next one.
    pub fn decide(&mut self, decision: Decision) -> Option<PhotoId> {
        if let Some(id) = self.current() {
            self.stage(id, decision);
            self.cursor = (self.cursor + 1).min(self.queue.len());
        }
        self.current()
    }

    /// Move back one photo, keeping every decision.
    pub fn back(&mut self) -> Option<PhotoId> {
        self.cursor = self.cursor.saturating_sub(1);
        self.current()
    }

    /// Jump to a photo in the queue.
    pub fn jump_to(&mut self, id: PhotoId) -> bool {
        match self.queue.iter().position(|queued| *queued == id) {
            Some(index) => {
                self.cursor = index;
                true
            }
            None => false,
        }
    }

    /// Move to the first photo that has no decision yet, if there is one.
    pub fn next_unjudged(&mut self) -> Option<PhotoId> {
        match self.queue.iter().position(|id| self.decision(*id) == Decision::Unjudged) {
            Some(index) => {
                self.cursor = index;
                self.current()
            }
            None => None,
        }
    }

    /// Photos whose decision changes their flag, in queue order: what Apply will act on.
    pub fn changed(&self) -> Vec<(PhotoId, Decision)> {
        self.queue
            .iter()
            .filter_map(|id| {
                // Only an explicit decision can change anything; an unjudged photo keeps its flag
                // even when that flag is a pick or a reject from an earlier session.
                let decision = *self.decisions.get(id)?;
                (decision.flag().is_some_and(|flag| flag != self.original_flag(*id))).then_some((*id, decision))
            })
            .collect()
    }

    /// Photos staged for rejection, in queue order.
    pub fn rejects(&self) -> Vec<PhotoId> {
        self.changed()
            .into_iter()
            .filter(|(_, decision)| *decision == Decision::Reject)
            .map(|(id, _)| id)
            .collect()
    }

    /// The summary the action bar shows.
    pub fn summary(&self) -> Summary {
        let mut summary = Summary {
            total: self.queue.len(),
            ..Summary::default()
        };
        for id in &self.queue {
            match self.decision(*id) {
                Decision::Unjudged => summary.unjudged += 1,
                Decision::Pick => summary.picks += 1,
                Decision::Reject => summary.rejects += 1,
                Decision::Deferred => summary.deferred += 1,
            }
        }
        summary
    }

    /// Throw the session's decisions away. Nothing was written, so nothing needs undoing.
    pub fn clear(&mut self) {
        self.decisions.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new(
            vec![PhotoId(1), PhotoId(2), PhotoId(3)],
            [(PhotoId(1), Flag::None), (PhotoId(2), Flag::Pick), (PhotoId(3), Flag::Reject)],
        )
    }

    #[test]
    fn a_session_starts_at_the_first_photo_with_nothing_staged() {
        let session = session();
        assert_eq!(session.current(), Some(PhotoId(1)));
        assert_eq!(session.summary().unjudged, 3);
        assert!(session.changed().is_empty());
    }

    #[test]
    fn the_transition_table_is_a_partition() {
        // Every source state, every decision: exactly one destination, and the counts add up.
        for from in Decision::ALL {
            for to in Decision::ALL {
                let mut session = Session::new(vec![PhotoId(1)], [(PhotoId(1), Flag::None)]);
                session.stage(PhotoId(1), from);
                session.stage(PhotoId(1), to);
                assert_eq!(session.decision(PhotoId(1)), to);
                let summary = session.summary();
                assert_eq!(summary.picks + summary.rejects + summary.deferred + summary.unjudged, 1);
            }
        }
    }

    #[test]
    fn deciding_advances_and_records() {
        let mut session = session();
        assert_eq!(session.decide(Decision::Pick), Some(PhotoId(2)));
        assert_eq!(session.decision(PhotoId(1)), Decision::Pick);
        assert_eq!(session.cursor(), 1);
    }

    #[test]
    fn back_keeps_the_decisions() {
        let mut session = session();
        session.decide(Decision::Reject);
        session.decide(Decision::Pick);
        assert_eq!(session.back(), Some(PhotoId(2)));
        assert_eq!(session.back(), Some(PhotoId(1)));
        assert_eq!(session.decision(PhotoId(1)), Decision::Reject);
        assert_eq!(session.summary().rejects, 1);
    }

    #[test]
    fn re_applying_the_same_value_writes_nothing() {
        let mut session = session();
        // Photo 2 is already a pick; deciding pick again must not appear in the change set.
        session.jump_to(PhotoId(2));
        assert_eq!(session.stage(PhotoId(2), Decision::Pick), Staged::Unchanged);
        assert!(session.changed().is_empty());
    }

    #[test]
    fn skipping_preserves_the_existing_flag() {
        let mut session = session();
        session.jump_to(PhotoId(2));
        assert_eq!(session.stage(PhotoId(2), Decision::Deferred), Staged::Deferred);
        assert_eq!(session.original_flag(PhotoId(2)), Flag::Pick);
        assert!(session.changed().is_empty(), "deferring changes no flag");
        assert_eq!(session.summary().deferred, 1);
    }

    #[test]
    fn clearing_a_flag_is_a_change() {
        let mut session = session();
        session.jump_to(PhotoId(3));
        assert_eq!(session.stage(PhotoId(3), Decision::Unjudged), Staged::Changed);
        assert_eq!(session.changed(), vec![(PhotoId(3), Decision::Unjudged)]);
    }

    #[test]
    fn rejects_are_staged_in_queue_order_and_never_applied_here() {
        let mut session = session();
        session.jump_to(PhotoId(1));
        session.decide(Decision::Reject);
        session.decide(Decision::Deferred);
        session.decide(Decision::Reject);
        // Photo 3 was already a reject, so re-rejecting it is a no-op and is not staged.
        assert_eq!(session.rejects(), vec![PhotoId(1)]);
        assert_eq!(session.total(), 3, "the queue is unchanged");
    }

    #[test]
    fn the_summary_reaches_completion_only_when_nothing_is_unjudged() {
        let mut session = session();
        while session.current().is_some() {
            session.decide(Decision::Deferred);
        }
        let summary = session.summary();
        assert!(summary.is_complete());
        assert_eq!(summary.deferred, 3);
        assert_eq!(summary.changed(), 0);
        assert_eq!(session.current(), None);
    }

    #[test]
    fn next_unjudged_finds_the_gap() {
        let mut session = session();
        session.jump_to(PhotoId(1));
        session.decide(Decision::Pick);
        session.decide(Decision::Deferred);
        session.decide(Decision::Deferred);
        session.decide(Decision::Pick);
        session.jump_to(PhotoId(1));
        session.stage(PhotoId(1), Decision::Unjudged);
        assert_eq!(session.next_unjudged(), Some(PhotoId(1)));
    }

    #[test]
    fn clearing_the_session_forgets_everything() {
        let mut session = session();
        session.decide(Decision::Reject);
        session.clear();
        assert_eq!(session.cursor(), 0);
        assert_eq!(session.summary().unjudged, 3);
    }

    #[test]
    fn an_empty_queue_has_nothing_to_do() {
        let session = Session::new(Vec::new(), []);
        assert_eq!(session.current(), None);
        assert!(session.summary().is_complete());
    }

    #[test]
    fn a_snapshot_restores_cursor_and_decisions() {
        let mut original = session();
        original.decide(Decision::Reject);
        let snapshot = original.snapshot();
        let restored = Session::restore(snapshot).expect("restore");
        assert_eq!(restored.cursor(), 1);
        assert_eq!(restored.decision(PhotoId(1)), Decision::Reject);
        assert_eq!(restored.original_flag(PhotoId(2)), Flag::Pick);
        assert_eq!(restored.summary(), original.summary());
    }

    #[test]
    fn a_future_snapshot_is_refused() {
        let mut snapshot = session().snapshot();
        snapshot.version = SNAPSHOT_VERSION + 1;
        assert!(Session::restore(snapshot).is_none());
        let empty = Session::new(Vec::new(), []).snapshot();
        assert!(Session::restore(empty).is_none());
    }

    #[test]
    fn restore_clamps_a_cursor_past_the_end() {
        let mut snapshot = session().snapshot();
        snapshot.cursor = 99;
        let restored = Session::restore(snapshot).expect("restore");
        assert_eq!(restored.cursor(), 2);
        assert_eq!(restored.current(), Some(PhotoId(3)));
    }

    #[test]
    fn retain_drops_photos_that_are_gone() {
        let mut session = session();
        session.decide(Decision::Reject);
        session.retain(|id| id != PhotoId(1));
        assert_eq!(session.queue(), &[PhotoId(2), PhotoId(3)]);
        assert_eq!(session.summary().rejects, 0);
        assert_eq!(session.current(), Some(PhotoId(2)));
    }

    #[test]
    fn decisions_parse_back_from_their_names() {
        for decision in Decision::ALL {
            assert_eq!(Decision::parse(decision.id()), Some(decision));
        }
        assert_eq!(Decision::parse("nope"), None);
    }
}
