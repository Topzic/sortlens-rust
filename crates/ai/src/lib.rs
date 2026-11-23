#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]
//! Local model runtime for tagging and captioning — the store, the manifest and the inference.
//!
//! Everything here answers one of two questions. *Is this file the model the manifest promised?* is
//! [`store`] and [`manifest`]'s job: a model is the largest and least inspectable thing this
//! application loads, so nothing is used until its SHA-256 and its size match what shipped in the
//! manifest. *What does the model say about this picture?* is [`infer`]'s job, through
//! `tract-onnx`, strictly offline, on the CPU, desktop targets only.
//!
//! One deliberate absence, stated so nobody assumes otherwise:
//!
//! - **No downloader.** [`store::Store::install_file`] accepts a file the user already has, and the
//!   pinned URLs plus digests give the download job something to fetch; fetching itself is a separate,
//!   consent-gated engine command (`model.install`, in `sortlens-engine`) that lives outside this
//!   crate on purpose.
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

#[cfg(not(target_arch = "wasm32"))]
pub mod infer;
#[cfg(not(target_arch = "wasm32"))]
pub use infer::{COCO_LABELS_TEXT, Detector, Embedding, IMAGENET_MEAN, IMAGENET_STD, Image, ImageEmbedder, TextEmbedder};
#[cfg(not(target_arch = "wasm32"))]
pub mod text_cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod tokenize;
#[cfg(not(target_arch = "wasm32"))]
pub use tokenize::{BOS as TEXT_BOS, CONTEXT as TEXT_CONTEXT, EOS as TEXT_EOS, PAD as TEXT_PAD, Tokenizer, pack_prompt};

pub use detect::{Box, Detection, LabelMap, suppress};
pub use hints::{Hints, focal_band, hints};
pub use manifest::{Manifest, Model, ModelKind};
pub use packs::{Context as PackContext, Pack, PackError, Rule as PackRule, When as PackWhen};
pub use sha256::Digest;
pub use store::{Install, Status, Store};
pub use suggest::{Candidate, Caps, Source, Suggestion, merge, summary};

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
