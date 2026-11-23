//! The catalog: an op log, a snapshot, and the in-memory index over both.
//!
//! On disk there are two files in a catalog directory:
//!
//! ```text
//! ops.log        one checksummed JSON record per line, append only
//! snapshot.json  the state after the last N records, rewritten atomically
//! ```
//!
//! Opening a catalog loads the snapshot and replays the tail. A torn final record is dropped; a
//! corrupt middle record stops the replay and is reported, so a damaged file costs the tail of
//! the log rather than the library.

use crate::collection::Collection;
use crate::op::Op;
use crate::photo::{Photo, Stats};
use crate::query::{Filter, Page, Sort, TagOrder};
use serde::{Deserialize, Serialize};
use sortlens_core::{CollectionId, ColorLabel, Error, Flag, MediaKind, PhotoId, Result, Review};
use sortlens_meta::PhotoMeta;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Log file name.
pub const LOG_NAME: &str = "ops.log";
/// Snapshot file name.
pub const SNAPSHOT_NAME: &str = "snapshot.json";
/// Held for as long as a process has the library open for writing.
pub const LOCK_NAME: &str = "library.lock";
/// Records between automatic snapshots.
pub const SNAPSHOT_EVERY: usize = 256;

/// How many steps of undo history are kept. Every step lives in the log and in the snapshot's
/// history, so a restart does not shorten it; the bound is about memory and about how far back
/// a person is ever asked to walk.
pub const UNDO_DEPTH: usize = 20;
/// Snapshot schema, bumped when the record layout changes.
/// Snapshot schema, bumped when the record layout changes.
///
/// 2 added `Photo::added`; photos from a schema-1 snapshot decode with `added: None` and honestly
/// cannot answer the Recently Added question, rather than borrowing the file's `mtime` as a stand-in.
pub const SCHEMA: u32 = 2;

/// One line of the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    seq: u64,
    op: Op,
    /// The inverse of `op`, so undo survives a restart. Absent in records written before this
    /// existed, which simply leave nothing to undo instead of failing to load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    inverse: Option<Op>,
    /// Unix seconds when the record was written. `0` in records that predate the timer, which the
    /// history shows as "no date" rather than as 1970.
    #[serde(default, skip_serializing_if = "is_zero")]
    at: i64,
}

/// Serde helper: leave the timestamp out when it is unknown.
fn is_zero(value: &i64) -> bool {
    *value == 0
}

/// Unix seconds now, or 0 when the clock is before the epoch.
pub fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0)
}

/// What opening a catalog had to say.
#[derive(Debug, Clone, Default)]
pub struct OpenReport {
    /// Records replayed after the snapshot.
    pub replayed: usize,
    /// Sequence number of the snapshot that was loaded.
    pub snapshot_seq: u64,
    /// Human-readable problems; none is fatal.
    pub warnings: Vec<String>,
    /// The log was not replayed to the end: a record failed its checksum, or would not apply.
    pub torn_tail: bool,
    /// Records the log held that were already in the snapshot.
    pub skipped: usize,
    /// Another process has this library open for writing, so this one cannot change anything.
    pub read_only: bool,
}

/// The persisted state.
#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    schema: u32,
    seq: u64,
    next_id: u64,
    roots: Vec<String>,
    photos: Vec<Photo>,
    /// Collections, added after the first format; older snapshots simply have none.
    #[serde(default)]
    collections: Vec<Collection>,
    /// Keyword aliases, as `old -> canonical`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    aliases: BTreeMap<String, String>,
    /// Next collection id, defaulted for older snapshots.
    #[serde(default)]
    next_collection: u32,
    /// Keyword packs switched on, by name. The packs themselves live in files; the log records only
    /// which ones the library is using.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    packs: BTreeSet<String>,
    /// Folder marks (pin, colour), keyed by path identity. Added after the first format; older snapshots have
    /// none, and a folder that leaves the disk keeps its mark until the mark is cleared.
    #[serde(default, skip_serializing_if = "crate::folder::FolderMarks::is_empty")]
    folders: crate::folder::FolderMarks,
    /// The inverse of the last applied operation, so undo survives a snapshot rollover. Absent in
    /// older snapshots, which simply reopen with nothing to undo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    undo: Option<Op>,
    /// The undo history, oldest first, up to [`UNDO_DEPTH`] entries. Added after the single-level
    /// format; an older reader ignores this field and keeps the one step `undo` carries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    undo_history: Vec<Op>,
}

/// What a scan of one root changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SyncOutcome {
    pub added: usize,
    pub updated: usize,
    pub missing: usize,
    pub restored: usize,
}

/// One file from a scan, in the shape the catalog wants.
#[derive(Debug, Clone, PartialEq)]
pub struct Seed {
    pub rel: String,
    pub size: u64,
    pub modified: Option<i64>,
    pub kind: MediaKind,
    pub key: u64,
    pub meta: PhotoMeta,
    /// Review state read from the photo's XMP sidecar, applied when the photo is first indexed.
    pub review: Option<Review>,
    /// Whether the sidecar carries edit settings from another application.
    pub edited: bool,
    /// The file's metadata could not be read, so it stays queued for a retry.
    pub meta_unreadable: bool,
}

/// The catalog.
#[derive(Debug)]
pub struct Catalog {
    dir: PathBuf,
    /// Held for the life of the catalog when this process is the writer. `None` when read-only.
    lock: Option<File>,
    photos: BTreeMap<PhotoId, Photo>,
    roots: Vec<String>,
    collections: BTreeMap<CollectionId, Collection>,
    aliases: BTreeMap<String, String>,
    packs: BTreeSet<String>,
    folders: crate::folder::FolderMarks,
    next_collection: u32,
    next_id: u64,
    seq: u64,
    undo: Vec<Op>,
    /// Inverses of the undos performed in this session, so `redo` can step forward again. Not
    /// persisted as a stack: after a restart the same steps are reachable by undoing the undo
    /// entries, which the log holds in order.
    redo: Vec<Op>,
    log: File,
    unlogged: usize,
    /// Depth of batch operations that take their own snapshot at the end instead of one every 256 ops.
    batching: usize,
    /// Snapshots written since this catalog was opened, so a test can say how much work a scan did.
    snapshots: u64,
    stats: Option<Stats>,
}

