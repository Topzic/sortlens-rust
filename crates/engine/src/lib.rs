//! L4: the command layer.
//!
//! The engine owns the open catalog, the scan worker and the command registry. The UI, the CLI
//! and the MCP server all call the same typed methods, and [`Engine::run`] is the JSON face of
//! them for scripts.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod command;
pub mod job;

pub use command::{CommandSpec, Group, Registry, Status};
pub use job::{Job, JobKind, JobStatus};
pub use sortlens_catalog::backup::Backup;
pub use sortlens_catalog::history::Entry as HistoryEntry;
pub use sortlens_catalog::{Filter, Op, Page, Photo, Sort, Stats, SyncOutcome, TagMode, TagOrder};
pub use sortlens_core::{ColorLabel, Error, Flag, MediaKind, PhotoId, Result};
pub use sortlens_cull::trash::Receipt as ApplyReceipt;
pub use sortlens_cull::{Decision, Summary as CullSummary};
pub use sortlens_insights::{Insights, Row as InsightRow};
pub use sortlens_similar::{
    DEFAULT_COLOUR as SIMILAR_COLOUR, DEFAULT_THRESHOLD as SIMILAR_THRESHOLD, Group as SimilarGroup, Member as SimilarMember,
};

use serde_json::{Value, json};
use sortlens_catalog::{Catalog, Seed};
use sortlens_scan::{ScanControl, ScanProgress, ScanReport};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Instant;

/// What a command asks the caller to do when the effect is not the engine's to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Nothing further; the result value is the answer.
    None,
    /// Ask the host to open the settings dialog.
    OpenSettings,
    /// Ask the host to close the window.
    Quit,
}

/// A scan running on a worker thread.
struct PendingScan {
    root: String,
    job: u64,
    rx: Receiver<Result<ScanReport>>,
    control: ScanControl,
    started: Instant,
}

/// A hash pass in flight.
struct PendingHash {
    job: u64,
    rx: Receiver<Vec<SimilarMember>>,
    total: usize,
    done: Arc<std::sync::atomic::AtomicUsize>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

/// What a finished scan changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanFinished {
    /// Root that was scanned.
    pub root: String,
    /// New files.
    pub added: usize,
    /// Files whose size or content changed.
    pub updated: usize,
    /// Files that disappeared.
    pub missing: usize,
    /// Files that reappeared.
    pub restored: usize,
    /// Read failures, as messages.
    pub errors: Vec<String>,
    /// How long the walk took.
    pub elapsed: std::time::Duration,
    /// The walk was cancelled; nothing was committed.
    pub cancelled: bool,
}

/// The command engine.
pub struct Engine {
    registry: Registry,
    dir: Option<PathBuf>,
    last_effect: Effect,
    catalog: Option<Catalog>,
    pending: Option<PendingScan>,
    pending_hash: Option<PendingHash>,
    hashes: std::collections::HashMap<u64, (sortlens_hash::Hash, sortlens_hash::Colour)>,
    similar_members: Vec<SimilarMember>,
    similar: Vec<SimilarGroup>,
    receipt: Option<ApplyReceipt>,
    cull: Option<sortlens_cull::Session>,
    deleted: Vec<sortlens_cull::trash::DeletedBatch>,
    next_batch: u64,
    jobs: Vec<Job>,
    next_job: u64,
    write_sidecars: bool,
    warnings: Vec<String>,
}

impl Engine {
    /// An engine with no library open.
    pub fn new() -> Self {
        Self {
            registry: Registry::v0(),
            last_effect: Effect::None,
            dir: None,
            catalog: None,
            pending: None,
            pending_hash: None,
            hashes: std::collections::HashMap::new(),
            similar_members: Vec::new(),
            similar: Vec::new(),
            receipt: None,
            cull: None,
            deleted: Vec::new(),
            next_batch: 1,
            jobs: Vec::new(),
            next_job: 1,
            write_sidecars: true,
            warnings: Vec::new(),
        }
    }

