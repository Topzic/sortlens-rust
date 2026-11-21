//! Format detection and the routing table from a file to a decoder.
//!
//! M0 scaffold. The public surface is specified in `docs/ARCHITECTURE.md` and lands in the
//! milestone named there.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
