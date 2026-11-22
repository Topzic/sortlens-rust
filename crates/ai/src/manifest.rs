//! The model manifest: what a model is, what it may be used under, and what its bytes must hash to.
//!
//! The manifest is data, not code. It ships with the application so that the answer to "what am I
//! about to load, and under whose licence" is a file someone can read — and so that a digest can be
//! checked before anything is executed.

use crate::sha256::{self, Digest};
use serde::{Deserialize, Serialize};

/// What a model does, which decides where its output may be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    /// Finds things in the picture and draws boxes round them.
    Detector,
    /// Turns an image into a vector, for "find similar".
    Embedder,
    /// Writes words about a picture.
    Captioner,
}

impl ModelKind {
    /// A short name for a settings row.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            ModelKind::Detector => "Detector",
            ModelKind::Embedder => "Embedder",
            ModelKind::Captioner => "Captioner",
        }
    }

    /// The file name the store writes, since two models of different kinds may share a name.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            ModelKind::Detector => "detector",
            ModelKind::Embedder => "embedder",
            ModelKind::Captioner => "captioner",
        }
    }
}

/// One model the application knows how to describe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Stable id, used in settings and in the log. Never changes once shipped.
    pub id: String,
    /// What a person sees in settings.
    pub label: String,
    /// What this does.
    pub kind: ModelKind,
    /// The licence the weights are distributed under, as the licence's own name.
    pub licence: String,
    /// Lowercase hex SHA-256 of the weights file.
    pub sha256: String,
    /// Exact size in bytes, so a truncated download is caught before it is hashed.
    pub bytes: u64,
    /// Where the weights come from. Empty when no downloader exists for this model.
    #[serde(default)]
    pub url: String,
    /// The input edge length the network expects, when it has an opinion.
    #[serde(default)]
    pub input_edge: Option<u32>,
}

impl Model {
    /// The digest this model must have, or `None` when the manifest entry is malformed.
    #[must_use]
    pub fn digest(&self) -> Option<Digest> {
        sha256::from_hex(&self.sha256)
    }

    /// Check a candidate file against the manifest: size first, because it is free, then the digest.
    pub fn verify(&self, bytes: &[u8]) -> Result<(), Mismatch> {
        let expected = self.digest().ok_or_else(|| Mismatch::Malformed(self.sha256.clone()))?;
        let actual_size = bytes.len() as u64;
        if actual_size != self.bytes {
            return Err(Mismatch::Size {
                expected: self.bytes,
                actual: actual_size,
            });
        }
        let actual = sha256::digest(bytes);
        if actual != expected {
            return Err(Mismatch::Digest {
                expected: sha256::to_hex(&expected),
                actual: sha256::to_hex(&actual),
            });
        }
        Ok(())
    }

    /// The file name the store uses: the digest, so two models cannot overwrite each other and a
    /// renamed file is still identifiable.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}.{}", self.sha256.to_lowercase(), self.kind.extension())
    }

    /// One line for a settings row: what it is and how big.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{} · {} · {}",
            self.kind.label(),
            self.licence,
            sortlens_core::format_bytes(self.bytes)
        )
    }
}

/// Why a file is not the model the manifest described.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mismatch {
    /// The manifest's own digest is not a digest, which is a bug in the manifest and not in the file.
    Malformed(String),
    /// The file is the wrong length.
    Size { expected: u64, actual: u64 },
    /// The file is the right length and the wrong content.
    Digest { expected: String, actual: String },
}

impl Mismatch {
    /// What to tell the user.
    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Mismatch::Malformed(text) => format!("the manifest's digest {text:?} is not a SHA-256 digest"),
            Mismatch::Size { expected, actual } => {
                format!("expected {expected} bytes, found {actual} — the file is truncated or the manifest is stale")
            }
            Mismatch::Digest { expected, actual } => {
                format!("expected {expected}, found {actual} — the file is not the model the manifest describes")
            }
        }
    }
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail())
    }
}

/// The set of models this build knows about.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Schema version, so a newer manifest is refused rather than half-understood.
    #[serde(default = "current_schema")]
    pub schema: u32,
    /// The models, in the order settings should show them.
    #[serde(default)]
    pub models: Vec<Model>,
}

/// The manifest schema this build understands.
pub const SCHEMA: u32 = 1;

fn current_schema() -> u32 {
    SCHEMA
}

impl Manifest {
    /// The manifest that ships with this build.
    ///
    /// It is empty, and that is the honest state: no model has been chosen, pinned and licensed for
    /// redistribution yet, so there is nothing to describe. The moment one is, its entry goes here with
    /// its digest, and the store and the download job start having something to do.
    #[must_use]
    pub fn shipped() -> Manifest {
        Manifest {
            schema: SCHEMA,
            models: Vec::new(),
        }
    }