    /// Open (or create) the catalog in `dir`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        let (catalog, report) = Catalog::open(&dir)?;
        let mut engine = Self {
            dir: Some(dir),
            catalog: Some(catalog),
            warnings: report.warnings,
            ..Self::new()
        };
        engine.restore_cull();
        engine.load_deleted();
        Ok(engine)
    }

    /// Where the rejects bin is saved.
    fn deleted_path(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|dir| dir.join("deleted.json"))
    }

    fn load_deleted(&mut self) {
        let Some(path) = self.deleted_path() else {
            return;
        };
        let Ok(text) = std::fs::read(&path) else {
            return;
        };
        match serde_json::from_slice::<Vec<sortlens_cull::trash::DeletedBatch>>(&text) {
            Ok(batches) => {
                self.next_batch = batches.iter().map(|batch| batch.id).max().unwrap_or(0) + 1;
                self.deleted = batches;
            }
            Err(_) => self.warnings.push("ignored an unreadable rejects bin".to_string()),
        }
    }

    fn persist_deleted(&mut self) {
        let Some(path) = self.deleted_path() else {
            return;
        };
        let result = serde_json::to_vec_pretty(&self.deleted)
            .map_err(|error| Error::Invalid(format!("rejects bin: {error}")))
            .and_then(|bytes| write_atomic(&path, &bytes));
        if let Err(error) = result {
            self.warnings.push(format!("could not save the rejects bin: {error}"));
        }
    }

    /// The photo before or after `id` in the current view order, clamped at the ends.
    ///
    /// Detail navigation walks the same order the grid shows, so `←`/`→` never jumps somewhere the
    /// user cannot see. Returns `None` at the ends rather than wrapping.
    pub fn neighbour(&self, filter: &Filter, sort: Sort, id: PhotoId, delta: isize) -> Option<PhotoId> {
        const WINDOW: usize = 5000;
        let page = self.page(filter, sort, 0, WINDOW);
        let index = page.photos.iter().position(|photo| photo.id == id)?;
        let target = index as isize + delta;
        if target < 0 {
            return None;
        }
        page.photos.get(target as usize).map(|photo| photo.id)
    }

    /// Position of a photo in the current view order, one-based, with the total.
    pub fn position(&self, filter: &Filter, sort: Sort, id: PhotoId) -> Option<(usize, usize)> {
        const WINDOW: usize = 5000;
        let page = self.page(filter, sort, 0, WINDOW);
        let index = page.photos.iter().position(|photo| photo.id == id)?;
        Some((index + 1, page.photos.len()))
    }

    /// Immediate subfolders of a scope, with recursive photo counts.
    pub fn subfolders(&self, root: &str, prefix: &str) -> Vec<(String, usize)> {
        self.catalog
            .as_ref()
            .map(|catalog| catalog.folders(root, prefix))
            .unwrap_or_default()
    }

    /// Every collection, by name.
    pub fn collections(&self) -> Vec<sortlens_catalog::Collection> {
        self.catalog.as_ref().map(Catalog::collections).unwrap_or_default()
    }

    /// Create a collection.
    pub fn create_collection(&mut self, name: &str) -> Result<sortlens_core::CollectionId> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .create_collection(name)
    }

    /// Rename a collection.
    pub fn rename_collection(&mut self, id: sortlens_core::CollectionId, name: &str) -> Result<()> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .rename_collection(id, name)
    }

    /// Set or clear a collection's colour.
    pub fn set_collection_colour(&mut self, id: sortlens_core::CollectionId, colour: Option<ColorLabel>) -> Result<()> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .set_collection_colour(id, colour)
    }

    /// Pin or unpin a collection.
    pub fn set_collection_favourite(&mut self, id: sortlens_core::CollectionId, favourite: bool) -> Result<()> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .set_collection_favourite(id, favourite)
    }

    /// Remember the sort a collection is browsed with.
    pub fn set_collection_sort(&mut self, id: sortlens_core::CollectionId, sort: Sort) -> Result<()> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .set_collection_sort(id, sort)
    }

    /// Remember the search a collection was narrowed with.
    pub fn set_collection_search(&mut self, id: sortlens_core::CollectionId, search: &str) -> Result<()> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .set_collection_search(id, search)
    }

    /// Delete a collection; the photos stay in the library.
    pub fn delete_collection(&mut self, id: sortlens_core::CollectionId) -> Result<()> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .delete_collection(id)
    }

    /// Add photos to a collection.
    pub fn add_to_collection(&mut self, ids: &[PhotoId], id: sortlens_core::CollectionId) -> Result<usize> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .collect(id, ids, &[])
    }

    /// Remove photos from a collection.
    pub fn remove_from_collection(&mut self, ids: &[PhotoId], id: sortlens_core::CollectionId) -> Result<usize> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .collect(id, &[], ids)
    }

    /// Check every photo against the disk and record what changed.
    ///
    /// This is the "library doctor": it stats each file, then commits one group of missing flags, so
    /// the health report and the badges agree and the whole check is one undo step.
    pub fn check_health(&mut self) -> Result<Health> {
        let mut health = Health::default();
        let photos: Vec<(PhotoId, String, bool)> = self
            .all_photos()
            .into_iter()
            .map(|photo| (photo.id, photo.root.clone(), photo.rel.clone()))
            .map(|(id, root, rel)| (id, format!("{root}/{rel}"), false))
            .collect();
        let mut changes = Vec::new();
        for (id, _path, _) in &photos {
            let photo = self.photo(*id).ok_or(Error::NoSuchPhoto(*id))?;
            let exists = PathBuf::from(&photo.root).join(&photo.rel).exists();
            health.checked += 1;
            if exists {
                health.present += 1;
                if photo.missing {
                    changes.push((*id, false));
                }
            } else {
                health.missing += 1;
                if !photo.missing {
                    changes.push((*id, true));
                }
            }
        }
        if !changes.is_empty() {
            let ops = changes
                .iter()
                .map(|(id, missing)| Op::Missing {
                    id: *id,
                    missing: *missing,
                })
                .collect();
            self.mutate(Op::Group { ops })?;
        }
        health.changed = changes.len();
        Ok(health)
    }

    /// Recent changes, newest first, read from the log.
    pub fn history(&mut self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let dir = self.dir.clone().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        if let Some(catalog) = self.catalog.as_mut() {
            catalog.flush()?;
        }
        sortlens_catalog::history::recent(&dir, limit)
    }

    /// Write the change report — the log as plain text — for a support request.
    pub fn export_history(&mut self, path: &Path) -> Result<usize> {
        let dir = self.dir.clone().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        if let Some(catalog) = self.catalog.as_mut() {
            catalog.flush()?;
        }
        let text = sortlens_catalog::history::export(&dir, HISTORY_LIMIT, VERSION)?;
        std::fs::write(path, text.as_bytes()).map_err(|error| Error::io(path.display().to_string(), error))?;
        Ok(text.lines().count())
    }

    /// Audit the catalog files without changing them.
    pub fn audit(&mut self) -> Result<sortlens_catalog::integrity::Audit> {
        let dir = self.dir.clone().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        if let Some(catalog) = self.catalog.as_mut() {
            catalog.flush()?;
        }
        sortlens_catalog::integrity::audit(&dir)
    }

    /// Compact the catalog: snapshot everything, then start a fresh log.
    ///
    /// Also the repair path — a torn log tail or a stale record cannot survive it, because the log
    /// is rewritten from a clean snapshot.
    pub fn compact(&mut self) -> Result<u64> {
        // Compaction rewrites the log from the snapshot, so it would cement whatever the snapshot
        // got wrong. A torn log tail is the one thing it is meant to repair; anything else the audit
        // found is a reason to stop and ask rather than to write.
        let audit = self.audit()?;
        if !audit.is_clean() && !audit.torn_tail {
            return Err(Error::Invalid(format!("refusing to compact: {}", audit.notes.join("; "))));
        }
        let catalog = self.catalog.as_mut().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        catalog.compact()
    }

    /// Repoint an indexed root at a new location, after checking a sample of its photos.
    ///
    /// A renamed or moved volume is the common case: every file keeps its relative path, so the
    /// whole root can be repointed in one undoable change. The sample — up to [`REMAP_SAMPLE`] photos
    /// spread through the root, each checked for presence and size — is what makes this safe to offer
    /// on a path typed by hand: a wrong folder fails loudly here instead of marking the library
    /// missing on the next scan.
    pub fn remap_root(&mut self, from: &str, to: &str) -> Result<usize> {
        let from = from.trim_end_matches('/');
        let to = to.trim_end_matches('/').to_string();
        if from.is_empty() || to.is_empty() {
            return Err(Error::Invalid("both a current and a new folder are required".into()));
        }
        if from == to {
            return Err(Error::Invalid("the folder is already this one".into()));
        }
        if !self.roots().iter().any(|root| root.trim_end_matches('/') == from) {
            return Err(Error::Invalid(format!("{from} is not indexed")));
        }
        if self.roots().iter().any(|root| root.trim_end_matches('/') == to) {
            return Err(Error::Invalid(format!("{to} is already indexed")));
        }
        if !Path::new(&to).is_dir() {
            return Err(Error::Invalid(format!("{to} is not a folder")));
        }

        let mut members: Vec<Photo> = self
            .all_photos()
            .into_iter()
            .filter(|photo| photo.root.trim_end_matches('/') == from)
            .collect();
        members.sort_by(|a, b| a.rel.cmp(&b.rel));
        if members.is_empty() {
            return Err(Error::Invalid(format!("no photos are indexed under {from}")));
        }

        let sampled = sample_indices(members.len(), REMAP_SAMPLE);
        let mut absent = Vec::new();
        for index in &sampled {
            let photo = &members[*index];
            let candidate = Path::new(&to).join(&photo.rel);
            match std::fs::metadata(&candidate) {
                Ok(meta) if meta.len() == photo.size => {}
                _ => absent.push(photo.rel.clone()),
            }
        }
        if !absent.is_empty() {
            absent.sort();
            let list = absent.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            return Err(Error::Invalid(format!(
                "{} of {} sampled photo(s) were not found under {to} ({list})",
                absent.len(),
                sampled.len()
            )));
        }

        let count = members.len();
        self.mutate(Op::RemapRoot {
            from: from.to_string(),
            to,
        })?;
        Ok(count)
    }

    /// Look for missing photos elsewhere under their root and record the new paths.
    ///
    /// Matching is by content key — size plus a hash of the ends — so a moved or renamed file is
    /// found without reading every byte.
    pub fn relocate_missing(&mut self) -> Result<usize> {
        let missing = self.catalog.as_ref().map(|catalog| catalog.missing_photos()).unwrap_or_default();
        if missing.is_empty() {
            return Ok(0);
        }
        let mut by_root: std::collections::BTreeMap<String, Vec<Photo>> = std::collections::BTreeMap::new();
        for photo in missing {
            by_root.entry(photo.root.clone()).or_default().push(photo);
        }
        let mut changes = Vec::new();
        for (root, photos) in by_root {
            let Ok(report) = sortlens_scan::scan_root(&PathBuf::from(&root)) else {
                continue;
            };
            let taken: std::collections::HashSet<String> = self
                .all_photos()
                .into_iter()
                .filter(|photo| photo.root == root && !photo.missing)
                .map(|photo| photo.rel)
                .collect();
            for photo in photos {
                if let Some(found) = report
                    .files
                    .iter()
                    .find(|file| file.size == photo.size && file.key == photo.key && !taken.contains(&file.rel))
                {
                    changes.push((photo.id, found.rel.clone()));
                }
            }
        }
        if changes.is_empty() {
            return Ok(0);
        }
        let count = changes.len();
        self.mutate(Op::Relocate { changes })?;
        Ok(count)
    }

    /// Remove every photo the catalog believes is missing.
    pub fn remove_missing(&mut self) -> Result<usize> {
        match self.catalog.as_mut() {
            Some(catalog) => catalog.remove_missing(),
            None => Ok(0),
        }
    }

    /// Every photo in the library, whatever the filters.
    /// Content key for a photo, so a renamed file keeps its hash.
    fn key_for(&self, id: PhotoId) -> Option<u64> {
        self.catalog.as_ref()?.photo(id).map(|photo| photo.key)
    }

    fn all_photos(&self) -> Vec<Photo> {
        match &self.catalog {
            Some(catalog) => catalog.page(&Filter::default(), Sort::Natural, 0, 1_000_000).photos,
            None => Vec::new(),
        }
    }

    /// Applied batches, newest last, for the rejects bin.
    pub fn deleted_batches(&self) -> &[sortlens_cull::trash::DeletedBatch] {
        &self.deleted
    }

    /// Restore one applied batch: put moved files back and restore the catalog state.
    ///
    /// Returns how many photos were restored. Files sent to the system Trash cannot be restored by
    /// Sortlens and are reported by name instead of being silently skipped.
    pub fn restore_batch(&mut self, id: u64) -> Result<usize> {
        let index = self
            .deleted
            .iter()
            .position(|batch| batch.id == id)
            .ok_or_else(|| Error::Invalid(format!("no applied batch {id}")))?;
        let batch = self.deleted[index].clone();
        let report = sortlens_cull::trash::undo(&batch.items);
        // A file that cannot come back would leave the catalog claiming it exists: refuse the
        // restore and say which file needs handling by hand.
        if let Some((_, message)) = report.failed.first() {
            self.warnings.push(message.clone());
            return Err(Error::Invalid(message.clone()));
        }

        let mut ops = Vec::new();
        for item in &batch.items {
            ops.push(Op::Missing {
                id: item.id,
                missing: false,
            });
        }
        if !batch.previous_flags.is_empty() {
            ops.push(Op::Flag {
                changes: batch.previous_flags.clone(),
            });
        }
        self.mutate(Op::Group { ops })?;
        self.deleted.remove(index);
        self.persist_deleted();
        Ok(batch.items.len())
    }

    /// Write the library's catalog into one verified archive.
    ///
    /// The catalog is flushed first, so the archive matches what is on screen rather than what was
    /// last written. Photos are never copied — a backup carries the index and the review state.
    pub fn export_backup(&mut self, path: &Path) -> Result<usize> {
        let dir = self.dir.clone().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .flush()?;
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs() as i64)
            .unwrap_or_default();
        let backup = sortlens_catalog::backup::create(&dir, created)?;
        let bytes = sortlens_catalog::backup::encode(&backup)?;
        let tmp = path.with_extension("backup.tmp");
        std::fs::write(&tmp, &bytes).map_err(|error| Error::io(tmp.display().to_string(), error))?;
        std::fs::rename(&tmp, path).map_err(|error| Error::io(path.display().to_string(), error))?;
        let files = backup.len();
        let job = self.start_job(JobKind::Export, format!("Backup · {files} file(s)"));
        self.finish_job(job, JobStatus::Done, files, 0, 0, None);
        Ok(files)
    }

    /// Verify an archive and unpack it into `dest` for opening as a separate library.
    ///
    /// Restoring never writes into the open library: a backup older than the current state would
    /// quietly undo work. The caller opens the restored folder when it is ready.
    pub fn import_backup(path: &Path, dest: &Path) -> Result<usize> {
        let bytes = std::fs::read(path).map_err(|error| Error::io(path.display().to_string(), error))?;
        let backup = sortlens_catalog::backup::read(&bytes)?;
        let written = sortlens_catalog::backup::restore(&backup, dest)?;
        Ok(written.len())
    }

    /// Forget one applied batch: its files stay where they are, only the restore record goes.
    ///
    /// This is the "empty the bin" half of the bin. It cannot be undone, because the record it
    /// removes is the only thing that knew how to put the files back — which is why the dialog that
    /// calls it says so.
    pub fn forget_batch(&mut self, id: u64) -> Result<usize> {
        let index = self
            .deleted
            .iter()
            .position(|batch| batch.id == id)
            .ok_or_else(|| Error::Invalid(format!("no applied batch {id}")))?;
        let files = self.deleted.remove(index).items.len();
        self.persist_deleted();
        Ok(files)
    }

    /// Forget every applied batch. Returns how many batches and files went.
    pub fn forget_all_batches(&mut self) -> (usize, usize) {
        let batches = self.deleted.len();
        let files = self.deleted.iter().map(|batch| batch.items.len()).sum();
        if batches > 0 {
            self.deleted.clear();
            self.persist_deleted();
        }
        (batches, files)
    }

    /// Where a cull session is saved between runs.
    fn cull_path(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|dir| dir.join("cull-session.json"))
    }

    /// Save the session, or remove the file when there is none.
    ///
    /// A failure here is a warning, never an error: losing the ability to resume must not stop a
    /// photographer from culling.
    fn persist_cull(&mut self) {
        let Some(path) = self.cull_path() else {
            return;
        };
        let result = match &self.cull {
            Some(session) => {
                let json = serde_json::to_vec_pretty(&session.snapshot()).map_err(|error| Error::Invalid(format!("cull session: {error}")));
                json.and_then(|bytes| write_atomic(&path, &bytes))
            }
            None => match std::fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(Error::io(path.display().to_string(), error)),
            },
        };
        if let Err(error) = result {
            self.warnings.push(format!("could not save the cull session: {error}"));
        }
    }

    /// Restore a session saved by a previous run, dropping photos that no longer exist.
    fn restore_cull(&mut self) {
        let Some(path) = self.cull_path() else {
            return;
        };
        let Ok(text) = std::fs::read(&path) else {
            return;
        };
        let Ok(snapshot) = serde_json::from_slice::<sortlens_cull::Snapshot>(&text) else {
            self.warnings.push("ignored an unreadable cull session file".to_string());
            return;
        };
        let Some(mut session) = sortlens_cull::Session::restore(snapshot) else {
            return;
        };
        session.retain(|id| self.catalog.as_ref().is_some_and(|catalog| catalog.photo(id).is_some()));
        if session.queue().is_empty() {
            let _ = std::fs::remove_file(&path);
            return;
        }
        self.cull = Some(session);
    }

    /// The command table, for menus and the palette.
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// The effect requested by the most recent [`Self::run`] call.
    pub fn last_effect(&self) -> Effect {
        self.last_effect
    }

    /// Problems reported while opening the catalog, and scan failures.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Whether a catalog is open.
    pub fn has_library(&self) -> bool {
        self.catalog.is_some()
    }

    /// Whether a scan is running.
    pub fn is_scanning(&self) -> bool {
        self.pending.is_some()
    }

    /// Registered roots.
    pub fn roots(&self) -> &[String] {
        self.catalog.as_ref().map(|catalog| catalog.roots()).unwrap_or(&[])
    }

    /// One page of photos.
    pub fn page(&self, filter: &Filter, sort: Sort, offset: usize, limit: usize) -> Page {
        match &self.catalog {
            Some(catalog) => catalog.page(filter, sort, offset, limit),
            None => Page::default(),
        }
    }

    /// How many photos match a filter.
    pub fn count(&self, filter: &Filter) -> usize {
        self.catalog.as_ref().map(|catalog| catalog.count(filter)).unwrap_or(0)
    }

    /// Photo counts per root, for the sidebar.
    pub fn counts_by_root(&self) -> Vec<(String, usize)> {
        self.catalog.as_ref().map(|catalog| catalog.counts_by_root()).unwrap_or_default()
    }

    /// Counters for the status bar.
    pub fn stats(&mut self) -> Stats {
        self.catalog.as_mut().map(Catalog::stats).unwrap_or_default()
    }

    /// One photo by id.
    pub fn photo(&self, id: PhotoId) -> Option<&Photo> {
        self.catalog.as_ref().and_then(|catalog| catalog.photo(id))
    }

    /// Start scanning a folder on a worker thread.
    ///
    /// Returns immediately. Poll [`Self::poll`] each frame until it yields a [`ScanFinished`].
    pub fn begin_scan(&mut self, path: impl Into<String>) -> Result<()> {
        if self.catalog.is_none() {
            return Err(Error::Invalid("no library is open".into()));
        }
        if self.pending.is_some() {
            return Err(Error::Invalid("a scan is already running".into()));
        }
        let root: String = path.into();
        let root_path = PathBuf::from(&root);
        let (tx, rx) = channel();
        let control = ScanControl::new();
        let worker_control = control.clone();
        std::thread::Builder::new()
            .name("sortlens-scan".into())
            .spawn(move || {
                let _ = tx.send(sortlens_scan::scan_root_with(&root_path, &worker_control));
            })
            .map_err(|error| Error::Invalid(format!("could not start the scan worker: {error}")))?;
        let job = self.start_job(JobKind::Scan, folder_name(&root));
        self.pending = Some(PendingScan {
            root,
            job,
            rx,
            control,
            started: Instant::now(),
        });
        Ok(())
    }

    /// The activity stack, newest last, with the running scan's counters filled in.
    pub fn jobs(&self) -> Vec<Job> {
        let mut jobs = self.jobs.clone();
        if let (Some(progress), Some(pending)) = (self.scan_progress(), &self.pending)
            && let Some(job) = jobs.iter_mut().find(|job| job.id == pending.job)
        {
            job.files = progress.files;
            job.dirs = progress.dirs;
            job.unreadable = progress.errors;
        }
        jobs
    }

    fn start_job(&mut self, kind: JobKind, label: impl Into<String>) -> u64 {
        const KEEP: usize = 50;
        if self.jobs.len() >= KEEP
            && let Some(index) = self.jobs.iter().position(|job| !job.status.is_running())
        {
            self.jobs.remove(index);
        }
        let id = self.next_job;
        self.next_job += 1;
        self.jobs.push(Job::running(id, kind, label));
        id
    }

    fn finish_job(&mut self, id: u64, status: JobStatus, files: usize, dirs: usize, unreadable: usize, error: Option<String>) {
        if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) {
            job.files = files;
            job.dirs = dirs;
            job.unreadable = unreadable;
            job.error = error;
            job.finish(status);
        }
    }

    /// Files and folders visited so far, while a scan is running.
    pub fn scan_progress(&self) -> Option<ScanProgress> {
        self.pending.as_ref().map(|pending| pending.control.snapshot())
    }

    /// Ask the running scan to stop. It finishes the directory it is in and commits nothing.
    pub fn cancel_scan(&mut self) -> bool {
        match &self.pending {
            Some(pending) => {
                pending.control.cancel();
                true
            }
            None => false,
        }
    }

    /// Collect a finished scan, if one is ready.
    pub fn poll(&mut self) -> Option<ScanFinished> {
        let pending = self.pending.as_ref()?;
        match pending.rx.try_recv() {
            Ok(Ok(report)) => {
                let pending = self.pending.take()?;
                let elapsed = pending.started.elapsed();
                Some(self.finish_scan(pending.root, pending.job, report, elapsed))
            }
            Ok(Err(error)) => {
                let pending = self.pending.take()?;
                self.warnings.push(format!("{}: {error}", pending.root));
                self.finish_job(pending.job, JobStatus::Failed, 0, 0, 0, Some(error.to_string()));
                Some(ScanFinished {
                    root: pending.root,
                    added: 0,
                    updated: 0,
                    missing: 0,
                    restored: 0,
                    errors: vec![error.to_string()],
                    elapsed: pending.started.elapsed(),
                    cancelled: false,
                })
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                None
            }
        }
    }

    fn finish_scan(&mut self, root: String, job: u64, report: ScanReport, elapsed: std::time::Duration) -> ScanFinished {
        if report.cancelled {
            self.finish_job(job, JobStatus::Cancelled, 0, 0, 0, None);
            // A partial walk must never be synced: it would mark every unseen file as missing.
            return ScanFinished {
                root,
                added: 0,
                updated: 0,
                missing: 0,
                restored: 0,
                errors: Vec::new(),
                elapsed,
                cancelled: true,
            };
        }
        let seeds: Vec<Seed> = report
            .files
            .iter()
            .map(|file| Seed {
                rel: file.rel.clone(),
                size: file.size,
                modified: file.modified,
                kind: file.kind,
                key: file.key,
                meta: file.meta.clone(),
                review: file.review.clone(),
                edited: file.edited,
            })
            .collect();
        let errors: Vec<String> = report
            .errors
            .iter()
            .map(|error| format!("{}: {}", error.path.display(), error.message))
            .collect();

        let outcome = match self.catalog.as_mut() {
            Some(catalog) => match catalog.sync_root(&root, seeds) {
                Ok(outcome) => outcome,
                Err(error) => {
                    self.warnings.push(format!("{root}: {error}"));
                    SyncOutcome::default()
                }
            },
            None => SyncOutcome::default(),
        };

        for error in &errors {
            self.warnings.push(error.clone());
        }

        let status = if errors.is_empty() { JobStatus::Done } else { JobStatus::Failed };
        self.finish_job(job, status, report.files.len(), report.dirs, errors.len(), errors.first().cloned());

        ScanFinished {
            root,
            added: outcome.added,
            updated: outcome.updated,
            missing: outcome.missing,
            restored: outcome.restored,
            errors,
            elapsed,
            cancelled: false,
        }
    }

    /// Set stars on a selection, clamped to 0–5.
    pub fn rate(&mut self, ids: &[PhotoId], stars: u8) -> Result<usize> {
        let changed = self.mutate(Op::Rate {
            changes: ids.iter().map(|id| (*id, stars.min(5))).collect(),
        })?;
        self.sync_sidecars(ids);
        Ok(changed)
    }

    /// Set a flag on a selection.
    pub fn flag(&mut self, ids: &[PhotoId], flag: Flag) -> Result<usize> {
        let changed = self.mutate(Op::Flag {
            changes: ids.iter().map(|id| (*id, flag)).collect(),
        })?;
        self.sync_sidecars(ids);
        Ok(changed)
    }

    /// Add or remove one keyword on a selection.
    pub fn set_tag(&mut self, ids: &[PhotoId], tag: &str, add: bool) -> Result<usize> {
        let tag = normalize_tag(tag)?;
        let mut changes = Vec::new();
        for id in ids {
            let photo = self.photo(*id).ok_or(Error::NoSuchPhoto(*id))?;
            let mut tags = photo.tags.clone();
            if add {
                if !tags.iter().any(|existing| existing == &tag) {
                    tags.push(tag.clone());
                }
            } else {
                tags.retain(|existing| existing != &tag);
            }
            tags.sort_unstable();
            tags.dedup();
            if tags != photo.tags {
                changes.push((*id, tags));
            }
        }
        if changes.is_empty() {
            return Ok(0);
        }
        let count = changes.len();
        self.mutate(Op::Tagged { changes })?;
        self.sync_sidecars(ids);
        Ok(count)
    }

    /// Rename a keyword everywhere, merging into an existing one when the name is taken.
    ///
    /// Sidecars follow, so a rename is not silently undone the next time a file is read.
    pub fn rename_tag(&mut self, from: &str, to: &str) -> Result<usize> {
        let from = normalize_tag(from)?;
        let to = normalize_tag(to)?;
        let touched: Vec<PhotoId> = self
            .all_photos()
            .into_iter()
            .filter(|photo| photo.has_tag(&from))
            .map(|photo| photo.id)
            .collect();
        let catalog = self.catalog.as_mut().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        let count = catalog.rename_tag(&from, &to)?;
        if count > 0 {
            self.sync_sidecars(&touched);
            let job = self.start_job(JobKind::Sidecar, format!("#{from} → #{to}"));
            self.finish_job(job, JobStatus::Done, count, 0, 0, None);
        }
        Ok(count)
    }

    /// Statistics over the whole library, recomputed each time.
    pub fn insights(&self) -> Insights {
        match self.catalog.as_ref() {
            Some(catalog) => Insights::compute(catalog.photos()),
            None => Insights::empty(),
        }
    }

    /// Groups found by the last finished hash pass.
    pub fn similar_groups(&self) -> &[SimilarGroup] {
        &self.similar
    }

    /// Start hashing every photo, reusing hashes already computed for unchanged files.
    ///
    /// Returns the number of photos that still need a hash: zero means the groups are already
    /// up to date and no job was started.
    pub fn begin_similar(&mut self, threshold: u32) -> Result<usize> {
        if self.catalog.is_none() {
            return Err(Error::Invalid("no library is open".into()));
        }
        if self.pending_hash.is_some() {
            return Err(Error::Invalid("a similarity pass is already running".into()));
        }
        let mut members: Vec<SimilarMember> = Vec::new();
        let mut todo: Vec<(PhotoId, String, u64, u64, u8)> = Vec::new();
        for photo in self.all_photos() {
            match self.hashes.get(&photo.key) {
                Some((hash, colour)) => members.push(SimilarMember {
                    id: photo.id,
                    hash: *hash,
                    colour: Some(*colour),
                    pixels: u64::from(photo.width) * u64::from(photo.height),
                    rating: photo.rating,
                    bytes: photo.size,
                }),
                None => todo.push((
                    photo.id,
                    photo.path(),
                    u64::from(photo.width) * u64::from(photo.height),
                    photo.size,
                    photo.rating,
                )),
            }
        }
        if todo.is_empty() {
            self.similar = sortlens_similar::group(&members, threshold);
            return Ok(0);
        }

        let (tx, rx) = channel();
        let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_done = Arc::clone(&done);
        let worker_cancel = Arc::clone(&cancel);
        let total = todo.len();
        std::thread::Builder::new()
            .name("sortlens-hash".into())
            .spawn(move || {
                let mut found = Vec::with_capacity(todo.len());
                for (id, path, pixels, bytes, rating) in todo {
                    if worker_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    if let Some((hash, colour)) = sortlens_hash::analyse_file(std::path::Path::new(&path), 64) {
                        found.push(SimilarMember {
                            id,
                            hash,
                            colour: Some(colour),
                            pixels,
                            rating,
                            bytes,
                        });
                    }
                    worker_done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let _ = tx.send(found);
            })
            .map_err(|error| Error::Invalid(format!("could not start the hash worker: {error}")))?;

        let job = self.start_job(JobKind::Hash, format!("{total} photos"));
        self.pending_hash = Some(PendingHash {
            job,
            rx,
            total,
            done,
            cancel,
        });
        // Keep the members already known so new hashes can be folded in without re-reading them.
        self.similar_members = members;
        Ok(total)
    }

    /// How far a running or finished hash pass got.
    pub fn hash_progress(&self) -> Option<(usize, usize)> {
        self.pending_hash
            .as_ref()
            .map(|pending| (pending.done.load(std::sync::atomic::Ordering::Relaxed), pending.total))
    }

    /// Stop the running hash pass; hashes already computed are kept.
    pub fn cancel_similar(&mut self) {
        if let Some(pending) = &self.pending_hash {
            pending.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Fold finished hashes into the cache and regroup. Returns the number of groups when the pass
    /// just finished.
    pub fn poll_similar(&mut self, threshold: u32) -> Option<usize> {
        self.poll_similar_with(threshold, SIMILAR_COLOUR)
    }

    /// Poll with an explicit colour tolerance.
    pub fn poll_similar_with(&mut self, threshold: u32, colour: u32) -> Option<usize> {
        let pending = self.pending_hash.as_ref()?;
        let found = match pending.rx.try_recv() {
            Ok(found) => found,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                self.pending_hash = None;
                return None;
            }
        };
        let pending = self.pending_hash.take()?;
        for member in &found {
            let key = self.key_for(member.id).unwrap_or_default();
            if let Some(colour) = member.colour {
                self.hashes.insert(key, (member.hash, colour));
            }
            self.similar_members.push(*member);
        }
        let status = if pending.cancel.load(std::sync::atomic::Ordering::Relaxed) {
            JobStatus::Cancelled
        } else {
            JobStatus::Done
        };
        self.finish_job(pending.job, status, found.len(), 0, 0, None);
        self.similar = sortlens_similar::group_with(&self.similar_members, threshold, colour);
        Some(self.similar.len())
    }

    /// Groups around one photo, for "find similar to this".
    pub fn similar_to(&self, id: PhotoId, threshold: u32) -> Option<SimilarGroup> {
        sortlens_similar::around(&self.similar_members, id, threshold)
    }

    /// Every keyword in use, most used first.
    pub fn tag_vocabulary(&self) -> Vec<(String, usize)> {
        self.tag_vocabulary_in(TagOrder::Count)
    }

    /// Every keyword in use, in the requested order.
    pub fn tag_vocabulary_in(&self, order: TagOrder) -> Vec<(String, usize)> {
        self.catalog
            .as_ref()
            .map(|catalog| catalog.tag_vocabulary_in(order))
            .unwrap_or_default()
    }

    /// Start a cull session over the listed photos, in the order given.
    ///
    /// Photos the catalog does not know are ignored; nothing is staged yet, and nothing is written
    /// until the session is applied.
    pub fn begin_cull(&mut self, ids: &[PhotoId]) -> Result<usize> {
        let mut queue = Vec::with_capacity(ids.len());
        let mut flags = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(photo) = self.photo(*id) {
                queue.push(photo.id);
                flags.push((photo.id, photo.flag));
            }
        }
        if queue.is_empty() {
            return Err(Error::Invalid("there are no photos to cull".into()));
        }
        let total = queue.len();
        self.cull = Some(sortlens_cull::Session::new(queue, flags));
        self.persist_cull();
        Ok(total)
    }

    /// Apply the session: commit every changed decision, then dispose of the rejects.
    ///
    /// The batch is audited first and the catalog change is one `Op::Group`, so a failed audit
    /// leaves both the session and the catalog untouched, and one undo reverses the whole batch.
    pub fn cull_apply(&mut self, mode: sortlens_cull::trash::Mode) -> Result<sortlens_cull::trash::Report> {
        let Some(session) = self.cull.take() else {
            return Err(Error::Invalid("no cull session is running".into()));
        };
        match self.apply_session(&session, mode) {
            Ok(report) => Ok(report),
            Err(error) => {
                // A refused batch leaves the session exactly as it was.
                self.cull = Some(session);
                self.persist_cull();
                Err(error)
            }
        }
    }

    fn apply_session(
        &mut self,
        session: &sortlens_cull::Session,
        mode: sortlens_cull::trash::Mode,
    ) -> Result<sortlens_cull::trash::Report> {
        let changed = session.changed();
        if changed.is_empty() {
            return Err(Error::Invalid("nothing is staged yet".into()));
        }
        let mode_id = mode.id();
        let mut candidates = Vec::new();
        let mut flags = Vec::new();
        let mut previous_flags = Vec::new();
        for (id, decision) in &changed {
            // Unknown ids are an error, not a silent skip: Apply must not quietly drop a decision.
            let photo = self.photo(*id).ok_or(Error::NoSuchPhoto(*id))?;
            previous_flags.push((*id, photo.flag));
            if let Some(flag) = decision.flag() {
                flags.push((*id, flag));
            }
        }
        for id in session.rejects() {
            let photo = self.photo(id).ok_or(Error::NoSuchPhoto(id))?;
            candidates.push(sortlens_cull::trash::Candidate {
                id,
                path: PathBuf::from(&photo.root).join(&photo.rel),
                root: PathBuf::from(&photo.root),
                size: photo.size,
                modified: photo.modified,
            });
        }

        let batch = sortlens_cull::trash::Batch { mode, candidates };
        if let Err(refusals) = sortlens_cull::trash::audit(&batch) {
            let first = refusals.first().map(sortlens_cull::trash::Refusal::message).unwrap_or_default();
            for refusal in &refusals {
                self.warnings.push(refusal.message());
            }
            return Err(Error::Invalid(format!(
                "{} file(s) failed the safety checks: {first}",
                refusals.len()
            )));
        }

        let report = sortlens_cull::trash::apply(&batch);
        let picks = flags.iter().filter(|(_, flag)| *flag == Flag::Pick).count();
        let mut ops = vec![Op::Flag { changes: flags }];
        for entry in &report.applied {
            if !matches!(entry.location, sortlens_cull::trash::Location::Kept) {
                ops.push(Op::Missing {
                    id: entry.id,
                    missing: true,
                });
            }
        }
        self.mutate(Op::Group { ops })?;

        for (_, message) in &report.failed {
            self.warnings.push(message.clone());
        }
        let touched: Vec<PhotoId> = report.applied.iter().map(|entry| entry.id).collect();
        self.sync_sidecars(&touched);

        let job = self.start_job(JobKind::Export, format!("Applied {} photo(s)", report.applied.len()));
        let status = if report.has_failures() {
            JobStatus::Failed
        } else {
            JobStatus::Done
        };
        self.finish_job(
            job,
            status,
            report.applied.len(),
            0,
            report.failed.len(),
            report.failed.first().map(|(_, message)| message.clone()),
        );

        let applied_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs() as i64)
            .unwrap_or_default();
        let batch = sortlens_cull::trash::DeletedBatch {
            id: self.next_batch,
            applied_at,
            mode: mode_id,
            items: report.applied.clone(),
            previous_flags,
            failed: report.failed.len(),
        };
        self.receipt = Some(sortlens_cull::trash::Receipt::of(&batch, picks));
        self.deleted.push(batch);
        self.next_batch += 1;
        self.persist_deleted();

        self.cull = None;
        self.persist_cull();
        Ok(report)
    }

    /// Counters for the cull action bar, zeroed when no session is running.
    pub fn cull_summary(&self) -> Value {
        match &self.cull {
            Some(session) => {
                let summary = session.summary();
                json!({
                    "total": summary.total,
                    "picks": summary.picks,
                    "rejects": summary.rejects,
                    "deferred": summary.deferred,
                    "unjudged": summary.unjudged,
                    "current": session.current().map(|id| id.0),
                    "cursor": session.cursor(),
                })
            }
            None => json!({ "total": 0, "picks": 0, "rejects": 0, "deferred": 0, "unjudged": 0, "current": null, "cursor": 0 }),
        }
    }

    /// Work that would be lost if the window closed now.
    ///
    /// A cull session's staged decisions are only persisted as a session file, so quitting mid-cull
    /// loses the queue rather than the decisions; this names what is in flight so the guard can say
    /// something specific instead of "are you sure?".
    pub fn unsaved(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        if self.pending.is_some() {
            reasons.push("An index scan is running; the files found so far are not saved yet.".to_string());
        }
        if let Some(session) = &self.cull {
            let summary = session.summary();
            let staged = summary.changed();
            if staged > 0 {
                reasons.push(format!(
                    "{staged} staged cull decision(s) have not been applied. The session is kept for next time."
                ));
            }
        }
        if self.pending_hash.is_some() {
            reasons.push("A similarity pass is running; hashes finished so far are kept.".to_string());
        }
        reasons
    }

    /// Whether closing the window would drop something worth asking about.
    pub fn has_unsaved(&self) -> bool {
        !self.unsaved().is_empty()
    }

    /// What the last finished Apply did, for the receipt dialog.
    pub fn receipt(&self) -> Option<&ApplyReceipt> {
        self.receipt.as_ref()
    }

    /// Dismiss the receipt, so it is not shown again.
    pub fn clear_receipt(&mut self) {
        self.receipt = None;
    }

    /// Whether every photo in the running session has been judged.
    pub fn cull_complete(&self) -> bool {
        self.cull.as_ref().is_some_and(|session| session.summary().is_complete())
    }

    /// The running cull session, if any.
    pub fn cull(&self) -> Option<&sortlens_cull::Session> {
        self.cull.as_ref()
    }

    /// Stage a decision for the current photo and advance.
    pub fn cull_decide(&mut self, decision: sortlens_cull::Decision) -> Option<PhotoId> {
        let next = self.cull.as_mut().and_then(|session| session.decide(decision));
        self.persist_cull();
        next
    }

    /// Stage a decision for one photo without moving the cursor.
    pub fn cull_stage(&mut self, id: PhotoId, decision: sortlens_cull::Decision) -> bool {
        let changed = self
            .cull
            .as_mut()
            .is_some_and(|session| session.stage(id, decision) != sortlens_cull::Staged::Unchanged);
        self.persist_cull();
        changed
    }

    /// Jump to a photo in the session's queue.
    pub fn cull_jump(&mut self, id: PhotoId) -> bool {
        let jumped = self.cull.as_mut().is_some_and(|session| session.jump_to(id));
        self.persist_cull();
        jumped
    }

    /// Jump to the first photo without a decision, if there is one.
    pub fn cull_next_unjudged(&mut self) -> Option<PhotoId> {
        let next = self.cull.as_mut().and_then(sortlens_cull::Session::next_unjudged);
        self.persist_cull();
        next
    }

    /// Move back one photo in the session.
    pub fn cull_back(&mut self) -> Option<PhotoId> {
        let next = self.cull.as_mut().and_then(sortlens_cull::Session::back);
        self.persist_cull();
        next
    }

    /// Drop the session. Nothing was written, so nothing needs undoing.
    pub fn cull_clear(&mut self) {
        self.cull = None;
        self.persist_cull();
    }

    /// Whether review changes are written to XMP sidecars.
    pub fn write_sidecars(&self) -> bool {
        self.write_sidecars
    }

    /// Turn sidecar writing on or off. The setting is applied to the next change, not retroactively.
    pub fn set_write_sidecars(&mut self, on: bool) {
        self.write_sidecars = on;
    }

    /// Rewrite the sidecar of every listed photo from the catalog.
    ///
    /// Failures are collected on the job rather than aborting the batch: a photo with a read-only
    /// sidecar must still be rateable, per `docs/BEHAVIOUR.md` §4.
    pub fn sync_sidecars(&mut self, ids: &[PhotoId]) {
        if !self.write_sidecars || ids.is_empty() {
            return;
        }
        let job = self.start_job(JobKind::Sidecar, format!("{} photo(s)", ids.len()));
        let mut written = 0;
        let mut failures = Vec::new();
        for id in ids {
            let Some(photo) = self.photo(*id).cloned() else {
                continue;
            };
            let image = PathBuf::from(&photo.root).join(&photo.rel);
            let sidecar = sortlens_sidecar::Sidecar::new(photo.rating, photo.flag, photo.label, photo.tags.clone());
            match sortlens_sidecar::write(&image, &sidecar) {
                Ok(_) => written += 1,
                Err(error) => {
                    let message = format!("{}: {error}", photo.rel);
                    self.warnings.push(message.clone());
                    failures.push(message);
                }
            }
        }
        let status = if failures.is_empty() { JobStatus::Done } else { JobStatus::Failed };
        self.finish_job(job, status, written, 0, failures.len(), failures.first().cloned());
    }

    /// Set a colour label on a selection.
    pub fn label(&mut self, ids: &[PhotoId], label: Option<ColorLabel>) -> Result<usize> {
        let changed = self.mutate(Op::Label {
            changes: ids.iter().map(|id| (*id, label)).collect(),
        })?;
        self.sync_sidecars(ids);
        Ok(changed)
    }

    /// Undo the last catalog operation.
    pub fn undo(&mut self) -> Result<bool> {
        match self.catalog.as_mut() {
            Some(catalog) => catalog.undo(),
            None => Ok(false),
        }
    }

    /// Remove a root and its photos.
    pub fn remove_root(&mut self, root: &str) -> Result<usize> {
        match self.catalog.as_mut() {
            Some(catalog) => catalog.remove_root(root),
            None => Err(Error::Invalid("no library is open".into())),
        }
    }

    /// Flush the catalog to disk. Called on exit.
    pub fn close(&mut self) -> Result<()> {
        match self.catalog.as_mut() {
            Some(catalog) => catalog.close(),
            None => Ok(()),
        }
    }

    fn mutate(&mut self, op: Op) -> Result<usize> {
        let catalog = self.catalog.as_mut().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        let count = match &op {
            Op::Rate { changes } => changes.len(),
            Op::Flag { changes } => changes.len(),
            Op::Label { changes } => changes.len(),
            Op::Tagged { changes } => changes.len(),
            Op::Group { ops } => ops.len(),
            _ => 0,
        };
        catalog.commit(op)?;
        Ok(count)
    }

    /// Run one command by id. The JSON face of the typed methods above.
    pub fn run(&mut self, command: &str, args: &Value) -> Result<Value> {
        self.last_effect = Effect::None;
        let spec = *self.registry.expect(command)?;
        if let Some(_milestone) = spec.milestone() {
            return Err(Error::NotYetAvailable(match command {
                "library.import" => "card import arrives in M2",
                "photo.keyword" => "keywords arrive in M3",
                "collection.new" | "collection.addTo" => "collections arrive in M5",
                "view.detail" => "the detail view arrives in M4",
                "view.places" => "the map arrives in M5",
                "library.findMissing" => "missing-file repair arrives in M7",
                _ => "this command is planned",
            }));
        }
        match command {
            "library.addFolder" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("path is required".into()))?;
                self.begin_scan(path)?;
                Ok(json!({ "scanning": path }))
            }
            "library.rescan" => {
                let root = match args.get("path").and_then(Value::as_str) {
                    Some(path) => path.to_string(),
                    None => self
                        .roots()
                        .first()
                        .cloned()
                        .ok_or_else(|| Error::Invalid("no folder to rescan".into()))?,
                };
                self.begin_scan(root.clone())?;
                Ok(json!({ "scanning": root }))
            }
            "library.removeRoot" => {
                let root = args
                    .get("root")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("root is required".into()))?;
                let removed = self.remove_root(root)?;
                Ok(json!({ "removed": removed }))
            }
            "library.page" => {
                let filter: Filter = args
                    .get("filter")
                    .map(|value| serde_json::from_value(value.clone()))
                    .transpose()
                    .map_err(|error| Error::Invalid(error.to_string()))?
                    .unwrap_or_default();
                let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
                let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(128) as usize;
                let page = self.page(&filter, Sort::Natural, offset, limit);
                Ok(json!({ "total": page.total, "offset": page.offset, "photos": page.photos }))
            }
            "photo.rate" => {
                let ids = ids_from(args)?;
                let stars = args.get("stars").and_then(Value::as_u64).unwrap_or(0) as u8;
                Ok(json!({ "changed": self.rate(&ids, stars)? }))
            }
            "photo.flag" => {
                let ids = ids_from(args)?;
                let flag = match args.get("flag").and_then(Value::as_str).unwrap_or("none") {
                    "pick" => Flag::Pick,
                    "reject" => Flag::Reject,
                    _ => Flag::None,
                };
                Ok(json!({ "changed": self.flag(&ids, flag)? }))
            }
            "photo.label" => {
                let ids = ids_from(args)?;
                let label = args
                    .get("label")
                    .and_then(Value::as_str)
                    .and_then(|name| ColorLabel::ALL.into_iter().find(|label| label.id() == name));
                Ok(json!({ "changed": self.label(&ids, label)? }))
            }
            "photo.keyword" => {
                let ids = ids_from(args)?;
                let tag = args
                    .get("tag")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("tag is required".into()))?;
                let add = args.get("add").and_then(Value::as_bool).unwrap_or(true);
                Ok(json!({ "changed": self.set_tag(&ids, tag, add)? }))
            }
            "cull.start" => {
                let ids = ids_from(args)?;
                Ok(json!({ "total": self.begin_cull(&ids)? }))
            }
            "cull.decide" => {
                let name = args
                    .get("decision")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("decision is required".into()))?;
                let decision = sortlens_cull::Decision::parse(name).ok_or_else(|| Error::Invalid(format!("unknown decision: {name}")))?;
                let next = self.cull_decide(decision);
                Ok(json!({ "next": next.map(|id| id.0), "summary": self.cull_summary() }))
            }
            "cull.back" => Ok(json!({ "current": self.cull_back().map(|id| id.0) })),
            "cull.clear" => {
                self.cull_clear();
                Ok(json!({ "cleared": true }))
            }
            "cull.summary" => Ok(self.cull_summary()),
            "collection.new" => {
                let name = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a collection needs a name".into()))?;
                Ok(json!({ "id": self.create_collection(name)?.0 }))
            }
            "collection.addTo" => {
                let ids = ids_from(args)?;
                let id = sortlens_core::CollectionId(collection_id_from(args)?);
                Ok(json!({ "changed": self.add_to_collection(&ids, id)? }))
            }
            "collection.colour" => {
                let id = sortlens_core::CollectionId(collection_id_from(args)?);
                let colour = match args.get("colour").and_then(Value::as_str) {
                    None | Some("") | Some("none") => None,
                    Some(name) => Some(
                        ColorLabel::ALL
                            .into_iter()
                            .find(|label| label.id() == name)
                            .ok_or_else(|| Error::Invalid(format!("{name} is not a colour label")))?,
                    ),
                };
                self.set_collection_colour(id, colour)?;
                Ok(json!({ "colour": colour.map(|label| label.id()) }))
            }
            "collection.favourite" => {
                let id = sortlens_core::CollectionId(collection_id_from(args)?);
                let favourite = args.get("favourite").and_then(Value::as_bool).unwrap_or(true);
                self.set_collection_favourite(id, favourite)?;
                Ok(json!({ "favourite": favourite }))
            }
            "collection.sort" => {
                let id = sortlens_core::CollectionId(collection_id_from(args)?);
                let sort = Sort::parse(args.get("sort").and_then(Value::as_str).unwrap_or_default());
                self.set_collection_sort(id, sort)?;
                Ok(json!({ "sort": sort.id() }))
            }
            "tag.rename" => {
                let from = args
                    .get("from")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a keyword to rename is required".into()))?;
                let to = args
                    .get("to")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a new name is required".into()))?;
                Ok(json!({ "changed": self.rename_tag(from, to)? }))
            }
            "collection.search" => {
                let id = sortlens_core::CollectionId(collection_id_from(args)?);
                let search = args.get("search").and_then(Value::as_str).unwrap_or_default();
                self.set_collection_search(id, search)?;
                Ok(json!({ "search": self.collections().into_iter().find(|item| item.id == id).and_then(|item| item.search) }))
            }
            "collection.removeFrom" => {
                let ids = ids_from(args)?;
                let id = sortlens_core::CollectionId(collection_id_from(args)?);
                Ok(json!({ "changed": self.remove_from_collection(&ids, id)? }))
            }
            "collection.rename" => {
                let id = sortlens_core::CollectionId(collection_id_from(args)?);
                let name = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a collection needs a name".into()))?;
                self.rename_collection(id, name)?;
                Ok(json!({ "renamed": true }))
            }
            "collection.delete" => {
                let id = sortlens_core::CollectionId(collection_id_from(args)?);
                self.delete_collection(id)?;
                Ok(json!({ "deleted": true }))
            }
            "similar.start" => {
                let threshold = args
                    .get("threshold")
                    .and_then(Value::as_u64)
                    .unwrap_or(u64::from(SIMILAR_THRESHOLD)) as u32;
                let queued = self.begin_similar(threshold)?;
                Ok(json!({ "queued": queued }))
            }
            "similar.groups" => {
                let threshold = args
                    .get("threshold")
                    .and_then(Value::as_u64)
                    .unwrap_or(u64::from(SIMILAR_THRESHOLD)) as u32;
                let _ = self.poll_similar(threshold);
                let groups: Vec<Value> = self
                    .similar_groups()
                    .iter()
                    .map(|group| {
                        json!({
                            "best": group.best.0,
                            "size": group.len(),
                            "spread": group.spread,
                            "members": group.members.iter().map(|member| json!({
                                "id": member.id.0,
                                "pixels": member.pixels,
                                "rating": member.rating,
                            })).collect::<Vec<_>>(),
                        })
                    })
                    .collect();
                Ok(json!({
                    "groups": groups,
                    "hashed": self.hashes.len(),
                    "pending": self.hash_progress(),
                }))
            }
            "view.insights" => {
                let insights = self.insights();
                Ok(json!({
                "total": insights.stats.total,
                "bytes": insights.stats.bytes,
                "rated": insights.stats.rated,
                "picks": insights.stats.picks,
                "rejects": insights.stats.rejects,
                "labelled": insights.stats.labelled,
                "tagged": insights.stats.tagged,
                "missing": insights.stats.missing,
                "undated": insights.undated,
                "unknownCamera": insights.unknown_camera,
                "byRating": insights.by_rating,
                "byLabel": insights.by_label,
                "byKind": insights.by_kind,
                "cameras": rows(&insights.cameras),
                "lenses": rows(&insights.lenses),
                "keywords": rows(&insights.keywords),
                "years": insights.years.iter().map(|year| json!({ "year": year.year, "count": year.count })).collect::<Vec<_>>(),
                }))
            }
            "library.subfolders" => {
                let root = args
                    .get("root")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a root is required".into()))?;
                let prefix = args.get("prefix").and_then(Value::as_str).unwrap_or("");
                let folders: Vec<Value> = self
                    .subfolders(root, prefix)
                    .into_iter()
                    .map(|(name, count)| json!({ "name": name, "count": count }))
                    .collect();
                Ok(json!({ "folders": folders }))
            }
            "library.health" => {
                let health = self.check_health()?;
                Ok(json!({
                    "checked": health.checked,
                    "present": health.present,
                    "missing": health.missing,
                    "changed": health.changed,
                }))
            }
            "library.relocate" => Ok(json!({ "relocated": self.relocate_missing()? })),
            "library.history" => {
                let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(HISTORY_LIMIT as u64) as usize;
                let entries: Vec<Value> = self
                    .history(limit)?
                    .into_iter()
                    .map(|entry| {
                        json!({
                            "seq": entry.seq,
                            "at": entry.at,
                            "kind": entry.kind,
                            "summary": entry.summary,
                            "photos": entry.photos,
                        })
                    })
                    .collect();
                Ok(json!({ "entries": entries }))
            }
            "library.exportHistory" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a destination path is required".into()))?;
                Ok(json!({ "lines": self.export_history(Path::new(path))? }))
            }
            "library.audit" => {
                let audit = self.audit()?;
                Ok(json!({
                    "clean": audit.is_clean(),
                    "photos": audit.photos,
                    "collections": audit.collections,
                    "seq": audit.final_seq,
                    "snapshotSeq": audit.snapshot_seq,
                    "replayed": audit.replayed,
                    "skipped": audit.skipped,
                    "logBytes": audit.log_bytes,
                    "tornTail": audit.torn_tail,
                    "headline": audit.headline(),
                    "detail": audit.detail(),
                    "notes": audit.notes,
                }))
            }
            "library.compact" => Ok(json!({ "reclaimed": self.compact()? })),
            "library.remapRoot" => {
                let from = args
                    .get("from")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("the current folder is required".into()))?;
                let to = args
                    .get("to")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("the new folder is required".into()))?;
                Ok(json!({ "remapped": self.remap_root(from, to)? }))
            }
            "library.removeMissing" => Ok(json!({ "removed": self.remove_missing()? })),
            "library.deleted" => {
                let batches: Vec<Value> = self
                    .deleted
                    .iter()
                    .rev()
                    .map(|batch| {
                        json!({
                            "id": batch.id,
                            "appliedAt": batch.applied_at,
                            "mode": batch.mode,
                            "count": batch.items.len(),
                            "failed": batch.failed,
                            "summary": batch.summary(),
                        })
                    })
                    .collect();
                Ok(json!({ "batches": batches }))
            }
            "library.backup" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a destination path is required".into()))?;
                Ok(json!({ "files": self.export_backup(Path::new(path))? }))
            }
            "library.restoreBackup" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a backup path is required".into()))?;
                let dest = args
                    .get("dest")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a destination folder is required".into()))?;
                Ok(json!({ "files": Self::import_backup(Path::new(path), Path::new(dest))? }))
            }
            "library.forgetBatch" => {
                let id = args
                    .get("id")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| Error::Invalid("a batch id is required".into()))?;
                Ok(json!({ "forgotten": self.forget_batch(id)? }))
            }
            "library.clearDeleted" => {
                let (batches, files) = self.forget_all_batches();
                Ok(json!({ "batches": batches, "files": files }))
            }
            "library.restoreBatch" => {
                let id = args
                    .get("id")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| Error::Invalid("a batch id is required".into()))?;
                Ok(json!({ "restored": self.restore_batch(id)? }))
            }
            "cull.apply" => {
                let mode = match args.get("mode").and_then(Value::as_str).unwrap_or("trash") {
                    "trash" => sortlens_cull::trash::Mode::Trash,
                    "tag" => sortlens_cull::trash::Mode::TagOnly,
                    "move" => {
                        let target = args
                            .get("target")
                            .and_then(Value::as_str)
                            .ok_or_else(|| Error::Invalid("move needs a target folder".into()))?;
                        sortlens_cull::trash::Mode::Move(PathBuf::from(target))
                    }
                    other => return Err(Error::Invalid(format!("unknown delete mode: {other}"))),
                };
                let report = self.cull_apply(mode)?;
                Ok(json!({
                    "applied": report.applied.len(),
                    "failed": report.failed.len(),
                }))
            }
            "edit.undo" => Ok(json!({ "undone": self.undo()? })),
            "library.stats" => Ok(serde_json::to_value(self.stats()).unwrap_or(Value::Null)),
            "library.jobs" => {
                let jobs: Vec<Value> = self
                    .jobs()
                    .iter()
                    .map(|job| {
                        json!({
                            "id": job.id,
                            "kind": job.kind.label(),
                            "label": job.label,
                            "status": job.status.label(),
                            "files": job.files,
                            "dirs": job.dirs,
                            "detail": job.detail(),
                            "elapsedSeconds": job.elapsed().as_secs_f64(),
                        })
                    })
                    .collect();
                Ok(json!({ "jobs": jobs }))
            }
            "app.about" => Ok(json!({
                "name": "Sortlens",
                "version": sortlens_core::VERSION,
                "engine": VERSION,
                "commands": self.registry.all().len(),
                "readyCommands": self.registry.ready_count(),
                "libraryOpen": self.has_library(),
            })),
            "app.supportReport" => Ok(json!({
                "sortlens": sortlens_core::VERSION,
                "engine": VERSION,
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "commands": self.registry.all().len(),
                "planned": self.registry.all().len() - self.registry.ready_count(),
                "photos": self.stats().total,
                "roots": self.roots(),
                "warnings": self.warnings,
                "reason": args.get("reason").and_then(Value::as_str).unwrap_or("user request"),
            })),
            "app.settings" => {
                self.last_effect = Effect::OpenSettings;
                Ok(json!({ "opened": "settings" }))
            }
            "app.quit" => {
                self.last_effect = Effect::Quit;
                Ok(json!({ "quitting": true }))
            }
            other => Err(Error::Invalid(format!("command has no handler: {other}"))),
        }
    }
}

