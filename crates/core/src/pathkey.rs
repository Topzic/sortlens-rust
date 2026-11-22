//! A path identity that survives the things that actually vary between two spellings of one folder.
//!
//! Three things make "the same folder" not the same string:
//!
//! 1. **Unicode normalisation.** macOS stores filenames in NFD (`Cafe\u{301}`) and a person typing a name, or a
//!    shell glob, produces NFC (`Café`). Byte comparison says they differ; the disk says they are one folder.
//! 2. **Trailing separators.** `/photos`, `/photos/` and `/photos//` are one path. They arrive from drag and
//!    drop, from a config file written by hand, and from `Path::join`.
//! 3. **Case.** macOS and Windows default to case-insensitive volumes, so `/Photos` opens the folder stored as
//!    `/photos`. Folding case unconditionally would be worse than doing nothing: on a case-sensitive disk
//!    `Ångström.jpg` and `ångström.jpg` are two files, and merging them loses one.
//!
//! So this type does the first two unconditionally, and resolves the third by **asking the disk**: for each
//! component, a case-insensitive lookup is made in its parent, and the on-disk spelling is adopted only when
//! exactly one entry matches. Two matches means the volume can hold both spellings, so nothing is folded and the
//! given spelling stands. A component that does not exist keeps the spelling it was given — a folder that is not
//! mounted yet still has an identity, and it should be the one that was typed.
//!
//! The result is a stable string for keys: two spellings of one folder produce one key on a case-insensitive
//! volume, two keys on a case-sensitive one, and nothing is ever silently merged.

use std::fmt;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

/// A folder's identity as a string, for keying settings that belong to a folder.
///
/// Compare with [`PathKey::as_str`]; construct with [`PathKey::of`] or [`PathKey::of_display`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct PathKey(String);

impl PathKey {
    /// The identity of an existing (or not yet mounted) path.
    pub fn of(path: &Path) -> Self {
        Self::of_display(&path.to_string_lossy())
    }

    /// The identity of a path written as text.
    ///
    /// The text is normalised, its trailing separators are dropped, and each component is resolved against the
    /// disk when the disk can answer unambiguously. Separators are the platform's: on Windows a `\` separates
    /// components exactly as a `/` does, and a key is spelled with `/` either way, so a key built from a
    /// `Path::join` matches the same folder spelled by hand.
    pub fn of_display(text: &str) -> Self {
        let cleaned = text.nfc().collect::<String>();
        let trimmed = trim_separators(&cleaned);
        if trimmed.is_empty() {
            return Self(String::new());
        }
        let mut resolved = String::new();
        let rooted = text.starts_with(std::path::is_separator);
        for component in split_components(trimmed) {
            if !resolved.is_empty() || rooted {
                resolved.push('/');
            }
            resolved.push_str(&resolve_component(&resolved, component));
        }
        Self(resolved)
    }

