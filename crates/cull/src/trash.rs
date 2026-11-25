//! Applying a cull batch: audit first, act once, report every file.
//!
//! `docs/BEHAVIOUR.md` §3 is the contract. The batch is audited as a whole; if any invariant fails
//! the batch does not start, because half a cull is worse than none. Once it starts, each file is
//! re-checked immediately before it is touched and every outcome — success or failure — is reported.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use serde::{Deserialize, Serialize};
use sortlens_core::{Error, PhotoId, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// What Apply does with rejected photos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Send the file to the operating system's trash. Reversible outside Sortlens. On Android and
    /// iOS, where no OS trash is crate-accessible, the file moves into the library's own
    /// [`PHONE_TRASH`] folder instead — still reversible, still reported, never destroyed.
    Trash,
    /// Move the file into a folder, preserving its path relative to its root.
    Move(PathBuf),
    /// Move nothing: the reject state and the sidecar carry the decision.
    TagOnly,
}

impl Mode {
    /// Name for the command layer.
    pub fn id(&self) -> String {
        match self {
            Self::Trash => "trash".to_string(),
            Self::Move(folder) => format!("move:{}", folder.display()),
            Self::TagOnly => "tag".to_string(),
        }
    }
}

/// One photo the batch would act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Catalog id, kept so the caller can update its own state.
    pub id: PhotoId,
    /// Absolute path of the file.
    pub path: PathBuf,
    /// The watched root it belongs to.
    pub root: PathBuf,
    /// Size the catalog recorded.
    pub size: u64,
    /// Modification time the catalog recorded, in seconds.
    pub modified: Option<i64>,
}

/// Why a batch was refused. Every reason is listed, not just the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The file is not on disk any more.
    Missing(String),
    /// The file is outside the root it claims to belong to.
    OutsideRoot { path: String, root: String },
    /// The file's size changed since it was indexed, so deleting it could destroy new work.
    Changed { path: String, expected: u64, actual: u64 },
    /// The size still matches but the modification time does not.
    ///
    /// Its own variant because the message has to say so: reporting "1 → 1 bytes" for an edited file reads as a
    /// bug in the app rather than a refusal, and a refusal nobody believes is worse than no refusal.
    Modified { path: String, indexed: i64, on_disk: i64 },
    /// A symlink resolves outside the root.
    SymlinkEscape(String),
    /// The destination folder cannot be written to.
    TargetUnwritable(String),
}

impl Refusal {
    /// One-line explanation for the activity panel.
    pub fn message(&self) -> String {
        match self {
            Self::Missing(path) => format!("{path}: no longer on disk"),
            Self::OutsideRoot { path, root } => format!("{path}: outside its root {root}"),
            Self::Changed { path, expected, actual } => {
                format!("{path}: changed since it was indexed ({expected} → {actual} bytes)")
            }
            Self::Modified { path, indexed, on_disk } => {
                format!("{path}: modified since it was indexed ({indexed} → {on_disk})")
            }
            Self::SymlinkEscape(path) => format!("{path}: symlink resolves outside its root"),
            Self::TargetUnwritable(path) => format!("{path}: cannot write to the destination"),
        }
    }
}

/// A batch ready to be audited and applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    /// How to dispose of the files.
    pub mode: Mode,
    /// The files in the batch.
    pub candidates: Vec<Candidate>,
}

/// Where an applied file went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Location {
    /// It is in the operating system's trash.
    Trash,
    /// It was moved to this path.
    Moved(PathBuf),
    /// It was left exactly where it was.
    Kept,
}

/// One file the batch acted on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    /// Catalog id.
    pub id: PhotoId,
    /// Where it was.
    pub from: PathBuf,
    /// Where it went.
    pub location: Location,
}

/// A batch that was applied, kept so the rejects bin can list it and restore it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedBatch {
    /// Monotonic id within the library.
    pub id: u64,
    /// Unix seconds when it was applied.
    pub applied_at: i64,
    /// How the files were disposed of (`trash`, `tag`, or `move:<folder>`).
    pub mode: String,
    /// The files the batch acted on.
    pub items: Vec<Applied>,
    /// The flag each photo carried before the batch, so restoring is exact.
    pub previous_flags: Vec<(PhotoId, sortlens_core::Flag)>,
    /// How many files failed.
    pub failed: usize,
}