/// What a health check found.
#[derive(Debug, Clone, Copy, Default)]
pub struct Health {
    /// Photos examined.
    pub checked: usize,
    /// Photos whose file is there.
    pub present: usize,
    /// Photos whose file is gone.
    pub missing: usize,
    /// Photos whose missing state changed during this check.
    pub changed: usize,
}

/// Keywords are trimmed, lowercased and length-bounded so the vocabulary stays usable.
/// Serialise a breakdown for the JSON face.
fn rows(rows: &[InsightRow]) -> Vec<Value> {
    rows.iter().map(|row| json!({ "name": row.name, "count": row.count })).collect()
}

fn normalize_tag(tag: &str) -> Result<String> {
    let tag = tag.trim().trim_start_matches('#').trim().to_lowercase();
    if tag.is_empty() {
        return Err(Error::Invalid("a keyword cannot be empty".into()));
    }
    if tag.chars().count() > 64 {
        return Err(Error::Invalid("a keyword cannot be longer than 64 characters".into()));
    }
    Ok(tag)
}

/// Replace a file atomically: temp file in the same directory, then rename.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, bytes).map_err(|error| Error::io(temp.display().to_string(), error))?;
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        Error::io(path.display().to_string(), error)
    })
}

/// Last path component, for job labels.
fn folder_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

