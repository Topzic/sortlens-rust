//! The page's state machine, in a form its tests can run.
//!
//! A browser page is a device (SL-J02): it holds no library of its own, it speaks the paired
//! host's protocol, and what it keeps between sessions is exactly two strings — the token the
//! host issued and the snapshot it last served. Everything that can be decided without the
//! wire lives here, so the tests prove it natively and the wasm shell only paints it.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sortlens_remote::client::LocalCopy;

/// What the page is doing, decided only by answers that already arrived — no half states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    /// No token: the pairing form.
    #[default]
    Unpaired,
    /// A pair or snapshot call is on the wire; the form is still there, but busy.
    Connecting,
    /// A token, and a library hydrated from the snapshot it serves.
    Open,
}

/// What the page keeps between sessions, in browser storage. A token is the only thing that
/// lets the page talk; a snapshot without its token is still kept, because the next pairing
/// re-opens it — a page that re-paired would otherwise forget everything it had seen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSession {
    /// The token the host issued when this page paired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// The snapshot the host last served, exactly as `library.snapshot` carried it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

impl SavedSession {
    /// Parse what storage gave back. Storage is remembered, not trusted: anything unreadable
    /// is treated as absent, and an unreadable snapshot never becomes a broken page.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    /// Encode for storage. A value that cannot encode is an empty session — losing what the
    /// page remembered is bad; failing the frame because of it would be worse.
    #[must_use]
    pub fn encode(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Where one queued upload stands: the transfer is on the wire, the ack landed, or the
/// server refused and said why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadStatus {
    /// The transfer is on the wire.
    Sending,
    /// The host acked: the name the photograph landed under, after sanitising.
    Saved { saved: String },
    /// The host refused or the connection failed, and said why.
    Failed { message: String },
}

/// One photograph the page is sending to the host, as the uploads list paints it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadRecord {
    /// The page's own id for this transfer — two files with the same camera name are two
    /// rows, not one.
    pub id: u64,
    /// The file name the upload asked to land under.
    pub name: String,
    /// How many bytes the transfer carries, so the row is honest about what is moving.
    pub total: usize,
    /// Where the transfer stands.
    pub status: UploadStatus,
    /// The keep-or-remove decision (SL-J05), chosen up front on the toggle and shown on
    /// the ack — the decision lives on the item, not on the connection.
    pub decision: LocalCopy,
}

/// The page's own state: what phase it is in, what it has, and what it says went wrong.
#[derive(Debug, Default)]
pub struct ShellState {
    /// The phase, advanced only by [`ShellState::pair_finished`],
    /// [`ShellState::snapshot_finished`] and [`ShellState::failed`].
    pub phase: Phase,
    /// The paired token, kept the moment the host issues it.
    pub token: Option<String>,
    /// The snapshot the host last served.
    pub snapshot: Option<String>,
    /// The one notice the page shows, replaced — never accumulated.
    pub notice: Option<String>,
    /// The keep-or-remove toggle the uploads strip holds: on, the page asks to drop its
    /// own copy once the host acks; off (the default), the copy is kept. A browser page
    /// holds its copy only as long as the transfer runs — the decision is still the honest
    /// thing to carry and show, because it is what the ack reports.
    pub remove_after_upload: bool,
    /// The uploads this session queued, in the order they were chosen.
    pub uploads: Vec<UploadRecord>,
    /// The next id [`ShellState::enqueue_upload`] mints.
    next_upload: u64,
}

impl ShellState {
    /// The state a page starts from: what a previous session saved. A token means the page
    /// was paired and stays open — it can re-reach the host when the host is back, and its
    /// saved snapshot (if any) is what it shows until then. Nothing means the pairing form.
    #[must_use]
    pub fn restored(session: SavedSession) -> Self {
        let phase = if session.token.is_some() { Phase::Open } else { Phase::Unpaired };
        Self {
            phase,
            token: session.token,
            snapshot: session.snapshot,
            notice: None,
            remove_after_upload: false,
            uploads: Vec::new(),
            next_upload: 0,
        }
    }

    /// What the page keeps between sessions right now.
    #[must_use]
    pub fn session(&self) -> SavedSession {
        SavedSession {
            token: self.token.clone(),
            snapshot: self.snapshot.clone(),
        }
    }

    /// The host answered the pairing call: the token is kept at once, and the page moves to
    /// `Connecting` — open means a library, and a library comes with the snapshot.
    pub fn pair_finished(&mut self, token: impl Into<String>) {
        self.token = Some(token.into());
        self.phase = Phase::Connecting;
        self.notice = None;
    }

