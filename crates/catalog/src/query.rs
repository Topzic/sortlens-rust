//! Filtering, sorting and paging.
//!
//! The grid asks for a page, never for the whole catalog. That keeps the frame cost flat at
//! 100,000 photos and makes the same query reusable from the CLI and from MCP.

use crate::photo::Photo;
use serde::{Deserialize, Serialize};
use sortlens_core::{ColorLabel, Flag};
use std::cmp::Ordering;

/// A composable filter. Empty fields do not constrain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    /// Only photos under this root (exact match on the stored root string).
    pub root: Option<String>,
    /// Only photos whose `rel` starts with this prefix, e.g. `2026/trip`.
    pub folder: Option<String>,
    /// Case-insensitive substring of the file name or relative path.
    pub text: String,
    /// Minimum stars; 0 disables.
    pub min_rating: u8,
    /// Exact star count; `None` disables.
    pub rating: Option<u8>,
    /// Only this flag.
    pub flag: Option<Flag>,
    /// Only this label.
    pub label: Option<ColorLabel>,
    /// Include entries whose file is missing.
    pub include_missing: bool,
    /// Restricted id set, used by selection and by collection views.
    pub ids: Option<Vec<sortlens_core::PhotoId>>,
}

impl Filter {
    /// Whether a photo passes.
    pub fn matches(&self, photo: &Photo) -> bool {
        if !self.include_missing && photo.missing {
            return false;
        }
        if let Some(root) = &self.root
            && &photo.root != root
        {
            return false;
        }
        if let Some(folder) = &self.folder
            && !photo.rel.starts_with(folder.as_str())
        {
            return false;
        }
        if !self.text.is_empty() {
            let needle = self.text.to_lowercase();
            if !photo.name.to_lowercase().contains(&needle) && !photo.rel.to_lowercase().contains(&needle) {
                return false;
            }
        }
        if self.min_rating > 0 && photo.rating < self.min_rating {
            return false;
        }
        if let Some(rating) = self.rating
            && photo.rating != rating
        {
            return false;
        }
        if let Some(flag) = self.flag
            && photo.flag != flag
        {
            return false;
        }
        if let Some(label) = self.label
            && photo.label != Some(label)
        {
            return false;
        }
        if let Some(ids) = &self.ids
            && !ids.contains(&photo.id)
        {
            return false;
        }
        true
    }
}

/// Grid order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Sort {
    /// Insertion order, which is scan order.
    #[default]
    Natural,
    /// File name, case-insensitive.
    Name,
    /// Capture time, newest first.
    Newest,
    /// Capture time, oldest first.
    Oldest,
    /// Largest file first.
    Largest,
    /// Most stars first.
    Rating,
}

impl Sort {
    /// Compare two photos for this order.
    pub fn compare(self, left: &Photo, right: &Photo) -> Ordering {
        match self {
            Self::Natural => left.id.cmp(&right.id),
            Self::Name => left
                .name
                .to_lowercase()
                .cmp(&right.name.to_lowercase())
                .then(left.id.cmp(&right.id)),
            Self::Newest => right.timestamp().cmp(&left.timestamp()).then(left.id.cmp(&right.id)),
            Self::Oldest => left.timestamp().cmp(&right.timestamp()).then(left.id.cmp(&right.id)),
            Self::Largest => right.size.cmp(&left.size).then(left.id.cmp(&right.id)),
            Self::Rating => right.rating.cmp(&left.rating).then(left.id.cmp(&right.id)),
        }
    }
}

/// One page of results.
#[derive(Debug, Default)]
pub struct Page {
    /// The photos in this window.
    pub photos: Vec<Photo>,
    /// How many photos match the filter in total.
    pub total: usize,
    /// Offset this page starts at.
    pub offset: usize,
}

impl Page {
    /// Offset of the next page, if any.
    pub fn next_offset(&self) -> Option<usize> {
        let end = self.offset + self.photos.len();
        (end < self.total).then_some(end)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use sortlens_core::{MediaKind, PhotoId};

    fn photo(id: u64, name: &str, rating: u8, flag: Flag, label: Option<ColorLabel>) -> Photo {
        let mut photo = Photo::from_scan(
            PhotoId(id),
            "/p",
            &format!("trip/{name}"),
            100 + id,
            Some(id as i64),
            MediaKind::Image,
            id,
        );
        photo.rating = rating;
        photo.flag = flag;
        photo.label = label;
        photo
    }

    #[test]
    fn empty_filter_matches_everything_visible() {
        let filter = Filter::default();
        assert!(filter.matches(&photo(1, "a.jpg", 0, Flag::None, None)));
    }

    #[test]
    fn text_matches_name_and_path_case_insensitively() {
        let mut filter = Filter {
            text: "TRIP".into(),
            ..Filter::default()
        };
        assert!(filter.matches(&photo(1, "a.jpg", 0, Flag::None, None)));
        filter.text = "b.jpg".into();
        assert!(!filter.matches(&photo(1, "a.jpg", 0, Flag::None, None)));
    }

    #[test]
    fn rating_flag_and_label_compose() {
        let filter = Filter {
            min_rating: 3,
            flag: Some(Flag::Pick),
            ..Filter::default()
        };
        assert!(filter.matches(&photo(1, "a.jpg", 4, Flag::Pick, None)));
        assert!(!filter.matches(&photo(1, "a.jpg", 2, Flag::Pick, None)));
        assert!(!filter.matches(&photo(1, "a.jpg", 4, Flag::Reject, None)));
    }

    #[test]
    fn missing_photos_are_hidden_unless_asked_for() {
        let mut missing = photo(1, "a.jpg", 0, Flag::None, None);
        missing.missing = true;
        assert!(!Filter::default().matches(&missing));
        assert!(
            Filter {
                include_missing: true,
                ..Filter::default()
            }
            .matches(&missing)
        );
    }

    #[test]
    fn sorting_is_total_and_stable_across_equal_keys() {
        let a = photo(1, "same.jpg", 3, Flag::None, None);
        let b = photo(2, "same.jpg", 3, Flag::None, None);
        assert_eq!(Sort::Name.compare(&a, &b), Ordering::Less);
        assert_eq!(Sort::Newest.compare(&a, &b), Ordering::Greater);
    }
}