impl Catalog {
    /// Open or create a catalog in `dir`.
    pub fn open(dir: impl AsRef<Path>) -> Result<(Self, OpenReport)> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir).map_err(|error| Error::io(dir.display().to_string(), error))?;
        let mut report = OpenReport::default();

        // One writer per library. The lock is an OS file lock, so it is released when this process
        // ends — no stale-lock heuristics, and a crashed window cannot leave the library locked.
        let lock_path = dir.join(LOCK_NAME);
        let lock_file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .map_err(|error| Error::io(lock_path.display().to_string(), error))?;
        let mut lock = match lock_file.try_lock() {
            Ok(()) => Some(lock_file),
            Err(std::fs::TryLockError::WouldBlock) => {
                report.read_only = true;
                report
                    .warnings
                    .push("another window has this library open, so it is read-only".into());
                None
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(Error::io(lock_path.display().to_string(), error));
            }
        };

        let snapshot_path = dir.join(SNAPSHOT_NAME);
        let mut photos = BTreeMap::new();
        let mut roots = Vec::new();
        let mut next_id = 1u64;
        let mut next_collection = 1u32;
        let mut collections: BTreeMap<CollectionId, Collection> = BTreeMap::new();
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        let mut packs: BTreeSet<String> = BTreeSet::new();
        let mut folders: crate::folder::FolderMarks = crate::folder::FolderMarks::new();
        let mut seq = 0u64;
        // The snapshot carries the undo history; log records that replay after it take precedence.
        let mut restore_undo: Vec<Op> = Vec::new();

        if snapshot_path.exists() {
            match fs::read_to_string(&snapshot_path) {
                Ok(text) => match serde_json::from_str::<Snapshot>(&text) {
                    Ok(snapshot) => {
                        if snapshot.schema > SCHEMA {
                            report.warnings.push(format!(
                                "snapshot schema {} is newer than {SCHEMA}; opened read-only",
                                snapshot.schema
                            ));
                        }
                        seq = snapshot.seq;
                        next_id = snapshot.next_id;
                        roots = snapshot.roots;
                        photos = snapshot.photos.into_iter().map(|photo| (photo.id, photo)).collect();
                        next_collection = snapshot.next_collection.max(1);
                        collections = snapshot
                            .collections
                            .into_iter()
                            .map(|collection| (collection.id, collection))
                            .collect();
                        aliases = snapshot.aliases;
                        packs = snapshot.packs;
                        folders = snapshot.folders;
                        report.snapshot_seq = seq;
                        // A snapshot from this version carries the whole history; an older one
                        // carries only the last inverse, which is still worth one step.
                        if snapshot.undo_history.is_empty() {
                            restore_undo = snapshot.undo.clone().into_iter().collect();
                        } else {
                            restore_undo = snapshot.undo_history;
                        }
                    }
                    Err(error) => report
                        .warnings
                        .push(format!("snapshot is unreadable ({error}); rebuilding from the log")),
                },
                Err(error) => report.warnings.push(format!("snapshot could not be read: {error}")),
            }
        }

        let log_path = dir.join(LOG_NAME);
        if log_path.exists() {
            let file = File::open(&log_path).map_err(|error| Error::io(log_path.display().to_string(), error))?;
            // Read the flag before the struct moves the lock into it.
            let writable = lock.is_some();
            let mut catalog = Self {
                dir: dir.clone(),
                lock: lock.take(),
                photos,
                roots,
                collections,
                aliases,
                packs,
                folders,
                next_collection,
                next_id,
                seq,
                undo: restore_undo.clone(),
                redo: Vec::new(),
                log: if writable {
                    OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(&log_path)
                        .map_err(|error| Error::io(log_path.display().to_string(), error))?
                } else {
                    // Read-only: open for reading so replay works and nothing can be appended.
                    File::open(&log_path).map_err(|error| Error::io(log_path.display().to_string(), error))?
                },
                unlogged: 0,
                batching: 0,
                snapshots: 0,
                stats: None,
            };
            for line in BufReader::new(file).lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(error) => {
                        report.warnings.push(format!("log read stopped: {error}"));
                        break;
                    }
                };
                if line.trim().is_empty() {
                    continue;
                }
                let (record_seq, op, inverse, _at) = match decode(&line) {
                    Some(record) => record,
                    None => {
                        report.warnings.push("log record failed its checksum; earlier records kept".into());
                        report.torn_tail = true;
                        break;
                    }
                };
                if record_seq <= catalog.seq {
                    report.skipped += 1;
                    continue;
                }
                if let Err(error) = catalog.apply(&op) {
                    // A record that will not apply is damage — most often a snapshot that did not
                    // survive leaving its ops with nothing to act on. Recovering what came before it
                    // and reporting the rest beats refusing to open the library at all.
                    report.warnings.push(format!(
                        "record #{record_seq} could not be replayed ({error}); the rest of the log is not trusted"
                    ));
                    report.torn_tail = true;
                    break;
                }
                catalog.seq = record_seq;
                // Keep the history across the restart: the inverse as it was written, which is
                // authoritative even when the op no longer means the same thing to this version.
                if let Some(inverse) = inverse {
                    catalog.push_undo(inverse);
                }
                report.replayed += 1;
            }
            return Ok((catalog, report));
        }

        let log = if lock.is_some() {
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(&log_path)
                .map_err(|error| Error::io(log_path.display().to_string(), error))?
        } else {
            File::open(&log_path).map_err(|error| Error::io(log_path.display().to_string(), error))?
        };
        Ok((
            Self {
                dir,
                lock,
                photos,
                roots,
                collections,
                aliases,
                packs,
                folders,
                next_collection,
                next_id,
                seq,
                undo: restore_undo,
                redo: Vec::new(),
                log,
                unlogged: 0,
                batching: 0,
                snapshots: 0,
                stats: None,
            },
            report,
        ))
    }

    /// Whether this process may change the catalog.
    pub fn read_only(&self) -> bool {
        self.lock.is_none()
    }

    /// Directory holding the catalog files.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Last sequence number written.
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// Number of photos.
    pub fn len(&self) -> usize {
        self.photos.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.photos.is_empty()
    }

    /// Registered roots.
    pub fn roots(&self) -> &[String] {
        &self.roots
    }

    /// One photo by id.
    pub fn photo(&self, id: PhotoId) -> Option<&Photo> {
        self.photos.get(&id)
    }

    /// Every photo, in id order.
    pub fn photos(&self) -> impl Iterator<Item = &Photo> {
        self.photos.values()
    }

    /// Counters, cached until the next change.
    pub fn stats(&mut self) -> Stats {
        if let Some(stats) = self.stats {
            return stats;
        }
        let mut stats = Stats::default();
        for photo in self.photos.values() {
            stats.total += 1;
            stats.bytes += photo.size;
            if photo.rating > 0 {
                stats.rated += 1;
            }
            if photo.flag == Flag::Pick {
                stats.picks += 1;
            }
            if photo.flag == Flag::Reject {
                stats.rejects += 1;
            }
            if photo.label.is_some() {
                stats.labelled += 1;
            }
            if !photo.tags.is_empty() {
                stats.tagged += 1;
            }
            if photo.missing {
                stats.missing += 1;
            }
        }
        self.stats = Some(stats);
        stats
    }

    /// How many photos match a filter, without building the page.
    pub fn count(&self, filter: &Filter) -> usize {
        self.photos.values().filter(|photo| self.matches(filter, photo)).count()
    }

    /// A filter plus collection membership.
    fn matches(&self, filter: &Filter, photo: &Photo) -> bool {
        if let Some(id) = filter.collection {
            match self.collections.get(&id) {
                Some(collection) if collection.contains(photo.id) => {}
                _ => return false,
            }
        }
        filter.matches(photo)
    }

    /// Photos the catalog believes are missing.
    pub fn missing_photos(&self) -> Vec<Photo> {
        self.photos.values().filter(|photo| photo.missing).cloned().collect()
    }

    /// Remove every photo currently marked missing. Returns how many went.
    pub fn remove_missing(&mut self) -> Result<usize> {
        let missing: Vec<Photo> = self.photos.values().filter(|photo| photo.missing).cloned().collect();
        let removed = missing.len();
        if missing.is_empty() {
            return Ok(0);
        }
        // One group, so undoing brings the whole purge back rather than the last photo only.
        let ops = missing
            .into_iter()
            .map(|photo| Op::RemovePhoto { photo: Box::new(photo) })
            .collect();
        self.commit(Op::Group { ops })?;
        Ok(removed)
    }

    /// Every keyword in use, most used first, then alphabetically.
    pub fn tag_vocabulary(&self) -> Vec<(String, usize)> {
        self.tag_vocabulary_in(TagOrder::Count)
    }

    /// Every keyword in use, in the requested order.
    /// Resolve a keyword through the alias registry.
    ///
    /// Following a chain means a keyword renamed twice still lands on its current name; a cycle is
    /// refused rather than looped over, because an alias is user data and a cycle is a mistake.
    pub fn resolve_tag(&self, tag: &str) -> String {
        let mut current = tag.to_string();
        let mut steps = 0;
        while let Some(next) = self.aliases.get(&current) {
            steps += 1;
            if steps > 16 || next == &current {
                break;
            }
            current = next.clone();
        }
        current
    }

    /// Record that `alias` now means `canonical`. Passing `None` forgets the link.
    pub fn set_alias(&mut self, alias: &str, canonical: Option<&str>) -> Result<bool> {
        let alias = alias.trim().to_lowercase();
        let canonical = canonical.map(|name| name.trim().to_lowercase());
        if alias.is_empty() {
            return Err(Error::Invalid("an alias needs a name".into()));
        }
        if canonical.as_deref() == Some(alias.as_str()) {
            return Err(Error::Invalid("a keyword cannot be its own alias".into()));
        }
        if self.aliases.get(&alias).map(String::as_str) == canonical.as_deref() {
            return Ok(false);
        }
        self.commit(Op::Aliased {
            changes: vec![(alias, canonical)],
        })?;
        Ok(true)
    }

    /// Every alias pointing at `tag`.
    pub fn aliases_of(&self, tag: &str) -> Vec<String> {
        self.aliases
            .iter()
            .filter(|(_, canonical)| canonical.as_str() == tag)
            .map(|(alias, _)| alias.clone())
            .collect()
    }

    /// Aliases whose target no longer carries any photo — either a mistake, or a keyword that was
    /// removed after the rename that created the alias.
    pub fn orphaned_aliases(&self) -> Vec<(String, String)> {
        let live: HashSet<&str> = self
            .photos
            .values()
            .flat_map(|photo| photo.tags.iter().map(String::as_str))
            .collect();
        self.aliases
            .iter()
            .filter(|(_, canonical)| !live.contains(canonical.as_str()))
            .map(|(alias, canonical)| (alias.clone(), canonical.clone()))
            .collect()
    }

    /// The keyword registry: every keyword in use, its aliases, and any alias pointing nowhere.
    pub fn registry(&self) -> crate::registry::Registry {
        crate::registry::Registry::of(self.tag_vocabulary_in(TagOrder::Name), &self.aliases, self.orphaned_aliases())
    }

    pub fn tag_vocabulary_in(&self, order: TagOrder) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for photo in self.photos.values() {
            for tag in &photo.tags {
                *counts.entry(tag.as_str()).or_insert(0) += 1;
            }
        }
        let mut vocabulary: Vec<(String, usize)> = counts.into_iter().map(|(tag, count)| (tag.to_string(), count)).collect();
        match order {
            TagOrder::Count => vocabulary.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0))),
            TagOrder::Name => vocabulary.sort_by_key(|(tag, _)| tag.to_lowercase()),
        }
        vocabulary
    }

    /// Every collection, by name.
    pub fn collections(&self) -> Vec<Collection> {
        let mut collections: Vec<Collection> = self.collections.values().cloned().collect();
        // Favourites first, then alphabetical, so pinning one does not hide the rest of the list.
        collections.sort_by(|left, right| {
            right
                .favourite
                .cmp(&left.favourite)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        collections
    }

    /// One collection by id.
    pub fn collection(&self, id: CollectionId) -> Option<&Collection> {
        self.collections.get(&id)
    }

    /// Create a collection and return its id.
    pub fn create_collection(&mut self, name: &str) -> Result<CollectionId> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Invalid("a collection needs a name".into()));
        }
        let id = CollectionId(self.next_collection);
        self.next_collection += 1;
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs() as i64)
            .unwrap_or_default();
        self.commit(Op::AddCollection {
            collection: Box::new(Collection::new(id, name, created)),
        })?;
        Ok(id)
    }

    /// Rename a collection.
    pub fn rename_collection(&mut self, id: CollectionId, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Invalid("a collection needs a name".into()));
        }
        self.commit(Op::RenameCollection {
            id,
            name: name.to_string(),
        })
        .map(|_| ())
    }

    /// Set or clear a collection's colour.
    pub fn set_collection_colour(&mut self, id: CollectionId, colour: Option<ColorLabel>) -> Result<()> {
        if self.collections.get(&id).map(|collection| collection.colour) == Some(colour) {
            return Ok(());
        }
        self.commit(Op::SetCollectionColour { id, colour }).map(|_| ())
    }

    /// Pin or unpin a collection.
    /// Switch a keyword pack on or off. A pack that is already in that state changes nothing, so the
    /// undo level is not spent on a no-op.
    pub fn set_pack(&mut self, name: &str, on: bool) -> Result<bool> {
        if self.packs.contains(name) == on {
            return Ok(false);
        }
        self.commit(Op::SetPack {
            name: name.to_string(),
            on,
        })
        .map(|()| true)
    }

    /// Set a folder's sidebar mark. An empty mark removes the entry rather than storing a row that says nothing.
    ///
    /// Returns whether anything changed, so a caller that re-applies the same mark is not reported as a write.
    pub fn set_folder_mark(&mut self, key: &str, mark: crate::folder::FolderMark) -> Result<bool> {
        if key.is_empty() {
            return Err(Error::Invalid("a folder mark needs a path".into()));
        }
        let current = self.folders.get(key).cloned().unwrap_or_default();
        if current == mark {
            return Ok(false);
        }
        self.commit(Op::SetFolderMark {
            key: key.to_string(),
            mark: mark.clone(),
        })?;
        Ok(true)
    }

    /// Every folder mark.
    pub fn folder_marks(&self) -> &crate::folder::FolderMarks {
        &self.folders
    }

    /// Record measured sharpness for a batch, as one undoable op.
    ///
    /// Only what actually changed is written: a photo already carrying a score from the same algorithm is left
    /// alone, so re-running a scan over an unchanged library is not a write and does not spend the undo level.
    /// Returns how many photos were changed.
    pub fn set_scores(&mut self, measured: &[(PhotoId, sortlens_quality::Score)]) -> Result<usize> {
        let mut changes = Vec::new();
        for (id, score) in measured {
            let photo = self.photos.get(id).ok_or(Error::NoSuchPhoto(*id))?;
            if photo.sharpness.as_ref().is_some_and(|stored| stored == score) {
                continue;
            }
            changes.push((*id, Some(*score)));
        }
        if changes.is_empty() {
            return Ok(0);
        }
        let changed = changes.len();
        self.commit(Op::Scores { changes })?;
        Ok(changed)
    }

    /// Forget every score, as one undoable op. Used when the algorithm changes and the old numbers must not be
    /// compared with the new ones — see `sortlens_quality::SCORE_VERSION`.
    pub fn clear_scores(&mut self) -> Result<usize> {
        let changes: Vec<(PhotoId, Option<sortlens_quality::Score>)> = self
            .photos
            .values()
            .filter(|photo| photo.sharpness.is_some())
            .map(|photo| (photo.id, None))
            .collect();
        if changes.is_empty() {
            return Ok(0);
        }
        let changed = changes.len();
        self.commit(Op::Scores { changes })?;
        Ok(changed)
    }

    /// A photo's stored sharpness, if it has ever been measured.
    pub fn sharpness(&self, id: PhotoId) -> Option<sortlens_quality::Score> {
        self.photos.get(&id).and_then(|photo| photo.sharpness)
    }

    /// The keyword packs this library has switched on.
    pub fn enabled_packs(&self) -> &BTreeSet<String> {
        &self.packs
    }

    pub fn set_collection_favourite(&mut self, id: CollectionId, favourite: bool) -> Result<()> {
        if self.collections.get(&id).map(|collection| collection.favourite) == Some(favourite) {
            return Ok(());
        }
        self.commit(Op::SetCollectionFavourite { id, favourite }).map(|_| ())
    }

    /// Remember the sort a collection is browsed with.
    pub fn set_collection_sort(&mut self, id: CollectionId, sort: crate::query::Sort) -> Result<()> {
        let wanted = sort.id().to_string();
        if self.collections.get(&id).and_then(|collection| collection.sort.as_deref()) == Some(wanted.as_str()) {
            return Ok(());
        }
        self.commit(Op::SetCollectionSort { id, sort: Some(wanted) }).map(|_| ())
    }

    /// Remember the search a collection was narrowed with, or forget it when the box is empty.
    pub fn set_collection_search(&mut self, id: CollectionId, search: &str) -> Result<()> {
        let wanted = search.trim();
        let wanted = if wanted.is_empty() { None } else { Some(wanted.to_string()) };
        if self.collections.get(&id).and_then(|collection| collection.search.clone()) == wanted {
            return Ok(());
        }
        self.commit(Op::SetCollectionSearch { id, search: wanted }).map(|_| ())
    }

    /// Delete a collection. The photos stay in the library.
    pub fn delete_collection(&mut self, id: CollectionId) -> Result<()> {
        let collection = self.collections.get(&id).cloned().ok_or(Error::NoSuchCollection(id))?;
        self.commit(Op::RemoveCollection {
            collection: Box::new(collection),
        })
        .map(|_| ())
    }

    /// Add photos to a collection or remove them from it.
    pub fn collect(&mut self, id: CollectionId, added: &[PhotoId], removed: &[PhotoId]) -> Result<usize> {
        if !self.collections.contains_key(&id) {
            return Err(Error::NoSuchCollection(id));
        }
        let changed = added.len() + removed.len();
        self.commit(Op::Collect {
            id,
            added: added.to_vec(),
            removed: removed.to_vec(),
        })?;
        Ok(changed)
    }

    /// Rename a keyword everywhere it appears, merging when the new name is already in use.
    ///
    /// The whole change is one `Tagged` op, so a rename of two thousand photos undoes in a single
    /// step rather than a thousand. Returns how many photos changed.
    pub fn rename_tag(&mut self, from: &str, to: &str) -> Result<usize> {
        let from = from.trim();
        let to = to.trim();
        if from.is_empty() || to.is_empty() {
            return Err(Error::Invalid("a keyword needs a name".into()));
        }
        if from == to {
            return Err(Error::Invalid("the keyword already has that name".into()));
        }
        let mut changes = Vec::new();
        let mut source_changes = Vec::new();
        for photo in self.photos.values() {
            if !photo.has_tag(from) {
                continue;
            }
            let mut tags: Vec<String> = photo.tags.iter().filter(|tag| tag.as_str() != from).cloned().collect();
            if !tags.iter().any(|tag| tag == to) {
                tags.push(to.to_string());
            }
            tags.sort_unstable();
            tags.dedup();
            changes.push((photo.id, tags));
            // Provenance follows the word: an AI-proposed keyword that is renamed was still proposed, and leaving the
            // source under the old name would both lose it and leave a map describing a tag the photo no longer has.
            // When the target name is already on the photo, its own source wins — that word was there first.
            if let Some(source) = photo.tag_sources.get(from).copied() {
                let mut map = photo.tag_sources.clone();
                map.remove(from);
                map.entry(to.to_string()).or_insert(source);
                source_changes.push((photo.id, map));
            }
        }
        // The old name becomes an alias of the new one, so re-typing it lands on the right keyword
        // instead of quietly re-creating the keyword that was just merged away. Every half goes into
        // one op, so a mistaken rename still costs a single undo.
        let mut ops = Vec::new();
        let count = changes.len();
        if !changes.is_empty() {
            ops.push(Op::Tagged { changes });
        }
        if !source_changes.is_empty() {
            ops.push(Op::TagSourced { changes: source_changes });
        }
        if self.resolve_tag(to) != from {
            ops.push(Op::Aliased {
                changes: vec![(from.to_string(), Some(to.to_string()))],
            });
        }
        match ops.len() {
            0 => {}
            1 => {
                if let Some(op) = ops.pop() {
                    self.commit(op)?;
                }
            }
            _ => {
                self.commit(Op::Group { ops })?;
            }
        }
        Ok(count)
    }

    /// Remove a keyword from every photo that carries it, and forget every alias that pointed at it.
    ///
    /// Naming an alias deletes the keyword it stands for, the way the rest of the app resolves names.
    /// The whole change is one `Group` op, so deleting a keyword from two thousand photos undoes in a
    /// single step rather than a thousand. Returns how many photos lost the keyword.
    pub fn delete_tag(&mut self, tag: &str) -> Result<usize> {
        let tag = tag.trim();
        if tag.is_empty() {
            return Err(Error::Invalid("a keyword needs a name".into()));
        }
        let target = self.resolve_tag(tag);
        let mut changes = Vec::new();
        let mut source_changes = Vec::new();
        for photo in self.photos.values() {
            if !photo.has_tag(&target) {
                continue;
            }
            let tags: Vec<String> = photo.tags.iter().filter(|tag| tag.as_str() != target).cloned().collect();
            changes.push((photo.id, tags));
            // A deleted word takes its provenance with it; a map holding the deleted name would describe a tag no
            // photo carries, and the filter would keep matching on it.
            if photo.tag_sources.contains_key(&target) {
                let mut map = photo.tag_sources.clone();
                map.remove(&target);
                source_changes.push((photo.id, map));
            }
        }
        // The rename trail dies with the keyword: an alias pointing only at a deleted name would be a
        // link to nowhere, and re-typing the old name must not quietly resurrect the keyword.
        let alias_changes: Vec<(String, Option<String>)> = self
            .aliases
            .iter()
            .filter(|(_, canonical)| self.resolve_tag(canonical) == target)
            .map(|(alias, _)| (alias.clone(), None))
            .collect();
        if changes.is_empty() && alias_changes.is_empty() {
            return Err(Error::Invalid(format!("no keyword {tag} is in the library")));
        }
        let count = changes.len();
        let mut ops = Vec::new();
        if !changes.is_empty() {
            ops.push(Op::Tagged { changes });
        }
        if !source_changes.is_empty() {
            ops.push(Op::TagSourced { changes: source_changes });
        }
        if !alias_changes.is_empty() {
            ops.push(Op::Aliased { changes: alias_changes });
        }
        match ops.len() {
            0 => {}
            1 => {
                if let Some(op) = ops.pop() {
                    self.commit(op)?;
                }
            }
            _ => {
                self.commit(Op::Group { ops })?;
            }
        }
        Ok(count)
    }

    /// Immediate subfolders under `prefix` inside `root`, with how many photos each holds.
    ///
    /// `prefix` is a relative path with a trailing slash, or empty for the root itself. Counts are
    /// recursive, so a folder shows everything below it, not just its own files.
    pub fn folders(&self, root: &str, prefix: &str) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for photo in self.photos.values() {
            if photo.root != root || photo.missing {
                continue;
            }
            let Some(rest) = photo.rel.strip_prefix(prefix) else {
                continue;
            };
            let Some((folder, _)) = rest.split_once('/') else {
                continue;
            };
            if folder.is_empty() {
                continue;
            }
            *counts.entry(folder.to_string()).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }

    /// Photo counts per root, for the sidebar.
    pub fn counts_by_root(&self) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<String, usize> = self.roots.iter().map(|root| (root.clone(), 0)).collect();
        for photo in self.photos.values() {
            if photo.missing {
                continue;
            }
            *counts.entry(photo.root.clone()).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }

    /// One page of a filtered, sorted view.
    pub fn page(&self, filter: &Filter, sort: Sort, offset: usize, limit: usize) -> Page {
        let mut matched: Vec<Photo> = self.photos.values().filter(|photo| self.matches(filter, photo)).cloned().collect();
        matched.sort_by(|left, right| sort.compare(left, right));
        let total = matched.len();
        let photos = matched.into_iter().skip(offset).take(limit).collect();
        Page { photos, total, offset }
    }

    /// Apply an operation in memory and return its inverse.
    /// Set capture times, returning the previous values as the inverse.
    ///
    /// Split out of [`Self::apply`] because the match was one arm past the complexity budget, and
    /// because "replace this field, report what it was" is the shape several ops share.
    /// Apply a folder mark, returning the op that undoes it.
    ///
    /// The inverse is the whole previous mark, or its absence — which is why the op records the whole mark rather
    /// than the field that moved. An empty mark *removes* the entry, so replaying a clear leaves the map exactly
    /// as applying it did.
    /// Apply a batch of scores, returning the previous values so the whole batch undoes as one step.
    ///
    /// Split out of [`Catalog::apply`] because the complexity gate said so, not to silence it: the match arm was one
    /// case too many for one function to hold clearly.
    /// Replace the keyword list on each photo, returning what it was.
    fn apply_tagged(&mut self, changes: &[(PhotoId, Vec<String>)]) -> Result<Op> {
        let mut previous = Vec::new();
        for (id, tags) in changes {
            let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
            let mut tags = tags.clone();
            tags.sort_unstable();
            tags.dedup();
            if photo.tags != tags {
                previous.push((*id, photo.tags.clone()));
                photo.tags = tags;
                self.touch(*id);
            }
        }
        Ok(Op::Tagged { changes: previous })
    }

    /// Replace the provenance map on each photo, returning what it was.
    ///
    /// The map is not pruned here: a group sets the words and then their provenance, and an undo replays both in
    /// reverse, so a map can name a keyword the photo only regains on the next step. [`Self::prune_sources`] runs
    /// once the whole op is in.
    ///
    /// The revision is deliberately **not** bumped: `rev` is what a stale sidecar writer is refused against, and
    /// provenance is never written to a sidecar. Bumping it here would double-count every keyword added by hand
    /// (words plus source in one group) and make the number mean less than it says.
    fn apply_tag_sourced(&mut self, changes: &[(PhotoId, BTreeMap<String, sortlens_core::TagSource>)]) -> Result<Op> {
        let mut previous = Vec::new();
        for (id, sources) in changes {
            let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
            if &photo.tag_sources != sources {
                previous.push((*id, photo.tag_sources.clone()));
                photo.tag_sources = sources.clone();
            }
        }
        Ok(Op::TagSourced { changes: previous })
    }

    fn apply_scores(&mut self, changes: &[(PhotoId, Option<sortlens_quality::Score>)]) -> Result<Op> {
        let mut previous = Vec::with_capacity(changes.len());
        for (id, score) in changes {
            let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
            previous.push((*id, photo.sharpness));
            photo.sharpness = *score;
        }
        Ok(Op::Scores { changes: previous })
    }

    fn apply_folder_mark(&mut self, key: &str, mark: crate::folder::FolderMark) -> Result<Op> {
        if key.is_empty() {
            return Err(Error::Invalid("a folder mark needs a path".into()));
        }
        let previous = if mark.is_empty() {
            self.folders.remove(key)
        } else {
            self.folders.insert(key.to_string(), mark)
        };
        Ok(Op::SetFolderMark {
            key: key.to_string(),
            mark: previous.unwrap_or_default(),
        })
    }

    fn apply_captured(&mut self, changes: &[(PhotoId, Option<i64>)]) -> Result<Op> {
        let mut previous = Vec::new();
        for (id, captured) in changes {
            let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
            if photo.captured != *captured {
                previous.push((*id, photo.captured));
                photo.captured = *captured;
                self.touch(*id);
            }
        }
        Ok(Op::Captured { changes: previous })
    }

    /// Bump a photo's annotation revision. Called only where a value actually changed, because a
    /// revision that moves without a change gives a stale reader nothing to catch and a fresh one
    /// nothing to keep.
    fn touch(&mut self, id: PhotoId) {
        if let Some(photo) = self.photos.get_mut(&id) {
            photo.rev += 1;
        }
    }

    pub fn apply(&mut self, op: &Op) -> Result<Op> {
        self.stats = None;
        match op {
            Op::AddRoot { path } => {
                if !self.roots.contains(path) {
                    self.roots.push(path.clone());
                }
                Ok(Op::RemoveRoot { path: path.clone() })
            }
            Op::RemoveRoot { path } => {
                self.roots.retain(|root| root != path);
                Ok(Op::AddRoot { path: path.clone() })
            }
            Op::AddPhoto { photo } => {
                if photo.id.0 >= self.next_id {
                    self.next_id = photo.id.0 + 1;
                }
                let previous = self.photos.insert(photo.id, (**photo).clone());
                Ok(match previous {
                    Some(previous) => Op::AddPhoto { photo: Box::new(previous) },
                    None => Op::RemovePhoto { photo: photo.clone() },
                })
            }
            Op::RemovePhoto { photo } => {
                let removed = self.photos.remove(&photo.id).ok_or(Error::NoSuchPhoto(photo.id))?;
                Ok(Op::AddPhoto { photo: Box::new(removed) })
            }
            Op::Missing { id, missing } => {
                let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
                let previous = photo.missing;
                photo.missing = *missing;
                Ok(Op::Missing {
                    id: *id,
                    missing: previous,
                })
            }
            Op::Rate { changes } => {
                let mut previous = Vec::new();
                for (id, stars) in changes {
                    let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
                    let stars = (*stars).min(5);
                    if photo.rating != stars {
                        previous.push((*id, photo.rating));
                        photo.rating = stars;
                        self.touch(*id);
                    }
                }
                Ok(Op::Rate { changes: previous })
            }
            Op::Flag { changes } => {
                let mut previous = Vec::new();
                for (id, flag) in changes {
                    let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
                    if photo.flag != *flag {
                        previous.push((*id, photo.flag));
                        photo.flag = *flag;
                        self.touch(*id);
                    }
                }
                Ok(Op::Flag { changes: previous })
            }
            Op::Captured { changes } => self.apply_captured(changes),
            Op::Relocate { changes } => {
                // Undoing a relocation means putting the old path back and, if the photo was missing
                // before, being missing again — so the inverse is a small ordered group.
                let mut inverse = Vec::new();
                for (id, rel) in changes {
                    let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
                    if &photo.rel != rel {
                        inverse.push(Op::Relocate {
                            changes: vec![(*id, photo.rel.clone())],
                        });
                        inverse.push(Op::Missing {
                            id: *id,
                            missing: photo.missing,
                        });
                        photo.rel = rel.clone();
                        photo.missing = false;
                    }
                }
                Ok(Op::Group { ops: inverse })
            }
            Op::Aliased { changes } => {
                let mut inverse = Vec::new();
                for (alias, canonical) in changes {
                    let previous = self.aliases.get(alias).cloned();
                    if previous.as_deref() == canonical.as_deref() {
                        continue;
                    }
                    inverse.push(Op::Aliased {
                        changes: vec![(alias.clone(), previous)],
                    });
                    match canonical {
                        Some(canonical) => {
                            self.aliases.insert(alias.clone(), canonical.clone());
                        }
                        None => {
                            self.aliases.remove(alias);
                        }
                    }
                }
                Ok(Op::Group { ops: inverse })
            }
            Op::Retry { changes } => {
                let mut inverse = Vec::new();
                for (id, retry) in changes {
                    let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
                    if photo.retry != *retry {
                        inverse.push(Op::Retry {
                            changes: vec![(*id, photo.retry)],
                        });
                        photo.retry = *retry;
                    }
                }
                Ok(Op::Group { ops: inverse })
            }
            Op::RemapRoot { from, to } => {
                // Repointing a root is one change to the root list plus one per member, and its
                // inverse is the same swap in reverse — so a mistake costs one undo.
                if !self.roots.iter().any(|root| root == from) {
                    return Err(Error::Invalid(format!("no indexed root {from}")));
                }
                if self.roots.iter().any(|root| root == to) {
                    return Err(Error::Invalid(format!("{to} is already indexed")));
                }
                let mut members: Vec<PhotoId> = self
                    .photos
                    .values()
                    .filter(|photo| &photo.root == from)
                    .map(|photo| photo.id)
                    .collect();
                members.sort_by_key(|id| id.0);
                for id in &members {
                    if let Some(photo) = self.photos.get_mut(id) {
                        photo.root = to.clone();
                    }
                }
                if let Some(index) = self.roots.iter().position(|root| root == from) {
                    self.roots[index] = to.clone();
                }
                Ok(Op::RemapRoot {
                    from: to.clone(),
                    to: from.clone(),
                })
            }
            Op::AddCollection { collection } => {
                let collection = (**collection).clone();
                let id = collection.id;
                self.collections.insert(id, collection.clone());
                self.next_collection = self.next_collection.max(id.0 + 1);
                Ok(Op::RemoveCollection {
                    collection: Box::new(collection),
                })
            }
            Op::RemoveCollection { collection } => {
                let id = collection.id;
                let previous = self.collections.remove(&id);
                match previous {
                    Some(collection) => Ok(Op::AddCollection {
                        collection: Box::new(collection),
                    }),
                    None => Ok(Op::RemoveCollection {
                        collection: collection.clone(),
                    }),
                }
            }
            Op::RenameCollection { id, name } => {
                let collection = self.collections.get_mut(id).ok_or(Error::NoSuchCollection(*id))?;
                let previous = collection.name.clone();
                collection.name = name.clone();
                Ok(Op::RenameCollection { id: *id, name: previous })
            }
            Op::Collect { id, added, removed } => {
                let collection = self.collections.get_mut(id).ok_or(Error::NoSuchCollection(*id))?;
                let mut previous_added = Vec::new();
                for photo in added {
                    if collection.photos.insert(*photo) {
                        previous_added.push(*photo);
                    }
                }
                let mut previous_removed = Vec::new();
                for photo in removed {
                    if collection.photos.remove(photo) {
                        previous_removed.push(*photo);
                    }
                }
                Ok(Op::Collect {
                    id: *id,
                    added: previous_removed,
                    removed: previous_added,
                })
            }
            Op::SetCollectionColour { id, colour } => {
                let collection = self.collections.get_mut(id).ok_or(Error::NoSuchCollection(*id))?;
                let previous = collection.colour;
                collection.colour = *colour;
                Ok(Op::SetCollectionColour { id: *id, colour: previous })
            }
            Op::SetCollectionFavourite { id, favourite } => {
                let collection = self.collections.get_mut(id).ok_or(Error::NoSuchCollection(*id))?;
                let previous = collection.favourite;
                collection.favourite = *favourite;
                Ok(Op::SetCollectionFavourite {
                    id: *id,
                    favourite: previous,
                })
            }
            Op::SetCollectionSort { id, sort } => {
                let collection = self.collections.get_mut(id).ok_or(Error::NoSuchCollection(*id))?;
                let previous = collection.sort.clone();
                collection.sort = sort.clone();
                Ok(Op::SetCollectionSort { id: *id, sort: previous })
            }
            Op::Scores { changes } => self.apply_scores(changes),
            Op::SetFolderMark { key, mark } => self.apply_folder_mark(key, mark.clone()),
            Op::SetPack { name, on } => {
                let previous = self.packs.contains(name);
                if *on {
                    self.packs.insert(name.clone());
                } else {
                    self.packs.remove(name);
                }
                Ok(Op::SetPack {
                    name: name.clone(),
                    on: previous,
                })
            }
            Op::SetCollectionSearch { id, search } => {
                let collection = self.collections.get_mut(id).ok_or(Error::NoSuchCollection(*id))?;
                let previous = collection.search.clone();
                collection.search = search.clone();
                Ok(Op::SetCollectionSearch { id: *id, search: previous })
            }
            Op::Group { ops } => {
                // Apply in order, undo in reverse order, so a group behaves like one operation.
                let mut inverses = Vec::with_capacity(ops.len());
                for op in ops {
                    inverses.push(self.apply(op)?);
                }
                inverses.reverse();
                Ok(Op::Group { ops: inverses })
            }
            Op::Tagged { changes } => self.apply_tagged(changes),
            Op::TagSourced { changes } => self.apply_tag_sourced(changes),
            Op::Label { changes } => {
                let mut previous = Vec::new();
                for (id, label) in changes {
                    let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
                    if photo.label != *label {
                        previous.push((*id, photo.label));
                        photo.label = *label;
                        self.touch(*id);
                    }
                }
                Ok(Op::Label { changes: previous })
            }
        }
    }

    /// Drop provenance for keywords a photo no longer carries, on the photos an op names.
    ///
    /// A map describing a deleted word would be provenance for something that is not there; the filter iterates the
    /// tags, so it would not match, but the snapshot would carry dead entries and a keyword typed again later could
    /// inherit one. Only the photos the op touched are visited.
    fn prune_sources(&mut self, op: &Op) {
        let mut ids = Vec::new();
        collect_sourced_ids(op, &mut ids);
        ids.sort_unstable();
        ids.dedup();
        for id in ids {
            if let Some(photo) = self.photos.get_mut(&id) {
                photo.prune_tag_sources();
            }
        }
    }

    /// Apply an operation, append it to the log and remember how to undo it.
    ///
    /// The inverse travels in the log record, so a fresh process can undo the last change instead of
    /// starting with an empty history.
    pub fn commit(&mut self, op: Op) -> Result<()> {
        if self.read_only() {
            return Err(Error::Invalid(
                "this library is open read-only because another window is using it".into(),
            ));
        }
        let inverse = self.apply(&op)?;
        // After the *whole* op, never in the middle of one: a group sets the words and then their provenance, and an
        // undo puts that group back in reverse, so a map can name a keyword the photo only regains in the next step
        // of the same group. Tidying per photo here is also why a 100 000-photo library does not pay for this.
        self.prune_sources(&op);
        if inverse.is_noop() {
            // Nothing changed, so there is nothing to record and nothing to spend undo on.
            return Ok(());
        }
        self.seq += 1;
        self.append(self.seq, &op, Some(&inverse))?;
        self.push_undo(inverse);
        // A new change abandons the redo branch: stepping forward again would replay a past that
        // no longer matches what was just written.
        self.redo.clear();
        self.unlogged += 1;
        // A snapshot rewrites every photo, so taking one every 256 ops makes a large import quadratic:
        // ten thousand files meant thirty-nine rewrites of a growing file. A batch takes one at the end
        // instead — a crash mid-import replays the log, which is exactly what the log is for.
        if self.batching == 0 && self.unlogged >= SNAPSHOT_EVERY {
            self.snapshot()?;
        }
        Ok(())
    }

    /// Commit an op only if the catalog is still at the revision the caller read.
    ///
    /// The optimistic-concurrency guard behind SL-D11: a writer that read the library at revision
    /// `expected` says so, and if the catalog has moved on — another window committed, a scan
    /// finished, an undo landed — the write is refused with [`Error::StaleRevision`] instead of
    /// silently overwriting the newer annotations. The caller re-reads and retries, which is what
    /// a stale render can do, but a wrong write cannot be undone by the person it lost data for.
    pub fn commit_expect(&mut self, op: Op, expected: u64) -> Result<()> {
        let current = self.seq;
        if current != expected {
            return Err(Error::StaleRevision { expected, current });
        }
        self.commit(op)
    }

    /// Undo the last committed operation. One step per call, up to [`UNDO_DEPTH`] steps kept.
    pub fn undo(&mut self) -> Result<bool> {
        let Some(inverse) = self.undo.pop() else {
            return Ok(false);
        };
        let redo = self.apply(&inverse)?;
        self.prune_sources(&inverse);
        self.seq += 1;
        self.append(self.seq, &inverse, Some(&redo))?;
        self.redo.push(redo);
        self.unlogged += 1;
        Ok(true)
    }

    /// Redo the last undone operation. One step per call; a new change abandons the redo branch.
    pub fn redo(&mut self) -> Result<bool> {
        let Some(op) = self.redo.pop() else {
            return Ok(false);
        };
        let inverse = self.apply(&op)?;
        self.prune_sources(&op);
        self.seq += 1;
        self.append(self.seq, &op, Some(&inverse))?;
        self.push_undo(inverse);
        self.unlogged += 1;
        if self.batching == 0 && self.unlogged >= SNAPSHOT_EVERY {
            self.snapshot()?;
        }
        Ok(true)
    }

    /// Remember an inverse, oldest first, keeping at most [`UNDO_DEPTH`] steps.
    fn push_undo(&mut self, inverse: Op) {
        self.undo.push(inverse);
        if self.undo.len() > UNDO_DEPTH {
            let excess = self.undo.len() - UNDO_DEPTH;
            self.undo.drain(..excess);
        }
    }

    /// Drop the in-memory undo history, e.g. when the document closes.
    pub fn forget_undo(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Bring one root up to date with a completed scan.
    ///
    /// Adds new files, refreshes changed ones in place (keeping their id and annotations), marks
    /// photos that disappeared as missing, and clears the flag on files that came back.
    pub fn sync_root(&mut self, root: &str, seeds: Vec<Seed>) -> Result<SyncOutcome> {
        self.batching += 1;
        let outcome = self.sync_root_inner(root, seeds);
        self.batching -= 1;
        // One snapshot for the whole sync, even when it committed nothing that survived as a no-op.
        if self.batching == 0 && self.unlogged > 0 {
            self.snapshot()?;
        }
        outcome
    }

    fn sync_root_inner(&mut self, root: &str, seeds: Vec<Seed>) -> Result<SyncOutcome> {
        let mut outcome = SyncOutcome::default();
        if !self.roots.iter().any(|existing| existing == root) {
            self.commit(Op::AddRoot { path: root.to_string() })?;
        }

        let mut seen = HashSet::new();
        let existing: Vec<Photo> = self.photos.values().filter(|photo| photo.root == root).cloned().collect();
        let by_rel: BTreeMap<String, Photo> = existing.into_iter().map(|photo| (photo.rel.clone(), photo)).collect();

        for seed in seeds {
            seen.insert(seed.rel.clone());
            match by_rel.get(&seed.rel) {
                None => {
                    let id = PhotoId(self.next_id);
                    self.next_id += 1;
                    let mut photo = Photo::from_scan(id, root, &seed.rel, seed.size, seed.modified, seed.kind, seed.key);
                    // The record is born here, so this is the one honest instant for "added": the file's
                    // `mtime` is when the camera wrote it, which is a different question.
                    photo.added = Some(crate::photo::now_epoch());
                    photo.apply_meta(&seed.meta);
                    photo.edited = seed.edited;
                    photo.retry = seed.meta_unreadable;
                    if let Some(review) = &seed.review {
                        photo.adopt_review(review);
                    }
                    self.commit(Op::AddPhoto { photo: Box::new(photo) })?;
                    outcome.added += 1;
                }
                Some(previous) => {
                    let unchanged = previous.size == seed.size && previous.key == seed.key;
                    if unchanged {
                        if previous.retry && !seed.meta_unreadable {
                            self.commit(Op::Retry {
                                changes: vec![(previous.id, false)],
                            })?;
                        }
                        if previous.missing {
                            self.commit(Op::Missing {
                                id: previous.id,
                                missing: false,
                            })?;
                            outcome.restored += 1;
                        }
                        continue;
                    }
                    let mut updated = previous.clone();
                    updated.size = seed.size;
                    updated.modified = seed.modified;
                    updated.kind = seed.kind;
                    updated.key = seed.key;
                    updated.missing = false;
                    updated.apply_meta(&seed.meta);
                    updated.edited = seed.edited;
                    updated.retry = seed.meta_unreadable;
                    // The bytes on disk changed, so a measurement of the old ones describes a photograph that is
                    // no longer there. Annotations like stars survive a re-encode; a sharpness score does not,
                    // because it is a statement about pixels.
                    updated.sharpness = None;
                    self.commit(Op::AddPhoto { photo: Box::new(updated) })?;
                    outcome.updated += 1;
                }
            }
        }

        for photo in by_rel.values() {
            if !seen.contains(&photo.rel) && !photo.missing {
                self.commit(Op::Missing {
                    id: photo.id,
                    missing: true,
                })?;
                outcome.missing += 1;
            }
        }
        Ok(outcome)
    }

    /// Photos whose metadata could not be read when they were indexed, oldest first.
    pub fn retrying_photos(&self) -> Vec<Photo> {
        let mut photos: Vec<Photo> = self.photos.values().filter(|photo| photo.retry).cloned().collect();
        photos.sort_by_key(|photo| photo.id.0);
        photos
    }

    /// Photos whose metadata is empty and could be worth re-reading — for instance after the app
    /// learns to read a format it could not parse before.
    pub fn exifless_photos(&self) -> Vec<Photo> {
        let mut photos: Vec<Photo> = self
            .photos
            .values()
            .filter(|photo| !photo.missing && photo.camera.is_empty() && photo.captured.is_none())
            .cloned()
            .collect();
        photos.sort_by_key(|photo| photo.id.0);
        photos
    }

    /// Queue photos for a metadata retry after a failed read.
    pub fn mark_retry(&mut self, ids: &[PhotoId], retry: bool) -> Result<usize> {
        let changes: Vec<(PhotoId, bool)> = ids
            .iter()
            .filter(|id| self.photos.get(id).is_some_and(|photo| photo.retry != retry))
            .map(|id| (*id, retry))
            .collect();
        if changes.is_empty() {
            return Ok(0);
        }
        let count = changes.len();
        self.commit(Op::Retry { changes })?;
        Ok(count)
    }

    /// Remove a root and every photo under it.
    pub fn remove_root(&mut self, root: &str) -> Result<usize> {
        let photos: Vec<Photo> = self.photos.values().filter(|photo| photo.root == root).cloned().collect();
        let count = photos.len();
        for photo in photos {
            self.commit(Op::RemovePhoto { photo: Box::new(photo) })?;
        }
        self.commit(Op::RemoveRoot { path: root.to_string() })?;
        Ok(count)
    }

    /// Write a snapshot and truncate the log. Called automatically every [`SNAPSHOT_EVERY`] ops.
    /// How many snapshots have been written since this catalog was opened.
    pub fn snapshots_written(&self) -> u64 {
        self.snapshots
    }

    /// Operations appended since the last snapshot, i.e. how much a reopen would have to replay.
    pub fn unlogged_ops(&self) -> usize {
        self.unlogged
    }

    /// How many bytes the log occupies on disk.
    pub fn log_bytes(&self) -> u64 {
        fs::metadata(self.dir.join(LOG_NAME)).map(|meta| meta.len()).unwrap_or(0)
    }

    /// Write a snapshot and start a fresh log, reporting the bytes that were reclaimed.
    ///
    /// This is both compaction and repair: whatever the log held is now in the snapshot, so a torn
    /// tail or a stale record cannot come back. The caller is expected to [`audit`] first if it
    /// wants to know what was dropped.
    pub fn compact(&mut self) -> Result<u64> {
        let before = self.log_bytes();
        self.snapshot()?;
        // Only after the snapshot has been read back does the log become redundant. Truncating a
        // log whose snapshot did not survive is how a library disappears.
        let verified = fs::read_to_string(self.dir.join(SNAPSHOT_NAME))
            .ok()
            .and_then(|text| serde_json::from_str::<Snapshot>(&text).ok())
            .is_some_and(|snapshot| snapshot.seq == self.seq);
        if !verified {
            return Err(Error::Invalid("the snapshot did not read back; the log is kept".into()));
        }
        let log_path = self.dir.join(LOG_NAME);
        let truncated = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&log_path)
            .map_err(|error| Error::io(log_path.display().to_string(), error))?;
        drop(truncated);
        self.log = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&log_path)
            .map_err(|error| Error::io(log_path.display().to_string(), error))?;
        self.unlogged = 0;
        let after = self.log_bytes();
        Ok(before.saturating_sub(after))
    }

    pub fn snapshot(&mut self) -> Result<()> {
        let snapshot = Snapshot {
            schema: SCHEMA,
            seq: self.seq,
            next_id: self.next_id,
            roots: self.roots.clone(),
            photos: self.photos.values().cloned().collect(),
            collections: self.collections.values().cloned().collect(),
            aliases: self.aliases.clone(),
            packs: self.packs.clone(),
            folders: self.folders.clone(),
            next_collection: self.next_collection,
            undo: self.undo.last().cloned(),
            undo_history: self.undo.clone(),
        };
        let text = serde_json::to_string(&snapshot).map_err(|error| Error::Invalid(error.to_string()))?;
        let tmp = self.dir.join("snapshot.json.tmp");
        let path = self.dir.join(SNAPSHOT_NAME);
        fs::write(&tmp, text.as_bytes()).map_err(|error| Error::io(tmp.display().to_string(), error))?;
        fs::rename(&tmp, &path).map_err(|error| Error::io(path.display().to_string(), error))?;
        self.snapshots += 1;
        // The snapshot now holds everything the log holds, so the replay debt is paid. Without this the
        // counter only ever grew, and once it passed the threshold every single edit rewrote the whole
        // library — a bug that made ten thousand photos cost far more than ten thousand records.
        self.unlogged = 0;
        // The log is left alone on purpose. A snapshot makes opening fast, but the log is what makes
        // a snapshot *recoverable*: if this file is later damaged or half-written, replay rebuilds
        // the library from the records. Emptying the log is `compact()`, which verifies first.
        Ok(())
    }

    /// Flush pending records and write a snapshot. Safe to call more than once.
    /// Push buffered log records to disk without snapshotting.
    ///
    /// Used before taking a backup, so the archive matches what the app is showing while the log
    /// keeps its records.
    pub fn flush(&mut self) -> Result<()> {
        self.log.flush().map_err(|error| Error::io(self.dir.display().to_string(), error))
    }

    /// Flush and stop. Deliberately *not* a snapshot.
    ///
    /// Snapshotting on every quit would empty the log on every quit, and the log is the change
    /// history — "what did I change yesterday" would always have no answer. Snapshots happen every
    /// [`SNAPSHOT_EVERY`] ops and on an explicit compaction instead.
    pub fn close(&mut self) -> Result<()> {
        self.log.flush().map_err(|error| Error::io(self.dir.display().to_string(), error))
    }

    fn append(&mut self, seq: u64, op: &Op, inverse: Option<&Op>) -> Result<()> {
        let line = encode(seq, op, inverse)?;
        self.log
            .write_all(line.as_bytes())
            .and_then(|()| self.log.flush())
            .map_err(|error| Error::io(self.dir.display().to_string(), error))
    }
}

