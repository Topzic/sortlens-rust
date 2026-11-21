//! L3: the photo catalog.
//!
//! An append-only op log, a snapshot, and the in-memory index over both. Ratings, flags, labels
//! and membership are all operations, so undo, audit and crash recovery come from one mechanism
//! rather than three.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod collection;
pub mod op;
pub mod photo;
pub mod query;
pub mod store;

pub use collection::Collection;
pub use op::Op;
pub use photo::{Photo, Stats};
pub use query::{Filter, Page, Sort, TagMode};
pub use store::{Catalog, LOG_NAME, OpenReport, SNAPSHOT_NAME, Seed, SyncOutcome};

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
