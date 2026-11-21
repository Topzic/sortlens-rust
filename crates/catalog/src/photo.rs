//! The photo record.
//!
//! One row per file that the catalog knows about. Paths are stored relative to their root so a
//! library keeps working when the root is remapped to another volume, and the content key is what
//! survives a rename.

use serde::{Deserialize, Serialize};
use sortlens_core::{ColorLabel, Flag, MediaKind, PhotoId};

/// Everything Sortlens knows about one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Photo {
    /// Stable identifier; never reused.
    pub id: PhotoId,
    /// Root this photo was found under, as an absolute path string.
    pub root: String,
    /// Path inside the root, `/`-separated.
    pub rel: String,
    /// File name, for display and sorting.
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    /// Modification time, seconds since the Unix epoch.
    pub modified: Option<i64>,
    /// Image, raw or video.
    pub kind: MediaKind,
    /// Cheap content identity from the scan.
    pub key: u64,
    /// Stars, 0–5. Zero means unrated.
    #[serde(default)]
    pub rating: u8,
    /// Pick, reject or none.
    #[serde(default)]
    pub flag: Flag,
    /// Colour label, if any.
    #[serde(default)]
    pub label: Option<ColorLabel>,
    /// Filesystem path was missing at the last scan.
    #[serde(default)]
    pub missing: bool,
    /// Pixel width, once metadata has been read.
    #[serde(default)]
    pub width: u32,
    /// Pixel height, once metadata has been read.
    #[serde(default)]
    pub height: u32,
    /// Capture time, seconds since the Unix epoch, from EXIF when available.
    #[serde(default)]
    pub captured: Option<i64>,
    /// Camera model, once metadata has been read.
    #[serde(default)]
    pub camera: String,
}

impl Photo {
    /// Build a record from what the walker knows.
    pub fn from_scan(id: PhotoId, root: &str, rel: &str, size: u64, modified: Option<i64>, kind: MediaKind, key: u64) -> Self {
        let name = rel.rsplit('/').next().unwrap_or(rel).to_string();
        Self {
            id,
            root: root.to_string(),
            rel: rel.to_string(),
            name,
            size,
            modified,
            kind,
            key,
            rating: 0,
            flag: Flag::None,
            label: None,
            missing: false,
            width: 0,
            height: 0,
            captured: None,
            camera: String::new(),
        }
    }

    /// Absolute path, joined with `/` so it is stable to store and compare.
    pub fn path(&self) -> String {
        if self.rel.is_empty() {
            self.root.clone()
        } else {
            format!("{}/{}", self.root.trim_end_matches('/'), self.rel)
        }
    }

    /// The immediate parent folder relative to the root, for grouping in the grid.
    pub fn folder(&self) -> &str {
        match self.rel.rfind('/') {
            Some(index) => &self.rel[..index],
            None => "",
        }
    }

    /// Whether the photo carries any review state.
    pub fn is_reviewed(&self) -> bool {
        self.rating > 0 || self.flag != Flag::None || self.label.is_some()
    }

    /// Timestamp used for ordering when EXIF is absent.
    pub fn timestamp(&self) -> i64 {
        self.captured.or(self.modified).unwrap_or(0)
    }
}

/// Aggregate counts, recomputed as the catalog changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub total: usize,
    pub rated: usize,
    pub picks: usize,
    pub rejects: usize,
    pub labelled: usize,
    pub missing: usize,
    pub bytes: u64,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn name_and_folder_come_from_the_relative_path() {
        let photo = Photo::from_scan(PhotoId(1), "/photos", "2026/trip/IMG_1.jpg", 10, Some(5), MediaKind::Image, 7);
        assert_eq!(photo.name, "IMG_1.jpg");
        assert_eq!(photo.folder(), "2026/trip");
        assert_eq!(photo.path(), "/photos/2026/trip/IMG_1.jpg");
    }

    #[test]
    fn a_root_level_file_has_no_folder() {
        let photo = Photo::from_scan(PhotoId(1), "/photos/", "a.jpg", 10, None, MediaKind::Image, 7);
        assert_eq!(photo.folder(), "");
        assert_eq!(photo.path(), "/photos/a.jpg");
    }

    #[test]
    fn timestamp_prefers_capture_time() {
        let mut photo = Photo::from_scan(PhotoId(1), "/p", "a.jpg", 1, Some(100), MediaKind::Image, 1);
        assert_eq!(photo.timestamp(), 100);
        photo.captured = Some(50);
        assert_eq!(photo.timestamp(), 50);
    }
}
