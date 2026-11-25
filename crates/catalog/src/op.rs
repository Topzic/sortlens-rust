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
    /// Where keywords came from, on the listed photos. Each map is the complete new set for that photo.
    ///
    /// A separate op from [`Self::Tagged`] rather than a field on it, for one reason: the op log and the snapshots
    /// written before this build are 2-tuples, and a tag write that also carried provenance would either break them
    /// or need a bespoke decoder. Two ops committed as one [`Self::Group`] undo together, so a tag typed by hand is
    /// still a single ⌘Z — the property the user cares about — while the bytes already on disk still replay.
    TagSourced {
        changes: Vec<(PhotoId, std::collections::BTreeMap<String, sortlens_core::TagSource>)>,
    },
    /// Keywords a photo's owner does not want proposed again. Each list is the complete read-then-dismissed set for
    /// that photo, so the inverse is the previous set — the same shape as [`Self::Tagged`].
    ///
    /// This is stored rather than computed because the suggestions themselves are derived from the file and the packs:
    /// a dismissal kept only in the panel would last until the panel was reopened, which is a button that does nothing.
    SuggestionDismissed { changes: Vec<(PhotoId, Vec<String>)> },
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
            Self::TagSourced { .. } => "tagSourced",
            Self::SuggestionDismissed { .. } => "suggestionDismissed",
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
            // A group that only says where a keyword came from, around one real change, *is* that change: the change
            // list should name what happened to the library, not the bookkeeping that went with it.
            Self::Group { ops } => match described(ops).as_slice() {
                [only] => only.name(),
                _ => "group",
            },
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
            Self::TagSourced { changes } => changes.is_empty(),
            Self::SuggestionDismissed { changes } => changes.is_empty(),
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
            // Provenance rides with the words it describes, so counting its photos as well would report a keyword
            // added to forty photos as eighty. A source-only change is described by its own summary instead.
            Self::TagSourced { .. } => 0,
            Self::SuggestionDismissed { changes } => changes.len(),
            Self::Relocate { changes } => changes.len(),
            Self::Retry { changes } => changes.len(),
            Self::Aliased { .. } => 0,
            Self::Collect { added, removed, .. } => added.len() + removed.len(),
            Self::Group { ops } => ops.iter().map(Self::photo_count).sum(),
            _ => 0,
        }
    }

    /// The change as a piece a language can word.
    ///
    /// `summary` is this piece in English: the change viewer prints the piece in the interface
    /// language, while the log, the support report and the command line keep the English sentence
    /// — so a translated viewer and an English report can never describe different work.
    pub fn description(&self) -> Description {
        match self {
            Self::AddRoot { path } => Description::Indexed { path: path.clone() },
            Self::RemoveRoot { path } => Description::StoppedIndexing { path: path.clone() },
            Self::AddPhoto { photo } => Description::PhotoAdded { rel: photo.rel.clone() },
            Self::RemovePhoto { photo } => Description::PhotoRemoved { rel: photo.rel.clone() },
            Self::Missing { id, missing: true } => Description::MarkedMissing { id: id.0 },
            Self::Missing { id, missing: false } => Description::FoundAgain { id: id.0 },
            Self::Rate { changes } => Description::Rated { photos: changes.len() },
            Self::Flag { changes } => Description::Flagged { photos: changes.len() },
            Self::Label { changes } => Description::Labelled { photos: changes.len() },
            Self::Captured { changes } => Description::CaptureDate { photos: changes.len() },
            Self::Tagged { changes } => Description::Keyworded { photos: changes.len() },
            Self::TagSourced { changes } => Description::KeywordSources { photos: changes.len() },
            Self::SuggestionDismissed { changes } => {
                let keywords: usize = changes.iter().map(|(_, tags)| tags.len()).sum();
                if keywords == 0 {
                    Description::SuggestionsNone
                } else {
                    Description::SuggestionsStopped { keywords }
                }
            }
            Self::Relocate { changes } => Description::Relocated { photos: changes.len() },
            Self::Retry { changes } => Description::RetryQueued { photos: changes.len() },
            Self::Aliased { changes } => Description::Aliases { aliases: changes.len() },
            Self::RemapRoot { from, to } => Description::Repointed {
                from: from.clone(),
                to: to.clone(),
            },
            Self::AddCollection { collection } => Description::CollectionCreated {
                name: collection.name.clone(),
            },
            Self::RemoveCollection { collection } => Description::CollectionDeleted {
                name: collection.name.clone(),
            },
            Self::RenameCollection { name, .. } => Description::CollectionRenamed { name: name.clone() },
            Self::Collect { added, removed, .. } => Description::CollectionChanged {
                added: added.len(),
                removed: removed.len(),
            },
            Self::SetCollectionColour { colour, .. } => match colour {
                Some(colour) => Description::CollectionColour { colour: Some(*colour) },
                None => Description::CollectionColour { colour: None },
            },
            Self::SetCollectionFavourite { favourite, .. } => Description::CollectionFavourite { starred: *favourite },
            Self::SetCollectionSort { .. } => Description::CollectionSorted,
            Self::SetCollectionSearch { .. } => Description::CollectionSearched,
            Self::SetPack { name, on } => Description::Pack {
                name: name.clone(),
                on: *on,
            },
            Self::Scores { changes } => Description::Sharpness { photos: changes.len() },
            Self::SetFolderMark { key, mark } => Description::FolderMark {
                key: key.clone(),
                favourite: mark.favourite,
                colour: mark.colour,
            },
            Self::Group { ops } => {
                // Provenance is not a second change a person made: it is the receipt for the keyword they typed. Left
                // in, every ordinary tag would read "Keyworded 1 photo (+1 more change(s))", which is noise about
                // bookkeeping rather than news about the library.
                let real = described(ops);
                match real.first() {
                    Some(first) => {
                        if real.len() == 1 {
                            first.description()
                        } else {
                            Description::GroupedMore {
                                first: Box::new(first.description()),
                                others: real.len() - 1,
                            }
                        }
                    }
                    None => match ops.first() {
                        Some(first) => first.description(),
                        None => Description::GroupedNothing,
                    },
                }
            }
        }
    }

    /// One sentence describing the change, for the change list and the support report.
    pub fn summary(&self) -> String {
        self.description().english()
    }
}

