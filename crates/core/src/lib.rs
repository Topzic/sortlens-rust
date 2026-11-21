//! L0: the vocabulary every other Sortlens crate speaks.
//!
//! Identifiers, rating/flag/label enums, byte and count formatting, and the one error type.
//! No I/O, no UI, no filesystem: this crate must stay dependency-light so it can be used from
//! the catalog, the engine and the wasm client alike.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod error;
pub mod ids;
pub mod labels;
pub mod media;
pub mod units;

pub use error::{Error, Result};
pub use ids::{CollectionId, FolderId, JobId, PhotoId, TagId, VolumeId};
pub use labels::{ColorLabel, Flag, MediaKind, Review};
pub use media::{is_raw_path, kind_from_extension};
pub use units::{format_bytes, format_count};

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
