//! The page's state machine, in a form its tests can run.
//!
//! A browser page is a device (SL-J02): it holds no library of its own, it speaks the paired
//! host's protocol, and what it keeps between sessions is exactly two strings — the token the
//! host issued and the snapshot it last served. Everything that can be decided without the
//! wire lives here, so the tests prove it natively and the wasm shell only paints it.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

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
}