    /// The host answered the snapshot call: the state is kept and the page is open.
    pub fn snapshot_finished(&mut self, snapshot: impl Into<String>) {
        self.snapshot = Some(snapshot.into());
        self.phase = Phase::Open;
        self.notice = None;
    }

    /// Something the wire refused or never answered. A paired page stays open — its snapshot
    /// is what it holds, and the notice says what it could not refresh. An unpaired page
    /// returns to the form.
    pub fn failed(&mut self, message: impl Into<String>) {
        self.notice = Some(message.into());
        if self.token.is_none() {
            self.phase = Phase::Unpaired;
        }
    }

    /// Queue one photograph for the host: the row is minted here — its id is the page's
    /// own, two files with one camera name stay two rows — and its keep-or-remove decision
    /// is recorded the moment the toggle chose it, not rediscovered later.
    pub fn enqueue_upload(&mut self, name: impl Into<String>, total: usize) -> u64 {
        let id = self.next_upload;
        self.next_upload += 1;
        let decision = if self.remove_after_upload {
            LocalCopy::Remove
        } else {
            LocalCopy::Keep
        };
        self.uploads.push(UploadRecord {
            id,
            name: name.into(),
            total,
            status: UploadStatus::Sending,
            decision,
        });
        id
    }

    /// The host acked the transfer: the row names the file it landed under (sanitised,
    /// ` (n)` beside a collision — the published name is the truth) and shows the decision
    /// the toggle made.
    pub fn upload_saved(&mut self, id: u64, saved: impl Into<String>) {
        if let Some(record) = self.uploads.iter_mut().find(|record| record.id == id) {
            record.status = UploadStatus::Saved { saved: saved.into() };
        }
    }

    /// The transfer failed: the row stays, with the server's own reason — an upload that
    /// did not land must not vanish from the page as if it had been chosen away.
    pub fn upload_failed(&mut self, id: u64, message: impl Into<String>) {
        if let Some(record) = self.uploads.iter_mut().find(|record| record.id == id) {
            record.status = UploadStatus::Failed { message: message.into() };
        }
    }

    /// Whether any transfer is still on the wire — the moment the last one lands, the page
    /// refreshes its snapshot to see the new photographs in the grid.
    #[must_use]
    pub fn uploads_inflight(&self) -> bool {
        self.uploads.iter().any(|record| record.status == UploadStatus::Sending)
    }

