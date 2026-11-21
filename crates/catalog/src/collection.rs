//! Named sets of photos. A collection is a membership list, never a copy: the photos stay where
//! they are on disk and keep their ids.

use serde::{Deserialize, Serialize};
use sortlens_core::{CollectionId, PhotoId};
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
}

impl Collection {
    /// A new, empty collection.
    pub fn new(id: CollectionId, name: impl Into<String>, created: i64) -> Self {
        Self {
            id,
            name: name.into(),
            photos: BTreeSet::new(),
            created,
        }
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
    fn membership_is_a_set() {
        let mut collection = Collection::new(CollectionId(1), "Best of 2024", 0);
        collection.photos.insert(PhotoId(3));
        collection.photos.insert(PhotoId(3));
        assert_eq!(collection.len(), 1);
        assert!(collection.contains(PhotoId(3)));
        assert!(!collection.contains(PhotoId(4)));
    }
}
