//! Library pairing, capability tokens and the remote protocol binding.
//!
//! The rule (`docs/ARCHITECTURE.md` §10): the remote API is the command registry with a
//! restricted allow-list, not a second API. This crate holds the three pieces every remote
//! surface shares — [`tokens`] (scoped device tokens and their store), [`pairing`] (the
//! short-lived code that becomes a token) and [`allowlist`] (which registry commands the wire
//! accepts, and the scope each needs), plus [`client`] — the device side of the same protocol
//! (SL-J02): what a paired wasm page or phone speaks, and the keep-or-remove decision it
//! applies on the server's ack (SL-J05). The HTTP face lives in `apps/sortlens-server`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod allowlist;
pub mod client;
pub mod pairing;
pub mod supervise;
pub mod tokens;

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
