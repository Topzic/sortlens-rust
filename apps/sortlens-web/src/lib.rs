//! Sortlens: the wasm client (SL-J02).
//!
//! The browser page is the same [`sortlens_ui`] shell over an engine that holds the library in
//! memory: `Engine::open_memory` opens the state `library.snapshot` serves over the wire, ops
//! apply with the same undo, redo and stale-revision rules as the desktop, and [`RemoteLibrary`]
//! hands the state back as the JSON the client saves between sessions. The protocol it speaks
//! is [`sortlens_remote::client`]'s — pairing, `/api` calls, the resumable upload and the
//! keep-or-remove decision on the ack (SL-J05) — proven against the real server in
//! `apps/sortlens-server/tests/client.rs`.
//!
//! The page itself ([`web`], wasm only) runs this shell in the browser: it pairs with the
//! host, refreshes the snapshot, paints the grid from wire thumbnails, and keeps the token
//! and the snapshot in browser storage between sessions. Build it with `cargo xtask web`;
//! the host serves it from `/app` (`--web dist`), same-origin, so no CORS is involved.

#![forbid(unsafe_code)]

pub mod shell;

#[cfg(target_arch = "wasm32")]
pub mod web;

use sortlens_core::Error;
use sortlens_engine::Engine;

/// The client's half of the library: an engine in memory, opened from the snapshot the paired
/// server holds, and saved back through the same JSON between sessions.
pub struct RemoteLibrary {
    engine: Engine,
}

impl RemoteLibrary {
    /// A fresh client: no snapshot yet, so an honest empty library, and nothing on disk —
    /// there is no disk.
    pub fn fresh() -> Result<Self, Error> {
        Ok(Self {
            engine: Engine::open_memory(None)?,
        })
    }

    /// Open the state a `library.snapshot` answer carried. A snapshot newer than this build's
    /// is refused, not opened read-only: a client that cannot read the format must not pretend
    /// to hold the library.
    pub fn hydrate(snapshot: &str) -> Result<Self, Error> {
        Ok(Self {
            engine: Engine::open_memory(Some(snapshot))?,
        })
    }

    /// The engine, for the shell to read and run commands against.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// The engine, for the shell to run commands against.
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// The state to keep between sessions: the exact JSON [`Engine::open_memory`] reopens.
    pub fn save(&self) -> Result<String, Error> {
        self.engine.snapshot_json()
    }
}
