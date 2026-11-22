//! Marks a folder carries in the sidebar: pinned, and a colour.
//!
//! Folders are not objects the library owns — they are discovered by the scan, and a folder that leaves the disk
//! is gone with its photos. So a mark is not stored *in* a folder; it is a record keyed by a
//! [`sortlens_core::pathkey::PathKey`], which is what makes it survive a rescan, a remap, and the same folder
//! being opened by its other Unicode spelling.

use sortlens_core::ColorLabel;
use sortlens_core::pathkey::PathKey;

/// A folder's sidebar mark.
///
/// A mark with neither field set is the default and is not stored at all: a snapshot should not fill with rows
/// saying "nothing", and `is_empty` is how the catalog decides.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FolderMark {
    /// Pinned to the top of the folder list.
    pub favourite: bool,
    /// The colour dot beside the name.
    pub colour: Option<ColorLabel>,
}

impl FolderMark {
    /// A mark that says nothing, which the catalog drops rather than stores.
    pub fn is_empty(self) -> bool {
        !self.favourite && self.colour.is_none()
    }
}

/// Every folder mark, keyed by path identity.
pub type FolderMarks = std::collections::BTreeMap<String, FolderMark>;

/// The key a folder's mark is stored under.
pub fn key_of(path: &str) -> String {
    PathKey::of_display(path).as_str().to_string()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_mark_says_nothing_and_a_set_one_says_what_it_has() {
        assert!(FolderMark::default().is_empty());
        assert!(
            !FolderMark {
                favourite: true,
                colour: None
            }
            .is_empty()
        );
        assert!(
            !FolderMark {
                favourite: false,
                colour: Some(ColorLabel::Red)
            }
            .is_empty()
        );
        // The mark round-trips through the log and the snapshot as plain JSON.
        let mark = FolderMark {
            favourite: true,
            colour: Some(ColorLabel::Blue),
        };
        let text = serde_json::to_string(&mark).expect("serialise");
        assert_eq!(serde_json::from_str::<FolderMark>(&text).expect("parse"), mark);
        // An unknown field is a mistake, not something to ignore silently.
        assert!(serde_json::from_str::<FolderMark>(r#"{"favourite":true,"sparkle":1}"#).is_err());
    }

    #[test]
    fn a_key_is_a_path_identity_not_a_string() {
        assert_eq!(key_of("/photos/2024/"), key_of("/photos/2024"));
        // NFC and NFD of the same name are one key.
        assert_eq!(key_of("/photos/Caf\u{e9}"), key_of("/photos/Cafe\u{301}"));
    }
}