impl DeletedBatch {
    /// A readable summary for the bin row.
    pub fn summary(&self) -> String {
        let noun = if self.items.len() == 1 { "photo" } else { "photos" };
        let mut text = format!("{} {noun}", self.items.len());
        if self.mode == "tag" {
            text.push_str(" · kept on disk");
        }
        if self.failed > 0 {
            text.push_str(&format!(" · {} failed", self.failed));
        }
        text
    }
}

/// What happened to the batch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Files that were disposed of.
    pub applied: Vec<Applied>,
    /// Files that failed, with the reason.
    pub failed: Vec<(PhotoId, String)>,
}

impl Report {
    /// Whether anything at all went wrong.
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }
}

/// What a [`Receipt`]'s headline says.
///
/// A translated receipt cannot join an English sentence with its own words without contradicting
/// the count beside it, so the dialog is handed this piece instead — the same shape the cull
/// summary sheet's `Headline` gave its own sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptHeadline {
    /// The batch handled this many files.
    Handled {
        /// Files the batch disposed of.
        photos: usize,
    },
}

impl ReceiptHeadline {
    /// This headline in English.
    pub fn english(&self) -> String {
        match *self {
            Self::Handled { photos } => {
                let noun = if photos == 1 { "photo" } else { "photos" };
                format!("{photos} {noun} handled")
            }
        }
    }
}

/// Where a batch put the files it handled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// The system Trash.
    Trash,
    /// Left where they were, flagged as rejected.
    KeptOnDisk,
    /// Moved into this folder.
    Folder(String),
}

/// One piece of a [`Receipt`]'s detail line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailPart {
    /// Where the files went.
    Destination(Destination),
    /// Picks the batch also wrote.
    Picks(usize),
    /// Files that could not be handled.
    Failed(usize),
}

impl DetailPart {
    /// This piece in English.
    pub fn english(&self) -> String {
        match self {
            Self::Destination(Destination::Trash) => "Moved to the system Trash".to_owned(),
            Self::Destination(Destination::KeptOnDisk) => "Left on disk and flagged as rejected".to_owned(),
            Self::Destination(Destination::Folder(folder)) => format!("Moved to {folder}"),
            Self::Picks(count) => format!("{count} pick(s) written"),
            Self::Failed(count) => format!("{count} failed"),
        }
    }
}

/// What one Apply did, in the terms the receipt dialog uses.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Receipt {
    /// Batch id in the rejects bin.
    pub batch_id: u64,
    /// Mode id: `trash`, `tag` or `move:<folder>`.
    pub mode: String,
    /// Files the batch disposed of; picks are counted separately.
    pub applied: usize,
    /// Files that could not be disposed of.
    pub failed: usize,
    /// Rejects that stayed on disk because the mode was tag-only.
    pub kept: usize,
    /// Picks the batch also wrote.
    pub picks: usize,
    /// Unix seconds when it finished.
    pub at: i64,
}

impl Receipt {
    /// Build the receipt for a finished batch.
    pub fn of(batch: &DeletedBatch, picks: usize) -> Self {
        Self {
            batch_id: batch.id,
            mode: batch.mode.clone(),
            applied: batch.items.len(),
            failed: batch.failed,
            kept: batch.items.iter().filter(|item| matches!(item.location, Location::Kept)).count(),
            picks,
            at: batch.applied_at,
        }
    }

    /// The headline as a piece: what a language other than English words for itself.
    ///
    /// [`Receipt::headline`] is joined from this, so the English sentence the report prints and
    /// the piece a translated receipt paints cannot disagree about the count.
    pub const fn headline_kind(&self) -> ReceiptHeadline {
        ReceiptHeadline::Handled { photos: self.applied }
    }

    /// A one-line headline for the dialog title.
    pub fn headline(&self) -> String {
        self.headline_kind().english()
    }

