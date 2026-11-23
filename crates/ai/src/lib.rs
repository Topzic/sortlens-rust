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
//! - **No inference yet.** The manifest pins real, permissively licensed ONNX weights (a YOLOS-tiny
//!   detector and a CLIP ViT-B/32 image + text pair), each verified to load and run under
//!   `tract-onnx` before it was pinned — but the sessions, the non-maximum-suppression glue and the
//!   label map that would actually answer questions with them are the next piece of M8.
//! - **No downloader.** [`store::Store::install_file`] accepts a file the user already has, and the
//!   pinned URLs plus digests give the future download job something to fetch; fetching itself is a
//!   separate, consent-gated engine command (`model.install`, Phase 2) that does not exist yet, and
//!   the engine exposes no command for it — a menu item that fails would be worse than no menu item.
//!
//! There is no networking code in this crate, and a test enforces that rather than trusting review.

pub mod detect;
pub mod hints;
pub mod manifest;
#[cfg(test)]
mod no_network;
pub mod packs;
pub mod sha256;
pub mod store;
pub mod suggest;

pub use detect::{Box, Detection, LabelMap, suppress};
pub use hints::{Hints, focal_band, hints};
pub use manifest::{Manifest, Model, ModelKind};
pub use packs::{Context as PackContext, Pack, PackError, Rule as PackRule, When as PackWhen};
pub use sha256::Digest;
pub use store::{Install, Status, Store};
pub use suggest::{Candidate, Caps, Source, Suggestion, merge, summary};

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
