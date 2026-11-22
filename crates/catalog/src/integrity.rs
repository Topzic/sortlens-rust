//! Checking that a library's catalog files agree with each other.
//!
//! An audit answers one question honestly: *can this library be read back exactly?* It does that by
//! copying the two catalog files to a scratch directory and opening them there, which is the only
//! way to know for sure — the same code that a normal open runs, with none of its side effects on
//! the library itself.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::store::{Catalog, LOG_NAME, SNAPSHOT_NAME};
use sortlens_core::{Error, Result};

/// What an audit found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Audit {
    /// Sequence number of the snapshot, if there was one.
    pub snapshot_seq: Option<u64>,
    /// Sequence number reached after replaying the log.
    pub final_seq: u64,
    /// Records replayed after the snapshot.
    pub replayed: usize,
    /// Records that were already covered by the snapshot.
    pub skipped: usize,
    /// Bytes in the log.
    pub log_bytes: u64,
    /// Photos after replay.
    pub photos: usize,
    /// Collections after replay.
    pub collections: usize,
    /// The log ended in a record that failed its checksum and was dropped.
    pub torn_tail: bool,
    /// Anything worth telling the photographer, in order.
    pub notes: Vec<String>,
}

impl Audit {
    /// Whether nothing needs attention.
    pub fn is_clean(&self) -> bool {
        !self.torn_tail && self.notes.is_empty()
    }

    /// One line for a dialog title or a CLI.
    pub fn headline(&self) -> String {
        if self.is_clean() {
            format!("Clean · {} photos · seq {}", self.photos, self.final_seq)
        } else if self.torn_tail {
            format!("Recoverable · {} photos, {} record(s) replayed", self.photos, self.replayed)
        } else {
            "Needs attention".to_string()
        }
    }

    /// The detail behind the headline.
    pub fn detail(&self) -> String {
        format!(
            "snapshot at seq {}, {} record(s) replayed, {} already covered, {} log bytes",
            self.snapshot_seq.map_or_else(|| "none".to_string(), |seq| seq.to_string()),
            self.replayed,
            self.skipped,
            self.log_bytes
        )
    }
}

/// Audit the catalog in `dir` without changing it.
pub fn audit(dir: &Path) -> Result<Audit> {
    let log_path = dir.join(LOG_NAME);
    let snapshot_path = dir.join(SNAPSHOT_NAME);
    if !log_path.exists() && !snapshot_path.exists() {
        return Err(Error::Invalid("this library has no catalog yet".into()));
    }

    let scratch = Scratch::new()?;
    for name in [SNAPSHOT_NAME, LOG_NAME] {
        let source = dir.join(name);
        if source.exists() {
            let bytes = std::fs::read(&source).map_err(|error| Error::io(source.display().to_string(), error))?;
            let target = scratch.path().join(name);
            std::fs::write(&target, bytes).map_err(|error| Error::io(target.display().to_string(), error))?;
        }
    }

    let mut audit = Audit {
        log_bytes: std::fs::metadata(&log_path).map(|meta| meta.len()).unwrap_or(0),
        ..Audit::default()
    };
    // A corrupt snapshot is not fatal: open() falls back to the log, and the audit says so.
    let (mut catalog, report) = Catalog::open(scratch.path())?;
    audit.snapshot_seq = (report.snapshot_seq > 0).then_some(report.snapshot_seq);
    audit.replayed = report.replayed;
    audit.skipped = report.skipped;
    audit.torn_tail = report.torn_tail;
    audit.final_seq = catalog.seq();
    audit.photos = catalog.stats().total;
    audit.collections = catalog.collections().len();
    audit.notes = report.warnings;
    Ok(audit)
}

/// A directory that deletes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Self> {
        let path = std::env::temp_dir().join(format!(
            "sortlens-audit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.subsec_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&path).map_err(|error| Error::io(path.display().to_string(), error))?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::op::Op;
    use crate::store::Seed;
    use sortlens_core::{MediaKind, PhotoId};

    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sortlens-integrity-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("dir");
            Self(dir)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn seed(name: &str) -> Seed {
        Seed {
            rel: name.to_string(),
            size: 10,
            modified: Some(1),
            kind: MediaKind::Image,
            key: 7,
            meta: sortlens_meta::PhotoMeta::default(),
            review: None,
            edited: false,
        }
    }

    fn library(name: &str, photos: usize) -> Temp {
        let temp = Temp::new(name);
        let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
        let seeds = (0..photos).map(|index| seed(&format!("p{index}.jpg"))).collect();
        catalog.sync_root("/photos", seeds).expect("sync");
        catalog.close().expect("close");
        temp
    }

    #[test]
    fn a_fresh_library_audits_clean() {
        let temp = library("clean", 3);
        let audit = audit(&temp.0).expect("audit");
        assert!(audit.is_clean(), "{audit:?}");
        assert_eq!(audit.photos, 3);
        assert!(audit.headline().contains("Clean"));
        assert!(audit.replayed > 0, "a library that never snapshot replays its log");
    }

    #[test]
    fn a_log_with_records_replays_and_reports_them() {
        let temp = library("replay", 2);
        {
            let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 5)],
                })
                .expect("rate");
        }
        let audit = audit(&temp.0).expect("audit");
        assert!(audit.is_clean(), "{audit:?}");
        assert!(audit.replayed > 1, "the index pass plus the rating");
        assert!(audit.log_bytes > 0);
    }

    #[test]
    fn a_torn_tail_is_reported_but_the_rest_replays() {
        let temp = library("torn", 2);
        {
            let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 4)],
                })
                .expect("rate");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(2), 3)],
                })
                .expect("rate");
            catalog.flush().expect("flush");
        }
        // Chop the last record in half, which is what a power cut leaves behind.
        let path = temp.0.join(LOG_NAME);
        let text = std::fs::read_to_string(&path).expect("read");
        std::fs::write(&path, &text[..text.len() - 6]).expect("truncate");

        let audit = audit(&temp.0).expect("audit");
        assert!(audit.torn_tail, "the damage is named");
        assert!(!audit.is_clean());
        assert!(audit.headline().contains("Recoverable"), "{}", audit.headline());

        // Compaction keeps the readable prefix and starts a fresh log.
        let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
        assert_eq!(catalog.photo(PhotoId(1)).expect("photo").rating, 4);
        let before = catalog.log_bytes();
        catalog.compact().expect("compact");
        assert_eq!(catalog.log_bytes(), 0);
        assert!(before > 0);
        assert!(super::audit(&temp.0).expect("audit again").is_clean(), "compaction repaired it");
        assert_eq!(catalog.photo(PhotoId(2)).expect("photo").rating, 0, "the dropped record is gone");
    }

    #[test]
    fn an_empty_folder_is_refused_and_compaction_reports_what_it_reclaimed() {
        let empty = Temp::new("empty");
        assert!(audit(&empty.0).is_err());

        let temp = library("compact", 2);
        {
            let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
            catalog
                .commit(Op::Rate {
                    changes: vec![(PhotoId(1), 2)],
                })
                .expect("rate");
        }
        let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
        let reclaimed = catalog.compact().expect("compact");
        assert!(reclaimed > 0, "the log had bytes to reclaim");
        let after = super::audit(&temp.0).expect("audit");
        assert!(after.is_clean());
        assert_eq!(after.replayed, 0, "everything is in the snapshot now");
    }
}
