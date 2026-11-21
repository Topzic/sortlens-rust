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
use crate::query::{Filter, Page, Sort};
use serde::{Deserialize, Serialize};
use sortlens_core::{CollectionId, Error, Flag, MediaKind, PhotoId, Result, Review};
use sortlens_meta::PhotoMeta;
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Log file name.
pub const LOG_NAME: &str = "ops.log";
/// Snapshot file name.
pub const SNAPSHOT_NAME: &str = "snapshot.json";
/// Records between automatic snapshots.
pub const SNAPSHOT_EVERY: usize = 256;
/// Snapshot schema, bumped when the record layout changes.
pub const SCHEMA: u32 = 1;

/// One line of the log.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    seq: u64,
    op: Op,
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
    /// Next collection id, defaulted for older snapshots.
    #[serde(default)]
    next_collection: u32,
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
}

/// The catalog.
#[derive(Debug)]
pub struct Catalog {
    dir: PathBuf,
    photos: BTreeMap<PhotoId, Photo>,
    roots: Vec<String>,
    collections: BTreeMap<CollectionId, Collection>,
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

        let snapshot_path = dir.join(SNAPSHOT_NAME);
        let mut photos = BTreeMap::new();
        let mut roots = Vec::new();
        let mut next_id = 1u64;
        let mut next_collection = 1u32;
        let mut collections: BTreeMap<CollectionId, Collection> = BTreeMap::new();
        let mut seq = 0u64;

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
                        report.snapshot_seq = seq;
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
            let mut catalog = Self {
                dir: dir.clone(),
                photos,
                roots,
                collections,
                next_collection,
                next_id,
                seq,
                undo: Vec::new(),
                log: OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&log_path)
                    .map_err(|error| Error::io(log_path.display().to_string(), error))?,
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
                let (record_seq, op) = match decode(&line) {
                    Some(record) => record,
                    None => {
                        report.warnings.push("log record failed its checksum; earlier records kept".into());
                        break;
                    }
                };
                if record_seq <= catalog.seq {
                    continue;
                }
                let _ = catalog.apply(&op)?;
                catalog.seq = record_seq;
                report.replayed += 1;
            }
            return Ok((catalog, report));
        }

        let log = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&log_path)
            .map_err(|error| Error::io(log_path.display().to_string(), error))?;
        Ok((
            Self {
                dir,
                photos,
                roots,
                collections,
                next_collection,
                next_id,
                seq,
                undo: Vec::new(),
                log,
                unlogged: 0,
                stats: None,
            },
            report,
        ))
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
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for photo in self.photos.values() {
            for tag in &photo.tags {
                *counts.entry(tag.as_str()).or_insert(0) += 1;
            }
        }
        let mut vocabulary: Vec<(String, usize)> = counts.into_iter().map(|(tag, count)| (tag.to_string(), count)).collect();
        vocabulary.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
        vocabulary
    }

    /// Every collection, by name.
    pub fn collections(&self) -> Vec<Collection> {
        let mut collections: Vec<Collection> = self.collections.values().cloned().collect();
        collections.sort_by_key(|collection| collection.name.to_lowercase());
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
    pub fn commit(&mut self, op: Op) -> Result<()> {
        let inverse = self.apply(&op)?;
        self.seq += 1;
        self.append(self.seq, &op)?;
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
        let _ = self.apply(&inverse)?;
        self.seq += 1;
        self.append(self.seq, &inverse)?;
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
                    if let Some(review) = &seed.review {
                        photo.adopt_review(review);
                    }
                    self.commit(Op::AddPhoto { photo: Box::new(photo) })?;
                    outcome.added += 1;
                }
                Some(previous) => {
                    let unchanged = previous.size == seed.size && previous.key == seed.key;
                    if unchanged {
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
    pub fn snapshot(&mut self) -> Result<()> {
        let snapshot = Snapshot {
            schema: SCHEMA,
            seq: self.seq,
            next_id: self.next_id,
            roots: self.roots.clone(),
            photos: self.photos.values().cloned().collect(),
            collections: self.collections.values().cloned().collect(),
            next_collection: self.next_collection,
        };
        let text = serde_json::to_string(&snapshot).map_err(|error| Error::Invalid(error.to_string()))?;
        let tmp = self.dir.join("snapshot.json.tmp");
        let path = self.dir.join(SNAPSHOT_NAME);
        fs::write(&tmp, text.as_bytes()).map_err(|error| Error::io(tmp.display().to_string(), error))?;
        fs::rename(&tmp, &path).map_err(|error| Error::io(path.display().to_string(), error))?;

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
        Ok(())
    }

    /// Flush pending records and write a snapshot. Safe to call more than once.
    pub fn close(&mut self) -> Result<()> {
        self.log.flush().map_err(|error| Error::io(self.dir.display().to_string(), error))?;
        self.snapshot()
    }

    fn append(&mut self, seq: u64, op: &Op) -> Result<()> {
        let line = encode(seq, op)?;
        self.log
            .write_all(line.as_bytes())
            .and_then(|()| self.log.flush())
            .map_err(|error| Error::io(self.dir.display().to_string(), error))
    }
}

/// Encode one record as `<crc32> <json>\n`.
fn encode(seq: u64, op: &Op) -> Result<String> {
    let record = Record { seq, op: op.clone() };
    let json = serde_json::to_string(&record).map_err(|error| Error::Invalid(error.to_string()))?;
    let crc = crc32fast::hash(json.as_bytes());
    Ok(format!("{crc:08x} {json}\n"))
}

/// Decode one line, returning `None` when the checksum or the JSON does not hold.
fn decode(line: &str) -> Option<(u64, Op)> {
    let (crc_text, json) = line.split_once(' ')?;
    let crc = u32::from_str_radix(crc_text, 16).ok()?;
    if crc32fast::hash(json.as_bytes()) != crc {
        return None;
    }
    let record: Record = serde_json::from_str(json).ok()?;
    Some((record.seq, record.op))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::query::Filter;
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
            tag: Some("beach".into()),
            ..Filter::default()
        };
        assert_eq!(catalog.count(&filter), 2);
        let filter = Filter {
            tag: Some("sunset".into()),
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