    /// Where this batch put the files.
    pub fn destination(&self) -> Destination {
        if let Some(folder) = self.mode.strip_prefix("move:") {
            Destination::Folder(folder.to_owned())
        } else if self.mode == "tag" {
            Destination::KeptOnDisk
        } else {
            Destination::Trash
        }
    }

    /// What happened to the files, as the pieces a receipt is worded from, in reading order.
    pub fn detail_parts(&self) -> Vec<DetailPart> {
        let mut parts = vec![DetailPart::Destination(self.destination())];
        if self.picks > 0 {
            parts.push(DetailPart::Picks(self.picks));
        }
        if self.failed > 0 {
            parts.push(DetailPart::Failed(self.failed));
        }
        parts
    }

    /// What happened to the files, as one sentence.
    pub fn detail(&self) -> String {
        self.detail_parts().iter().map(DetailPart::english).collect::<Vec<_>>().join(" · ")
    }

    /// Whether the batch can be put back.
    pub const fn is_restorable(&self) -> bool {
        self.applied > 0 && self.failed == 0
    }
}

/// Check the batch before anything is touched.
///
/// Returns every refusal it finds, in batch order. An empty `Ok` means the batch may start.
pub fn audit(batch: &Batch) -> std::result::Result<(), Vec<Refusal>> {
    let mut refusals = Vec::new();
    for candidate in &batch.candidates {
        if let Some(refusal) = audit_one(candidate) {
            refusals.push(refusal);
        }
    }
    if let Mode::Move(folder) = &batch.mode
        && let Some(refusal) = audit_target(folder, &batch.candidates)
    {
        refusals.push(refusal);
    }
    if refusals.is_empty() { Ok(()) } else { Err(refusals) }
}

fn audit_one(candidate: &Candidate) -> Option<Refusal> {
    let path = candidate.path.display().to_string();
    let root = candidate.root.display().to_string();
    // Rule 1: the file lives inside a watched root.
    if !candidate.path.starts_with(&candidate.root) {
        return Some(Refusal::OutsideRoot { path: path.clone(), root });
    }
    // Rule 4: a symlink may not lead out of the root.
    if let Ok(resolved) = candidate.path.canonicalize()
        && let Ok(resolved_root) = candidate.root.canonicalize()
        && !resolved.starts_with(&resolved_root)
    {
        return Some(Refusal::SymlinkEscape(path));
    }
    // Rule 2: it exists and is what the catalog saw.
    let meta = match fs::metadata(&candidate.path) {
        Ok(meta) => meta,
        Err(_) => return Some(Refusal::Missing(path)),
    };
    if meta.len() != candidate.size {
        return Some(Refusal::Changed {
            path,
            expected: candidate.size,
            actual: meta.len(),
        });
    }
    if let (Some(expected), Ok(modified)) = (candidate.modified, meta.modified())
        && let Ok(since) = modified.duration_since(std::time::UNIX_EPOCH)
        && (since.as_secs() as i64) != expected
    {
        return Some(Refusal::Modified {
            path,
            indexed: expected,
            on_disk: since.as_secs() as i64,
        });
    }
    None
}

fn audit_target(folder: &Path, candidates: &[Candidate]) -> Option<Refusal> {
    if let Err(error) = fs::create_dir_all(folder) {
        return Some(Refusal::TargetUnwritable(format!("{}: {error}", folder.display())));
    }
    let probe = folder.join(".sortlens-write-test");
    if let Err(error) = fs::write(&probe, b"ok") {
        return Some(Refusal::TargetUnwritable(format!("{}: {error}", folder.display())));
    }
    let _ = fs::remove_file(&probe);
    // A destination inside a watched root would make rejects reappear on the next scan.
    for candidate in candidates {
        if folder.starts_with(&candidate.root) {
            return Some(Refusal::OutsideRoot {
                path: folder.display().to_string(),
                root: candidate.root.display().to_string(),
            });
        }
    }
    None
}

