//! Folder walking and file identity.
//!
//! The walker is deliberately boring: one directory at a time, no symlink following, a fixed
//! skip list, and a depth cap. It returns everything it found plus everything it skipped and
//! everything that failed, so the UI can explain a count instead of guessing at it.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use sortlens_core::{MediaKind, Result, Review};
use sortlens_meta::PhotoMeta;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::UNIX_EPOCH;

/// Directories that never contain user photos.
const SKIP_DIR_NAMES: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    ".sortlens",
    "@eadir",
    "#recycle",
    "$recycle.bin",
    "system volume information",
    ".trash",
    ".trashes",
    ".spotlight-v100",
    ".fseventsd",
    "node_modules",
    "lost+found",
];

/// Files that are never photos even though they sit beside them.
const SKIP_FILE_NAMES: &[&str] = &[".ds_store", "thumbs.db", "desktop.ini", ".directory"];

/// How deep the walker will go below the root. Deeper than any sane folder tree, shallow enough
/// that a pathological mount cannot hang the import.
pub const MAX_DEPTH: usize = 64;

/// The class of file, from its extension only. Extension wins over content sniffing because the
/// walker must not open every file to decide whether to open it.
pub fn media_kind(extension: &str) -> Option<MediaKind> {
    sortlens_core::media::kind_from_extension(extension)
}

/// Whether a file extension is a sidecar rather than a photo.
pub fn is_sidecar(extension: &str) -> bool {
    sortlens_core::media::is_sidecar_extension(extension)
}

/// The review a sidecar carries. An unreadable sidecar is not a scan failure: the photo still
/// belongs in the library, it just arrives without a review.
fn sidecar_review(text: &str) -> Option<Review> {
    let sidecar = sortlens_sidecar::parse(text);
    let review = Review {
        rating: sidecar.rating,
        flag: sidecar.flag,
        label: sidecar.label,
        tags: sidecar.keywords,
    };
    (!review.is_empty()).then_some(review)
}

/// A file the walker accepted.
#[derive(Debug, Clone, PartialEq)]
pub struct ScannedFile {
    /// Absolute path.
    pub path: PathBuf,
    /// Path relative to the scan root, with `/` separators on every platform.
    pub rel: String,
    /// Size in bytes.
    pub size: u64,
    /// Modification time in seconds since the Unix epoch, if the platform reports one.
    pub modified: Option<i64>,
    /// What the extension says it is.
    pub kind: MediaKind,
    /// Cheap content identity: size plus a hash of the first and last 64 KiB.
    pub key: u64,
    /// Directories between the root and the file.
    pub depth: usize,
    /// EXIF and XMP facts, empty when the file has none.
    pub meta: PhotoMeta,
    /// The file could not be read for metadata, so the empty result is not final.
    pub meta_unreadable: bool,
    /// Review state from the photo's sidecar, when one exists beside it.
    pub review: Option<Review>,
    /// Whether the sidecar carries edit settings from another application.
    pub edited: bool,
}

/// Why a candidate was skipped, counted rather than listed one by one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Skipped {
    /// Hidden files and folders.
    pub hidden: usize,
    /// Sidecars, thumbnails and desktop litter.
    pub non_media: usize,
    /// Directories on the skip list.
    pub system_dir: usize,
    /// Symlinks, which are never followed.
    pub symlink: usize,
    /// Entries deeper than [`MAX_DEPTH`].
    pub too_deep: usize,
    /// Files whose header contradicted their extension; counted, not skipped.
    pub mislabelled: usize,
}

impl Skipped {
    /// Total skipped entries.
    pub const fn total(&self) -> usize {
        self.hidden + self.non_media + self.system_dir + self.symlink + self.too_deep
    }
}

/// A directory or file that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    /// Path that failed.
    pub path: PathBuf,
    /// What the operating system said.
    pub message: String,
}

/// Live counters and a cancel switch for a running scan.
///
/// Cloning the control is cheap and shares the same state, so a worker thread can advance the
/// counters while the UI reads them.
#[derive(Debug, Clone, Default)]
pub struct ScanControl {
    cancel: Arc<AtomicBool>,
    files: Arc<AtomicUsize>,
    dirs: Arc<AtomicUsize>,
    errors: Arc<AtomicUsize>,
}