    /// The key, for storing beside a setting.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is the empty path — no folder at all.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for PathKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Drop trailing separators, keeping the root of a rooted path as `/`.
fn trim_separators(text: &str) -> &str {
    let trimmed = text.trim_end_matches(std::path::is_separator);
    if trimmed.is_empty() && text.starts_with(std::path::is_separator) {
        "/"
    } else {
        trimmed
    }
}

/// Split into components on the platform's separators. On Windows a `Path::join` writes `\`; on Unix a
/// backslash is an ordinary character in a name and must not split one.
fn split_components(text: &str) -> impl Iterator<Item = &str> {
    text.split(std::path::is_separator).filter(|component| !component.is_empty())
}

/// The on-disk spelling of `component` inside `parent`, when the disk answers unambiguously.
///
/// `parent` is the already-resolved prefix, which is what makes this a top-down walk: a folder's identity should
/// use the spelling the disk uses at every level, not only in its last component.
fn resolve_component(parent: &str, component: &str) -> String {
    let directory = if parent.is_empty() { Path::new(".") } else { Path::new(parent) };
    let Ok(entries) = std::fs::read_dir(directory) else {
        return component.to_string();
    };
    // Compare NFC-normalised and case-insensitively, and count the matches: two entries differing only by case
    // mean this volume can hold both, so there is nothing to resolve and folding would merge two folders.
    let wanted: String = component.nfc().collect::<String>().to_lowercase();
    let mut matches: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // Only the *last* component may be something other than a directory: `read_dir` on a file's parent is
        // fine, and this walk never descends into a file.
        if name.nfc().collect::<String>().to_lowercase() == wanted {
            matches.push(name);
        }
    }
    match matches.as_slice() {
        [only] => only.clone(),
        _ => component.to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sortlens-pathkey-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("dir");
            Self(dir)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// NFD and NFC are one folder, and so are the trailing-separator spellings.
    #[test]
    fn the_two_normal_forms_and_stray_separators_are_one_key() {
        let temp = Temp::new("forms");
        // The disk stores NFD; the person types NFC. Both must key the same folder.
        let decomposed = format!("{}/Cafe\u{301}", temp.0.display());
        let composed = format!("{}/Caf\u{e9}", temp.0.display());
        std::fs::create_dir_all(&decomposed).expect("dir");
        assert_eq!(PathKey::of_display(&decomposed), PathKey::of_display(&composed));

        let plain = format!("{}/plain", temp.0.display());
        std::fs::create_dir_all(&plain).expect("dir");
        for spelling in [plain.clone(), format!("{plain}/"), format!("{plain}//")] {
            assert_eq!(
                PathKey::of_display(&spelling).as_str().trim_end_matches('/'),
                PathKey::of_display(&plain).as_str().trim_end_matches('/'),
                "{spelling} is the same folder"
            );
        }
        // A path that does not exist yet still has a stable identity, spelled as given apart from the
        // canonical separator: a key is always spelled with `/`, so a `Path::join` spelling and a hand-written
        // one come out the same.
        let absent = format!("{}/not-mounted-yet", temp.0.display());
        assert_eq!(PathKey::of_display(&absent).as_str(), canonicalise(&absent));
        assert!(PathKey::of_display("").is_empty());
    }

    /// A case-insensitive volume resolves to the spelling the disk uses; a case-sensitive one keeps both.
    ///
    /// The second half is the one that matters: this test runs on whatever volume the test directory is on, so it
    /// asserts what that volume actually does rather than what the platform usually does.
    #[test]
    fn case_is_resolved_by_the_disk_and_never_folded() {
        let temp = Temp::new("case");
        std::fs::create_dir_all(temp.0.join("Photos")).expect("dir");
        let typed = format!("{}/photos", temp.0.display());
        let stored = format!("{}/Photos", temp.0.display());
        let key = PathKey::of_display(&typed).as_str().to_string();

        // Does this volume distinguish case? Ask it, then hold the key to that answer.
        let case_sensitive = std::fs::metadata(&typed).is_err();
        if case_sensitive {
            assert_eq!(key, canonicalise(&typed), "two real folders must not be merged");
        } else {
            assert_eq!(key, canonicalise(&stored), "one real folder resolves to its own spelling");
            assert_eq!(PathKey::of_display(&stored).as_str(), key);
        }

        // Two files differing only by case: whichever volume this is, the two keys are never merged.
        let first = temp.0.join("\u{c5}ngstr\u{f6}m.jpg");
        let second = temp.0.join("\u{e5}ngstr\u{f6}m.jpg");
        std::fs::write(&first, b"a").expect("write");
        let upper = PathKey::of(&first);
        let lower = PathKey::of(&second);
        if std::fs::metadata(&second).is_ok() {
            // A case-insensitive volume cannot hold both files at all: `second` is `first`.
            assert_eq!(upper, lower, "the volume itself treats them as one file");
        } else {
            assert_ne!(upper, lower, "two files that exist are two keys");
            assert_eq!(
                upper.as_str(),
                canonicalise(first.to_string_lossy()),
                "and each keeps its own spelling"
            );
        }
    }

    /// The spelling a path was given, with the platform's separator made canonical — what a key of an
    /// absent path keeps, and what the tests hold the keys against when the disk does not resolve.
    fn canonicalise(text: impl std::fmt::Display) -> String {
        text.to_string().replace(std::path::MAIN_SEPARATOR_STR, "/")
    }

    /// The key is normalised at every level, not only in the last component.
    #[test]
    fn every_component_is_resolved() {
        let temp = Temp::new("components");
        std::fs::create_dir_all(temp.0.join("Shoot/Day One")).expect("dir");
        // Typed with two wrong-case components, on a volume that folds case: both are adopted.
        let typed = format!("{}/shoot/day one", temp.0.display());
        let key = PathKey::of_display(&typed);
        if std::fs::metadata(&typed).is_ok() {
            assert!(
                key.as_str().ends_with("/Shoot/Day One"),
                "the disk's spelling is used all the way down: {key}"
            );
        } else {
            assert_eq!(key.as_str(), typed, "a case-sensitive disk keeps what was typed");
        }
    }

    /// A path built by `Path::join` is one key with the same folder spelled by hand.
    ///
    /// On Windows `join` writes `\`, and a key that kept the separator of the machine it was built on
    /// would not match the same folder written with `/` in a config file. The separator is not part of
    /// a folder's identity.
    #[test]
    fn a_joined_path_and_a_hand_spelled_one_are_one_key() {
        let temp = Temp::new("join");
        std::fs::create_dir_all(temp.0.join("Photos")).expect("dir");
        let joined = PathKey::of(&temp.0.join("photos"));
        let hand = PathKey::of_display(&format!("{}/Photos", temp.0.display()));
        assert_eq!(joined, hand, "the separator is not part of a folder's identity");
        assert!(joined.as_str().ends_with("/Photos"), "the disk's spelling is adopted: {joined}");

        // A file inside, spelled with the volume's own separator, resolves all the same.
        let file = temp.0.join("Photos").join("a.jpg");
        std::fs::write(&file, b"a").expect("write");
        assert_eq!(
            PathKey::of(&file),
            PathKey::of_display(&format!("{}/Photos/a.jpg", temp.0.display()))
        );
    }
}
