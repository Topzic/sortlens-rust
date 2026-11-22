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
use std::collections::{BTreeMap, HashSet};
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
/// Snapshot schema, bumped when the record layout changes.
pub const SCHEMA: u32 = 1;

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
    /// The inverse of the last applied operation, so undo survives a snapshot rollover. Absent in
    /// older snapshots, which simply reopen with nothing to undo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    undo: Option<Op>,
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
    next_collection: u32,
    next_id: u64,
    seq: u64,
    undo: Vec<Op>,
    log: File,
    unlogged: usize,
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
        let mut seq = 0u64;
        // The snapshot carries the last inverse; log records that replay after it take precedence.
        let mut restore_undo: Option<Op> = None;

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
                        report.snapshot_seq = seq;
                        restore_undo = snapshot.undo;
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
                next_collection,
                next_id,
                seq,
                undo: restore_undo.clone().into_iter().collect(),
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
                // Keep one level of undo across the restart: the inverse as it was written, which is
                // authoritative even when the op no longer means the same thing to this version.
                if let Some(inverse) = inverse {
                    catalog.undo.clear();
                    catalog.undo.push(inverse);
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
                next_collection,
                next_id,
                seq,
                undo: restore_undo.clone().into_iter().collect(),
                log,
                unlogged: 0,
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
        }
        // The old name becomes an alias of the new one, so re-typing it lands on the right keyword
        // instead of quietly re-creating the keyword that was just merged away. Both halves go into
        // one op, so a mistaken rename still costs a single undo.
        let mut ops = Vec::new();
        let count = changes.len();
        if !changes.is_empty() {
            ops.push(Op::Tagged { changes });
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
    fn apply_captured(&mut self, changes: &[(PhotoId, Option<i64>)]) -> Result<Op> {
        let mut previous = Vec::new();
        for (id, captured) in changes {
            let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
            if photo.captured != *captured {
                previous.push((*id, photo.captured));
                photo.captured = *captured;
            }
        }
        Ok(Op::Captured { changes: previous })
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
            Op::Tagged { changes } => {
                let mut previous = Vec::new();
                for (id, tags) in changes {
                    let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
                    let mut tags = tags.clone();
                    tags.sort_unstable();
                    tags.dedup();
                    if photo.tags != tags {
                        previous.push((*id, photo.tags.clone()));
                        photo.tags = tags;
                    }
                }
                Ok(Op::Tagged { changes: previous })
            }
            Op::Label { changes } => {
                let mut previous = Vec::new();
                for (id, label) in changes {
                    let photo = self.photos.get_mut(id).ok_or(Error::NoSuchPhoto(*id))?;
                    if photo.label != *label {
                        previous.push((*id, photo.label));
                        photo.label = *label;
                    }
                }
                Ok(Op::Label { changes: previous })
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
        if inverse.is_noop() {
            // Nothing changed, so there is nothing to record and nothing to spend undo on.
            return Ok(());
        }
        self.seq += 1;
        self.append(self.seq, &op, Some(&inverse))?;
        self.undo.push(inverse);
        self.unlogged += 1;
        if self.unlogged >= SNAPSHOT_EVERY {
            self.snapshot()?;
        }
        Ok(())
    }

    /// Undo the last committed operation. One level per call.
    pub fn undo(&mut self) -> Result<bool> {
        let Some(inverse) = self.undo.pop() else {
            return Ok(false);
        };
        let redo = self.apply(&inverse)?;
        self.seq += 1;
        self.append(self.seq, &inverse, Some(&redo))?;
        self.unlogged += 1;
        Ok(true)
    }

    /// Drop the in-memory undo history, e.g. when the document closes.
    pub fn forget_undo(&mut self) {
        self.undo.clear();
    }

    /// Bring one root up to date with a completed scan.
    ///
    /// Adds new files, refreshes changed ones in place (keeping their id and annotations), marks
    /// photos that disappeared as missing, and clears the flag on files that came back.
    pub fn sync_root(&mut self, root: &str, seeds: Vec<Seed>) -> Result<SyncOutcome> {
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
            next_collection: self.next_collection,
            undo: self.undo.last().cloned(),
        };
        let text = serde_json::to_string(&snapshot).map_err(|error| Error::Invalid(error.to_string()))?;
        let tmp = self.dir.join("snapshot.json.tmp");
        let path = self.dir.join(SNAPSHOT_NAME);
        fs::write(&tmp, text.as_bytes()).map_err(|error| Error::io(tmp.display().to_string(), error))?;
        fs::rename(&tmp, &path).map_err(|error| Error::io(path.display().to_string(), error))?;
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

    struct TempCatalog(PathBuf);

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

        // A fresh process: the inverse came back from the log, not from memory.
        let (mut reopened, report) = temp.open();
        assert!(report.replayed >= 2);
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
        let seed = Seed {
            review: Some(review),
            ..seed("a.jpg")
        };
        catalog.sync_root("/photos", vec![seed]).expect("sync");
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.rating, 4);
        assert_eq!(photo.flag, Flag::Reject);
        assert!(photo.has_tag("keeper"));
        assert!(photo.label.is_some());
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
}