impl ScanControl {
    /// A fresh control.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ask the scan to stop after the directory it is in.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Whether a cancel has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Counters as of now.
    pub fn snapshot(&self) -> ScanProgress {
        ScanProgress {
            files: self.files.load(Ordering::Relaxed),
            dirs: self.dirs.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }

    fn publish(&self, report: &ScanReport) {
        self.files.store(report.files.len(), Ordering::Relaxed);
        self.dirs.store(report.dirs, Ordering::Relaxed);
        self.errors.store(report.errors.len(), Ordering::Relaxed);
    }
}

/// What a scan has done so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanProgress {
    /// Files accepted so far.
    pub files: usize,
    /// Directories visited so far.
    pub dirs: usize,
    /// Unreadable entries so far.
    pub errors: usize,
}

impl ScanProgress {
    /// Whether anything has been visited yet.
    pub fn is_empty(&self) -> bool {
        self.files == 0 && self.dirs == 0
    }
}

/// What one root produced.
#[derive(Debug, Default)]
pub struct ScanReport {
    /// Accepted files, in walk order.
    pub files: Vec<ScannedFile>,
    /// Counts by reason.
    pub skipped: Skipped,
    /// Read failures, which never abort the scan.
    pub errors: Vec<ScanError>,
    /// Directories visited.
    pub dirs: usize,
    /// The walk stopped because it was cancelled; the file list is partial and must not be
    /// treated as a complete picture of the folder.
    pub cancelled: bool,
}

impl ScanReport {
    /// Total bytes of accepted files.
    pub fn bytes(&self) -> u64 {
        self.files.iter().map(|file| file.size).sum()
    }
}

/// Walk one root and collect media files.
///
/// Directories are visited breadth-first so a huge folder does not starve its siblings, and the
/// result is sorted by path at the end for a stable grid order.
pub fn scan_root(root: &Path) -> Result<ScanReport> {
    scan_root_with(root, &ScanControl::new())
}

