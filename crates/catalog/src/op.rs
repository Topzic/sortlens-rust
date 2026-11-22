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
    /// Capture times changed, where EXIF was wrong or missing.
    Captured { changes: Vec<(PhotoId, Option<i64>)> },
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
    /// A keyword now means another one, so later uses of the old name follow the rename.
    Aliased { changes: Vec<(String, Option<String>)> },
    /// A photo's metadata read failed (or was retried successfully).
    Retry { changes: Vec<(PhotoId, bool)> },
    /// A whole root was repointed at a new location, which rewrites every member's root.
    RemapRoot { from: String, to: String },
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
    /// The sort a collection is browsed with was remembered.
    SetCollectionSort { id: CollectionId, sort: Option<String> },
    /// The search a collection was last narrowed with was remembered.
    SetCollectionSearch { id: CollectionId, search: Option<String> },
    /// A keyword pack was switched on or off. Packs are vocabulary files, so the log records only the
    /// switch — the words live on disk where someone can read them.
    SetPack { name: String, on: bool },
    /// A folder's sidebar mark (pin and colour) changed.
    ///
    /// The whole mark is recorded rather than the field that moved, because the inverse of "the whole mark" is
    /// "the previous whole mark": a field-by-field op would need the caller to say which field, and the log
    /// would then be the only place that knows the other one's value. An empty mark removes the entry, so a
    /// folder with nothing set costs no space in the snapshot.
    SetFolderMark { key: String, mark: crate::folder::FolderMark },
    /// Sharpness scores were measured. `None` clears a score, which is how a re-measure at a new `SCORE_VERSION`
    /// and a clear-out both travel through the log.
    ///
    /// The whole batch is one op, so measuring a library is one undo step rather than one per photograph — the same
    /// reason a cull batch is a `Group`. It also keeps the log small: a 5 000-photo scan is a single entry.
    Scores {
        changes: Vec<(PhotoId, Option<sortlens_quality::Score>)>,
    },
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
            Self::Captured { .. } => "captured",
            Self::Tagged { .. } => "tagged",
            Self::Relocate { .. } => "relocate",
            Self::Retry { .. } => "retry",
            Self::Aliased { .. } => "aliased",
            Self::RemapRoot { .. } => "remapRoot",
            Self::AddCollection { .. } => "addCollection",
            Self::RemoveCollection { .. } => "removeCollection",
            Self::RenameCollection { .. } => "renameCollection",
            Self::Collect { .. } => "collect",
            Self::SetCollectionColour { .. } => "setCollectionColour",
            Self::SetCollectionFavourite { .. } => "setCollectionFavourite",
            Self::SetCollectionSort { .. } => "setCollectionSort",
            Self::SetCollectionSearch { .. } => "setCollectionSearch",
            Self::SetPack { .. } => "setPack",
            Self::SetFolderMark { .. } => "setFolderMark",
            Self::Scores { .. } => "scores",
            Self::Group { .. } => "group",
        }
    }

    /// Whether the op changed nothing.
    ///
    /// Anything that rewrites a value computes its inverse from what it actually changed, so an op
    /// whose inverse is empty did nothing. Recording one would fill the log with no-ops and, worse,
    /// spend the single undo level on it — the bug the capture-date test caught.
    pub fn is_noop(&self) -> bool {
        match self {
            Self::AddRoot { .. }
            | Self::RemoveRoot { .. }
            | Self::AddPhoto { .. }
            | Self::RemovePhoto { .. }
            | Self::Missing { .. }
            | Self::AddCollection { .. }
            | Self::RemoveCollection { .. }
            | Self::RenameCollection { .. }
            | Self::RemapRoot { .. }
            | Self::SetCollectionColour { .. }
            | Self::SetCollectionFavourite { .. }
            | Self::SetCollectionSort { .. }
            | Self::SetCollectionSearch { .. }
            | Self::SetFolderMark { .. }
            | Self::SetPack { .. } => false,
            Self::Rate { changes } => changes.is_empty(),
            Self::Scores { changes } => changes.is_empty(),
            Self::Flag { changes } => changes.is_empty(),
            Self::Captured { changes } => changes.is_empty(),
            Self::Label { changes } => changes.is_empty(),
            Self::Tagged { changes } => changes.is_empty(),
            Self::Relocate { changes } => changes.is_empty(),
            Self::Retry { changes } => changes.is_empty(),
            Self::Aliased { changes } => changes.is_empty(),
            Self::Collect { added, removed, .. } => added.is_empty() && removed.is_empty(),
            Self::Group { ops } => ops.iter().all(Self::is_noop),
        }
    }

    /// How many photos the op touches, for a change list.
    pub fn photo_count(&self) -> usize {
        match self {
            Self::AddPhoto { .. } | Self::RemovePhoto { .. } => 1,
            Self::Missing { .. } => 1,
            Self::Rate { changes } => changes.len(),
            Self::Scores { changes } => changes.len(),
            Self::Flag { changes } => changes.len(),
            Self::Label { changes } => changes.len(),
            Self::Captured { changes } => changes.len(),
            Self::Tagged { changes } => changes.len(),
            Self::Relocate { changes } => changes.len(),
            Self::Retry { changes } => changes.len(),
            Self::Aliased { .. } => 0,
            Self::Collect { added, removed, .. } => added.len() + removed.len(),
            Self::Group { ops } => ops.iter().map(Self::photo_count).sum(),
            _ => 0,
        }
    }

    /// One sentence describing the change, for the change list and the support report.
    pub fn summary(&self) -> String {
        match self {
            Self::AddRoot { path } => format!("Indexed {path}"),
            Self::RemoveRoot { path } => format!("Stopped indexing {path}"),
            Self::AddPhoto { photo } => format!("Added {}", photo.rel),
            Self::RemovePhoto { photo } => format!("Removed {}", photo.rel),
            Self::Missing { id, missing: true } => format!("Marked photo #{} as missing", id.0),
            Self::Missing { id, missing: false } => format!("Found photo #{} again", id.0),
            Self::Rate { changes } => format!("Rated {}", photos(changes.len())),
            Self::Flag { changes } => format!("Flagged {}", photos(changes.len())),
            Self::Label { changes } => format!("Labelled {}", photos(changes.len())),
            Self::Captured { changes } => {
                format!("Set the capture date on {}", photos(changes.len()))
            }
            Self::Tagged { changes } => format!("Keyworded {}", photos(changes.len())),
            Self::Relocate { changes } => format!("Found {} somewhere else", photos(changes.len())),
            Self::Retry { changes } => format!("Queued {} for a metadata retry", photos(changes.len())),
            Self::Aliased { changes } => {
                format!("Linked {} keyword alias(es)", changes.len())
            }
            Self::RemapRoot { from, to } => format!("Repointed {from} to {to}"),
            Self::AddCollection { collection } => format!("Created the collection {}", collection.name),
            Self::RemoveCollection { collection } => format!("Deleted the collection {}", collection.name),
            Self::RenameCollection { name, .. } => format!("Renamed a collection to {name}"),
            Self::Collect { added, removed, .. } => {
                format!("Collection change: +{} -{}", added.len(), removed.len())
            }
            Self::SetCollectionColour { colour, .. } => match colour {
                Some(colour) => format!("Set a collection's colour to {colour:?}"),
                None => "Cleared a collection's colour".to_string(),
            },
            Self::SetCollectionFavourite { favourite, .. } => {
                if *favourite {
                    "Starred a collection".to_string()
                } else {
                    "Unstarred a collection".to_string()
                }
            }
            Self::SetCollectionSort { .. } => "Changed a collection's sort".to_string(),
            Self::SetCollectionSearch { .. } => "Changed a collection's search".to_string(),
            Self::SetPack { name, on: true } => format!("Switched on the “{name}” keyword pack"),
            Self::SetPack { name, on: false } => format!("Switched off the “{name}” keyword pack"),
            Self::Scores { changes } => match changes.len() {
                1 => "Measured the sharpness of 1 photo".to_string(),
                count => format!("Measured the sharpness of {count} photos"),
            },
            Self::SetFolderMark { key, mark } => match (mark.favourite, mark.colour) {
                (true, Some(colour)) => format!("Pinned {key} and set it to {colour:?}"),
                (true, None) => format!("Pinned {key}"),
                (false, Some(colour)) => format!("Set {key} to {colour:?}"),
                (false, None) => format!("Cleared the mark on {key}"),
            },
            Self::Group { ops } => match ops.first() {
                Some(first) => {
                    if ops.len() == 1 {
                        first.summary()
                    } else {
                        format!("{} (+{} more change(s))", first.summary(), ops.len() - 1)
                    }
                }
                None => "Grouped change".to_string(),
            },
        }
    }
}

/// `1 photo` / `3 photos`, so summaries read like sentences.
fn photos(count: usize) -> String {
    if count == 1 {
        "1 photo".to_string()
    } else {
        format!("{count} photos")
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
