//! One-file backups of a library's catalog.
//!
//! A backup is a JSON document holding the catalog's own text files, each with its CRC32, so a
//! truncated download or a hand-edited file is refused instead of half-restored. There is no
//! compression and no binary encoding: the catalog's files are already text, and a format you can
//! read with `less` is worth more here than a few saved kilobytes.
//!
//! What a backup does *not* contain is photos. Sortlens never copies image files, so restoring a
//! backup brings back ratings, keywords, collections and the index — not pixels.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::store::{LOG_NAME, SNAPSHOT_NAME};
use sortlens_core::{Error, Result};

/// Format version. A newer archive is refused rather than guessed at.
pub const SCHEMA: u32 = 1;

/// The files a backup carries, in the order they are restored.
const CARRIED: [&str; 2] = [SNAPSHOT_NAME, LOG_NAME];

/// One file inside a backup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupFile {
    /// File name as it sits in the library directory.
    pub name: String,
    /// CRC32 of `text`, checked on read.
    pub crc: u32,
    /// The file's contents.
    pub text: String,
}

/// A whole-library backup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backup {
    /// Format version.
    pub schema: u32,
    /// Unix seconds when it was written.
    pub created: i64,
    /// The catalog's files.
    pub files: Vec<BackupFile>,
}

impl Backup {
    /// How many files it carries.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether it carries nothing; never true for a backup that came from [`read`].
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// The names it carries, for a restore report.
    pub fn names(&self) -> Vec<&str> {
        self.files.iter().map(|file| file.name.as_str()).collect()
    }

    /// Whether it holds an op log, i.e. changes since the last snapshot.
    pub fn has_log(&self) -> bool {
        self.files.iter().any(|file| file.name == LOG_NAME)
    }
}