/// A collection id from the command arguments.
fn collection_id_from(args: &Value) -> Result<u32> {
    args.get("collection")
        .or_else(|| args.get("id"))
        .and_then(Value::as_u64)
        .map(|id| id as u32)
        .ok_or_else(|| Error::Invalid("a collection id is required".into()))
}

fn ids_from(args: &Value) -> Result<Vec<PhotoId>> {
    let ids = args
        .get("ids")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Invalid("ids are required".into()))?;
    ids.iter()
        .map(|value| {
            value
                .as_u64()
                .map(PhotoId)
                .ok_or_else(|| Error::Invalid("ids must be numbers".into()))
        })
        .collect()
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How many recent changes the change viewer and the support report cover.
pub const HISTORY_LIMIT: usize = 200;

/// How many spread-out photos a root remap checks before repointing the whole root.
pub const REMAP_SAMPLE: usize = 12;

/// Up to `count` indices spread evenly through `len`, so a sample covers the whole root rather than
/// its first folder.
fn sample_indices(len: usize, count: usize) -> Vec<usize> {
    if len == 0 || count == 0 {
        return Vec::new();
    }
    if len <= count {
        return (0..len).collect();
    }
    (0..count).map(|slot| slot * len / count).collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sortlens-engine-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn library(name: &str) -> (Temp, Engine) {
        let temp = Temp::new(name);
        let engine = Engine::open(&temp.0).expect("open");
        (temp, engine)
    }

    fn wait_for_scan(engine: &mut Engine) -> ScanFinished {
        for _ in 0..600 {
            if let Some(done) = engine.poll() {
                return done;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("scan did not finish");
    }

    #[test]
    fn adding_a_folder_indexes_it() {
        let folder = Temp::new("folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"jpeg-ish").expect("write");
        std::fs::write(folder.0.join("b.cr3"), b"raw-ish").expect("write");
        std::fs::write(folder.0.join("notes.txt"), b"text").expect("write");

        let (_lib, mut engine) = library("add");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        let done = wait_for_scan(&mut engine);
        assert_eq!(done.added, 2);
        assert!(done.errors.is_empty());
        assert_eq!(engine.stats().total, 2);
        assert_eq!(engine.roots().len(), 1);
    }

    #[test]
    fn rating_flagging_and_labelling_go_through_one_op_each() {
        let folder = Temp::new("rate-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        let (_lib, mut engine) = library("rate");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);

        let id = engine.page(&Filter::default(), Sort::Natural, 0, 10).photos[0].id;
        assert_eq!(engine.rate(&[id], 4).expect("rate"), 1);
        assert_eq!(engine.flag(&[id], Flag::Pick).expect("flag"), 1);
        assert_eq!(engine.label(&[id], Some(ColorLabel::Red)).expect("label"), 1);

        let photo = engine.photo(id).expect("photo");
        assert_eq!((photo.rating, photo.flag, photo.label), (4, Flag::Pick, Some(ColorLabel::Red)));

        assert!(engine.undo().expect("undo"));
        assert!(engine.photo(id).expect("photo").label.is_none());
    }

    #[test]
    fn a_second_scan_does_not_duplicate_photos() {
        let folder = Temp::new("rescan-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        let (_lib, mut engine) = library("rescan");
        let root = folder.0.to_string_lossy().to_string();

        engine.begin_scan(root.clone()).expect("first");
        assert_eq!(wait_for_scan(&mut engine).added, 1);
        engine.begin_scan(root).expect("second");
        let done = wait_for_scan(&mut engine);
        assert_eq!(done.added, 0);
        assert_eq!(done.updated, 0);
        assert_eq!(engine.stats().total, 1);
    }

    #[test]
    fn commands_and_typed_methods_agree() {
        let (_lib, mut engine) = library("commands");
        let about = engine.run("app.about", &json!({})).expect("about");
        assert_eq!(about["name"], "Sortlens");

        let error = engine.run("library.import", &json!({})).expect_err("planned");
        assert!(error.to_string().contains("M2"), "{error}");

        let error = engine.run("library.addFolder", &json!({})).expect_err("path required");
        assert!(matches!(error, Error::Invalid(_)));
    }

    #[test]
    fn a_scan_creates_a_job_that_finishes_done() {
        let folder = Temp::new("job-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        let (_lib, mut engine) = library("job");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");

        let running = engine.jobs();
        assert_eq!(running.len(), 1);
        assert!(running[0].status.is_running());

        wait_for_scan(&mut engine);
        let jobs = engine.jobs();
        assert_eq!(jobs[0].status, JobStatus::Done);
        assert_eq!(jobs[0].files, 1);
        assert!(jobs[0].finished.is_some());
    }

    #[test]
    fn a_cull_session_stages_without_touching_the_catalog() {
        let folder = Temp::new("cull-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        let (_lib, mut engine) = library("cull");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();

        assert_eq!(engine.begin_cull(&ids).expect("begin"), 2);
        assert_eq!(engine.cull_decide(sortlens_cull::Decision::Reject), Some(ids[1]));
        assert_eq!(engine.cull_decide(sortlens_cull::Decision::Pick), None);
        assert_eq!(engine.cull_summary()["rejects"], 1);
        assert_eq!(engine.cull_summary()["picks"], 1);
        assert!(engine.cull().expect("session").summary().is_complete());

        // Staged only: the catalog still says neither photo carries a flag.
        assert_eq!(engine.photo(ids[0]).expect("photo").flag, Flag::None);
        assert_eq!(engine.photo(ids[1]).expect("photo").flag, Flag::None);

        assert_eq!(engine.cull_back(), Some(ids[1]));
        engine.cull_clear();
        assert!(engine.cull().is_none());
    }

    #[test]
    fn a_cull_session_survives_a_restart() {
        let folder = Temp::new("cull-resume-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        let root = folder.0.to_string_lossy().to_string();
        let (library_dir, _) = library("cull-resume");
        let ids = {
            let mut engine = Engine::open(&library_dir.0).expect("open");
            engine.begin_scan(root.clone()).expect("scan");
            wait_for_scan(&mut engine);
            let ids: Vec<PhotoId> = engine
                .page(&Filter::default(), Sort::Natural, 0, 10)
                .photos
                .iter()
                .map(|photo| photo.id)
                .collect();
            engine.begin_cull(&ids).expect("begin");
            engine.cull_decide(sortlens_cull::Decision::Reject);
            let _ = engine.close();
            ids
        };

        let mut engine = Engine::open(&library_dir.0).expect("reopen");
        let session = engine.cull().expect("session restored");
        assert_eq!(session.current(), Some(ids[1]), "resumes on the next photo");
        assert_eq!(session.decision(ids[0]), sortlens_cull::Decision::Reject);
        assert_eq!(engine.cull_summary()["rejects"], 1);

        engine.cull_clear();
        assert!(engine.cull().is_none());
        let mut again = Engine::open(&library_dir.0).expect("reopen again");
        assert!(again.cull().is_none(), "ending the session removes the file");
        let _ = again.close();
    }

    #[test]
    fn a_saved_session_drops_photos_that_are_gone() {
        let folder = Temp::new("cull-prune-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        let root = folder.0.to_string_lossy().to_string();
        let (library_dir, _) = library("cull-prune");
        let kept = {
            let mut engine = Engine::open(&library_dir.0).expect("open");
            engine.begin_scan(root.clone()).expect("scan");
            wait_for_scan(&mut engine);
            let ids: Vec<PhotoId> = engine
                .page(&Filter::default(), Sort::Natural, 0, 10)
                .photos
                .iter()
                .map(|photo| photo.id)
                .collect();
            engine.begin_cull(&ids).expect("begin");
            engine.remove_root(&root).expect("remove root");
            ids[1]
        };
        let _ = kept;
        let engine = Engine::open(&library_dir.0).expect("reopen");
        assert!(engine.cull().is_none(), "a session with no surviving photos is discarded");
    }

    #[test]
    fn applying_tag_only_records_the_decisions_and_undos_as_one_step() {
        let folder = Temp::new("apply-tag-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        let (_lib, mut engine) = library("apply-tag");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        engine.begin_cull(&ids).expect("begin");
        engine.cull_decide(sortlens_cull::Decision::Reject);
        engine.cull_decide(sortlens_cull::Decision::Pick);

        let report = engine.cull_apply(sortlens_cull::trash::Mode::TagOnly).expect("apply");
        assert_eq!(report.applied.len(), 1);
        assert_eq!(engine.photo(ids[0]).expect("photo").flag, Flag::Reject);
        assert_eq!(engine.photo(ids[1]).expect("photo").flag, Flag::Pick);
        assert!(engine.cull().is_none(), "the session ends with the batch");
        assert!(folder.0.join("a.jpg").exists(), "tag-only moves nothing");

        assert!(engine.undo().expect("undo"));
        assert_eq!(engine.photo(ids[0]).expect("photo").flag, Flag::None);
        assert_eq!(engine.photo(ids[1]).expect("photo").flag, Flag::None);
    }

    #[test]
    fn renaming_a_keyword_through_the_engine_touches_every_photo() {
        let folder = Temp::new("rename-tag-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"b").expect("write");
        let (_lib, mut engine) = library("rename-tag");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        engine.set_tag(&ids, "Beach", true).expect("tag");

        let value = engine
            .run("tag.rename", &json!({ "from": "BEACH", "to": "#shore" }))
            .expect("rename");
        assert_eq!(value["changed"], 2, "names are normalised before matching");
        assert_eq!(engine.tag_vocabulary(), vec![("shore".to_string(), 2)]);
        assert!(engine.photo(ids[0]).expect("photo").has_tag("shore"));

        // Renaming onto an existing keyword merges the two.
        engine.set_tag(&ids[..1], "sand", true).expect("tag");
        engine.run("tag.rename", &json!({ "from": "sand", "to": "shore" })).expect("merge");
        assert_eq!(engine.photo(ids[0]).expect("photo").tags, vec!["shore".to_string()]);

        assert!(engine.run("tag.rename", &json!({ "from": "shore" })).is_err());
    }

    #[test]
    fn a_collection_remembers_its_search() {
        let (_lib, mut engine) = library("collection-search");
        let id = engine.create_collection("Trip").expect("create");
        let value = engine
            .run("collection.search", &json!({ "id": id.0, "search": " beach " }))
            .expect("search");
        assert_eq!(value["search"], "beach");
        let cleared = engine
            .run("collection.search", &json!({ "id": id.0, "search": "" }))
            .expect("clear");
        assert!(cleared["search"].is_null(), "an empty box forgets the search");
    }

    #[test]
    fn a_collection_remembers_the_sort_it_was_browsed_with() {
        let (_lib, mut engine) = library("collection-sort");
        let id = engine.create_collection("Trip").expect("create");
        assert!(engine.collections()[0].sort.is_none());

        let value = engine
            .run("collection.sort", &json!({ "id": id.0, "sort": "largest" }))
            .expect("sort");
        assert_eq!(value["sort"], "largest");
        let remembered = engine.collections()[0].sort.clone().expect("remembered");
        assert_eq!(Sort::parse(&remembered), Sort::Largest);

        engine
            .run("collection.sort", &json!({ "id": id.0, "sort": "nonsense" }))
            .expect("unknown sorts fall back");
        assert_eq!(engine.collections()[0].sort.as_deref(), Some(Sort::Natural.id()));
    }

    #[test]
    fn collections_carry_a_colour_and_a_favourite_flag() {
        let (_lib, mut engine) = library("collection-flags");
        let id = engine.create_collection("Trip").expect("create");
        assert!(engine.collections()[0].colour.is_none());

        let value = engine
            .run("collection.colour", &json!({ "id": id.0, "colour": "blue" }))
            .expect("colour");
        assert_eq!(value["colour"], "blue");
        engine
            .run("collection.favourite", &json!({ "id": id.0, "favourite": true }))
            .expect("favourite");

        let listed = engine.collections();
        assert_eq!(listed[0].colour, Some(ColorLabel::Blue));
        assert!(listed[0].favourite);

        let bad = engine.run("collection.colour", &json!({ "id": id.0, "colour": "chartreuse" }));
        assert!(bad.is_err(), "an unknown colour is rejected");
        assert_eq!(engine.collections()[0].colour, Some(ColorLabel::Blue), "unchanged");

        engine
            .run("collection.colour", &json!({ "id": id.0, "colour": "none" }))
            .expect("clear");
        assert!(engine.collections()[0].colour.is_none());
    }

    #[test]
    fn recent_changes_and_the_report_name_what_happened() {
        let folder = Temp::new("history-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        let (lib, mut engine) = library("lib-history");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
        engine.rate(&[id], 5).expect("rate");
        engine.set_tag(&[id], "beach", true).expect("tag");

        let value = engine.run("library.history", &json!({ "limit": 5 })).expect("history");
        let entries = value["entries"].as_array().expect("entries");
        // The scan writes a root record plus one per photo; the limit caps the list, not the log.
        assert_eq!(entries.len(), 4, "one root, one photo, a rating, a keyword");
        assert_eq!(entries[0]["summary"], "Keyworded 1 photo", "newest first");
        assert_eq!(entries[1]["summary"], "Rated 1 photo");
        assert!(entries[0]["at"].as_i64().expect("time") > 0);
        assert_eq!(entries[0]["kind"], "tagged");

        let report = lib.0.join("change-report.txt");
        let lines = engine.export_history(&report).expect("export");
        assert!(lines > 3);
        let text = std::fs::read_to_string(&report).expect("read");
        assert!(text.contains("Sortlens change report"));
        assert!(text.contains(&format!("app {VERSION}")));
        assert!(text.contains("Keyworded 1 photo"));
        assert!(text.contains("photo contents never do"), "the report says what it holds");
    }

    #[test]
    fn compaction_refuses_a_library_it_could_not_read() {
        let folder = Temp::new("bad-snapshot-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        let (lib, mut engine) = library("lib-bad-snapshot");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        // Compaction writes the snapshot this test then damages.
        engine.compact().expect("compact");
        engine.close().expect("close");

        // A half-written snapshot is what a crash during a snapshot leaves.
        let path = lib.0.join(sortlens_catalog::SNAPSHOT_NAME);
        let text = std::fs::read_to_string(&path).expect("read");
        std::fs::write(&path, &text[..text.len() / 2]).expect("truncate");

        let mut engine = Engine::open(&lib.0).expect("reopen");
        let peek = engine.audit().expect("audit");
        assert!(!peek.is_clean(), "the audit saw nothing wrong: {peek:?}");
        let error = engine.compact().expect_err("refused");
        assert!(error.to_string().contains("refusing to compact"), "{error}");
        assert!(
            std::fs::read_to_string(&path).expect("still there").len() < text.len(),
            "the damaged snapshot was left alone"
        );
        assert_eq!(engine.stats().total, 0, "the log was empty, so there is nothing to rebuild from");
    }

    #[test]
    fn the_catalog_can_be_audited_and_compacted() {
        let folder = Temp::new("audit-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        let (_lib, mut engine) = library("lib-audit");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
        engine.rate(&[id], 4).expect("rate");

        let audit = engine.run("library.audit", &json!({})).expect("audit");
        assert_eq!(audit["clean"], true);
        assert_eq!(audit["photos"], 1);
        assert!(audit["headline"].as_str().expect("headline").contains("Clean"));

        let compact = engine.run("library.compact", &json!({})).expect("compact");
        assert!(compact["reclaimed"].as_u64().expect("bytes") > 0, "the log had records");

        let after = engine.run("library.audit", &json!({})).expect("audit again");
        assert_eq!(after["clean"], true);
        assert_eq!(after["replayed"], 0, "everything is in the snapshot");
        assert_eq!(engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].rating, 4);
    }

    #[test]
    fn a_moved_volume_is_repointed_in_one_undoable_step() {
        let folder = Temp::new("remap-old");
        std::fs::create_dir_all(folder.0.join("day one")).expect("dir");
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::write(folder.0.join("day one").join(name), name.as_bytes()).expect("write");
        }
        let (_lib, mut engine) = library("remap");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        assert_eq!(engine.stats().total, 3);

        // The volume is renamed, keeping the layout.
        let moved = Temp::new("remap-new");
        let new_root = moved.0.join("day one");
        std::fs::create_dir_all(moved.0.clone()).expect("dir");
        std::fs::rename(folder.0.join("day one"), &new_root).expect("rename");

        let value = engine
            .run(
                "library.remapRoot",
                &json!({ "from": folder.0.to_string_lossy(), "to": moved.0.to_string_lossy() }),
            )
            .expect("remap");
        assert_eq!(value["remapped"], 3);
        assert_eq!(engine.roots().len(), 1);
        assert_eq!(engine.roots()[0], moved.0.to_string_lossy());
        assert_eq!(engine.run("library.health", &json!({})).expect("health")["missing"], 0);

        // One undo puts the library back on the old folder, which is why remap is an op.
        engine.undo().expect("undo");
        assert_eq!(engine.roots()[0], folder.0.to_string_lossy());
    }

    #[test]
    fn a_remap_refuses_a_folder_that_does_not_hold_the_photos() {
        let folder = Temp::new("remap-oops");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"aaaa").expect("write");
        let (_lib, mut engine) = library("lib-remap-oops");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        assert_eq!(engine.stats().total, 1, "the photo indexed");

        let wrong = Temp::new("remap-empty");
        std::fs::create_dir_all(&wrong.0).expect("dir");
        let error = engine
            .run(
                "library.remapRoot",
                &json!({ "from": folder.0.to_string_lossy(), "to": wrong.0.to_string_lossy() }),
            )
            .expect_err("refused");
        let text = error.to_string();
        assert!(text.contains("not found under"), "{text}");
        assert_eq!(engine.roots()[0], folder.0.to_string_lossy(), "the root is unchanged");

        // A file at the right path but with the wrong size is refused too.
        std::fs::write(wrong.0.join("a.jpg"), b"different length").expect("write");
        assert!(
            engine
                .run(
                    "library.remapRoot",
                    &json!({ "from": folder.0.to_string_lossy(), "to": wrong.0.to_string_lossy() })
                )
                .is_err(),
            "a size mismatch is not the same photo"
        );
        assert!(
            engine
                .run("library.remapRoot", &json!({ "from": "/nothing", "to": wrong.0.to_string_lossy() }))
                .is_err(),
            "an unindexed root cannot be remapped"
        );
        assert_eq!(sample_indices(30, 12).len(), 12);
        assert_eq!(sample_indices(3, 12), vec![0, 1, 2]);
        assert!(sample_indices(0, 12).is_empty());
    }

    #[test]
    fn a_backup_round_trips_into_a_separate_library() {
        let folder = Temp::new("backup-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        let (lib, mut engine) = library("backup");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
        engine.rate(&[id], 5).expect("rate");
        engine.set_tag(&[id], "keeper", true).expect("tag");

        let archive = lib.0.join("library.backup");
        let files = engine.export_backup(&archive).expect("export");
        assert!(files >= 1);
        assert!(archive.exists());

        let dest = Temp::new("backup-dest");
        let written = Engine::import_backup(&archive, &dest.0).expect("import");
        assert!(written >= 1);

        let mut restored = Engine::open(&dest.0).expect("open restored");
        let photo = restored.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].clone();
        assert_eq!(photo.rating, 5, "the rating travelled");
        assert!(photo.has_tag("keeper"), "the keyword travelled");
        assert_eq!(restored.stats().total, 1);

        // Restoring never writes into the library that is still open.
        assert_eq!(engine.stats().total, 1);
        assert!(!lib.0.join(format!("{}.restore", sortlens_catalog::LOG_NAME)).exists());
    }

    #[test]
    fn a_damaged_backup_is_refused() {
        let folder = Temp::new("damaged-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        let (lib, mut engine) = library("damaged");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);

        let archive = lib.0.join("broken.backup");
        engine.export_backup(&archive).expect("export");
        let text = std::fs::read_to_string(&archive).expect("read");
        std::fs::write(&archive, text.replace("\"schema\": 1", "\"schema\": 99")).expect("tamper");
        let dest = Temp::new("damaged-dest");
        let error = Engine::import_backup(&archive, &dest.0).expect_err("refused");
        assert!(error.to_string().contains("newer Sortlens"), "{error}");
        assert!(!dest.0.join(sortlens_catalog::SNAPSHOT_NAME).exists(), "nothing written");
    }

    #[test]
    fn the_bin_can_be_emptied_without_touching_the_files() {
        let folder = Temp::new("bin-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"b").expect("write");
        let (_lib, mut engine) = library("bin");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();

        // Two tag-only batches, so both files stay on disk and can be inspected afterwards.
        for id in &ids {
            engine.begin_cull(&[*id]).expect("begin");
            engine.cull_decide(sortlens_cull::Decision::Reject);
            engine.cull_apply(sortlens_cull::trash::Mode::TagOnly).expect("apply");
        }
        assert_eq!(engine.deleted_batches().len(), 2);

        let value = engine
            .run("library.forgetBatch", &json!({ "id": engine.deleted_batches()[0].id }))
            .expect("forget");
        assert_eq!(value["forgotten"], 1);
        assert_eq!(engine.deleted_batches().len(), 1, "the other batch stays");

        let cleared = engine.run("library.clearDeleted", &json!({})).expect("clear");
        assert_eq!(cleared["batches"], 1);
        assert_eq!(cleared["files"], 1);
        assert!(engine.deleted_batches().is_empty());
        assert!(folder.0.join("a.jpg").exists(), "forgetting the record never deletes a file");
        assert!(folder.0.join("b.jpg").exists());
        assert_eq!(engine.stats().total, 2, "the photos are still in the library");

        assert!(
            engine.run("library.forgetBatch", &json!({ "id": 99 })).is_err(),
            "an unknown batch is an error, not a silent success"
        );
        let empty = engine.run("library.clearDeleted", &json!({})).expect("clear again");
        assert_eq!(empty["batches"], 0, "emptying an empty bin is harmless");
    }

    #[test]
    fn the_quit_guard_names_work_in_flight() {
        let folder = Temp::new("quit-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        let (_lib, mut engine) = library("quit");
        assert!(!engine.has_unsaved(), "a quiet library has nothing to lose");

        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        assert!(engine.has_unsaved(), "a running scan counts");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        engine.begin_cull(&[id]).expect("begin");
        assert!(!engine.has_unsaved(), "an empty session is nothing to lose");
        engine.cull_decide(sortlens_cull::Decision::Pick);
        let reasons = engine.unsaved();
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("staged cull"), "{}", reasons[0]);

        engine.cull_clear();
        assert!(!engine.has_unsaved(), "discarding the session clears the warning");
    }

    #[test]
    fn the_receipt_describes_the_batch_and_survives_until_dismissed() {
        let folder = Temp::new("receipt-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        let (_lib, mut engine) = library("receipt");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();

        engine.begin_cull(&ids).expect("begin");
        assert!(!engine.cull_complete(), "nothing is judged yet");
        engine.cull_decide(sortlens_cull::Decision::Pick);
        assert!(!engine.cull_complete());
        engine.cull_decide(sortlens_cull::Decision::Reject);
        assert!(engine.cull_complete(), "every photo has a decision now");

        engine.cull_apply(sortlens_cull::trash::Mode::TagOnly).expect("apply");
        let receipt = engine.receipt().expect("receipt").clone();
        assert_eq!(receipt.applied, 1, "one reject was disposed of");
        assert_eq!(receipt.mode, "tag");
        assert_eq!(receipt.picks, 1, "the pick is written as a flag");
        assert_eq!(receipt.kept, 1, "tag-only leaves the file on disk");
        assert!(receipt.is_restorable());
        assert!(receipt.detail().contains("Left on disk"), "{}", receipt.detail());
        assert_eq!(receipt.batch_id, engine.deleted_batches()[0].id);

        engine.clear_receipt();
        assert!(engine.receipt().is_none(), "dismissing forgets it");
    }

    #[test]
    fn applying_move_moves_the_file_and_a_changed_file_refuses_the_batch() {
        let folder = Temp::new("apply-move-photos");
        std::fs::create_dir_all(folder.0.join("day1")).expect("dir");
        let image = folder.0.join("day1/a.jpg");
        std::fs::write(&image, b"x").expect("write");
        let destination = std::env::temp_dir().join(format!("sortlens-engine-apply-dest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&destination);

        let (_lib, mut engine) = library("apply-move");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        // A file that changed after indexing refuses the batch, and the session survives.
        engine.begin_cull(&[id]).expect("begin");
        engine.cull_decide(sortlens_cull::Decision::Reject);
        std::fs::write(&image, b"edited after indexing").expect("edit");
        let refused = engine.cull_apply(sortlens_cull::trash::Mode::Move(destination.clone()));
        assert!(refused.is_err(), "a changed file stops the batch");
        assert!(engine.cull().is_some(), "the session is kept");
        assert!(image.exists(), "nothing was moved");

        // Put the size back and the same batch goes through.
        std::fs::write(&image, b"x").expect("restore size");
        let report = engine
            .cull_apply(sortlens_cull::trash::Mode::Move(destination.clone()))
            .expect("applied");
        assert_eq!(report.applied.len(), 1);
        assert!(!image.exists());
        assert!(destination.join("day1/a.jpg").exists(), "relative path preserved");
        assert!(engine.photo(id).expect("photo").missing, "the catalog knows it is gone");
        assert!(engine.cull().is_none());

        let _ = std::fs::remove_dir_all(&destination);
    }

    #[test]
    fn detail_navigation_walks_the_view_order_and_stops_at_the_ends() {
        let folder = Temp::new("detail-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::write(folder.0.join(name), name.as_bytes()).expect("write");
        }
        let (_lib, mut engine) = library("detail");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Name, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();

        let filter = Filter::default();
        assert_eq!(engine.neighbour(&filter, Sort::Name, ids[0], 1), Some(ids[1]));
        assert_eq!(engine.neighbour(&filter, Sort::Name, ids[2], 1), None);
        assert_eq!(engine.neighbour(&filter, Sort::Name, ids[0], -1), None);
        assert_eq!(engine.position(&filter, Sort::Name, ids[1]), Some((2, 3)));
    }

    #[test]
    fn subfolders_are_listed_per_level() {
        let folder = Temp::new("subfolder-photos");
        std::fs::create_dir_all(folder.0.join("day1/sub")).expect("dir");
        std::fs::create_dir_all(folder.0.join("day2")).expect("dir");
        std::fs::write(folder.0.join("top.jpg"), b"t").expect("write");
        std::fs::write(folder.0.join("day1/a.jpg"), b"a").expect("write");
        std::fs::write(folder.0.join("day1/sub/b.jpg"), b"b").expect("write");
        std::fs::write(folder.0.join("day2/c.jpg"), b"c").expect("write");
        let root = folder.0.to_string_lossy().to_string();
        let (_lib, mut engine) = library("subfolders");
        engine.begin_scan(root.clone()).expect("scan");
        wait_for_scan(&mut engine);

        let value = engine
            .run("library.subfolders", &json!({ "root": root, "prefix": "" }))
            .expect("subfolders");
        let folders = value["folders"].as_array().expect("array");
        assert_eq!(folders.len(), 2);
        assert_eq!(folders[0]["name"], "day1");
        assert_eq!(folders[0]["count"], 2, "counts include nested photos");
        assert_eq!(folders[1]["name"], "day2");

        let inner = engine
            .run("library.subfolders", &json!({ "root": root, "prefix": "day1/" }))
            .expect("subfolders");
        assert_eq!(inner["folders"][0]["name"], "sub");
    }

    #[test]
    fn a_hash_pass_groups_identical_files_and_keeps_the_best() {
        let folder = Temp::new("similar-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        let gradient = encode_gradient(120, 80);
        let stripes = encode_gradient(120, 80);
        std::fs::write(folder.0.join("a.jpg"), &gradient).expect("write");
        std::fs::write(folder.0.join("copy.jpg"), &gradient).expect("write");
        // A visibly different picture, so it must stay on its own.
        std::fs::write(folder.0.join("other.jpg"), &stripes[..stripes.len() / 3]).expect("write");

        let (_lib, mut engine) = library("similar");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        assert_eq!(engine.stats().total, 3);

        engine.begin_similar(SIMILAR_THRESHOLD).expect("start");
        assert!(engine.hash_progress().is_some());
        for _ in 0..600 {
            if engine.poll_similar(SIMILAR_THRESHOLD).is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let groups = engine.similar_groups();
        assert_eq!(groups.len(), 1, "only the two identical files pair up");
        assert_eq!(groups[0].len(), 2);
        assert!(engine.hash_progress().is_none());

        let workers = engine.jobs();
        let job = workers.iter().find(|job| job.kind == JobKind::Hash).expect("hash job");
        assert_eq!(job.status, JobStatus::Done);

        // A second pass reuses the hashes and starts no new job.
        assert_eq!(engine.begin_similar(SIMILAR_THRESHOLD).expect("second"), 0);
        assert_eq!(engine.similar_groups().len(), 1);

        let best = engine.similar_groups()[0].best;
        let found = engine.similar_to(best, SIMILAR_THRESHOLD).expect("anchor");
        assert_eq!(found.len(), 2);
    }

    /// A greyscale JPEG with a diagonal ramp, for the similarity test.
    fn encode_gradient(width: u32, height: u32) -> Vec<u8> {
        let mut luma = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                luma.push(((x * 2 + y) % 240) as u8);
            }
        }
        craft_jpeg::encode_baseline(
            &luma,
            width as usize,
            height as usize,
            1,
            92,
            craft_jpeg::ChromaSubsampling::S444,
            &[],
        )
        .expect("encode")
    }

    #[test]
    fn insights_reconcile_with_the_catalog() {
        let folder = Temp::new("insights-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"aaaa").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"bbbb").expect("write");
        let (_lib, mut engine) = library("insights");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);

        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        engine.rate(&ids[..1], 4).expect("rate");
        engine.flag(&ids[1..], Flag::Pick).expect("flag");
        engine.set_tag(&ids[..1], "beach", true).expect("tag");

        let value = engine.run("view.insights", &json!({})).expect("insights");
        assert_eq!(value["total"], 2);
        assert_eq!(value["bytes"], 8);
        assert_eq!(value["rated"], 1);
        assert_eq!(value["picks"], 1);
        assert_eq!(value["tagged"], 1);
        assert_eq!(value["byRating"][4], 1);
        assert_eq!(value["keywords"][0]["name"], "beach");
        assert_eq!(value["keywords"][0]["count"], 1);
        assert_eq!(engine.count(&Filter::default()), 2);

        let typed = engine.insights();
        assert_eq!(typed.stats.total, 2);
        assert_eq!(typed.by_rating[4], 1);
    }

    #[test]
    fn collections_round_trip_through_the_engine() {
        let folder = Temp::new("collection-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        let (_lib, mut engine) = library("collection");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();

        let created = engine.run("collection.new", &json!({ "name": "Best of 2024" })).expect("create");
        let collection = sortlens_core::CollectionId(created["id"].as_u64().expect("id") as u32);
        assert_eq!(engine.collections().len(), 1);

        let added = engine
            .run("collection.addTo", &json!({ "collection": collection.0, "ids": ids }))
            .expect("add");
        assert_eq!(added["changed"], 2);

        let filter = Filter {
            collection: Some(collection),
            ..Filter::default()
        };
        assert_eq!(engine.count(&filter), 2);
        engine
            .run("collection.removeFrom", &json!({ "collection": collection.0, "ids": [ids[0]] }))
            .expect("remove");
        assert_eq!(engine.count(&filter), 1);

        engine
            .run("collection.rename", &json!({ "collection": collection.0, "name": "Keepers" }))
            .expect("rename");
        assert_eq!(engine.collections()[0].name, "Keepers");

        engine
            .run("collection.delete", &json!({ "collection": collection.0 }))
            .expect("delete");
        assert!(engine.collections().is_empty());
        assert_eq!(engine.count(&Filter::default()), 2, "photos stay in the library");
        assert!(engine.run("collection.addTo", &json!({ "collection": 99, "ids": ids })).is_err());
    }

    #[test]
    fn health_check_marks_missing_photos_and_a_moved_file_is_found() {
        let folder = Temp::new("health-photos");
        std::fs::create_dir_all(folder.0.join("day1")).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"photo-a").expect("write");
        std::fs::write(folder.0.join("day1/b.jpg"), b"photo-b").expect("write");
        let (_lib, mut engine) = library("health");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let a = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        let clean = engine.run("library.health", &json!({})).expect("health");
        assert_eq!(clean["missing"], 0);
        assert_eq!(clean["checked"], 2);

        // Move one file to a new subfolder: the catalog still points at the old path.
        let moved = folder.0.join("day1/a.jpg");
        std::fs::rename(folder.0.join("a.jpg"), &moved).expect("move");
        let after = engine.run("library.health", &json!({})).expect("health");
        assert_eq!(after["missing"], 1);
        assert!(engine.photo(a).expect("photo").missing);

        let relocated = engine.run("library.relocate", &json!({})).expect("relocate");
        assert_eq!(relocated["relocated"], 1);
        let photo = engine.photo(a).expect("photo");
        assert_eq!(photo.rel, "day1/a.jpg");
        assert!(!photo.missing, "a found photo is no longer missing");
        assert_eq!(engine.run("library.health", &json!({})).expect("health")["missing"], 0);
    }

    #[test]
    fn removing_missing_photos_can_be_undone() {
        let folder = Temp::new("health-remove-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        let (_lib, mut engine) = library("health-remove");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        std::fs::remove_file(folder.0.join("a.jpg")).expect("remove");
        engine.check_health().expect("health");

        assert_eq!(engine.run("library.removeMissing", &json!({})).expect("remove")["removed"], 1);
        assert_eq!(engine.count(&Filter::default()), 1);
        assert!(engine.undo().expect("undo"));
        let with_missing = Filter {
            include_missing: true,
            ..Filter::default()
        };
        assert_eq!(engine.count(&with_missing), 2);
    }

    #[test]
    fn an_applied_batch_lands_in_the_rejects_bin_and_restores() {
        let folder = Temp::new("bin-photos");
        std::fs::create_dir_all(folder.0.join("day1")).expect("dir");
        let image = folder.0.join("day1/a.jpg");
        std::fs::write(&image, b"x").expect("write");
        let destination = std::env::temp_dir().join(format!("sortlens-engine-bin-dest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&destination);
        let (library_dir, _) = library("bin");

        let id = {
            let mut engine = Engine::open(&library_dir.0).expect("open");
            engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
            wait_for_scan(&mut engine);
            let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
            engine.begin_cull(&[id]).expect("begin");
            engine.cull_decide(sortlens_cull::Decision::Reject);
            engine
                .cull_apply(sortlens_cull::trash::Mode::Move(destination.clone()))
                .expect("apply");
            assert_eq!(engine.deleted_batches().len(), 1);
            assert_eq!(engine.deleted_batches()[0].items.len(), 1);
            assert_eq!(engine.deleted_batches()[0].summary(), "1 photo");
            id
        };

        // The bin survives a restart.
        let mut engine = Engine::open(&library_dir.0).expect("reopen");
        let batch_id = engine.deleted_batches()[0].id;
        assert_eq!(engine.run("library.deleted", &json!({})).expect("list")["batches"][0]["count"], 1);

        assert_eq!(engine.restore_batch(batch_id).expect("restore"), 1);
        assert!(image.exists(), "the file is back");
        assert!(!engine.photo(id).expect("photo").missing);
        assert_eq!(engine.photo(id).expect("photo").flag, Flag::None, "the flag is back");
        assert!(engine.deleted_batches().is_empty(), "the batch left the bin");
        assert!(engine.run("library.restoreBatch", &json!({ "id": batch_id })).is_err());
        let _ = std::fs::remove_dir_all(&destination);
    }

    #[test]
    fn a_cull_session_needs_a_photo() {
        let (_lib, mut engine) = library("cull-empty");
        assert!(engine.begin_cull(&[]).is_err());
    }

    #[test]
    fn cull_commands_round_trip_through_json() {
        let folder = Temp::new("cull-json-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        let (_lib, mut engine) = library("cull-json");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        let started = engine.run("cull.start", &json!({ "ids": [id] })).expect("start");
        assert_eq!(started["total"], 1);
        let decided = engine.run("cull.decide", &json!({ "decision": "reject" })).expect("decide");
        assert_eq!(decided["summary"]["rejects"], 1);
        assert_eq!(engine.run("cull.summary", &json!({})).expect("summary")["total"], 1);
        assert!(engine.run("cull.decide", &json!({ "decision": "nonsense" })).is_err());
    }

    #[test]
    fn keywords_are_normalised_added_and_removed() {
        let folder = Temp::new("tag-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        let (_lib, mut engine) = library("tag");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        assert_eq!(engine.set_tag(&[id], "  #Beach ", true).expect("tag"), 1);
        assert_eq!(engine.photo(id).expect("photo").tags, vec!["beach".to_string()]);
        assert_eq!(engine.set_tag(&[id], "beach", true).expect("tag"), 0, "already present");
        assert_eq!(engine.tag_vocabulary(), vec![("beach".to_string(), 1)]);

        assert_eq!(engine.set_tag(&[id], "beach", false).expect("untag"), 1);
        assert!(engine.photo(id).expect("photo").tags.is_empty());
        assert!(engine.set_tag(&[id], "   ", true).is_err());
    }

    #[test]
    fn keywords_are_undoable_and_filterable() {
        let folder = Temp::new("tag-filter-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        let (_lib, mut engine) = library("tag-filter");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let photos = engine.page(&Filter::default(), Sort::Natural, 0, 10).photos;
        let ids: Vec<PhotoId> = photos.iter().map(|photo| photo.id).collect();
        engine.set_tag(&ids, "keepers", true).expect("tag");

        let filter = Filter {
            tags: vec!["keepers".into()],
            ..Filter::default()
        };
        assert_eq!(engine.count(&filter), 2);
        let value = engine
            .run("photo.keyword", &json!({ "ids": ids, "tag": "keepers", "add": false }))
            .expect("command");
        assert_eq!(value["changed"], 2);
        assert_eq!(engine.count(&filter), 0);
        assert!(engine.undo().expect("undo"));
        assert_eq!(engine.count(&filter), 2);
    }

    #[test]
    fn review_changes_are_written_to_a_sidecar() {
        let folder = Temp::new("sidecar-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        let image = folder.0.join("IMG_0001.jpg");
        std::fs::write(&image, b"x").expect("write");
        let (_lib, mut engine) = library("sidecar");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        engine.rate(&[id], 3).expect("rate");
        engine.flag(&[id], Flag::Pick).expect("flag");
        engine.set_tag(&[id], "beach", true).expect("tag");

        let sidecar = sortlens_sidecar::read(&image).expect("read").expect("present");
        assert_eq!(sidecar.rating, 3);
        assert_eq!(sidecar.flag, Flag::Pick);
        assert_eq!(sidecar.keywords, vec!["beach".to_string()]);
        assert!(
            engine
                .jobs()
                .iter()
                .any(|job| job.kind == JobKind::Sidecar && job.status == JobStatus::Done),
            "the write shows up in the activity stack"
        );
    }

    #[test]
    fn a_sidecar_measured_before_the_scan_seeds_the_photo() {
        let folder = Temp::new("sidecar-import-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        let image = folder.0.join("IMG_0002.jpg");
        std::fs::write(&image, b"x").expect("write");
        sortlens_sidecar::write(
            &image,
            &sortlens_sidecar::Sidecar::new(4, Flag::Reject, Some(sortlens_core::ColorLabel::Blue), vec!["imported".into()]),
        )
        .expect("sidecar");

        let (_lib, mut engine) = library("sidecar-import");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let photo = &engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0];
        assert_eq!(photo.rating, 4);
        assert_eq!(photo.flag, Flag::Reject);
        assert_eq!(photo.label, Some(sortlens_core::ColorLabel::Blue));
        assert_eq!(photo.tags, vec!["imported".to_string()]);
    }

    #[test]
    fn sidecar_writing_can_be_switched_off() {
        let folder = Temp::new("sidecar-off-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        let image = folder.0.join("a.jpg");
        std::fs::write(&image, b"x").expect("write");
        let (_lib, mut engine) = library("sidecar-off");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        engine.set_write_sidecars(false);
        engine.rate(&[id], 5).expect("rate");
        assert!(sortlens_sidecar::read(&image).expect("read").is_none());
        assert_eq!(engine.photo(id).expect("photo").rating, 5, "the catalog still records it");
    }

    #[cfg(unix)]
    #[test]
    fn an_unwritable_folder_reports_the_failure_and_keeps_the_rating() {
        use std::os::unix::fs::PermissionsExt;
        let folder = Temp::new("sidecar-fail-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        let (_lib, mut engine) = library("sidecar-fail");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        let mut permissions = std::fs::metadata(&folder.0).expect("meta").permissions();
        permissions.set_mode(0o555);
        std::fs::set_permissions(&folder.0, permissions).expect("chmod");

        engine.rate(&[id], 2).expect("rating still applies");
        assert_eq!(engine.photo(id).expect("photo").rating, 2);
        assert!(
            engine
                .jobs()
                .iter()
                .any(|job| job.kind == JobKind::Sidecar && job.status == JobStatus::Failed)
        );

        let mut permissions = std::fs::metadata(&folder.0).expect("meta").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&folder.0, permissions).expect("chmod back");
    }

    #[test]
    fn a_missing_folder_fails_the_job() {
        let missing = std::env::temp_dir().join("sortlens-no-such-folder-xyz");
        let _ = std::fs::remove_dir_all(&missing);
        let (_lib, mut engine) = library("job-fail");
        engine.begin_scan(missing.to_string_lossy().to_string()).expect("start");
        wait_for_scan(&mut engine);
        let jobs = engine.jobs();
        assert_eq!(jobs[0].status, JobStatus::Failed);
        assert!(jobs[0].error.is_some());
    }

    #[test]
    fn jobs_are_listed_as_json() {
        let (_lib, mut engine) = library("job-json");
        let value = engine.run("library.jobs", &json!({})).expect("jobs");
        assert_eq!(value["jobs"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn stats_are_reported_as_json() {
        let (_lib, mut engine) = library("stats-json");
        let stats = engine.run("library.stats", &json!({})).expect("stats");
        assert_eq!(stats["total"], 0);
    }
}