/// What a change did, as a piece a language can word: the counts, paths and names travel with it,
/// so the interface fills its own sentence rather than receiving an English one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Description {
    Indexed {
        path: String,
    },
    StoppedIndexing {
        path: String,
    },
    PhotoAdded {
        rel: String,
    },
    PhotoRemoved {
        rel: String,
    },
    MarkedMissing {
        id: u64,
    },
    FoundAgain {
        id: u64,
    },
    Rated {
        photos: usize,
    },
    Flagged {
        photos: usize,
    },
    Labelled {
        photos: usize,
    },
    CaptureDate {
        photos: usize,
    },
    Keyworded {
        photos: usize,
    },
    KeywordSources {
        photos: usize,
    },
    SuggestionsNone,
    SuggestionsStopped {
        keywords: usize,
    },
    Relocated {
        photos: usize,
    },
    RetryQueued {
        photos: usize,
    },
    Aliases {
        aliases: usize,
    },
    Repointed {
        from: String,
        to: String,
    },
    CollectionCreated {
        name: String,
    },
    CollectionDeleted {
        name: String,
    },
    CollectionRenamed {
        name: String,
    },
    CollectionChanged {
        added: usize,
        removed: usize,
    },
    CollectionColour {
        colour: Option<ColorLabel>,
    },
    CollectionFavourite {
        starred: bool,
    },
    CollectionSorted,
    CollectionSearched,
    Pack {
        name: String,
        on: bool,
    },
    Sharpness {
        photos: usize,
    },
    FolderMark {
        key: String,
        favourite: bool,
        colour: Option<ColorLabel>,
    },
    GroupedMore {
        first: Box<Description>,
        others: usize,
    },
    GroupedNothing,
}

