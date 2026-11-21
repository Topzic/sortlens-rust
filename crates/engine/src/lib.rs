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
pub use sortlens_catalog::{Filter, Op, Page, Photo, Sort, Stats, SyncOutcome};
pub use sortlens_core::{ColorLabel, Error, Flag, MediaKind, PhotoId, Result};
pub use sortlens_cull::{Decision, Summary as CullSummary};

use serde_json::{Value, json};
use sortlens_catalog::{Catalog, Seed};
use sortlens_scan::{ScanControl, ScanProgress, ScanReport};
use std::path::{Path, PathBuf};
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
    cull: Option<sortlens_cull::Session>,
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
            cull: None,
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
        Ok(engine)
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

    /// Every keyword in use, most used first.
    pub fn tag_vocabulary(&self) -> Vec<(String, usize)> {
        self.catalog.as_ref().map(|catalog| catalog.tag_vocabulary()).unwrap_or_default()
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
                "cull.start" | "cull.apply" | "cull.undo" => "culling arrives in M4",
                "view.detail" => "the detail view arrives in M4",
                "view.similar" => "similar search arrives in M6",
                "view.places" => "the map arrives in M5",
                "view.insights" => "insights arrive in M6",
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

/// Keywords are trimmed, lowercased and length-bounded so the vocabulary stays usable.
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
            tag: Some("keepers".into()),
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