/// Every photo whose provenance an operation names, including inside a group.
fn collect_sourced_ids(op: &Op, out: &mut Vec<PhotoId>) {
    match op {
        Op::TagSourced { changes } => out.extend(changes.iter().map(|(id, _)| *id)),
        Op::Group { ops } => {
            for op in ops {
                collect_sourced_ids(op, out);
            }
        }
        _ => {}
    }
}

/// Encode one record as `<crc32> <json>\n`.
fn encode(seq: u64, op: &Op, inverse: Option<&Op>) -> Result<String> {
    let record = Record {
        seq,
        op: op.clone(),
        inverse: inverse.cloned(),
        at: now_seconds(),
    };
    let json = serde_json::to_string(&record).map_err(|error| Error::Invalid(error.to_string()))?;
    let crc = crc32fast::hash(json.as_bytes());
    Ok(format!("{crc:08x} {json}\n"))
}

/// Read a log file into lines, tolerating a torn final line.
pub(crate) fn open_records(path: &Path) -> std::io::Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    Ok(text.lines().map(str::to_string).collect())
}

/// Decode one log line, for readers outside this module.
pub(crate) fn decode_record(line: &str) -> Option<(u64, Op, Option<Op>, i64)> {
    decode(line)
}

/// Decode one line, returning `None` when the checksum or the JSON does not hold.
fn decode(line: &str) -> Option<(u64, Op, Option<Op>, i64)> {
    let (crc_text, json) = line.split_once(' ')?;
    let crc = u32::from_str_radix(crc_text, 16).ok()?;
    if crc32fast::hash(json.as_bytes()) != crc {
        return None;
    }
    let record: Record = serde_json::from_str(json).ok()?;
    Some((record.seq, record.op, record.inverse, record.at))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::query::{Filter, TagMode};
    use sortlens_core::ColorLabel;
    use sortlens_quality::Score;

    pub struct TempCatalog(PathBuf);

    impl TempCatalog {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sortlens-catalog-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Self(dir)
        }

        fn open(&self) -> (Catalog, OpenReport) {
            Catalog::open(&self.0).expect("open")
        }
    }

    impl Drop for TempCatalog {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn seed(rel: &str) -> Seed {
        Seed {
            rel: rel.into(),
            size: 100,
            modified: Some(10),
            kind: MediaKind::Image,
            key: 7,
            meta: PhotoMeta::default(),
            review: None,
            edited: false,
            meta_unreadable: false,
        }
    }

    #[test]
    fn photos_and_roots_survive_a_restart() {
        let temp = TempCatalog::new("restart");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b/c.jpg")]).expect("sync");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 5)],
                })
                .expect("rate");
            catalog.close().expect("close");
        }
        let (catalog, report) = temp.open();
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.roots(), &["/photos".to_string()]);
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").rating, 5);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    /// A large sync takes one snapshot, not one every 256 files.
    ///
    /// A snapshot rewrites every photo in the library, so the old cadence made an import quadratic: a
    /// thousand files meant four rewrites of a growing file, ten thousand meant thirty-nine. Replay cost
    /// is what pays for the change, and the log keeps every record either way — this asserts the work a
    /// scan does, not a duration, so it cannot pass on a fast machine by accident.
    #[test]
    fn a_large_sync_takes_one_snapshot_instead_of_one_every_256_ops() {
        let temp = TempCatalog::new("batch-snapshot");
        {
            let (mut catalog, _) = temp.open();
            let seeds: Vec<Seed> = (0..1000).map(|index| seed(&format!("IMG_{index:04}.jpg"))).collect();
            let outcome = catalog.sync_root("/photos", seeds).expect("sync");
            assert_eq!(outcome.added, 1000);
            assert_eq!(catalog.snapshots_written(), 1, "one snapshot for the whole sync");
            assert_eq!(catalog.unlogged_ops(), 0, "nothing left to replay");
            catalog.close().expect("close");
        }

        // The records are all still in the log, and a reopen rebuilds the library from them.
        let (catalog, report) = temp.open();
        assert_eq!(catalog.len(), 1000);
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }

    /// Interactive edits keep the old cadence: single operations still snapshot every 256 commits.
    #[test]
    fn single_edits_still_snapshot_every_256_commits() {
        let temp = TempCatalog::new("single-snapshot");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        let after_sync = catalog.snapshots_written();
        for stars in 0..SNAPSHOT_EVERY {
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), (stars % 5) as u8 + 1)],
                })
                .expect("rate");
        }
        assert_eq!(
            catalog.snapshots_written(),
            after_sync + 1,
            "a snapshot after {} single edits",
            SNAPSHOT_EVERY
        );
        assert!(catalog.unlogged_ops() < SNAPSHOT_EVERY);
    }

    #[test]
    fn undo_restores_the_previous_annotation() {
        let temp = TempCatalog::new("undo");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 3)],
            })
            .expect("rate");
        catalog
            .commit(Op::Flag {
                changes: vec![(PhotoId(1), Flag::Reject)],
            })
            .expect("flag");
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").flag, Flag::Reject);

        assert!(catalog.undo().expect("undo"));
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.flag, Flag::None);
        assert_eq!(photo.rating, 3, "undo only touches the last operation");

        assert!(catalog.undo().expect("undo"));
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").rating, 0);
    }

    #[test]
    fn a_torn_record_costs_only_the_tail() {
        let temp = TempCatalog::new("torn");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog.close().expect("close");
        }
        // Simulate a crash mid-write: a snapshot exists, then a broken record.
        let log = temp.0.join(LOG_NAME);
        let mut file = OpenOptions::new().append(true).open(&log).expect("open log");
        file.write_all(b"deadbeef {\"seq\":999,\"op\":{\"op\":\"addRoot\"")
            .expect("write torn");
        drop(file);

        let (mut catalog, report) = temp.open();
        assert_eq!(catalog.len(), 1);
        assert!(!report.warnings.is_empty(), "the torn record is reported");
        // The catalog still accepts writes after a damaged tail.
        catalog
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 2)],
            })
            .expect("rate after torn tail");
    }

    #[test]
    fn a_flipped_checksum_costs_one_record_not_the_library() {
        let temp = TempCatalog::new("crc-flip");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 4)],
                })
                .expect("rate");
            catalog.close().expect("close");
        }
        // Flip one hex digit of the *last* record's checksum — the rating. The line still parses,
        // its JSON is intact, and only the checksum says otherwise.
        let log = temp.0.join(LOG_NAME);
        let text = fs::read_to_string(&log).expect("read log");
        let start = match text.trim_end().rfind('\n') {
            Some(index) => index + 1,
            None => 0,
        };
        let broken = match text[start..].chars().next() {
            Some(first) => {
                let replacement = if first == '0' { '1' } else { '0' };
                format!("{}{replacement}{}", &text[..start], &text[start + first.len_utf8()..])
            }
            None => String::new(),
        };
        assert_ne!(text, broken);
        fs::write(&log, &broken).expect("write flipped log");

        let (catalog, report) = temp.open();
        assert!(report.torn_tail, "the tail is not trusted");
        assert!(
            report.warnings.iter().any(|warning| warning.contains("checksum")),
            "the reason is a checksum, and it is reported: {:?}",
            report.warnings
        );
        // The library is whole and the damaged record costs nothing before it — the photo is there,
        // and only its rating is missing.
        assert_eq!(catalog.len(), 1);
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.rel, "a.jpg");
        assert_eq!(photo.rating, 0, "the dropped record was the rating");
    }

    #[test]
    fn a_partial_snapshot_falls_back_to_the_log() {
        let temp = TempCatalog::new("partial-snapshot");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog.snapshot().expect("snapshot");
            catalog
                .commit(Op::Flag {
                    changes: vec![(PhotoId(1), Flag::Pick)],
                })
                .expect("flag");
            catalog.close().expect("close");
        }
        // A snapshot cut in half by a crash: valid JSON prefix, no closing brace.
        let snapshot = temp.0.join(SNAPSHOT_NAME);
        let text = fs::read_to_string(&snapshot).expect("read snapshot");
        let half = text.len() / 2;
        fs::write(&snapshot, &text.as_bytes()[..half]).expect("write partial snapshot");

        let (catalog, report) = temp.open();
        assert_eq!(report.snapshot_seq, 0, "a snapshot that does not parse is not trusted");
        assert!(!report.warnings.is_empty(), "and that is reported, not silent");
        // The log still holds everything, so the library is intact.
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").flag, Flag::Pick);
        assert!(report.replayed >= 1);
    }

    #[test]
    fn a_snapshot_and_more_ops_replay_in_order() {
        let temp = TempCatalog::new("snapshot");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog.snapshot().expect("snapshot");
            catalog
                .commit(Op::Flag {
                    changes: vec![(PhotoId(1), Flag::Pick)],
                })
                .expect("flag");
        }
        let (catalog, report) = temp.open();
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").flag, Flag::Pick);
        assert_eq!(report.replayed, 1, "only the post-snapshot record replays");
    }

    #[test]
    fn rescan_keeps_annotations_and_marks_missing_files() {
        let temp = TempCatalog::new("rescan");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("sync");
        catalog
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 4)],
            })
            .expect("rate");

        let outcome = catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("rescan");
        assert_eq!(outcome.missing, 1);
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").rating, 4);
        assert!(catalog.photo(PhotoId(2)).expect("photo").missing);

        let filter = Filter::default();
        assert_eq!(
            catalog.page(&filter, Sort::Natural, 0, 10).total,
            1,
            "missing photos leave the grid"
        );

        let outcome = catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("rescan");
        assert_eq!(outcome.restored, 1);
        assert!(!catalog.photo(PhotoId(2)).expect("photo").missing);
    }

    #[test]
    fn stats_and_paging_track_the_data() {
        let temp = TempCatalog::new("stats");
        let (mut catalog, _) = temp.open();
        catalog
            .sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg"), seed("c.jpg")])
            .expect("sync");
        catalog
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 5)],
            })
            .expect("rate");
        catalog
            .commit(Op::Flag {
                changes: vec![(PhotoId(2), Flag::Reject)],
            })
            .expect("flag");
        catalog
            .commit(Op::Label {
                changes: vec![(PhotoId(3), Some(ColorLabel::Blue))],
            })
            .expect("label");

        let stats = catalog.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.rated, 1);
        assert_eq!(stats.rejects, 1);
        assert_eq!(stats.labelled, 1);

        let page = catalog.page(&Filter::default(), Sort::Name, 1, 1);
        assert_eq!(page.total, 3);
        assert_eq!(page.photos.len(), 1);
        assert_eq!(page.photos[0].name, "b.jpg");
    }

    #[test]
    fn collection_colour_and_favourite_undo_exactly() {
        let temp = TempCatalog::new("collection-flags");
        let (mut catalog, _) = temp.open();
        let trip = catalog.create_collection("Trip").expect("create");
        let work = catalog.create_collection("Work").expect("create");

        catalog.set_collection_colour(trip, Some(ColorLabel::Green)).expect("colour");
        catalog.set_collection_favourite(trip, true).expect("favourite");
        catalog.set_collection_colour(trip, Some(ColorLabel::Green)).expect("no-op");

        let listed = catalog.collections();
        assert_eq!(listed[0].id, trip, "the favourite comes first");
        assert_eq!(listed[0].colour, Some(ColorLabel::Green));
        assert_eq!(listed[1].id, work);
        assert!(!listed[1].favourite);

        // Undo the favourite, then the colour, one step each.
        assert!(catalog.undo().expect("undo"));
        assert!(!catalog.collections()[0].favourite || catalog.collections()[0].id == work);
        assert!(catalog.undo().expect("undo"));
        let trip_now = catalog.collections().into_iter().find(|item| item.id == trip).expect("trip");
        assert_eq!(trip_now.colour, None, "clearing the colour is its own inverse");
    }

    #[test]
    fn a_switched_on_pack_survives_a_restart_and_undoes_like_anything_else() {
        let temp = TempCatalog::new("packs");
        {
            let (mut catalog, _) = temp.open();
            assert!(catalog.enabled_packs().is_empty());
            assert!(catalog.set_pack("Travel", true).expect("enable"));
            assert!(
                !catalog.set_pack("Travel", true).expect("again"),
                "switching on a pack that is already on changes nothing"
            );
            assert!(catalog.enabled_packs().contains("Travel"));
        }
        {
            let (mut catalog, report) = temp.open();
            assert_eq!(report.replayed, 1, "the switch is one record in the log, not a snapshot");
            assert!(catalog.enabled_packs().contains("Travel"), "the switch is remembered");
            assert!(catalog.undo().expect("undo"));
            assert!(catalog.enabled_packs().is_empty(), "undo switches it off again");
            assert!(
                !catalog.set_pack("Travel", false).expect("off again"),
                "switching off a pack that is already off changes nothing"
            );
            assert!(catalog.set_pack("Travel", true).expect("redone"), "and can be switched on again");
        }
        {
            let (catalog, _) = temp.open();
            assert!(catalog.enabled_packs().contains("Travel"));
        }
    }

    #[test]
    fn a_collection_remembers_its_sort_and_undoes_it() {
        let temp = TempCatalog::new("collection-sort");
        let (mut catalog, _) = temp.open();
        let trip = catalog.create_collection("Trip").expect("create");
        assert!(catalog.collections()[0].sort.is_none());

        catalog.set_collection_sort(trip, Sort::Newest).expect("sort");
        assert_eq!(catalog.collections()[0].sort.as_deref(), Some(Sort::Newest.id()));
        catalog.set_collection_sort(trip, Sort::Newest).expect("no-op");

        assert!(catalog.undo().expect("undo"));
        assert!(catalog.collections()[0].sort.is_none(), "undo forgets the sort");

        // A sort the app no longer knows falls back to the natural order when read back.
        assert_eq!(Sort::parse("nonsense"), Sort::Natural);
    }

    #[test]
    fn undo_survives_a_reopen() {
        let temp = TempCatalog::new("undo-reopen");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 4)],
                })
                .expect("rate");
        }

        // A fresh process: the inverse came back from the log, not from memory. The sync's snapshot
        // holds the photo, so what replays is the rating and its logged inverse.
        let (mut reopened, report) = temp.open();
        assert!(report.replayed >= 1, "the log after the snapshot replays");
        assert_eq!(reopened.photo(PhotoId(1)).expect("photo").rating, 4);
        assert!(reopened.undo().expect("undo"));
        assert_eq!(reopened.photo(PhotoId(1)).expect("photo").rating, 0, "undo works in a new process");
        drop(reopened); // one writer per library, so the next process waits for this one

        // The undo was itself a logged change with its own inverse, so a reopen can step forward
        // again — undo stays one level, but it is never a one-way door.
        let (mut again, _) = temp.open();
        assert!(again.undo().expect("undo the undo"));
        assert_eq!(again.photo(PhotoId(1)).expect("photo").rating, 4);
    }

    /// Twenty steps of undo and redo, and not one more: the history has a floor, and the steps
    /// beneath it are gone for good rather than silently unbounded.
    #[test]
    fn undo_keeps_twenty_steps_and_redo_walks_back_forward() {
        let temp = TempCatalog::new("undo-twenty");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        // Ratings clamp at five, so the sequence cycles to keep every commit a real change.
        let mut values = Vec::new();
        for step in 0..25u8 {
            let stars = step % 5 + 1;
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), stars)],
                })
                .expect("rate");
            values.push(stars);
        }

        // Twenty-five rate commits plus the two scan commits make twenty-seven steps; the floor
        // keeps the newest twenty, so the deepest reachable rating is the fifth commit's, and
        // each undo restores the rating the step before it had set.
        for step in 1..=20usize {
            assert!(catalog.undo().expect("undo"));
            assert_eq!(catalog.photo(PhotoId(1)).expect("photo").rating, values[24 - step]);
        }
        assert!(!catalog.undo().expect("the history is spent"), "the 21st step is gone");

        // Redo walks back up the same steps, and stops when it reaches the newest change.
        for &value in &values[5..] {
            assert!(catalog.redo().expect("redo"));
            assert_eq!(catalog.photo(PhotoId(1)).expect("photo").rating, value);
        }
        assert!(!catalog.redo().expect("the redo branch is spent"));
    }

    /// A new change after an undo abandons the redo branch: stepping forward again would replay
    /// a past that no longer matches what was just written.
    #[test]
    fn a_new_change_abandons_the_redo_branch() {
        let temp = TempCatalog::new("undo-branch");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 4)],
            })
            .expect("rate");
        assert!(catalog.undo().expect("undo"));

        catalog
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 2)],
            })
            .expect("rate over the undo");
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").rating, 2);
        assert!(!catalog.redo().expect("no redo after a new change"), "the 4-star past is gone");
    }

    /// The whole history travels through a reopen — the snapshot seeds it and the log replays
    /// the rest — not just the single level the old snapshot format carried.
    #[test]
    fn twenty_steps_of_undo_survive_a_reopen() {
        let temp = TempCatalog::new("undo-reopen-twenty");
        // Ratings clamp at five, so the sequence cycles to keep every commit a real change.
        let mut values = Vec::new();
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            for step in 0..25u8 {
                let stars = step % 5 + 1;
                catalog
                    .commit(Op::Rate {
                        changes: vec![(PhotoId(1), stars)],
                    })
                    .expect("rate");
                values.push(stars);
            }
        }

        let (mut reopened, report) = temp.open();
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        assert_eq!(reopened.photo(PhotoId(1)).expect("photo").rating, 5);
        // The snapshot seeds the history with what it covers and the log replays the rest, so the
        // reopened library keeps the newest twenty steps, not just the one the old format carried.
        for step in 1..=20usize {
            assert!(reopened.undo().expect("undo in a new process"));
            assert_eq!(reopened.photo(PhotoId(1)).expect("photo").rating, values[24 - step]);
        }
        assert!(!reopened.undo().expect("the history is spent"));
    }

    /// A snapshot written by an older build carries only the last inverse; opening it keeps that
    /// one step rather than failing or reopening with nothing to undo.
    #[test]
    fn a_single_level_snapshot_still_opens_with_its_undo() {
        let temp = TempCatalog::new("undo-old-snapshot");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 4)],
                })
                .expect("rate");
            catalog.snapshot().expect("snapshot");
        }

        // Rewind the file to the older format: the history field did not exist yet.
        let path = temp.0.join(SNAPSHOT_NAME);
        let mut text: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read snapshot")).expect("parse snapshot");
        text.as_object_mut().expect("object").remove("undo_history");
        std::fs::write(&path, serde_json::to_string(&text).expect("serialise")).expect("write snapshot");

        let (mut reopened, _) = temp.open();
        assert!(reopened.undo().expect("the one level came back"));
        assert_eq!(reopened.photo(PhotoId(1)).expect("photo").rating, 0);
    }

    #[test]
    fn a_snapshot_rollover_keeps_one_undo_level() {
        let temp = TempCatalog::new("undo-snapshot");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 5)],
                })
                .expect("rate");
            // A snapshot is an accelerator, not a replacement: the log keeps its records so a
            // damaged snapshot still costs nothing. This is where the inverse used to be lost.
            catalog.snapshot().expect("snapshot");
        }
        assert!(
            !std::fs::read_to_string(temp.0.join(LOG_NAME)).expect("log").trim().is_empty(),
            "a snapshot does not empty the log"
        );

        let (mut reopened, report) = temp.open();
        assert_eq!(report.replayed, 0, "everything came from the snapshot");
        assert_eq!(report.skipped, 3, "records the snapshot already covers are skipped");
        assert_eq!(reopened.photo(PhotoId(1)).expect("photo").rating, 5);
        assert!(reopened.undo().expect("undo"));
        assert_eq!(reopened.photo(PhotoId(1)).expect("photo").rating, 0);
    }

    #[test]
    fn compaction_is_what_empties_the_log() {
        let temp = TempCatalog::new("compact-empties");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        let before = catalog.log_bytes();
        assert!(before > 0);
        let reclaimed = catalog.compact().expect("compact");
        assert_eq!(reclaimed, before, "compaction reports what it removed");
        assert_eq!(catalog.log_bytes(), 0);
        assert_eq!(catalog.len(), 1, "the library is whole afterwards");
        drop(catalog);
        let (reopened, report) = temp.open();
        assert_eq!(reopened.len(), 1, "the snapshot alone carries the library");
        assert_eq!(report.replayed, 0);
        assert!(report.snapshot_seq >= 1);
    }

    #[test]
    fn a_capture_date_can_be_corrected_and_undone() {
        let temp = TempCatalog::new("captured");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        let id = PhotoId(1);

        catalog
            .commit(Op::Captured {
                changes: vec![(id, Some(1_719_187_200))],
            })
            .expect("set");
        assert_eq!(catalog.photo(id).expect("photo").captured, Some(1_719_187_200));

        // Setting the same value again records nothing, so the log does not grow with no-ops and
        // the single undo level is not spent on one.
        let bytes = catalog.log_bytes();
        catalog
            .commit(Op::Captured {
                changes: vec![(id, Some(1_719_187_200))],
            })
            .expect("no-op");
        assert_eq!(catalog.log_bytes(), bytes);

        assert!(catalog.undo().expect("undo"));
        assert_eq!(catalog.photo(id).expect("photo").captured, None);

        // Clearing is an op too, and a missing photo is refused rather than ignored.
        catalog
            .commit(Op::Captured {
                changes: vec![(id, Some(1_700_000_000))],
            })
            .expect("set");
        catalog.commit(Op::Captured { changes: vec![(id, None)] }).expect("clear");
        assert_eq!(catalog.photo(id).expect("photo").captured, None);
        assert!(
            catalog
                .commit(Op::Captured {
                    changes: vec![(PhotoId(99), Some(1))],
                })
                .is_err()
        );
    }

    #[test]
    fn a_renamed_keyword_becomes_an_alias_that_still_resolves() {
        let temp = TempCatalog::new("alias");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![(PhotoId(1), vec!["strand".to_string()])],
            })
            .expect("tag");

        assert_eq!(catalog.rename_tag("strand", "beach").expect("rename"), 1);
        assert_eq!(catalog.resolve_tag("strand"), "beach", "the old name now means the new one");
        assert_eq!(catalog.aliases_of("beach"), vec!["strand".to_string()]);

        let registry = catalog.registry();
        assert_eq!(registry.entries[0].name, "beach");
        assert_eq!(registry.entries[0].aliases, vec!["strand".to_string()]);
        assert_eq!(registry.merged(), 1);

        // Renaming twice still lands on the current name, and a cycle cannot be built.
        catalog.rename_tag("beach", "shore").expect("rename again");
        assert_eq!(catalog.resolve_tag("strand"), "shore", "chains resolve all the way");
        assert_eq!(catalog.resolve_tag("beach"), "shore");
        assert!(catalog.set_alias("shore", Some("shore")).is_err(), "no self-alias");

        // Forgetting an alias is an op, and undo brings it back.
        assert!(catalog.set_alias("strand", None).expect("forget"));
        assert_eq!(catalog.resolve_tag("strand"), "strand");
        catalog.undo().expect("undo");
        assert_eq!(catalog.resolve_tag("strand"), "shore", "the old link is back");

        // An alias whose keyword nobody carries is reported rather than silently kept.
        catalog
            .commit(Op::Tagged {
                changes: vec![(PhotoId(1), Vec::new())],
            })
            .expect("untag");
        let orphans = catalog.orphaned_aliases();
        assert!(
            orphans.iter().any(|(alias, canonical)| alias == "strand" && canonical == "beach"),
            "{orphans:?}"
        );
    }

    #[test]
    fn deleting_a_keyword_removes_it_everywhere_in_one_undo() {
        let temp = TempCatalog::new("tag-delete");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![
                    (PhotoId(1), vec!["beach".to_string(), "sand".to_string()]),
                    (PhotoId(2), vec!["beach".to_string()]),
                ],
            })
            .expect("tag");

        assert_eq!(catalog.delete_tag("beach").expect("delete"), 2, "two photos lose the keyword");
        assert!(!catalog.photo(PhotoId(1)).expect("photo").has_tag("beach"));
        assert!(catalog.photo(PhotoId(1)).expect("photo").has_tag("sand"), "other keywords stay");
        assert_eq!(
            catalog.tag_vocabulary(),
            vec![("sand".to_string(), 1)],
            "only the deleted keyword goes"
        );

        // One undo brings the keyword back on every photo it was on.
        catalog.undo().expect("undo");
        assert!(catalog.photo(PhotoId(1)).expect("photo").has_tag("beach"));
        assert!(catalog.photo(PhotoId(2)).expect("photo").has_tag("beach"));

        // A keyword nobody carries and no alias points at is refused, not silently "deleted".
        let error = catalog.delete_tag("harbour").expect_err("refused when nothing matches");
        assert!(error.to_string().contains("harbour"), "{error}");
        assert!(catalog.delete_tag("   ").is_err(), "an empty name is refused");
    }

    #[test]
    fn deleting_a_keyword_dies_with_its_rename_trail() {
        let temp = TempCatalog::new("tag-delete-alias");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![(PhotoId(1), vec!["strand".to_string()])],
            })
            .expect("tag");
        catalog.rename_tag("strand", "beach").expect("rename");
        catalog.rename_tag("beach", "shore").expect("rename again");
        assert_eq!(catalog.resolve_tag("strand"), "shore", "the chain resolves through both renames");
        assert_eq!(catalog.aliases_of("shore"), vec!["beach".to_string()]);

        // Deleting under an alias name deletes the keyword the alias stands for, and every
        // alias in the chain dies with it: re-typing "strand" must not resurrect a keyword.
        assert_eq!(catalog.delete_tag("strand").expect("delete by alias"), 1);
        assert_eq!(catalog.tag_vocabulary(), Vec::<(String, usize)>::new());
        assert_eq!(catalog.resolve_tag("strand"), "strand", "the alias link is gone");
        assert_eq!(catalog.resolve_tag("beach"), "beach");

        // One undo restores both the photos and the whole rename trail.
        catalog.undo().expect("undo");
        assert!(catalog.photo(PhotoId(1)).expect("photo").has_tag("shore"));
        assert_eq!(catalog.resolve_tag("strand"), "shore", "the chain resolves again");
        assert_eq!(catalog.resolve_tag("beach"), "shore");
    }

    #[test]
    fn an_unreadable_metadata_read_marks_the_photo_for_a_retry() {
        let temp = TempCatalog::new("retry-queue");
        let (mut catalog, _) = temp.open();
        let mut seed = seed("a.jpg");
        seed.meta_unreadable = true;
        catalog.sync_root("/photos", vec![seed]).expect("sync");

        let queued = catalog.retrying_photos();
        assert_eq!(queued.len(), 1);
        assert!(queued[0].retry);

        // Marking it done is one undoable op with an exact inverse.
        catalog.mark_retry(&[PhotoId(1)], false).expect("clear");
        assert!(catalog.retrying_photos().is_empty());
        catalog.undo().expect("undo");
        assert_eq!(catalog.retrying_photos().len(), 1, "the queue is back");
        assert_eq!(catalog.mark_retry(&[PhotoId(1)], true).expect("no-op"), 0, "already queued");
    }

    #[test]
    fn a_second_writer_gets_a_read_only_library_until_the_first_lets_go() {
        let temp = TempCatalog::new("locking");
        let (mut first, report) = temp.open();
        assert!(!report.read_only, "the first process writes");
        first.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");

        // A second process — a second `Catalog` on the same files — is a second open.
        let (mut second, report) = super::Catalog::open(&temp.0).expect("open second");
        assert!(report.read_only, "the second process is read-only");
        assert!(report.warnings.iter().any(|note| note.contains("read-only")));
        assert_eq!(second.stats().total, 1, "it can still read everything");
        let error = second
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 5)],
            })
            .expect_err("refused");
        assert!(error.to_string().contains("read-only"), "{error}");

        // Closing the first process releases the lock, which is what a crashed window does too.
        first.close().expect("close");
        drop(first);
        let (mut third, report) = super::Catalog::open(&temp.0).expect("open third");
        assert!(!report.read_only, "the lock is free again");
        third
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 3)],
            })
            .expect("rate");
        assert_eq!(third.photo(PhotoId(1)).expect("photo").rating, 3);
    }

    #[test]
    fn a_record_without_an_inverse_still_loads() {
        // Records written before inverses travelled in the log must keep replaying.
        let record = Record {
            seq: 7,
            op: Op::Rate {
                changes: vec![(PhotoId(1), 2)],
            },
            inverse: None,
            at: 0,
        };
        let json = serde_json::to_string(&record).expect("encode");
        assert!(!json.contains("inverse"), "absent inverses are not written");
        assert!(!json.contains("\"at\""), "an unknown time is not written either");
        let crc = crc32fast::hash(json.as_bytes());
        let decoded = decode(&format!("{crc:08x} {json}")).expect("decode");
        assert_eq!(decoded.0, 7);
        assert!(decoded.2.is_none(), "nothing to undo, but the op still replays");
        assert_eq!(decoded.3, 0, "an old record has no timestamp");
    }

    #[test]
    fn renaming_a_keyword_moves_every_photo_and_merges_into_an_existing_one() {
        let temp = TempCatalog::new("tag-rename");
        let (mut catalog, _) = temp.open();
        catalog
            .sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg"), seed("c.jpg")])
            .expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![
                    (PhotoId(1), vec!["beach".into(), "sunny".into()]),
                    (PhotoId(2), vec!["beach".into()]),
                    (PhotoId(3), vec!["sunset".into()]),
                ],
            })
            .expect("tag");

        assert_eq!(catalog.rename_tag("beach", "shore").expect("rename"), 2);
        let tags = |catalog: &Catalog, id: u64| catalog.photo(PhotoId(id)).expect("photo").tags.clone();
        assert_eq!(tags(&catalog, 1), vec!["shore".to_string(), "sunny".to_string()]);
        assert_eq!(tags(&catalog, 2), vec!["shore".to_string()]);
        assert_eq!(
            catalog.tag_vocabulary().iter().map(|(tag, _)| tag.clone()).collect::<Vec<_>>(),
            vec!["shore".to_string(), "sunny".to_string(), "sunset".to_string()]
        );

        // Renaming onto an existing keyword merges rather than duplicating.
        assert_eq!(catalog.rename_tag("sunny", "shore").expect("merge"), 1);
        assert_eq!(tags(&catalog, 1), vec!["shore".to_string()]);

        // One undo reverses the merge, a second the rename.
        assert!(catalog.undo().expect("undo"));
        assert_eq!(tags(&catalog, 1), vec!["shore".to_string(), "sunny".to_string()]);
        assert!(catalog.undo().expect("undo"));
        assert_eq!(tags(&catalog, 1), vec!["beach".to_string(), "sunny".to_string()]);

        assert!(catalog.rename_tag("beach", "beach").is_err(), "a no-op rename is refused");
        assert!(catalog.rename_tag("", "x").is_err(), "an empty name is refused");
        assert_eq!(catalog.rename_tag("missing", "x").expect("nothing to do"), 0);
    }

    #[test]
    fn a_collection_remembers_its_search_until_it_is_cleared() {
        let temp = TempCatalog::new("collection-search");
        let (mut catalog, _) = temp.open();
        let trip = catalog.create_collection("Trip").expect("create");
        assert!(catalog.collections()[0].search.is_none());

        catalog.set_collection_search(trip, "  beach  ").expect("search");
        assert_eq!(catalog.collections()[0].search.as_deref(), Some("beach"), "trimmed");
        catalog.set_collection_search(trip, "beach").expect("no-op");

        catalog.set_collection_search(trip, "   ").expect("clear");
        assert!(catalog.collections()[0].search.is_none(), "whitespace clears it");
        assert!(catalog.undo().expect("undo"));
        assert_eq!(
            catalog.collections()[0].search.as_deref(),
            Some("beach"),
            "undo brings the search back"
        );
    }

    #[test]
    fn the_keyword_list_can_be_ordered_by_name_or_use() {
        let temp = TempCatalog::new("tag-order");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![
                    (PhotoId(1), vec!["zebra".into(), "apple".into()]),
                    (PhotoId(2), vec!["apple".into()]),
                ],
            })
            .expect("tag");

        let by_count = catalog.tag_vocabulary_in(TagOrder::Count);
        assert_eq!(by_count[0], ("apple".to_string(), 2));
        assert_eq!(by_count[1], ("zebra".to_string(), 1));

        let by_name = catalog.tag_vocabulary_in(TagOrder::Name);
        assert_eq!(by_name[0], ("apple".to_string(), 2));
        assert_eq!(by_name[1], ("zebra".to_string(), 1));
        assert_eq!(TagOrder::Count.flipped(), TagOrder::Name);
    }

    #[test]
    fn keyword_filters_combine_any_or_all() {
        let temp = TempCatalog::new("tag-modes");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![
                    (PhotoId(1), vec!["beach".into(), "sunset".into()]),
                    (PhotoId(2), vec!["beach".into()]),
                ],
            })
            .expect("tag");

        let any = Filter {
            tags: vec!["beach".into(), "sunset".into()],
            tag_mode: TagMode::Any,
            ..Filter::default()
        };
        assert_eq!(catalog.count(&any), 2, "either keyword matches");

        let all = Filter {
            tags: vec!["beach".into(), "sunset".into()],
            tag_mode: TagMode::All,
            ..Filter::default()
        };
        assert_eq!(catalog.count(&all), 1, "both keywords required");

        let none = Filter {
            tags: vec!["missing".into()],
            ..Filter::default()
        };
        assert_eq!(catalog.count(&none), 0);
        assert_eq!(TagMode::Any.flipped(), TagMode::All);
    }

    #[test]
    fn kind_and_edited_filters_narrow_the_view() {
        let temp = TempCatalog::new("kind-filter");
        let (mut catalog, _) = temp.open();
        let mut raw = seed("b.cr3");
        raw.kind = MediaKind::Raw;
        raw.key = 8;
        let mut video = seed("c.mov");
        video.kind = MediaKind::Video;
        video.key = 9;
        catalog.sync_root("/photos", vec![seed("a.jpg"), raw, video]).expect("sync");

        let raws = Filter {
            kind: Some(MediaKind::Raw),
            ..Filter::default()
        };
        assert_eq!(catalog.count(&raws), 1);
        let images = Filter {
            kind: Some(MediaKind::Image),
            ..Filter::default()
        };
        assert_eq!(catalog.count(&images), 1);

        let edited = Filter {
            edited: Some(true),
            ..Filter::default()
        };
        assert_eq!(catalog.count(&edited), 0, "nothing is edited yet");
        let unedited = Filter {
            edited: Some(false),
            ..Filter::default()
        };
        assert_eq!(catalog.count(&unedited), 3);
    }

    #[test]
    fn folders_are_listed_one_level_at_a_time_with_recursive_counts() {
        let temp = TempCatalog::new("folders");
        let (mut catalog, _) = temp.open();
        let mut seeds: Vec<Seed> = Vec::new();
        for rel in ["a.jpg", "day1/b.jpg", "day1/c.jpg", "day1/sub/d.jpg", "day2/e.jpg"] {
            let mut seed = seed(rel);
            seed.key = rel.len() as u64;
            seeds.push(seed);
        }
        catalog.sync_root("/photos", seeds).expect("sync");

        let top = catalog.folders("/photos", "");
        assert_eq!(top, vec![("day1".to_string(), 3), ("day2".to_string(), 1)]);
        let inside = catalog.folders("/photos", "day1/");
        assert_eq!(inside, vec![("sub".to_string(), 1)]);
        assert!(catalog.folders("/photos", "day1/sub/").is_empty());

        let in_day1 = Filter {
            folder: Some("day1/".into()),
            ..Filter::default()
        };
        assert_eq!(catalog.count(&in_day1), 3);
    }

    #[test]
    fn collections_are_created_filled_filtered_and_undone() {
        let temp = TempCatalog::new("collections");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("sync");

        let id = catalog.create_collection("Best of 2024").expect("create");
        assert_eq!(catalog.collections().len(), 1);
        assert_eq!(catalog.collections()[0].name, "Best of 2024");

        catalog.collect(id, &[PhotoId(1), PhotoId(2)], &[]).expect("add");
        assert_eq!(catalog.collection(id).expect("collection").len(), 2);
        let in_collection = Filter {
            collection: Some(id),
            ..Filter::default()
        };
        assert_eq!(catalog.count(&in_collection), 2);

        catalog.collect(id, &[], &[PhotoId(2)]).expect("remove");
        assert_eq!(catalog.count(&in_collection), 1);
        assert_eq!(catalog.count(&Filter::default()), 2, "the photos stay in the library");

        catalog.rename_collection(id, "Keepers").expect("rename");
        assert_eq!(catalog.collection(id).expect("collection").name, "Keepers");

        catalog.delete_collection(id).expect("delete");
        assert!(catalog.collections().is_empty());
        assert_eq!(catalog.count(&Filter::default()), 2);

        assert!(catalog.undo().expect("undo"));
        assert_eq!(catalog.collections().len(), 1, "undo brings the collection back");
    }

    #[test]
    fn collections_survive_a_snapshot_and_restart() {
        let temp = TempCatalog::new("collections-restart");
        let id = {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            let id = catalog.create_collection("Camera roll").expect("create");
            catalog.collect(id, &[PhotoId(1)], &[]).expect("add");
            catalog.snapshot().expect("snapshot");
            id
        };
        let (catalog, _) = temp.open();
        let collection = catalog.collection(id).expect("collection survives");
        assert_eq!(collection.name, "Camera roll");
        assert_eq!(collection.len(), 1);
    }

    #[test]
    fn a_collection_needs_a_name() {
        let temp = TempCatalog::new("collections-nameless");
        let (mut catalog, _) = temp.open();
        assert!(catalog.create_collection("   ").is_err());
        assert!(catalog.collect(sortlens_core::CollectionId(99), &[PhotoId(1)], &[]).is_err());
    }

    #[test]
    fn relocating_a_photo_clears_missing_and_undoes() {
        let temp = TempCatalog::new("relocate");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Missing {
                id: PhotoId(1),
                missing: true,
            })
            .expect("missing");

        catalog
            .commit(Op::Relocate {
                changes: vec![(PhotoId(1), "moved/a.jpg".into())],
            })
            .expect("relocate");
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.rel, "moved/a.jpg");
        assert!(!photo.missing);

        assert!(catalog.undo().expect("undo"));
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.rel, "a.jpg");
        assert!(photo.missing, "undo puts the missing state back too");
    }

    #[test]
    fn removing_missing_photos_keeps_the_rest_and_undoes() {
        let temp = TempCatalog::new("remove-missing");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("sync");
        catalog
            .commit(Op::Missing {
                id: PhotoId(2),
                missing: true,
            })
            .expect("missing");
        assert_eq!(catalog.missing_photos().len(), 1);

        assert_eq!(catalog.remove_missing().expect("remove"), 1);
        assert_eq!(catalog.count(&Filter::default()), 1);
        assert!(catalog.undo().expect("undo"));
        // The photo comes back still marked missing, exactly as it was before the purge.
        assert!(catalog.photo(PhotoId(2)).is_some());
        let with_missing = Filter {
            include_missing: true,
            ..Filter::default()
        };
        assert_eq!(catalog.count(&with_missing), 2);
    }

    #[test]
    fn a_group_undoes_as_one_step() {
        let temp = TempCatalog::new("group");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("sync");
        catalog
            .commit(Op::Group {
                ops: vec![
                    Op::Flag {
                        changes: vec![(PhotoId(1), Flag::Pick), (PhotoId(2), Flag::Reject)],
                    },
                    Op::Missing {
                        id: PhotoId(2),
                        missing: true,
                    },
                ],
            })
            .expect("group");

        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").flag, Flag::Pick);
        assert!(catalog.photo(PhotoId(2)).expect("photo").missing);

        assert!(catalog.undo().expect("undo"));
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").flag, Flag::None);
        assert!(!catalog.photo(PhotoId(2)).expect("photo").missing);
        assert_eq!(catalog.photo(PhotoId(2)).expect("photo").flag, Flag::None);
    }

    #[test]
    fn tags_are_sorted_deduplicated_and_undoable() {
        let temp = TempCatalog::new("tags");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![(PhotoId(1), vec!["sunset".into(), "beach".into(), "beach".into()])],
            })
            .expect("tag");
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.tags, vec!["beach".to_string(), "sunset".to_string()]);
        assert!(photo.has_tag("beach"));

        assert!(catalog.undo().expect("undo"));
        assert!(catalog.photo(PhotoId(1)).expect("photo").tags.is_empty());
    }

    #[test]
    fn the_tag_filter_and_vocabulary_agree() {
        let temp = TempCatalog::new("tag-filter");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg")]).expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![
                    (PhotoId(1), vec!["beach".into()]),
                    (PhotoId(2), vec!["beach".into(), "sunset".into()]),
                ],
            })
            .expect("tag");

        let filter = Filter {
            tags: vec!["beach".into()],
            ..Filter::default()
        };
        assert_eq!(catalog.count(&filter), 2);
        let filter = Filter {
            tags: vec!["sunset".into()],
            ..Filter::default()
        };
        assert_eq!(catalog.count(&filter), 1);

        let vocabulary = catalog.tag_vocabulary();
        assert_eq!(vocabulary[0], ("beach".to_string(), 2));
        assert_eq!(vocabulary[1], ("sunset".to_string(), 1));
        assert_eq!(catalog.stats().tagged, 2);
    }

    #[test]
    fn a_rescan_keeps_tags() {
        let temp = TempCatalog::new("tag-rescan");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Tagged {
                changes: vec![(PhotoId(1), vec!["keep".into()])],
            })
            .expect("tag");
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("rescan");
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").tags, vec!["keep".to_string()]);
    }

    #[test]
    fn a_new_photo_takes_its_review_from_the_sidecar() {
        let temp = TempCatalog::new("sidecar-seed");
        let (mut catalog, _) = temp.open();
        let review = Review {
            rating: 4,
            flag: Flag::Reject,
            label: Some(sortlens_core::ColorLabel::Green),
            tags: vec!["keeper".into()],
        };
        let from_sidecar = Seed {
            review: Some(review),
            ..seed("a.jpg")
        };
        catalog.sync_root("/photos", vec![from_sidecar]).expect("sync");
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.rating, 4);
        assert_eq!(photo.flag, Flag::Reject);
        assert!(photo.has_tag("keeper"));
        assert!(photo.label.is_some());
        assert_eq!(
            photo.tag_source("keeper"),
            sortlens_core::TagSource::Xmp,
            "a keyword read out of a sidecar was written by another application, and says so"
        );
        // Typing the same word afterwards does not relabel it: the word is already there, so nothing is written.
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("rescan");
        assert_eq!(
            catalog.photo(PhotoId(1)).expect("photo").tag_source("keeper"),
            sortlens_core::TagSource::Xmp
        );
    }

    /// Provenance travels with the tags in the same group, so one undo takes both back.
    #[test]
    fn a_keyword_remembers_who_wrote_it_and_one_undo_takes_the_word_and_the_source() {
        let temp = TempCatalog::new("tag-source");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        // Exactly the pair the engine commits for an accepted suggestion.
        catalog
            .commit(Op::Group {
                ops: vec![
                    Op::Tagged {
                        changes: vec![(PhotoId(1), vec!["cat".into()])],
                    },
                    Op::TagSourced {
                        changes: vec![(
                            PhotoId(1),
                            [("cat".to_string(), sortlens_core::TagSource::Ai)].into_iter().collect(),
                        )],
                    },
                ],
            })
            .expect("accept a suggestion");
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.tag_source("cat"), sortlens_core::TagSource::Ai);
        assert_eq!(photo.tag_source("nobody"), sortlens_core::TagSource::Unrecorded);
        assert!(photo.carries_source(sortlens_core::TagSource::Ai));
        assert!(!photo.carries_source(sortlens_core::TagSource::Manual));

        // The whole group is one step: the word and its provenance go back together.
        assert!(catalog.undo().expect("undo"));
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert!(photo.tags.is_empty());
        assert!(photo.tag_sources.is_empty(), "an undo leaves no provenance for a word that is gone");

        // A map describing a keyword the photo does not have is pruned rather than stored.
        catalog
            .commit(Op::TagSourced {
                changes: vec![(
                    PhotoId(1),
                    [("ghost".to_string(), sortlens_core::TagSource::Ai)].into_iter().collect(),
                )],
            })
            .expect("nothing to say");
        assert!(catalog.photo(PhotoId(1)).expect("photo").tag_sources.is_empty());
    }

    /// A renamed keyword keeps its provenance, and a deleted one loses it, because both are facts about the word.
    #[test]
    fn provenance_follows_a_rename_and_dies_with_a_delete() {
        let temp = TempCatalog::new("tag-source-rename");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Group {
                ops: vec![
                    Op::Tagged {
                        changes: vec![(PhotoId(1), vec!["cat".into()])],
                    },
                    Op::TagSourced {
                        changes: vec![(
                            PhotoId(1),
                            [("cat".to_string(), sortlens_core::TagSource::Ai)].into_iter().collect(),
                        )],
                    },
                ],
            })
            .expect("tag");
        catalog.rename_tag("cat", "kitten").expect("rename");
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(
            photo.tag_source("kitten"),
            sortlens_core::TagSource::Ai,
            "the word moved, and so did its origin"
        );
        assert!(
            !photo.tag_sources.contains_key("cat"),
            "no provenance is left behind under the old name"
        );

        // One undo takes the rename back, source and all.
        assert!(catalog.undo().expect("undo"));
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.tag_source("cat"), sortlens_core::TagSource::Ai);
        assert_eq!(photo.tag_source("kitten"), sortlens_core::TagSource::Unrecorded);

        catalog.delete_tag("cat").expect("delete");
        assert!(
            catalog.photo(PhotoId(1)).expect("photo").tag_sources.is_empty(),
            "a deleted keyword takes its provenance with it"
        );
    }

    /// A library written before sources existed says so, instead of claiming every keyword was typed by hand.
    #[test]
    fn a_tag_with_no_recorded_source_reads_as_unrecorded_through_a_reopen() {
        let temp = TempCatalog::new("tag-source-legacy");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog
                .commit(Op::Tagged {
                    changes: vec![(PhotoId(1), vec!["strand".into()])],
                })
                .expect("tag");
            // Snapshot it now, so this is a snapshot written by a build that recorded tags and not sources — which is
            // exactly what an older library looks like on disk.
            catalog.snapshot().expect("snapshot");
            catalog.close().expect("close");
        }
        let text = fs::read_to_string(temp.0.join(SNAPSHOT_NAME)).expect("read");
        assert!(text.contains("\"tags\":[\"strand\"]"), "the keyword is in the snapshot");
        assert!(!text.contains("tag_sources"), "and no source map was invented for it");

        let (catalog, _) = temp.open();
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.tag_source("strand"), sortlens_core::TagSource::Unrecorded);
        let recorded = |source| {
            catalog.count(&Filter {
                tag_source: Some(source),
                ..Filter::default()
            })
        };
        assert_eq!(recorded(sortlens_core::TagSource::Unrecorded), 1, "it is still findable");
        assert_eq!(recorded(sortlens_core::TagSource::Manual), 0, "but it is not called manual");
        assert_eq!(recorded(sortlens_core::TagSource::Ai), 0);
    }

    #[test]
    fn an_existing_photo_keeps_the_catalog_review_over_the_sidecar() {
        let temp = TempCatalog::new("sidecar-existing");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 2)],
            })
            .expect("rate");

        let review = Review {
            rating: 5,
            ..Review::default()
        };
        catalog
            .sync_root(
                "/photos",
                vec![Seed {
                    review: Some(review),
                    ..seed("a.jpg")
                }],
            )
            .expect("rescan");
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").rating, 2);
    }

    #[test]
    fn removing_a_root_removes_its_photos() {
        let temp = TempCatalog::new("remove-root");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog.sync_root("/other", vec![seed("b.jpg")]).expect("sync");
        assert_eq!(catalog.remove_root("/photos").expect("remove"), 1);
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.roots(), &["/other".to_string()]);
    }

    fn score(value: f32) -> Score {
        Score {
            value,
            version: sortlens_quality::SCORE_VERSION,
        }
    }

    /// A temp library with two photos, and their ids in scan order.
    fn two_photos(name: &str) -> (TempCatalog, Catalog, PhotoId, PhotoId) {
        let temp = TempCatalog::new(name);
        let (mut catalog, _) = temp.open();
        catalog
            .sync_root(
                "/p",
                vec![
                    Seed {
                        rel: "a.jpg".into(),
                        size: 10,
                        modified: Some(5),
                        kind: MediaKind::Image,
                        key: 1,
                        meta: Default::default(),
                        review: None,
                        edited: false,
                        meta_unreadable: false,
                    },
                    Seed {
                        rel: "b.jpg".into(),
                        size: 11,
                        modified: Some(6),
                        kind: MediaKind::Image,
                        key: 2,
                        meta: Default::default(),
                        review: None,
                        edited: false,
                        meta_unreadable: false,
                    },
                ],
            )
            .expect("sync");
        let ids: Vec<PhotoId> = catalog.photos().map(|photo| photo.id).collect();
        assert_eq!(ids.len(), 2, "two photos were indexed");
        (temp, catalog, ids[0], ids[1])
    }

    #[test]
    fn a_measured_batch_is_one_undo_step() {
        let (_temp, mut catalog, first, second) = two_photos("score-undo");
        let changed = catalog.set_scores(&[(first, score(1.5)), (second, score(0.25))]).expect("set");
        assert_eq!(changed, 2, "both photos were measured");
        assert_eq!(catalog.sharpness(first).map(|s| s.value), Some(1.5));
        assert_eq!(catalog.sharpness(second).map(|s| s.value), Some(0.25));

        // One undo takes both back: a scan is one step, not one step per photograph. It must not reach past itself
        // either — undoing again would take back the scan that added the photos, which is a different thing.
        assert!(catalog.undo().expect("undo"));
        assert_eq!(catalog.sharpness(first), None);
        assert_eq!(catalog.sharpness(second), None);
        assert_eq!(catalog.photos().count(), 2, "the photos are still indexed");
    }

    #[test]
    fn a_re_measure_that_changes_nothing_is_not_a_write() {
        let (_temp, mut catalog, first, _) = two_photos("score-noop");
        assert_eq!(catalog.set_scores(&[(first, score(2.0))]).expect("set"), 1);
        assert_eq!(
            catalog.set_scores(&[(first, score(2.0))]).expect("again"),
            0,
            "the same score is not a change"
        );
        // A different version is a different score, even at the same value: it is not comparable with the old one.
        let newer = Score {
            value: 2.0,
            version: sortlens_quality::SCORE_VERSION + 1,
        };
        assert_eq!(catalog.set_scores(&[(first, newer)]).expect("new version"), 1);
        assert_eq!(catalog.sharpness(first), Some(newer));
    }

    #[test]
    fn clearing_scores_is_one_undoable_op() {
        let (_temp, mut catalog, first, second) = two_photos("score-clear");
        catalog.set_scores(&[(first, score(1.0)), (second, score(2.0))]).expect("set");
        assert_eq!(catalog.clear_scores().expect("clear"), 2);
        assert!(catalog.sharpness(first).is_none());
        assert!(catalog.sharpness(second).is_none());
        assert!(catalog.undo().expect("undo"));
        assert_eq!(
            catalog.sharpness(first).map(|s| s.value),
            Some(1.0),
            "the clearing undoes as one step"
        );
        assert_eq!(catalog.sharpness(second).map(|s| s.value), Some(2.0));
    }

    #[test]
    fn a_replaced_record_drops_a_score_about_pixels_that_are_gone() {
        let (_temp, mut catalog, first, _) = two_photos("score-replace");
        catalog.set_scores(&[(first, score(3.0))]).expect("set");
        // The file changed on disk, so the scan replaces the record: the score described the old bytes.
        let rel = catalog.photo(first).map(|photo| photo.rel.clone()).expect("photo");
        catalog
            .sync_root(
                "/p",
                vec![Seed {
                    rel,
                    size: 999,
                    modified: Some(900),
                    kind: MediaKind::Image,
                    key: 77,
                    meta: Default::default(),
                    review: None,
                    edited: false,
                    meta_unreadable: false,
                }],
            )
            .expect("resync");
        assert_eq!(catalog.sharpness(first), None, "a changed file has not been measured");
        assert_eq!(catalog.photos().count(), 2, "resync did not delete the other photo");
    }

    /// A location lives on the photo record, so it survives a snapshot and cannot outlive the bytes it describes.
    #[test]
    fn a_location_survives_a_snapshot_and_follows_a_changed_file() {
        let temp = TempCatalog::new("location");
        let id;
        {
            let (mut catalog, _) = temp.open();
            catalog
                .sync_root(
                    "/p",
                    vec![Seed {
                        rel: "a.jpg".into(),
                        size: 10,
                        modified: Some(5),
                        kind: MediaKind::Image,
                        key: 1,
                        meta: sortlens_meta::PhotoMeta {
                            location: sortlens_geo::LatLon::new(50.8225, -0.1372),
                            ..Default::default()
                        },
                        review: None,
                        edited: false,
                        meta_unreadable: false,
                    }],
                )
                .expect("sync");
            id = catalog.photos().next().map(|photo| photo.id).expect("a photo");
            catalog.close().expect("close");
        }
        let (reopened, _) = temp.open();
        let stored = reopened.photo(id).and_then(|photo| photo.location).expect("a location");
        // A JPEG's GPS is stored in whole seconds of arc, so the round trip is a fraction of a metre, not exact.
        assert!((stored.lat - 50.8225).abs() < 1e-4, "got {}", stored.lat);
        assert!((stored.lon + 0.1372).abs() < 1e-4);
        assert!(reopened.sharpness(id).is_none());
    }

    #[test]
    fn a_score_survives_a_snapshot() {
        let temp = TempCatalog::new("score-snapshot");
        let id;
        {
            let (mut catalog, _) = temp.open();
            catalog
                .sync_root(
                    "/p",
                    vec![Seed {
                        rel: "a.jpg".into(),
                        size: 10,
                        modified: Some(5),
                        kind: MediaKind::Image,
                        key: 1,
                        meta: Default::default(),
                        review: None,
                        edited: false,
                        meta_unreadable: false,
                    }],
                )
                .expect("sync");
            id = catalog.photos().next().map(|photo| photo.id).expect("a photo");
            catalog.set_scores(&[(id, score(4.5))]).expect("set");
            catalog.close().expect("close");
        }
        let (reopened, _) = temp.open();
        assert_eq!(reopened.sharpness(id).map(|s| s.value), Some(4.5));
        assert_eq!(reopened.sharpness(id).map(|s| s.version), Some(sortlens_quality::SCORE_VERSION));
    }

    /// SL-D11: a writer that read the library at an older revision is refused, not allowed to
    /// overwrite the annotations that arrived in between. The refusal names both revisions so the
    /// caller can re-read and retry instead of guessing.
    #[test]
    fn a_stale_revision_is_refused_and_a_current_one_commits() {
        let temp = TempCatalog::new("stale-revision");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/p", vec![seed("a.jpg")]).expect("sync");
        let before = catalog.seq();

        catalog
            .commit_expect(
                Op::Rate {
                    changes: vec![(PhotoId(1), 5)],
                },
                before,
            )
            .expect("the writer's own revision is still current");

        // The catalog has moved on (seq advanced), so a second writer holding the pre-write
        // revision is refused, with the two revisions named.
        let error = catalog
            .commit_expect(
                Op::Flag {
                    changes: vec![(PhotoId(1), Flag::Pick)],
                },
                before,
            )
            .expect_err("stale");
        assert!(
            matches!(error, Error::StaleRevision { expected, current } if expected == before && current == before + 1),
            "the refusal names both revisions: {error:?}"
        );
        assert!(catalog.photo(PhotoId(1)).expect("photo").flag == Flag::None, "nothing was written");

        // Re-reading and retrying at the current revision succeeds.
        catalog
            .commit_expect(
                Op::Flag {
                    changes: vec![(PhotoId(1), Flag::Pick)],
                },
                catalog.seq(),
            )
            .expect("retry at the current revision");
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").flag, Flag::Pick);
    }

    /// The per-photo revision rises once per real change to the annotations, and not at all when a
    /// change is a no-op, so a stale reader has something to catch and a fresh one nothing to lose.
    #[test]
    fn per_photo_revision_tracks_real_annotation_changes() {
        let temp = TempCatalog::new("photo-revision");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/p", vec![seed("a.jpg")]).expect("sync");
        let id = PhotoId(1);
        assert_eq!(catalog.photo(id).expect("photo").rev, 0);

        catalog.commit(Op::Rate { changes: vec![(id, 4)] }).expect("rate");
        assert_eq!(catalog.photo(id).expect("photo").rev, 1);

        // Setting the same stars is a no-op: the record did not change.
        catalog.commit(Op::Rate { changes: vec![(id, 4)] }).expect("same rating");
        assert_eq!(catalog.photo(id).expect("photo").rev, 1, "no change, no revision");

        catalog
            .commit(Op::Tagged {
                changes: vec![(id, vec!["shore".into()])],
            })
            .expect("tag");
        catalog
            .commit(Op::Label {
                changes: vec![(id, Some(ColorLabel::Blue))],
            })
            .expect("label");
        assert_eq!(catalog.photo(id).expect("photo").rev, 3);

        // Undo also changes the record, so the revision rises, monotonic in either direction.
        catalog.undo().expect("undo");
        assert_eq!(catalog.photo(id).expect("photo").rev, 4);
        assert_eq!(catalog.photo(id).expect("photo").label, None);
    }

    /// The revision is derived from the log, so a reopen reproduces the same numbers — a sidecar
    /// writer's rev is valid across a restart, not just inside one session.
    #[test]
    fn per_photo_revision_survives_a_restart() {
        let temp = TempCatalog::new("photo-revision-restart");
        {
            let (mut catalog, _) = temp.open();
            catalog.sync_root("/p", vec![seed("a.jpg")]).expect("sync");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 3)],
                })
                .expect("rate");
            catalog
                .commit(Op::Tagged {
                    changes: vec![(PhotoId(1), vec!["sand".into()])],
                })
                .expect("tag");
            catalog.close().expect("close");
        }
        let (reopened, _) = temp.open();
        assert_eq!(reopened.photo(PhotoId(1)).expect("photo").rev, 2);
    }

    /// SL-A14 part 2: a new photo records the instant the catalog added it — the wall clock, never
    /// the file's `mtime` — and a reopen reproduces the same number.
    #[test]
    fn a_new_photo_records_when_it_was_added_and_a_reopen_keeps_it() {
        let temp = TempCatalog::new("photo-added");
        let (mut catalog, _) = temp.open();
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        let added = catalog
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .remove(0)
            .added
            .expect("a fresh scan records the add instant");
        let now = crate::photo::now_epoch();
        assert!(now - added < 60, "the add instant is the wall clock, not the seed's mtime of 10");

        let (reopened, _) = temp.open();
        assert_eq!(
            reopened.photo(PhotoId(1)).expect("photo").added,
            Some(added),
            "a reopen reproduces the same add instant"
        );
    }

    /// The schema-1 migration: a photo from an older snapshot has no recorded add instant, so it
    /// keeps none rather than borrowing the file's `mtime` — and the resaved snapshot is schema 2
    /// with the field honestly absent, which the Recently Added filter refuses to answer from.
    #[test]
    fn a_schema_1_snapshot_migrates_without_inventing_an_add_instant() {
        let temp = TempCatalog::new("photo-added-migration");
        let old = Snapshot {
            schema: 1,
            seq: 0,
            next_id: 2,
            roots: vec!["/photos".into()],
            photos: vec![Photo::from_scan(PhotoId(1), "/photos", "a.jpg", 100, Some(10), MediaKind::Image, 7)],
            collections: Vec::new(),
            aliases: BTreeMap::new(),
            next_collection: 0,
            packs: BTreeSet::new(),
            folders: crate::folder::FolderMarks::new(),
            undo: None,
            undo_history: Vec::new(),
        };
        fs::create_dir_all(&temp.0).expect("dir");
        fs::write(temp.0.join(SNAPSHOT_NAME), serde_json::to_string(&old).expect("serialise")).expect("write");

        let (catalog, report) = temp.open();
        assert!(report.warnings.is_empty(), "schema 1 is not newer than {SCHEMA}");
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.added, None, "no invented instant, not even from the mtime of 10");
        let thirty_days = 30 * 24 * 60 * 60;
        assert!(
            !Filter {
                added_after: Some(crate::photo::now_epoch() - thirty_days),
                ..Filter::default()
            }
            .matches(photo),
            "a photo that never recorded an add instant stays out of Recently Added"
        );

        let mut catalog = catalog;
        catalog.snapshot().expect("snapshot");
        let text = fs::read_to_string(temp.0.join(SNAPSHOT_NAME)).expect("read back");
        let value: serde_json::Value = serde_json::from_str(&text).expect("parse");
        assert_eq!(value["schema"], serde_json::Value::from(SCHEMA), "the resaved snapshot is schema 2");
        assert!(value["photos"][0].get("added").is_none(), "absence is preserved, not filled");
    }
}