/// Read a library directory into a backup.
///
/// A library with no snapshot yet (nothing scanned) is an error: there is nothing worth carrying.
pub fn create(dir: &Path, created: i64) -> Result<Backup> {
    let mut files = Vec::new();
    for name in CARRIED {
        let path = dir.join(name);
        match std::fs::read_to_string(&path) {
            Ok(text) => files.push(BackupFile {
                name: name.to_string(),
                crc: crc32fast::hash(text.as_bytes()),
                text,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(Error::io(path.display().to_string(), error)),
        }
    }
    if files.is_empty() {
        return Err(Error::Invalid("this library has no catalog to back up yet".into()));
    }
    Ok(Backup {
        schema: SCHEMA,
        created,
        files,
    })
}

/// Serialise a backup for writing to disk.
pub fn encode(backup: &Backup) -> Result<Vec<u8>> {
    serde_json::to_vec_pretty(backup).map_err(|error| Error::Invalid(format!("backup: {error}")))
}

/// Parse and verify a backup.
///
/// Every file's checksum is checked here rather than at restore time, so a corrupt archive is
/// reported before anything is written.
pub fn read(bytes: &[u8]) -> Result<Backup> {
    let backup: Backup = serde_json::from_slice(bytes).map_err(|error| Error::Invalid(format!("not a Sortlens backup: {error}")))?;
    if backup.schema > SCHEMA {
        return Err(Error::Invalid(format!(
            "this backup is from a newer Sortlens (format {}, this build reads {SCHEMA})",
            backup.schema
        )));
    }
    if backup.files.is_empty() {
        return Err(Error::Invalid("the backup is empty".into()));
    }
    for file in &backup.files {
        if !CARRIED.contains(&file.name.as_str()) {
            return Err(Error::Invalid(format!("unexpected file in backup: {}", file.name)));
        }
        if crc32fast::hash(file.text.as_bytes()) != file.crc {
            return Err(Error::Invalid(format!("{} failed its checksum", file.name)));
        }
    }
    Ok(backup)
}

/// Write a verified backup into `dest`, creating the directory if needed.
///
/// Files are written to a temporary name and renamed, so an interrupted restore cannot leave a
/// half-written catalog behind. Returns the names written.
pub fn restore(backup: &Backup, dest: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(dest).map_err(|error| Error::io(dest.display().to_string(), error))?;
    let mut written = Vec::new();
    for file in &backup.files {
        let path: PathBuf = dest.join(&file.name);
        let tmp = dest.join(format!("{}.restore", file.name));
        std::fs::write(&tmp, file.text.as_bytes()).map_err(|error| Error::io(tmp.display().to_string(), error))?;
        std::fs::rename(&tmp, &path).map_err(|error| Error::io(path.display().to_string(), error))?;
        written.push(file.name.clone());
    }
    Ok(written)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::store::{Catalog, Seed};
    use sortlens_core::{MediaKind, PhotoId};

    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sortlens-backup-{name}-{}", std::process::id()));
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

    fn populated(name: &str) -> Temp {
        let temp = Temp::new(name);
        {
            let (mut catalog, _) = Catalog::open(&temp.0).expect("open");
            catalog.sync_root("/photos", vec![seed("a.jpg")]).expect("sync");
            catalog
                .commit(crate::op::Op::Rate {
                    changes: vec![(PhotoId(1), 4)],
                })
                .expect("rate");
            catalog.close().expect("close");
        }
        temp
    }

    #[test]
    fn a_backup_round_trips_and_restores_the_same_catalog() {
        let source = populated("roundtrip");
        let backup = create(&source.0, 1_700_000_000).expect("create");
        assert!(backup.names().contains(&SNAPSHOT_NAME));
        assert!(!backup.is_empty());

        let bytes = encode(&backup).expect("encode");
        let read_back = read(&bytes).expect("read");
        assert_eq!(read_back, backup);
        assert_eq!(read_back.created, 1_700_000_000);

        let dest = Temp::new("restored");
        let written = restore(&read_back, &dest.0).expect("restore");
        assert!(written.contains(&SNAPSHOT_NAME.to_string()));

        let (mut catalog, _) = Catalog::open(&dest.0).expect("reopen");
        let photo = catalog.photo(PhotoId(1)).expect("photo");
        assert_eq!(photo.rating, 4, "the rating came back");
        assert_eq!(catalog.stats().total, 1);
    }

    #[test]
    fn a_tampered_file_is_refused_before_anything_is_written() {
        let source = populated("tampered");
        let mut backup = create(&source.0, 0).expect("create");
        backup.files[0].text.push(' ');
        assert!(read(&encode(&backup).expect("encode")).is_err(), "checksum mismatch");
    }

    #[test]
    fn a_newer_format_is_refused_with_a_readable_message() {
        let source = populated("newer");
        let mut backup = create(&source.0, 0).expect("create");
        backup.schema = SCHEMA + 1;
        let error = read(&encode(&backup).expect("encode")).expect_err("refused");
        assert!(error.to_string().contains("newer Sortlens"), "{error}");
    }

    #[test]
    fn unknown_files_and_empty_archives_are_refused() {
        let source = populated("odd");
        let mut backup = create(&source.0, 0).expect("create");
        backup.files.push(BackupFile {
            name: "thumbs.db".into(),
            crc: crc32fast::hash(b"x"),
            text: "x".into(),
        });
        assert!(read(&encode(&backup).expect("encode")).is_err(), "unexpected file");

        let empty = Backup {
            schema: SCHEMA,
            created: 0,
            files: Vec::new(),
        };
        assert!(read(&encode(&empty).expect("encode")).is_err(), "empty archive");
    }

    #[test]
    fn an_empty_library_has_nothing_to_back_up() {
        let temp = Temp::new("empty");
        assert!(create(&temp.0, 0).is_err());
    }

    #[test]
    fn a_library_without_a_log_still_backs_up() {
        let source = populated("snapshot-only");
        // Closing writes a snapshot and empties the log, so only the snapshot has content.
        std::fs::write(source.0.join(LOG_NAME), "").expect("empty log");
        let backup = create(&source.0, 0).expect("create");
        assert!(backup.has_log(), "the empty log is carried");
        let restored = read(&encode(&backup).expect("encode")).expect("read");
        let dest = Temp::new("snapshot-only-restored");
        restore(&restored, &dest.0).expect("restore");
        assert!(Catalog::open(&dest.0).is_ok());
    }
}
