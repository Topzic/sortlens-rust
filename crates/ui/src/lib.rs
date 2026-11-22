//! L5: the egui shell.
//!
//! M0 surface: colour tokens, the four presets, the five-surface shell and the dialogs that
//! M0 needs (settings, about). Grid, lightbox and the review surfaces land in later milestones.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod a11y;
pub mod charts;
pub mod fonts;
pub mod keymap;
pub mod shell;
pub mod theme;
pub mod thumbs;

pub use shell::{Action, RightTab, Source, UiState, View, show};
pub use theme::{ThemeKind, Tokens, apply};
pub use thumbs::Thumbnails;

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
