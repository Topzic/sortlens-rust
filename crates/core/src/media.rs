//! File-kind vocabulary: which extensions are photos, raws, videos or sidecars.
//!
//! This lives in L0 because both the scanner and the thumbnail decoder need the same answer, and
//! two lists that drift apart produce files that index but never preview.

use crate::labels::MediaKind;
use std::path::Path;

/// Extensions treated as camera raw.
const RAW_EXTENSIONS: &[&str] = &[
    "3fr", "ari", "arw", "bay", "cr2", "cr3", "crw", "dcr", "dng", "erf", "fff", "iiq", "k25", "kdc", "mef", "mos", "mrw", "nef", "nrw",
    "orf", "pef", "raf", "raw", "rw2", "rwl", "sr2", "srf", "srw", "x3f",
];

/// Extensions treated as rendered images.
const IMAGE_EXTENSIONS: &[&str] = &[
    "avif", "bmp", "gif", "heic", "heif", "jpe", "jpeg", "jpg", "jxl", "png", "qoi", "tif", "tiff", "webp",
];

/// Extensions treated as video.
const VIDEO_EXTENSIONS: &[&str] = &["avi", "m4v", "mkv", "mov", "mp4", "mts", "m2ts", "webm", "wmv"];

/// Sidecar and cache files that belong to a photo but are not photos.
const SIDECAR_EXTENSIONS: &[&str] = &["aae", "xmp", "pp3", "dop", "on1", "cos", "thm", "lrf"];

/// Classify by extension. `None` for anything that is not a photo, raw, video or sidecar.
pub fn kind_from_extension(extension: &str) -> Option<MediaKind> {
    let extension = extension.to_ascii_lowercase();
    if RAW_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Raw)
    } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Video)
    } else if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Image)
    } else {
        None
    }
}

/// Whether the extension names a sidecar rather than a photo.
pub fn is_sidecar_extension(extension: &str) -> bool {
    SIDECAR_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
}

/// Whether a path looks like a camera raw, and therefore needs a preview rather than a decode.
pub fn is_raw_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| kind_from_extension(extension) == Some(MediaKind::Raw))
}

/// The extension of a path, lowercased.
pub fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn extensions_map_to_kinds_case_insensitively() {
        assert_eq!(kind_from_extension("JPG"), Some(MediaKind::Image));
        assert_eq!(kind_from_extension("cr3"), Some(MediaKind::Raw));
        assert_eq!(kind_from_extension("MOV"), Some(MediaKind::Video));
        assert_eq!(kind_from_extension("txt"), None);
    }

    #[test]
    fn sidecars_are_not_photos() {
        assert!(is_sidecar_extension("XMP"));
        assert_eq!(kind_from_extension("xmp"), None);
    }

    #[test]
    fn raw_paths_are_recognised() {
        assert!(is_raw_path(Path::new("/photos/IMG_1.CR3")));
        assert!(!is_raw_path(Path::new("/photos/IMG_1.jpg")));
        assert!(!is_raw_path(Path::new("/photos/no-extension")));
    }
}
