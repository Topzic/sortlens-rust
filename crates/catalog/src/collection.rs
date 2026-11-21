//! Named sets of photos. A collection is a membership list, never a copy: the photos stay where
//! they are on disk and keep their ids.

use serde::{Deserialize, Serialize};
use sortlens_core::{CollectionId, ColorLabel, PhotoId};
use std::collections::BTreeSet;

/// A named set of photos.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection {
    /// Stable id for the lifetime of the catalog.
    pub id: CollectionId,
    /// The name the user sees.
    pub name: String,
    /// Member photos.
    #[serde(default)]
    pub photos: BTreeSet<PhotoId>,
    /// Unix seconds when it was created.
    #[serde(default)]
    pub created: i64,
    /// Colour dot in the sidebar, independent of the photos' own labels.
    #[serde(default)]
    pub colour: Option<ColorLabel>,
    /// Pinned to the top of the collection list.
    #[serde(default)]
    pub favourite: bool,
}

impl Collection {
    /// A new, empty collection.
    pub fn new(id: CollectionId, name: impl Into<String>, created: i64) -> Self {
        Self {
            id,
            name: name.into(),
            photos: BTreeSet::new(),
            created,
            colour: None,
            favourite: false,
        }
    }

    /// The colour's name, for menus and the JSON face.
    pub fn colour_id(&self) -> Option<&'static str> {
        self.colour.map(sortlens_core::ColorLabel::id)
    }

    /// Whether the collection contains a photo.
    pub fn contains(&self, id: PhotoId) -> bool {
        self.photos.contains(&id)
    }

    /// How many photos it holds.
    pub fn len(&self) -> usize {
        self.photos.len()
    }

    /// Whether it is empty.
    pub fn is_empty(&self) -> bool {
        self.photos.is_empty()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_new_collection_has_no_colour_and_is_not_a_favourite() {
        let collection = Collection::new(CollectionId(1), "Trip", 0);
        assert!(collection.colour.is_none());
        assert!(!collection.favourite);
        assert_eq!(collection.colour_id(), None);
    }

    #[test]
    fn an_old_record_without_the_new_fields_still_loads() {
        let text = r#"{"id":1,"name":"Trip","photos":[],"created":10}"#;
        let collection: Collection = serde_json::from_str(text).expect("loads");
        assert!(collection.colour.is_none());
        assert!(!collection.favourite, "missing fields default to absent");
    }

    #[test]
    fn membership_is_a_set() {
        let mut collection = Collection::new(CollectionId(1), "Best of 2024", 0);
        collection.photos.insert(PhotoId(3));
        collection.photos.insert(PhotoId(3));
        assert_eq!(collection.len(), 1);
        assert!(collection.contains(PhotoId(3)));
        assert!(!collection.contains(PhotoId(4)));
    }
}
