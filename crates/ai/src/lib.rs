#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]
//! Local model runtime for tagging and captioning — the store, not the inference.
//!
//! Everything here answers one question: *is this file the model the manifest promised?* A model is
//! the largest and least inspectable thing this application loads, so the rule is that nothing is
//! used until its SHA-256 and its size match what shipped in the manifest.
//!
//! Two deliberate absences, stated so nobody assumes otherwise:
//!
//! - **No inference yet.** `tract` sessions, non-maximum suppression and the label map are the next
//!   piece of M8; this crate currently has no model to run, so it ships no runtime for one.
//! - **No downloader.** The manifest carries the licence and digest a download would be checked
//!   against, and [`store::Store::install_file`] accepts a file the user already has. Fetching it is a
//!   separate, consent-gated job that does not exist yet, and the engine exposes no command for it —
//!   a menu item that fails on an empty manifest would be worse than no menu item.
//!
//! There is no networking code in this crate, and a test enforces that rather than trusting review.

pub mod detect;
pub mod manifest;
#[cfg(test)]
mod no_network;
pub mod sha256;
pub mod store;
pub mod suggest;

pub use detect::{Box, Detection, LabelMap, suppress};
pub use manifest::{Manifest, Model, ModelKind};
pub use sha256::Digest;
pub use store::{Install, Status, Store};
pub use suggest::{Candidate, Caps, Suggestion, merge, summary};

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
