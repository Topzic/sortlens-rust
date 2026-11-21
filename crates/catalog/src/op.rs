//! Operations: the only way the catalog changes.
//!
//! Every mutation is one serialisable value. The log is a sequence of them, undo is the inverse
//! of the last one, and a snapshot is the result of applying all of them. Nothing else writes to
//! disk, which is why replay and crash recovery are the same code path.

use crate::photo::Photo;
use serde::{Deserialize, Serialize};
use sortlens_core::{CollectionId, ColorLabel, Flag, PhotoId};

/// One change to the catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Op {
    /// A root folder was added.
    AddRoot { path: String },
    /// A root folder was removed. Photos stay until a rescan or an explicit prune.
    RemoveRoot { path: String },
    /// A photo was added or replaced. Replacing keeps the id and therefore the annotations.
    AddPhoto { photo: Box<Photo> },
    /// A photo was removed from the catalog; the record travels with the op so undo can restore it.
    RemovePhoto { photo: Box<Photo> },
    /// A file appeared or disappeared on disk.
    Missing { id: PhotoId, missing: bool },
    /// Stars changed. Only entries whose value actually changed are recorded.
    Rate { changes: Vec<(PhotoId, u8)> },
    /// Flags changed.
    Flag { changes: Vec<(PhotoId, Flag)> },
    /// Colour labels changed.
    Label { changes: Vec<(PhotoId, Option<ColorLabel>)> },
    /// Keyword tags replaced on the listed photos. Each list is the complete new set, sorted.
    Tagged { changes: Vec<(PhotoId, Vec<String>)> },
    /// Photos were found at a new path inside the same root.
    Relocate { changes: Vec<(PhotoId, String)> },
    /// A collection was created.
    AddCollection { collection: Box<crate::collection::Collection> },
    /// A collection was removed; the record travels with the op so undo restores it.
    RemoveCollection { collection: Box<crate::collection::Collection> },
    /// A collection was renamed.
    RenameCollection { id: CollectionId, name: String },
    /// Photos were added to and removed from a collection.
    Collect {
        id: CollectionId,
        added: Vec<PhotoId>,
        removed: Vec<PhotoId>,
    },
    /// A collection's colour dot was set or cleared.
    SetCollectionColour { id: CollectionId, colour: Option<ColorLabel> },
    /// A collection was pinned or unpinned.
    SetCollectionFavourite { id: CollectionId, favourite: bool },
    /// Several operations that must undo together, such as applying a cull batch.
    Group { ops: Vec<Op> },
}

impl Op {
    /// Stable operation name, for logs and tests.
    pub fn name(&self) -> &'static str {
        match self {
            Self::AddRoot { .. } => "addRoot",
            Self::RemoveRoot { .. } => "removeRoot",
            Self::AddPhoto { .. } => "addPhoto",
            Self::RemovePhoto { .. } => "removePhoto",
            Self::Missing { .. } => "missing",
            Self::Rate { .. } => "rate",
            Self::Flag { .. } => "flag",
            Self::Label { .. } => "label",
            Self::Tagged { .. } => "tagged",
            Self::Relocate { .. } => "relocate",
            Self::AddCollection { .. } => "addCollection",
            Self::RemoveCollection { .. } => "removeCollection",
            Self::RenameCollection { .. } => "renameCollection",
            Self::Collect { .. } => "collect",
            Self::SetCollectionColour { .. } => "setCollectionColour",
            Self::SetCollectionFavourite { .. } => "setCollectionFavourite",
            Self::Group { .. } => "group",
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use sortlens_core::MediaKind;

    #[test]
    fn operations_round_trip_through_json() {
        let photo = Photo::from_scan(PhotoId(3), "/p", "a/b.jpg", 10, Some(5), MediaKind::Image, 9);
        let ops = vec![
            Op::AddRoot { path: "/p".into() },
            Op::AddPhoto {
                photo: Box::new(photo.clone()),
            },
            Op::Rate {
                changes: vec![(PhotoId(3), 4)],
            },
            Op::Flag {
                changes: vec![(PhotoId(3), Flag::Pick)],
            },
            Op::Label {
                changes: vec![(PhotoId(3), Some(ColorLabel::Green))],
            },
            Op::Missing {
                id: PhotoId(3),
                missing: true,
            },
            Op::RemovePhoto { photo: Box::new(photo) },
        ];
        for op in ops {
            let text = serde_json::to_string(&op).expect("serialise");
            let back: Op = serde_json::from_str(&text).expect("deserialise");
            assert_eq!(back, op);
        }
    }

    #[test]
    fn the_tag_names_the_operation() {
        let text = serde_json::to_string(&Op::AddRoot { path: "/p".into() }).expect("serialise");
        assert!(text.contains("\"op\":\"addRoot\""), "{text}");
    }
}
