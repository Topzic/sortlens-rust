//! L4: the command layer.
//!
//! The engine owns the open catalog, the scan worker and the command registry. The UI, the CLI
//! and the MCP server all call the same typed methods, and [`Engine::run`] is the JSON face of
//! them for scripts.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

pub mod command;

pub use command::{CommandSpec, Group, Registry, Status};
pub use sortlens_catalog::{Filter, Op, Page, Photo, Sort, Stats, SyncOutcome};
pub use sortlens_core::{ColorLabel, Error, Flag, MediaKind, PhotoId, Result};

use serde_json::{Value, json};
use sortlens_catalog::{Catalog, Seed};
use sortlens_scan::ScanReport;
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
    rx: Receiver<Result<ScanReport>>,
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
}

/// The command engine.
pub struct Engine {
    registry: Registry,
    last_effect: Effect,
    catalog: Option<Catalog>,
    pending: Option<PendingScan>,
    warnings: Vec<String>,
}

impl Engine {
    /// An engine with no library open.
    pub fn new() -> Self {
        Self {
            registry: Registry::v0(),
            last_effect: Effect::None,
            catalog: None,
            pending: None,
            warnings: Vec::new(),
        }
    }

    /// Open (or create) the catalog in `dir`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let (catalog, report) = Catalog::open(dir)?;
        Ok(Self {
            catalog: Some(catalog),
            warnings: report.warnings,
            ..Self::new()
        })
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
        std::thread::Builder::new()
            .name("sortlens-scan".into())
            .spawn(move || {
                let _ = tx.send(sortlens_scan::scan_root(&root_path));
            })
            .map_err(|error| Error::Invalid(format!("could not start the scan worker: {error}")))?;
        self.pending = Some(PendingScan {
            root,
            rx,
            started: Instant::now(),
        });
        Ok(())
    }

    /// Collect a finished scan, if one is ready.
    pub fn poll(&mut self) -> Option<ScanFinished> {
        let pending = self.pending.as_ref()?;
        match pending.rx.try_recv() {
            Ok(Ok(report)) => {
                let pending = self.pending.take()?;
                let elapsed = pending.started.elapsed();
                Some(self.finish_scan(pending.root, report, elapsed))
            }
            Ok(Err(error)) => {
                let pending = self.pending.take()?;
                self.warnings.push(format!("{}: {error}", pending.root));
                Some(ScanFinished {
                    root: pending.root,
                    added: 0,
                    updated: 0,
                    missing: 0,
                    restored: 0,
                    errors: vec![error.to_string()],
                    elapsed: pending.started.elapsed(),
                })
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                None
            }
        }
    }

    fn finish_scan(&mut self, root: String, report: ScanReport, elapsed: std::time::Duration) -> ScanFinished {
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

        ScanFinished {
            root,
            added: outcome.added,
            updated: outcome.updated,
            missing: outcome.missing,
            restored: outcome.restored,
            errors,
            elapsed,
        }
    }

    /// Set stars on a selection, clamped to 0–5.
    pub fn rate(&mut self, ids: &[PhotoId], stars: u8) -> Result<usize> {
        self.mutate(Op::Rate {
            changes: ids.iter().map(|id| (*id, stars.min(5))).collect(),
        })
    }

    /// Set a flag on a selection.
    pub fn flag(&mut self, ids: &[PhotoId], flag: Flag) -> Result<usize> {
        self.mutate(Op::Flag {
            changes: ids.iter().map(|id| (*id, flag)).collect(),
        })
    }

    /// Set a colour label on a selection.
    pub fn label(&mut self, ids: &[PhotoId], label: Option<ColorLabel>) -> Result<usize> {
        self.mutate(Op::Label {
            changes: ids.iter().map(|id| (*id, label)).collect(),
        })
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
            "edit.undo" => Ok(json!({ "undone": self.undo()? })),
            "library.stats" => Ok(serde_json::to_value(self.stats()).unwrap_or(Value::Null)),
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
    fn stats_are_reported_as_json() {
        let (_lib, mut engine) = library("stats-json");
        let stats = engine.run("library.stats", &json!({})).expect("stats");
        assert_eq!(stats["total"], 0);
    }
}
