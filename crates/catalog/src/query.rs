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
    /// Only photos carrying these keywords. An empty list disables the filter.
    pub tags: Vec<String>,
    /// Only photos where some keyword came from this source. `None` disables the filter.
    ///
    /// "Some keyword", not every keyword: the question provenance answers is "did a model touch this photo's
    /// keywords", so a photo with one accepted suggestion and ten typed words belongs in the answer. The per-keyword
    /// question is answered by the tag chip's own hover, not by this filter.
    pub tag_source: Option<sortlens_core::TagSource>,
    /// Whether every keyword must match or just one of them.
    pub tag_mode: TagMode,
    /// Only photos in this collection. Membership is resolved by the catalog, not here.
    pub collection: Option<sortlens_core::CollectionId>,
    /// Only photos, raws or videos.
    pub kind: Option<sortlens_core::MediaKind>,
    /// Only photos recorded into the catalog at or after this instant, seconds since the Unix epoch.
    ///
    /// This is the Recently Added question, and it is answered from `Photo::added`, never from the file's
    /// `mtime`: a photo whose add time Sortlens never recorded (`added: None`) cannot answer it and is
    /// excluded, rather than standing in with a modified time it did not choose.
    pub added_after: Option<i64>,
    /// `Some(true)` for edited photos only, `Some(false)` for unedited only.
    pub edited: Option<bool>,
    /// Restrict to files that have gone away (`Some(true)`) or to the ones that are still there
    /// (`Some(false)`).
    ///
    /// This is the sidebar's Missing scope. It is a *narrowing*, so it outranks
    /// [`Self::include_missing`], which only widens the view: "everything, missing included" and "only
    /// what is missing" cannot both be satisfied, and the narrower question is the deliberately asked one.
    pub missing: Option<bool>,
    /// Include entries whose file is missing.
    pub include_missing: bool,
    /// Restricted id set, used by selection and by collection views.
    pub ids: Option<Vec<sortlens_core::PhotoId>>,
}

impl Filter {
    /// Whether a photo passes.
    pub fn matches(&self, photo: &Photo) -> bool {
        if let Some(missing) = self.missing {
            // Only-missing is not the same question as "also missing": a restriction wins, and a photo
            // that fails it does not come back by asking to include missing ones.
            if photo.missing != missing {
                return false;
            }
        } else if !self.include_missing && photo.missing {
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
        if !self.tags.is_empty() {
            let hits = self.tags.iter().filter(|tag| photo.has_tag(tag)).count();
            let pass = match self.tag_mode {
                TagMode::Any => hits > 0,
                TagMode::All => hits == self.tags.len(),
            };
            if !pass {
                return false;
            }
        }
        if let Some(source) = self.tag_source
            && !photo.carries_source(source)
        {
            return false;
        }
        if let Some(after) = self.added_after
            && photo.added.is_none_or(|added| added < after)
        {
            return false;
        }
        if let Some(kind) = self.kind
            && photo.kind != kind
        {
            return false;
        }
        if let Some(edited) = self.edited
            && photo.edited != edited
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
    /// Recently added first: the record's birth, not the file's mtime.
    RecentlyAdded,
}

impl Sort {
    /// Every order, in menu order.
    pub const ALL: [Self; 7] = [
        Self::Natural,
        Self::RecentlyAdded,
        Self::Name,
        Self::Newest,
        Self::Oldest,
        Self::Largest,
        Self::Rating,
    ];

    /// Stable id for settings and the command layer.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Natural => "added",
            Self::RecentlyAdded => "recently-added",
            Self::Name => "name",
            Self::Newest => "newest",
            Self::Oldest => "oldest",
            Self::Largest => "largest",
            Self::Rating => "rating",
        }
    }

    /// Label for the header control.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Natural => "Added order",
            Self::RecentlyAdded => "Recently added first",
            Self::Name => "Name",
            Self::Newest => "Newest first",
            Self::Oldest => "Oldest first",
            Self::Largest => "Largest first",
            Self::Rating => "Highest rated",
        }
    }

    /// Parse a stored id, falling back to the default order.
    pub fn parse(value: &str) -> Self {
        Self::ALL.into_iter().find(|sort| sort.id() == value).unwrap_or_default()
    }

    /// Compare two photos for this order.
    pub fn compare(self, left: &Photo, right: &Photo) -> Ordering {
        match self {
            Self::Natural => left.id.cmp(&right.id),
            // Newest record first, and a photo whose add time was never recorded sinks to the end
            // rather than pretending to be from 1970. Ties go to the later id, because ids are handed
            // out in add order, so the tie-break is the same question answered more coarsely.
            Self::RecentlyAdded => right.added.cmp(&left.added).then(right.id.cmp(&left.id)),
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

/// How several keyword filters combine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagMode {
    /// A photo matches when it carries any of the keywords.
    #[default]
    Any,
    /// A photo matches only when it carries every keyword.
    All,
}

impl TagMode {
    /// The other mode, for the toggle.
    pub const fn flipped(self) -> Self {
        match self {
            Self::Any => Self::All,
            Self::All => Self::Any,
        }
    }

    /// Label for the chip.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::All => "all",
        }
    }
}

/// How the keyword list is ordered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagOrder {
    /// Most used first, then alphabetically.
    #[default]
    Count,
    /// Alphabetically, ignoring case.
    Name,
}

impl TagOrder {
    /// The other order, for the toggle.
    pub const fn flipped(self) -> Self {
        match self {
            Self::Count => Self::Name,
            Self::Name => Self::Count,
        }
    }

    /// Label for the toggle.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Count => "most used",
            Self::Name => "A–Z",
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

