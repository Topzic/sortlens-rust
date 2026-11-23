//! L4: the command layer.
//!
//! The engine owns the open catalog, the scan worker and the command registry. The UI, the CLI
//! and the MCP server all call the same typed methods, and [`Engine::run`] is the JSON face of
//! them for scripts.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod command;
pub mod diagnostics;
pub mod job;
pub mod moves;

pub use command::{CommandSpec, Group, Registry, Status};
pub use job::{Job, JobKind, JobStatus};
pub use sortlens_ai::{Caps as SuggestionCaps, Source as SuggestionSource, Suggestion};
pub use sortlens_catalog::backup::Backup;
pub use sortlens_catalog::folder::{FolderMark, key_of as folder_key};
pub use sortlens_catalog::history::Entry as HistoryEntry;
pub use sortlens_catalog::photo::now_epoch;
pub use sortlens_catalog::{Filter, Op, Page, Photo, Sort, Stats, SyncOutcome, TagMode, TagOrder};
pub use sortlens_core::{ColorLabel, Error, Flag, MediaKind, PhotoId, Result};
pub use sortlens_cull::trash::Receipt as ApplyReceipt;
pub use sortlens_cull::{Decision, Summary as CullSummary};
pub use sortlens_export::{Collision, Format as ExportFormat, Report as ExportReport, Spec as ExportSpec};
pub use sortlens_insights::{Highlight as InsightHighlight, Insights, Row as InsightRow, month_name};
pub use sortlens_similar::{
    DEFAULT_COLOUR as SIMILAR_COLOUR, DEFAULT_THRESHOLD as SIMILAR_THRESHOLD, Group as SimilarGroup, Match as SimilarMatch,
    Member as SimilarMember,
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
    /// Ask the host to bring a view forward: `"detail"`, `"similar"` or `"insights"`.
    ///
    /// The engine knows the command and the data, but only the host knows what is on screen, so the
    /// view is named rather than switched. A command in the registry must have a handler; these two
    /// were `Ready` while `run` had no arm for them, which this variant and the examples test fixed.
    ShowView(&'static str),
}

/// Which of the two OS actions a photo asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealMode {
    /// Show the file in the desktop's file manager.
    Reveal,
    /// Hand the file to the default application.
    Open,
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
/// An export running on its own thread.
struct PendingExport {
    job: u64,
    rx: Receiver<sortlens_export::Report>,
    total: usize,
    done: Arc<std::sync::atomic::AtomicUsize>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

struct PendingBatch {
    job: u64,
    rx: Receiver<Vec<(PhotoId, Vec<Suggestion>)>>,
    total: usize,
    done: Arc<std::sync::atomic::AtomicUsize>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

/// A sharpness pass in flight.
///
/// A score is a self-contained statement about one photograph, so a cancelled pass still commits what it measured —
/// unlike a cancelled scan, which commits nothing because a half-scanned folder would leave the catalog describing
/// a disk that is not there.
struct PendingSharpness {
    job: u64,
    rx: Receiver<Vec<(PhotoId, sortlens_quality::Score)>>,
    total: usize,
    done: Arc<std::sync::atomic::AtomicUsize>,
    unjudged: Arc<std::sync::atomic::AtomicUsize>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

/// What a finished sharpness pass measured.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SharpnessReport {
    /// Photos whose pixels could be measured, and whose score was stored (or already matched).
    pub measured: usize,
    /// Photos that were measured but could not be judged — too dark, or a file this build cannot decode.
    ///
    /// They are reported rather than scored: a near-black frame has no detail to measure, and calling it sharp or
    /// blurry on the strength of its noise would be a wrong answer rather than a missing one.
    pub unjudged: usize,
    /// Photos the pass did not get to, because it was cancelled.
    pub remaining: usize,
}

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
    /// Files whose header contradicted their extension and were routed by content instead.
    pub mislabelled: usize,
    /// How long the walk took.
    pub elapsed: std::time::Duration,
    /// The walk was cancelled; nothing was committed.
    pub cancelled: bool,
}

/// One root and the state of the volume under it, for the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootVolume {
    /// The root as stored in the catalog.
    pub root: String,
    /// How many photos live under it.
    pub photos: usize,
    /// Where its volume is mounted, if the platform could say.
    pub mount: Option<String>,
    /// Whether that volume looks like removable media.
    pub removable: bool,
    /// Whether the mount point exists right now; `None` when it could not be probed.
    pub mounted: Option<bool>,
}

/// The command engine.
pub struct Engine {
    registry: Registry,
    dir: Option<PathBuf>,
    last_effect: Effect,
    catalog: Option<Catalog>,
    pending: Option<PendingScan>,
    pending_export: Option<PendingExport>,
    last_export: Option<sortlens_export::Report>,
    pending_hash: Option<PendingHash>,
    pending_sharpness: Option<PendingSharpness>,
    hashes: std::collections::HashMap<u64, (sortlens_hash::Hash, sortlens_hash::Colour)>,
    pending_insights: Option<PendingInsights>,
    pending_batch: Option<PendingBatch>,
    batch: Vec<(PhotoId, Vec<Suggestion>)>,
    insights_ready: Option<Insights>,
    similar_members: Vec<SimilarMember>,
    similar: Vec<SimilarGroup>,
    receipt: Option<ApplyReceipt>,
    cull: Option<sortlens_cull::Session>,
    deleted: Vec<sortlens_cull::trash::DeletedBatch>,
    next_batch: u64,
    moves: Vec<crate::moves::MoveReceipt>,
    next_move: u64,
    jobs: Vec<Job>,
    next_job: u64,
    write_sidecars: bool,
    warnings: Vec<String>,
    /// The last keyword this session added, for `photo.tagFromPrev`.
    ///
    /// Session memory, not persisted: a keyword applied last week is not "the last keyword", and silently
    /// pasting somebody else's older tags onto a photo is a wrong write rather than a convenience. Only an
    /// *added* keyword counts — removing one is not something you then want repeated.
    last_tag: Option<String>,
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
            pending_export: None,
            last_export: None,
            pending_hash: None,
            pending_sharpness: None,
            hashes: std::collections::HashMap::new(),
            pending_insights: None,
            pending_batch: None,
            batch: Vec::new(),
            insights_ready: None,
            similar_members: Vec::new(),
            similar: Vec::new(),
            receipt: None,
            cull: None,
            deleted: Vec::new(),
            next_batch: 1,
            moves: Vec::new(),
            next_move: 1,
            jobs: Vec::new(),
            next_job: 1,
            write_sidecars: true,
            warnings: Vec::new(),
            last_tag: None,
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
        engine.load_moves();
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
        if self.read_only() {
            return;
        }
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

    /// Set the colour on several collections as **one** change.
    ///
    /// Looping the single-collection call would put one op in the log per collection, so undoing a recolour of
    /// five collections would take five ⌘Z. This commits a single `Op::Group`, whose inverse is one group too.
    /// Only collections whose colour actually moves are counted, like every other write.
    pub fn set_collections_colour(&mut self, ids: &[sortlens_core::CollectionId], colour: Option<ColorLabel>) -> Result<usize> {
        let existing: Vec<sortlens_catalog::Collection> = self.collections();
        let ops: Vec<Op> = ids
            .iter()
            .filter(|id| {
                existing
                    .iter()
                    .any(|collection| collection.id == **id && collection.colour != colour)
            })
            .map(|id| Op::SetCollectionColour { id: *id, colour })
            .collect();
        let count = ops.len();
        if !ops.is_empty() {
            self.commit_group(ops)?;
        }
        Ok(count)
    }

    /// Pin or unpin several collections as one change.
    pub fn set_collections_favourite(&mut self, ids: &[sortlens_core::CollectionId], favourite: bool) -> Result<usize> {
        let existing: Vec<sortlens_catalog::Collection> = self.collections();
        let ops: Vec<Op> = ids
            .iter()
            .filter(|id| {
                existing
                    .iter()
                    .any(|collection| collection.id == **id && collection.favourite != favourite)
            })
            .map(|id| Op::SetCollectionFavourite { id: *id, favourite })
            .collect();
        let count = ops.len();
        if !ops.is_empty() {
            self.commit_group(ops)?;
        }
        Ok(count)
    }

    /// Delete several collections as one change. The photos stay in the library, exactly as a single delete.
    pub fn delete_collections(&mut self, ids: &[sortlens_core::CollectionId]) -> Result<usize> {
        let existing: Vec<sortlens_catalog::Collection> = self.collections();
        let ops: Vec<Op> = existing
            .iter()
            .filter(|collection| ids.contains(&collection.id))
            .map(|collection| Op::RemoveCollection {
                collection: Box::new(collection.clone()),
            })
            .collect();
        let count = ops.len();
        if !ops.is_empty() {
            self.commit_group(ops)?;
        }
        Ok(count)
    }

    /// Commit several ops as one undoable step.
    fn commit_group(&mut self, ops: Vec<Op>) -> Result<()> {
        let catalog = self.catalog.as_mut().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        catalog.commit(Op::Group { ops })
    }

    /// Every folder mark: a sidebar pin and colour, keyed by path identity.
    pub fn folder_marks(&self) -> sortlens_catalog::folder::FolderMarks {
        self.catalog
            .as_ref()
            .map(|catalog| catalog.folder_marks().clone())
            .unwrap_or_default()
    }

    /// The mark on one folder, or the default when it has none.
    pub fn folder_mark(&self, key: &str) -> sortlens_catalog::folder::FolderMark {
        self.folder_marks().get(key).cloned().unwrap_or_default()
    }

    /// Set a folder's pin and colour as one change, keyed by `sortlens_core::pathkey::PathKey`.
    ///
    /// The key is computed from the path *here* rather than taken from the caller, so a UI that passed a typed
    /// spelling and a CLI that passed a normalised one cannot create two marks for one folder.
    pub fn set_folder_mark(&mut self, path: &str, mark: sortlens_catalog::folder::FolderMark) -> Result<bool> {
        let key = sortlens_catalog::folder::key_of(path);
        if key.is_empty() {
            return Err(Error::Invalid("a folder mark needs a path".into()));
        }
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .set_folder_mark(&key, mark)
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
            let path = PathBuf::from(&photo.root).join(&photo.rel);
            health.checked += 1;
            match std::fs::metadata(&path) {
                Ok(meta) => {
                    health.present += 1;
                    if photo.missing {
                        changes.push((*id, false));
                    }
                    // Size is definitive; a modification time alone is only a hint, so a tolerance
                    // covers filesystems that store whole seconds.
                    let size_differs = meta.len() != photo.size;
                    let stamp = meta
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|delta| delta.as_secs() as i64);
                    let time_differs = match (stamp, photo.modified) {
                        (Some(now), Some(indexed)) => (now - indexed).abs() > MODIFIED_TOLERANCE,
                        _ => false,
                    };
                    if size_differs || time_differs {
                        health.modified += 1;
                        health.modified_ids.push(*id);
                    }
                }
                Err(_) => {
                    health.missing += 1;
                    if !photo.missing {
                        changes.push((*id, true));
                    }
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

    /// Re-read the photos the last health check found changed on disk.
    ///
    /// Each photo is re-hashed and its metadata extracted again, then stored with one undoable
    /// update per photo — the same shape a rescan uses, only narrowed to what actually moved.
    /// Returns how many photos were refreshed.
    pub fn refresh_changed(&mut self) -> Result<usize> {
        let changed = self.check_health()?.modified_ids;
        let mut refreshed = 0usize;
        for id in changed {
            let Some(photo) = self.photo(id) else {
                continue;
            };
            let path = PathBuf::from(&photo.root).join(&photo.rel);
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            let Ok(key) = sortlens_scan::content_key(&path, meta.len()) else {
                continue;
            };
            let outcome = sortlens_meta::read_outcome(&path);
            let mut updated = photo.clone();
            updated.size = meta.len();
            updated.modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|delta| delta.as_secs() as i64);
            updated.key = key;
            updated.apply_meta(&outcome.meta());
            updated.retry = outcome.is_retryable();
            self.mutate(Op::AddPhoto { photo: Box::new(updated) })?;
            refreshed += 1;
        }
        Ok(refreshed)
    }

    /// Photos whose metadata could not be read when they were indexed.
    pub fn retry_count(&self) -> usize {
        self.catalog.as_ref().map_or(0, |catalog| catalog.retrying_photos().len())
    }

    /// How many photos have no camera and no capture date, so a better reader could still help.
    pub fn exifless_count(&self) -> usize {
        self.catalog.as_ref().map_or(0, |catalog| catalog.exifless_photos().len())
    }

    /// Retry metadata extraction for queued photos, and optionally for every photo with no EXIF.
    ///
    /// A photo that reads this time is updated and leaves the queue; one that still cannot be read
    /// stays queued and is counted as remaining rather than reported as a failure. `all` widens the
    /// pass to photos whose metadata is merely empty — useful after the app learns a new format.
    /// Returns `(updated, remaining)`.
    pub fn retry_metadata(&mut self, all: bool) -> Result<(usize, usize)> {
        let mut queued = match self.catalog.as_ref() {
            Some(catalog) => catalog.retrying_photos(),
            None => Vec::new(),
        };
        if all {
            let queued_ids: std::collections::HashSet<PhotoId> = queued.iter().map(|photo| photo.id).collect();
            if let Some(catalog) = self.catalog.as_ref() {
                for photo in catalog.exifless_photos() {
                    if !queued_ids.contains(&photo.id) {
                        queued.push(photo);
                    }
                }
            }
        }
        let mut recovered = 0usize;
        for photo in queued {
            let path = Path::new(&photo.root).join(&photo.rel);
            let outcome = sortlens_meta::read_outcome(&path);
            if outcome.is_retryable() {
                continue;
            }
            let mut updated = photo.clone();
            updated.apply_meta(&outcome.meta());
            updated.retry = false;
            self.mutate(Op::AddPhoto { photo: Box::new(updated) })?;
            recovered += 1;
        }
        let remaining = self.retry_count();
        Ok((recovered, remaining))
    }

    /// Moves that happened, newest last, so any of them can be put back.
    pub fn move_receipts(&self) -> &[crate::moves::MoveReceipt] {
        &self.moves
    }

    /// Move photos the user picked into a destination folder, verified as a batch.
    ///
    /// Nothing is touched unless the whole batch passes the audit — the same rule Apply uses, because
    /// half a move is worse than none. Every moved photo keeps its review state and its content key;
    /// only where the file lives changes, and the receipt can put them all back.
    pub fn move_picks(&mut self, dest: &str, ids: &[PhotoId]) -> Result<usize> {
        if self.read_only() {
            return Err(Error::Invalid(
                "this library is open read-only because another window is using it".into(),
            ));
        }
        if ids.is_empty() {
            return Err(Error::Invalid("nothing is selected".into()));
        }
        let dest = crate::moves::prepare_dest(Path::new(dest))?;
        let dest_text = dest.to_string_lossy().to_string();
        if self.roots().iter().any(|root| root.trim_end_matches('/') == dest_text) {
            return Err(Error::Invalid(format!("{dest_text} is already an indexed folder")));
        }

        let mut candidates = Vec::new();
        for id in ids {
            let photo = self.photo(*id).ok_or(Error::NoSuchPhoto(*id))?;
            let path = Path::new(&photo.root).join(&photo.rel);
            if prop_is_inside(&path, &dest) {
                return Err(Error::Invalid(format!("{} is already in {dest_text}", photo.name)));
            }
            candidates.push(sortlens_cull::trash::Candidate {
                id: *id,
                path,
                root: PathBuf::from(&photo.root),
                size: photo.size,
                modified: photo.modified,
            });
        }
        let batch = sortlens_cull::trash::Batch {
            mode: sortlens_cull::trash::Mode::Move(dest.clone()),
            candidates,
        };
        let report = sortlens_cull::trash::apply_checked(&batch)?;

        let mut items = Vec::new();
        let mut ops = vec![Op::AddRoot { path: dest_text.clone() }];
        for applied in &report.applied {
            let sortlens_cull::trash::Location::Moved(location) = &applied.location else {
                continue;
            };
            let Some(old) = self.photo(applied.id) else {
                continue;
            };
            let Ok(rel) = location.strip_prefix(&dest) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            let new = crate::moves::moved_record(old, &dest_text, &rel);
            ops.push(Op::RemovePhoto {
                photo: Box::new(old.clone()),
            });
            ops.push(Op::AddPhoto {
                photo: Box::new(new.clone()),
            });
            let sidecar = crate::moves::take_sidecar(&applied.from, location);
            items.push(crate::moves::MoveItem {
                old: old.clone(),
                new,
                sidecar,
            });
        }
        if items.is_empty() {
            return Err(Error::Invalid("no photo could be moved".into()));
        }
        self.mutate(Op::Group { ops })?;

        let receipt = crate::moves::MoveReceipt {
            id: self.next_move,
            dest: dest_text,
            at: sortlens_catalog::store::now_seconds(),
            items,
        };
        self.next_move += 1;
        let count = receipt.len();
        self.moves.push(receipt);
        self.persist_moves();
        Ok(count)
    }

    /// Put a moved batch back where it came from, files and records together.
    ///
    /// A file that cannot be moved back stops the undo and is named: the receipt stays, so the rest
    /// can be tried again once the problem is gone.
    pub fn undo_move(&mut self, id: u64) -> Result<usize> {
        if self.read_only() {
            return Err(Error::Invalid(
                "this library is open read-only because another window is using it".into(),
            ));
        }
        let index = self
            .moves
            .iter()
            .position(|receipt| receipt.id == id)
            .ok_or_else(|| Error::Invalid(format!("no move {id}")))?;
        let receipt = self.moves[index].clone();
        let mut ops = Vec::new();
        for item in receipt.items.iter().rev() {
            crate::moves::put_back(item)?;
            ops.push(Op::RemovePhoto {
                photo: Box::new(item.new.clone()),
            });
            ops.push(Op::AddPhoto {
                photo: Box::new(item.old.clone()),
            });
        }
        self.mutate(Op::Group { ops })?;
        self.moves.remove(index);
        self.persist_moves();
        Ok(receipt.len())
    }

    /// Where the move receipts are saved.
    fn moves_path(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|dir| dir.join("moves.json"))
    }

    fn persist_moves(&mut self) {
        if self.read_only() {
            return;
        }
        let Some(path) = self.moves_path() else {
            return;
        };
        let result = serde_json::to_vec_pretty(&self.moves)
            .map_err(|error| Error::Invalid(format!("moves: {error}")))
            .and_then(|bytes| write_atomic(&path, &bytes));
        if let Err(error) = result {
            self.warnings.push(format!("could not save the move receipts: {error}"));
        }
    }

    fn load_moves(&mut self) {
        let Some(path) = self.moves_path() else {
            return;
        };
        let Ok(text) = std::fs::read(&path) else {
            return;
        };
        match serde_json::from_slice::<Vec<crate::moves::MoveReceipt>>(&text) {
            Ok(receipts) => {
                self.next_move = receipts.iter().map(|receipt| receipt.id).max().unwrap_or(0) + 1;
                self.moves = receipts;
            }
            Err(_) => self.warnings.push("ignored unreadable move receipts".into()),
        }
    }

    /// Whether the library is open read-only because another window is writing it.
    pub fn read_only(&self) -> bool {
        self.catalog.as_ref().is_some_and(sortlens_catalog::Catalog::read_only)
    }

    /// The catalog's revision: the sequence number of the last change, carried in every command
    /// response as `rev` so a caller can pass it back as `expect` and refuse to write over
    /// annotations that arrived since it looked (SL-D11).
    pub fn revision(&self) -> u64 {
        self.catalog.as_ref().map(sortlens_catalog::Catalog::seq).unwrap_or(0)
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

    /// Content key for a photo, so a renamed file keeps its hash.
    fn key_for(&self, id: PhotoId) -> Option<u64> {
        self.catalog.as_ref()?.photo(id).map(|photo| photo.key)
    }

    /// Every photo in the library, whatever the filters.
    ///
    /// Used by the whole-library operations — exports, hashing, insights — which need the set, not a
    /// page of it.
    pub fn all_photos(&self) -> Vec<Photo> {
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
        if self.read_only() {
            // A read-only library must not write side files either: they would be silently lost.
            return;
        }
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

    /// Where each root lives, and whether that volume is still there.
    ///
    /// One check per root rather than one per photo: an unplugged drive holding ten thousand photos is one
    /// missing volume, not ten thousand missing files, and saying so is the difference between the app
    /// looking broken and the app knowing what happened. `mounted` is `None` when the platform cannot answer
    /// (see [`sortlens_core::volume::probe`]) — an unknown answer must never be shown as an absent drive.
    pub fn root_volumes(&self) -> Vec<RootVolume> {
        self.counts_by_root()
            .into_iter()
            .map(|(root, photos)| {
                let volume = sortlens_core::volume::probe(Path::new(&root));
                let mounted = volume.as_ref().map(sortlens_core::volume::mounted);
                RootVolume {
                    root,
                    photos,
                    mount: volume.as_ref().map(|volume| volume.mount.display().to_string()),
                    removable: volume.as_ref().is_some_and(|volume| volume.removable),
                    mounted,
                }
            })
            .collect()
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
        if self.read_only() {
            return Err(Error::Invalid(
                "this library is open read-only because another window is using it".into(),
            ));
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
                    mislabelled: 0,
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
                mislabelled: 0,
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
                meta_unreadable: file.meta_unreadable,
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

        let mislabelled = report.skipped.mislabelled;
        let status = if errors.is_empty() { JobStatus::Done } else { JobStatus::Failed };
        self.finish_job(job, status, report.files.len(), report.dirs, errors.len(), errors.first().cloned());

        ScanFinished {
            root,
            added: outcome.added,
            updated: outcome.updated,
            missing: outcome.missing,
            restored: outcome.restored,
            errors,
            mislabelled,
            elapsed,
            cancelled: false,
        }
    }

    /// Set stars on a selection, clamped to 0–5.
    pub fn rate(&mut self, ids: &[PhotoId], stars: u8) -> Result<usize> {
        let stars = stars.min(5);
        // Count the photos whose rating actually moved, not the request: a script that rates a page and is
        // told "12 changed" when eleven already had those stars cannot tell a write from a no-op.
        let changes: Vec<(PhotoId, u8)> = ids
            .iter()
            .filter(|id| self.photo(**id).is_some_and(|photo| photo.rating != stars))
            .map(|id| (*id, stars))
            .collect();
        let changed = self.mutate(Op::Rate { changes })?;
        self.sync_sidecars(ids);
        Ok(changed)
    }

    /// Set a flag on a selection.
    pub fn flag(&mut self, ids: &[PhotoId], flag: Flag) -> Result<usize> {
        let changes: Vec<(PhotoId, Flag)> = ids
            .iter()
            .filter(|id| self.photo(**id).is_some_and(|photo| photo.flag != flag))
            .map(|id| (*id, flag))
            .collect();
        let changed = self.mutate(Op::Flag { changes })?;
        self.sync_sidecars(ids);
        Ok(changed)
    }

    /// Add or remove one keyword on a selection.
    pub fn set_tag(&mut self, ids: &[PhotoId], tag: &str, add: bool) -> Result<usize> {
        let tag = normalize_tag(tag)?;
        // A keyword renamed earlier still means its current name.
        let tag = match self.catalog.as_ref() {
            Some(catalog) => catalog.resolve_tag(&tag),
            None => tag,
        };
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
        if add {
            self.last_tag = Some(tag);
        }
        self.sync_sidecars(ids);
        Ok(count)
    }

    /// The keyword `photo.tagFromPrev` would apply, if any has been applied this session.
    pub fn last_tag(&self) -> Option<&str> {
        self.last_tag.as_deref()
    }

    /// Re-apply the last keyword added this session to `ids`.
    ///
    /// This is the second photo of a look: you type "backlit" once, then move and press one key. It goes
    /// through `set_tag`, so a renamed keyword still means its current name, a photo that already carries it is
    /// not written twice, and the write is one undoable op like any other.
    pub fn tag_from_prev(&mut self, ids: &[PhotoId]) -> Result<usize> {
        let tag = self
            .last_tag
            .clone()
            .ok_or_else(|| Error::Invalid("no keyword has been applied yet this session".into()))?;
        self.set_tag(ids, &tag, true)
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

    /// Remove a keyword from every photo that carries it, and forget its aliases.
    ///
    /// Sidecars follow, so a deletion is not silently resurrected the next time a file is read.
    pub fn delete_tag(&mut self, tag: &str) -> Result<usize> {
        let tag = normalize_tag(tag)?;
        let touched: Vec<PhotoId> = self
            .all_photos()
            .into_iter()
            .filter(|photo| photo.has_tag(&tag))
            .map(|photo| photo.id)
            .collect();
        let catalog = self.catalog.as_mut().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        let count = catalog.delete_tag(&tag)?;
        if count > 0 {
            self.sync_sidecars(&touched);
            let job = self.start_job(JobKind::Sidecar, format!("#{tag} deleted everywhere"));
            self.finish_job(job, JobStatus::Done, count, 0, 0, None);
        }
        Ok(count)
    }

    /// Statistics over the whole library, recomputed each time.
    pub fn insights(&self) -> Insights {
        if let Some(ready) = self.insights_ready.clone() {
            return ready;
        }
        match self.catalog.as_ref() {
            Some(catalog) => Insights::compute(catalog.photos()),
            None => Insights::empty(),
        }
    }

    /// Set or clear the capture time on photos, for when EXIF is wrong or missing.
    ///
    /// The memories cards read capture dates, so a photo whose date is wrong can never take part
    /// until it is corrected. Sidecars are written so the correction is not undone by the next read.
    pub fn set_captured(&mut self, changes: &[(PhotoId, Option<i64>)]) -> Result<usize> {
        let ids: Vec<PhotoId> = changes.iter().map(|(id, _)| *id).collect();
        // Compare against what the catalog holds, so "the date moved" and "the date was already that" are
        // different answers. The old count of the request made a repeat call claim a write that never happened.
        let moved: Vec<(PhotoId, Option<i64>)> = changes
            .iter()
            .copied()
            .filter(|(id, captured)| self.photo(*id).is_some_and(|photo| photo.captured != *captured))
            .collect();
        let changed = self.mutate(Op::Captured { changes: moved })?;
        self.sync_sidecars(&ids);
        Ok(changed)
    }

    /// The absolute path of one photo, if the library knows it.
    pub fn path_of(&self, id: PhotoId) -> Option<std::path::PathBuf> {
        self.catalog
            .as_ref()
            .and_then(|catalog| catalog.photo(id))
            .map(|photo| std::path::Path::new(&photo.root).join(&photo.rel))
    }

    /// What the host should run to show a photo in the file manager or hand it to its application.
    ///
    /// The engine constructs the command and checks the file is still there; running it is the
    /// host's job, because a library opened by the wasm client has no process table to run it in.
    pub fn reveal_target(&self, id: PhotoId, mode: RevealMode) -> Result<sortlens_core::reveal::Command> {
        let desktop =
            sortlens_core::Desktop::current().ok_or_else(|| Error::Invalid("this build has no desktop to show files in".into()))?;
        let photo = self
            .catalog
            .as_ref()
            .and_then(|catalog| catalog.photo(id))
            .ok_or(Error::NoSuchPhoto(id))?;
        let path = std::path::Path::new(&photo.root).join(&photo.rel);
        if !path.exists() {
            return Err(Error::Invalid(format!(
                "{} is not on disk any more; rescan or relocate the folder first",
                photo.name
            )));
        }
        Ok(match mode {
            RevealMode::Reveal => sortlens_core::reveal::reveal(&path, desktop),
            RevealMode::Open => sortlens_core::reveal::open_with_default(&path, desktop),
        })
    }

    /// Resurfacing cards for a given day, defaulting to today.
    ///
    /// `today` exists so the cards can be reproduced and tested; the app passes the clock.
    pub fn memories(&self, today: Option<i64>) -> sortlens_memories::Memories {
        let day = today.unwrap_or_else(sortlens_catalog::now_seconds);
        let candidates: Vec<sortlens_memories::Candidate> = match self.catalog.as_ref() {
            Some(catalog) => catalog
                .photos()
                .filter(|photo| !photo.missing)
                .map(|photo| {
                    sortlens_memories::Candidate::new(
                        photo.id,
                        photo.captured,
                        photo.rating,
                        photo.flag,
                        u64::from(photo.width) * u64::from(photo.height),
                    )
                })
                .collect(),
            None => Vec::new(),
        };
        sortlens_memories::Memories::build(&candidates, day, sortlens_memories::CARD_PHOTOS)
    }

    /// Start counting the library on a worker, so a large library does not freeze the view.
    ///
    /// The photos are cloned once and folded in chunks; progress is published every chunk. Returns
    /// how many photos the pass will cover.
    pub fn begin_insights(&mut self) -> Result<usize> {
        if self.pending_insights.is_some() {
            return Err(Error::Invalid("an insights pass is already running".into()));
        }
        let photos: Vec<Photo> = match self.catalog.as_ref() {
            Some(catalog) => catalog.photos().cloned().collect(),
            None => Vec::new(),
        };
        let total = photos.len();
        let progress = std::sync::Arc::new(InsightsProgress::new());
        let worker_progress = progress.clone();
        let job = self.start_job(JobKind::Insights, format!("Counting · {total} photos"));
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("sortlens-insights".into())
            .spawn(move || {
                let mut accumulator = sortlens_insights::Accumulator::new();
                for (index, photo) in photos.iter().enumerate() {
                    accumulator.push(photo);
                    if index % INSIGHTS_CHUNK == 0 {
                        worker_progress.publish(index + 1);
                    }
                }
                worker_progress.publish(total);
                let _ = tx.send(accumulator.finish());
            })
            .map_err(|error| Error::Invalid(format!("insights worker: {error}")))?;
        self.pending_insights = Some(PendingInsights { rx, progress, job, total });
        Ok(total)
    }

    /// How far the running insights pass has come, if one is running.
    pub fn insights_progress(&self) -> Option<(usize, usize)> {
        self.pending_insights
            .as_ref()
            .map(|pending| (pending.progress.done(), pending.total))
    }

    /// Collect a finished pass. Returns the photo count once it lands.
    pub fn poll_insights(&mut self) -> Option<usize> {
        let pending = self.pending_insights.as_ref()?;
        let insights = match pending.rx.try_recv() {
            Ok(insights) => insights,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                let job = self.pending_insights.take().map(|pending| pending.job);
                if let Some(job) = job {
                    self.finish_job(job, JobStatus::Failed, 0, 0, 0, Some("the pass ended early".into()));
                }
                return None;
            }
        };
        let pending = self.pending_insights.take()?;
        let total = insights.stats.total;
        self.finish_job(pending.job, JobStatus::Done, total, 0, 0, None);
        self.insights_ready = Some(insights);
        Some(total)
    }

    /// Compute suggestions for the whole library on a worker.
    ///
    /// Returns how many photos the pass will look at. **Nothing is written**: the result is a list of
    /// proposals waiting to be read, reviewed and accepted one at a time, exactly like a single photo's
    /// suggestions in the detail view. A batch that wrote keywords by itself would be a bulk edit nobody
    /// asked for, and undo would only take back the last one.
    ///
    /// Only photos whose metadata arrived take part — a photo with no camera, lens, ISO or focal length,
    /// in a folder no pack matches, has nothing to suggest and would only slow the pass down.
    pub fn begin_batch_suggest(&mut self) -> Result<usize> {
        if self.catalog.is_none() {
            return Err(Error::Invalid("no library is open".into()));
        }
        if self.pending_batch.is_some() {
            return Err(Error::Invalid("a suggestion pass is already running".into()));
        }
        let photos: Vec<Photo> = self.all_photos().to_vec();
        let packs = self.enabled_pack_rules();
        let total = photos.len();
        let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_done = done.clone();
        let worker_cancel = cancel.clone();
        let caps = sortlens_ai::Caps::default();
        let job = self.start_job(JobKind::Suggest, format!("Suggesting · {total} photos"));
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("sortlens-suggest".into())
            .spawn(move || {
                let mut out = Vec::new();
                for (index, photo) in photos.iter().enumerate() {
                    if worker_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    let suggestions = suggestions_for(photo, &packs, &caps);
                    if !suggestions.is_empty() {
                        out.push((photo.id, suggestions));
                    }
                    if index % INSIGHTS_CHUNK == 0 {
                        worker_done.store(index + 1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                worker_done.store(total, std::sync::atomic::Ordering::Relaxed);
                let _ = tx.send(out);
            })
            .map_err(|error| Error::Invalid(format!("suggestion worker: {error}")))?;
        self.pending_batch = Some(PendingBatch {
            job,
            rx,
            total,
            done,
            cancel,
        });
        Ok(total)
    }

    /// How far the running suggestion pass has come.
    pub fn batch_progress(&self) -> Option<(usize, usize)> {
        self.pending_batch
            .as_ref()
            .map(|pending| (pending.done.load(std::sync::atomic::Ordering::Relaxed), pending.total))
    }

    /// Stop the running suggestion pass. Nothing is written, so there is nothing to clean up.
    pub fn cancel_batch(&mut self) {
        if let Some(pending) = &self.pending_batch {
            pending.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Collect a finished pass: the photos that have something to suggest, and their proposals.
    pub fn poll_batch(&mut self) -> Option<usize> {
        let pending = self.pending_batch.as_ref()?;
        let found = match pending.rx.try_recv() {
            Ok(found) => found,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                let job = self.pending_batch.take().map(|pending| pending.job);
                if let Some(job) = job {
                    self.finish_job(job, JobStatus::Failed, 0, 0, 0, Some("the pass ended early".into()));
                }
                return None;
            }
        };
        let pending = self.pending_batch.take()?;
        let cancelled = pending.cancel.load(std::sync::atomic::Ordering::Relaxed);
        let count = found.len();
        let status = if cancelled { JobStatus::Cancelled } else { JobStatus::Done };
        let error = cancelled.then(|| "stopped".to_string());
        self.finish_job(pending.job, status, count, 0, 0, error);
        self.batch = found;
        Some(count)
    }

    /// The suggestions from the last finished pass, and nothing while one is running.
    pub fn batch_results(&self) -> &[(PhotoId, Vec<Suggestion>)] {
        &self.batch
    }

    /// Forget the last pass, so the panel does not show proposals forever.
    pub fn clear_batch(&mut self) {
        self.batch.clear();
    }

    /// Groups found by the last finished hash pass.
    pub fn similar_groups(&self) -> &[SimilarGroup] {
        &self.similar
    }

    /// Suggested keywords for one photo, from what the file already says about itself.
    ///
    /// This is the half of tagging that needs no model: the camera, the lens, the focal length band, a
    /// high ISO and a slow shutter are all in the EXIF. Every suggestion is a proposal — nothing is
    /// written until [`Engine::accept_suggestion`] is called.
    pub fn suggestions(&self, id: PhotoId) -> Vec<Suggestion> {
        let packs = self.enabled_pack_rules();
        match self.photo(id) {
            Some(photo) => suggestions_for(photo, &packs, &sortlens_ai::Caps::default()),
            None => Vec::new(),
        }
    }

    /// The rules of every switched-on pack, parsed and ready. Unreadable packs are simply absent here:
    /// [`Engine::packs`] is where their problem is reported.
    fn enabled_pack_rules(&self) -> Vec<sortlens_ai::Pack> {
        self.packs()
            .into_iter()
            .filter(|summary| summary.enabled)
            .filter_map(|summary| summary.pack)
            .collect()
    }

    /// One keyword pack as the library sees it: what it is, and whether it is switched on.
    ///
    /// A pack whose file could not be read is reported with its problem rather than being dropped —
    /// someone who copied a pack with a typo needs to be told which file and what is wrong.
    pub fn packs(&self) -> Vec<PackSummary> {
        let enabled = self
            .catalog
            .as_ref()
            .map(|catalog| catalog.enabled_packs().clone())
            .unwrap_or_default();
        load_packs(self.dir.as_deref(), &enabled)
    }

    /// Switch a keyword pack on or off, through the op log so it persists and undoes.
    pub fn set_pack(&mut self, name: &str, on: bool) -> Result<bool> {
        let catalog = self.catalog.as_mut().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        catalog.set_pack(name, on)
    }

    /// Suggested keywords for several photos at once, for a panel that shows a selection.
    pub fn suggestions_for_all(&self, ids: &[PhotoId]) -> Vec<(PhotoId, Vec<Suggestion>)> {
        let packs = self.enabled_pack_rules();
        ids.iter()
            .filter_map(|id| {
                self.photo(*id)
                    .map(|photo| (*id, suggestions_for(photo, &packs, &sortlens_ai::Caps::default())))
            })
            .collect()
    }

    /// Accept one suggestion: add the keyword, through the same op every other tag uses.
    ///
    /// Returns whether anything changed, so the caller can say "already there" rather than reporting a
    /// tag it did not add.
    pub fn accept_suggestion(&mut self, id: PhotoId, tag: &str) -> Result<bool> {
        let photo = self.photo(id).ok_or(Error::NoSuchPhoto(id))?;
        let normalised = tag.trim().to_lowercase();
        if normalised.is_empty() {
            return Err(Error::Invalid("a suggestion needs a keyword".into()));
        }
        if photo.tags.iter().any(|existing| existing == &normalised) {
            return Ok(false);
        }
        let changed = self.set_tag(&[id], &normalised, true)?;
        Ok(changed > 0)
    }

    /// Start writing photos out to `dest`.
    ///
    /// The plan is built before anything is written, so a name clash is decided up front and the
    /// caller can be told how many files will be skipped before a byte moves. Returns the number of
    /// files the plan will write.
    pub fn begin_export(&mut self, dest: &Path, ids: &[PhotoId], spec: &sortlens_export::Spec) -> Result<usize> {
        if self.catalog.is_none() {
            return Err(Error::Invalid("no library is open".into()));
        }
        if self.pending_export.is_some() {
            return Err(Error::Invalid("an export is already running".into()));
        }
        spec.check()?;
        if dest.as_os_str().is_empty() {
            return Err(Error::Invalid("an export needs a destination folder".into()));
        }
        if !dest.is_absolute() {
            return Err(Error::Invalid(format!("{} is not an absolute path", dest.display())));
        }
        let mut sources = Vec::with_capacity(ids.len());
        for id in ids {
            let photo = self.photo(*id).ok_or(Error::NoSuchPhoto(*id))?;
            let path = PathBuf::from(photo.path());
            if !path.exists() {
                return Err(Error::Invalid(format!(
                    "{} is not on disk any more; rescan or relocate the folder first",
                    photo.rel
                )));
            }
            sources.push(path);
        }
        let existing = sortlens_export::existing_names(dest);
        let jobs: Vec<sortlens_export::Job> = sortlens_export::plan(spec, &sources, dest, &existing);
        let writing = jobs.iter().filter(|job| job.skip.is_none()).count();
        let skipped = jobs.len() - writing;
        if writing == 0 {
            // Nothing to write is a result, not a job: say so instead of showing a bar that never moves.
            let mut done = sortlens_export::Report {
                dest: dest.to_path_buf(),
                skipped,
                ..sortlens_export::Report::default()
            };
            done.written = 0;
            self.last_export = Some(done);
            return Ok(0);
        }

        let spec = spec.clone();
        let dest = dest.to_path_buf();
        let (tx, rx) = channel();
        let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_done = Arc::clone(&done);
        let worker_cancel = Arc::clone(&cancel);
        let total = jobs.len();
        std::thread::Builder::new()
            .name("sortlens-export".into())
            .spawn(move || {
                let mut report = sortlens_export::Report {
                    dest,
                    skipped,
                    ..sortlens_export::Report::default()
                };
                for job in &jobs {
                    if worker_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    match sortlens_export::export_one(&spec, job) {
                        Ok(_) if job.skip.is_some() => {}
                        Ok(bytes) => {
                            report.written += 1;
                            report.bytes += bytes;
                        }
                        Err(error) => report.failures.push(format!(
                            "{}: {error}",
                            job.source
                                .file_name()
                                .map(|name| name.to_string_lossy().to_string())
                                .unwrap_or_default()
                        )),
                    }
                    worker_done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let _ = tx.send(report);
            })
            .map_err(|error| Error::Invalid(format!("could not start the export worker: {error}")))?;

        let job = self.start_job(JobKind::Export, format!("{writing} files"));
        self.pending_export = Some(PendingExport {
            job,
            rx,
            total,
            done,
            cancel,
        });
        Ok(writing)
    }

    /// How far a running export got: files dealt with, and how many there were.
    pub fn export_progress(&self) -> Option<(usize, usize)> {
        self.pending_export
            .as_ref()
            .map(|pending| (pending.done.load(std::sync::atomic::Ordering::Relaxed), pending.total))
    }

    /// Stop a running export. Files already written stay written.
    pub fn cancel_export(&mut self) {
        if let Some(pending) = &self.pending_export {
            pending.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Take the report of a finished export, or `None` while it is still running.
    pub fn poll_export(&mut self) -> Option<sortlens_export::Report> {
        let pending = self.pending_export.as_ref()?;
        let report = match pending.rx.try_recv() {
            Ok(report) => report,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                self.pending_export = None;
                return None;
            }
        };
        let pending = self.pending_export.take()?;
        let cancelled = pending.cancel.load(std::sync::atomic::Ordering::Relaxed);
        let status = if cancelled {
            JobStatus::Cancelled
        } else if report.is_clean() {
            JobStatus::Done
        } else {
            JobStatus::Failed
        };
        let error = report.failures.first().cloned();
        self.finish_job(pending.job, status, report.written, 0, report.failures.len(), error);
        self.last_export = Some(report.clone());
        Some(report)
    }

    /// The most recent export, for the dialog's receipt.
    pub fn last_export(&self) -> Option<&sortlens_export::Report> {
        self.last_export.as_ref()
    }

    /// Start hashing every photo, reusing hashes already computed for unchanged files.
    ///
    /// Returns the number of photos that still need a hash: zero means the groups are already
    /// up to date and no job was started.
    /// Measure sharpness across the library, on a worker.
    ///
    /// Only photos that need it are queued: a photo with no score, a photo whose score came from another
    /// `SCORE_VERSION`, or a photo whose stored score is not a number. That predicate lives in `sortlens-quality`, so
    /// bumping the version re-measures the library instead of mixing two scales in one sort — and re-running a scan
    /// over an unchanged library queues nothing and writes nothing.
    ///
    /// Returns how many photos the pass will look at.
    pub fn begin_sharpness(&mut self) -> Result<usize> {
        if self.catalog.is_none() {
            return Err(Error::Invalid("no library is open".into()));
        }
        if self.pending_sharpness.is_some() {
            return Err(Error::Invalid("a sharpness pass is already running".into()));
        }
        let mut todo: Vec<(PhotoId, String)> = Vec::new();
        for photo in self.all_photos() {
            if photo.missing || !sortlens_quality::needs_rescore(photo.sharpness.as_ref()) {
                continue;
            }
            todo.push((photo.id, photo.path()));
        }
        if todo.is_empty() {
            return Ok(0);
        }

        let (tx, rx) = channel();
        let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let unjudged = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_done = Arc::clone(&done);
        let worker_unjudged = Arc::clone(&unjudged);
        let worker_cancel = Arc::clone(&cancel);
        let total = todo.len();
        std::thread::Builder::new()
            .name("sortlens-sharpness".into())
            .spawn(move || {
                let mut measured = Vec::with_capacity(todo.len());
                for (id, path) in todo {
                    if worker_cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    let score = sortlens_thumbs::decode_file(std::path::Path::new(&path), sortlens_thumbs::LARGE)
                        .ok()
                        .and_then(|thumbnail| sortlens_quality::sharpness(&thumbnail.rgba, thumbnail.width, thumbnail.height));
                    match score {
                        Some(score) => measured.push((id, score)),
                        // A file this build cannot decode, or a frame too dark to judge: counted, never scored.
                        None => {
                            worker_unjudged.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    worker_done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                let _ = tx.send(measured);
            })
            .map_err(|error| Error::Invalid(format!("could not start the sharpness worker: {error}")))?;

        let job = self.start_job(JobKind::Sharpness, format!("{total} photos"));
        self.pending_sharpness = Some(PendingSharpness {
            job,
            rx,
            total,
            done,
            unjudged,
            cancel,
        });
        Ok(total)
    }

    /// How far a running sharpness pass has come: how many photos it has looked at, and how many it will look at.
    pub fn sharpness_progress(&self) -> Option<(usize, usize)> {
        self.pending_sharpness
            .as_ref()
            .map(|pending| (pending.done.load(std::sync::atomic::Ordering::Relaxed), pending.total))
    }

    /// Stop the running sharpness pass. What it already measured is still committed, because each score is true on
    /// its own; only the photographs it never reached stay unmeasured.
    pub fn cancel_sharpness(&mut self) {
        if let Some(pending) = &self.pending_sharpness {
            pending.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Collect a finished sharpness pass, storing every score as one undoable op.
    pub fn poll_sharpness(&mut self) -> Option<SharpnessReport> {
        let pending = self.pending_sharpness.as_ref()?;
        let measured = match pending.rx.try_recv() {
            Ok(measured) => measured,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                let job = self.pending_sharpness.take().map(|pending| pending.job);
                if let Some(job) = job {
                    self.finish_job(job, JobStatus::Failed, 0, 0, 0, Some("the pass ended early".into()));
                }
                return None;
            }
        };
        let pending = self.pending_sharpness.take()?;
        let unjudged = pending.unjudged.load(std::sync::atomic::Ordering::Relaxed);
        let looked_at = pending.done.load(std::sync::atomic::Ordering::Relaxed);
        let remaining = pending.total.saturating_sub(looked_at);
        let stored = match self.catalog.as_mut() {
            Some(catalog) => catalog.set_scores(&measured).unwrap_or(0),
            None => 0,
        };
        if remaining > 0 {
            let message = format!("cancelled after {looked_at} of {} photos", pending.total);
            self.finish_job(pending.job, JobStatus::Cancelled, looked_at, 0, unjudged, Some(message));
        } else {
            self.finish_job(pending.job, JobStatus::Done, looked_at, 0, unjudged, None);
        }
        Some(SharpnessReport {
            measured: stored,
            unjudged,
            remaining,
        })
    }

    /// The gazetteer file, whether or not it exists.
    ///
    /// `places.txt` beside the catalog, in the same spirit as the tile folder: a file the person supplies, parsed by a
    /// documented format, with nothing shipped and nothing fetched. It is *not* loaded here — a gazetteer can be tens
    /// of megabytes and a view must not parse it to draw a frame.
    pub fn places_file(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|dir| dir.join("places.txt"))
    }

    /// Read the gazetteer, if the library has one.
    pub fn load_gazetteer(&self) -> Option<Result<sortlens_geo::Gazetteer, sortlens_geo::GazetteerError>> {
        let path = self.places_file()?;
        if !path.exists() {
            return None;
        }
        Some(sortlens_geo::Gazetteer::load(&path))
    }

    /// The basemap, if the library has one.
    ///
    /// The tile folder is `tiles/` beside the catalog, so a basemap is part of the library rather than a global setting:
    /// two libraries can have different basemaps, and moving a library brings its map with it. `None` means there is no
    /// folder; an `Err` means there is one and it cannot be used, and the message says why.
    pub fn tileset(&self) -> Option<Result<sortlens_geo::tiles::Tileset, sortlens_geo::tiles::TileError>> {
        let dir = self.dir.as_ref()?.join("tiles");
        if !dir.exists() {
            return None;
        }
        Some(sortlens_geo::tiles::Tileset::open(&dir))
    }

    /// Where the tile folder would go, whether or not there is one.
    pub fn tiles_dir(&self) -> Option<PathBuf> {
        self.dir.as_ref().map(|dir| dir.join("tiles"))
    }

    /// Every photo that has a location, with the coordinate.
    ///
    /// Unlocated photos are left out rather than given a placeholder: "we do not know where this was" is not a place,
    /// and a map that averaged it into one would be wrong about all of them.
    pub fn located(&self) -> Vec<(PhotoId, sortlens_geo::LatLon)> {
        self.all_photos()
            .into_iter()
            .filter(|photo| !photo.missing)
            .filter_map(|photo| photo.location.map(|location| (photo.id, location)))
            .collect()
    }

    /// How many photos have a location at all.
    pub fn geotagged(&self) -> usize {
        self.all_photos()
            .into_iter()
            .filter(|photo| !photo.missing && photo.location.is_some())
            .count()
    }

    /// The photos grouped into map dots at a zoom level.
    ///
    /// Deterministic by construction (`sortlens_geo::cluster` sorts its cells), so a view can call this every frame
    /// and get the same dots.
    pub fn places(&self, zoom: u8) -> Vec<sortlens_geo::Cluster> {
        sortlens_geo::cluster(&self.located(), zoom)
    }

    /// How many photos have no usable sharpness score yet.
    ///
    /// The window asks this to decide whether "Measure sharpness" is a job worth offering, so the wording is
    /// "has no usable score" rather than "has no score": a score from an older `SCORE_VERSION` is not usable and
    /// counts here.
    pub fn sharpness_todo(&self) -> usize {
        self.all_photos()
            .into_iter()
            .filter(|photo| !photo.missing && sortlens_quality::needs_rescore(photo.sharpness.as_ref()))
            .count()
    }

    /// Every stored score, paired with its photo, in photo order.
    pub fn scores(&self) -> Vec<(PhotoId, sortlens_quality::Score)> {
        self.all_photos()
            .into_iter()
            .filter_map(|photo| photo.sharpness.map(|score| (photo.id, score)))
            .collect()
    }

    /// A photo's stored sharpness, if it has been measured with the current algorithm.
    pub fn sharpness(&self, id: PhotoId) -> Option<sortlens_quality::Score> {
        self.catalog.as_ref()?.sharpness(id)
    }

    /// Store measured scores directly. Used by the CLI and by tests; the window goes through the worker.
    pub fn set_scores(&mut self, measured: &[(PhotoId, sortlens_quality::Score)]) -> Result<usize> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .set_scores(measured)
    }

    /// Forget every stored score, as one undoable op.
    pub fn clear_scores(&mut self) -> Result<usize> {
        self.catalog
            .as_mut()
            .ok_or_else(|| Error::Invalid("no library is open".into()))?
            .clear_scores()
    }

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

    /// Regroup the cached members under another set of signals, without touching the files.
    pub fn regroup(&mut self, threshold: u32, colour: u32, mode: SimilarMatch) {
        self.similar = sortlens_similar::group_in(&self.similar_members, mode, threshold, colour);
    }

    /// Fold finished hashes into the cache and regroup. Returns the number of groups when the pass
    /// just finished.
    pub fn poll_similar(&mut self, threshold: u32) -> Option<usize> {
        self.poll_similar_with(threshold, SIMILAR_COLOUR)
    }

    /// Poll with an explicit colour tolerance and the default set of signals.
    pub fn poll_similar_with(&mut self, threshold: u32, colour: u32) -> Option<usize> {
        self.poll_similar_in(threshold, colour, SimilarMatch::Families)
    }

    /// Poll, regrouping under an explicit set of signals.
    ///
    /// The hashes and palettes are already cached, so changing the mode is a regrouping rather than
    /// another pass over the files.
    pub fn poll_similar_in(&mut self, threshold: u32, colour: u32, mode: SimilarMatch) -> Option<usize> {
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
        self.similar = sortlens_similar::group_in(&self.similar_members, mode, threshold, colour);
        Some(self.similar.len())
    }

    /// Groups around one photo, for "find similar to this".
    pub fn similar_to(&self, id: PhotoId, threshold: u32) -> Option<SimilarGroup> {
        sortlens_similar::around(&self.similar_members, id, threshold)
    }

    /// The keyword registry: counts, the names each keyword swallowed, and aliases pointing nowhere.
    pub fn tag_registry(&self) -> sortlens_catalog::registry::Registry {
        match self.catalog.as_ref() {
            Some(catalog) => catalog.registry(),
            None => sortlens_catalog::registry::Registry::default(),
        }
    }

    /// What a typed keyword really means, after aliases.
    pub fn resolve_tag(&self, tag: &str) -> String {
        let normalised = tag.trim().to_lowercase();
        match self.catalog.as_ref() {
            Some(catalog) => catalog.resolve_tag(&normalised),
            None => normalised,
        }
    }

    /// The aliases that resolve to one keyword, for the sidebar hint.
    pub fn registry_aliases_of(&self, tag: &str) -> Vec<String> {
        match self.catalog.as_ref() {
            Some(catalog) => catalog.aliases_of(tag),
            None => Vec::new(),
        }
    }

    /// Link a keyword to another, or forget the link when `canonical` is `None`.
    pub fn set_alias(&mut self, alias: &str, canonical: Option<&str>) -> Result<bool> {
        let catalog = self.catalog.as_mut().ok_or_else(|| Error::Invalid("no library is open".into()))?;
        // A renamed keyword must keep resolving forward, so the target is resolved first.
        let canonical = canonical.map(|name| catalog.resolve_tag(&name.trim().to_lowercase()));
        catalog.set_alias(alias, canonical.as_deref())
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

        let picks = flags.iter().filter(|(_, flag)| *flag == Flag::Pick).count();
        let report = self.apply_candidates(mode, candidates, flags, previous_flags, picks)?;
        self.cull = None;
        self.persist_cull();
        Ok(report)
    }

    /// The part of a delete that is the same whoever asked for it: audit, apply, record, notify.
    ///
    /// Split out of [`Engine::apply_session`] so a cull batch and a set chosen in the similarity view cannot drift
    /// apart. The caller keeps the session bookkeeping; everything that touches the disk and the log is here, and
    /// there is exactly one audit.
    fn apply_candidates(
        &mut self,
        mode: sortlens_cull::trash::Mode,
        candidates: Vec<sortlens_cull::trash::Candidate>,
        flags: Vec<(PhotoId, Flag)>,
        previous_flags: Vec<(PhotoId, Flag)>,
        picks: usize,
    ) -> Result<sortlens_cull::trash::Report> {
        let mode_id = mode.id();
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
        Ok(report)
    }

    /// Move photos to the trash, the rejects folder, or tag them — without a cull session.
    ///
    /// This is the same executor a cull batch uses, deliberately: the audit that refuses a file whose bytes moved
    /// since it was indexed, the batch record the rejects bin restores from, and the `Flag::Reject` that makes an
    /// undo put the review state back all live in one place rather than in a second delete path. Two ways to delete
    /// would be two places for the safety check to be wrong.
    ///
    /// Nothing is deleted until every candidate passes the audit: a batch with one changed file is refused whole,
    /// because deleting the other four and reporting "3 of 4" leaves the library describing a disk nobody chose.
    pub fn delete_photos(&mut self, ids: &[PhotoId], mode: sortlens_cull::trash::Mode) -> Result<sortlens_cull::trash::Report> {
        if ids.is_empty() {
            return Err(Error::Invalid("no photos were chosen".into()));
        }
        let mut candidates = Vec::with_capacity(ids.len());
        let mut flags = Vec::with_capacity(ids.len());
        let mut previous_flags = Vec::with_capacity(ids.len());
        for id in ids {
            let photo = self.photo(*id).ok_or(Error::NoSuchPhoto(*id))?;
            previous_flags.push((*id, photo.flag));
            flags.push((*id, Flag::Reject));
            candidates.push(sortlens_cull::trash::Candidate {
                id: *id,
                path: PathBuf::from(&photo.root).join(&photo.rel),
                root: PathBuf::from(&photo.root),
                size: photo.size,
                modified: photo.modified,
            });
        }
        self.apply_candidates(mode, candidates, flags, previous_flags, 0)
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
        let changes: Vec<(PhotoId, Option<ColorLabel>)> = ids
            .iter()
            .filter(|id| self.photo(**id).is_some_and(|photo| photo.label != label))
            .map(|id| (*id, label))
            .collect();
        let changed = self.mutate(Op::Label { changes })?;
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

    /// Redo the last undone catalog operation.
    pub fn redo(&mut self) -> Result<bool> {
        match self.catalog.as_mut() {
            Some(catalog) => catalog.redo(),
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
        // The caller has already narrowed `changes` to what actually differs, so this count is "photos the
        // write touched" rather than "photos the caller named".
        let count = match &op {
            Op::Rate { changes } => changes.len(),
            Op::Flag { changes } => changes.len(),
            Op::Label { changes } => changes.len(),
            Op::Tagged { changes } => changes.len(),
            Op::Captured { changes } => changes.len(),
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
        // Optimistic concurrency (SL-D11): a caller that read the library at revision `expected`
        // passes it as `expect` and is refused the moment the catalog has moved on, instead of
        // silently overwriting newer annotations. A stale render can re-read and retry; a wrong
        // write cannot be undone by the person whose tags it lost.
        if let Some(expected) = args.get("expect").and_then(Value::as_u64) {
            let current = self.revision();
            if current != expected {
                return Err(Error::StaleRevision { expected, current });
            }
        }
        let response = match command {
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
                // The sort was hard-coded to Natural, so a caller could not walk the library by
                // rating or date even though every other paging path can.
                let sort = Sort::parse(args.get("sort").and_then(Value::as_str).unwrap_or_default());
                let page = self.page(&filter, sort, offset, limit);
                Ok(json!({
                    "total": page.total,
                    "offset": page.offset,
                    "sort": sort.id(),
                    "photos": page.photos,
                }))
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
            "photo.tagFromPrev" => {
                let ids = ids_from(args)?;
                Ok(json!({ "changed": self.tag_from_prev(&ids)?, "tag": self.last_tag() }))
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
            "folder.mark" => {
                let path = args
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a folder path is required".into()))?;
                let current = self.folder_mark(&sortlens_catalog::folder::key_of(path));
                let favourite = args.get("favourite").and_then(Value::as_bool).unwrap_or(current.favourite);
                let colour = match args.get("colour") {
                    // Absent keeps the current colour; an explicit null clears it.
                    None => current.colour,
                    Some(Value::Null) => None,
                    Some(Value::String(name)) if name == "none" => None,
                    Some(Value::String(name)) => Some(
                        ColorLabel::ALL
                            .into_iter()
                            .find(|label| label.id() == name)
                            .ok_or_else(|| Error::Invalid(format!("{name} is not a colour label")))?,
                    ),
                    Some(_) => return Err(Error::Invalid("colour must be a name or null".into())),
                };
                let sort = match args.get("sort") {
                    None => current.sort,
                    Some(Value::Null) => None,
                    Some(Value::String(id)) if id == "none" => None,
                    Some(Value::String(id)) => {
                        if !Sort::ALL.into_iter().any(|sort| sort.id() == id) {
                            return Err(Error::Invalid(format!("{id} is not a sort order")));
                        }
                        Some(id.clone())
                    }
                    Some(_) => return Err(Error::Invalid("sort must be an id or null".into())),
                };
                let search = match args.get("search") {
                    None => current.search,
                    Some(Value::Null) => None,
                    Some(Value::String(text)) => {
                        let text = text.trim();
                        if text.is_empty() { None } else { Some(text.to_string()) }
                    }
                    Some(_) => return Err(Error::Invalid("search must be text or null".into())),
                };
                let mark = sortlens_catalog::folder::FolderMark {
                    favourite,
                    colour,
                    sort,
                    search,
                };
                Ok(json!({
                    "changed": self.set_folder_mark(path, mark.clone())?,
                    "key": sortlens_catalog::folder::key_of(path),
                    "favourite": mark.favourite,
                    "colour": mark.colour.map(|label| label.id()),
                    "sort": mark.sort,
                    "search": mark.search,
                }))
            }
            "folder.marks" => {
                let marks: Vec<Value> = self
                    .folder_marks()
                    .into_iter()
                    .map(|(key, mark)| {
                        json!({
                            "key": key,
                            "favourite": mark.favourite,
                            "colour": mark.colour.map(|label| label.id()),
                            "sort": mark.sort,
                            "search": mark.search,
                        })
                    })
                    .collect();
                Ok(json!({ "folders": marks }))
            }
            "collection.colour" => {
                let ids = collection_ids_from(args)?;
                let colour = match args.get("colour").and_then(Value::as_str) {
                    None | Some("") | Some("none") => None,
                    Some(name) => Some(
                        ColorLabel::ALL
                            .into_iter()
                            .find(|label| label.id() == name)
                            .ok_or_else(|| Error::Invalid(format!("{name} is not a colour label")))?,
                    ),
                };
                let changed = self.set_collections_colour(&ids, colour)?;
                Ok(json!({ "colour": colour.map(|label| label.id()), "changed": changed }))
            }
            "collection.favourite" => {
                let ids = collection_ids_from(args)?;
                let favourite = args.get("favourite").and_then(Value::as_bool).unwrap_or(true);
                let changed = self.set_collections_favourite(&ids, favourite)?;
                Ok(json!({ "favourite": favourite, "changed": changed }))
            }
            "collection.delete" => {
                let ids = collection_ids_from(args)?;
                Ok(json!({ "deleted": self.delete_collections(&ids)? }))
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
            "tag.delete" => {
                let tag = args
                    .get("tag")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a keyword to delete is required".into()))?;
                Ok(json!({ "deleted": self.delete_tag(tag)? }))
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
                let colour = args.get("colour").and_then(Value::as_u64).unwrap_or(u64::from(SIMILAR_COLOUR)) as u32;
                let mode = SimilarMatch::parse(args.get("mode").and_then(Value::as_str).unwrap_or("families"));
                if self.poll_similar_in(threshold, colour, mode).is_none() {
                    self.regroup(threshold, colour, mode);
                }
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
            "view.detail" => {
                self.last_effect = Effect::ShowView("detail");
                Ok(json!({ "view": "detail" }))
            }
            "view.similar" => {
                self.last_effect = Effect::ShowView("similar");
                let groups = self.similar_groups().len();
                Ok(json!({ "view": "similar", "groups": groups }))
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
                "byMonth": insights.by_month,
                "peakMonth": insights.peak_month(),
                "peakSeason": insights.peak_season(),
                "busiestDay": insights.busiest_day.map(|day| json!({ "at": day.at, "count": day.count })),
                "highlights": insights.highlights().iter().map(|highlight| json!({ "label": highlight.label, "value": highlight.value })).collect::<Vec<_>>(),
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
            "library.readOnly" => Ok(json!({ "readOnly": self.read_only() })),
            "library.roots" => {
                let roots: Vec<Value> = self
                    .counts_by_root()
                    .into_iter()
                    .map(|(root, photos)| json!({ "root": root, "photos": photos }))
                    .collect();
                Ok(json!({ "roots": roots }))
            }
            "tag.keywords" => {
                let keywords: Vec<Value> = self
                    .tag_vocabulary()
                    .into_iter()
                    .map(|(tag, photos)| json!({ "tag": tag, "photos": photos }))
                    .collect();
                Ok(json!({ "keywords": keywords, "count": keywords.len() }))
            }
            "tag.registry" => {
                let registry = self.tag_registry();
                Ok(json!({
                    "headline": registry.headline(),
                    "entries": registry.entries.iter().map(|entry| json!({
                        "name": entry.name,
                        "count": entry.count,
                        "aliases": entry.aliases,
                    })).collect::<Vec<_>>(),
                    "orphans": registry.orphans.iter().map(|(alias, canonical)| json!({
                        "alias": alias,
                        "canonical": canonical,
                    })).collect::<Vec<_>>(),
                }))
            }
            "tag.alias" => {
                let alias = args
                    .get("alias")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a keyword is required".into()))?;
                let canonical = args.get("canonical").and_then(Value::as_str);
                Ok(json!({ "changed": self.set_alias(alias, canonical)? }))
            }
            "edit.movePicks" => {
                let dest = args
                    .get("dest")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a destination folder is required".into()))?;
                let ids: Vec<PhotoId> = args
                    .get("ids")
                    .and_then(Value::as_array)
                    .map(|values| values.iter().filter_map(Value::as_u64).map(PhotoId).collect())
                    .unwrap_or_default();
                Ok(json!({ "moved": self.move_picks(dest, &ids)? }))
            }
            "edit.undoMove" => {
                let id = args
                    .get("id")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| Error::Invalid("a move id is required".into()))?;
                Ok(json!({ "restored": self.undo_move(id)? }))
            }
            "suggest.start" => {
                let planned = self.begin_batch_suggest()?;
                Ok(json!({ "photos": planned }))
            }
            "suggest.progress" => {
                let progress = self.batch_progress();
                Ok(json!({
                    "running": progress.is_some(),
                    "done": progress.map(|(done, _)| done),
                    "total": progress.map(|(_, total)| total),
                }))
            }
            "suggest.cancel" => {
                self.cancel_batch();
                Ok(json!({ "cancelled": true }))
            }
            "suggest.results" => {
                let done = self.poll_batch();
                let results: Vec<Value> = self
                    .batch_results()
                    .iter()
                    .map(|(id, suggestions)| {
                        json!({
                            "id": id.0,
                            "suggestions": suggestions
                                .iter()
                                .map(|suggestion| json!({
                                    "tag": suggestion.tag,
                                    "score": f64::from(suggestion.score),
                                    "source": suggestion.source.label(),
                                }))
                                .collect::<Vec<Value>>(),
                        })
                    })
                    .collect();
                Ok(json!({ "finished": done, "photos": results, "count": results.len() }))
            }
            "suggest.clear" => {
                self.clear_batch();
                Ok(json!({ "cleared": true }))
            }
            "packs.list" => {
                let packs: Vec<Value> = self
                    .packs()
                    .iter()
                    .map(|pack| {
                        json!({
                            "name": pack.name,
                            "file": pack.file,
                            "licence": pack.licence,
                            "keywords": pack.keywords,
                            "enabled": pack.enabled,
                            "problem": pack.problem,
                        })
                    })
                    .collect();
                let folder = self.dir.as_ref().map(|dir| dir.join(PACKS_DIR).display().to_string());
                Ok(json!({ "packs": packs, "folder": folder }))
            }
            "packs.set" => {
                let name = args
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a pack name is required".into()))?;
                let on = args
                    .get("on")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| Error::Invalid("on is required, true or false".into()))?;
                Ok(json!({ "changed": self.set_pack(name, on)? }))
            }
            "suggest.list" => {
                let ids = ids_from(args)?;
                let photos: Vec<Value> = self
                    .suggestions_for_all(&ids)
                    .into_iter()
                    .map(|(id, suggestions)| {
                        json!({
                            "id": id.0,
                            "suggestions": suggestions
                                .iter()
                                .map(|suggestion| json!({
                                    "tag": suggestion.tag,
                                    "score": f64::from(suggestion.score),
                                    "source": suggestion.source.label(),
                                }))
                                .collect::<Vec<Value>>(),
                        })
                    })
                    .collect();
                Ok(json!({ "photos": photos }))
            }
            "suggest.accept" => {
                let id = args
                    .get("id")
                    .and_then(Value::as_u64)
                    .map(PhotoId)
                    .ok_or_else(|| Error::Invalid("a photo id is required".into()))?;
                let tag = args
                    .get("tag")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a keyword is required".into()))?;
                Ok(json!({ "tagged": self.accept_suggestion(id, tag)? }))
            }
            "edit.export" => {
                let dest = args
                    .get("dest")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::Invalid("a destination folder is required".into()))?;
                let ids = ids_from(args)?;
                let mut spec = sortlens_export::Spec::default();
                if let Some(format) = args.get("format").and_then(Value::as_str) {
                    spec.format =
                        sortlens_export::Format::parse(format).ok_or_else(|| Error::Invalid(format!("there is no {format} export")))?;
                }
                if let Some(edge) = args.get("maxEdge").and_then(Value::as_u64) {
                    spec.max_edge = Some(u32::try_from(edge).map_err(|_| Error::Invalid("that edge is out of range".into()))?);
                }
                if let Some(quality) = args.get("quality").and_then(Value::as_u64) {
                    spec.quality = u8::try_from(quality).map_err(|_| Error::Invalid("that quality is out of range".into()))?;
                }
                if let Some(metadata) = args.get("metadata").and_then(Value::as_bool) {
                    spec.metadata = metadata;
                }
                if let Some(collision) = args.get("collision").and_then(Value::as_str) {
                    spec.collision = sortlens_export::Collision::parse(collision)
                        .ok_or_else(|| Error::Invalid(format!("there is no {collision} collision policy")))?;
                }
                let planned = self.begin_export(Path::new(dest), &ids, &spec)?;
                Ok(json!({
                    "dest": dest,
                    "planned": planned,
                    "summary": spec.summary(),
                }))
            }
            "edit.cancelExport" => {
                self.cancel_export();
                Ok(json!({ "cancelled": true }))
            }
            "edit.moves" => {
                let receipts: Vec<Value> = self
                    .moves
                    .iter()
                    .rev()
                    .map(|receipt| {
                        json!({
                            "id": receipt.id,
                            "dest": receipt.dest,
                            "at": receipt.at,
                            "count": receipt.len(),
                            "headline": receipt.headline(),
                        })
                    })
                    .collect();
                Ok(json!({ "moves": receipts }))
            }
            "photo.sharpness" => Ok(json!({ "queued": self.begin_sharpness()? })),
            "photo.sharpnessProgress" => {
                let progress = self.sharpness_progress();
                let report = self.poll_sharpness();
                Ok(json!({
                    "running": progress.is_some(),
                    "done": progress.map(|(done, _)| done).unwrap_or(0),
                    "total": progress.map(|(_, total)| total).unwrap_or(0),
                    "measured": report.map(|report| report.measured),
                    "unjudged": report.map(|report| report.unjudged),
                }))
            }
            "insights.start" => Ok(json!({ "queued": self.begin_insights()? })),
            "insights.progress" => {
                let progress = self.insights_progress();
                let _ = self.poll_insights();
                Ok(json!({
                    "running": progress.is_some(),
                    "done": progress.map(|(done, _)| done).unwrap_or(0),
                    "total": progress.map(|(_, total)| total).unwrap_or(0),
                }))
            }
            "library.changed" => {
                let health = self.check_health()?;
                Ok(json!({
                    "checked": health.checked,
                    "present": health.present,
                    "missing": health.missing,
                    "modified": health.modified,
                    "ids": health.modified_ids,
                }))
            }
            "library.refreshChanged" => Ok(json!({ "refreshed": self.refresh_changed()? })),
            "library.retryMetadata" => {
                let all = args.get("all").and_then(Value::as_bool).unwrap_or(false);
                let (updated, remaining) = self.retry_metadata(all)?;
                Ok(json!({ "updated": updated, "remaining": remaining }))
            }
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
            "photo.delete" => {
                let ids = ids_from(args)?;
                if ids.is_empty() {
                    return Err(Error::Invalid("photo.delete needs ids".into()));
                }
                let mode = delete_mode_from(args)?;
                let report = self.delete_photos(&ids, mode)?;
                Ok(json!({
                    "applied": report.applied.len(),
                    "failed": report.failed.len(),
                }))
            }
            "cull.apply" => {
                let mode = delete_mode_from(args)?;
                let report = self.cull_apply(mode)?;
                Ok(json!({
                    "applied": report.applied.len(),
                    "failed": report.failed.len(),
                }))
            }
            "edit.undo" => Ok(json!({ "undone": self.undo()? })),
            "edit.redo" => Ok(json!({ "redone": self.redo()? })),
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
            "photo.reveal" | "photo.open" => {
                let id = args
                    .get("id")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| Error::Invalid("an id is required".into()))?;
                let mode = if command == "photo.reveal" {
                    RevealMode::Reveal
                } else {
                    RevealMode::Open
                };
                let built = self.reveal_target(PhotoId(id), mode)?;
                // The host runs it. `ran` is honest about that: a caller that was expecting the file
                // to open should look at it before believing anything happened.
                Ok(json!({
                    "program": built.program,
                    "args": built.args,
                    "line": built.display(),
                    "ran": false,
                    "selects": sortlens_core::reveal::reveal_selects(
                        sortlens_core::Desktop::current().unwrap_or(sortlens_core::Desktop::Linux)
                    ),
                }))
            }
            "photo.setCaptured" => {
                let changes: Vec<(PhotoId, Option<i64>)> = args
                    .get("changes")
                    .and_then(Value::as_array)
                    .ok_or_else(|| Error::Invalid("changes are required".into()))?
                    .iter()
                    .map(|entry| {
                        let id = entry
                            .get("id")
                            .and_then(Value::as_u64)
                            .ok_or_else(|| Error::Invalid("each change needs an id".into()))?;
                        let captured = entry.get("captured").and_then(Value::as_i64);
                        Ok((PhotoId(id), captured))
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(json!({ "changed": self.set_captured(&changes)? }))
            }
            "memories.list" => {
                let today = args.get("today").and_then(Value::as_i64);
                let memories = self.memories(today);
                let mut cards = Vec::new();
                for card in &memories.cards {
                    cards.push(json!({
                        "kind": card.kind.id(),
                        "title": card.title,
                        "detail": card.detail(),
                        "matched": card.matched,
                        "firstYear": card.first_year,
                        "photos": card.photos.iter().map(|id| id.0).collect::<Vec<_>>(),
                    }));
                }
                Ok(json!({
                    "headline": memories.headline(),
                    "cards": cards,
                }))
            }
            "app.shortcuts" => {
                let registry = command::Registry::v0();
                Ok(json!({
                    "count": registry.ready().filter(|command| command.shortcut.is_some()).count(),
                    "bindings": registry
                        .ready()
                        .filter(|command| command.shortcut.is_some())
                        .map(|command| json!({
                            "id": command.id,
                            "label": command.label,
                            "keys": command.shortcut,
                            "group": command.group.label(),
                        }))
                        .collect::<Vec<_>>(),
                }))
            }
            "app.about" => Ok(json!({
                "name": "Sortlens",
                "version": sortlens_core::VERSION,
                "engine": VERSION,
                "commands": self.registry.all().len(),
                "readyCommands": self.registry.ready_count(),
                "libraryOpen": self.has_library(),
            })),
            "app.diagnostics" => {
                let text = self.diagnostics()?;
                if let Some(path) = args.get("file").and_then(Value::as_str) {
                    std::fs::write(path, text.as_bytes()).map_err(|error| Error::io(path.to_string(), error))?;
                    return Ok(json!({ "written": path, "lines": text.lines().count() }));
                }
                Ok(json!({ "text": text, "lines": text.lines().count() }))
            }
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
        };
        // Every response carries the catalog revision, per the SL-D11 design note: a stale render
        // can be dropped rather than written back. Reading it once, after the command, is the
        // revision the response describes.
        let rev = self.revision();
        response.map(|value| match value {
            Value::Object(mut map) => {
                map.insert("rev".into(), json!(rev));
                Value::Object(map)
            }
            other => other,
        })
    }
}

/// What a health check found.
#[derive(Debug, Clone, Default)]
pub struct Health {
    /// Photos examined.
    pub checked: usize,
    /// Photos whose file is there.
    pub present: usize,
    /// Photos whose file is gone.
    pub missing: usize,
    /// Photos whose missing state changed during this check.
    pub changed: usize,
    /// Files whose size or modification time no longer matches the index.
    pub modified: usize,
    /// Those photos, so a refresh can act on exactly what the check found.
    pub modified_ids: Vec<PhotoId>,
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
/// One or many collection ids: `collections: [1,2]`, or the single `id`/`collection` key.
fn collection_ids_from(args: &Value) -> Result<Vec<sortlens_core::CollectionId>> {
    if let Some(list) = args.get("collections").and_then(Value::as_array) {
        return list
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .map(|id| sortlens_core::CollectionId(id as u32))
                    .ok_or_else(|| Error::Invalid("collection ids must be numbers".into()))
            })
            .collect();
    }
    Ok(vec![sortlens_core::CollectionId(collection_id_from(args)?)])
}

fn collection_id_from(args: &Value) -> Result<u32> {
    args.get("collection")
        .or_else(|| args.get("id"))
        .and_then(Value::as_u64)
        .map(|id| id as u32)
        .ok_or_else(|| Error::Invalid("a collection id is required".into()))
}

/// What a photo's own metadata suggests, merged with what it already has.
///
/// Kept free-standing and pure so it can be tested against a constructed `Photo` — scanning a fixture
/// JPEG cannot produce EXIF, and a test that needs a camera in the file would be a test of the scanner.
fn suggestions_for(photo: &Photo, packs: &[sortlens_ai::Pack], caps: &sortlens_ai::Caps) -> Vec<Suggestion> {
    let hints = sortlens_ai::Hints::new(&photo.camera, &photo.lens, photo.iso, photo.shutter, photo.focal);
    let mut candidates = sortlens_ai::hints(&hints);
    if !packs.is_empty() {
        let context = sortlens_ai::PackContext {
            folder: photo.rel.clone(),
            camera: photo.camera.clone(),
            lens: photo.lens.clone(),
            iso: photo.iso,
            focal: photo.focal,
        };
        for pack in packs {
            candidates.extend(pack.candidates(&context));
        }
    }
    sortlens_ai::merge(&candidates, &photo.tags, *caps)
}

/// Where pack files live inside a library.
pub const PACKS_DIR: &str = "packs";

/// Every pack file in a library, read or refused.
///
/// Never fails: a folder with no packs and a folder with a broken file are both answers, and the broken
/// file comes back labelled with what is wrong. Nothing is switched off because a file was mistyped —
/// that would silently drop keywords someone had been relying on.
pub fn load_packs(dir: Option<&Path>, enabled: &std::collections::BTreeSet<String>) -> Vec<PackSummary> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let folder = dir.join(PACKS_DIR);
    let Ok(entries) = std::fs::read_dir(&folder) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let file = path.file_name().and_then(|name| name.to_str()).unwrap_or("?").to_string();
        match std::fs::read_to_string(&path) {
            Ok(text) => match sortlens_ai::Pack::parse(&text) {
                Ok(pack) => out.push(PackSummary {
                    enabled: enabled.contains(&pack.name),
                    name: pack.name.clone(),
                    licence: pack.licence.clone(),
                    keywords: pack.len(),
                    file,
                    problem: None,
                    pack: Some(pack),
                }),
                Err(error) => out.push(PackSummary::broken(&file, error.to_string())),
            },
            Err(error) => out.push(PackSummary::broken(&file, error.to_string())),
        }
    }
    out.sort_by(|left, right| left.name.cmp(&right.name).then_with(|| left.file.cmp(&right.file)));
    out
}

/// One keyword pack, as listed in settings.
#[derive(Debug, Clone, PartialEq)]
pub struct PackSummary {
    /// The pack's name.
    pub name: String,
    /// What its words may be used under.
    pub licence: String,
    /// How many keywords it can propose.
    pub keywords: usize,
    /// The file it came from, for when it needs fixing.
    pub file: String,
    /// Whether the library has it switched on.
    pub enabled: bool,
    /// Why it could not be read, when it could not.
    pub problem: Option<String>,
    /// The parsed pack, absent when it could not be read.
    pub pack: Option<sortlens_ai::Pack>,
}

impl PackSummary {
    /// A pack file that could not be used.
    fn broken(file: &str, problem: String) -> PackSummary {
        PackSummary {
            name: file.to_string(),
            licence: String::new(),
            keywords: 0,
            file: file.to_string(),
            enabled: false,
            problem: Some(problem),
            pack: None,
        }
    }

    /// One line for the settings row.
    #[must_use]
    pub fn line(&self) -> String {
        match &self.problem {
            Some(problem) => format!("{}: {problem}", self.file),
            None => format!("{} · {} keyword(s) · {}", self.name, self.keywords, self.licence),
        }
    }
}

/// The delete mode a verb asked for, shared by `photo.delete` and `cull.apply` so the two cannot drift.
///
/// `trash` is the default because it is the reversible one on every platform this builds for; `move` needs a target
/// and says so rather than silently deleting in place.
fn delete_mode_from(args: &Value) -> Result<sortlens_cull::trash::Mode> {
    match args.get("mode").and_then(Value::as_str).unwrap_or("trash") {
        "trash" => Ok(sortlens_cull::trash::Mode::Trash),
        "tag" => Ok(sortlens_cull::trash::Mode::TagOnly),
        "move" => {
            let target = args
                .get("target")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Invalid("move needs a target folder".into()))?;
            Ok(sortlens_cull::trash::Mode::Move(PathBuf::from(target)))
        }
        other => Err(Error::Invalid(format!("unknown delete mode: {other}"))),
    }
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

/// Whether `path` already sits inside `folder`.
fn prop_is_inside(path: &Path, folder: &Path) -> bool {
    path.parent().is_some_and(|parent| parent.starts_with(folder))
}

/// Seconds of modification-time drift treated as "the file did not change".
pub const MODIFIED_TOLERANCE: i64 = 2;

/// How many photos an insights pass folds in between progress updates.
pub const INSIGHTS_CHUNK: usize = 512;

/// Progress shared between the insights worker and the poller.
#[derive(Debug)]
pub struct InsightsProgress {
    done: std::sync::atomic::AtomicUsize,
}

impl InsightsProgress {
    fn new() -> Self {
        Self {
            done: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Publish how far the pass has come.
    pub fn publish(&self, done: usize) {
        self.done.store(done, std::sync::atomic::Ordering::Relaxed);
    }

    /// How far the pass has come.
    pub fn done(&self) -> usize {
        self.done.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// An insights pass running on a worker.
#[derive(Debug)]
struct PendingInsights {
    rx: std::sync::mpsc::Receiver<Insights>,
    progress: std::sync::Arc<InsightsProgress>,
    job: u64,
    total: usize,
}

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

    /// Walk the sharpness pass to its end.
    fn wait_for_sharpness(engine: &mut Engine) -> SharpnessReport {
        for _ in 0..600 {
            if let Some(report) = engine.poll_sharpness() {
                return report;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the sharpness pass did not finish");
    }

    /// A scan reads a photo's GPS into the catalog, and the map groups it.
    ///
    /// Three photos: two a few hundred metres apart in Brighton, one in Sydney, one with no GPS block at all. Every
    /// step is the real one — the fixture carries an EXIF APP1 segment, `sortlens-meta` parses it, the catalog stores
    /// it, and `sortlens-geo` clusters what comes out.
    #[test]
    fn a_scan_finds_locations_and_the_map_clusters_them() {
        let library_dir = Temp::new("places-library");
        let photos = Temp::new("places-photos");
        let fixture = sortlens_testkit::Fixture::new("places");
        std::fs::create_dir_all(&photos.0).expect("dir");
        std::fs::copy(
            fixture.jpeg_with_location("IMG_0001.jpg", [40, 90, 160], 48, 32, 50.8225, -0.1372),
            photos.0.join("IMG_0001.jpg"),
        )
        .expect("copy");
        std::fs::copy(
            // About twenty metres from the first: the same dot at any zoom, so the count does not depend on which
            // side of a grid boundary they happen to fall on.
            fixture.jpeg_with_location("IMG_0002.jpg", [40, 90, 170], 48, 32, 50.8227, -0.1370),
            photos.0.join("IMG_0002.jpg"),
        )
        .expect("copy");
        std::fs::copy(
            fixture.jpeg_with_location("IMG_0003.jpg", [40, 60, 90], 48, 32, -33.8688, 151.2093),
            photos.0.join("IMG_0003.jpg"),
        )
        .expect("copy");
        std::fs::copy(fixture.jpeg("IMG_0004.jpg", [10, 20, 30], 48, 32), photos.0.join("IMG_0004.jpg")).expect("copy");

        let mut engine = Engine::open(&library_dir.0).expect("open");
        engine.begin_scan(photos.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);

        assert_eq!(engine.geotagged(), 3, "three photos carried a location");
        let located = engine.located();
        assert_eq!(located.len(), 3);
        let sydney = located
            .iter()
            .find(|(_, at)| at.lat < 0.0)
            .map(|(_, at)| *at)
            .expect("the southern hemisphere photo");
        assert!((sydney.lat + 33.8688).abs() < 1e-4, "the sign survives the whole path");
        assert!((sydney.lon - 151.2093).abs() < 1e-4);

        // At a city zoom the two Brighton photos are one dot and Sydney is another; nothing is drawn for the photo
        // with no GPS block.
        let clusters = engine.places(12);
        assert_eq!(clusters.len(), 2, "two places, not two photos and a gap");
        assert_eq!(clusters.iter().map(sortlens_geo::Cluster::len).sum::<usize>(), 3);
        let brighton = clusters.iter().find(|cluster| cluster.centre.lat > 0.0).expect("Brighton");
        assert_eq!(brighton.len(), 2);
        // Zoomed right out the two hemisphere photos are still separate dots, but the count is stable.
        assert_eq!(engine.places(0).iter().map(sortlens_geo::Cluster::len).sum::<usize>(), 3);
        // The same call twice is the same answer, which is what keeps dots from jumping between frames.
        assert_eq!(
            engine.places(6).iter().map(|c| (c.cell, c.len())).collect::<Vec<_>>(),
            engine.places(6).iter().map(|c| (c.cell, c.len())).collect::<Vec<_>>()
        );
    }

    /// Measuring the library stores one score per photo, as a single undoable change.
    #[test]
    fn measuring_the_library_scores_every_photo_in_one_undoable_step() {
        let (_cache, _photos, mut engine) = library_with_images("sharpness", 3);
        let ids: Vec<PhotoId> = engine.all_photos().into_iter().map(|photo| photo.id).collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.iter().all(|id| engine.sharpness(*id).is_none()), "nothing measured yet");

        assert_eq!(engine.begin_sharpness().expect("begin"), 3, "three photos to measure");
        let report = wait_for_sharpness(&mut engine);
        assert_eq!(report.measured, 3);
        assert_eq!(report.unjudged, 0, "flat colour JPEGs are measurable");
        assert_eq!(report.remaining, 0);
        for id in &ids {
            let score = engine.sharpness(*id).expect("a score");
            assert_eq!(score.version, sortlens_quality::SCORE_VERSION);
            assert!(score.value.is_finite() && score.value >= 0.0);
        }

        // A second pass over an unchanged library has nothing to do and writes nothing.
        assert_eq!(engine.begin_sharpness().expect("again"), 0, "everything is current");

        // One undo takes back the whole measurement, not one photo of it.
        assert!(engine.undo().expect("undo"));
        assert!(ids.iter().all(|id| engine.sharpness(*id).is_none()), "the whole pass undid");
        assert_eq!(engine.all_photos().len(), 3, "the photos are still there");
    }

    /// An older algorithm's score is re-measured rather than compared with today's.
    #[test]
    fn a_score_from_another_version_is_queued_again() {
        let (_cache, _photos, mut engine) = library_with_images("sharpness-version", 1);
        let id = engine.all_photos().first().map(|photo| photo.id).expect("a photo");
        engine
            .set_scores(&[(id, sortlens_quality::Score { value: 9.0, version: 0 })])
            .expect("store an old score");
        assert_eq!(engine.begin_sharpness().expect("begin"), 1, "the old score is not comparable");
        let report = wait_for_sharpness(&mut engine);
        assert_eq!(report.measured, 1);
        assert_eq!(
            engine.sharpness(id).map(|score| score.version),
            Some(sortlens_quality::SCORE_VERSION)
        );
    }

    /// A file that cannot be decoded is counted as unjudged, never stored as a score — "cannot tell" is not
    /// "blurry", and it is certainly not "sharp".
    #[test]
    fn a_file_that_cannot_be_decoded_is_counted_rather_than_scored() {
        let (cache, photos, mut engine) = library_with_images("sharpness-unjudged", 1);
        // A file with a .jpg name and no image inside it: the scan indexes it by extension, the decoder refuses it.
        std::fs::write(photos.0.join("broken.jpg"), b"not an image").expect("write");
        engine.begin_scan(photos.0.to_string_lossy().to_string()).expect("rescan");
        wait_for_scan(&mut engine);
        let _ = cache;

        let report = {
            engine.begin_sharpness().expect("begin");
            wait_for_sharpness(&mut engine)
        };
        assert_eq!(report.measured, 1, "the real photo was measured");
        assert_eq!(report.unjudged, 1, "the broken file was counted, not scored");
        assert_eq!(report.remaining, 0);
        let broken = engine
            .all_photos()
            .into_iter()
            .find(|photo| photo.rel == "broken.jpg")
            .map(|photo| photo.id)
            .expect("the broken file is indexed");
        assert_eq!(engine.sharpness(broken), None, "an undecodable file has no score");
    }

    /// A folder's pin and colour survive a restart, undo as one change, and never become two marks for one folder
    /// spelled two ways.
    #[test]
    fn a_folder_mark_is_keyed_by_path_identity_and_undoes_in_one_step() {
        let (_cache, photos, mut engine) = library_with_images("folder-mark", 1);
        let root = photos.0.to_string_lossy().to_string();

        assert!(engine.folder_marks().is_empty(), "a folder starts unmarked");
        let mark = sortlens_catalog::folder::FolderMark {
            favourite: true,
            colour: Some(ColorLabel::Blue),
            sort: Some("largest".into()),
            search: Some("beach".into()),
        };
        assert!(engine.set_folder_mark(&root, mark.clone()).expect("mark"));
        assert_eq!(
            engine.folder_mark(&sortlens_catalog::folder::key_of(&root)).colour,
            Some(ColorLabel::Blue)
        );

        assert_eq!(
            engine.folder_mark(&sortlens_catalog::folder::key_of(&root)).sort.as_deref(),
            Some("largest"),
            "a remembered sort is part of the mark"
        );
        assert_eq!(
            engine.folder_mark(&sortlens_catalog::folder::key_of(&root)).search.as_deref(),
            Some("beach")
        );

        // The same folder typed with a trailing separator is the same mark, not a second one.
        assert!(
            !engine.set_folder_mark(&format!("{root}/"), mark).expect("again"),
            "nothing changed"
        );
        assert_eq!(engine.folder_marks().len(), 1, "one folder, one mark");

        // One undo clears the whole mark, because the op records the whole mark.
        assert!(engine.undo().expect("undo"));
        assert!(engine.folder_marks().is_empty(), "the mark is gone, not half of it");

        // Clearing an empty mark is not a write.
        assert!(
            !engine
                .set_folder_mark(&root, sortlens_catalog::folder::FolderMark::default())
                .expect("no-op")
        );
        // An empty path is refused rather than stored under the empty key.
        assert!(
            engine
                .set_folder_mark(
                    "",
                    sortlens_catalog::folder::FolderMark {
                        favourite: true,
                        ..sortlens_catalog::folder::FolderMark::default()
                    }
                )
                .is_err()
        );
    }

    /// Recolouring, pinning or deleting several collections is **one** step to undo, and counts only what moved.
    #[test]
    fn a_batch_collection_change_undoes_in_one_step() {
        let (_cache, _photos, mut engine) = library_with_images("collection-batch", 2);
        let ids: Vec<sortlens_core::CollectionId> = (0..3)
            .map(|index| engine.create_collection(&format!("Set {index}")).expect("create"))
            .collect();

        assert_eq!(engine.set_collections_colour(&ids, Some(ColorLabel::Blue)).expect("colour"), 3);
        assert_eq!(
            engine.set_collections_colour(&ids, Some(ColorLabel::Blue)).expect("again"),
            0,
            "nothing moved the second time"
        );
        // One undo puts all three back: the group's inverse is one group.
        assert!(engine.undo().expect("undo"));
        assert!(
            engine.collections().iter().all(|collection| collection.colour.is_none()),
            "every collection lost the colour at once"
        );
        // A second undo reaches past the group to the collection *creation* underneath it — one op, not
        // three — which is how we know the batch went in as a single entry rather than three.
        assert!(engine.undo().expect("undo again"));
        assert_eq!(engine.collections().len(), 2, "one creation undone, not three");

        // Put a third collection back so the rest of the test has three: ids are not reused, so this is new.
        let restored = engine.create_collection("Set 3").expect("create again");
        assert_ne!(restored, ids[2], "a deleted collection's id is not handed out again");
        let ids: Vec<sortlens_core::CollectionId> = engine.collections().iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), 3);

        assert_eq!(engine.set_collections_favourite(&ids, true).expect("pin"), 3);
        assert_eq!(engine.set_collections_favourite(&ids, true).expect("pin again"), 0);
        assert!(engine.undo().expect("undo"));
        assert!(engine.collections().iter().all(|collection| !collection.favourite));

        // Deleting a batch is one step too, and the photos survive it.
        let before = engine.all_photos().len();
        assert_eq!(engine.delete_collections(&ids).expect("delete"), 3);
        assert!(engine.collections().is_empty());
        assert!(engine.undo().expect("undo"));
        assert_eq!(engine.collections().len(), 3, "all three came back");
        assert_eq!(engine.all_photos().len(), before, "and the photos were never touched");
    }

    /// A write reports the photos it changed, not the photos it was given.
    ///
    /// A script that rates a page and is told "12 changed" when eleven already had those stars cannot tell a
    /// write from a no-op, and the JSON face of these commands is exactly what a script reads.
    #[test]
    fn a_repeat_write_reports_no_change() {
        let (_cache, _photos, mut engine) = library_with_images("repeat-write", 3);
        let ids: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
        assert_eq!(engine.rate(&ids, 4).expect("rate"), 3);
        assert_eq!(engine.rate(&ids, 4).expect("rate again"), 0, "nothing moved the second time");
        assert_eq!(engine.rate(&ids[..1], 5).expect("one"), 1);

        assert_eq!(engine.flag(&ids, Flag::Pick).expect("flag"), 3);
        assert_eq!(engine.flag(&ids, Flag::Pick).expect("flag again"), 0);

        assert_eq!(engine.label(&ids, Some(ColorLabel::Red)).expect("label"), 3);
        assert_eq!(engine.label(&ids, Some(ColorLabel::Red)).expect("label again"), 0);
        assert_eq!(engine.label(&ids, None).expect("clear"), 3);

        let day = 1_700_000_000;
        assert_eq!(engine.set_captured(&[(ids[0], Some(day))]).expect("date"), 1);
        assert_eq!(engine.set_captured(&[(ids[0], Some(day))]).expect("date again"), 0);
        assert_eq!(
            engine.photo(ids[0]).expect("photo").captured,
            Some(day),
            "and it really is that date"
        );
    }

    /// The second photo of a look: one keyword typed once, then the rest of the selection gets it by key.
    #[test]
    fn the_last_keyword_can_be_applied_to_another_photo() {
        let (_cache, _photos, mut engine) = library_with_images("tag-from-prev", 4);
        let ids: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
        // Nothing applied yet: a refusal that says what to do, not a silent no-op.
        let error = engine.tag_from_prev(&ids[1..]).expect_err("nothing remembered");
        assert!(error.to_string().contains("no keyword has been applied"));
        assert_eq!(engine.last_tag(), None);

        engine.set_tag(&ids[..1], "backlit", true).expect("tag");
        assert_eq!(engine.last_tag(), Some("backlit"));
        // Removing a keyword is not something you then want repeated: only an add is remembered.
        engine.set_tag(&ids[..1], "backlit", false).expect("untag");
        assert_eq!(engine.last_tag(), Some("backlit"), "the add is what is remembered");

        let changed = engine.tag_from_prev(&ids[1..]).expect("apply");
        assert_eq!(changed, 3, "three photos gained the keyword");
        for id in &ids[1..] {
            assert!(engine.photo(*id).expect("photo").tags.contains(&"backlit".to_string()));
        }
        // Pressing it twice is not a second write.
        assert_eq!(engine.tag_from_prev(&ids[1..]).expect("again"), 0);
        // The first photo had the keyword removed above, so this re-adds it — exactly once, not twice.
        assert_eq!(engine.tag_from_prev(&ids[..1]).expect("first"), 1);
        assert_eq!(
            engine
                .photo(ids[0])
                .expect("photo")
                .tags
                .iter()
                .filter(|tag| *tag == "backlit")
                .count(),
            1
        );
        // And a photo that already carries it is not written again.
        assert_eq!(engine.tag_from_prev(&ids[..1]).expect("again"), 0);
    }

    fn wait_for_export(engine: &mut Engine) -> sortlens_export::Report {
        for _ in 0..600 {
            if let Some(report) = engine.poll_export() {
                return report;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("export did not finish");
    }

    /// A library with `count` real JPEGs indexed, and the folder they live in.
    fn library_with_images(name: &str, count: usize) -> (Temp, Temp, Engine) {
        let library_dir = Temp::new(name);
        let photos = Temp::new(&format!("{name}-photos"));
        let fixture = sortlens_testkit::Fixture::new(name);
        for tick in 0..count {
            let source = fixture.jpeg(&format!("IMG_{tick:04}.jpg"), [40 + tick as u8 * 40, 80, 160], 48, 32);
            let target = photos.0.join(format!("IMG_{tick:04}.jpg"));
            std::fs::create_dir_all(&photos.0).expect("dir");
            std::fs::copy(&source, &target).expect("copy fixture");
        }
        let mut engine = Engine::open(&library_dir.0).expect("open");
        engine.begin_scan(photos.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        (library_dir, photos, engine)
    }

    fn photo_with(camera: &str, lens: &str, iso: Option<u32>, shutter: Option<f64>, focal: Option<f64>, tags: &[&str]) -> Photo {
        let mut photo = Photo {
            id: PhotoId(1),
            root: "/photos".into(),
            rel: "a.jpg".into(),
            name: "a.jpg".into(),
            size: 1,
            modified: Some(0),
            kind: MediaKind::Image,
            key: 1,
            rating: 0,
            flag: Flag::None,
            label: None,
            missing: false,
            duration_ms: None,
            fps: None,
            video_codec: None,
            width: 0,
            height: 0,
            captured: None,
            camera: camera.into(),
            lens: lens.into(),
            iso,
            aperture: None,
            shutter,
            focal,
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            edited: false,
            retry: false,
            sharpness: None,
            location: None,
            rev: 0,
            added: None,
        };
        photo.tags.sort_unstable();
        photo
    }

    #[test]
    fn a_photos_own_metadata_becomes_suggestions() {
        let photo = photo_with("Nikon Z6", "NIKKOR Z 24-70mm f/4 S", Some(6400), Some(2.0), Some(24.0), &[]);
        let suggestions = suggestions_for(&photo, &[], &sortlens_ai::Caps::default());
        let tags: Vec<&str> = suggestions.iter().map(|suggestion| suggestion.tag.as_str()).collect();
        // The merger ranks by score, then alphabetically — not in the order the rules fired.
        assert_eq!(
            tags,
            vec!["nikkor z 24-70mm f/4 s", "nikon z6", "long exposure", "low light", "wide"]
        );
        assert!(
            suggestions.windows(2).all(|pair| pair[0].score >= pair[1].score),
            "the best suggestion is first"
        );
        assert!(
            suggestions
                .iter()
                .all(|suggestion| suggestion.source == sortlens_ai::Source::ExifHint)
        );
    }

    #[test]
    fn a_keyword_the_photo_already_has_is_not_suggested_again() {
        let photo = photo_with("Nikon Z6", "", Some(6400), None, None, &["Nikon Z6"]);
        let suggestions = suggestions_for(&photo, &[], &sortlens_ai::Caps::default());
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].tag, "low light");
    }

    #[test]
    fn a_photo_with_no_metadata_gets_no_suggestions() {
        let photo = photo_with("", "", None, None, None, &[]);
        assert!(suggestions_for(&photo, &[], &sortlens_ai::Caps::default()).is_empty());
    }

    #[test]
    fn a_pack_on_disk_is_listed_switched_on_and_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("sortlens-engine-packs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(PACKS_DIR)).expect("packs dir");
        std::fs::write(
            dir.join(PACKS_DIR).join("travel.json"),
            r#"{"name":"Travel","licence":"CC0-1.0","keywords":[{"keyword":"japan","when":{"folder_contains":"Japan"}}]}"#,
        )
        .expect("write pack");
        std::fs::write(dir.join(PACKS_DIR).join("broken.json"), "{ nocode").expect("write broken");

        let mut engine = Engine::open(&dir).expect("open");
        let listed = engine.packs();
        assert_eq!(listed.len(), 2, "a broken file is listed, not hidden");
        let travel = listed.iter().find(|pack| pack.name == "Travel").expect("travel");
        assert_eq!(travel.keywords, 1);
        assert_eq!(travel.licence, "CC0-1.0");
        assert!(!travel.enabled, "a pack is off until it is switched on");
        let broken = listed.iter().find(|pack| pack.name == "broken.json").expect("broken");
        assert!(broken.problem.as_deref().is_some_and(|problem| !problem.is_empty()));
        assert!(broken.line().contains("broken.json"));

        assert!(engine.set_pack("Travel", true).expect("enable"));
        assert!(engine.packs().iter().find(|pack| pack.name == "Travel").expect("travel").enabled);
        drop(engine);

        let engine = Engine::open(&dir).expect("reopen");
        assert!(
            engine.packs().iter().find(|pack| pack.name == "Travel").expect("travel").enabled,
            "the switch is in the log, so it survives a restart"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_enabled_pack_adds_keywords_for_a_photo_in_the_folder_it_names() {
        let photo = photo_with("", "", None, None, None, &[]);
        let pack = sortlens_ai::Pack::parse(
            r#"{"name":"Travel","licence":"CC0-1.0","keywords":[{"keyword":"japan","when":{"folder_contains":"Japan"}}]}"#,
        )
        .expect("parse");
        let mut in_japan = photo.clone();
        in_japan.rel = "Trips/Japan/DSC1.jpg".to_string();
        assert!(suggestions_for(&photo, std::slice::from_ref(&pack), &sortlens_ai::Caps::default()).is_empty());
        let suggested = suggestions_for(&in_japan, std::slice::from_ref(&pack), &sortlens_ai::Caps::default());
        assert_eq!(suggested.len(), 1);
        assert_eq!(suggested[0].tag, "japan");
        assert_eq!(suggested[0].source, sortlens_ai::Source::Pack);
    }

    #[test]
    fn the_diagnostics_bundle_says_what_is_happening_and_where_nothing_is() {
        let (library_dir, photos, mut engine) = library_with_images("diagnostics", 2);
        let root = photos.0.display().to_string();
        let library = library_dir.0.display().to_string();
        let id = engine.all_photos()[0].id;
        engine.set_tag(&[id], "harbour", true).expect("tag");

        let text = engine.diagnostics().expect("diagnostics");
        assert!(text.starts_with("# Sortlens diagnostics"));
        assert!(text.contains("sortlens = "), "the version is the first thing a maintainer asks");
        assert!(text.contains("photos = 2"), "{text}");
        assert!(text.contains("tagged = 1"));
        assert!(text.contains("[health]"));
        assert!(text.contains("present = 2"));
        assert!(text.contains("[recent changes]"));
        assert!(text.contains("Keyworded"), "the change summary is what a maintainer reads: {text}");

        // The promise of the bundle: no absolute path of this machine is in it.
        assert!(!text.contains(&root), "the root path leaked: {text}");
        assert!(!text.contains(&library), "the library path leaked: {text}");
        assert!(!text.contains("/Users/"), "a home path leaked: {text}");

        // Writing it to a file produces the same text, and the file is what someone attaches.
        let report = std::env::temp_dir().join(format!("sortlens-diagnostics-{}.txt", std::process::id()));
        let written = engine
            .run("app.diagnostics", &json!({ "file": report.display().to_string() }))
            .expect("write");
        assert_eq!(written["written"], json!(report.display().to_string()));
        let on_disk = std::fs::read_to_string(&report).expect("read back");
        assert_eq!(on_disk, text, "what is written is what was printed");
        let _ = std::fs::remove_file(&report);

        // And asking for it without a file returns the text, for an agent.
        let inline = engine.run("app.diagnostics", &json!({})).expect("inline");
        assert!(inline["text"].as_str().is_some_and(|text| text.contains("[library]")));
    }

    #[test]
    fn the_roots_and_keyword_commands_answer_with_what_a_script_needs() {
        let (_library_dir, _photos, mut engine) = library_with_images("jsonverbs", 2);
        let roots = engine.run("library.roots", &json!({})).expect("roots");
        assert_eq!(roots["roots"].as_array().map(Vec::len), Some(1));
        assert_eq!(roots["roots"][0]["photos"], json!(2));
        let root = roots["roots"][0]["root"].as_str().expect("a root path");
        assert!(!root.is_empty(), "the root is named, so a script can show it: {root}");

        let id = engine.all_photos()[0].id;
        engine.set_tag(&[id], "harbour", true).expect("tag");
        let keywords = engine.run("tag.keywords", &json!({})).expect("keywords");
        assert_eq!(keywords["count"], json!(1));
        assert_eq!(keywords["keywords"][0]["tag"], json!("harbour"));
        assert_eq!(keywords["keywords"][0]["photos"], json!(1));
        // An empty library is an empty list, not an error: a script should not have to special-case it.
        engine.set_tag(&[id], "harbour", false).expect("untag");
        assert_eq!(engine.run("tag.keywords", &json!({})).expect("again")["count"], json!(0));
    }

    #[test]
    fn a_suggestion_pass_proposes_without_writing_anything() {
        let (_library_dir, _photos, mut engine) = library_with_images("batch", 3);
        let before: Vec<Vec<String>> = engine.all_photos().iter().map(|photo| photo.tags.clone()).collect();
        let log_before = engine.history(50).expect("history").len();

        assert_eq!(engine.begin_batch_suggest().expect("start"), 3);
        assert!(engine.batch_progress().is_some(), "the pass reports progress while it runs");
        assert!(engine.begin_batch_suggest().is_err(), "a second pass is refused while one runs");

        let mut found = None;
        for _ in 0..400 {
            if let Some(count) = engine.poll_batch() {
                found = Some(count);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(found, Some(0), "a fixture JPEG has no metadata, so there is nothing to propose");
        assert!(engine.batch_progress().is_none(), "the pass is over");

        // Nothing was written: no tag added, no op logged. That is the whole promise of a batch pass.
        let after: Vec<Vec<String>> = engine.all_photos().iter().map(|photo| photo.tags.clone()).collect();
        assert_eq!(before, after, "a pass must not tag anything");
        assert_eq!(
            engine.history(50).expect("history").len(),
            log_before,
            "a pass writes no op either, so the change log is untouched"
        );
        assert_eq!(engine.batch_results().len(), 0);

        engine.clear_batch();
        assert!(engine.batch_results().is_empty());
    }

    #[test]
    fn a_suggestion_pass_finds_the_photos_a_pack_applies_to_and_writes_none_of_it() {
        let (library_dir, _photos, mut engine) = library_with_images("batchpack", 3);
        std::fs::create_dir_all(library_dir.0.join(PACKS_DIR)).expect("packs dir");
        // The fixture photos are all `.jpg`, so this rule matches every one of them without EXIF.
        std::fs::write(
            library_dir.0.join(PACKS_DIR).join("jpgs.json"),
            r#"{"name":"JPEGs","licence":"CC0-1.0","keywords":[{"keyword":"jpeg","when":{"folder_contains":".jpg"}}]}"#,
        )
        .expect("write pack");
        let listed = engine.packs();
        assert_eq!(listed.len(), 1, "the pack is found in the library's own folder");
        engine.set_pack("JPEGs", true).expect("enable");

        engine.begin_batch_suggest().expect("start");
        let mut found = None;
        for _ in 0..400 {
            if let Some(count) = engine.poll_batch() {
                found = Some(count);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(found, Some(3), "the pack matches every fixture photo");

        let results = engine.batch_results();
        assert_eq!(results.len(), 3);
        for (_, suggestions) in results {
            assert_eq!(suggestions.len(), 1);
            assert_eq!(suggestions[0].tag, "jpeg");
            assert_eq!(suggestions[0].source, sortlens_ai::Source::Pack);
        }
        for photo in engine.all_photos() {
            assert!(photo.tags.is_empty(), "the pass proposed, it did not tag");
        }
        // Accepting one proposal afterwards is the only path to a keyword, and it works.
        let first = results[0].0;
        assert!(engine.accept_suggestion(first, "jpeg").expect("accept"));
        assert!(engine.photo(first).is_some_and(|photo| photo.tags == vec!["jpeg".to_string()]));
    }

    #[test]
    fn the_pack_commands_list_and_switch_a_pack() {
        let dir = std::env::temp_dir().join(format!("sortlens-engine-packcmds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(PACKS_DIR)).expect("packs dir");
        std::fs::write(
            dir.join(PACKS_DIR).join("travel.json"),
            r#"{"name":"Travel","licence":"CC0-1.0","keywords":[{"keyword":"japan","when":{"folder_contains":"Japan"}}]}"#,
        )
        .expect("write pack");
        let mut engine = Engine::open(&dir).expect("open");

        let listed = engine.run("packs.list", &json!({})).expect("list");
        assert_eq!(listed["packs"][0]["name"], json!("Travel"));
        assert_eq!(listed["packs"][0]["enabled"], json!(false));
        assert_eq!(listed["packs"][0]["keywords"], json!(1));
        assert_eq!(listed["packs"][0]["problem"], json!(null));

        assert_eq!(
            engine.run("packs.set", &json!({ "name": "Travel", "on": true })).expect("on")["changed"],
            json!(true)
        );
        assert_eq!(
            engine.run("packs.list", &json!({})).expect("list")["packs"][0]["enabled"],
            json!(true)
        );
        assert_eq!(
            engine.run("packs.set", &json!({ "name": "Travel", "on": true })).expect("again")["changed"],
            json!(false),
            "switching on a pack that is already on says it changed nothing"
        );
        // And the switch is in the op log, so it undoes.
        assert!(engine.undo().expect("undo"));
        assert_eq!(
            engine.run("packs.list", &json!({})).expect("list")["packs"][0]["enabled"],
            json!(false)
        );

        assert!(engine.run("packs.set", &json!({ "name": "Travel" })).is_err(), "on is required");
        assert!(engine.run("packs.set", &json!({ "on": true })).is_err(), "a name is required");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn suggestions_are_listed_through_the_command_and_accepting_one_writes_the_keyword() {
        let (_library_dir, _photos, mut engine) = library_with_images("suggest", 1);
        let id = engine.all_photos()[0].id;
        // A fixture JPEG carries no EXIF, so there is nothing to propose — the command still answers.
        let listed = engine.run("suggest.list", &json!({ "ids": [id.0] })).expect("list");
        assert_eq!(listed["photos"][0]["id"], json!(id.0));
        assert_eq!(listed["photos"][0]["suggestions"], json!([]));

        // Accepting writes a keyword through the ordinary op, so it is in the log and undoable.
        let accepted = engine
            .run("suggest.accept", &json!({ "id": id.0, "tag": "Low Light" }))
            .expect("accept");
        assert_eq!(accepted["tagged"], json!(true));
        assert!(engine.photo(id).is_some_and(|photo| photo.tags.contains(&"low light".to_string())));
        // Accepting the same suggestion twice says it changed nothing.
        let again = engine
            .run("suggest.accept", &json!({ "id": id.0, "tag": "low light" }))
            .expect("accept again");
        assert_eq!(again["tagged"], json!(false));
        // And it is undoable like any other tag, because it *is* any other tag.
        assert!(engine.undo().expect("undo"));
        assert!(engine.photo(id).is_some_and(|photo| !photo.tags.contains(&"low light".to_string())));
        // A refusal is an error, not a silent no-op.
        assert!(engine.run("suggest.accept", &json!({ "id": id.0, "tag": "  " })).is_err());
        assert!(engine.run("suggest.accept", &json!({ "id": 9999, "tag": "dog" })).is_err());
    }

    #[test]
    fn an_export_writes_the_picks_and_reports_what_it_did() {
        let (library_dir, photos, mut engine) = library_with_images("export", 3);
        let dest = Temp::new("export-dest");
        let ids: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
        let spec = sortlens_export::Spec::default();
        let planned = engine.begin_export(&dest.0, &ids, &spec).expect("begin");
        assert_eq!(planned, 3);
        // Progress is observable while it runs, and the job appears in the activity stack.
        assert!(engine.export_progress().is_some());
        assert!(engine.jobs().iter().any(|job| job.kind == JobKind::Export));
        let report = wait_for_export(&mut engine);
        assert_eq!(report.written, 3);
        assert!(report.is_clean(), "{:?}", report.failures);
        assert!(report.bytes > 0);
        let written: Vec<String> = std::fs::read_dir(&dest.0)
            .expect("dest")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(written.len(), 3);
        // A copy is byte-identical, which is the whole promise of "Original".
        let source = std::fs::read(photos.0.join("IMG_0001.jpg")).expect("source");
        let copy = std::fs::read(dest.0.join("IMG_0001.jpg")).expect("copy");
        assert_eq!(source, copy);
        // Exporting does not touch the library.
        assert_eq!(engine.stats().total, 3);
        assert_eq!(engine.last_export().map(|report| report.written), Some(3));
        drop(library_dir);
    }

    #[test]
    fn a_second_export_into_the_same_folder_does_not_eat_the_first() {
        let (_library_dir, _photos, mut engine) = library_with_images("export-twice", 1);
        let dest = Temp::new("export-twice-dest");
        let ids: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
        let skip = sortlens_export::Spec::default();
        engine.begin_export(&dest.0, &ids, &skip).expect("first");
        assert_eq!(wait_for_export(&mut engine).written, 1);
        let mut rename = sortlens_export::Spec {
            collision: sortlens_export::Collision::Rename,
            ..Default::default()
        };
        engine.begin_export(&dest.0, &ids, &rename).expect("second");
        let report = wait_for_export(&mut engine);
        assert_eq!(report.written, 1);
        assert!(dest.0.join("IMG_0000-2.jpg").exists(), "the second file got its own name");
        // And the skip policy leaves the folder alone.
        rename.collision = sortlens_export::Collision::Skip;
        assert_eq!(engine.begin_export(&dest.0, &ids, &rename).expect("third"), 0, "nothing to write");
        let report = engine.last_export().cloned().expect("a receipt");
        assert_eq!(report.written, 0);
        assert_eq!(report.skipped, 1);
        assert_eq!(std::fs::read_dir(&dest.0).expect("dest").count(), 2);
    }

    #[test]
    fn a_png_export_writes_png_files_the_command_face_can_ask_for() {
        let (library_dir, _photos, mut engine) = library_with_images("export-png", 2);
        let dest = Temp::new("export-png-dest");
        let ids: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
        let spec = sortlens_export::Spec {
            format: sortlens_export::Format::Png,
            max_edge: Some(512),
            ..Default::default()
        };
        assert_eq!(engine.begin_export(&dest.0, &ids, &spec).expect("begin"), 2);
        let report = wait_for_export(&mut engine);
        assert_eq!(report.written, 2);
        assert!(report.is_clean(), "{:?}", report.failures);
        let bytes = std::fs::read(dest.0.join("IMG_0000.png")).expect("export");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "the file really is a png");
        let decoded = sortlens_thumbs::decode_file(&dest.0.join("IMG_0000.png"), 4096).expect("decode");
        assert_eq!(decoded.rgba[3], 255, "a jpeg source exports fully opaque");
        drop(library_dir);
    }

    #[test]
    fn an_export_refuses_a_relative_folder_and_a_missing_photo() {
        let (_library_dir, _photos, mut engine) = library_with_images("export-refuse", 1);
        let ids: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
        let spec = sortlens_export::Spec::default();
        assert!(engine.begin_export(Path::new("relative"), &ids, &spec).is_err());
        // A photo whose file has gone is named rather than silently dropped from the plan.
        std::fs::remove_file(engine.photo(ids[0]).expect("photo").path()).expect("remove");
        let dest = Temp::new("export-refuse-dest");
        let error = engine.begin_export(&dest.0, &ids, &spec).expect_err("missing file");
        assert!(error.to_string().contains("is not on disk"), "{error}");
    }

    #[test]
    fn an_export_with_nothing_to_write_says_so_instead_of_starting_a_job() {
        let (_library_dir, _photos, mut engine) = library_with_images("export-empty", 1);
        let dest = Temp::new("export-empty-dest");
        std::fs::create_dir_all(&dest.0).expect("dest");
        std::fs::write(dest.0.join("IMG_0000.jpg"), b"in the way").expect("existing");
        let ids: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
        let spec = sortlens_export::Spec::default();
        assert_eq!(engine.begin_export(&dest.0, &ids, &spec).expect("begin"), 0);
        assert!(engine.export_progress().is_none(), "no bar for a job that writes nothing");
        let report = engine.last_export().expect("a receipt");
        assert_eq!(report.skipped, 1);
        assert_eq!(report.written, 0);
        assert_eq!(std::fs::read(dest.0.join("IMG_0000.jpg")).expect("read"), b"in the way");
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

    /// A mislabelled file is indexed as what it is, and the scan says how many were routed that way.
    ///
    /// The count reaches the user through `ScanFinished`, which the window shows in its notice and which the
    /// CLI prints — a correction nobody is told about is a silent rewrite of their tree's meaning.
    /// Every root reports where it lives, and the volume of a folder that exists says it is mounted.
    ///
    /// The point of the probe is that an unplugged drive is one answer for a whole root instead of one
    /// `stat` per photo, so this asserts the shape the sidebar needs rather than a specific mount point,
    /// which differs on every machine.
    #[test]
    fn every_root_reports_its_volume() {
        let (_lib, photos, engine) = library_with_images("root-volumes", 2);
        let volumes = engine.root_volumes();
        assert_eq!(volumes.len(), 1, "one root holds the fixtures");
        let volume = &volumes[0];
        assert_eq!(volume.photos, 2);
        assert_eq!(volume.root, photos.0.display().to_string().trim_end_matches('/'));
        assert!(volume.mount.is_some(), "a root on a real disk has a volume");
        assert_eq!(volume.mounted, Some(true), "and that volume is mounted while the test runs");

        // A root whose folder has been deleted still has a volume if its mount point survives; what must not
        // happen is a probe that panics or invents an answer.
        let missing = std::env::temp_dir().join("sortlens-definitely-missing-root-volume");
        let _ = std::fs::remove_dir_all(&missing);
        let probed = sortlens_core::volume::probe(&missing);
        assert!(probed.is_some(), "a path that does not exist still lives on a volume");
    }

    #[test]
    fn a_mislabelled_file_is_indexed_by_its_header_and_counted() {
        let folder = Temp::new("mislabelled-folder");
        std::fs::create_dir_all(&folder.0).expect("dir");
        let jpeg = [0xff_u8, 0xd8, 0xff, 0xe0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1];
        std::fs::write(folder.0.join("preview.cr2"), jpeg).expect("write raw-named jpeg");
        std::fs::write(folder.0.join("plain.jpg"), jpeg).expect("write jpeg");
        let (_lib, mut engine) = library("mislabelled");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        let done = wait_for_scan(&mut engine);

        assert_eq!(done.added, 2, "nothing was dropped for being mislabelled");
        assert_eq!(done.mislabelled, 1, "one header contradicted its extension");
        let photos = engine.all_photos();
        let preview = photos.iter().find(|photo| photo.name == "preview.cr2").expect("indexed");
        assert_eq!(preview.kind, MediaKind::Image, "a JPEG preview is a still, whatever it is called");
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

    /// A set chosen outside a cull session goes through the same executor: batch, flags, reject bin, undo.
    #[test]
    fn deleting_a_chosen_set_records_a_batch_and_undoes_the_flags() {
        let folder = Temp::new("delete-chosen-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"x").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"y").expect("write");
        std::fs::write(folder.0.join("c.jpg"), b"z").expect("write");
        let (_lib, mut engine) = library("delete-chosen");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        assert_eq!(ids.len(), 3);

        // Two of the three, into a rejects folder. `Mode::Trash` is the same code path but hands the file to the
        // operating system's trash, which in a test would move a real file into the real user's Trash — the batch,
        // audit and undo machinery is what is being tested here, and `Move` exercises all of it.
        // The rejects folder lives outside the scanned root on purpose: the audit refuses a destination inside a
        // watched root, because rejects there would reappear on the next scan.
        let rejects_dir = Temp::new("delete-chosen-rejects");
        let rejects = rejects_dir.0.clone();
        let report = engine
            .delete_photos(&[ids[0], ids[2]], sortlens_cull::trash::Mode::Move(rejects.clone()))
            .expect("delete");
        assert_eq!(report.applied.len(), 2);
        assert!(engine.cull().is_none(), "no cull session was started");
        assert_eq!(engine.deleted_batches().len(), 1, "one batch, so the rejects bin can restore it");
        assert_eq!(engine.photo(ids[0]).expect("photo").flag, Flag::Reject);
        assert_eq!(engine.photo(ids[1]).expect("photo").flag, Flag::None, "not chosen, not touched");
        assert!(!folder.0.join("a.jpg").exists(), "the file left the folder");
        assert!(
            rejects.join("a.jpg").exists(),
            "and is in the rejects folder, where it can be restored"
        );
        assert!(folder.0.join("b.jpg").exists());

        // One undo puts the review state back; the file is in the trash, where the rejects bin restores it from.
        assert!(engine.undo().expect("undo"));
        assert_eq!(engine.photo(ids[0]).expect("photo").flag, Flag::None);
        assert_eq!(engine.photo(ids[2]).expect("photo").flag, Flag::None);
        assert_eq!(engine.deleted_batches().len(), 1, "the batch record survives the undo, as it must");
    }

    /// The audit is shared, so a chosen set gets the same refusal a cull batch gets — and nothing is deleted.
    #[test]
    fn deleting_a_chosen_set_refuses_a_file_that_changed() {
        let folder = Temp::new("delete-changed-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"first").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"second").expect("write");
        let (_lib, mut engine) = library("delete-changed");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();

        // The file changes after it was indexed, so its bytes no longer match the record.
        std::fs::write(folder.0.join("a.jpg"), b"rewritten and longer").expect("rewrite");
        let rejects_dir = Temp::new("delete-changed-rejects");
        let refused = engine.delete_photos(&ids, sortlens_cull::trash::Mode::Move(rejects_dir.0.clone()));
        assert!(refused.is_err(), "a changed file stops the batch");
        assert!(folder.0.join("a.jpg").exists(), "nothing was deleted");
        assert!(folder.0.join("b.jpg").exists(), "not even the file that had not changed");
        assert!(engine.deleted_batches().is_empty(), "and no batch was recorded");
        assert!(
            engine
                .warnings()
                .iter()
                .any(|warning| warning.contains("changed since it was indexed"))
        );
    }

    #[test]
    fn deleting_nothing_is_an_error_rather_than_a_silent_no_op() {
        let (_lib, mut engine) = library("delete-nothing");
        let error = engine
            .delete_photos(&[], sortlens_cull::trash::Mode::Move("/tmp/rejects".into()))
            .expect_err("empty");
        assert!(error.to_string().contains("no photos were chosen"), "{error}");
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
    fn deleting_a_keyword_through_the_command_touches_every_photo_and_undoes() {
        let folder = Temp::new("delete-tag-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"b").expect("write");
        let (_lib, mut engine) = library("delete-tag");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        engine.set_tag(&ids, "Beach", true).expect("tag");
        engine.set_tag(&ids[..1], "sand", true).expect("tag");

        let value = engine.run("tag.delete", &json!({ "tag": "#BEACH" })).expect("delete");
        assert_eq!(value["deleted"], 2, "names are normalised before matching");
        assert_eq!(engine.tag_vocabulary(), vec![("sand".to_string(), 1)]);
        assert!(engine.photo(ids[1]).expect("photo").tags.is_empty());

        // One undo restores the keyword on every photo it was on.
        assert!(engine.undo().expect("undo"));
        assert_eq!(engine.tag_vocabulary(), vec![("beach".to_string(), 2), ("sand".to_string(), 1)]);

        assert!(engine.run("tag.delete", &json!({})).is_err(), "a keyword is required");
        assert!(engine.run("tag.delete", &json!({ "tag": "harbour" })).is_err(), "nothing to delete");
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
    fn a_renamed_keyword_still_resolves_when_it_is_typed_again() {
        let folder = Temp::new("alias-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"b").expect("write");
        let (_lib, mut engine) = library("lib-alias");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();

        engine.set_tag(&[ids[0]], "strand", true).expect("tag");
        engine.rename_tag("strand", "beach").expect("rename");

        // Typing the old name now lands on the keyword it was renamed to.
        engine.set_tag(&[ids[1]], "STRAND", true).expect("tag again");
        assert!(engine.photo(ids[1]).expect("photo").has_tag("beach"));
        assert!(!engine.photo(ids[1]).expect("photo").has_tag("strand"));

        let value = engine.run("tag.registry", &json!({})).expect("registry");
        assert_eq!(value["entries"][0]["name"], "beach");
        assert_eq!(value["entries"][0]["count"], 2);
        assert_eq!(value["entries"][0]["aliases"][0], "strand");
        assert!(value["headline"].as_str().expect("headline").contains("renamed"));

        // Linking and forgetting are commands too.
        let linked = engine
            .run("tag.alias", &json!({ "alias": "seaside", "canonical": "beach" }))
            .expect("alias");
        assert_eq!(linked["changed"], true);
        engine.set_tag(&[ids[0]], "seaside", true).expect("tag alias");
        assert!(engine.photo(ids[0]).expect("photo").has_tag("beach"));
        let forgotten = engine.run("tag.alias", &json!({ "alias": "seaside" })).expect("forget");
        assert_eq!(forgotten["changed"], true);
    }

    #[test]
    fn a_page_can_be_read_in_any_order_not_just_the_order_it_was_written_with() {
        let folder = Temp::new("page-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        for name in ["aa.jpg", "bb.jpg", "cc.jpg"] {
            std::fs::write(folder.0.join(name), name.as_bytes()).expect("write");
        }
        let (_lib, mut engine) = library("lib-page");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<u64> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id.0)
            .collect();
        // The last photo by name gets five stars, so a rating sort must lead with it.
        engine.rate(&[PhotoId(ids[2])], 5).expect("rate");

        let page = engine.run("library.page", &json!({ "sort": "rating", "limit": 3 })).expect("page");
        assert_eq!(page["sort"], "rating");
        assert_eq!(page["total"], 3);
        assert_eq!(page["photos"][0]["id"], ids[2], "the five-star photo leads");

        // A window, not the whole library.
        let window = engine.run("library.page", &json!({ "offset": 1, "limit": 1 })).expect("page");
        assert_eq!(window["photos"].as_array().expect("photos").len(), 1);
        assert_eq!(window["offset"], 1);
    }

    /// SL-A14 part 2: the Recently Added question is callable from the command layer — a scan adds
    /// every photo at the wall clock, so the last-30-days page holds everything, and a caller can
    /// narrow it with `addedAfter` and order it by the recorded instant rather than the mtime.
    #[test]
    fn the_recently_added_question_is_callable_from_the_command_layer() {
        let folder = Temp::new("recent-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        for name in ["aa.jpg", "bb.jpg"] {
            std::fs::write(folder.0.join(name), name.as_bytes()).expect("write");
        }
        let (_lib, mut engine) = library("lib-recent");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);

        let recent = engine
            .run(
                "library.page",
                &json!({ "filter": { "added_after": sortlens_catalog::photo::now_epoch() - 30 * 24 * 60 * 60 }, "sort": "recently-added", "limit": 10 }),
            )
            .expect("page");
        assert_eq!(recent["total"], 2, "both photos were indexed this second");
        assert_eq!(recent["sort"], "recently-added");
        let first = recent["photos"][0]["added"].as_i64().expect("added travels in the envelope");
        let second = recent["photos"][1]["added"].as_i64().expect("added travels in the envelope");
        assert!(first >= second, "newest record first");

        // A cutoff after everything excludes both: the question narrows, and photos with no recorded
        // instant would be excluded too, not borrowed from the file's mtime.
        let none = engine
            .run(
                "library.page",
                &json!({ "filter": { "added_after": 4_102_444_800_i64 }, "limit": 10 }),
            )
            .expect("page");
        assert_eq!(none["total"], 0);
    }

    #[test]
    fn revealing_a_photo_builds_a_command_and_refuses_a_missing_file() {
        let folder = Temp::new("reveal-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        let (_lib, mut engine) = library("lib-reveal");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        let value = engine.run("photo.reveal", &json!({ "id": id.0 })).expect("reveal");
        // The engine builds the command; running it is the host's job, and the result says so.
        assert_eq!(value["ran"], false);
        assert!(value["program"].as_str().expect("program").len() > 1);
        let args = value["args"].as_array().expect("args");
        assert!(
            args.iter().any(|arg| arg.as_str().unwrap_or_default().ends_with("a.jpg")),
            "the file should be named in the command: {args:?}"
        );

        let opened = engine.run("photo.open", &json!({ "id": id.0 })).expect("open");
        assert_ne!(opened["line"], value["line"], "the two actions differ");

        // A file deleted behind the app's back is refused with an instruction, not a broken spawn.
        std::fs::remove_file(folder.0.join("a.jpg")).expect("remove");
        let error = engine.run("photo.reveal", &json!({ "id": id.0 })).expect_err("missing file");
        let message = error.to_string();
        assert!(message.contains("rescan") || message.contains("relocate"), "{message}");
        assert!(engine.run("photo.reveal", &json!({})).is_err(), "an id is required");
    }

    #[test]
    fn memories_resurface_by_date_and_by_what_was_never_judged() {
        let folder = Temp::new("memory-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::write(folder.0.join(name), name.as_bytes()).expect("write");
        }
        let (_lib, mut engine) = library("lib-memories");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        assert_eq!(ids.len(), 3);

        // Capture dates are catalog data; set them the way metadata would.
        let day = |y: i64, m: u32, d: u32| sortlens_memories::midnight(sortlens_memories::Date { year: y, month: m, day: d });
        engine
            .run(
                "photo.setCaptured",
                &json!({
                    "changes": [
                        { "id": ids[0].0, "captured": day(2020, 6, 24) + 43_200 },
                        { "id": ids[1].0, "captured": day(2023, 6, 20) + 43_200 },
                    ]
                }),
            )
            .expect("captured");

        let today = day(2024, 6, 24) + 43_200;
        let value = engine.run("memories.list", &json!({ "today": today })).expect("memories");
        let cards = value["cards"].as_array().expect("cards");
        let titles: Vec<&str> = cards.iter().filter_map(|card| card["title"].as_str()).collect();
        assert!(titles.contains(&"June 24"), "{titles:?}");
        assert!(titles.contains(&"This week in past years"), "{titles:?}");
        assert!(titles.contains(&"Never looked at"), "{titles:?}");
        let on_this_day = cards.iter().find(|card| card["kind"] == "onThisDay").expect("on this day");
        assert_eq!(on_this_day["photos"], json!([ids[0].0]));
        assert_eq!(on_this_day["firstYear"], 2020);

        // Judging a photo takes it off the "never looked at" card.
        engine.flag(&[ids[2]], Flag::Pick).expect("flag");
        let after = engine.run("memories.list", &json!({ "today": today })).expect("memories");
        let forgotten = after["cards"]
            .as_array()
            .and_then(|cards| cards.iter().find(|card| card["kind"] == "forgotten"))
            .cloned();
        assert!(
            forgotten.is_none_or(|card| !card["photos"].as_array().into_iter().flatten().any(|id| id == &json!(ids[2].0))),
            "a picked photo should not be resurfaced as forgotten"
        );

        // Same day, same cards: nothing here is random.
        assert_eq!(engine.run("memories.list", &json!({ "today": today })).expect("again"), after);
    }

    #[test]
    fn insights_can_be_counted_in_the_background() {
        let folder = Temp::new("insights-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::write(folder.0.join(name), b"x").expect("write");
        }
        let (_lib, mut engine) = library("lib-insights");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
        engine.rate(&[id], 5).expect("rate");

        let queued = engine.run("insights.start", &json!({})).expect("start");
        assert_eq!(queued["queued"], 3);

        // The pass reports progress and lands, and the numbers agree with a one-shot count.
        let mut landed = None;
        for _ in 0..200 {
            if let Some(count) = engine.poll_insights() {
                landed = Some(count);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(landed, Some(3));
        assert!(engine.insights_progress().is_none(), "the pass is over");
        let insights = engine.insights();
        assert_eq!(insights.stats.total, 3);
        assert_eq!(insights.stats.rated, 1);
        assert_eq!(insights.highlights().len(), 1, "three undated photos and no camera");
        assert_eq!(insights.highlights()[0].label, "No capture date");

        let progress = engine.run("insights.progress", &json!({})).expect("progress");
        assert_eq!(progress["running"], false);
        let insights_json = engine.run("view.insights", &json!({})).expect("view");
        assert_eq!(insights_json["total"], 3);
        assert_eq!(insights_json["highlights"][0]["label"], "No capture date");
    }

    #[test]
    fn similar_can_group_by_palette_alone() {
        // Two frames with the same flat colour but nothing else in common.
        let folder = Temp::new("palette-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        for (name, colour) in [("red.jpg", [210u8, 20u8, 20u8]), ("blue.jpg", [20u8, 20u8, 210u8])] {
            let pixels: Vec<u8> = (0..64 * 64).flat_map(|_| colour).collect();
            let image = craft_jpeg::encode_baseline(&pixels, 64, 64, 3, 92, craft_jpeg::ChromaSubsampling::S444, &[]).expect("encode");
            std::fs::write(folder.0.join(name), image).expect("write");
        }
        let (_lib, mut engine) = library("lib-palette");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        engine.begin_similar(10).expect("hash");
        for _ in 0..200 {
            if engine.poll_similar_with(10, 42).is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(engine.similar_groups().len(), 0, "different palettes never group");

        // Regrouping needs no new pass over the files: the fingerprints are cached.
        let before = engine.hashes.len();
        engine.regroup(10, 42, SimilarMatch::Colour);
        assert_eq!(engine.hashes.len(), before);
        assert_eq!(engine.similar_groups().len(), 0);

        // The command face takes the mode by name.
        let value = engine
            .run("similar.groups", &json!({ "mode": "colour", "colour": 60 }))
            .expect("groups");
        assert!(value["groups"].as_array().is_some());
        assert_eq!(SimilarMatch::parse("either"), SimilarMatch::Either);
    }

    #[test]
    fn picks_move_to_a_folder_and_come_back_again() {
        let folder = Temp::new("move-photos");
        std::fs::create_dir_all(folder.0.join("day one")).expect("dir");
        std::fs::write(folder.0.join("day one/a.jpg"), b"aaaa").expect("write");
        std::fs::write(folder.0.join("day one/b.jpg"), b"bbbb").expect("write");
        let (_lib, mut engine) = library("lib-move");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        engine.rate(&[ids[0]], 5).expect("rate");
        engine.set_tag(&[ids[0]], "keeper", true).expect("tag");

        let dest = Temp::new("move-dest");
        let dest_text = dest.0.to_string_lossy().to_string();
        let moved = engine
            .run("edit.movePicks", &json!({ "dest": dest_text, "ids": [ids[0]] }))
            .expect("move");
        assert_eq!(moved["moved"], 1);
        assert!(dest.0.join("day one/a.jpg").exists(), "the file moved, keeping its layout");
        assert!(!folder.0.join("day one/a.jpg").exists());

        // The record follows the file, with its review state intact.
        let photo = engine.photo(ids[0]).expect("photo");
        assert_eq!(photo.root, dest_text);
        assert_eq!(photo.rel, "day one/a.jpg");
        assert_eq!(photo.rating, 5);
        assert!(photo.has_tag("keeper"));
        assert!(engine.roots().iter().any(|root| root == &dest_text), "the destination is indexed");

        // A receipt survives a restart, so a move can be undone tomorrow.
        let value = engine.run("edit.moves", &json!({})).expect("moves");
        let receipt_id = value["moves"][0]["id"].as_u64().expect("id");
        assert_eq!(value["moves"][0]["count"], 1);

        let restored = engine.run("edit.undoMove", &json!({ "id": receipt_id })).expect("undo");
        assert_eq!(restored["restored"], 1);
        assert!(folder.0.join("day one/a.jpg").exists(), "the file came home");
        assert!(!dest.0.join("day one/a.jpg").exists());
        assert_eq!(engine.photo(ids[0]).expect("photo").root, folder.0.to_string_lossy());
        // A sidecar the review state wrote travels with the photo, in both directions.
        assert!(!dest.0.join("day one/a.xmp").exists(), "the sidecar came home with the file");
        assert!(
            engine.run("edit.moves", &json!({})).expect("moves")["moves"]
                .as_array()
                .expect("list")
                .is_empty()
        );

        // And a move that cannot be made is refused before anything is touched.
        assert!(
            engine.run("edit.movePicks", &json!({ "dest": dest_text, "ids": [] })).is_err(),
            "nothing selected is an error, not a no-op"
        );
        assert!(
            engine
                .run("edit.movePicks", &json!({ "dest": "relative", "ids": [ids[1]] }))
                .is_err(),
            "a relative destination is refused"
        );
    }

    #[test]
    fn a_file_changed_on_disk_is_noticed_and_refreshed() {
        let folder = Temp::new("changed-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), vec![b'a'; 64]).expect("write");
        std::fs::write(folder.0.join("b.jpg"), vec![b'b'; 64]).expect("write");
        let (_lib, mut engine) = library("lib-changed");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);

        // Nothing changed yet: a plain stat must not invent work.
        let quiet = engine.run("library.changed", &json!({})).expect("check");
        assert_eq!(quiet["modified"], 0, "{quiet}");
        assert_eq!(quiet["checked"], 2);

        // Replace one file with different content *and* a different size.
        std::fs::write(folder.0.join("b.jpg"), vec![b'b'; 900]).expect("rewrite");
        let found = engine.run("library.changed", &json!({})).expect("check");
        assert_eq!(found["modified"], 1, "{found}");
        let ids = found["ids"].as_array().expect("ids").clone();
        assert_eq!(ids.len(), 1);

        // Refreshing re-hashes and re-reads, so the index matches the disk again.
        let refreshed = engine.run("library.refreshChanged", &json!({})).expect("refresh");
        assert_eq!(refreshed["refreshed"], 1);
        let changed_id = PhotoId(ids[0].as_u64().expect("id"));
        assert_eq!(engine.photo(changed_id).expect("photo").size, 900);
        let after = engine.run("library.changed", &json!({})).expect("check again");
        assert_eq!(after["modified"], 0, "the refresh settled it");

        // And it is one undoable update per photo, like every other change.
        engine.undo().expect("undo");
        assert_eq!(engine.photo(changed_id).expect("photo").size, 64);
    }

    #[test]
    fn unreadable_metadata_is_queued_and_retried() {
        let folder = Temp::new("retry-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"not really a jpeg").expect("write");
        std::fs::write(folder.0.join("b.jpg"), b"also not a jpeg").expect("write");
        let (_lib, mut engine) = library("lib-retry");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        assert_eq!(engine.retry_count(), 0, "readable files are never queued");

        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        assert_eq!(ids.len(), 2);

        // A file that could not be read at scan time stays queued once its file is gone.
        let gone = folder.0.join("b.jpg");
        std::fs::remove_file(&gone).expect("remove");
        engine.catalog.as_mut().expect("catalog").mark_retry(&ids, true).expect("queue");
        assert_eq!(engine.retry_count(), 2);

        let value = engine.run("library.retryMetadata", &json!({})).expect("retry");
        assert_eq!(value["updated"], 1, "a.jpg reads now");
        assert_eq!(value["remaining"], 1, "b.jpg is still unavailable");
        assert_eq!(engine.retry_count(), 1);

        // The retry is an op, so it undoes like everything else.
        let queued_before = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .filter(|photo| photo.retry)
            .count();
        assert_eq!(queued_before, 1);
    }

    #[test]
    fn a_second_window_reads_the_library_without_pretending_to_change_it() {
        let folder = Temp::new("ro-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"a").expect("write");
        let (lib, mut writer) = library("lib-ro");
        writer.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut writer);

        let mut reader = Engine::open(&lib.0).expect("second window");
        assert!(reader.read_only(), "the second window is read-only");
        assert_eq!(reader.run("library.readOnly", &json!({})).expect("ask")["readOnly"], true);
        assert_eq!(reader.stats().total, 1, "it still sees the library");

        let id = reader.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
        let error = reader.rate(&[id], 5).expect_err("refused");
        assert!(error.to_string().contains("read-only"), "{error}");
        let error = reader
            .begin_scan(folder.0.to_string_lossy().to_string())
            .expect_err("no rescan in a read-only library");
        assert!(error.to_string().contains("read-only"), "{error}");
        assert_eq!(reader.stats().total, 1);

        // The writer is unaffected, and when it goes the next window can write again.
        writer.rate(&[id], 4).expect("writer still writes");
        drop(reader);
        writer.close().expect("close");
        drop(writer);
        let mut next = Engine::open(&lib.0).expect("third window");
        assert!(!next.read_only());
        assert_eq!(next.stats().total, 1);
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

        // Restore what was indexed — the bytes *and* the timestamp — and the same batch goes through.
        //
        // Putting only the size back made this test a race against filesystem timestamp granularity: the
        // rewrite usually left a newer mtime and the batch was refused, but on a box whose clock ticked over
        // inside the same second it sometimes passed. The safety check compares both, so the test restores
        // both.
        std::fs::write(&image, b"x").expect("restore bytes");
        let indexed = engine.photo(id).expect("photo").modified.expect("an indexed mtime");
        let file = std::fs::OpenOptions::new().write(true).open(&image).expect("open for times");
        file.set_times(std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(indexed as u64)))
            .expect("restore mtime");
        drop(file);
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

    /// Undo and redo are both registry commands, so the MCP surface and the UI keymap drive the
    /// same twenty-step history the catalog keeps.
    #[test]
    fn undo_and_redo_run_as_commands() {
        let folder = Temp::new("undo-redo-photos");
        std::fs::create_dir_all(&folder.0).expect("dir");
        std::fs::write(folder.0.join("a.jpg"), b"aaaa").expect("write");
        let (_lib, mut engine) = library("undo-redo");
        engine.begin_scan(folder.0.to_string_lossy().to_string()).expect("scan");
        wait_for_scan(&mut engine);
        let id = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .first()
            .expect("photo")
            .id;
        engine.rate(&[id], 4).expect("rate");

        let undone = engine.run("edit.undo", &json!({})).expect("undo");
        assert_eq!(undone["undone"], true);
        assert_eq!(engine.photo(id).expect("photo").rating, 0);

        let redone = engine.run("edit.redo", &json!({})).expect("redo");
        assert_eq!(redone["redone"], true);
        assert_eq!(engine.photo(id).expect("photo").rating, 4);

        let nothing = engine.run("edit.redo", &json!({})).expect("redo again");
        assert_eq!(nothing["redone"], false);
    }

    /// SL-D11, engine level: a command carrying a stale `expect` is refused before it touches
    /// anything, and every response names the revision the caller can pass back next time.
    #[test]
    fn a_command_with_a_stale_expect_is_refused_and_responses_carry_the_revision() {
        let (_lib, _photos, mut engine) = library_with_images("stale-expect", 2);
        let ids: Vec<u64> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id.0)
            .collect();

        // A read is also stamped: the revision a caller saw comes from the response itself.
        let seen = engine.run("library.page", &json!({ "limit": 10 })).expect("page");
        let rev = seen["rev"].as_u64().expect("rev");
        assert!(rev > 0, "a scan has committed by now");

        // A command that carries the revision it read goes through.
        let rated = engine
            .run("photo.rate", &json!({ "ids": [ids[0]], "stars": 5, "expect": rev }))
            .expect("current revision");
        assert_eq!(rated["changed"], 1);
        assert!(rated["rev"].as_u64().expect("rev") > rev, "the response names the new revision");

        // The same command holding the revision from before that write is refused, before anything
        // is touched — the newer stars are not overwritten by the stale intent.
        let stale = engine
            .run("photo.rate", &json!({ "ids": [ids[0]], "stars": 0, "expect": rev }))
            .expect_err("stale");
        assert!(
            matches!(stale, Error::StaleRevision { expected, current } if expected == rev && current > rev),
            "both revisions are named: {stale:?}"
        );
        assert_eq!(engine.photo(sortlens_core::PhotoId(ids[0])).expect("photo").rating, 5);

        // A read-only command also accepts `expect`, so a caller can assert it is looking at what
        // it read before rendering from it.
        assert!(engine.run("library.page", &json!({ "expect": engine.revision() })).is_ok());
        assert!(engine.run("library.page", &json!({ "expect": rev })).is_err());
    }

    /// The revision is the catalog's sequence number, so it is stable across a reopen: a remote
    /// caller's revision from yesterday is still the same number the engine answers with today.
    #[test]
    fn the_revision_is_the_catalog_sequence_and_survives_a_reopen() {
        let temp = Temp::new("revision-reopen");
        {
            let mut engine = Engine::open(&temp.0).expect("open");
            assert_eq!(engine.revision(), 0, "nothing has happened yet");
            let about = engine.run("app.about", &json!({})).expect("about");
            assert_eq!(about["rev"], 0, "a read carries the revision it looked at");
        }
        let engine = Engine::open(&temp.0).expect("reopen");
        assert_eq!(engine.revision(), 0, "no writes, no revision");
    }
}