    /// Parse a manifest from JSON, refusing a newer schema.
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let manifest: Manifest = serde_json::from_str(text).map_err(|error| format!("the manifest is not valid: {error}"))?;
        if manifest.schema > SCHEMA {
            return Err(format!(
                "the manifest is schema {} and this build understands {SCHEMA}",
                manifest.schema
            ));
        }
        for model in &manifest.models {
            if model.digest().is_none() {
                return Err(format!("model {} has no usable SHA-256 digest", model.id));
            }
            if model.bytes == 0 {
                return Err(format!("model {} has no size", model.id));
            }
            if model.licence.trim().is_empty() {
                return Err(format!("model {} does not say what licence it is under", model.id));
            }
        }
        Ok(manifest)
    }

    /// Find a model by id.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|model| model.id == id)
    }

    /// Every model of one kind.
    #[must_use]
    pub fn of_kind(&self, kind: ModelKind) -> Vec<&Model> {
        self.models.iter().filter(|model| model.kind == kind).collect()
    }

    /// Whether there is anything to install.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn model() -> Model {
        Model {
            id: "test-detector".into(),
            label: "Test detector".into(),
            kind: ModelKind::Detector,
            licence: "Apache-2.0".into(),
            sha256: sha256::to_hex(&sha256::digest(b"weights")),
            bytes: 7,
            url: String::new(),
            input_edge: Some(320),
        }
    }

    fn json(model: &Model) -> String {
        serde_json::to_string(&Manifest {
            schema: SCHEMA,
            models: vec![model.clone()],
        })
        .expect("serialise")
    }

    #[test]
    fn a_model_verifies_against_its_own_digest() {
        assert!(model().verify(b"weights").is_ok());
    }

    #[test]
    fn a_truncated_file_is_caught_by_size_before_hashing() {
        let error = model().verify(b"weigh").expect_err("short file");
        assert!(matches!(error, Mismatch::Size { expected: 7, actual: 5 }), "{error:?}");
        assert!(error.detail().contains("truncated"));
    }

    #[test]
    fn the_right_size_and_the_wrong_bytes_is_a_digest_mismatch() {
        let error = model().verify(b"weightz").expect_err("wrong bytes");
        assert!(matches!(error, Mismatch::Digest { .. }), "{error:?}");
        assert!(error.detail().contains("not the model the manifest describes"));
    }

    #[test]
    fn a_manifest_entry_without_a_digest_is_refused() {
        let mut broken = model();
        broken.sha256 = "not-a-digest".into();
        let error = Manifest::parse(&json(&broken)).expect_err("bad digest");
        assert!(error.contains("no usable SHA-256 digest"), "{error}");
        assert!(broken.digest().is_none());
        assert!(matches!(broken.verify(b"weights"), Err(Mismatch::Malformed(_))));
    }

    #[test]
    fn a_manifest_entry_without_a_licence_is_refused() {
        let mut broken = model();
        broken.licence = "  ".into();
        let error = Manifest::parse(&json(&broken)).expect_err("no licence");
        assert!(error.contains("licence"), "{error}");
    }

    #[test]
    fn a_newer_manifest_is_refused_rather_than_half_understood() {
        let text = format!(r#"{{"schema": {}, "models": []}}"#, SCHEMA + 1);
        let error = Manifest::parse(&text).expect_err("newer schema");
        assert!(error.contains("schema"), "{error}");
    }

    #[test]
    fn the_shipped_manifest_is_empty_and_says_so() {
        let manifest = Manifest::shipped();
        assert!(manifest.is_empty(), "no model has been licensed for redistribution yet");
        assert!(manifest.get("anything").is_none());
        assert!(manifest.of_kind(ModelKind::Detector).is_empty());
        assert_eq!(manifest.schema, SCHEMA);
    }

    #[test]
    fn a_manifest_round_trips_through_json() {
        let original = Manifest {
            schema: SCHEMA,
            models: vec![model()],
        };
        let text = serde_json::to_string(&original).expect("serialise");
        let parsed = Manifest::parse(&text).expect("parse");
        assert_eq!(parsed, original);
        assert_eq!(
            parsed.get("test-detector").map(|found| found.label.clone()),
            Some("Test detector".into())
        );
        assert_eq!(parsed.of_kind(ModelKind::Embedder).len(), 0);
    }

    #[test]
    fn a_file_name_is_the_digest_so_two_models_cannot_collide() {
        let one = model();
        let mut two = model();
        two.sha256 = sha256::to_hex(&sha256::digest(b"other weights"));
        assert_ne!(one.file_name(), two.file_name());
        assert!(one.file_name().starts_with(&one.sha256));
        assert!(one.file_name().ends_with(".detector"));
    }

    #[test]
    fn a_summary_says_kind_licence_and_size() {
        let text = model().summary();
        assert!(text.contains("Detector"));
        assert!(text.contains("Apache-2.0"));
        assert!(text.contains("7 B"));
    }
}
