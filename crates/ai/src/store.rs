//! The content-addressed model store.
//!
//! A model lives at `<root>/<sha256>.<kind>` — the name *is* the digest, so a file cannot be silently
//! replaced, two models cannot overwrite each other, and a renamed file is still identifiable. An
//! install writes to a temporary file, verifies what landed on disk, and only then renames it into
//! place; a crash mid-install leaves no half-model where a reader would find it.

use crate::manifest::{Manifest, Mismatch, Model};
use crate::sha256;
use std::fs;
use std::path::{Path, PathBuf};

/// What is on disk for one model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// The file is there and its bytes hash to the manifest's digest.
    Installed { path: PathBuf },
    /// Nothing is installed for this model.
    Missing,
    /// A file is there and is not the promised model. It is left alone, not deleted: it may be a
    /// model someone is in the middle of downloading, and quietly removing a large file is rude.
    Wrong { path: PathBuf, reason: Mismatch },
}

impl Status {
    /// Whether this model can be used.
    #[must_use]
    pub fn is_installed(&self) -> bool {
        matches!(self, Status::Installed { .. })
    }

    /// One line for settings.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Status::Installed { path } => format!("Installed · {}", path.display()),
            Status::Missing => "Not installed".to_string(),
            Status::Wrong { reason, .. } => format!("Not usable: {}", reason.detail()),
        }
    }
}

/// The result of an install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Install {
    /// Where the model now lives.
    pub path: PathBuf,
    /// How many bytes it is.
    pub bytes: u64,
    /// The digest, as hex, which is also its name.
    pub sha256: String,
}

impl Install {
    /// One line for a receipt.
    #[must_use]
    pub fn headline(&self) -> String {
        format!(
            "{} installed · {}",
            sortlens_core::format_bytes(self.bytes),
            &self.sha256[..self.sha256.len().min(12)]
        )
    }
}