/// Apply an audited batch.
///
/// Each file is re-checked immediately before it is touched, so a file that changed between audit
/// and apply is reported instead of deleted. Nothing here is retried automatically.
pub fn apply(batch: &Batch) -> Report {
    let mut report = Report::default();
    for candidate in &batch.candidates {
        if let Some(refusal) = audit_one(candidate) {
            report.failed.push((candidate.id, refusal.message()));
            continue;
        }
        let outcome = match &batch.mode {
            Mode::TagOnly => Ok(Location::Kept),
            Mode::Trash => delete_to_trash(candidate).map(|()| Location::Trash),
            Mode::Move(folder) => move_into(candidate, folder).map(Location::Moved),
        };
        match outcome {
            Ok(location) => report.applied.push(Applied {
                id: candidate.id,
                from: candidate.path.clone(),
                location,
            }),
            Err(message) => report
                .failed
                .push((candidate.id, format!("{}: {message}", candidate.path.display()))),
        }
    }
    report
}

/// The library's own trash folder, used on the platforms with no crate-accessible OS trash
/// (Android, iOS). Its first component is a name `crates/scan` skips (`SKIP_DIR_NAMES` contains
/// `".sortlens"`), so a trashed photograph never reappears in the grid; `crates/engine`'s tests
/// hold the two sides of that contract together by scanning a library that contains it.
pub const PHONE_TRASH: &str = ".sortlens/trash";

/// The desktop's trash: the operating system's own, through the `trash` crate.
#[cfg(any(target_os = "windows", target_os = "macos", all(target_os = "linux", not(target_os = "android"))))]
fn delete_to_trash(candidate: &Candidate) -> std::result::Result<(), String> {
    trash::delete(&candidate.path).map_err(|error| error.to_string())
}

/// The phone's trash: the same audited move the `Move` mode uses, into [`PHONE_TRASH`], so a
/// rejected photograph is re-checkable, size-verified and reversible exactly as on the desktop,
/// and nothing is destroyed that a mistake could not bring back.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn delete_to_trash(candidate: &Candidate) -> std::result::Result<(), String> {
    move_to_phone_trash(candidate).map(|_| ())
}

/// Move one candidate into the library's own trash folder (see [`PHONE_TRASH`]). Compiled and
/// tested on every platform, so the phone path is host-testable; only `delete_to_trash`'s
/// Android/iOS arm calls it in production.
#[cfg(any(test, target_os = "android", target_os = "ios"))]
pub(crate) fn move_to_phone_trash(candidate: &Candidate) -> std::result::Result<(), String> {
    // The re-check the audited path promises: a file whose bytes or mtime moved since it was
    // indexed is refused here, not moved, even when this is called outside `apply`.
    if let Some(refusal) = audit_one(candidate) {
        return Err(refusal.message());
    }
    let folder = candidate.root.join(PHONE_TRASH);
    move_into(candidate, &folder).map(|_| ())
}

fn move_into(candidate: &Candidate, folder: &Path) -> std::result::Result<PathBuf, String> {
    let relative = candidate
        .path
        .strip_prefix(&candidate.root)
        .map_err(|_| "not inside its root".to_string())?;
    let destination = folder.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    if let Err(error) = fs::rename(&candidate.path, &destination) {
        // A different volume cannot be renamed across: copy, verify the size, then remove.
        fs::copy(&candidate.path, &destination).map_err(|copy| format!("{}: {error}; copy: {copy}", destination.display()))?;
        let copied = fs::metadata(&destination).map_err(|error| error.to_string())?.len();
        if copied != candidate.size {
            let _ = fs::remove_file(&destination);
            return Err(format!(
                "{}: copy is {copied} bytes, expected {}",
                destination.display(),
                candidate.size
            ));
        }
        fs::remove_file(&candidate.path).map_err(|remove| format!("{}: {remove}", candidate.path.display()))?;
    }
    Ok(destination)
}