    /// Drop the rows whose transfer ended — the acks the user has read, and the failures —
    /// keeping the ones still on the wire.
    pub fn clear_finished_uploads(&mut self) {
        self.uploads.retain(|record| record.status == UploadStatus::Sending);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page with nothing saved starts at the pairing form.
    #[test]
    fn an_empty_session_starts_unpaired() {
        let state = ShellState::restored(SavedSession::default());
        assert_eq!(state.phase, Phase::Unpaired);
        assert!(state.token.is_none());
        assert!(state.snapshot.is_none());
    }

    /// A page with a saved token stays open, and its saved snapshot is what it holds; the
    /// next pairing is not needed to look at what it already saw.
    #[test]
    fn a_saved_token_reopens_the_page() {
        let state = ShellState::restored(SavedSession {
            token: Some("tok-1".into()),
            snapshot: Some("{\"version\":3}".into()),
        });
        assert_eq!(state.phase, Phase::Open);
        assert_eq!(state.token.as_deref(), Some("tok-1"));
        assert_eq!(state.snapshot.as_deref(), Some("{\"version\":3}"));
    }

    /// Pairing keeps the token the moment it is issued, but the page is not open until the
    /// snapshot answer hydrates it.
    #[test]
    fn pairing_is_not_open_until_the_snapshot_arrives() {
        let mut state = ShellState::default();
        state.pair_finished("tok-2");
        assert_eq!(state.phase, Phase::Connecting);
        assert_eq!(state.token.as_deref(), Some("tok-2"));
        state.snapshot_finished("{\"version\":3}");
        assert_eq!(state.phase, Phase::Open);
        assert_eq!(state.snapshot.as_deref(), Some("{\"version\":3}"));
        assert!(state.notice.is_none(), "an answer clears the notice");
    }

    /// A failure on an unpaired page returns to the form; a failure on a paired page stays
    /// open — the snapshot it holds is still its library, and the notice says what failed.
    #[test]
    fn a_failure_returns_to_the_form_only_unpaired() {
        let mut state = ShellState::default();
        state.failed("the host refused the code");
        assert_eq!(state.phase, Phase::Unpaired);
        assert_eq!(state.notice.as_deref(), Some("the host refused the code"));

        let mut state = ShellState::default();
        state.pair_finished("tok-2");
        state.snapshot_finished("{}");
        state.failed("the host is not answering");
        assert_eq!(state.phase, Phase::Open, "a paired page keeps its library");
        assert_eq!(state.notice.as_deref(), Some("the host is not answering"));
    }

    /// The session round-trips through its encoded form, and storage that holds something
    /// unreadable is treated as absent rather than trusted.
    #[test]
    fn the_session_round_trips_and_untrusted_storage_reads_as_absent() {
        let mut state = ShellState::default();
        state.pair_finished("tok-3");
        state.snapshot_finished("{\"version\":3}");
        let encoded = state.session().encode();
        let parsed = SavedSession::parse(&encoded);
        assert_eq!(parsed.token.as_deref(), Some("tok-3"));
        assert_eq!(parsed.snapshot.as_deref(), Some("{\"version\":3}"));
        // Anything else in storage is a default, never a panic and never a broken page.
        assert_eq!(SavedSession::parse("not json"), SavedSession::default());
        assert_eq!(SavedSession::parse("[1,2,3]"), SavedSession::default());
        assert_eq!(SavedSession::parse(""), SavedSession::default());
    }

    /// An upload is queued as `Sending` with the decision the toggle chose — keep by
    /// default, remove when it is on — and the ack shows both the published name and that
    /// decision. Two files with one camera name are two rows, not one.
    #[test]
    fn an_upload_is_queued_and_the_ack_shows_the_published_name_and_the_decision() {
        let mut state = ShellState::default();
        state.snapshot_finished("{}");
        assert!(!state.remove_after_upload, "the toggle's default is keep");
        let first = state.enqueue_upload("PXL_today.jpg", 2_048);
        let second = state.enqueue_upload("PXL_today.jpg", 3_331);
        assert_ne!(first, second, "two files with one name are two rows");
        assert_eq!(state.uploads.len(), 2);
        assert_eq!(state.uploads[0].decision, LocalCopy::Keep);
        assert!(state.uploads_inflight(), "a queued transfer is on the wire");

        state.upload_saved(first, "PXL_today.jpg");
        assert_eq!(
            state.uploads[0].status,
            UploadStatus::Saved {
                saved: "PXL_today.jpg".into()
            },
            "the ack names the file that landed"
        );
        assert!(state.uploads_inflight(), "the second transfer is still moving");

        // The toggle flips between the two queueings: the first row keeps its own decision,
        // because it lives on the item, not on the connection (SL-J05).
        state.remove_after_upload = true;
        let third = state.enqueue_upload("PXL_later.jpg", 1_000);
        state.upload_saved(second, "PXL_today (1).jpg");
        assert_eq!(
            state.uploads[1].status,
            UploadStatus::Saved {
                saved: "PXL_today (1).jpg".into()
            }
        );
        state.upload_saved(third, "PXL_later.jpg");
        assert_eq!(state.uploads[2].decision, LocalCopy::Remove, "the toggle said remove at queue time");
        assert_eq!(state.uploads[0].decision, LocalCopy::Keep, "an earlier row keeps its own decision");
        assert!(!state.uploads_inflight(), "every transfer has landed");
    }

    /// A failed upload stays listed with the server's own reason, and clearing the finished
    /// rows keeps only what is still on the wire.
    #[test]
    fn a_failed_upload_stays_listed_and_clearing_keeps_the_inflight() {
        let mut state = ShellState::default();
        state.snapshot_finished("{}");
        let done = state.enqueue_upload("PXL_done.jpg", 512);
        let failed = state.enqueue_upload("PXL_bad.jpg", 512);
        let moving = state.enqueue_upload("PXL_slow.jpg", 512);
        state.upload_saved(done, "PXL_done.jpg");
        state.upload_failed(failed, "the total does not match the upload in progress");

        state.clear_finished_uploads();
        assert_eq!(
            state.uploads.len(),
            1,
            "the landed and failed rows are cleared, the moving one stays"
        );
        // A failure that arrives while the row exists is recorded; a clear-then-failure is
        // a row that no longer exists, and the id is simply not found.
        state.upload_failed(moving, "the connection dropped");
        assert_eq!(
            state.uploads[0].status,
            UploadStatus::Failed {
                message: "the connection dropped".into()
            }
        );
        // An unknown id answers nothing and breaks nothing.
        state.upload_saved(999, "nowhere");
        state.upload_failed(999, "nowhere");
        assert_eq!(state.uploads.len(), 1);
    }
}