/// A folder of verified models.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Open (creating if needed) the store at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Store, String> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| format!("could not create {}: {error}", root.display()))?;
        Ok(Store { root })
    }

    /// The folder models live in.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// What is on disk for one model, verified rather than assumed.
    ///
    /// Verification reads the whole file. That is the cost of trusting it, and it is why the answer is
    /// asked once per session rather than per photo.
    #[must_use]
    pub fn status(&self, model: &Model) -> Status {
        let path = self.root.join(model.file_name());
        match fs::read(&path) {
            Ok(bytes) => match model.verify(&bytes) {
                Ok(()) => Status::Installed { path },
                Err(reason) => Status::Wrong { path, reason },
            },
            Err(_) => Status::Missing,
        }
    }

    /// Whether the store holds a usable copy. Cheap only in the sense that it does not parse anything.
    #[must_use]
    pub fn has(&self, model: &Model) -> bool {
        self.status(model).is_installed()
    }

    /// Install model bytes.
    ///
    /// The bytes are verified in memory first, so a wrong file never reaches the disk; then they are
    /// written under a temporary name, read back, verified again, and renamed into place. The second
    /// verification is not superstition: it catches a short write, which is what a full disk looks like.
    pub fn install(&self, model: &Model, bytes: &[u8]) -> Result<Install, String> {
        model.verify(bytes).map_err(|mismatch| mismatch.detail())?;
        let dest = self.root.join(model.file_name());
        let temporary = self.root.join(format!("{}.incoming", model.file_name()));
        fs::write(&temporary, bytes).map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
        let written = fs::read(&temporary).map_err(|error| format!("could not read back {}: {error}", temporary.display()))?;
        if let Err(mismatch) = model.verify(&written) {
            let _ = fs::remove_file(&temporary);
            return Err(format!("the copy on disk is not the model: {mismatch}"));
        }
        fs::rename(&temporary, &dest).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!("could not move the model into place at {}: {error}", dest.display())
        })?;
        Ok(Install {
            path: dest,
            bytes: written.len() as u64,
            sha256: sha256::to_hex(&sha256::digest(&written)),
        })
    }

    /// Install from a file the user already has, which is the only install path today.
    pub fn install_file(&self, model: &Model, source: &Path) -> Result<Install, String> {
        let bytes = fs::read(source).map_err(|error| format!("could not read {}: {error}", source.display()))?;
        self.install(model, &bytes)
    }

    /// Total bytes the store occupies, for a settings row.
    #[must_use]
    pub fn usage(&self) -> u64 {
        match fs::read_dir(&self.root) {
            Ok(entries) => entries
                .flatten()
                .filter_map(|entry| entry.metadata().ok())
                .filter(|meta| meta.is_file())
                .map(|meta| meta.len())
                .sum(),
            Err(_) => 0,
        }
    }

    /// Remove a model. Returns the bytes reclaimed.
    ///
    /// Only a verified model is removed: deleting a file this build cannot identify would be deleting
    /// someone else's download.
    pub fn remove(&self, model: &Model) -> Result<u64, String> {
        match self.status(model) {
            Status::Installed { path } => {
                let bytes = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
                fs::remove_file(&path).map_err(|error| format!("could not remove {}: {error}", path.display()))?;
                Ok(bytes)
            }
            Status::Missing => Ok(0),
            Status::Wrong { path, reason } => Err(format!(
                "{} is not a verified {}: {}; it was left where it is",
                path.display(),
                model.id,
                reason.detail()
            )),
        }
    }

    /// How many manifest models are usable right now.
    #[must_use]
    pub fn installed_count(&self, manifest: &Manifest) -> usize {
        manifest.models.iter().filter(|model| self.has(model)).count()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::manifest::ModelKind;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sortlens-ai-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn model(bytes: &[u8]) -> Model {
        Model {
            id: "detector".into(),
            label: "Detector".into(),
            kind: ModelKind::Detector,
            licence: "Apache-2.0".into(),
            sha256: sha256::to_hex(&sha256::digest(bytes)),
            bytes: bytes.len() as u64,
            url: String::new(),
            input_edge: None,
        }
    }

    #[test]
    fn a_fresh_store_is_empty() {
        let root = temp("fresh");
        let store = Store::open(&root).expect("open");
        let model = model(b"weights");
        assert_eq!(store.status(&model), Status::Missing);
        assert_eq!(store.usage(), 0);
        assert!(!store.has(&model));
        assert_eq!(store.installed_count(&Manifest::shipped()), 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_install_is_verified_and_lands_under_its_digest() {
        let root = temp("install");
        let store = Store::open(&root).expect("open");
        let model = model(b"weights");
        let install = store.install(&model, b"weights").expect("install");
        assert_eq!(install.bytes, 7);
        assert_eq!(install.sha256, model.sha256);
        assert!(install.path.exists());
        assert!(install.path.ends_with(model.file_name()));
        assert!(install.headline().contains("7 B"));
        assert!(store.has(&model));
        assert_eq!(store.usage(), 7);
        assert_eq!(
            store.status(&model),
            Status::Installed {
                path: install.path.clone()
            }
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_wrong_file_never_reaches_the_disk() {
        let root = temp("wrong");
        let store = Store::open(&root).expect("open");
        let model = model(b"weights");
        let error = store.install(&model, b"weightz").expect_err("wrong bytes");
        assert!(error.contains("not the model the manifest describes"), "{error}");
        assert_eq!(store.usage(), 0, "nothing was written");
        assert_eq!(store.status(&model), Status::Missing);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_tampered_file_on_disk_is_reported_and_left_alone() {
        let root = temp("tampered");
        let store = Store::open(&root).expect("open");
        let model = model(b"weights");
        let install = store.install(&model, b"weights").expect("install");
        // Something rewrote the file after it was verified.
        fs::write(&install.path, b"weightz").expect("tamper");
        let status = store.status(&model);
        assert!(
            matches!(
                &status,
                Status::Wrong {
                    reason: Mismatch::Digest { .. },
                    ..
                }
            ),
            "a tampered file is a digest mismatch, got {status:?}"
        );
        assert!(!store.has(&model));
        let error = store.remove(&model).expect_err("refuses to remove an unverified file");
        assert!(error.contains("was left where it is"), "{error}");
        assert!(install.path.exists(), "an unidentifiable file is not deleted");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn removing_a_model_reclaims_its_bytes_and_is_idempotent() {
        let root = temp("remove");
        let store = Store::open(&root).expect("open");
        let model = model(b"weights");
        store.install(&model, b"weights").expect("install");
        assert_eq!(store.remove(&model).expect("remove"), 7);
        assert_eq!(store.status(&model), Status::Missing);
        assert_eq!(store.remove(&model).expect("second remove"), 0);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn installing_from_a_file_verifies_the_file() {
        let root = temp("install-file");
        let store = Store::open(&root).expect("open");
        let model = model(b"weights");
        let source = root.join("downloaded.bin");
        fs::write(&source, b"weights").expect("write source");
        let install = store.install_file(&model, &source).expect("install from file");
        assert!(install.path.exists());
        // The source is the user's file; it is read, not moved.
        assert!(source.exists());
        // A source that changed after it was downloaded is refused before anything is written, and
        // the wrong bytes stay the user's file untouched.
        fs::write(&source, b"weightz").expect("rewrite source");
        let error = store.install_file(&model, &source).expect_err("changed source");
        assert!(error.contains("not the model"), "{error}");
        assert_eq!(fs::read(&source).expect("read source"), b"weightz");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_model_status_says_so_in_words() {
        assert_eq!(Status::Missing.label(), "Not installed");
        assert!(!Status::Missing.is_installed());
        let wrong = Status::Wrong {
            path: PathBuf::from("/models/x"),
            reason: Mismatch::Size { expected: 10, actual: 4 },
        };
        assert!(wrong.label().starts_with("Not usable:"));
        assert!(wrong.label().contains("truncated"));
    }

    #[test]
    fn two_models_cannot_overwrite_each_other() {
        let root = temp("two");
        let store = Store::open(&root).expect("open");
        let first = model(b"weights");
        let mut second = model(b"other");
        second.id = "other".into();
        store.install(&first, b"weights").expect("first");
        store.install(&second, b"other").expect("second");
        assert!(store.has(&first));
        assert!(store.has(&second));
        assert_eq!(store.usage(), 12);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn no_incoming_file_is_left_behind_by_a_failed_install() {
        let root = temp("incoming");
        let store = Store::open(&root).expect("open");
        let model = model(b"weights");
        let _ = store.install(&model, b"weightz");
        let leftovers: Vec<String> = fs::read_dir(&root)
            .expect("read dir")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        assert!(leftovers.is_empty(), "a failed install leaves nothing behind: {leftovers:?}");
        let _ = fs::remove_dir_all(&root);
    }
}
