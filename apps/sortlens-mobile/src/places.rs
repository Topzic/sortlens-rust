//! Where the phone keeps the library and where it looks for photographs.
//!
//! Pure path logic, so it is tested on the machine that builds the app.

use std::path::{Path, PathBuf};

/// The catalog directory under the app's private data directory.
#[must_use]
pub fn library_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("Library")
}

/// The thumbnail cache, beside the library and safe to delete.
#[must_use]
pub fn cache_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("Cache").join("thumbs")
}

/// The last component of a path, for a notice.
#[must_use]
pub fn folder_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

/// The folders a phone keeps photographs in, filtered to those that exist and can be listed.
///
/// Android shares the camera roll as files under shared storage once the media permission is
/// granted. iOS keeps the photo library behind its own API, so until that is wired the app reads
/// the folder the Files app shows for it. A desktop build of this crate (used by the tests) reads
/// the user's Pictures folder.
#[must_use]
pub fn photo_roots() -> Vec<PathBuf> {
    candidates().into_iter().filter(|path| std::fs::read_dir(path).is_ok()).collect()
}

#[cfg(target_os = "android")]
fn candidates() -> Vec<PathBuf> {
    ["DCIM", "Pictures"]
        .iter()
        .map(|name| PathBuf::from("/storage/emulated/0").join(name))
        .collect()
}

#[cfg(target_os = "ios")]
fn candidates() -> Vec<PathBuf> {
    home().map(|home| vec![home.join("Documents")]).unwrap_or_default()
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn candidates() -> Vec<PathBuf> {
    home().map(|home| vec![home.join("Pictures")]).unwrap_or_default()
}

#[cfg(not(target_os = "android"))]
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// The directory the iOS app keeps its library in: the sandbox's Application Support.
#[cfg(target_os = "ios")]
#[must_use]
pub fn ios_data_dir() -> PathBuf {
    home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Library")
        .join("Application Support")
        .join("Sortlens")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_and_its_cache_live_under_the_app_data_directory() {
        let data = Path::new("/data/app");
        assert_eq!(library_dir(data), Path::new("/data/app/Library"));
        assert_eq!(cache_dir(data), Path::new("/data/app/Cache/thumbs"));
        assert!(
            !cache_dir(data).starts_with(library_dir(data)),
            "deleting the cache never touches the library"
        );
    }

    #[test]
    fn a_folder_is_named_by_its_last_component() {
        assert_eq!(folder_name("/storage/emulated/0/DCIM/"), "DCIM");
        assert_eq!(folder_name("C:\\Photos\\Trip"), "Trip");
        assert_eq!(folder_name("/"), "/");
    }

    #[test]
    fn only_folders_that_can_be_listed_are_offered() {
        for root in photo_roots() {
            assert!(std::fs::read_dir(root).is_ok());
        }
    }
}