    /// Provenance filters ask about *some* keyword, not all of them: "did a model touch this photo" is answered by
    /// one accepted suggestion among ten typed words.
    #[test]
    fn a_source_filter_asks_whether_any_keyword_came_from_there() {
        let mut photo = photo(1, "a.jpg", 0, Flag::None, None);
        photo.tags = vec!["typed".to_string(), "guessed".to_string()];
        photo.tag_sources.insert("typed".to_string(), sortlens_core::TagSource::Manual);
        photo.tag_sources.insert("guessed".to_string(), sortlens_core::TagSource::Ai);

        let from = |source| Filter {
            tag_source: Some(source),
            ..Filter::default()
        };
        assert!(from(sortlens_core::TagSource::Ai).matches(&photo));
        assert!(from(sortlens_core::TagSource::Manual).matches(&photo));
        assert!(!from(sortlens_core::TagSource::Xmp).matches(&photo));
        assert!(!from(sortlens_core::TagSource::Unrecorded).matches(&photo));
        assert!(Filter::default().matches(&photo), "no source asked for means no source constraint");

        // A photo with nothing recorded is the unrecorded case, and only that case.
        let mut legacy = photo.clone();
        legacy.tag_sources.clear();
        assert!(from(sortlens_core::TagSource::Unrecorded).matches(&legacy));
        assert!(!from(sortlens_core::TagSource::Manual).matches(&legacy));
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

    /// The Missing scope asks for what is gone and for nothing else, and the restriction outranks the
    /// widening flag — otherwise "Missing" would mean "Missing and everything else".
    #[test]
    fn only_missing_narrows_while_include_missing_widens() {
        let present = photo(1, "here.jpg", 0, Flag::None, None);
        let mut gone = photo(2, "gone.jpg", 0, Flag::None, None);
        gone.missing = true;

        let only = Filter {
            missing: Some(true),
            ..Filter::default()
        };
        assert!(only.matches(&gone));
        assert!(!only.matches(&present));

        let only_present = Filter {
            missing: Some(false),
            ..Filter::default()
        };
        assert!(only_present.matches(&present));
        assert!(!only_present.matches(&gone));

        // Asked for only-missing *and* include-missing: the narrower question is the one that answers.
        let contradictory = Filter {
            missing: Some(true),
            include_missing: true,
            ..Filter::default()
        };
        assert!(contradictory.matches(&gone));
        assert!(!contradictory.matches(&present));
        let contradictory = Filter {
            missing: Some(false),
            include_missing: true,
            ..Filter::default()
        };
        assert!(
            !contradictory.matches(&gone),
            "a present-only filter does not get the missing ones back"
        );
        assert!(contradictory.matches(&present));
    }

    #[test]
    fn sorting_is_total_and_stable_across_equal_keys() {
        let a = photo(1, "same.jpg", 3, Flag::None, None);
        let b = photo(2, "same.jpg", 3, Flag::None, None);
        assert_eq!(Sort::Name.compare(&a, &b), Ordering::Less);
        assert_eq!(Sort::Newest.compare(&a, &b), Ordering::Greater);
    }

    /// SL-A14 part 2: the Recently Added window answers from the recorded add instant. A photo whose
    /// add time was never recorded stays out, rather than standing in with a modified time.
    #[test]
    fn the_recent_window_answers_from_the_recorded_add_instant() {
        let now = crate::photo::now_epoch();
        let day: i64 = 24 * 60 * 60;
        let mut fresh = photo(1, "fresh.jpg", 0, Flag::None, None);
        fresh.added = Some(now);
        let mut old = photo(2, "old.jpg", 0, Flag::None, None);
        old.added = Some(now - 40 * day);
        let unknown = photo(3, "unknown.jpg", 0, Flag::None, None);
        assert!(unknown.added.is_none(), "from_scan records nothing");
        let filter = Filter {
            added_after: Some(now - 30 * day),
            ..Filter::default()
        };
        assert!(filter.matches(&fresh));
        assert!(!filter.matches(&old), "added 40 days ago is not a recent addition");
        assert!(!filter.matches(&unknown), "a photo with no recorded add instant cannot answer");
    }

    /// The Recently Added order puts the newest record first, sinks photos with no recorded instant
    /// to the end, and breaks ties by the later id, which ids being handed out in add order make
    /// the same question answered more coarsely.
    #[test]
    fn recently_added_orders_by_the_record_and_sinks_the_unknown() {
        let mut newest = photo(1, "new.jpg", 0, Flag::None, None);
        newest.added = Some(200);
        let mut older = photo(2, "old.jpg", 0, Flag::None, None);
        older.added = Some(100);
        let mut tied = photo(3, "tied.jpg", 0, Flag::None, None);
        tied.added = Some(200);
        let unknown = photo(4, "unknown.jpg", 0, Flag::None, None);
        assert_eq!(Sort::RecentlyAdded.compare(&newest, &older), Ordering::Less);
        assert_eq!(
            Sort::RecentlyAdded.compare(&newest, &tied),
            Ordering::Greater,
            "the later id wins the tie"
        );
        assert_eq!(Sort::RecentlyAdded.compare(&newest, &unknown), Ordering::Less, "unknown sinks last");
        assert_eq!(Sort::parse("recently-added"), Sort::RecentlyAdded);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod sort_tests {
    use super::*;

    #[test]
    fn sorts_round_trip_through_their_ids() {
        for sort in Sort::ALL {
            assert_eq!(Sort::parse(sort.id()), sort);
            assert!(!sort.label().is_empty());
        }
        assert_eq!(Sort::parse("nonsense"), Sort::Natural, "unknown ids fall back");
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<&str> = Sort::ALL.iter().map(|sort| sort.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Sort::ALL.len());
    }
}