/// Walk one root, reporting progress and honouring cancellation.
pub fn scan_root_with(root: &Path, control: &ScanControl) -> Result<ScanReport> {
    let root = fs::canonicalize(root).map_err(|error| sortlens_core::Error::io(root.display().to_string(), error))?;
    let mut report = ScanReport::default();
    let mut queue = vec![(root.clone(), 0usize)];
    let mut index = 0;

    while index < queue.len() {
        if control.is_cancelled() {
            report.cancelled = true;
            break;
        }
        let (dir, depth) = queue[index].clone();
        index += 1;
        report.dirs += 1;
        control.publish(&report);

        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                report.errors.push(ScanError {
                    path: dir.clone(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        let mut children = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    report.errors.push(ScanError {
                        path: dir.clone(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = match entry.metadata() {
                Ok(meta) => meta,
                Err(error) => {
                    report.errors.push(ScanError {
                        path: path.clone(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            // `metadata` follows symlinks; `symlink_metadata` does not. Guard before descending.
            if entry.file_type().map(|kind| kind.is_symlink()).unwrap_or(false) {
                report.skipped.symlink += 1;
                continue;
            }

            if is_hidden(&name) {
                report.skipped.hidden += 1;
                continue;
            }

            if meta.is_dir() {
                if SKIP_DIR_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
                    report.skipped.system_dir += 1;
                    continue;
                }
                if depth + 1 > MAX_DEPTH {
                    report.skipped.too_deep += 1;
                    continue;
                }
                children.push(path);
                continue;
            }

            if !meta.is_file() {
                report.skipped.non_media += 1;
                continue;
            }
            if SKIP_FILE_NAMES.contains(&name.to_ascii_lowercase().as_str()) {
                report.skipped.non_media += 1;
                continue;
            }

            let extension = Path::new(&name)
                .extension()
                .map(|ext| ext.to_string_lossy().to_string())
                .unwrap_or_default();
            if is_sidecar(&extension) {
                report.skipped.non_media += 1;
                continue;
            }
            let Some(claimed) = media_kind(&extension) else {
                report.skipped.non_media += 1;
                continue;
            };
            // The header decides when it contradicts the name, because the name is the thing that is wrong:
            // a raw that is really a JPEG preview draws a broken cell otherwise, and a video named `.jpg`
            // opens a decoder that cannot read it. Ambiguity (a TIFF, which every raw also is) keeps the
            // extension's answer.
            let (kind, corrected) = sortlens_core::media::resolve_kind(claimed, read_magic(&path));
            if corrected {
                report.skipped.mislabelled += 1;
            }

            let size = meta.len();
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|delta| delta.as_secs() as i64);
            let key = match content_key(&path, size) {
                Ok(key) => key,
                Err(error) => {
                    report.errors.push(ScanError {
                        path: path.clone(),
                        message: error.to_string(),
                    });
                    continue;
                }
            };
            let rel = relative_path(&root, &path);
            let meta_outcome = sortlens_meta::read_outcome(&path);
            let meta = meta_outcome.meta();
            let sidecar_text = sortlens_sidecar::read_text(&path);
            let review = sidecar_text.as_deref().and_then(sidecar_review);
            let edited = sidecar_text.as_deref().is_some_and(sortlens_sidecar::has_edits);
            report.files.push(ScannedFile {
                path,
                rel,
                size,
                modified,
                kind,
                key,
                depth,
                meta,
                meta_unreadable: meta_outcome.is_retryable(),
                review,
                edited,
            });
        }

        control.publish(&report);
        queue.extend(children.into_iter().map(|child| (child, depth + 1)));
    }

    report.files.sort_by_cached_key(|file| file.rel.to_lowercase());
    Ok(report)
}

/// Hash of the first and last 64 KiB plus the size.
///
/// This is an identity, not a duplicate finder: it is fast enough to run on import and stable
/// enough that moving a file inside a root keeps its rating. Duplicate detection upgrades the
/// same key with a full hash in M6.
pub fn content_key(path: &Path, size: u64) -> std::io::Result<u64> {
    const CHUNK: u64 = 64 * 1024;
    let mut file = fs::File::open(path)?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut buffer = vec![0u8; CHUNK as usize];

    let head = file.read(&mut buffer)?;
    hash = fnv1a(hash, &buffer[..head]);

    if size > CHUNK {
        use std::io::{Seek, SeekFrom};
        let tail_start = size.saturating_sub(CHUNK);
        file.seek(SeekFrom::Start(tail_start))?;
        let tail = file.read(&mut buffer)?;
        hash = fnv1a(hash, &buffer[..tail]);
    }
    hash = fnv1a(hash, &size.to_le_bytes());
    Ok(hash)
}

/// The first [`sortlens_core::media::SNIFF_BYTES`] bytes of a file, or an empty slice if it cannot be read.
///
/// A read failure here is not an error: `content_key` opens the same file immediately afterwards and reports
/// the problem once, and a second complaint about the same unreadable file is noise.
fn read_magic(path: &Path) -> sortlens_core::media::Magic {
    let mut head = [0_u8; sortlens_core::media::SNIFF_BYTES];
    match fs::File::open(path).and_then(|mut file| file.read(&mut head)) {
        Ok(read) => sortlens_core::media::sniff(&head[..read]),
        Err(_) => sortlens_core::media::Magic::Unknown,
    }
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.') && name != "." && name != ".."
}

/// Relative path with forward slashes, so a Windows catalog reads the same as a macOS one.
fn relative_path(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let mut parts = Vec::new();
    for component in relative.components() {
        if let Component::Normal(part) = component {
            parts.push(part.to_string_lossy().to_string());
        }
    }
    parts.join("/")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::io::Write;

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sortlens-scan-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("temp dir");
            Self(dir)
        }

        fn file(&self, rel: &str, bytes: usize) -> PathBuf {
            self.bytes(rel, &vec![7u8; bytes])
        }

        /// A file with chosen contents, for the header checks.
        fn bytes(&self, rel: &str, contents: &[u8]) -> PathBuf {
            let path = self.0.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent");
            }
            let mut file = fs::File::create(&path).expect("create");
            file.write_all(contents).expect("write");
            path
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finds_images_in_nested_folders() {
        let tree = TempTree::new("nested");
        tree.file("a.jpg", 10);
        tree.file("sub/b.CR3", 10);
        tree.file("sub/deeper/c.png", 10);
        let report = scan_root(&tree.0).expect("scan");
        assert_eq!(report.files.len(), 3);
        let rels: Vec<_> = report.files.iter().map(|file| file.rel.as_str()).collect();
        assert_eq!(rels, vec!["a.jpg", "sub/b.CR3", "sub/deeper/c.png"]);
        assert_eq!(report.files[1].kind, MediaKind::Raw);
        assert_eq!(report.files[1].depth, 1);
        assert_eq!(report.dirs, 3);
    }

    #[test]
    fn a_sidecar_beside_a_photo_becomes_its_review() {
        let tree = TempTree::new("sidecar");
        let image = tree.file("a.jpg", 4);
        tree.file("b.jpg", 4);
        let sidecar = sortlens_sidecar::Sidecar::new(
            5,
            sortlens_core::Flag::Pick,
            Some(sortlens_core::ColorLabel::Red),
            vec!["keeper".into()],
        );
        sortlens_sidecar::write(&image, &sidecar).expect("write sidecar");

        let report = scan_root(&tree.0).expect("scan");
        let with = report.files.iter().find(|file| file.rel == "a.jpg").expect("a.jpg");
        let review = with.review.as_ref().expect("review");
        assert_eq!(review.rating, 5);
        assert_eq!(review.flag, sortlens_core::Flag::Pick);
        assert_eq!(review.tags, vec!["keeper".to_string()]);

        let without = report.files.iter().find(|file| file.rel == "b.jpg").expect("b.jpg");
        assert!(without.review.is_none());
    }

    #[test]
    fn a_sidecar_with_edit_settings_marks_the_photo_edited() {
        let tree = TempTree::new("edited");
        let image = tree.file("a.jpg", 4);
        let plain = tree.file("b.jpg", 4);
        std::fs::write(
            plain.with_extension("xmp"),
            sortlens_sidecar::merge(None, &sortlens_sidecar::Sidecar::new(3, sortlens_core::Flag::None, None, vec![])),
        )
        .expect("sidecar");
        std::fs::write(
            image.with_extension("xmp"),
            r#"<rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/" crs:Exposure2012="+1.0"/>"#,
        )
        .expect("edited sidecar");

        let report = scan_root(&tree.0).expect("scan");
        let edited = report.files.iter().find(|file| file.rel == "a.jpg").expect("a.jpg");
        assert!(edited.edited);
        let plain = report.files.iter().find(|file| file.rel == "b.jpg").expect("b.jpg");
        assert!(!plain.edited, "a rating is not an edit");
    }

    #[test]
    fn skips_hidden_sidecars_litter_and_system_folders() {
        let tree = TempTree::new("skips");
        tree.file("keep.jpg", 4);
        tree.file(".hidden.jpg", 4);
        tree.file("keep.xmp", 4);
        tree.file("Thumbs.db", 4);
        tree.file("notes.txt", 4);
        tree.file(".git/object", 4);
        let report = scan_root(&tree.0).expect("scan");
        assert_eq!(report.files.len(), 1);
        // `.git` is hidden before it is a system folder, and is not descended into at all.
        assert_eq!(report.skipped.hidden, 2);
        assert_eq!(report.skipped.non_media, 3);
        assert_eq!(report.skipped.system_dir, 0);
        assert_eq!(report.dirs, 1);
    }

    #[test]
    fn identity_survives_a_rename_but_not_an_edit() {
        let tree = TempTree::new("identity");
        let path = tree.file("one.jpg", 1000);
        let key = content_key(&path, 1000).expect("key");
        let moved = tree.0.join("two.jpg");
        fs::rename(&path, &moved).expect("rename");
        assert_eq!(content_key(&moved, 1000).expect("key"), key);

        let mut file = fs::OpenOptions::new().append(true).open(&moved).expect("open");
        file.write_all(&[1u8; 10]).expect("append");
        assert_ne!(content_key(&moved, 1010).expect("key"), key);
    }

    #[test]
    fn media_kinds_come_from_the_extension() {
        assert_eq!(media_kind("JPG"), Some(MediaKind::Image));
        assert_eq!(media_kind("cr3"), Some(MediaKind::Raw));
        assert_eq!(media_kind("mov"), Some(MediaKind::Video));
        assert_eq!(media_kind("txt"), None);
    }

    /// The header wins when it contradicts the name, and the report says how often that happened.
    ///
    /// Three files, three contradictions, and one unambiguous TIFF that must be left alone:
    ///
    /// * `preview.cr2` holding JPEG bytes is a still, not a raw — the case that drew a broken cell.
    /// * `clip.jpg` holding an MP4 header is a video, not a still.
    /// * `still.mov` holding JPEG bytes is a still, not a video.
    /// * `scan.tif` holding TIFF bytes stays an image: every raw is also a TIFF, so the name decides.
    #[test]
    fn a_header_that_contradicts_the_name_corrects_the_kind() {
        let tree = TempTree::new("magic");
        let jpeg = [0xff_u8, 0xd8, 0xff, 0xe0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1];
        let mp4 = *b"\0\0\0\x18ftypmp42\0\0\0\0mp42isom";
        let tiff = *b"II*\0\x08\0\0\0\0\0\0\0";
        tree.bytes("preview.cr2", &jpeg);
        tree.bytes("clip.jpg", &mp4);
        tree.bytes("still.mov", &jpeg);
        tree.bytes("scan.tif", &tiff);
        tree.bytes("plain.jpg", &jpeg);

        let report = scan_root(&tree.0).expect("scan");
        let kind = |rel: &str| {
            report
                .files
                .iter()
                .find(|file| file.rel == rel)
                .map(|file| file.kind)
                .unwrap_or_else(|| panic!("{rel} was not indexed"))
        };
        assert_eq!(kind("preview.cr2"), MediaKind::Image, "a JPEG preview is not a raw");
        assert_eq!(kind("clip.jpg"), MediaKind::Video, "an MP4 header is not a still");
        assert_eq!(kind("still.mov"), MediaKind::Image, "a JPEG header is not a video");
        assert_eq!(kind("scan.tif"), MediaKind::Image, "TIFF stays whatever the name said");
        assert_eq!(kind("plain.jpg"), MediaKind::Image);
        assert_eq!(report.skipped.mislabelled, 3, "three contradictions, counted not skipped");
        assert_eq!(report.files.len(), 5, "and nothing was dropped for being mislabelled");
    }

    #[test]
    fn cancelling_stops_the_walk_and_marks_the_report() {
        let tree = TempTree::new("cancel");
        tree.file("a.jpg", 4);
        tree.file("sub/b.jpg", 4);
        let control = ScanControl::new();
        control.cancel();
        let report = scan_root_with(&tree.0, &control).expect("scan");
        assert!(report.cancelled);
        assert!(report.files.is_empty(), "a cancelled walk reports no files");
    }

    #[test]
    fn progress_counters_follow_the_walk() {
        let tree = TempTree::new("progress");
        tree.file("a.jpg", 4);
        tree.file("sub/b.jpg", 4);
        let control = ScanControl::new();
        assert!(control.snapshot().is_empty());
        let report = scan_root_with(&tree.0, &control).expect("scan");
        assert!(!report.cancelled);
        let progress = control.snapshot();
        assert_eq!(progress.files, 2);
        assert_eq!(progress.dirs, report.dirs);
    }

    #[test]
    fn missing_root_is_an_error() {
        let missing = std::env::temp_dir().join("sortlens-definitely-missing-root");
        let _ = fs::remove_dir_all(&missing);
        assert!(scan_root(&missing).is_err());
    }
}
