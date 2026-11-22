//! Reading the op log back as a list of changes.
//!
//! The log is the audit trail: every change since the last snapshot is a record, newest last. This
//! module turns those records into something a person can read — one line per change, with the time
//! it happened — and into a plain-text report that can be attached to a support request without
//! sending a single photo.

#![forbid(unsafe_code)]

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::store::{LOG_NAME, open_records};
use sortlens_core::{Error, Result};

/// One line of history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Sequence number the record carries.
    pub seq: u64,
    /// Unix seconds when it was written; `0` for records that predate the timer.
    pub at: i64,
    /// The op kind, as the log names it.
    pub kind: String,
    /// A sentence describing the change.
    pub summary: String,
    /// How many photos the change touched, when that is meaningful.
    pub photos: usize,
}

impl Entry {
    /// Whether the record carries a time at all.
    pub fn is_dated(&self) -> bool {
        self.at > 0
    }
}

/// Read up to `limit` recent changes, newest first.
///
/// Only the log is read, which is what "recent" means: older changes were folded into the snapshot
/// when it was last written and are no longer individual records.
pub fn recent(dir: &Path, limit: usize) -> Result<Vec<Entry>> {
    let log_path = dir.join(LOG_NAME);
    let records = match open_records(&log_path) {
        Ok(records) => records,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(Error::io(log_path.display().to_string(), error)),
    };
    let mut entries: Vec<Entry> = records
        .into_iter()
        .filter_map(|line| crate::store::decode_record(&line))
        .map(|(seq, op, _inverse, at)| Entry {
            seq,
            at,
            kind: op.name().to_string(),
            summary: op.summary(),
            photos: op.photo_count(),
        })
        .collect();
    entries.reverse();
    entries.truncate(limit);
    Ok(entries)
}

/// A plain-text report of recent changes, for a support request.
pub fn export(dir: &Path, limit: usize, app_version: &str) -> Result<String> {
    let entries = recent(dir, limit)?;
    let mut text = String::new();
    text.push_str("Sortlens change report\n");
    text.push_str(&format!("app {app_version} · library {}\n", dir.display()));
    text.push_str(&format!("{} record(s) in the log\n", entries.len()));
    text.push_str("Older changes were folded into the snapshot and are not listed.\n");
    text.push_str("Folder and file names appear below; photo contents never do.\n\n");
    if entries.is_empty() {
        text.push_str("No changes in the log.\n");
        return Ok(text);
    }
    for entry in &entries {
        let when = if entry.is_dated() {
            format_seconds(entry.at)
        } else {
            "no date".to_string()
        };
        text.push_str(&format!("#{:<6} {when:>20}  {:<16} {}\n", entry.seq, entry.kind, entry.summary));
    }
    Ok(text)
}

/// `2026-11-21 09:14:03 UTC`, from unix seconds, without pulling in a date library.
///
/// A civil-date conversion is thirty lines of arithmetic; a dependency for one line of a support
/// report is not worth its build time or its advisories.
fn format_seconds(seconds: i64) -> String {
    if seconds <= 0 {
        return "no date".to_string();
    }
    let days = seconds / 86_400;
    let rest = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

/// Days since 1970-01-01 to a civil date. Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(m <= 2), m as u32, d as u32)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::op::Op;
    use crate::store::{Catalog, Seed};
    use sortlens_core::{ColorLabel, Flag, MediaKind, PhotoId};

    struct Temp(std::path::PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sortlens-history-{name}-{}", std::process::id()));
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

    #[test]
    fn recent_changes_come_back_newest_first_and_dated() {
        let temp = Temp::new("recent");
        let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
        catalog
            .sync_root("/photos", vec![seed("a.jpg"), seed("b.jpg"), seed("c.jpg")])
            .expect("sync");
        catalog
            .commit(Op::Rate {
                changes: vec![(PhotoId(1), 5), (PhotoId(2), 3)],
            })
            .expect("rate");
        catalog
            .commit(Op::Tagged {
                changes: vec![(PhotoId(1), vec!["beach".to_string()])],
            })
            .expect("tag");

        let entries = recent(&temp.0, 10).expect("recent");
        // An index pass writes a root record plus one per photo, so the count is whatever the sync
        // plus two changes add up to; what matters is the order and the wording.
        assert!(entries.len() > 2, "{entries:?}");
        assert_eq!(entries[0].kind, "tagged", "newest first");
        assert_eq!(entries[0].summary, "Keyworded 1 photo");
        assert_eq!(entries[1].summary, "Rated 2 photos");
        assert!(entries.iter().any(|entry| entry.summary.contains("Indexed")));
        assert!(entries.iter().all(Entry::is_dated), "records carry a time");
        assert_eq!(entries[1].photos, 2);

        let limited = recent(&temp.0, 1).expect("recent");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].seq, entries[0].seq);
    }

    #[test]
    fn an_empty_log_reads_as_no_changes() {
        let temp = Temp::new("empty");
        assert!(recent(&temp.0, 10).expect("recent").is_empty());
        let text = export(&temp.0, 10, "9.9.9").expect("export");
        assert!(text.contains("No changes in the log"));
        assert!(text.contains("app 9.9.9"));
    }

    #[test]
    fn the_report_names_every_change_and_its_time() {
        let temp = Temp::new("report");
        let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
        catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
        catalog
            .commit(Op::Flag {
                changes: vec![(PhotoId(1), Flag::Pick)],
            })
            .expect("flag");
        catalog
            .commit(Op::Label {
                changes: vec![(PhotoId(1), Some(ColorLabel::Red))],
            })
            .expect("label");

        let text = export(&temp.0, 50, "0.1.0").expect("export");
        assert!(text.starts_with("Sortlens change report"));
        assert!(text.contains("app 0.1.0"));
        let lower = text.to_lowercase();
        assert!(lower.contains("flagged"), "{text}");
        assert!(lower.contains("labelled"), "{text}");
        assert!(text.contains("UTC"), "records are dated");
        assert!(text.contains("Folder and file names appear below"), "the report says what it holds");
        assert!(!text.contains("\\x00"), "no binary");
    }

    #[test]
    fn dates_render_without_a_date_library() {
        assert_eq!(format_seconds(0), "no date");
        assert_eq!(format_seconds(1_700_000_000), "2023-11-14 22:13:20 UTC");
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn every_op_has_a_sentence() {
        // A summary is what the change viewer shows, so a new op must not arrive mute.
        let ops = [
            Op::AddRoot { path: "/p".into() },
            Op::RemoveRoot { path: "/p".into() },
            Op::Missing {
                id: PhotoId(1),
                missing: true,
            },
            Op::Relocate {
                changes: vec![(PhotoId(1), "a.jpg".into())],
            },
            Op::RemapRoot {
                from: "/a".into(),
                to: "/b".into(),
            },
            Op::RenameCollection {
                id: sortlens_core::CollectionId(1),
                name: "Trips".into(),
            },
            Op::SetCollectionFavourite {
                id: sortlens_core::CollectionId(1),
                favourite: true,
            },
            Op::Group { ops: Vec::new() },
        ];
        for op in ops {
            let summary = op.summary();
            assert!(!summary.is_empty(), "{} has no summary", op.name());
            assert!(summary.chars().next().is_some_and(char::is_uppercase), "{summary}");
        }
    }
}