/// Put moved files back. Trashed files cannot be restored by Sortlens, and say so.
pub fn undo(applied: &[Applied]) -> Report {
    let mut report = Report::default();
    for entry in applied {
        match &entry.location {
            Location::Kept => {}
            Location::Moved(destination) => {
                if let Some(parent) = entry.from.parent()
                    && let Err(error) = fs::create_dir_all(parent)
                {
                    report.failed.push((entry.id, format!("{}: {error}", parent.display())));
                    continue;
                }
                match fs::rename(destination, &entry.from) {
                    Ok(()) => {}
                    Err(error) => report.failed.push((
                        entry.id,
                        format!("{}: {error} — restore it from the Trash yourself", destination.display()),
                    )),
                }
            }
            Location::Trash => report.failed.push((
                entry.id,
                format!("{}: restore this file from the Trash yourself", entry.from.display()),
            )),
        }
    }
    report
}

/// Convenience for the engine: refuse a batch whose audit failed.
pub fn apply_checked(batch: &Batch) -> Result<Report> {
    if let Err(refusals) = audit(batch) {
        let first = refusals.first().map(Refusal::message).unwrap_or_default();
        return Err(Error::Invalid(format!(
            "{} file(s) failed the safety checks: {first}",
            refusals.len()
        )));
    }
    Ok(apply(batch))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    #[test]
    fn a_deleted_batch_round_trips_through_json() {
        let batch = DeletedBatch {
            id: 4,
            applied_at: 1_700_000_000,
            mode: "move:/tmp/rejects".into(),
            items: vec![Applied {
                id: PhotoId(9),
                from: PathBuf::from("/photos/a.jpg"),
                location: Location::Moved(PathBuf::from("/tmp/rejects/a.jpg")),
            }],
            previous_flags: vec![(PhotoId(9), sortlens_core::Flag::None)],
            failed: 0,
        };
        let json = serde_json::to_string(&batch).expect("serialise");
        let back: DeletedBatch = serde_json::from_str(&json).expect("parse");
        assert_eq!(back, batch);
        assert_eq!(back.summary(), "1 photo");
    }

    use super::*;

    /// The receipt's English is joined from its pieces, so a translated receipt and the record
    /// the report prints cannot describe different batches.
    #[test]
    fn the_receipt_english_is_joined_from_its_pieces() {
        let mut receipt = Receipt {
            batch_id: 7,
            mode: "move:/photos/rejects".into(),
            applied: 2,
            failed: 0,
            kept: 0,
            picks: 1,
            at: 1_700_000_000,
        };
        assert_eq!(receipt.headline_kind(), ReceiptHeadline::Handled { photos: 2 });
        assert_eq!(receipt.headline(), "2 photos handled");
        assert_eq!(receipt.headline(), receipt.headline_kind().english());
        assert_eq!(receipt.destination(), Destination::Folder("/photos/rejects".into()));
        assert_eq!(
            receipt.detail_parts(),
            vec![
                DetailPart::Destination(Destination::Folder("/photos/rejects".into())),
                DetailPart::Picks(1),
            ]
        );
        assert_eq!(receipt.detail(), "Moved to /photos/rejects · 1 pick(s) written");

        receipt.mode = "tag".into();
        receipt.applied = 1;
        receipt.picks = 0;
        receipt.failed = 2;
        assert_eq!(receipt.headline(), "1 photo handled");
        assert_eq!(receipt.headline(), receipt.headline_kind().english());
        assert_eq!(receipt.destination(), Destination::KeptOnDisk);
        assert_eq!(receipt.detail(), "Left on disk and flagged as rejected · 2 failed");

        receipt.mode = "trash".into();
        assert_eq!(receipt.destination(), Destination::Trash);
        assert_eq!(receipt.detail(), "Moved to the system Trash · 2 failed");
    }

    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sortlens-trash-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("dir");
            Self(dir)
        }

        fn file(&self, rel: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().expect("parent")).expect("parent");
            fs::write(&path, bytes).expect("write");
            path
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn candidate(id: u64, root: &Path, path: &Path) -> Candidate {
        let meta = fs::metadata(path).expect("meta");
        let modified = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|since| since.as_secs() as i64);
        Candidate {
            id: PhotoId(id),
            path: path.to_path_buf(),
            root: root.to_path_buf(),
            size: meta.len(),
            modified,
        }
    }

    #[test]
    fn an_intact_batch_audits_clean() {
        let temp = Temp::new("clean");
        let image = temp.file("a.jpg", b"photo");
        let batch = Batch {
            mode: Mode::TagOnly,
            candidates: vec![candidate(1, &temp.0, &image)],
        };
        assert_eq!(audit(&batch), Ok(()));
    }

    #[test]
    fn a_missing_file_is_refused() {
        let temp = Temp::new("missing");
        let image = temp.file("a.jpg", b"photo");
        let entry = candidate(1, &temp.0, &image);
        fs::remove_file(&image).expect("remove");
        let batch = Batch {
            mode: Mode::TagOnly,
            candidates: vec![entry],
        };
        let refusals = audit(&batch).expect_err("refused");
        assert!(matches!(refusals[0], Refusal::Missing(_)));
    }

    /// A rewrite that keeps the size is still refused, and the message says which fact changed.
    #[test]
    fn a_file_whose_timestamp_moved_is_refused_and_says_so() {
        let temp = Temp::new("modified");
        let image = temp.file("a.jpg", b"photo");
        let mut entry = candidate(1, &temp.0, &image);
        // The same length, so only the timestamp can be what the audit notices.
        fs::write(&image, b"PHOTO").expect("rewrite");
        let file = fs::OpenOptions::new().write(true).open(&image).expect("open");
        file.set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(9_999)))
            .expect("mtime");
        drop(file);
        entry.modified = Some(1_000);
        let batch = Batch {
            mode: Mode::TagOnly,
            candidates: vec![entry],
        };
        let refusals = audit(&batch).expect_err("refused");
        assert_eq!(
            refusals,
            vec![Refusal::Modified {
                path: image.display().to_string(),
                indexed: 1_000,
                on_disk: 9_999,
            }]
        );
        let message = refusals[0].message();
        assert!(message.contains("modified since it was indexed"), "{message}");
        assert!(
            !message.contains("0 → 0 bytes"),
            "a timestamp refusal must not be dressed up as a size change: {message}"
        );
    }

    #[test]
    fn a_file_that_changed_since_it_was_indexed_is_refused() {
        let temp = Temp::new("changed");
        let image = temp.file("a.jpg", b"photo");
        let entry = candidate(1, &temp.0, &image);
        fs::write(&image, b"photo with new edits").expect("append");
        let batch = Batch {
            mode: Mode::TagOnly,
            candidates: vec![entry],
        };
        let refusals = audit(&batch).expect_err("refused");
        assert!(matches!(refusals[0], Refusal::Changed { .. }), "{:?}", refusals[0]);
    }

    #[test]
    fn a_file_outside_its_root_is_refused() {
        let temp = Temp::new("outside");
        let other = Temp::new("outside-other");
        let image = other.file("a.jpg", b"photo");
        let batch = Batch {
            mode: Mode::TagOnly,
            candidates: vec![candidate(1, &temp.0, &image)],
        };
        let refusals = audit(&batch).expect_err("refused");
        assert!(matches!(refusals[0], Refusal::OutsideRoot { .. }));
    }

    #[test]
    fn a_destination_inside_a_watched_root_is_refused() {
        let temp = Temp::new("dest-inside");
        let image = temp.file("a.jpg", b"photo");
        let batch = Batch {
            mode: Mode::Move(temp.0.join("rejects")),
            candidates: vec![candidate(1, &temp.0, &image)],
        };
        let refusals = audit(&batch).expect_err("refused");
        assert!(refusals.iter().any(|refusal| matches!(refusal, Refusal::OutsideRoot { .. })));
        assert!(image.exists(), "nothing was moved");
    }

    #[test]
    fn moving_preserves_the_relative_path_and_undo_restores_it() {
        let temp = Temp::new("move");
        let image = temp.file("shoot/day1/a.jpg", b"photo");
        let destination = std::env::temp_dir().join(format!("sortlens-trash-move-dest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&destination);

        let batch = Batch {
            mode: Mode::Move(destination.clone()),
            candidates: vec![candidate(1, &temp.0, &image)],
        };
        let report = apply_checked(&batch).expect("applied");
        assert_eq!(report.applied.len(), 1);
        assert!(!image.exists());
        let moved = destination.join("shoot/day1/a.jpg");
        assert!(moved.exists(), "relative path preserved");

        let undo_report = undo(&report.applied);
        assert!(!undo_report.has_failures(), "{:?}", undo_report.failed);
        assert!(image.exists(), "back where it came from");
        assert!(!moved.exists());

        let _ = fs::remove_dir_all(&destination);
    }

    #[test]
    fn tag_only_moves_nothing() {
        let temp = Temp::new("tag-only");
        let image = temp.file("a.jpg", b"photo");
        let batch = Batch {
            mode: Mode::TagOnly,
            candidates: vec![candidate(1, &temp.0, &image)],
        };
        let report = apply_checked(&batch).expect("applied");
        assert_eq!(report.applied[0].location, Location::Kept);
        assert!(image.exists());
    }

    #[test]
    fn a_refused_batch_reports_why_and_touches_nothing() {
        let temp = Temp::new("refused");
        let image = temp.file("a.jpg", b"photo");
        let entry = candidate(1, &temp.0, &image);
        fs::remove_file(&image).expect("remove");
        let batch = Batch {
            mode: Mode::TagOnly,
            candidates: vec![entry],
        };
        let error = apply_checked(&batch).expect_err("refused");
        assert!(error.to_string().contains("safety checks"), "{error}");
    }

    #[test]
    fn trash_undo_tells_the_user_to_restore_it_themselves() {
        let temp = Temp::new("trash-undo");
        let image = temp.file("a.jpg", b"photo");
        let applied = vec![Applied {
            id: PhotoId(1),
            from: image,
            location: Location::Trash,
        }];
        let report = undo(&applied);
        assert_eq!(report.failed.len(), 1);
        assert!(report.failed[0].1.contains("Trash"));
    }

    /// The phone's trash is a real move into the library's own `.sortlens/trash`, with the same
    /// re-check and size verification a `Move` gets, and the undo that puts the file back. This
    /// runs on the host on every platform, even though only the Android/iOS arm calls it in
    /// production, because a phone path nobody can test would be exactly the placeholder the
    /// plan forbids.
    #[test]
    fn the_phone_trash_moves_into_the_librarys_own_skipped_folder() {
        let temp = Temp::new("phone-trash");
        let image = temp.file("roll/day2/b.jpg", b"phone photo");
        let photo = candidate(1, &temp.0, &image);

        move_to_phone_trash(&photo).expect("moved to the phone's trash");
        assert!(!image.exists(), "gone from where it was");
        let trashed = temp.0.join(".sortlens").join("trash").join("roll/day2/b.jpg");
        assert!(trashed.exists(), "the relative path is preserved under .sortlens/trash");
        assert_eq!(fs::read(&trashed).expect("read back"), b"phone photo");

        // A file whose bytes moved since it was indexed is refused, not moved: the same guard
        // the desktop's Move mode has.
        let image2 = temp.file("roll/day2/c.jpg", b"four bytes");
        let mut changed = candidate(2, &temp.0, &image2);
        fs::write(&image2, b"a different length now").expect("rewrite");
        changed.size = 4;
        assert!(move_to_phone_trash(&changed).is_err(), "a changed file is refused");
        assert!(image2.exists(), "the refused file is still where it was");

        // The receipt says Trash, and `undo` refuses a Trash receipt the way it does on the
        // desktop: the file sits in `.sortlens/trash` for a person to recover, not in a
        // location Sortlens will silently move back.
        let applied = vec![Applied {
            id: PhotoId(1),
            from: trashed.clone(),
            location: Location::Trash,
        }];
        let undo_report = undo(&applied);
        assert_eq!(undo_report.failed.len(), 1);
        assert!(undo_report.failed[0].1.contains("Trash"));
        assert!(trashed.exists(), "the undo leaves the trashed file alone");
    }

    #[test]
    fn the_phone_trash_folder_name_is_the_one_the_scanner_skips() {
        // The contract `crates/engine`'s scan test holds both ends of: the folder this crate
        // moves into starts with the name `crates/scan` refuses to descend into.
        assert!(PHONE_TRASH.starts_with(".sortlens"), "{PHONE_TRASH}");
    }
}