impl Description {
    /// The piece as the English sentence the log, the report and the command line print.
    pub fn english(&self) -> String {
        match self {
            Self::Indexed { path } => format!("Indexed {path}"),
            Self::StoppedIndexing { path } => format!("Stopped indexing {path}"),
            Self::PhotoAdded { rel } => format!("Added {rel}"),
            Self::PhotoRemoved { rel } => format!("Removed {rel}"),
            Self::MarkedMissing { id } => format!("Marked photo #{id} as missing"),
            Self::FoundAgain { id } => format!("Found photo #{id} again"),
            Self::Rated { photos: count } => format!("Rated {}", photos(*count)),
            Self::Flagged { photos: count } => format!("Flagged {}", photos(*count)),
            Self::Labelled { photos: count } => format!("Labelled {}", photos(*count)),
            Self::CaptureDate { photos: count } => format!("Set the capture date on {}", photos(*count)),
            Self::Keyworded { photos: count } => format!("Keyworded {}", photos(*count)),
            Self::KeywordSources { photos: count } => {
                format!("Recorded where keywords came from on {}", photos(*count))
            }
            Self::SuggestionsNone => "Dismissed nothing".to_string(),
            Self::SuggestionsStopped { keywords: 1 } => "Stopped suggesting 1 keyword".to_string(),
            Self::SuggestionsStopped { keywords: count } => format!("Stopped suggesting {count} keywords"),
            Self::Relocated { photos: count } => format!("Found {} somewhere else", photos(*count)),
            Self::RetryQueued { photos: count } => format!("Queued {} for a metadata retry", photos(*count)),
            Self::Aliases { aliases } => format!("Linked {aliases} keyword alias(es)"),
            Self::Repointed { from, to } => format!("Repointed {from} to {to}"),
            Self::CollectionCreated { name } => format!("Created the collection {name}"),
            Self::CollectionDeleted { name } => format!("Deleted the collection {name}"),
            Self::CollectionRenamed { name } => format!("Renamed a collection to {name}"),
            Self::CollectionChanged { added, removed } => format!("Collection change: +{added} -{removed}"),
            Self::CollectionColour { colour: Some(colour) } => format!("Set a collection's colour to {colour:?}"),
            Self::CollectionColour { colour: None } => "Cleared a collection's colour".to_string(),
            Self::CollectionFavourite { starred: true } => "Starred a collection".to_string(),
            Self::CollectionFavourite { starred: false } => "Unstarred a collection".to_string(),
            Self::CollectionSorted => "Changed a collection's sort".to_string(),
            Self::CollectionSearched => "Changed a collection's search".to_string(),
            Self::Pack { name, on: true } => format!("Switched on the “{name}” keyword pack"),
            Self::Pack { name, on: false } => format!("Switched off the “{name}” keyword pack"),
            Self::Sharpness { photos: count } => format!("Measured the sharpness of {}", photos(*count)),
            Self::FolderMark {
                key,
                favourite: true,
                colour: Some(colour),
            } => format!("Pinned {key} and set it to {colour:?}"),
            Self::FolderMark {
                key,
                favourite: true,
                colour: None,
            } => format!("Pinned {key}"),
            Self::FolderMark {
                key,
                favourite: false,
                colour: Some(colour),
            } => format!("Set {key} to {colour:?}"),
            Self::FolderMark {
                key,
                favourite: false,
                colour: None,
            } => format!("Cleared the mark on {key}"),
            Self::GroupedMore { first, others } => {
                format!("{} (+{others} more change(s))", first.english())
            }
            Self::GroupedNothing => "Grouped change".to_string(),
        }
    }
}

