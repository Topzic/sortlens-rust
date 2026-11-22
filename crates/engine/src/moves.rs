//! Moving picks into a destination folder, with a receipt that can put them back.
//!
//! The filesystem half is [`sortlens_cull::trash`], which already audits a batch as a whole, keeps
//! the path relative to the root and verifies a copy when the move crosses a volume. What is added
//! here is the other half: the catalog follows the files, and the receipt remembers enough old
//! records to reverse both.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sortlens_catalog::Photo;
use sortlens_core::{Error, PhotoId, Result};
use std::path::{Path, PathBuf};

/// One photo that moved: the record before and after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveItem {
    /// The record as it was.
    pub old: Photo,
    /// The record as it is now.
    pub new: Photo,
    /// Whether the photo's XMP sidecar moved with it.
    #[serde(default)]
    pub sidecar: bool,
}

/// A move that happened, kept so it can be undone later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveReceipt {
    /// Batch number, never reused.
    pub id: u64,
    /// Destination folder.
    pub dest: String,
    /// Unix seconds when it happened.
    pub at: i64,
    /// What moved.
    pub items: Vec<MoveItem>,
}

impl MoveReceipt {
    /// How many photos moved.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether nothing moved; never true for a stored receipt.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// One line for a dialog.
    pub fn headline(&self) -> String {
        format!("Moved {} photo(s) to {}", self.items.len(), folder_name(&self.dest))
    }
}

/// The last path component, for display.
pub fn folder_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

/// The record a photo has at `dest` after moving.
///
/// The bytes are the same, so size, modification time, content key and every review field carry over
/// untouched; only where the file lives changes.
pub fn moved_record(photo: &Photo, dest: &str, rel: &str) -> Photo {
    let mut moved = photo.clone();
    moved.root = dest.to_string();
    moved.rel = rel.to_string();
    moved.name = rel.rsplit('/').next().unwrap_or(rel).to_string();
    moved.missing = false;
    moved
}

/// The sidecar beside a photo, if it has one.
pub fn sidecar_of(path: &Path) -> Option<PathBuf> {
    let sidecar = path.with_extension("xmp");
    sidecar.is_file().then_some(sidecar)
}

/// Take a photo's sidecar along, so the review state written next to it never loses the file it
/// describes.
pub fn take_sidecar(from: &Path, to: &Path) -> bool {
    let Some(source) = sidecar_of(from) else {
        return false;
    };
    let target = to.with_extension("xmp");
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::rename(&source, &target).is_ok()
}

/// Move `photo` back to where the receipt says it came from.
///
/// Returns an error naming the file when the move cannot be made, because a partially restored
/// library must say which photo is still elsewhere.
pub fn put_back(item: &MoveItem) -> Result<()> {
    let from = Path::new(&item.new.root).join(&item.new.rel);
    let to = Path::new(&item.old.root).join(&item.old.rel);
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|error| Error::io(parent.display().to_string(), error))?;
    }
    match std::fs::rename(&from, &to) {
        Ok(()) => {
            if item.sidecar {
                let _ = take_sidecar(&from, &to);
            }
            Ok(())
        }
        Err(error) => Err(Error::Invalid(format!("{}: {error}", from.display()))),
    }
}

/// The ids a receipt covers, for a report.
pub fn moved_ids(receipt: &MoveReceipt) -> Vec<PhotoId> {
    receipt.items.iter().map(|item| item.new.id).collect()
}

/// A destination folder, created if it does not exist.
pub fn prepare_dest(dest: &Path) -> Result<PathBuf> {
    if !dest.is_absolute() {
        return Err(Error::Invalid("the destination must be an absolute folder".into()));
    }
    std::fs::create_dir_all(dest).map_err(|error| Error::io(dest.display().to_string(), error))?;
    Ok(dest.to_path_buf())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use sortlens_core::MediaKind;

    fn photo(root: &str, rel: &str) -> Photo {
        let mut photo = Photo::from_scan(PhotoId(1), root, rel, 10, Some(5), MediaKind::Image, 7);
        photo.rating = 4;
        photo.tags = vec!["beach".to_string()];
        photo
    }

    #[test]
    fn a_moved_record_keeps_every_fact_but_its_place() {
        let before = photo("/a", "day one/x.jpg");
        let after = moved_record(&before, "/b", "day one/x.jpg");
        assert_eq!(after.root, "/b");
        assert_eq!(after.rel, "day one/x.jpg");
        assert_eq!(after.name, "x.jpg");
        assert_eq!(after.rating, 4);
        assert_eq!(after.tags, vec!["beach".to_string()]);
        assert_eq!(after.key, before.key);
        assert_eq!(after.size, before.size);
        assert_eq!(after.id, before.id, "the id never changes");
    }

    #[test]
    fn putting_a_photo_back_moves_the_file_and_creates_folders() {
        let base = std::env::temp_dir().join(format!("sortlens-move-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("old/sub")).expect("dir");
        std::fs::write(base.join("old/sub/a.jpg"), b"a").expect("write");

        let old = photo(&base.join("old").to_string_lossy(), "sub/a.jpg");
        let new = moved_record(&old, &base.join("new").to_string_lossy(), "sub/a.jpg");
        std::fs::create_dir_all(base.join("new/sub")).expect("dir");
        std::fs::rename(base.join("old/sub/a.jpg"), base.join("new/sub/a.jpg")).expect("move");

        put_back(&MoveItem {
            old: old.clone(),
            new: new.clone(),
            sidecar: false,
        })
        .expect("put back");
        assert!(base.join("old/sub/a.jpg").exists(), "the file is home");
        assert!(!base.join("new/sub/a.jpg").exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_destination_must_be_absolute() {
        assert!(prepare_dest(Path::new("relative/folder")).is_err());
    }
}