/// The operations in a group that a person would recognise as changes.
///
/// Provenance travels in the same group as the keyword it describes, so it is left out: a keyword typed by hand is
/// one change with a receipt, not two changes.
fn described(ops: &[Op]) -> Vec<&Op> {
    ops.iter().filter(|op| !matches!(op, Op::TagSourced { .. })).collect()
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

    /// A keyword typed by hand is one change, not two: the provenance rides with the word and must not read as a
    /// second edit or count the photos twice.
    #[test]
    fn a_keyword_and_its_provenance_read_as_one_change() {
        let op = Op::Group {
            ops: vec![
                Op::Tagged {
                    changes: vec![(PhotoId(1), vec!["cat".into()])],
                },
                Op::TagSourced {
                    changes: vec![(
                        PhotoId(1),
                        [("cat".to_string(), sortlens_core::TagSource::Manual)].into_iter().collect(),
                    )],
                },
            ],
        };
        assert_eq!(op.summary(), "Keyworded 1 photo", "the receipt is not a second change");
        assert_eq!(op.photo_count(), 1, "one photo, not two");
        assert!(!op.is_noop());
        // A change that is *only* provenance still describes itself, because it does change what the library knows.
        let only = Op::TagSourced {
            changes: vec![(
                PhotoId(1),
                [("cat".to_string(), sortlens_core::TagSource::Ai)].into_iter().collect(),
            )],
        };
        assert_eq!(only.summary(), "Recorded where keywords came from on 1 photo");
        assert_eq!(only.name(), "tagSourced");
    }

    /// The change viewer's sentence is a piece first, and its English is that piece joined: the
    /// two cannot drift, and a language words the piece itself rather than an English string.
    #[test]
    fn the_summary_is_joined_from_the_description_piece() {
        let photo = Photo::from_scan(PhotoId(3), "/p", "a/b.jpg", 10, Some(5), MediaKind::Image, 9);
        let cases: Vec<(Op, Description, &str)> = vec![
            (
                Op::AddRoot { path: "/p".into() },
                Description::Indexed { path: "/p".into() },
                "Indexed /p",
            ),
            (
                Op::AddPhoto {
                    photo: Box::new(photo.clone()),
                },
                Description::PhotoAdded { rel: "a/b.jpg".into() },
                "Added a/b.jpg",
            ),
            (
                Op::Rate {
                    changes: vec![(PhotoId(1), 4)],
                },
                Description::Rated { photos: 1 },
                "Rated 1 photo",
            ),
            (
                Op::Rate {
                    changes: vec![(PhotoId(1), 4), (PhotoId(2), 2), (PhotoId(3), 5)],
                },
                Description::Rated { photos: 3 },
                "Rated 3 photos",
            ),
            (
                Op::Scores {
                    changes: vec![(PhotoId(1), None), (PhotoId(2), None)],
                },
                Description::Sharpness { photos: 2 },
                "Measured the sharpness of 2 photos",
            ),
            (
                Op::SuggestionDismissed {
                    changes: vec![(PhotoId(1), vec!["cat".into()])],
                },
                Description::SuggestionsStopped { keywords: 1 },
                "Stopped suggesting 1 keyword",
            ),
            (
                Op::SuggestionDismissed { changes: Vec::new() },
                Description::SuggestionsNone,
                "Dismissed nothing",
            ),
            (
                Op::SetPack {
                    name: "Kitchen".into(),
                    on: true,
                },
                Description::Pack {
                    name: "Kitchen".into(),
                    on: true,
                },
                "Switched on the “Kitchen” keyword pack",
            ),
            (
                Op::SetFolderMark {
                    key: "Trips".into(),
                    mark: crate::folder::FolderMark {
                        favourite: true,
                        colour: Some(ColorLabel::Green),
                        ..crate::folder::FolderMark::default()
                    },
                },
                Description::FolderMark {
                    key: "Trips".into(),
                    favourite: true,
                    colour: Some(ColorLabel::Green),
                },
                "Pinned Trips and set it to Green",
            ),
            (
                Op::Collect {
                    id: CollectionId(1),
                    added: vec![PhotoId(1)],
                    removed: vec![PhotoId(2), PhotoId(3)],
                },
                Description::CollectionChanged { added: 1, removed: 2 },
                "Collection change: +1 -2",
            ),
            (
                Op::Aliased {
                    changes: vec![("kitten".into(), Some("cat".into()))],
                },
                Description::Aliases { aliases: 1 },
                "Linked 1 keyword alias(es)",
            ),
        ];
        for (op, piece, english) in cases {
            assert_eq!(op.description(), piece, "{}", op.name());
            assert_eq!(op.summary(), english, "{}", op.name());
            assert_eq!(piece.english(), english, "{}", op.name());
        }
        // A group of one real change is that change; two read as the first plus how many followed.
        let one = Op::Group {
            ops: vec![Op::Rate {
                changes: vec![(PhotoId(1), 4)],
            }],
        };
        assert_eq!(one.description(), Description::Rated { photos: 1 });
        let two = Op::Group {
            ops: vec![
                Op::Rate {
                    changes: vec![(PhotoId(1), 4)],
                },
                Op::Flag {
                    changes: vec![(PhotoId(1), Flag::Pick)],
                },
            ],
        };
        assert_eq!(
            two.description(),
            Description::GroupedMore {
                first: Box::new(Description::Rated { photos: 1 }),
                others: 1
            }
        );
        assert_eq!(two.summary(), "Rated 1 photo (+1 more change(s))");
        assert_eq!(Op::Group { ops: Vec::new() }.description(), Description::GroupedNothing);
        assert_eq!(Op::Group { ops: Vec::new() }.summary(), "Grouped change");
    }

    #[test]
    fn the_tag_names_the_operation() {
        let text = serde_json::to_string(&Op::AddRoot { path: "/p".into() }).expect("serialise");
        assert!(text.contains("\"op\":\"addRoot\""), "{text}");
    }
}
